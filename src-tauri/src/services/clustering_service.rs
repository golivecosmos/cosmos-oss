use crate::services::database_service::DatabaseService;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use ndarray::{Array1, Array2, Axis};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const EMBEDDING_DIMS: usize = 768;

// ===== Data structures =====

/// A file with its representative embedding vector
#[derive(Clone)]
pub struct FileEmbedding {
    pub file_id: String,
    pub file_path: String,
    pub source_type: String,
    pub mime_type: Option<String>,
    pub embedding: Vec<f32>,
}

/// A computed cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCluster {
    pub cluster_id: i64,
    pub name: String,
    pub position_x: f64,
    pub position_y: f64,
    pub dominant_type: String,
    pub auto_tags: Vec<String>,
    pub file_count: usize,
}

/// A file's 2D position on the visual map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePosition2D {
    pub file_id: String,
    pub file_path: String,
    pub x: f64,
    pub y: f64,
    pub cluster_id: i64,
    pub source_type: String,
}

// ===== Service =====

pub struct ClusteringService {
    db_service: Arc<DatabaseService>,
}

impl ClusteringService {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    /// Run the full clustering pipeline: load embeddings -> cluster -> name -> persist
    pub fn compute_clusters(&self) -> Result<Vec<FileCluster>> {
        app_log_info!("🧮 CLUSTERING: Starting cluster computation");

        // Step 1: Load all file-level embeddings
        let file_embeddings = self.load_file_embeddings()?;
        let n = file_embeddings.len();
        app_log_info!("🧮 CLUSTERING: Loaded {} file embeddings", n);

        if n < 2 {
            app_log_warn!("⚠️ CLUSTERING: Not enough files to cluster (need at least 2)");
            return Ok(vec![]);
        }

        // Step 2: Build embedding matrix (n x 768)
        let mut matrix = Array2::<f32>::zeros((n, EMBEDDING_DIMS));
        for (i, fe) in file_embeddings.iter().enumerate() {
            for (j, &val) in fe.embedding.iter().enumerate() {
                matrix[[i, j]] = val;
            }
        }

        // Step 3: Random projection 768d -> 50d for faster k-means
        let reduced = random_projection(&matrix, 50);

        // Step 4: Auto-select k and run k-means
        let max_k = ((n as f64).sqrt() / 2.0).ceil() as usize;
        let max_k = max_k.clamp(2, 30);
        let k = auto_select_k(&reduced, max_k);
        app_log_info!("🧮 CLUSTERING: Auto-selected k={} for {} files", k, n);

        let assignments = kmeans(&reduced, k, 50);

        // Step 5: PCA to 2D for visualization
        let positions_2d = pca_2d(&matrix);

        // Step 6: Generate cluster names via TF-IDF
        let cluster_names = self.generate_cluster_names(&file_embeddings, &assignments, k)?;

        // Step 7: Compute cluster metadata
        let mut clusters = Vec::with_capacity(k);
        for cluster_idx in 0..k {
            let member_indices: Vec<usize> = assignments
                .iter()
                .enumerate()
                .filter(|(_, &a)| a == cluster_idx)
                .map(|(i, _)| i)
                .collect();

            if member_indices.is_empty() {
                continue;
            }

            // Centroid in 2D
            let cx: f64 = member_indices.iter().map(|&i| positions_2d[[i, 0]] as f64).sum::<f64>()
                / member_indices.len() as f64;
            let cy: f64 = member_indices.iter().map(|&i| positions_2d[[i, 1]] as f64).sum::<f64>()
                / member_indices.len() as f64;

            // Dominant type
            let mut type_counts: HashMap<&str, usize> = HashMap::new();
            for &i in &member_indices {
                *type_counts
                    .entry(&file_embeddings[i].source_type)
                    .or_insert(0) += 1;
            }
            let dominant_type = type_counts
                .into_iter()
                .max_by_key(|(_, c)| *c)
                .map(|(t, _)| t.to_string())
                .unwrap_or_else(|| "mixed".to_string());

            let (name, tags) = &cluster_names[cluster_idx];

            clusters.push(FileCluster {
                cluster_id: cluster_idx as i64,
                name: name.clone(),
                position_x: cx,
                position_y: cy,
                dominant_type,
                auto_tags: tags.clone(),
                file_count: member_indices.len(),
            });
        }

        // Step 8: Persist to database
        self.persist_clusters(&clusters, &file_embeddings, &assignments, &positions_2d)?;

        app_log_info!(
            "✅ CLUSTERING: Computed {} clusters for {} files",
            clusters.len(),
            n
        );
        Ok(clusters)
    }

    /// Load file-level embeddings from both images and text_chunks tables
    fn load_file_embeddings(&self) -> Result<Vec<FileEmbedding>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut results: Vec<FileEmbedding> = Vec::new();

        // Load image embeddings (one per file, excluding video frames which we'll group)
        {
            let mut stmt = db.prepare(
                "SELECT id, file_path, source_type, mime_type, embedding
                 FROM images
                 WHERE embedding IS NOT NULL
                   AND (source_type IS NULL OR source_type NOT IN ('video_frame', 'transcript'))",
            )?;

            let rows = stmt.query_map(rusqlite::params![], |row| {
                let id: String = row.get(0)?;
                let file_path: String = row.get(1)?;
                let source_type: Option<String> = row.get(2)?;
                let mime_type: Option<String> = row.get(3)?;
                let embedding_blob: Vec<u8> = row.get(4)?;
                Ok((id, file_path, source_type, mime_type, embedding_blob))
            })?;

            for row in rows {
                let (id, file_path, source_type, mime_type, blob) = row?;
                if let Some(embedding) = blob_to_f32_vec(&blob) {
                    results.push(FileEmbedding {
                        file_id: id,
                        file_path,
                        source_type: source_type.unwrap_or_else(|| "image".to_string()),
                        mime_type,
                        embedding,
                    });
                }
            }
        }

        // Load video frame embeddings grouped by parent_file_path (average per video)
        {
            let mut stmt = db.prepare(
                "SELECT parent_file_path, GROUP_CONCAT(id) as ids
                 FROM images
                 WHERE embedding IS NOT NULL AND source_type = 'video_frame'
                   AND parent_file_path IS NOT NULL
                 GROUP BY parent_file_path",
            )?;

            let video_groups: Vec<(String, String)> = stmt
                .query_map(rusqlite::params![], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (parent_path, ids_str) in &video_groups {
                let ids: Vec<&str> = ids_str.split(',').collect();
                let mut embeddings: Vec<Vec<f32>> = Vec::new();

                for id in &ids {
                    let blob: Vec<u8> = db.query_row(
                        "SELECT embedding FROM images WHERE id = ?",
                        rusqlite::params![id.trim()],
                        |row| row.get(0),
                    )?;
                    if let Some(emb) = blob_to_f32_vec(&blob) {
                        embeddings.push(emb);
                    }
                }

                if !embeddings.is_empty() {
                    let avg = average_embeddings(&embeddings);
                    results.push(FileEmbedding {
                        file_id: format!("video:{}", parent_path),
                        file_path: parent_path.clone(),
                        source_type: "video".to_string(),
                        mime_type: Some("video/mp4".to_string()),
                        embedding: avg,
                    });
                }
            }
        }

        // Load text chunk embeddings grouped by file_path (average per document)
        {
            let mut stmt = db.prepare(
                "SELECT file_path, mime_type FROM text_chunks
                 WHERE embedding IS NOT NULL
                 GROUP BY file_path",
            )?;

            let text_files: Vec<(String, Option<String>)> = stmt
                .query_map(rusqlite::params![], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (file_path, mime_type) in &text_files {
                let mut chunk_stmt = db.prepare(
                    "SELECT embedding FROM text_chunks WHERE file_path = ? AND embedding IS NOT NULL",
                )?;

                let embeddings: Vec<Vec<f32>> = chunk_stmt
                    .query_map(rusqlite::params![file_path], |row| {
                        let blob: Vec<u8> = row.get(0)?;
                        Ok(blob)
                    })?
                    .filter_map(|r| r.ok())
                    .filter_map(|blob| blob_to_f32_vec(&blob))
                    .collect();

                if !embeddings.is_empty() {
                    let avg = average_embeddings(&embeddings);
                    results.push(FileEmbedding {
                        file_id: format!("text:{}", file_path),
                        file_path: file_path.clone(),
                        source_type: "text_document".to_string(),
                        mime_type: mime_type.clone(),
                        embedding: avg,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Generate cluster names using TF-IDF on text content and directory patterns
    fn generate_cluster_names(
        &self,
        files: &[FileEmbedding],
        assignments: &[usize],
        k: usize,
    ) -> Result<Vec<(String, Vec<String>)>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut cluster_docs: Vec<Vec<String>> = vec![Vec::new(); k];
        let mut cluster_dirs: Vec<Vec<String>> = vec![Vec::new(); k];

        for (i, fe) in files.iter().enumerate() {
            let cluster = assignments[i];

            // Collect directory names
            if let Some(dir) = std::path::Path::new(&fe.file_path).parent() {
                if let Some(name) = dir.file_name() {
                    cluster_dirs[cluster].push(name.to_string_lossy().to_string());
                }
            }

            // For text files, grab first chunk text
            if fe.source_type == "text_document" {
                let chunk_text: Option<String> = db
                    .query_row(
                        "SELECT chunk_text FROM text_chunks WHERE file_path = ? AND chunk_index = 0",
                        rusqlite::params![fe.file_path],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(text) = chunk_text {
                    cluster_docs[cluster].push(text);
                }
            }

            // Use filename as a document too
            if let Some(name) = std::path::Path::new(&fe.file_path).file_stem() {
                let name_words = name
                    .to_string_lossy()
                    .replace(|c: char| !c.is_alphanumeric(), " ");
                cluster_docs[cluster].push(name_words);
            }
        }

        // Simple TF-IDF: term frequency per cluster, inverse document frequency across clusters
        let stop_words: std::collections::HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on",
            "with", "at", "by", "from", "as", "into", "through", "during", "before", "after",
            "above", "below", "between", "out", "off", "over", "under", "again", "further",
            "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
            "every", "both", "few", "more", "most", "other", "some", "such", "no", "nor", "not",
            "only", "own", "same", "so", "than", "too", "very", "just", "and", "but", "or",
            "if", "while", "this", "that", "these", "those", "it", "its", "img", "jpg", "png",
            "pdf", "txt", "doc", "docx", "md", "csv",
        ]
        .iter()
        .copied()
        .collect();

        // Build term frequency per cluster
        let mut cluster_tf: Vec<HashMap<String, f64>> = Vec::with_capacity(k);
        let mut doc_freq: HashMap<String, usize> = HashMap::new();

        for docs in &cluster_docs {
            let mut tf: HashMap<String, f64> = HashMap::new();
            let mut seen_terms: std::collections::HashSet<String> = std::collections::HashSet::new();

            for doc in docs {
                for word in doc.split_whitespace() {
                    let word = word.to_lowercase();
                    if word.len() < 3 || stop_words.contains(word.as_str()) {
                        continue;
                    }
                    *tf.entry(word.clone()).or_insert(0.0) += 1.0;
                    seen_terms.insert(word);
                }
            }

            // Normalize TF
            let max_tf = tf.values().cloned().fold(1.0_f64, f64::max);
            for v in tf.values_mut() {
                *v /= max_tf;
            }

            for term in seen_terms {
                *doc_freq.entry(term).or_insert(0) += 1;
            }

            cluster_tf.push(tf);
        }

        let num_clusters = k as f64;
        let mut results: Vec<(String, Vec<String>)> = Vec::with_capacity(k);

        for cluster_idx in 0..k {
            let tf = &cluster_tf[cluster_idx];

            // Compute TF-IDF scores
            let mut scored_terms: Vec<(String, f64)> = tf
                .iter()
                .map(|(term, &tf_val)| {
                    let df = *doc_freq.get(term).unwrap_or(&1) as f64;
                    let idf = (num_clusters / df).ln() + 1.0;
                    (term.clone(), tf_val * idf)
                })
                .collect();

            scored_terms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let top_terms: Vec<String> = scored_terms.iter().take(5).map(|(t, _)| t.clone()).collect();

            // Build cluster name
            let name = if !top_terms.is_empty() {
                // Use top 2-3 terms as the name
                let name_terms: Vec<String> = top_terms
                    .iter()
                    .take(3)
                    .map(|t| {
                        let mut c = t.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().to_string() + c.as_str(),
                        }
                    })
                    .collect();
                name_terms.join(", ")
            } else {
                // Fall back to most common directory name
                let mut dir_counts: HashMap<&String, usize> = HashMap::new();
                for dir in &cluster_dirs[cluster_idx] {
                    *dir_counts.entry(dir).or_insert(0) += 1;
                }
                dir_counts
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(d, _)| d.clone())
                    .unwrap_or_else(|| format!("Cluster {}", cluster_idx + 1))
            };

            results.push((name, top_terms));
        }

        Ok(results)
    }

    /// Persist clusters and member assignments to the database
    fn persist_clusters(
        &self,
        clusters: &[FileCluster],
        files: &[FileEmbedding],
        assignments: &[usize],
        positions_2d: &Array2<f32>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Clear existing clusters
        db.execute("DELETE FROM cluster_members", rusqlite::params![])?;
        db.execute("DELETE FROM clusters", rusqlite::params![])?;

        let now = chrono::Utc::now().to_rfc3339();

        // Insert clusters
        for cluster in clusters {
            db.execute(
                "INSERT INTO clusters (id, name, position_x, position_y, dominant_type, auto_tags, file_count, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    cluster.cluster_id,
                    cluster.name,
                    cluster.position_x,
                    cluster.position_y,
                    cluster.dominant_type,
                    serde_json::to_string(&cluster.auto_tags).unwrap_or_else(|_| "[]".to_string()),
                    cluster.file_count,
                    now,
                    now,
                ],
            )?;
        }

        // Insert cluster members
        let mut stmt = db.prepare(
            "INSERT OR REPLACE INTO cluster_members (cluster_id, file_id, file_path, position_x, position_y, source_type)
             VALUES (?, ?, ?, ?, ?, ?)",
        )?;

        for (i, fe) in files.iter().enumerate() {
            let cluster_id = assignments[i] as i64;
            stmt.execute(rusqlite::params![
                cluster_id,
                fe.file_id,
                fe.file_path,
                positions_2d[[i, 0]] as f64,
                positions_2d[[i, 1]] as f64,
                fe.source_type,
            ])?;
        }

        app_log_info!(
            "✅ CLUSTERING: Persisted {} clusters with {} total members",
            clusters.len(),
            files.len()
        );
        Ok(())
    }

    /// Get all clusters from the database
    pub fn get_clusters(&self) -> Result<Vec<FileCluster>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, name, position_x, position_y, dominant_type, auto_tags, file_count
             FROM clusters ORDER BY file_count DESC",
        )?;

        let clusters = stmt
            .query_map(rusqlite::params![], |row| {
                let auto_tags_str: String = row.get(5)?;
                let auto_tags: Vec<String> =
                    serde_json::from_str(&auto_tags_str).unwrap_or_default();

                Ok(FileCluster {
                    cluster_id: row.get(0)?,
                    name: row.get(1)?,
                    position_x: row.get(2)?,
                    position_y: row.get(3)?,
                    dominant_type: row.get(4)?,
                    auto_tags,
                    file_count: row.get::<_, i64>(6)? as usize,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(clusters)
    }

    /// Get all file positions for the visual map
    pub fn get_file_positions(&self) -> Result<Vec<FilePosition2D>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT file_id, file_path, position_x, position_y, cluster_id, source_type
             FROM cluster_members",
        )?;

        let positions = stmt
            .query_map(rusqlite::params![], |row| {
                Ok(FilePosition2D {
                    file_id: row.get(0)?,
                    file_path: row.get(1)?,
                    x: row.get(2)?,
                    y: row.get(3)?,
                    cluster_id: row.get(4)?,
                    source_type: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(positions)
    }

    /// Get files in a specific cluster
    pub fn get_cluster_files(&self, cluster_id: i64) -> Result<Vec<FilePosition2D>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT file_id, file_path, position_x, position_y, cluster_id, source_type
             FROM cluster_members WHERE cluster_id = ?",
        )?;

        let files = stmt
            .query_map(rusqlite::params![cluster_id], |row| {
                Ok(FilePosition2D {
                    file_id: row.get(0)?,
                    file_path: row.get(1)?,
                    x: row.get(2)?,
                    y: row.get(3)?,
                    cluster_id: row.get(4)?,
                    source_type: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(files)
    }
}

// ===== Math utilities =====

/// Convert a BLOB of little-endian f32 values to Vec<f32>
fn blob_to_f32_vec(blob: &[u8]) -> Option<Vec<f32>> {
    if blob.len() % 4 != 0 {
        return None;
    }
    let n = blob.len() / 4;
    if n != EMBEDDING_DIMS {
        app_log_warn!(
            "⚠️ CLUSTERING: Expected {} dims, got {} dims",
            EMBEDDING_DIMS,
            n
        );
        return None;
    }
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let bytes: [u8; 4] = [blob[i * 4], blob[i * 4 + 1], blob[i * 4 + 2], blob[i * 4 + 3]];
        result.push(f32::from_le_bytes(bytes));
    }
    Some(result)
}

/// Average multiple embedding vectors
fn average_embeddings(embeddings: &[Vec<f32>]) -> Vec<f32> {
    let n = embeddings.len() as f32;
    let dims = embeddings[0].len();
    let mut avg = vec![0.0f32; dims];
    for emb in embeddings {
        for (i, &v) in emb.iter().enumerate() {
            avg[i] += v;
        }
    }
    for v in &mut avg {
        *v /= n;
    }
    avg
}

/// Random projection from high-dim to lower-dim (Johnson-Lindenstrauss)
fn random_projection(data: &Array2<f32>, target_dims: usize) -> Array2<f32> {
    let (n, d) = data.dim();
    // Use a deterministic seed for reproducibility
    let mut rng_state: u64 = 42;
    let scale = (1.0 / target_dims as f32).sqrt();

    let mut projection = Array2::<f32>::zeros((d, target_dims));
    for i in 0..d {
        for j in 0..target_dims {
            // Simple xorshift64 PRNG for speed
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            // Map to approximately Gaussian via Box-Muller-lite (just use +1/-1 for simplicity)
            let val = if rng_state % 2 == 0 { scale } else { -scale };
            projection[[i, j]] = val;
        }
    }

    data.dot(&projection)
}

/// PCA reduction to 2D using power iteration
fn pca_2d(data: &Array2<f32>) -> Array2<f32> {
    let (n, d) = data.dim();

    // Center the data
    let mean = data.mean_axis(Axis(0)).unwrap();
    let mut centered = data.clone();
    for mut row in centered.rows_mut() {
        row -= &mean;
    }

    // Power iteration for first 2 principal components
    let mut components = Array2::<f32>::zeros((2, d));

    for comp in 0..2 {
        // Initialize with random vector
        let mut v = Array1::<f32>::zeros(d);
        let mut rng_state: u64 = 123 + comp as u64 * 456;
        for i in 0..d {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            v[i] = (rng_state as f32 / u64::MAX as f32) - 0.5;
        }

        // Power iteration: v = X^T * X * v (normalized)
        for _ in 0..30 {
            let xv = centered.dot(&v);
            let xtxv = centered.t().dot(&xv);
            let norm = xtxv.dot(&xtxv).sqrt();
            if norm > 1e-10 {
                v = xtxv / norm;
            }
        }

        components.row_mut(comp).assign(&v);

        // Deflate: remove this component from the data
        if comp == 0 {
            let projection = centered.dot(&v);
            for i in 0..n {
                for j in 0..d {
                    centered[[i, j]] -= projection[i] * v[j];
                }
            }
        }
    }

    // Project data onto 2D
    let result = data.dot(&components.t());

    // Normalize to [0, 1] range for consistent map rendering
    let mut normalized = Array2::<f32>::zeros((n, 2));
    for col in 0..2 {
        let col_data = result.column(col);
        let min = col_data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = col_data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = if (max - min).abs() < 1e-10 {
            1.0
        } else {
            max - min
        };
        for i in 0..n {
            normalized[[i, col]] = (result[[i, col]] - min) / range;
        }
    }

    normalized
}

/// K-means clustering with k-means++ initialization
fn kmeans(data: &Array2<f32>, k: usize, max_iter: usize) -> Vec<usize> {
    let (n, d) = data.dim();
    let k = k.min(n);

    // K-means++ initialization
    let mut centroids = Array2::<f32>::zeros((k, d));

    // Pick first centroid randomly (deterministic seed)
    let first = 0;
    centroids.row_mut(0).assign(&data.row(first));

    for c in 1..k {
        // Compute distances to nearest existing centroid
        let distances: Vec<f32> = (0..n)
            .into_par_iter()
            .map(|i| {
                let point = data.row(i);
                let mut min_dist = f32::INFINITY;
                for j in 0..c {
                    let centroid = centroids.row(j);
                    let dist: f32 = point
                        .iter()
                        .zip(centroid.iter())
                        .map(|(&a, &b)| (a - b) * (a - b))
                        .sum();
                    min_dist = min_dist.min(dist);
                }
                min_dist
            })
            .collect();

        // Pick next centroid with probability proportional to distance squared
        let total: f32 = distances.iter().sum();
        if total <= 0.0 {
            centroids.row_mut(c).assign(&data.row(c % n));
            continue;
        }
        // Deterministic selection using cumulative distribution
        let target = total * (c as f32 / k as f32);
        let mut cumulative = 0.0f32;
        let mut chosen = 0;
        for (i, &d) in distances.iter().enumerate() {
            cumulative += d;
            if cumulative >= target {
                chosen = i;
                break;
            }
        }
        centroids.row_mut(c).assign(&data.row(chosen));
    }

    // Lloyd's iterations
    let mut assignments = vec![0usize; n];

    for _iter in 0..max_iter {
        // Assign each point to nearest centroid (parallel)
        let new_assignments: Vec<usize> = (0..n)
            .into_par_iter()
            .map(|i| {
                let point = data.row(i);
                let mut best = 0;
                let mut best_dist = f32::INFINITY;
                for j in 0..k {
                    let centroid = centroids.row(j);
                    let dist: f32 = point
                        .iter()
                        .zip(centroid.iter())
                        .map(|(&a, &b)| (a - b) * (a - b))
                        .sum();
                    if dist < best_dist {
                        best_dist = dist;
                        best = j;
                    }
                }
                best
            })
            .collect();

        // Check convergence
        let changed = new_assignments
            .iter()
            .zip(assignments.iter())
            .filter(|(a, b)| a != b)
            .count();
        assignments = new_assignments;

        if changed == 0 {
            app_log_debug!(
                "🧮 CLUSTERING: K-means converged at iteration {}",
                _iter + 1
            );
            break;
        }

        // Update centroids
        let mut new_centroids = Array2::<f32>::zeros((k, d));
        let mut counts = vec![0usize; k];

        for i in 0..n {
            let c = assignments[i];
            counts[c] += 1;
            for j in 0..d {
                new_centroids[[c, j]] += data[[i, j]];
            }
        }

        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..d {
                    new_centroids[[c, j]] /= counts[c] as f32;
                }
            } else {
                // Empty cluster: keep old centroid
                new_centroids.row_mut(c).assign(&centroids.row(c));
            }
        }

        centroids = new_centroids;
    }

    assignments
}

/// Auto-select k using the elbow method (inertia drop-off)
fn auto_select_k(data: &Array2<f32>, max_k: usize) -> usize {
    let n = data.nrows();
    let max_k = max_k.min(n);

    if max_k <= 2 {
        return max_k.max(2);
    }

    // Test k values from 2 to max_k
    let mut inertias: Vec<(usize, f64)> = Vec::new();

    for k in 2..=max_k {
        let assignments = kmeans(data, k, 20); // fewer iterations for selection
        let (_, d) = data.dim();

        // Compute centroids
        let mut centroids = Array2::<f32>::zeros((k, d));
        let mut counts = vec![0usize; k];
        for i in 0..n {
            let c = assignments[i];
            counts[c] += 1;
            for j in 0..d {
                centroids[[c, j]] += data[[i, j]];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..d {
                    centroids[[c, j]] /= counts[c] as f32;
                }
            }
        }

        // Compute inertia (sum of squared distances to centroid)
        let inertia: f64 = (0..n)
            .map(|i| {
                let c = assignments[i];
                let point = data.row(i);
                let centroid = centroids.row(c);
                point
                    .iter()
                    .zip(centroid.iter())
                    .map(|(&a, &b)| ((a - b) as f64) * ((a - b) as f64))
                    .sum::<f64>()
            })
            .sum();

        inertias.push((k, inertia));
    }

    // Elbow method: find the k where the rate of decrease slows most
    if inertias.len() < 3 {
        return inertias[0].0;
    }

    let mut best_k = inertias[0].0;
    let mut best_angle = f64::NEG_INFINITY;

    for i in 1..inertias.len() - 1 {
        let prev = inertias[i - 1].1;
        let curr = inertias[i].1;
        let next = inertias[i + 1].1;

        // "Angle" at this point: larger angle means sharper elbow
        let d1 = prev - curr;
        let d2 = curr - next;
        let angle = d1 - d2; // Second derivative of inertia

        if angle > best_angle {
            best_angle = angle;
            best_k = inertias[i].0;
        }
    }

    best_k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_to_f32_vec() {
        let original: Vec<f32> = (0..EMBEDDING_DIMS).map(|i| i as f32 * 0.001).collect();
        let blob: Vec<u8> = original
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let result = blob_to_f32_vec(&blob).unwrap();
        assert_eq!(result.len(), EMBEDDING_DIMS);
        for (a, b) in original.iter().zip(result.iter()) {
            assert!((a - b).abs() < 1e-7);
        }
    }

    #[test]
    fn test_average_embeddings() {
        let e1 = vec![1.0, 2.0, 3.0];
        let e2 = vec![3.0, 4.0, 5.0];
        let avg = average_embeddings(&[e1, e2]);
        assert_eq!(avg, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_kmeans_basic() {
        // Two clear clusters in 2D
        let mut data = Array2::<f32>::zeros((10, 2));
        for i in 0..5 {
            data[[i, 0]] = 0.0 + (i as f32) * 0.1;
            data[[i, 1]] = 0.0 + (i as f32) * 0.1;
        }
        for i in 5..10 {
            data[[i, 0]] = 10.0 + (i as f32) * 0.1;
            data[[i, 1]] = 10.0 + (i as f32) * 0.1;
        }

        let assignments = kmeans(&data, 2, 50);

        // All points in the same group should have the same assignment
        let group_a = assignments[0];
        let group_b = assignments[5];
        assert_ne!(group_a, group_b);
        for i in 0..5 {
            assert_eq!(assignments[i], group_a);
        }
        for i in 5..10 {
            assert_eq!(assignments[i], group_b);
        }
    }

    #[test]
    fn test_pca_2d() {
        let data = Array2::<f32>::from_shape_fn((20, 10), |(i, j)| (i * 10 + j) as f32);
        let result = pca_2d(&data);
        assert_eq!(result.dim(), (20, 2));
        // Check normalization to [0, 1]
        for i in 0..20 {
            assert!(result[[i, 0]] >= 0.0 && result[[i, 0]] <= 1.0);
            assert!(result[[i, 1]] >= 0.0 && result[[i, 1]] <= 1.0);
        }
    }

    #[test]
    fn test_random_projection() {
        let data = Array2::<f32>::from_shape_fn((10, 768), |(i, j)| (i * 768 + j) as f32 * 0.001);
        let reduced = random_projection(&data, 50);
        assert_eq!(reduced.dim(), (10, 50));
    }
}

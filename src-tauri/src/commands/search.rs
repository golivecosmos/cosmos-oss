use crate::app_log_error;
use crate::app_log_info;
use crate::app_log_warn;
use crate::models::embedding::ImageVectorDataResponse;
use crate::AppState;
use base64::Engine;
use image;
use serde_json::json;
use std::collections::HashSet;
use tauri::State;

// **NEW: Benchmark result structure for comparing backends**
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResult {
    pub backend: String,
    pub query_time_ms: f64,
    pub result_count: usize,
    pub results: Vec<ImageVectorDataResponse>,
    pub memory_usage_mb: Option<f64>,
    pub index_size_mb: Option<f64>,
    pub error: Option<String>,
}

/// Search for similar images using text
#[tauri::command]
pub async fn search_semantic(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<ImageVectorDataResponse>, String> {
    app_log_info!("🔍 SEMANTIC SEARCH: Starting search for query: '{}'", query);

    // Validate search query
    if query.trim().is_empty() {
        app_log_warn!("Empty search query provided");
        return Err("Search query cannot be empty".to_string());
    }

    // Check if semantic search is available
    if !state.embedding_service.is_semantic_search_available() {
        app_log_warn!("Semantic search not available - models not loaded");
        return Err(
            "Search models are not loaded. Please wait for model download to complete.".to_string(),
        );
    }

    app_log_info!("🗄️ Using unified semantic search (text chunks + image vectors)");

    // Get text embedding for SQLite search
    match state
        .embedding_service
        .get_text_embedding_for_benchmark(&query)
        .await
    {
        Ok(embedding) => {
            let text_results = match state
                .sqlite_service
                .search_text_chunks_strict(&embedding.0, 120)
            {
                Ok(results) => dedupe_text_chunk_results(results, 80),
                Err(e) => {
                    app_log_error!("❌ TEXT SEARCH FAILED: {}", e);
                    Vec::new()
                }
            };

            let image_results = match state.sqlite_service.search_vectors(&embedding.0, 120) {
                Ok(results) => results,
                Err(e) => {
                    app_log_error!("❌ IMAGE SEARCH FAILED: {}", e);
                    Vec::new()
                }
            };

            let merged_results = merge_semantic_results(text_results, image_results, 120);
            let text_count = merged_results
                .iter()
                .filter(|r| r.source_type.as_deref() == Some("text_chunk"))
                .count();
            let image_count = merged_results.len().saturating_sub(text_count);
            app_log_info!(
                "✅ SEARCH RESULTS: Got {} unified semantic results (text={}, image/video={})",
                merged_results.len(),
                text_count,
                image_count
            );

            for (i, result) in merged_results.iter().enumerate().take(5) {
                app_log_info!(
                    "🔍 RESULT {}: file='{}', score={:.4}, source_type={:?}, chunk_index={:?}",
                    i + 1,
                    result.file_path,
                    result.score,
                    result.source_type,
                    result.chunk_index
                );
            }

            Ok(merged_results)
        }
        Err(e) => {
            app_log_error!("❌ EMBEDDING GENERATION FAILED: {}", e);
            Err(format!("Failed to generate embedding: {}", e))
        }
    }
}

fn dedupe_text_chunk_results(
    results: Vec<ImageVectorDataResponse>,
    limit: usize,
) -> Vec<ImageVectorDataResponse> {
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut deduped = Vec::new();

    for mut result in results {
        if seen_paths.insert(result.file_path.clone()) {
            // Keep snippets lightweight for UI payload.
            if let Some(snippet) = &result.snippet {
                let trimmed = snippet.trim();
                result.snippet = Some(if trimmed.chars().count() > 360 {
                    let prefix: String = trimmed.chars().take(360).collect();
                    format!("{}...", prefix)
                } else {
                    trimmed.to_string()
                });
            }

            deduped.push(result);
            if deduped.len() >= limit {
                break;
            }
        }
    }

    deduped
}

fn merge_semantic_results(
    text_results: Vec<ImageVectorDataResponse>,
    image_results: Vec<ImageVectorDataResponse>,
    limit: usize,
) -> Vec<ImageVectorDataResponse> {
    if limit == 0 {
        return Vec::new();
    }

    let deduped_text = dedupe_by_file_path_keep_best(text_results);
    let deduped_image = dedupe_by_file_path_keep_best(image_results);

    // Score scales differ between text-chunk and image/vector retrieval.
    // Reserve a slice for image/video so text queries remain multimodal.
    let reserved_image_slots = if !deduped_text.is_empty() && !deduped_image.is_empty() {
        std::cmp::min(deduped_image.len(), std::cmp::max(10, limit / 3))
    } else {
        0
    };
    let reserved_text_slots = std::cmp::min(deduped_text.len(), limit - reserved_image_slots);

    let mut text_iter = deduped_text.into_iter().take(reserved_text_slots);
    let mut image_iter = deduped_image.into_iter().take(reserved_image_slots);
    let mut merged = Vec::with_capacity(limit);
    let mut seen_paths: HashSet<String> = HashSet::new();

    // Interleave 2 text + 1 image to keep relevance while preserving modality diversity.
    while merged.len() < limit {
        let mut pushed_any = false;

        for _ in 0..2 {
            if let Some(candidate) = next_unseen(&mut text_iter, &mut seen_paths) {
                merged.push(candidate);
                pushed_any = true;
                if merged.len() >= limit {
                    break;
                }
            } else {
                break;
            }
        }

        if merged.len() < limit {
            if let Some(candidate) = next_unseen(&mut image_iter, &mut seen_paths) {
                merged.push(candidate);
                pushed_any = true;
            }
        }

        if !pushed_any {
            break;
        }
    }

    merged
}

fn dedupe_by_file_path_keep_best(
    mut results: Vec<ImageVectorDataResponse>,
) -> Vec<ImageVectorDataResponse> {
    results.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut deduped = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();
    for result in results {
        if seen_paths.insert(result.file_path.clone()) {
            deduped.push(result);
        }
    }
    deduped
}

fn next_unseen(
    iter: &mut impl Iterator<Item = ImageVectorDataResponse>,
    seen_paths: &mut HashSet<String>,
) -> Option<ImageVectorDataResponse> {
    for candidate in iter {
        if seen_paths.insert(candidate.file_path.clone()) {
            return Some(candidate);
        }
    }
    None
}

/// Search for similar images using visual similarity
#[tauri::command]
pub async fn search_visual(
    image_data: String,
    state: State<'_, AppState>,
) -> Result<Vec<ImageVectorDataResponse>, String> {
    app_log_info!("🔍 VISUAL SEARCH: Starting visual search");

    // Validate base64 data
    if image_data.trim().is_empty() {
        app_log_warn!("Empty image data provided");
        return Err("Image data cannot be empty".to_string());
    }

    // Check if semantic search is available
    if !state.embedding_service.is_semantic_search_available() {
        app_log_warn!("Visual search not available - models not loaded");
        return Err(
            "Search models are not loaded. Please wait for model download to complete.".to_string(),
        );
    }

    // Decode base64 image
    let image_bytes = match base64::engine::general_purpose::STANDARD.decode(&image_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            app_log_error!("Failed to decode base64 image: {}", e);
            return Err("Failed to decode image data".to_string());
        }
    };

    // Load image
    let img = match image::load_from_memory(&image_bytes) {
        Ok(img) => img,
        Err(e) => {
            app_log_error!("Failed to load image: {}", e);
            return Err("Failed to load image".to_string());
        }
    };

    app_log_info!(
        "🖼️ VISUAL SEARCH: Image loaded successfully, dimensions: {}x{}",
        img.width(),
        img.height()
    );

    // Perform visual search using SQLite backend
    match state.embedding_service.search_by_image(&img, 20).await {
        Ok(results) => {
            app_log_info!("✅ VISUAL SEARCH: Got {} results", results.len());

            // Log scores for debugging
            for (i, result) in results.iter().enumerate().take(5) {
                app_log_info!(
                    "🔍 RESULT {}: file='{}', score={:.4}, is_video_frame={}",
                    i + 1,
                    result.file_path,
                    result.score,
                    result.is_video_frame()
                );
            }

            // **CHANGED: Return individual frames instead of grouping them**
            // This allows users to see multiple matching frames from videos

            // Sort by score (ascending - lower cosine distance = better similarity)
            let mut final_results = results;
            final_results.sort_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            app_log_info!(
                "✅ RETURNING {} INDIVIDUAL RESULTS (including video frames)",
                final_results.len()
            );

            Ok(final_results)
        }
        Err(e) => {
            app_log_error!("❌ VISUAL SEARCH FAILED: {}", e);
            Err(format!("Visual search failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn check_search_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 STATUS CHECK: Checking search system status");

    let embedding_service = &state.embedding_service;
    let sqlite_service = &state.sqlite_service;

    // Check if semantic search is available
    let model_loaded = embedding_service.is_semantic_search_available();
    app_log_info!("📊 MODEL STATUS: Loaded = {}", model_loaded);

    // Get SQLite stats
    let sqlite_stats = match sqlite_service.get_stats() {
        Ok(stats) => {
            app_log_info!(
                "📊 SQLITE STATUS: {}",
                serde_json::to_string_pretty(&stats).unwrap_or_default()
            );
            stats
        }
        Err(e) => {
            app_log_error!("📊 SQLITE ERROR: Failed to get stats - {}", e);
            json!({})
        }
    };

    // Get indexed count from SQLite
    let indexed_count = match sqlite_service.get_semantic_file_count() {
        Ok(count) => {
            app_log_info!("📊 INDEX STATUS: {} files indexed", count);
            count
        }
        Err(e) => {
            app_log_error!("📊 INDEX ERROR: Failed to get count - {}", e);
            0
        }
    };

    let status = serde_json::json!({
        "model_loaded": model_loaded,
        "indexed_count": indexed_count,
        "sqlite_stats": sqlite_stats,
        "status": if model_loaded { "ready" } else { "not_ready" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    app_log_info!("📊 OVERALL STATUS: {:?}", status);

    Ok(status)
}

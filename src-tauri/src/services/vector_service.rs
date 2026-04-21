use crate::models::embedding::ImageVectorDataResponse;
use crate::services::database_service::DatabaseService;
use crate::services::schema_service::SchemaService;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde_json;
use std::sync::Arc;
use zerocopy::AsBytes;

#[derive(Debug, Clone)]
pub struct ImageVectorBulkData {
    pub id: String,
    pub file_path: String,
    pub parent_file_path: Option<String>,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    pub drive_uuid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextChunkBulkData {
    pub id: String,
    pub file_path: String,
    pub parent_file_path: Option<String>,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub chunk_index: i64,
    pub chunk_text: String,
    pub char_start: i64,
    pub char_end: i64,
    pub token_estimate: i64,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
    pub drive_uuid: Option<String>,
}

pub struct VectorService {
    db_service: Arc<DatabaseService>,
}

impl VectorService {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    pub fn store_image_vector_with_drive(
        &self,
        id: String,
        file_path: String,
        parent_file_path: Option<String>,
        file_name: String,
        mime_type: Option<String>,
        embedding: Vec<f32>,
        metadata: serde_json::Value,
        drive_uuid: Option<String>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        if embedding.len() != 768 {
            return Err(anyhow!(
                "Invalid embedding dimensions: expected 768, got {}",
                embedding.len()
            ));
        }
        let width = metadata.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
        let height = metadata.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
        let aspect_ratio = metadata
            .get("aspect_ratio")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let fs_size = metadata
            .get("fs_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i64;
        let source_type = metadata.get("source_type").and_then(|v| v.as_str());
        let timestamp = metadata.get("timestamp").and_then(|v| v.as_f64());
        let timestamp_formatted = metadata.get("timestamp_formatted").and_then(|v| v.as_str());
        let frame_number = metadata
            .get("frame_number")
            .and_then(|v| v.as_u64())
            .map(|v| v as i64);
        let video_duration = metadata.get("video_duration").and_then(|v| v.as_f64());
        let now = chrono::Utc::now().to_rfc3339();
        let file_path_for_error = file_path.clone();
        let tx = db.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO images (
                id, file_path, parent_file_path, file_name, mime_type,
                width, height, aspect_ratio, fs_size,
                source_type, timestamp, timestamp_formatted, frame_number, video_duration,
                metadata, embedding, drive_uuid,
                created_at, updated_at, last_indexed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                &id,
                &file_path,
                &parent_file_path,
                &file_name,
                &mime_type,
                width,
                height,
                aspect_ratio,
                fs_size,
                source_type,
                timestamp,
                timestamp_formatted,
                frame_number,
                video_duration,
                metadata.to_string(),
                embedding.as_bytes(),
                drive_uuid,
                now,
                now,
                now,
            ],
        )?;
        // Get actual rowid via SELECT. last_insert_rowid() is unreliable with
        // TEXT primary keys + INSERT OR REPLACE (returns stale values, often 1).
        let rowid: i64 = tx.query_row(
            "SELECT rowid FROM images WHERE id = ?1",
            rusqlite::params![&id],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO vec_images(rowid, embedding) VALUES (?, ?)",
            rusqlite::params![rowid, embedding.as_bytes()],
        )
        .map_err(|e| {
            anyhow!(
                "Failed to insert vector for rowid {} ({}): {}",
                rowid,
                file_path_for_error,
                e
            )
        })?;
        tx.commit()?;
        app_log_debug!("✅ SQLITE: Stored vector for: {}", file_path);
        Ok(())
    }

    pub fn store_image_vectors_bulk(&self, vectors: Vec<ImageVectorBulkData>) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        if vectors.is_empty() {
            return Ok(0);
        }
        app_log_debug!(
            "🚀 SQLITE BULK: Starting bulk insert of {} vectors",
            vectors.len()
        );
        let tx = db.transaction()?;
        let mut main_stmt = tx.prepare(
            "INSERT OR REPLACE INTO images (
                id, file_path, parent_file_path, file_name, mime_type,
                width, height, aspect_ratio, fs_size,
                source_type, timestamp, timestamp_formatted, frame_number, video_duration,
                metadata, embedding, drive_uuid,
                created_at, updated_at, last_indexed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        let mut vec_stmt =
            tx.prepare("INSERT OR REPLACE INTO vec_images(rowid, embedding) VALUES (?, ?)")?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut success_count = 0;
        for vector in vectors {
            if vector.embedding.len() != 768 {
                return Err(anyhow!(
                    "Invalid embedding dimensions for {}: expected 768, got {}",
                    vector.file_path,
                    vector.embedding.len()
                ));
            }
            let file_path_for_error = vector.file_path.clone();
            let width = vector
                .metadata
                .get("width")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as i64;
            let height = vector
                .metadata
                .get("height")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as i64;
            let aspect_ratio = vector
                .metadata
                .get("aspect_ratio")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let fs_size = vector
                .metadata
                .get("fs_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as i64;
            let source_type = vector.metadata.get("source_type").and_then(|v| v.as_str());
            let timestamp = vector.metadata.get("timestamp").and_then(|v| v.as_f64());
            let timestamp_formatted = vector
                .metadata
                .get("timestamp_formatted")
                .and_then(|v| v.as_str());
            let frame_number = vector
                .metadata
                .get("frame_number")
                .and_then(|v| v.as_u64())
                .map(|v| v as i64);
            let video_duration = vector
                .metadata
                .get("video_duration")
                .and_then(|v| v.as_f64());
            main_stmt
                .execute(rusqlite::params![
                    &vector.id,
                    &vector.file_path,
                    &vector.parent_file_path,
                    &vector.file_name,
                    &vector.mime_type,
                    width,
                    height,
                    aspect_ratio,
                    fs_size,
                    source_type,
                    timestamp,
                    timestamp_formatted,
                    frame_number,
                    video_duration,
                    vector.metadata.to_string(),
                    vector.embedding.as_bytes(),
                    vector.drive_uuid,
                    now,
                    now,
                    now,
                ])
                .map_err(|e| {
                    anyhow!(
                        "Failed to insert image row for {}: {}",
                        file_path_for_error,
                        e
                    )
                })?;

            let rowid: i64 = tx.query_row(
                "SELECT rowid FROM images WHERE id = ?1",
                rusqlite::params![&vector.id],
                |row| row.get(0),
            ).map_err(|e| {
                anyhow!("Failed to get rowid for {}: {}", file_path_for_error, e)
            })?;
            vec_stmt
                .execute(rusqlite::params![rowid, vector.embedding.as_bytes()])
                .map_err(|e| {
                    anyhow!(
                        "Failed to insert vector row for {} (rowid {}): {}",
                        file_path_for_error,
                        rowid,
                        e
                    )
                })?;

            success_count += 1;
        }
        drop(main_stmt);
        drop(vec_stmt);
        tx.commit()?;
        app_log_info!(
            "✅ SQLITE BULK: Completed bulk insert - {} successful, 0 failed",
            success_count
        );
        Ok(success_count)
    }

    pub fn search_vectors(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<ImageVectorDataResponse>> {
        if query_vector.len() != 768 {
            return Err(anyhow!(
                "Invalid query vector dimensions: expected 768, got {}",
                query_vector.len()
            ));
        }
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        if let Ok(results) = self.search_with_virtual_table(&db, query_vector, limit) {
            if !results.is_empty() {
                app_log_debug!(
                    "✅ SQLITE: Using virtual table search, returned {} results",
                    results.len()
                );
                return Ok(results);
            }
        }
        app_log_debug!(
            "🔍 SQLITE: Virtual table search failed, falling back to manual distance calculation"
        );
        app_log_debug!("🔍 SQLITE: Testing vec_distance_cosine function with sample data...");
        let test_result: Result<f64, _> = db.query_row(
            "SELECT vec_distance_cosine(embedding, ?) as test_distance
             FROM images WHERE embedding IS NOT NULL LIMIT 1",
            rusqlite::params![query_vector.as_bytes()],
            |row| row.get(0),
        );
        match test_result {
            Ok(distance) => {
                app_log_debug!(
                    "✅ SQLITE: vec_distance_cosine test successful, sample distance: {}",
                    distance
                );
            }
            Err(e) => {
                app_log_error!("❌ SQLITE: vec_distance_cosine test failed: {}", e);
                app_log_info!("🔄 SQLITE: Falling back to search without distance calculation");
                return self.search_without_distance(&db, limit);
            }
        }
        self.search_with_manual_distance(&db, query_vector, limit)
    }

    fn search_with_virtual_table(
        &self,
        db: &Connection,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<ImageVectorDataResponse>> {
        app_log_debug!("🚀 SQLITE: Attempting virtual table search...");
        let mut stmt = db.prepare(
            "SELECT
                i.id, i.file_path, i.metadata,
                v.distance as score,
                i.created_at, i.updated_at, i.last_indexed_at, i.mime_type,
                i.parent_file_path, i.tags,
                i.timestamp, i.timestamp_formatted, i.frame_number, i.video_duration,
                i.drive_uuid, d.name as drive_name, d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location, d.status as drive_status,
                i.source_type
            FROM vec_images v
            JOIN images i ON i.rowid = v.rowid
            LEFT JOIN drives d ON i.drive_uuid = d.uuid
            WHERE v.embedding MATCH ?
            ORDER BY v.distance
            LIMIT ?",
        )?;
        let results = stmt.query_map(rusqlite::params![query_vector.as_bytes(), limit], |row| {
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score: row.get::<_, f64>(3)? as f32,
                status: "indexed".to_string(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_indexed_at: row.get(6)?,
                mime_type: row.get(7)?,
                parent_file_path: row.get(8)?,
                tags: row.get(9)?,
                timestamp: row.get(10)?,
                timestamp_formatted: row.get(11)?,
                frame_number: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
                video_duration: row.get(13)?,
                drive_uuid: row.get(14)?,
                drive_name: row.get(15)?,
                drive_custom_name: row.get(16)?,
                drive_physical_location: row.get(17)?,
                drive_status: row.get(18)?,
                source_type: row.get(19)?,
                chunk_index: None,
                snippet: None,
            })
        })?;
        let collected_results: Result<Vec<_>, _> = results.collect();
        let final_results = collected_results?;
        app_log_debug!(
            "✅ SQLITE: Virtual table search returned {} results",
            final_results.len()
        );
        Ok(final_results)
    }

    fn search_with_manual_distance(
        &self,
        db: &Connection,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<ImageVectorDataResponse>> {
        let total_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM images WHERE embedding IS NOT NULL",
                rusqlite::params![],
                |row| row.get(0),
            )
            .unwrap_or(0);
        app_log_debug!(
            "🔍 SQLITE MANUAL SEARCH: Database has {} records with embeddings",
            total_count
        );
        if total_count == 0 {
            app_log_warn!("⚠️ SQLITE: No records with embeddings found!");
            return Ok(Vec::new());
        }
        app_log_debug!("🔍 SQLITE: Testing basic query without distance calculation...");
        let test_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM images WHERE embedding IS NOT NULL LIMIT ?",
                rusqlite::params![limit],
                |row| row.get(0),
            )
            .unwrap_or(0);
        app_log_debug!("🔍 SQLITE: Basic query would return {} records", test_count);
        app_log_debug!("🔍 SQLITE: Attempting vector distance query with NULL handling...");
        let mut stmt = match db.prepare(
            "SELECT
                i.id, i.file_path, i.metadata,
                COALESCE(vec_distance_cosine(i.embedding, ?), 1.0) as score,
                i.created_at, i.updated_at, i.last_indexed_at, i.mime_type,
                i.parent_file_path, i.tags,
                i.timestamp, i.timestamp_formatted, i.frame_number, i.video_duration,
                i.drive_uuid, d.name as drive_name, d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location, d.status as drive_status,
                i.source_type
            FROM images i
            LEFT JOIN drives d ON i.drive_uuid = d.uuid
            WHERE i.embedding IS NOT NULL
            ORDER BY score ASC
            LIMIT ?",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                app_log_error!("❌ SQLITE: Failed to prepare distance query: {}", e);
                return self.search_without_distance(db, limit);
            }
        };
        let results = stmt.query_map(rusqlite::params![query_vector.as_bytes(), limit], |row| {
            let score_result = row.get::<_, Option<f64>>(3);
            let score = match score_result {
                Ok(Some(s)) => s as f32,
                Ok(None) => {
                    app_log_warn!("⚠️ SQLITE: NULL score for record, using default value 1.0");
                    1.0f32
                }
                Err(e) => {
                    app_log_warn!(
                        "⚠️ SQLITE: Error getting score: {}, using default value 1.0",
                        e
                    );
                    1.0f32
                }
            };
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score,
                status: "indexed".to_string(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_indexed_at: row.get(6)?,
                mime_type: row.get(7)?,
                parent_file_path: row.get(8)?,
                tags: row.get(9)?,
                timestamp: row.get(10)?,
                timestamp_formatted: row.get(11)?,
                frame_number: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
                video_duration: row.get(13)?,
                drive_uuid: row.get(14)?,
                drive_name: row.get(15)?,
                drive_custom_name: row.get(16)?,
                drive_physical_location: row.get(17)?,
                drive_status: row.get(18)?,
                source_type: row.get(19)?,
                chunk_index: None,
                snippet: None,
            })
        });
        match results {
            Ok(iter) => {
                let collected_results: Result<Vec<_>, _> = iter.collect();
                match collected_results {
                    Ok(final_results) => {
                        app_log_debug!(
                            "✅ SQLITE: Manual distance search returned {} results",
                            final_results.len()
                        );
                        for (i, result) in final_results.iter().enumerate().take(3) {
                            app_log_debug!(
                                "🔍 SQLITE RESULT {}: file='{}', score={:.4}",
                                i + 1,
                                result.file_path,
                                result.score
                            );
                        }
                        Ok(final_results)
                    }
                    Err(e) => {
                        app_log_error!("❌ SQLITE: Failed to collect results: {}", e);
                        self.search_without_distance(db, limit)
                    }
                }
            }
            Err(e) => {
                app_log_error!("❌ SQLITE: Failed to execute distance query: {}", e);
                self.search_without_distance(db, limit)
            }
        }
    }

    fn search_without_distance(
        &self,
        db: &Connection,
        limit: usize,
    ) -> Result<Vec<ImageVectorDataResponse>> {
        app_log_debug!("🔄 SQLITE: Using fallback search without distance calculation");
        let mut stmt = db.prepare(
            "SELECT
                i.id, i.file_path, i.metadata,
                i.created_at, i.updated_at, i.last_indexed_at, i.mime_type,
                i.parent_file_path, i.tags,
                i.timestamp, i.timestamp_formatted, i.frame_number, i.video_duration,
                i.drive_uuid, d.name as drive_name, d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location, d.status as drive_status,
                i.source_type
            FROM images i
            LEFT JOIN drives d ON i.drive_uuid = d.uuid
            WHERE i.embedding IS NOT NULL
            ORDER BY i.created_at DESC
            LIMIT ?",
        )?;
        let results = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score: 0.5,
                status: "indexed".to_string(),
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                last_indexed_at: row.get(5)?,
                mime_type: row.get(6)?,
                parent_file_path: row.get(7)?,
                tags: row.get(8)?,
                timestamp: row.get(9)?,
                timestamp_formatted: row.get(10)?,
                frame_number: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                video_duration: row.get(12)?,
                drive_uuid: row.get(13)?,
                drive_name: row.get(14)?,
                drive_custom_name: row.get(15)?,
                drive_physical_location: row.get(16)?,
                drive_status: row.get(17)?,
                source_type: row.get(18)?,
                chunk_index: None,
                snippet: None,
            })
        })?;
        let collected_results: Result<Vec<_>, _> = results.collect();
        let final_results = collected_results?;
        app_log_debug!(
            "✅ SQLITE: Fallback search returned {} results",
            final_results.len()
        );
        Ok(final_results)
    }

    pub fn get_all_images(&self) -> Result<Vec<ImageVectorDataResponse>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT
                i.id, i.file_path, i.metadata, 0.0 as score,
                i.created_at, i.updated_at, i.last_indexed_at, i.mime_type,
                i.parent_file_path, i.tags,
                i.timestamp, i.timestamp_formatted, i.frame_number, i.video_duration,
                i.drive_uuid, d.name as drive_name, d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location, d.status as drive_status,
                i.source_type
            FROM images i
            LEFT JOIN drives d ON i.drive_uuid = d.uuid
            ORDER BY i.parent_file_path, i.created_at DESC",
        )?;
        let results = stmt.query_map(rusqlite::params![], |row| {
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score: row.get::<_, f64>(3)? as f32,
                status: "indexed".to_string(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_indexed_at: row.get(6)?,
                mime_type: row.get(7)?,
                parent_file_path: row.get(8)?,
                tags: row.get(9)?,
                timestamp: row.get(10)?,
                timestamp_formatted: row.get(11)?,
                frame_number: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
                video_duration: row.get(13)?,
                drive_uuid: row.get(14)?,
                drive_name: row.get(15)?,
                drive_custom_name: row.get(16)?,
                drive_physical_location: row.get(17)?,
                drive_status: row.get(18)?,
                source_type: row.get(19)?,
                chunk_index: None,
                snippet: None,
            })
        })?;
        let collected_results: Result<Vec<_>, _> = results.collect();
        Ok(collected_results?)
    }

    pub fn get_all_text_file_entries(&self) -> Result<Vec<ImageVectorDataResponse>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut stmt = db.prepare(
            "WITH first_chunks AS (
                SELECT file_path, MIN(chunk_index) AS first_chunk_index
                FROM text_chunks
                GROUP BY file_path
            )
            SELECT
                t.id, t.file_path, t.metadata, 0.0 as score,
                t.created_at, t.updated_at, t.last_indexed_at, t.mime_type,
                t.parent_file_path, '' as tags,
                NULL as timestamp, NULL as timestamp_formatted, NULL as frame_number, NULL as video_duration,
                t.drive_uuid, d.name as drive_name, d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location, d.status as drive_status,
                'text_document' as source_type, t.chunk_index, NULL as snippet
            FROM text_chunks t
            JOIN first_chunks fc
              ON fc.file_path = t.file_path
             AND fc.first_chunk_index = t.chunk_index
            LEFT JOIN drives d ON t.drive_uuid = d.uuid
            ORDER BY t.parent_file_path, t.created_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![], |row| {
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score: row.get::<_, f64>(3)? as f32,
                status: "indexed".to_string(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_indexed_at: row.get(6)?,
                mime_type: row.get(7)?,
                parent_file_path: row.get(8)?,
                tags: row.get(9)?,
                timestamp: row.get(10)?,
                timestamp_formatted: row.get(11)?,
                frame_number: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
                video_duration: row.get(13)?,
                drive_uuid: row.get(14)?,
                drive_name: row.get(15)?,
                drive_custom_name: row.get(16)?,
                drive_physical_location: row.get(17)?,
                drive_status: row.get(18)?,
                source_type: row.get(19)?,
                chunk_index: row.get(20)?,
                snippet: row.get(21)?,
            })
        })?;
        let collected: Result<Vec<_>, _> = rows.collect();
        Ok(collected?)
    }

    /// Get count of indexed images
    pub fn get_image_count(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let count: usize = db.query_row(
            "SELECT COUNT(*) FROM images WHERE embedding IS NOT NULL",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get count of uniquely indexed files across image/video and text chunk tables.
    pub fn get_semantic_file_count(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let count: usize = db.query_row(
            "SELECT COUNT(*) FROM (
                SELECT file_path FROM images
                UNION
                SELECT file_path FROM text_chunks
            )",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get count of video frames
    pub fn get_video_frame_count(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let count: usize = db.query_row(
            "SELECT COUNT(*) FROM images WHERE source_type = 'video_frame' AND embedding IS NOT NULL",
            rusqlite::params![],
            |row| row.get(0)
        )?;

        Ok(count)
    }

    /// Get count of indexed text chunks
    pub fn get_text_chunk_count(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let count: usize = db.query_row(
            "SELECT COUNT(*) FROM text_chunks WHERE embedding IS NOT NULL",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    pub fn recreate_virtual_table(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let schema_service = SchemaService::new(Arc::clone(&self.db_service));
        schema_service.create_nomic_virtual_table(&db)?;
        schema_service.create_text_chunks_virtual_table(&db)?;
        Ok(())
    }

    pub fn delete_image_vector(&self, id: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.execute("DELETE FROM images WHERE id = ?", rusqlite::params![id])?;
        app_log_debug!("✅ SQLITE: Deleted vector for id: {}", id);
        Ok(())
    }

    pub fn delete_indexed_data_for_file(&self, file_path: &str) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        let tx = db.transaction()?;

        let deleted_images = tx.execute(
            "DELETE FROM images WHERE file_path = ?1",
            rusqlite::params![file_path],
        )?;
        tx.execute(
            "DELETE FROM vec_text_chunks
             WHERE rowid IN (SELECT rowid FROM text_chunks WHERE file_path = ?1)",
            rusqlite::params![file_path],
        )?;
        let deleted_text_chunks = tx.execute(
            "DELETE FROM text_chunks WHERE file_path = ?1",
            rusqlite::params![file_path],
        )?;

        tx.commit()?;

        let deleted_total = deleted_images + deleted_text_chunks;
        if deleted_total > 0 {
            app_log_info!(
                "🧹 SQLITE: Removed {} indexed rows for {}",
                deleted_total,
                file_path
            );
        }

        Ok(deleted_total)
    }

    pub fn clear_index(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.execute("DELETE FROM images", rusqlite::params![])?;
        db.execute("DELETE FROM text_chunks", rusqlite::params![])?;
        db.execute("VACUUM", rusqlite::params![])?;
        app_log_info!("✅ SQLITE: Successfully cleared search index");
        Ok(())
    }

    pub fn file_exists(&self, file_path: &str) -> Result<bool> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT (
                (SELECT COUNT(*) FROM images WHERE file_path = ?1) +
                (SELECT COUNT(*) FROM text_chunks WHERE file_path = ?1)
            )",
            rusqlite::params![file_path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn delete_text_chunks_for_file(&self, file_path: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        let tx = db.transaction()?;

        tx.execute(
            "DELETE FROM vec_text_chunks
             WHERE rowid IN (SELECT rowid FROM text_chunks WHERE file_path = ?1)",
            rusqlite::params![file_path],
        )?;
        tx.execute(
            "DELETE FROM text_chunks WHERE file_path = ?1",
            rusqlite::params![file_path],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn delete_transcript_chunks_for_file(&self, file_path: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        let tx = db.transaction()?;

        tx.execute(
            "DELETE FROM vec_text_chunks
             WHERE rowid IN (
                SELECT rowid FROM text_chunks
                WHERE file_path = ?1
                  AND json_extract(metadata, '$.source_type') = 'transcript_chunk'
             )",
            rusqlite::params![file_path],
        )?;
        tx.execute(
            "DELETE FROM text_chunks
             WHERE file_path = ?1
               AND json_extract(metadata, '$.source_type') = 'transcript_chunk'",
            rusqlite::params![file_path],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn store_text_chunk_vectors_bulk(&self, chunks: Vec<TextChunkBulkData>) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();
        if chunks.is_empty() {
            return Ok(0);
        }

        let tx = db.transaction()?;
        let mut main_stmt = tx.prepare(
            "INSERT OR REPLACE INTO text_chunks (
                id, file_path, parent_file_path, file_name, mime_type,
                chunk_index, chunk_text, char_start, char_end, token_estimate,
                metadata, embedding, drive_uuid, created_at, updated_at, last_indexed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        let mut vec_stmt =
            tx.prepare("INSERT OR REPLACE INTO vec_text_chunks(rowid, embedding) VALUES (?, ?)")?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut success_count = 0usize;

        for chunk in chunks {
            if chunk.embedding.len() != 768 {
                return Err(anyhow!(
                    "Invalid text chunk embedding dimensions for {}: expected 768, got {}",
                    chunk.file_path,
                    chunk.embedding.len()
                ));
            }

            main_stmt.execute(rusqlite::params![
                &chunk.id,
                &chunk.file_path,
                &chunk.parent_file_path,
                &chunk.file_name,
                &chunk.mime_type,
                chunk.chunk_index,
                &chunk.chunk_text,
                chunk.char_start,
                chunk.char_end,
                chunk.token_estimate,
                chunk.metadata.to_string(),
                chunk.embedding.as_bytes(),
                chunk.drive_uuid,
                &now,
                &now,
                &now,
            ])?;

            let rowid: i64 = tx.query_row(
                "SELECT rowid FROM text_chunks WHERE id = ?1",
                rusqlite::params![&chunk.id],
                |row| row.get(0),
            )?;
            vec_stmt.execute(rusqlite::params![rowid, chunk.embedding.as_bytes()])?;
            success_count += 1;
        }

        drop(main_stmt);
        drop(vec_stmt);
        tx.commit()?;
        Ok(success_count)
    }

    pub fn search_text_chunks_strict(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<ImageVectorDataResponse>> {
        if query_vector.len() != 768 {
            return Err(anyhow!(
                "Invalid query vector dimensions: expected 768, got {}",
                query_vector.len()
            ));
        }

        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT
                t.id,
                t.file_path,
                t.metadata,
                v.distance as score,
                t.created_at,
                t.updated_at,
                t.last_indexed_at,
                t.mime_type,
                t.parent_file_path,
                '' as tags,
                CAST(json_extract(t.metadata, '$.time_start_seconds') AS REAL) as timestamp,
                COALESCE(
                    json_extract(t.metadata, '$.timestamp_formatted'),
                    json_extract(t.metadata, '$.time_start_formatted')
                ) as timestamp_formatted,
                NULL as frame_number,
                NULL as video_duration,
                t.drive_uuid,
                d.name as drive_name,
                d.custom_name as drive_custom_name,
                d.physical_location as drive_physical_location,
                d.status as drive_status,
                COALESCE(json_extract(t.metadata, '$.source_type'), 'text_chunk') as source_type,
                t.chunk_index,
                t.chunk_text
            FROM vec_text_chunks v
            JOIN text_chunks t ON t.rowid = v.rowid
            LEFT JOIN drives d ON t.drive_uuid = d.uuid
            WHERE v.embedding MATCH ?
              AND v.k = ?
            ORDER BY v.distance ASC",
        )?;

        let rows = stmt.query_map(rusqlite::params![query_vector.as_bytes(), limit], |row| {
            Ok(ImageVectorDataResponse {
                id: row.get(0)?,
                file_path: row.get(1)?,
                metadata: row.get(2)?,
                score: row.get::<_, f64>(3)? as f32,
                status: "indexed".to_string(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_indexed_at: row.get(6)?,
                mime_type: row.get(7)?,
                parent_file_path: row.get(8)?,
                tags: row.get(9)?,
                timestamp: row.get(10)?,
                timestamp_formatted: row.get(11)?,
                frame_number: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
                video_duration: row.get(13)?,
                drive_uuid: row.get(14)?,
                drive_name: row.get(15)?,
                drive_custom_name: row.get(16)?,
                drive_physical_location: row.get(17)?,
                drive_status: row.get(18)?,
                source_type: row.get(19)?,
                chunk_index: row.get(20)?,
                snippet: row.get(21)?,
            })
        })?;

        let collected: Result<Vec<_>, _> = rows.collect();
        Ok(collected?)
    }

    pub fn test_vector_functionality(&self) -> Result<()> {
        app_log_info!("🧪 SQLITE: Testing vector functionality");
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let test_vector: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001).collect();
        let length: f64 = db.query_row(
            "SELECT vec_length(?)",
            rusqlite::params![test_vector.as_bytes()],
            |row| row.get(0),
        )?;
        app_log_info!("🧪 SQLITE: Test vector length: {}", length);
        let json_repr: String = db.query_row(
            "SELECT vec_to_json(?)",
            rusqlite::params![test_vector.as_bytes()],
            |row| row.get(0),
        )?;
        app_log_debug!(
            "🧪 SQLITE: Test vector JSON (first 100 chars): {}",
            &json_repr.chars().take(100).collect::<String>()
        );
        db.execute(
            "INSERT OR REPLACE INTO images (
                id, file_path, file_name, embedding, created_at, updated_at
            ) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
            rusqlite::params![
                "test_vector_1",
                "/tmp/test_image.jpg",
                "test_image.jpg",
                test_vector.as_bytes(),
            ],
        )?;
        let test_query_vector: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001 + 0.1).collect();
        let distance: f64 = db.query_row(
            "SELECT vec_distance_cosine(embedding, ?) as distance
             FROM images WHERE id = 'test_vector_1'",
            rusqlite::params![test_query_vector.as_bytes()],
            |row| row.get(0),
        )?;
        app_log_info!("🧪 SQLITE: Cosine distance test result: {}", distance);
        db.execute(
            "DELETE FROM images WHERE id = 'test_vector_1'",
            rusqlite::params![],
        )?;
        app_log_info!("✅ SQLITE: Vector functionality test completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::schema_service::SchemaService;
    use tempfile::tempdir;

    #[test]
    fn test_vector_functionality() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf()))
            .expect("Service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        schema_service
            .handle_schema_setup()
            .expect("Schema setup failed");
        let vector_service = VectorService::new(Arc::clone(&db_service_arc));
        vector_service
            .test_vector_functionality()
            .expect("Vector functionality test failed");
    }

    #[test]
    fn test_strict_text_chunk_search() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf()))
            .expect("Service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        schema_service
            .handle_schema_setup()
            .expect("Schema setup failed");
        let vector_service = VectorService::new(Arc::clone(&db_service_arc));

        let mut emb1 = vec![0.0f32; 768];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; 768];
        emb2[1] = 1.0;

        let chunks = vec![
            TextChunkBulkData {
                id: "chunk-1".to_string(),
                file_path: "/tmp/doc-a.txt".to_string(),
                parent_file_path: Some("/tmp".to_string()),
                file_name: "doc-a.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                chunk_index: 0,
                chunk_text: "alpha beta gamma".to_string(),
                char_start: 0,
                char_end: 16,
                token_estimate: 3,
                metadata: serde_json::json!({"source_type":"text_chunk"}),
                embedding: emb1.clone(),
                drive_uuid: None,
            },
            TextChunkBulkData {
                id: "chunk-2".to_string(),
                file_path: "/tmp/doc-b.txt".to_string(),
                parent_file_path: Some("/tmp".to_string()),
                file_name: "doc-b.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                chunk_index: 0,
                chunk_text: "delta epsilon zeta".to_string(),
                char_start: 0,
                char_end: 19,
                token_estimate: 3,
                metadata: serde_json::json!({"source_type":"text_chunk"}),
                embedding: emb2,
                drive_uuid: None,
            },
        ];

        let inserted = vector_service
            .store_text_chunk_vectors_bulk(chunks)
            .expect("Failed to store text chunks");
        assert_eq!(inserted, 2);

        let results = vector_service
            .search_text_chunks_strict(&emb1, 10)
            .expect("Strict text chunk search failed");
        assert!(!results.is_empty(), "Expected at least one search result");
        assert_eq!(results[0].source_type.as_deref(), Some("text_chunk"));
        assert_eq!(results[0].file_path, "/tmp/doc-a.txt");
        assert!(
            results[0].snippet.is_some(),
            "Expected snippet in text result"
        );
    }
}

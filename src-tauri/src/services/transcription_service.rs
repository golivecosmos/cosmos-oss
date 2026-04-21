use crate::app_log_info;
use crate::services::database_service::DatabaseService;
use anyhow::Result;
use chrono;
use rusqlite::OptionalExtension;
use serde_json;
use std::sync::Arc;
use uuid;

/// Service for managing audio transcriptions in the database
pub struct TranscriptionService {
    db_service: Arc<DatabaseService>,
}

impl TranscriptionService {
    /// Create a new transcription service
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    /// Store transcription result in the database
    pub fn store_transcription(
        &self,
        transcription_result: &crate::services::audio_service::TranscriptionResult,
        file_path: &str,
    ) -> Result<String> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Extract file information
        let path = std::path::Path::new(file_path);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let parent_path = path.parent().and_then(|p| p.to_str()).unwrap_or("");

        // Get file metadata
        let metadata = std::fs::metadata(file_path).ok();
        let fs_size = metadata.as_ref().map(|m| m.len() as i64);
        let last_modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

        // Serialize segments to JSON
        let segments_json = serde_json::to_string(&transcription_result.segments)
            .unwrap_or_else(|_| "[]".to_string());

        db.execute(
            "INSERT INTO transcriptions (
                id, file_path, parent_file_path, file_name, mime_type,
                duration_seconds, fs_size, created_at, last_modified, last_indexed_at,
                status, transcription_text, segments, language, model_name, confidence_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                id,
                file_path,
                parent_path,
                file_name,
                "audio/*", // Will be determined by file extension
                transcription_result.duration,
                fs_size,
                now,
                last_modified,
                now,
                "completed",
                transcription_result.text,
                segments_json,
                transcription_result.language,
                "whisper-base", // Model name
                transcription_result
                    .segments
                    .first()
                    .and_then(|s| s.confidence)
            ],
        )?;

        app_log_info!("✅ TRANSCRIPTION: Stored transcription for {}", file_name);
        Ok(id)
    }

    /// Get transcription by file path
    pub fn get_transcription_by_path(&self, file_path: &str) -> Result<Option<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let result = db
            .query_row(
                "SELECT id, file_path, file_name, duration_seconds, transcription_text,
                    segments, language, model_name, confidence_score, created_at
             FROM transcriptions
             WHERE file_path = ?",
                rusqlite::params![file_path],
                |row| {
                    let segments_json: String = row.get(5)?;
                    let segments: serde_json::Value =
                        serde_json::from_str(&segments_json).unwrap_or(serde_json::json!([]));

                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "file_path": row.get::<_, String>(1)?,
                        "file_name": row.get::<_, String>(2)?,
                        "duration_seconds": row.get::<_, Option<f64>>(3)?,
                        "transcription_text": row.get::<_, String>(4)?,
                        "segments": segments,
                        "language": row.get::<_, Option<String>>(6)?,
                        "model_name": row.get::<_, Option<String>>(7)?,
                        "confidence_score": row.get::<_, Option<f32>>(8)?,
                        "created_at": row.get::<_, String>(9)?
                    }))
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn delete_transcription_by_path(&self, file_path: &str) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let deleted = db.execute(
            "DELETE FROM transcriptions WHERE file_path = ?",
            rusqlite::params![file_path],
        )?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audio_service::TranscriptionResult;
    use crate::services::schema_service::SchemaService;
    use tempfile::tempdir;

    #[test]
    fn test_transcription_service_creation() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf()))
            .expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);

        let transcription_service = TranscriptionService::new(Arc::clone(&db_service_arc));
        assert!(transcription_service.db_service.get_db_path().is_ok());
    }

    #[test]
    fn test_transcription_storage_and_retrieval() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf()))
            .expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));

        let transcription_service = TranscriptionService::new(Arc::clone(&db_service_arc));

        // Initialize schema - ensure transcriptions table exists
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        schema_service
            .create_transcriptions_table(&db)
            .expect("Failed to create transcriptions table");
        drop(db);

        // Create a mock transcription result
        let transcription_result = TranscriptionResult {
            text: "Hello world".to_string(),
            duration: 10.5,
            language: Some("en".to_string()),
            segments: vec![
                crate::services::audio_service::TranscriptionSegment {
                    start: 0.0,
                    end: 5.0,
                    text: "Hello".to_string(),
                    confidence: Some(0.95),
                },
                crate::services::audio_service::TranscriptionSegment {
                    start: 5.0,
                    end: 10.5,
                    text: "world".to_string(),
                    confidence: Some(0.92),
                },
            ],
        };

        // Store transcription
        let file_path = "/test/audio/file.mp3";
        let id = transcription_service
            .store_transcription(&transcription_result, file_path)
            .expect("Failed to store transcription");
        assert!(!id.is_empty());

        // Retrieve transcription
        let retrieved = transcription_service
            .get_transcription_by_path(file_path)
            .expect("Failed to get transcription");
        assert!(retrieved.is_some());

        let transcription = retrieved.unwrap();
        assert_eq!(
            transcription["transcription_text"].as_str().unwrap(),
            "Hello world"
        );
        assert_eq!(transcription["duration_seconds"].as_f64().unwrap(), 10.5);
        assert_eq!(transcription["language"].as_str().unwrap(), "en");
    }
}

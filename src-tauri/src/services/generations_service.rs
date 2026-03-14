use crate::services::database_service::DatabaseService;
use crate::{app_log_debug, app_log_info};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Video Generation Record
#[derive(Debug, Serialize, Deserialize)]
pub struct Generation {
    pub id: String,
    pub user_prompt: String,
    pub json_prompt: String,
    pub source: String,
    pub generated_file_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Service for managing video generation records
pub struct GenerationsService {
    db_service: Arc<DatabaseService>,
}

impl GenerationsService {
    fn ensure_generations_schema(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        db.execute(
            "CREATE TABLE IF NOT EXISTS generations (
                id TEXT PRIMARY KEY,
                user_prompt TEXT NOT NULL,
                json_prompt TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'veo3',
                generated_file_path TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            rusqlite::params![],
        )?;

        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_generations_created_at ON generations(created_at)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_generations_source ON generations(source)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_generations_file_path ON generations(generated_file_path)",
            rusqlite::params![],
        )?;

        Ok(())
    }

    /// Create a new GenerationsService instance
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    /// Create a new generation record
    pub fn create_generation(
        &self,
        user_prompt: &str,
        json_prompt: &str,
        source: &str,
    ) -> Result<String> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let generation_id = format!("gen_{}", chrono::Utc::now().timestamp_millis());
        let now = chrono::Utc::now();

        db.execute(
            "INSERT INTO generations (id, user_prompt, json_prompt, source, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                generation_id,
                user_prompt,
                json_prompt,
                source,
                now.to_rfc3339(),
                now.to_rfc3339()
            ],
        )?;

        app_log_info!(
            "✅ GENERATIONS: Created generation record with ID: {}",
            generation_id
        );
        Ok(generation_id)
    }

    /// Update a generation record with the generated file path
    pub fn update_generation_file_path(&self, generation_id: &str, file_path: &str) -> Result<()> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let now = chrono::Utc::now();

        let rows_affected = db.execute(
            "UPDATE generations 
             SET generated_file_path = ?, updated_at = ?
             WHERE id = ?",
            rusqlite::params![file_path, now.to_rfc3339(), generation_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Generation record with ID {} not found",
                generation_id
            ));
        }

        app_log_info!(
            "✅ GENERATIONS: Updated generation {} with file path: {}",
            generation_id,
            file_path
        );
        Ok(())
    }

    /// Get all generations, ordered by creation date (newest first)
    pub fn get_all_generations(&self) -> Result<Vec<Generation>> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, user_prompt, json_prompt, source, generated_file_path, created_at, updated_at
             FROM generations
             ORDER BY created_at DESC"
        )?;

        let generations = stmt.query_map(rusqlite::params![], |row| {
            Ok(Generation {
                id: row.get(0)?,
                user_prompt: row.get(1)?,
                json_prompt: row.get(2)?,
                source: row.get(3)?,
                generated_file_path: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
                    .with_timezone(&Utc),
            })
        })?;

        let mut result = Vec::new();
        for generation in generations {
            result.push(generation?);
        }

        app_log_debug!(
            "📊 GENERATIONS: Retrieved {} generation records",
            result.len()
        );
        Ok(result)
    }

    /// Get a specific generation by ID
    pub fn get_generation_by_id(&self, generation_id: &str) -> Result<Option<Generation>> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, user_prompt, json_prompt, source, generated_file_path, created_at, updated_at
             FROM generations
             WHERE id = ?"
        )?;

        let generation = stmt.query_row(rusqlite::params![generation_id], |row| {
            Ok(Generation {
                id: row.get(0)?,
                user_prompt: row.get(1)?,
                json_prompt: row.get(2)?,
                source: row.get(3)?,
                generated_file_path: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
                    .with_timezone(&Utc),
            })
        });

        match generation {
            Ok(gen) => {
                app_log_debug!(
                    "📊 GENERATIONS: Retrieved generation record: {}",
                    generation_id
                );
                Ok(Some(gen))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                app_log_debug!(
                    "📊 GENERATIONS: Generation record not found: {}",
                    generation_id
                );
                Ok(None)
            }
            Err(e) => Err(anyhow!("Failed to get generation: {}", e)),
        }
    }

    /// Delete a generation record
    pub fn delete_generation(&self, generation_id: &str) -> Result<()> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let rows_affected = db.execute(
            "DELETE FROM generations WHERE id = ?",
            rusqlite::params![generation_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Generation record with ID {} not found",
                generation_id
            ));
        }

        app_log_info!(
            "🗑️ GENERATIONS: Deleted generation record: {}",
            generation_id
        );
        Ok(())
    }

    /// Get generation statistics
    pub fn get_generation_stats(&self) -> Result<serde_json::Value> {
        self.ensure_generations_schema()?;
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Total generations
        let total_generations: i64 = db.query_row(
            "SELECT COUNT(*) FROM generations",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        // Generations by source
        let mut source_stats = Vec::new();
        let mut stmt = db.prepare(
            "SELECT source, COUNT(*) as count
             FROM generations
             GROUP BY source",
        )?;

        let source_rows = stmt.query_map(rusqlite::params![], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in source_rows {
            let (source, count) = row?;
            source_stats.push(serde_json::json!({
                "source": source,
                "count": count
            }));
        }

        // Recent generations (last 7 days)
        let recent_generations: i64 = db.query_row(
            "SELECT COUNT(*) FROM generations 
             WHERE created_at >= datetime('now', '-7 days')",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        Ok(serde_json::json!({
            "total_generations": total_generations,
            "recent_generations": recent_generations,
            "source_stats": source_stats
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_generations_service_creation() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let generations_service = GenerationsService::new(Arc::new(db_service));
        assert!(generations_service
            .db_service
            .get_connection()
            .lock()
            .is_ok());
    }

    #[test]
    fn test_create_and_retrieve_generation() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let generations_service = GenerationsService::new(Arc::new(db_service));

        // Create a generation
        let generation_id = generations_service
            .create_generation("Test prompt", "{\"test\": \"json\"}", "veo3")
            .unwrap();

        // Retrieve the generation
        let generation = generations_service
            .get_generation_by_id(&generation_id)
            .unwrap()
            .unwrap();

        assert_eq!(generation.user_prompt, "Test prompt");
        assert_eq!(generation.json_prompt, "{\"test\": \"json\"}");
        assert_eq!(generation.source, "veo3");
        assert!(generation.generated_file_path.is_none());
    }

    #[test]
    fn test_update_generation_file_path() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let generations_service = GenerationsService::new(Arc::new(db_service));

        // Create a generation
        let generation_id = generations_service
            .create_generation("Test prompt", "{\"test\": \"json\"}", "veo3")
            .unwrap();

        // Update with file path
        generations_service
            .update_generation_file_path(&generation_id, "/path/to/video.mp4")
            .unwrap();

        // Retrieve and verify
        let generation = generations_service
            .get_generation_by_id(&generation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            generation.generated_file_path,
            Some("/path/to/video.mp4".to_string())
        );
    }

    #[test]
    fn test_get_all_generations() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let generations_service = GenerationsService::new(Arc::new(db_service));

        // Create multiple generations
        generations_service
            .create_generation("Prompt 1", "{\"test1\": \"json\"}", "veo3")
            .unwrap();
        generations_service
            .create_generation("Prompt 2", "{\"test2\": \"json\"}", "veo3")
            .unwrap();

        // Get all generations
        let generations = generations_service.get_all_generations().unwrap();
        assert_eq!(generations.len(), 2);
        assert_eq!(generations[0].user_prompt, "Prompt 2"); // Newest first
        assert_eq!(generations[1].user_prompt, "Prompt 1");
    }
}

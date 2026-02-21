use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::json;
use crate::{app_log_info, app_log_warn, app_log_error};
use crate::services::database_service::DatabaseService;
use crate::services::schema_service::SchemaService;
use crate::services::vector_service::{VectorService, ImageVectorBulkData};
use crate::services::job_queue_service::JobQueueService;
use crate::services::drive_service::DriveService;
use crate::services::transcription_service::TranscriptionService;

/// Service for managing SQLite database with vector search capabilities
///
/// This is now a pure coordinator that delegates all operations to specialized sub-services:
/// - DatabaseService: Core database connection and path management (with ConfigService)
/// - SchemaService: Schema creation, validation, and migration
/// - VectorService: Vector storage and search operations
/// - JobQueueService: Job management and queue operations
/// - DriveService: Drive tracking and management
/// - TranscriptionService: Audio transcription storage
pub struct SqliteVectorService {
    db_service: Arc<DatabaseService>,
    schema_service: Arc<SchemaService>,
    vector_service: Arc<VectorService>,
    job_queue_service: Arc<JobQueueService>,
    drive_service: Arc<DriveService>,
    transcription_service: Arc<TranscriptionService>,
}

impl SqliteVectorService {
    /// Create a new SQLite vector service
    pub fn new() -> Result<Self> {
        Self::new_with_path(None)
    }

    /// Create an in-memory SQLite vector service for testing
    /// This creates both databases in memory without any file I/O
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let db_service = DatabaseService::new_in_memory()?;
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        let vector_service = VectorService::new(Arc::clone(&db_service_arc));
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        let drive_service = DriveService::new(Arc::clone(&db_service_arc));
        let transcription_service = TranscriptionService::new(Arc::clone(&db_service_arc));
        let service = Self {
            db_service: db_service_arc,
            schema_service: schema_service_arc,
            vector_service: Arc::new(vector_service),
            job_queue_service: Arc::new(job_queue_service),
            drive_service: Arc::new(drive_service),
            transcription_service: Arc::new(transcription_service),
        };

        // Initialize schema
        service.schema_service.handle_schema_setup()?;

        Ok(service)
    }

    /// Parameterized
    pub fn new_with_path(custom_dir: Option<PathBuf>) -> Result<Self> {
        // Create database service with encryption support
        let mut db_service = DatabaseService::new_with_path(custom_dir.clone())?;

        // Initialize primary database with encryption
        db_service.initialize_database()?;
        let db_service_arc = Arc::new(db_service);

        // Initialize other services
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        let vector_service = VectorService::new(Arc::clone(&db_service_arc));
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        let drive_service = DriveService::new(Arc::clone(&db_service_arc));
        let transcription_service = TranscriptionService::new(Arc::clone(&db_service_arc));

        let service = Self {
            db_service: db_service_arc,
            schema_service: schema_service_arc,
            vector_service: Arc::new(vector_service),
            job_queue_service: Arc::new(job_queue_service),
            drive_service: Arc::new(drive_service),
            transcription_service: Arc::new(transcription_service),
        };

        // Initialize schema
        service.schema_service.handle_schema_setup()?;

        Ok(service)
    }

    // ===== DATABASE SERVICE DELEGATIONS =====

    pub fn get_db_path(&self) -> Result<(PathBuf, bool), String> {
        self.db_service.get_db_path()
    }

    /// Get the database service for use by other services
    pub fn get_database_service(&self) -> Arc<DatabaseService> {
        Arc::clone(&self.db_service)
    }

    pub fn get_schema_service(&self) -> Arc<SchemaService> {
        Arc::clone(&self.schema_service)
    }

    pub fn set_db_path(&self, new_dir: Option<&str>) -> Result<String, String> {
        app_log_info!("🔧 SQLITE_SET_PATH: Starting set_db_path");
        app_log_info!("🔧 SQLITE_SET_PATH: new_dir = {:?}", new_dir);

        // Get the new database path from the database service
        app_log_info!("🔧 SQLITE_SET_PATH: Calling db_service.set_db_path");
        let new_db_path = match self.db_service.set_db_path(new_dir) {
            Ok(path) => {
                app_log_info!("✅ SQLITE_SET_PATH: db_service.set_db_path succeeded: {}", path);
                path
            }
            Err(e) => {
                app_log_error!("❌ SQLITE_SET_PATH: db_service.set_db_path failed: {}", e);
                return Err(e);
            }
        };

        app_log_info!("✅ SQLITE_SET_PATH: set_db_path completed successfully: {}", new_db_path);
        Ok(new_db_path)
    }

    /// Get schema info for Nomic model
    pub fn get_schema_info(&self) -> Result<serde_json::Value> {
        self.schema_service.get_schema_info()
    }

    // ===== SCHEMA SERVICE DELEGATIONS =====

    /// Public method to ensure jobs table compatibility (for external use)
    pub fn ensure_jobs_table_compatibility(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        self.schema_service.ensure_jobs_table_exists(&db)
    }

    /// Recovery method for handling "jobs table not found" errors
    pub fn recover_from_jobs_table_error(&self) -> Result<()> {
        app_log_warn!("🔧 RECOVERY: Attempting to recover from jobs table error");

        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Force create the jobs table
        match self.schema_service.create_jobs_table(&db) {
            Ok(_) => {
                app_log_info!("✅ RECOVERY: Successfully created missing jobs table");
                Ok(())
            }
            Err(e) => {
                app_log_error!("❌ RECOVERY: Failed to create jobs table during recovery: {}", e);
                Err(e)
            }
        }
    }

    // ===== VECTOR SERVICE DELEGATIONS =====

    /// Test the vector search functionality
    pub fn test_vector_functionality(&self) -> Result<()> {
        self.vector_service.test_vector_functionality()
    }

    /// Store an image vector in SQLite with optional drive association
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
        self.vector_service.store_image_vector_with_drive(id, file_path, parent_file_path, file_name, mime_type, embedding, metadata, drive_uuid)
    }

    /// Search for similar vectors using manual distance calculation only (reliable)
    pub fn search_vectors(&self, query_vector: &[f32], limit: usize) -> Result<Vec<crate::models::embedding::ImageVectorDataResponse>> {
        self.vector_service.search_vectors(query_vector, limit)
    }

    /// Get all stored images
    pub fn get_all_images(&self) -> Result<Vec<crate::models::embedding::ImageVectorDataResponse>> {
        self.vector_service.get_all_images()
    }

    /// Get count of indexed images
    pub fn get_image_count(&self) -> Result<usize> {
        self.vector_service.get_image_count()
    }

    /// Recreate the virtual table with proper sqlite-vec setup
    pub fn recreate_virtual_table(&self) -> Result<()> {
        self.vector_service.recreate_virtual_table()
    }

    /// Delete an image vector from SQLite
    pub fn delete_image_vector(&self, id: &str) -> Result<()> {
        self.vector_service.delete_image_vector(id)
    }

    /// Clear the entire search index
    pub fn clear_index(&self) -> Result<()> {
        self.vector_service.clear_index()
    }

    /// Check if a file is already indexed
    pub fn file_exists(&self, file_path: &str) -> Result<bool> {
        self.vector_service.file_exists(file_path)
    }

    /// Bulk store multiple image vectors in a single transaction
    pub fn store_image_vectors_bulk(
        &self,
        vectors: Vec<ImageVectorBulkData>,
    ) -> Result<usize> {
        self.vector_service.store_image_vectors_bulk(vectors)
    }

    // ===== TRANSCRIPTION SERVICE DELEGATIONS =====

    /// Store transcription result in the database
    pub fn store_transcription(&self, transcription_result: &crate::services::audio_service::TranscriptionResult, file_path: &str) -> Result<String> {
        self.transcription_service.store_transcription(transcription_result, file_path)
    }

    /// Get transcription by file path
    pub fn get_transcription_by_path(&self, file_path: &str) -> Result<Option<serde_json::Value>> {
        self.transcription_service.get_transcription_by_path(file_path)
    }

    // ===== JOB QUEUE SERVICE DELEGATIONS =====

    /// Create a new job in the database
    pub fn create_job(&self, job_type: &str, target_path: &str, total_files: Option<usize>) -> Result<String> {
        self.job_queue_service.create_job(job_type, target_path, total_files)
    }

    /// Update job status and progress
    pub fn update_job_progress(
        &self,
        job_id: &str,
        status: &str,
        current_file: Option<&str>,
        processed: Option<usize>,
        errors: Option<&[String]>,
        failed_files: Option<&serde_json::Value>
    ) -> Result<()> {
        self.job_queue_service.update_job_progress(job_id, status, current_file, processed, errors, failed_files)
    }

    /// Get all jobs (recent first)
    pub fn get_jobs(&self, limit: Option<usize>) -> Result<Vec<serde_json::Value>> {
        self.job_queue_service.get_jobs(limit)
    }

    /// Get jobs by status
    pub fn get_jobs_by_status(&self, status: &str) -> Result<Vec<serde_json::Value>> {
        self.job_queue_service.get_jobs_by_status(status)
    }

    /// Mark job for automatic retry with exponential backoff
    pub fn schedule_job_retry(&self, job_id: &str, error_message: &str) -> Result<()> {
        self.job_queue_service.schedule_job_retry(job_id, error_message)
    }

    /// Get jobs ready for retry (past their next_retry_at time)
    pub fn get_jobs_ready_for_retry(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        self.job_queue_service.get_jobs_ready_for_retry(limit)
    }

    /// Manual user retry (resets retry count)
    pub fn manual_retry_job(&self, job_id: &str) -> Result<()> {
        self.job_queue_service.manual_retry_job(job_id)
    }

    /// Truly atomic job claiming that prevents race conditions
    pub fn claim_pending_jobs_atomic(&self, worker_id: usize, limit: usize) -> Result<Vec<serde_json::Value>> {
        self.job_queue_service.claim_pending_jobs_atomic(worker_id, limit)
    }

    /// Cancel a job (mark it as cancelled)
    pub fn cancel_job(&self, job_id: &str) -> Result<()> {
        self.job_queue_service.cancel_job(job_id)
    }

    /// Clean up old completed jobs (older than specified days)
    pub fn cleanup_old_jobs(&self, days_old: i64) -> Result<usize> {
        self.job_queue_service.cleanup_old_jobs(days_old)
    }

    /// Get a single job by ID
    pub fn get_job_by_id(&self, job_id: &str) -> Result<serde_json::Value> {
        self.job_queue_service.get_job_by_id(job_id)
    }

    /// Recover orphaned "running" jobs that have been stuck for too long
    pub fn recover_orphaned_jobs(&self, timeout_seconds: i64) -> Result<usize> {
        self.job_queue_service.recover_orphaned_jobs(timeout_seconds)
    }

    /// Clear jobs from the queue
    pub fn clear_jobs_queue(&self) -> Result<usize> {
        self.job_queue_service.clear_jobs_queue()
    }

    /// Clear all jobs regardless of status
    pub fn clear_all_jobs(&self) -> Result<usize> {
        self.job_queue_service.clear_all_jobs()
    }

    // ===== DRIVE SERVICE DELEGATIONS =====

    /// Update drive custom name and physical location
    pub fn update_drive_metadata(&self, uuid: &str, custom_name: Option<&str>, physical_location: Option<&str>) -> Result<()> {
        self.drive_service.update_drive_metadata(uuid, custom_name, physical_location)
    }

    /// Delete drive from database (with indexed files check)
    pub fn delete_drive(&self, uuid: &str) -> Result<()> {
        self.drive_service.delete_drive(uuid)
    }

    /// Get all drives with their metadata (including calculated indexed files count)
    pub fn get_all_drives(&self) -> Result<Vec<serde_json::Value>> {
        self.drive_service.get_all_drives_db()
    }

    /// Get drive information by UUID (including calculated indexed files count)
    pub fn get_drive_by_uuid(&self, uuid: &str) -> Result<Option<serde_json::Value>> {
        self.drive_service.get_drive_by_uuid_db(uuid)
    }

    /// Add a new drive to the database
    pub fn add_drive(&self, uuid: &str, name: &str, mount_path: &str, is_removable: bool) -> Result<()> {
        self.drive_service.add_drive(uuid, name, mount_path, is_removable)
    }

    /// Update drive connection status and last seen timestamp
    pub fn update_drive_status(&self, uuid: &str, status: &str, mount_path: Option<&str>) -> Result<()> {
        self.drive_service.update_drive_status_db(uuid, status, mount_path)
    }

    // ===== STATISTICS AND UTILITIES =====

    /// Get database statistics
    pub fn get_stats(&self) -> Result<serde_json::Value> {
        let total_images = self.vector_service.get_image_count()?;
        let total_video_frames = self.vector_service.get_video_frame_count()?;

        let (db_path, _) = self.db_service.get_db_path().map_err(|e| anyhow!("Failed to get database path: {}", e))?;
        let db_size = std::fs::metadata(&db_path)?.len();

        let stats = json!({
            "total_images": total_images,
            "total_video_frames": total_video_frames,
            "regular_images": total_images - total_video_frames,
            "database_size_bytes": db_size,
            "database_size_mb": (db_size as f64) / (1024.0 * 1024.0),
            "database_path": db_path.to_string_lossy(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        Ok(stats)
    }


}

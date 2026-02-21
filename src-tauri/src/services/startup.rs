use serde_json::json;
use std::sync::Arc;
use tauri::{App, Manager};

use crate::services::{
    audio_service::AudioService,
    download_service::{DownloadProgress, DownloadService, DownloadStatus},
    drive_service::DriveService,
    embedding_service::EmbeddingService,
    file_service::FileService,
    model_service::ModelService,
    sqlite_service::SqliteVectorService,
    video_service::VideoService,
};

use crate::{
    app_log_error, app_log_info, app_log_warn,
    utils::path_utils::{get_app_data_dir, migrate_app_data_if_needed},
};

use crate::commands::indexing::{get_worker_count, persistent_queue_worker};

/// Application state to be shared across commands
pub struct AppState {
    pub audio_service: Arc<tokio::sync::Mutex<AudioService>>,
    pub model_service: Arc<ModelService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub file_service: Arc<FileService>,
    pub sqlite_service: Arc<SqliteVectorService>,
    pub video_service: Arc<VideoService>,
    pub download_service: Arc<DownloadService>,
    pub drive_service: Arc<DriveService>,
    pub video_generation_status: Arc<tokio::sync::Mutex<std::collections::HashMap<String, crate::commands::video::VideoGenerationStatus>>>,
}

/// Manages application startup, service initialization, and background tasks
pub struct StartupManager {
    app_state: Option<AppState>,
}

impl StartupManager {
    pub fn new() -> Self {
        Self { app_state: None }
    }

    /// Initialize all services and return the app state
    pub async fn initialize_services(&mut self) -> Result<AppState, String> {
        app_log_info!("🚀 STARTUP: Initializing application services");

        // Initialize logger
        let _ = crate::utils::logger::LOGGER.get_or_init(|| crate::utils::logger::AppLogger::new());

        // Migrate app data directory if needed (MUST happen before any data access)
        app_log_info!("📁 STARTUP: Checking for app data migration...");
        match migrate_app_data_if_needed() {
            Ok(_) => app_log_info!("✅ STARTUP: App data migration check completed"),
            Err(e) => {
                app_log_error!("❌ STARTUP: Failed to migrate app data: {}", e);
                // Continue startup even if migration fails - the app might still work
                // with the old directory or might be a fresh install
                app_log_warn!("⚠️ STARTUP: Continuing with startup despite migration failure");
            }
        }

        // Perform startup cleanup
        self.perform_startup_cleanup().await;

        // Initialize services in dependency order
        let model_service = Arc::new(ModelService::new());
        let file_service = Arc::new(FileService::new());

        let sqlite_service = self.initialize_sqlite_service()?;
        // Get the database service from the sqlite service for the drive service
        let drive_service = Arc::new(DriveService::new(sqlite_service.get_database_service()));
        let embedding_service = Arc::new(EmbeddingService::new(
            model_service.clone(),
            sqlite_service.clone(),
            drive_service.clone(),
        ));

        let audio_service = Arc::new(tokio::sync::Mutex::new(AudioService::new()));
        let video_service = Arc::new(VideoService::new());
        let download_service = Arc::new(DownloadService::new());

        let app_state = AppState {
            audio_service,
            model_service,
            embedding_service,
            file_service,
            sqlite_service,
            video_service,
            download_service,
            drive_service,
            video_generation_status: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        };

        self.app_state = Some(app_state.clone());
        app_log_info!("✅ STARTUP: All services initialized successfully");

        Ok(app_state)
    }

    /// Initialize all services for testing (uses in-memory databases)
    #[cfg(test)]
    pub async fn initialize_services_for_testing(&mut self) -> Result<AppState, String> {
        app_log_info!("🚀 TEST STARTUP: Initializing application services for testing");

        // Initialize logger
        let _ = crate::utils::logger::LOGGER.get_or_init(|| crate::utils::logger::AppLogger::new());

        // Skip migration and cleanup for testing
        app_log_info!("📁 TEST STARTUP: Skipping migration and cleanup for testing");

        // Initialize services in dependency order using in-memory constructors
        let model_service = Arc::new(ModelService::new());
        let file_service = Arc::new(FileService::new());

        let sqlite_service = self.initialize_sqlite_service_for_testing()?;
        // Get the database service from the sqlite service for the drive service
        let drive_service = Arc::new(DriveService::new(sqlite_service.get_database_service()));
        let embedding_service = Arc::new(EmbeddingService::new(
            model_service.clone(),
            sqlite_service.clone(),
            drive_service.clone(),
        ));

        let audio_service = Arc::new(tokio::sync::Mutex::new(AudioService::new()));
        let video_service = Arc::new(VideoService::new());
        let download_service = Arc::new(DownloadService::new());

        let app_state = AppState {
            audio_service,
            model_service,
            embedding_service,
            file_service,
            sqlite_service,
            video_service,
            download_service,
            drive_service,
            video_generation_status: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        };

        self.app_state = Some(app_state.clone());
        app_log_info!("✅ TEST STARTUP: All services initialized successfully");

        Ok(app_state)
    }

    /// Setup background tasks and event handlers
    pub fn setup_background_tasks(&self, app: &App) -> Result<(), String> {
        let app_state = self.app_state.as_ref().ok_or("AppState not initialized")?;

        app_log_info!("🚀 STARTUP: Setting up background tasks");

        // Setup model download background task
        self.setup_model_download_task(app, &app_state.download_service)?;

        // Setup database schema checking
        self.setup_database_schema_check(app, &app_state.sqlite_service)?;

        // Setup background queue workers
        self.setup_background_workers(app, app_state)?;

        // Setup drive monitoring
        self.setup_drive_monitoring(app, &app_state.drive_service)?;

        // Setup security and development tools
        self.setup_security_and_devtools(app)?;

        app_log_info!("✅ STARTUP: Background tasks setup complete");
        Ok(())
    }

    /// Perform startup cleanup operations
    async fn perform_startup_cleanup(&self) {
        app_log_info!("🧹 STARTUP: Performing startup cleanup");

        // Cleanup model directories
        self.cleanup_model_directories().await;
    }

    /// Cleanup old model directories
    async fn cleanup_model_directories(&self) {
        app_log_info!("🧹 STARTUP: Cleaning up model directories");

        match get_app_data_dir() {
            Ok(app_data_dir) => {
                let models_dir = app_data_dir.join("models");
                let onnx_dir = models_dir.join("onnx");
                let xenova_dir = models_dir.join("Xenova");

                let mut removed_count = 0;
                let mut total_size_removed = 0u64;

                // Remove onnx directory
                if onnx_dir.exists() {
                    app_log_info!("🗑️ Removing onnx directory: {}", onnx_dir.display());

                    if let Ok(size) = self.calculate_directory_size(&onnx_dir) {
                        total_size_removed += size;
                    }

                    match std::fs::remove_dir_all(&onnx_dir) {
                        Ok(_) => {
                            app_log_info!("✅ Successfully removed onnx directory");
                            removed_count += 1;
                        }
                        Err(e) => {
                            app_log_error!("❌ Failed to remove onnx directory: {}", e);
                        }
                    }
                }

                // Remove Xenova directory
                if xenova_dir.exists() {
                    app_log_info!("🗑️ Removing Xenova directory: {}", xenova_dir.display());

                    if let Ok(size) = self.calculate_directory_size(&xenova_dir) {
                        total_size_removed += size;
                    }

                    match std::fs::remove_dir_all(&xenova_dir) {
                        Ok(_) => {
                            app_log_info!("✅ Successfully removed Xenova directory");
                            removed_count += 1;
                        }
                        Err(e) => {
                            app_log_error!("❌ Failed to remove Xenova directory: {}", e);
                        }
                    }
                }

                if removed_count > 0 {
                    let size_mb = total_size_removed as f64 / (1024.0 * 1024.0);
                    app_log_info!(
                        "🧹 CLEANUP COMPLETE: Removed {} directories, freed {:.2} MB",
                        removed_count,
                        size_mb
                    );
                } else {
                    app_log_info!("🧹 CLEANUP COMPLETE: No directories needed removal");
                }
            }
            Err(e) => {
                app_log_error!(
                    "❌ STARTUP CLEANUP: Failed to get app data directory: {}",
                    e
                );
            }
        }
    }

    /// Calculate directory size recursively
    fn calculate_directory_size(&self, dir: &std::path::Path) -> Result<u64, std::io::Error> {
        let mut total_size = 0u64;

        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    total_size += self.calculate_directory_size(&path)?;
                } else {
                    total_size += entry.metadata()?.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Initialize SQLite service with error handling
    pub fn initialize_sqlite_service(&self) -> Result<Arc<SqliteVectorService>, String> {
        app_log_info!("🔧 STARTUP: Initializing SQLite vector service");

        match SqliteVectorService::new() {
            Ok(service) => {
                app_log_info!("✅ SQLite vector service initialized successfully");

                // **NEW: Ensure jobs table compatibility for backwards compatibility**
                match service.ensure_jobs_table_compatibility() {
                    Ok(_) => app_log_info!("✅ STARTUP: Jobs table compatibility verified"),
                    Err(e) => {
                        app_log_warn!("⚠️ STARTUP: Jobs table compatibility check failed: {}", e);
                        app_log_warn!(
                            "📝 STARTUP: Job features may not work properly, but app will continue"
                        );
                    }
                }

                // Test the vector functionality
                if let Err(e) = service.test_vector_functionality() {
                    app_log_warn!("⚠️ SQLite vector functionality test failed: {}", e);
                } else {
                    app_log_info!("✅ SQLite vector functionality verified");
                }

                Ok(Arc::new(service))
            }
            Err(e) => {
                app_log_error!("❌ Failed to initialize SQLite vector service: {}", e);
                app_log_warn!("⚠️ App will continue with limited functionality - search features will be disabled");
                Err(format!("Failed to initialize SQLite service: {}", e))
            }
        }
    }

    /// Initialize SQLite service for testing (uses in-memory database)
    #[cfg(test)]
    pub fn initialize_sqlite_service_for_testing(&self) -> Result<Arc<SqliteVectorService>, String> {
        app_log_info!("🔧 STARTUP: Initializing SQLite vector service for testing");

        match SqliteVectorService::new_in_memory() {
            Ok(service) => {
                app_log_info!("✅ SQLite vector service initialized successfully for testing");
                Ok(Arc::new(service))
            }
            Err(e) => {
                app_log_error!("❌ Failed to initialize SQLite vector service for testing: {}", e);
                Err(format!("Failed to initialize SQLite service for testing: {}", e))
            }
        }
    }

    /// Setup background model download task
    fn setup_model_download_task(
        &self,
        app: &App,
        download_service: &Arc<DownloadService>,
    ) -> Result<(), String> {
        let download_service_clone = download_service.clone();
        let app_handle = app.handle();

        tokio::spawn(async move {
            app_log_info!("🔍 Background: Checking for required models in simplified structure...");

            if !DownloadService::are_models_available() {
                app_log_info!("📥 Background: Models missing, starting S3 download...");
                app_log_info!(
                    "🚀 Using LOCAL FILES ONLY strategy - downloading from secure model repository"
                );

                let background_progress_callback = |progress: DownloadProgress| {
                    // Log to console for debugging
                    match progress.status {
                        DownloadStatus::Downloading => {
                            app_log_info!(
                                "📥 Model download {}: {:.1}%",
                                progress.file_name,
                                progress.percentage
                            );
                        }
                        DownloadStatus::Completed => {
                            app_log_info!("✅ Model downloaded: {}", progress.file_name);
                        }
                        DownloadStatus::Failed(ref error) => {
                            app_log_error!(
                                "❌ Model download failed {}: {}",
                                progress.file_name,
                                error
                            );
                        }
                        _ => {}
                    }

                    // Emit progress event to frontend
                    if let Err(e) = app_handle.emit_all("download_progress", &progress) {
                        app_log_error!("Failed to emit download progress to frontend: {}", e);
                    }
                };

                match download_service_clone
                    .download_all_missing_models(background_progress_callback)
                    .await
                {
                    Ok(_) => {
                        app_log_info!("✅ Background: All models downloaded from secure repository successfully");
                        app_log_info!(
                            "✅ Background: Download complete - frontend will handle model loading"
                        );
                    }
                    Err(e) => {
                        app_log_error!(
                            "❌ Background: Failed to download models from secure repository: {}",
                            e
                        );
                        app_log_warn!("⚠️ Background: AI features will remain disabled until models are downloaded manually");
                    }
                }
            } else {
                app_log_info!("✅ Background: All required models are already available locally");
            }
        });

        Ok(())
    }

    /// Setup database schema checking and notifications
    fn setup_database_schema_check(
        &self,
        app: &App,
        sqlite_service: &Arc<SqliteVectorService>,
    ) -> Result<(), String> {
        let app_handle = app.handle();
        let sqlite_service_clone = sqlite_service.clone();

        tokio::spawn(async move {
            match sqlite_service_clone.get_schema_info() {
                Ok(schema_info) => {
                    app_log_info!("📋 STARTUP: Database schema status checked");

                    // Emit schema status to frontend
                    if let Err(e) = app_handle.emit_all("database_schema_status", &schema_info) {
                        app_log_error!("Failed to emit schema status: {}", e);
                    }

                    // Check if this was a fresh database creation or migration
                    if let Some(schema_data) = schema_info.get("schema_info") {
                        if let Some(schema_obj) = schema_data.as_object() {
                            if let Some(version_info) = schema_obj.get("schema_version") {
                                if let Some(version_obj) = version_info.as_object() {
                                    if let Some(updated_at) = version_obj.get("updated_at") {
                                        // Check if schema was updated recently (within last 10 seconds)
                                        if let Ok(updated_time) =
                                            chrono::DateTime::parse_from_rfc3339(
                                                updated_at.as_str().unwrap_or(""),
                                            )
                                        {
                                            let now = chrono::Utc::now();
                                            let duration = now.signed_duration_since(updated_time);

                                            if duration.num_seconds() < 10 {
                                                // Recent schema update - notify user about Nomic upgrade
                                                let notification = json!({
                                                    "type": "database_recreated",
                                                    "message": "Database recreated for Nomic model compatibility. Please re-index your files to restore search functionality.",
                                                    "action_required": true,
                                                    "details": "Your database has been upgraded to use the new Nomic embedding models with improved performance and accuracy.",
                                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                                });

                                                if let Err(e) = app_handle
                                                    .emit_all("startup_notification", &notification)
                                                {
                                                    app_log_error!("Failed to emit recreation notification: {}", e);
                                                }
                                            } else {
                                                // Existing database - check if it's using Nomic models
                                                if let Some(model_info) =
                                                    schema_obj.get("embedding_model")
                                                {
                                                    if let Some(model_obj) = model_info.as_object()
                                                    {
                                                        if let Some(model_name) =
                                                            model_obj.get("value")
                                                        {
                                                            if model_name.as_str()
                                                                == Some("nomic-embed-v1.5")
                                                            {
                                                                app_log_info!("✅ STARTUP: Database already using Nomic models");
                                                            } else {
                                                                app_log_warn!("⚠️ STARTUP: Database using old model: {}", model_name);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    app_log_error!("❌ STARTUP: Failed to check schema status: {}", e);

                    // Emit error notification
                    let notification = json!({
                        "type": "database_error",
                        "message": "Failed to verify database schema. Some features may not work correctly.",
                        "action_required": true,
                        "details": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });

                    if let Err(e) = app_handle.emit_all("startup_notification", &notification) {
                        app_log_error!("Failed to emit error notification: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Setup background queue workers
    fn setup_background_workers(&self, app: &App, app_state: &AppState) -> Result<(), String> {
        let sqlite_service_worker = app_state.sqlite_service.clone();
        let embedding_service_worker = app_state.embedding_service.clone();
        let video_service_worker = app_state.video_service.clone();
        let app_handle_worker = app.handle();

        app_log_info!(
            "🚀 STARTUP: Starting {} parallel background queue workers",
            get_worker_count()
        );

        // Spawn multiple workers for parallel processing
        for worker_id in 1..=get_worker_count() {
            let sqlite_service_clone = sqlite_service_worker.clone();
            let embedding_service_clone = embedding_service_worker.clone();
            let video_service_clone = video_service_worker.clone();
            let app_handle_clone = app_handle_worker.clone();

            tokio::spawn(async move {
                app_log_info!(
                    "🚀 STARTUP: Starting worker {} of {}",
                    worker_id,
                    get_worker_count()
                );
                persistent_queue_worker(
                    worker_id,
                    sqlite_service_clone,
                    embedding_service_clone,
                    video_service_clone,
                    app_handle_clone,
                )
                .await;
            });
        }

        Ok(())
    }

    /// Setup drive monitoring for external drives
    fn setup_drive_monitoring(
        &self,
        app: &App,
        drive_service: &Arc<DriveService>,
    ) -> Result<(), String> {
        app_log_info!("🚀 STARTUP: Setting up drive monitoring");

        let drive_service_clone = drive_service.clone();
        let app_handle = app.handle();

        tokio::spawn(async move {
            if let Err(e) = drive_service_clone.start_monitoring(app_handle).await {
                app_log_error!("❌ DRIVE: Failed to start drive monitoring: {}", e);
            }
        });

        app_log_info!("✅ STARTUP: Drive monitoring started");
        Ok(())
    }

    /// Setup security and development tools
    fn setup_security_and_devtools(&self, _app: &App) -> Result<(), String> {
        // Security: Disable devtools in production builds
        #[cfg(not(debug_assertions))]
        {
            // Disable right-click context menu in production
            let window = _app.get_window("main").ok_or("Main window not found")?;
            window.eval(r#"
                document.addEventListener('contextmenu', function(e) {
                    e.preventDefault();
                });

                // Disable F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+U
                document.addEventListener('keydown', function(e) {
                    if (e.key === 'F12' ||
                        (e.ctrlKey && e.shiftKey && (e.key === 'I' || e.key === 'J' || e.key === 'C')) ||
                        (e.ctrlKey && e.key === 'U')) {
                        e.preventDefault();
                    }
                });

                // Disable console in production
                if (typeof console !== 'undefined') {
                    console.log = function() {};
                    console.warn = function() {};
                    console.error = function() {};
                    console.info = function() {};
                    console.debug = function() {};
                    console.trace = function() {};
                    console.dir = function() {};
                    console.table = function() {};
                    console.clear = function() {};
                }
            "#).map_err(|e| format!("Failed to setup security: {}", e))?;
        }

        // Development mode: Enable devtools (only if feature is available)
        #[cfg(all(debug_assertions, feature = "dev-tools"))]
        {
            let window = _app.get_window("main").ok_or("Main window not found")?;
            window.open_devtools();
        }

        Ok(())
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            audio_service: self.audio_service.clone(),
            model_service: self.model_service.clone(),
            embedding_service: self.embedding_service.clone(),
            file_service: self.file_service.clone(),
            sqlite_service: self.sqlite_service.clone(),
            video_service: self.video_service.clone(),
            download_service: self.download_service.clone(),
            drive_service: self.drive_service.clone(),
            video_generation_status: self.video_generation_status.clone(),
        }
    }
}

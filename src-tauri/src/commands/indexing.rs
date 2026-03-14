use crate::app_log_debug;
use crate::app_log_error;
use crate::app_log_info;
use crate::app_log_warn;
use crate::constants::{
    is_supported_image_extension, is_supported_media_extension, is_supported_text_extension,
    is_supported_video_extension,
};
use crate::services::embedding_service::EmbeddingService;
use crate::services::sqlite_service::SqliteVectorService;
use crate::services::video_service::VideoService;
use crate::AppState;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{Emitter, State};
use tokio::sync::Semaphore;

// **CONSTANTS FOR BATCH PROCESSING**

/// Number of parallel workers for GPU-optimized batch processing
pub const WORKER_COUNT: usize = 4;
/// Optimal batch size for GPU memory and performance
pub const BATCH_SIZE: usize = 8;
pub const MAX_CONCURRENT_VIDEOS: usize = 3;
pub const MAX_CONCURRENT_TRANSCRIPTIONS: usize = 2;
const JOB_CREATED_EVENT_BATCH_SIZE: usize = 250;
const JOB_CREATED_EVENT_SAMPLE_LIMIT: usize = 100;

// **GLOBAL CONCURRENT LIMITS**
static VIDEO_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static TRANSCRIPTION_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static QUEUE_PROCESSING_PAUSED: AtomicBool = AtomicBool::new(false);

pub fn get_video_semaphore() -> Arc<Semaphore> {
    VIDEO_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_VIDEOS)))
        .clone()
}

pub fn get_transcription_semaphore() -> Arc<Semaphore> {
    TRANSCRIPTION_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_TRANSCRIPTIONS)))
        .clone()
}

pub fn set_queue_processing_paused(paused: bool) {
    QUEUE_PROCESSING_PAUSED.store(paused, Ordering::Relaxed);
    if paused {
        app_log_warn!("⏸️ QUEUE: Queue processing paused by user");
    } else {
        app_log_info!("▶️ QUEUE: Queue processing resumed by user");
    }
}

pub fn is_queue_processing_paused() -> bool {
    QUEUE_PROCESSING_PAUSED.load(Ordering::Relaxed)
}

async fn emit_event_with_retry<T: serde::Serialize + ?Sized>(
    app_handle: &tauri::AppHandle,
    event: &str,
    payload: &T,
) {
    const MAX_RETRIES: u32 = 3;
    for attempt in 0..MAX_RETRIES {
        match app_handle.emit(event, payload) {
            Ok(_) => return,
            Err(e) if attempt < MAX_RETRIES - 1 => {
                app_log_warn!(
                    "⚠️ EVENT: Failed to emit '{}' (attempt {}/{}): {}",
                    event,
                    attempt + 1,
                    MAX_RETRIES,
                    e
                );
                tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
            }
            Err(e) => {
                app_log_error!(
                    "❌ EVENT: Failed to emit '{}' after {} attempts: {}",
                    event,
                    MAX_RETRIES,
                    e
                );
            }
        }
    }
}

// **INDEXING PROGRESS STRUCTURES**

/// Progress structure for bulk indexing operations
#[derive(Clone, serde::Serialize)]
pub struct BulkIndexProgress {
    pub current_file: String,
    pub processed: usize,
    pub total: usize,
    pub status: String,
    pub errors: Vec<String>,
    pub directory_path: String,
    pub failed_files: Vec<FailedFileInfo>,
    // Video-specific progress fields
    pub video_progress: Option<VideoProgressInfo>,
    // Audio transcription-specific progress fields
    pub transcription_progress: Option<TranscriptionProgressInfo>,
}

/// Video-specific progress information
#[derive(Clone, serde::Serialize)]
pub struct VideoProgressInfo {
    pub current_frame: usize,
    pub total_frames: usize,
    pub processing_phase: String, // "extraction", "embedding", "storing"
    pub video_duration: f64,
    pub progress_percentage: f64,
    pub estimated_time_remaining: f64,
    pub current_operation: String,
}

/// Audio transcription-specific progress information
#[derive(Clone, serde::Serialize)]
pub struct TranscriptionProgressInfo {
    pub current_phase: String, // "validation", "conversion", "transcription", "storing"
    pub audio_duration: f64,   // Total audio duration in seconds
    pub progress_percentage: f64, // 0-100
    pub segments_processed: usize, // Number of transcription segments completed
    pub total_segments: Option<usize>, // Total expected segments (if known)
    pub estimated_time_remaining: f64, // Estimated seconds remaining
    pub current_operation: String, // Human readable current operation
    pub model_name: String,    // Which model is being used
    pub detected_language: Option<String>, // Auto-detected language
}

/// Information about failed files during indexing
#[derive(Clone, serde::Serialize, Debug)]
pub struct FailedFileInfo {
    pub name: String,
    pub path: String,
    pub error: String,
    pub error_type: String, // "temporary", "permanent", "unknown"
    pub timestamp: String,
}

// **HELPER FUNCTIONS**

/// Helper function to categorize errors
pub fn categorize_error(error: &str) -> &'static str {
    let error_lower = error.to_lowercase();

    // Temporary errors that might succeed on retry
    let temp_keywords = [
        "timeout",
        "connection",
        "network",
        "busy",
        "locked",
        "memory",
        "temporary",
        "ffmpeg not available",
        "temporarily",
        "resource temporarily",
        "try again",
        "out of memory",
        "lock timeout",
        "file creation failed",
        "server temporarily",
        "unreachable",
        "unavailable",
        "process crashed",
        "disk full",
        "refused",
    ];

    // Permanent errors unlikely to succeed on retry
    let perm_keywords = [
        "not found",
        "permission denied",
        "corrupted",
        "invalid format",
        "unsupported",
        "decode",
        "format",
        "access denied",
        "could not be found",
        "file not found",
        "invalid image",
        "invalid video",
        "malformed",
        "does not exist",
    ];

    if temp_keywords
        .iter()
        .any(|&keyword| error_lower.contains(keyword))
    {
        "temporary"
    } else if perm_keywords
        .iter()
        .any(|&keyword| error_lower.contains(keyword))
    {
        "permanent"
    } else {
        "unknown"
    }
}

fn file_name_from_path(path: &str) -> String {
    match std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
    {
        Some(name) => name.to_string(),
        None => {
            app_log_warn!("⚠️ Could not extract file name from path: {}", path);
            "unknown".to_string()
        }
    }
}

fn is_hidden_or_system_name(name: &str) -> bool {
    name.starts_with(".") || name == "DS_Store" || name == ".DS_Store" || name == "Thumbs.db"
}

/// Helper function to check if a file is already indexed
pub async fn is_file_already_indexed(
    sqlite_service: &Arc<SqliteVectorService>,
    file_path: &str,
) -> Result<bool, String> {
    // Check if file exists in SQLite database
    match sqlite_service.file_exists(file_path) {
        Ok(exists) => Ok(exists),
        Err(e) => {
            app_log_error!(
                "Database error checking if file is indexed in SQLite: {}",
                e
            );
            Err(format!(
                "Database error checking indexed state for {}: {}",
                file_path, e
            ))
        }
    }
}

// **INDEXING COMMANDS**

/// Index a single image file
#[tauri::command]
pub async fn index_image(path: String, state: State<'_, AppState>) -> Result<String, String> {
    state
        .embedding_service
        .index_image_file(&path)
        .await
        .map_err(|e| format!("Failed to index image: {}", e))
}

/// Index a single file (image or video)
#[tauri::command]
pub async fn index_file(
    app_handle: tauri::AppHandle,
    path: String,
    _name: Option<String>,
    _is_directory: Option<bool>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!("🗂️ SINGLE FILE INDEX: Starting indexing for file: {}", path);

    // Create a single-file job
    let file_name = file_name_from_path(&path);

    // Check if already indexed
    match is_file_already_indexed(&state.sqlite_service, &path).await {
        Ok(true) => {
            app_log_info!("⚠️ File already indexed, skipping: {}", path);
            return Ok("File is already indexed".to_string());
        }
        Ok(false) => {
            app_log_info!("✅ File not yet indexed, proceeding: {}", path);
        }
        Err(e) => {
            app_log_error!("❌ Could not check if file is indexed for {}: {}", path, e);
            return Err(format!("Failed to verify indexed state: {}", e));
        }
    }

    // **NEW: Create persistent job for single file**
    let job_id = match state.sqlite_service.create_job("file", &path, Some(1)) {
        Ok(id) => {
            app_log_info!("✅ JOB: Created persistent single file job: {}", id);

            // Emit job created event
            if let Ok(job_data) = state.sqlite_service.get_job_by_id(&id) {
                emit_event_with_retry(&app_handle, "job_created", &job_data).await;
            }

            id
        }
        Err(e) => {
            app_log_error!("❌ JOB: Failed to create single file job: {}", e);
            return Err(format!("Failed to create job: {}", e));
        }
    };

    // **NEW: No more progress events - database is the single source of truth**

    // Determine file type and index accordingly
    let extension = path
        .split('.')
        .last()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let mut errors = Vec::new();
    let mut failed_files = Vec::new();

    let result = if is_supported_video_extension(&extension) {
        // Index as video if FFmpeg is available
        if state.video_service.is_ffmpeg_available() {
            state
                .embedding_service
                .index_video_file_with_mode(&path, None, true, Some(app_handle.clone()))
                .await
                .map_err(|e| e.to_string())
        } else {
            app_log_warn!("Skipping video file {} - FFmpeg not available", path);
            Err("FFmpeg not available for video processing".to_string())
        }
    } else if is_supported_text_extension(&extension) {
        state
            .embedding_service
            .index_text_file(&path)
            .await
            .map_err(|e| e.to_string())
    } else {
        // Index as image
        state
            .embedding_service
            .index_image_file(&path)
            .await
            .map_err(|e| e.to_string())
    };

    let processed = match result {
        Ok(_) => {
            app_log_info!("✅ Successfully indexed: {}", path);
            1
        }
        Err(e) => {
            let error_msg = format!("Failed to index {}: {}", file_name, e);
            app_log_error!("❌ {}", error_msg);
            errors.push(error_msg);

            // Create detailed failed file info
            let failed_file = FailedFileInfo {
                name: file_name.clone(),
                path: path.clone(),
                error: e.clone(),
                error_type: categorize_error(&e).to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            failed_files.push(failed_file);

            0
        }
    };

    // **NEW: Update persistent job completion status**
    let final_status = if processed > 0 { "completed" } else { "failed" };
    let failed_files_json = serde_json::to_value(&failed_files).unwrap_or(serde_json::json!([]));

    if let Err(e) = state.sqlite_service.update_job_progress(
        &job_id,
        final_status,
        Some(&format!(
            "{}",
            if processed > 0 { "Completed" } else { "Failed" }
        )),
        Some(processed),
        Some(&errors),
        Some(&failed_files_json),
    ) {
        app_log_error!("❌ JOB: Failed to update single file job completion: {}", e);
    } else {
        // Emit job completion event
        if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
            emit_event_with_retry(&app_handle, "job_completed", &job_data).await;
        }
    }

    // **NEW: No more final progress events - database has the latest state**

    if processed > 0 {
        Ok("File indexed successfully".to_string())
    } else {
        Err(errors
            .first()
            .cloned()
            .unwrap_or_else(|| "Failed to index file".to_string()))
    }
}

/// Index a video file with specific parameters
#[tauri::command]
pub async fn index_video(
    app_handle: tauri::AppHandle,
    path: String,
    fps: Option<f32>,
    fast_mode: Option<bool>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let fast_mode = fast_mode.unwrap_or(true); // Default to optimized mode

    app_log_info!("🎬 VIDEO INDEX: Starting video indexing for: {}", path);

    // Ensure FFmpeg is available
    if !state.video_service.is_ffmpeg_available() {
        return Err(
            "FFmpeg not available. Please install FFmpeg to enable video processing.".to_string(),
        );
    }

    // Send initial progress
    let file_name = file_name_from_path(&path);
    let initial_progress = BulkIndexProgress {
        current_file: file_name.clone(),
        processed: 0,
        total: 1,
        status: "starting_video".to_string(),
        errors: Vec::new(),
        directory_path: path.clone(),
        failed_files: Vec::new(),
        video_progress: Some(VideoProgressInfo {
            current_frame: 0,
            total_frames: 0,
            processing_phase: "initializing".to_string(),
            video_duration: 0.0,
            progress_percentage: 0.0,
            estimated_time_remaining: 0.0,
            current_operation: "Starting video analysis...".to_string(),
        }),
        transcription_progress: None,
    };

    emit_event_with_retry(&app_handle, "bulk_index_progress", &initial_progress).await;

    // Index the video file with the specified frame rate and mode
    match state
        .embedding_service
        .index_video_file_with_mode(&path, fps, fast_mode, Some(app_handle.clone()))
        .await
    {
        Ok(video_id) => {
            // Send completion progress
            let final_progress = BulkIndexProgress {
                current_file: "✅ Video indexing completed".to_string(),
                processed: 1,
                total: 1,
                status: "completed".to_string(),
                errors: Vec::new(),
                directory_path: path.clone(),
                failed_files: Vec::new(),
                video_progress: Some(VideoProgressInfo {
                    current_frame: 0,
                    total_frames: 0,
                    processing_phase: "completed".to_string(),
                    video_duration: 0.0,
                    progress_percentage: 100.0,
                    estimated_time_remaining: 0.0,
                    current_operation: "Video indexing completed successfully".to_string(),
                }),
                transcription_progress: None,
            };

            emit_event_with_retry(&app_handle, "bulk_index_progress", &final_progress).await;

            app_log_info!(
                "✅ VIDEO INDEX: Successfully indexed video: {} (ID: {})",
                path,
                video_id
            );
            Ok(format!("Video indexed successfully with ID: {}", video_id))
        }
        Err(e) => {
            // Send error progress
            let error_progress = BulkIndexProgress {
                current_file: "❌ Video indexing failed".to_string(),
                processed: 0,
                total: 1,
                status: "error".to_string(),
                errors: vec![e.to_string()],
                directory_path: path.clone(),
                failed_files: vec![FailedFileInfo {
                    name: file_name,
                    path: path.clone(),
                    error: e.to_string(),
                    error_type: "video_processing".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }],
                video_progress: None,
                transcription_progress: None,
            };

            emit_event_with_retry(&app_handle, "bulk_index_progress", &error_progress).await;

            app_log_error!("❌ VIDEO INDEX: Failed to index {}: {}", path, e);
            Err(format!("Failed to index video: {}", e))
        }
    }
}

/// Index all files in a directory using queue-based approach
#[tauri::command]
pub async fn index_directory(
    app_handle: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!(
        "🗂️ QUEUE INDEX: Starting queue-based indexing for directory: {}",
        path
    );

    // Check if the path is a directory
    if !state.file_service.is_directory(&path) {
        return Err("Path is not a directory".to_string());
    }

    // Stream filesystem traversal to avoid materializing large recursive directory vectors.
    let mut created_jobs = 0;
    let mut total_files = 0;
    let mut skipped_files = 0;
    let mut batch_created_jobs = 0;
    let mut job_emit_sample: Vec<String> = Vec::new();

    let walker = walkdir::WalkDir::new(&path)
        .follow_links(true)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !is_hidden_or_system_name(&name)
        });

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                app_log_warn!("⚠️ QUEUE INDEX: Skipping unreadable entry: {}", e);
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path().to_string_lossy().to_string();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let extension = file_name
            .split('.')
            .last()
            .unwrap_or_default()
            .to_lowercase();

        if !is_supported_media_extension(&extension) {
            continue;
        }

        total_files += 1;

        // Check if file is already indexed (skip job creation)
        let already_indexed = match is_file_already_indexed(&state.sqlite_service, &file_path).await
        {
            Ok(result) => result,
            Err(e) => {
                app_log_error!("❌ QUEUE: Aborting directory indexing due to indexed-state check failure for {}: {}", file_path, e);
                return Err(format!(
                    "Failed checking indexed state for {}: {}",
                    file_path, e
                ));
            }
        };

        if already_indexed {
            app_log_info!("⏭️ Skipping already indexed file: {}", file_path);
            skipped_files += 1;
            continue;
        }

        // Create indexing job for this file
        match state.sqlite_service.create_job("file", &file_path, Some(1)) {
            Ok(job_id) => {
                app_log_info!(
                    "✅ QUEUE: Created indexing job {} for file: {}",
                    job_id,
                    file_name
                );
                created_jobs += 1;
                batch_created_jobs += 1;
                if job_emit_sample.len() < JOB_CREATED_EVENT_SAMPLE_LIMIT {
                    job_emit_sample.push(job_id);
                }

                if batch_created_jobs >= JOB_CREATED_EVENT_BATCH_SIZE {
                    let jobs_batch_payload = serde_json::json!({
                        "total_jobs": batch_created_jobs,
                        "directory_path": path.clone(),
                        "sample_job_ids": job_emit_sample,
                        "total_jobs_created_so_far": created_jobs
                    });
                    emit_event_with_retry(&app_handle, "jobs_batch_created", &jobs_batch_payload)
                        .await;
                    batch_created_jobs = 0;
                    job_emit_sample = Vec::new();
                }
            }
            Err(e) => {
                app_log_error!(
                    "❌ QUEUE: Failed to create indexing job for {}: {}",
                    file_path,
                    e
                );
            }
        }
    }

    app_log_info!("🗂️ QUEUE INDEX: Found {} indexable files", total_files);
    if total_files == 0 {
        return Ok("No indexable files found in directory".to_string());
    }

    app_log_info!(
        "📋 QUEUE: Created {} indexing jobs ({} skipped)",
        created_jobs,
        skipped_files
    );

    if batch_created_jobs > 0 {
        let jobs_batch_payload = serde_json::json!({
            "total_jobs": batch_created_jobs,
            "directory_path": path.clone(),
            "sample_job_ids": job_emit_sample,
            "total_jobs_created_so_far": created_jobs
        });
        emit_event_with_retry(&app_handle, "jobs_batch_created", &jobs_batch_payload).await;
    }

    if created_jobs > 0 {
        Ok(format!("Created {} indexing jobs in queue. Jobs will be processed automatically by background worker.", created_jobs))
    } else {
        Ok("All files were already indexed, no jobs created.".to_string())
    }
}

#[cfg(test)]
mod indexing_command_tests {
    use super::is_hidden_or_system_name;

    #[test]
    fn hidden_and_system_names_are_filtered() {
        assert!(is_hidden_or_system_name(".git"));
        assert!(is_hidden_or_system_name(".DS_Store"));
        assert!(is_hidden_or_system_name("DS_Store"));
        assert!(is_hidden_or_system_name("Thumbs.db"));
        assert!(!is_hidden_or_system_name("photo.jpg"));
    }
}

/// Transcribe audio in a single file
#[tauri::command]
pub async fn transcribe_file(
    app_handle: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!(
        "🎤 TRANSCRIBE FILE: Starting transcription for file: {}",
        path
    );

    // Check if file exists
    if !std::path::Path::new(&path).exists() {
        return Err("File not found".to_string());
    }

    // Check if file has audio content
    let extension = path.split('.').last().unwrap_or_default().to_lowercase();
    let has_audio = [
        "wav", "mp3", "m4a", "flac", "ogg", "aac", "wma", "mp4", "mov", "avi", "webm", "mkv",
        "flv", "wmv", "m4v",
    ]
    .contains(&extension.as_str());

    if !has_audio {
        return Err("File does not contain audio content".to_string());
    }

    // Create transcription job
    let file_name = file_name_from_path(&path);
    match state
        .sqlite_service
        .create_job("transcription", &path, Some(1))
    {
        Ok(job_id) => {
            app_log_info!(
                "🎤 QUEUE: Created transcription job {} for file: {}",
                job_id,
                file_name
            );

            // Emit job created event
            if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
                emit_event_with_retry(&app_handle, "job_created", &job_data).await;
            }

            Ok(format!("Created transcription job for {}. Job will be processed automatically by background worker.", file_name))
        }
        Err(e) => {
            app_log_error!(
                "❌ QUEUE: Failed to create transcription job for {}: {}",
                path,
                e
            );
            Err(format!("Failed to create transcription job: {}", e))
        }
    }
}

// **BACKGROUND WORKER FUNCTIONS**

/// Persistent background queue worker with batch processing
pub async fn persistent_queue_worker(
    worker_id: usize,
    sqlite_service: Arc<SqliteVectorService>,
    embedding_service: Arc<EmbeddingService>,
    video_service: Arc<VideoService>,
    app_handle: tauri::AppHandle,
) {
    app_log_info!(
        "🔄 WORKER {}: Starting persistent background queue worker with batch processing",
        worker_id
    );

    let mut idle_cycles = 0;
    let mut consecutive_errors = 0;
    let mut maintenance_cycles: u64 = 0;

    loop {
        if is_queue_processing_paused() {
            if worker_id == 1 {
                app_log_info!("⏸️ WORKERS: Processing paused, waiting...");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }

        maintenance_cycles = maintenance_cycles.saturating_add(1);

        // Exponential backoff on errors
        if consecutive_errors > 0 {
            let delay = std::cmp::min(30, 2_u64.pow(consecutive_errors as u32));
            app_log_warn!(
                "⚠️ WORKER {}: Backing off for {}s after {} consecutive errors",
                worker_id,
                delay,
                consecutive_errors
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        }

        // Periodic maintenance: stagger recovery across workers and run cleanup from worker 1.
        // Runs roughly every 5 minutes per worker (60 * 5s idle polling interval).
        if maintenance_cycles % 60 == (worker_id as u64 % 60) {
            if let Err(e) = sqlite_service.recover_orphaned_jobs(600) {
                app_log_warn!(
                    "⚠️ WORKER {}: Failed to recover orphaned jobs: {}",
                    worker_id,
                    e
                );
            }
        }
        if worker_id == 1 && maintenance_cycles % 720 == 0 {
            if let Err(e) = sqlite_service.cleanup_old_jobs(7) {
                app_log_warn!("⚠️ WORKER {}: Failed to cleanup old jobs: {}", worker_id, e);
            }
        }

        // **FIXED: Atomic job claiming to prevent race conditions**
        let claimed_jobs = match sqlite_service.claim_pending_jobs_atomic(worker_id, BATCH_SIZE) {
            Ok(jobs) => {
                if jobs.len() > 0 {
                    app_log_info!(
                        "🔄 WORKER {}: Atomically claimed {} pending jobs",
                        worker_id,
                        jobs.len()
                    );

                    // Log job IDs for debugging race conditions
                    let job_ids: Vec<String> = jobs
                        .iter()
                        .filter_map(|job| job["id"].as_str().map(|s| s.to_string()))
                        .collect();
                    app_log_debug!("🔄 WORKER {}: Claimed job IDs: {:?}", worker_id, job_ids);
                } else {
                    // Only check for pending jobs occasionally to avoid spam
                    if idle_cycles % 20 == 0 && worker_id == 1 {
                        // Every ~2 minutes
                        match sqlite_service.get_jobs_by_status("pending") {
                            Ok(pending_jobs) => {
                                if pending_jobs.len() > 0 {
                                    app_log_info!("📋 WORKER {}: {} pending jobs exist but none claimed (normal)", worker_id, pending_jobs.len());
                                }
                            }
                            Err(e) => {
                                app_log_error!(
                                    "❌ WORKER {}: Failed to check pending jobs count: {}",
                                    worker_id,
                                    e
                                );
                            }
                        }
                    }
                }
                jobs
            }
            Err(e) => {
                app_log_error!(
                    "❌ WORKER {}: Failed to claim pending jobs: {}",
                    worker_id,
                    e
                );
                consecutive_errors += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Retry-eligible jobs are included in atomic pending-claim query
        // via `next_retry_at <= now`; avoid a second non-atomic claim path.

        // If pause was requested while claiming jobs, release them back to pending immediately.
        if is_queue_processing_paused() && !claimed_jobs.is_empty() {
            app_log_info!(
                "⏸️ WORKER {}: Releasing {} claimed jobs back to pending due to pause request",
                worker_id,
                claimed_jobs.len()
            );
            for job in &claimed_jobs {
                if let Some(job_id) = job["id"].as_str() {
                    let _ = sqlite_service.update_job_progress(
                        job_id,
                        "pending",
                        Some("Paused by user"),
                        None,
                        None,
                        None,
                    );
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            continue;
        }

        if claimed_jobs.is_empty() {
            idle_cycles += 1;
            consecutive_errors = 0; // Reset error counter when successfully checking (even if empty)

            // Log occasional status updates when idle (only from worker 1 to avoid spam)
            if idle_cycles % 60 == 0 && worker_id == 1 {
                // Every 5 minutes when idle
                app_log_info!(
                    "💤 WORKERS: Idle, waiting for jobs... ({}m idle)",
                    idle_cycles / 12
                );
            }

            // Sleep longer when no jobs are available
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        } else {
            // Reset counters when we have jobs
            if idle_cycles > 0 || consecutive_errors > 0 {
                app_log_info!(
                    "🔄 WORKER {}: Processing {} jobs after {}m idle",
                    worker_id,
                    claimed_jobs.len(),
                    idle_cycles / 12
                );
                idle_cycles = 0;
                consecutive_errors = 0;
            }
        }

        // **🚀 GPU-OPTIMIZED BATCH PROCESSING**
        // Collect jobs by type for efficient processing
        let mut batch_jobs = Vec::with_capacity(BATCH_SIZE); // Image jobs for GPU batching
        let mut video_jobs = Vec::new(); // Video jobs for individual processing
        let mut audio_jobs = Vec::new(); // Audio jobs for transcription
        let mut text_jobs = Vec::new(); // Text jobs for chunk embedding

        // Collect jobs for batch processing
        for job in claimed_jobs {
            let job_id = job["id"].as_str().unwrap_or("unknown");
            let file_path = job["target_path"].as_str().unwrap_or("");
            let job_type = job["job_type"].as_str().unwrap_or("file");

            // Double check job status - it might have been cancelled
            if job["status"] != "running" {
                app_log_warn!(
                    "⏭️ WORKER {}: Skipping job {} because status is {}",
                    worker_id,
                    job_id,
                    job["status"].as_str().unwrap_or("unknown")
                );
                continue;
            }

            // Skip and retry when the source path is temporarily unavailable (e.g. disconnected drive).
            if !std::path::Path::new(file_path).exists() {
                let missing_msg = format!(
                    "Source path unavailable (possibly disconnected drive): {}",
                    file_path
                );
                app_log_warn!("⚠️ WORKER {}: {}", worker_id, missing_msg);
                if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &missing_msg) {
                    app_log_error!(
                        "❌ WORKER {}: Failed to schedule retry for missing path: {}",
                        worker_id,
                        retry_err
                    );
                    let _ = sqlite_service.update_job_progress(
                        job_id,
                        "failed",
                        Some(&missing_msg),
                        Some(0),
                        Some(&vec![missing_msg.clone()]),
                        None,
                    );
                }
                continue;
            }

            let file_name = file_name_from_path(file_path);
            let extension = file_name
                .split('.')
                .last()
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            // Route jobs based on job type first, then file extension
            if job_type == "transcription" {
                // All transcription jobs go to audio processing pipeline
                audio_jobs.push((job_id.to_string(), file_path.to_string()));
            } else if is_supported_video_extension(&extension) {
                // Video files for indexing go to video processing
                video_jobs.push((job_id.to_string(), file_path.to_string()));
            } else if ["wav", "mp3", "m4a", "flac", "ogg", "aac", "wma"]
                .contains(&extension.as_str())
            {
                // Pure audio files for indexing would go to audio processing, but we don't index audio files directly yet
                // For now, skip audio-only files that aren't transcription jobs
                app_log_warn!("⏭️ WORKER {}: Skipping pure audio file {} - audio indexing not implemented yet", worker_id, file_name);
                let _ = sqlite_service.update_job_progress(
                    job_id,
                    "completed",
                    Some("Skipped - audio indexing not implemented"),
                    None,
                    None,
                    None,
                );
            } else if is_supported_text_extension(&extension) {
                text_jobs.push((job_id.to_string(), file_path.to_string()));
            } else if is_supported_image_extension(&extension) {
                // Default to image batch processing for supported image files
                batch_jobs.push((job_id.to_string(), file_path.to_string()));
            } else {
                app_log_warn!(
                    "⏭️ WORKER {}: Unsupported file extension for semantic indexing: {}",
                    worker_id,
                    extension
                );
                let _ = sqlite_service.update_job_progress(
                    job_id,
                    "failed",
                    Some("Unsupported file extension for semantic indexing"),
                    Some(0),
                    Some(&vec![format!("Unsupported extension: {}", extension)]),
                    None,
                );
            }
        }

        // **NEW: Limit concurrent video processing to prevent resource conflicts**
        if video_jobs.len() > MAX_CONCURRENT_VIDEOS {
            app_log_warn!(
                "⚠️ WORKER {}: Limiting video processing to {} concurrent videos (found {})",
                worker_id,
                MAX_CONCURRENT_VIDEOS,
                video_jobs.len()
            );

            // Keep only the first MAX_CONCURRENT_VIDEOS videos, return others to pending
            let videos_to_process = video_jobs
                .drain(..MAX_CONCURRENT_VIDEOS)
                .collect::<Vec<_>>();
            let videos_to_return = video_jobs;

            // Return excess videos to pending status
            for (job_id, _) in videos_to_return {
                if let Err(e) = sqlite_service.update_job_progress(
                    &job_id,
                    "pending",
                    Some("Returned to queue due to concurrent video limit"),
                    None,
                    None,
                    None,
                ) {
                    app_log_error!(
                        "❌ WORKER {}: Failed to return video job to pending: {}",
                        worker_id,
                        e
                    );
                } else {
                    app_log_info!(
                        "🔄 WORKER {}: Returned video job {} to pending queue",
                        worker_id,
                        job_id
                    );
                }
            }

            video_jobs = videos_to_process;
        }

        // **Log job processing summary**
        let total_jobs_to_process =
            batch_jobs.len() + video_jobs.len() + audio_jobs.len() + text_jobs.len();
        if total_jobs_to_process > 0 {
            app_log_info!("📋 WORKER {}: Processing {} jobs: {} images, {} videos, {} transcriptions, {} text files",
                worker_id, total_jobs_to_process, batch_jobs.len(), video_jobs.len(), audio_jobs.len(), text_jobs.len());
        }

        // **Process image batch with GPU acceleration**
        if !batch_jobs.is_empty() {
            app_log_info!(
                "🚀 WORKER {}: Processing GPU-accelerated batch of {} images",
                worker_id,
                batch_jobs.len()
            );
            let batch_start = std::time::Instant::now();

            // Update job progress for all batch jobs
            for (job_id, _file_path) in &batch_jobs {
                if let Err(e) = sqlite_service.update_job_progress(
                    job_id,
                    "running",
                    Some(&format!(
                        "Processing in GPU batch with {} other images",
                        batch_jobs.len() - 1
                    )),
                    None,
                    None,
                    None,
                ) {
                    app_log_error!(
                        "❌ WORKER {}: Failed to update job progress for {}: {}",
                        worker_id,
                        job_id,
                        e
                    );
                }

                // Emit progress update
                if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                    emit_event_with_retry(&app_handle, "job_updated", &job_data).await;
                }
            }

            // Process batch using GPU-accelerated embedding service
            match embedding_service
                .index_image_files_batch(batch_jobs.iter().map(|(_, path)| path.clone()).collect())
                .await
            {
                Ok(batch_result) => {
                    let batch_time = batch_start.elapsed();
                    app_log_info!(
                        "🚀 WORKER {}: GPU batch completed in {:.2}ms - {} successful, {} failed",
                        worker_id,
                        batch_time.as_millis(),
                        batch_result.successful,
                        batch_result.failed
                    );

                    let error_by_path: HashMap<String, String> = batch_result
                        .failed_details
                        .iter()
                        .map(|(path, err)| (path.clone(), err.clone()))
                        .collect();
                    let failed_paths: HashSet<String> = error_by_path.keys().cloned().collect();

                    // Update each job based on file-path keyed failures rather than positional assumptions.
                    for (job_id, file_path) in &batch_jobs {
                        if !failed_paths.contains(file_path) {
                            if let Err(e) = sqlite_service.update_job_progress(
                                job_id,
                                "completed",
                                Some("GPU batch completed"),
                                Some(1),
                                None,
                                None,
                            ) {
                                app_log_error!(
                                    "❌ WORKER {}: Failed to mark job as completed: {}",
                                    worker_id,
                                    e
                                );
                            } else if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                                emit_event_with_retry(&app_handle, "job_completed", &job_data)
                                    .await;
                            }
                            continue;
                        }

                        let error_msg_owned = error_by_path
                            .get(file_path)
                            .cloned()
                            .unwrap_or_else(|| "Unknown batch processing error".to_string());
                        let error_msg = &error_msg_owned;

                        let error_type = categorize_error(error_msg);
                        if error_type == "temporary" {
                            if let Err(retry_err) =
                                sqlite_service.schedule_job_retry(job_id, error_msg)
                            {
                                app_log_error!(
                                    "❌ WORKER {}: Failed to schedule retry for batch job: {}",
                                    worker_id,
                                    retry_err
                                );
                                let _ = sqlite_service.update_job_progress(
                                    job_id,
                                    "failed",
                                    Some(&format!("GPU batch failed: {}", error_msg)),
                                    Some(0),
                                    Some(&vec![error_msg.clone()]),
                                    None,
                                );
                            }
                        } else if let Err(e) = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&format!("GPU batch failed: {}", error_msg)),
                            Some(0),
                            Some(&vec![error_msg.clone()]),
                            None,
                        ) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to mark job as failed: {}",
                                worker_id,
                                e
                            );
                        }

                        if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                            emit_event_with_retry(&app_handle, "job_updated", &job_data).await;
                        }
                    }
                }
                Err(e) => {
                    app_log_error!("❌ WORKER {}: Batch processing failed: {}", worker_id, e);
                    consecutive_errors += 1;

                    // Mark all jobs as failed
                    for (job_id, _) in &batch_jobs {
                        if let Err(e) = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&format!("Batch processing failed: {}", e)),
                            Some(0),
                            Some(&vec![e.to_string()]),
                            None,
                        ) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to mark job as failed: {}",
                                worker_id,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Process text jobs with strict text-chunk indexing
        for (job_id, file_path) in &text_jobs {
            let file_name = file_name_from_path(file_path);
            app_log_info!(
                "📝 WORKER {}: Processing text file {}",
                worker_id,
                file_name
            );

            let result = embedding_service
                .index_text_file(file_path)
                .await
                .map_err(|e| e.to_string());
            match result {
                Ok(_) => {
                    if let Err(e) = sqlite_service.update_job_progress(
                        job_id,
                        "completed",
                        Some("Text indexing completed"),
                        Some(1),
                        None,
                        None,
                    ) {
                        app_log_error!(
                            "❌ WORKER {}: Failed to mark text job as completed: {}",
                            worker_id,
                            e
                        );
                    } else if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                        emit_event_with_retry(&app_handle, "job_completed", &job_data).await;
                    }
                }
                Err(e) => {
                    let error_type = categorize_error(&e);
                    if error_type == "temporary" {
                        if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &e) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to schedule retry for text job: {}",
                                worker_id,
                                retry_err
                            );
                            let _ = sqlite_service.update_job_progress(
                                job_id,
                                "failed",
                                Some(&format!("Text indexing failed: {}", e)),
                                Some(0),
                                Some(&vec![e.clone()]),
                                None,
                            );
                        }
                    } else if let Err(update_err) = sqlite_service.update_job_progress(
                        job_id,
                        "failed",
                        Some(&format!("Text indexing failed: {}", e)),
                        Some(0),
                        Some(&vec![e.clone()]),
                        None,
                    ) {
                        app_log_error!(
                            "❌ WORKER {}: Failed to update failed text job: {}",
                            worker_id,
                            update_err
                        );
                    }

                    if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                        emit_event_with_retry(&app_handle, "job_updated", &job_data).await;
                    }
                }
            }
        }

        // **Process video jobs individually (still need FFmpeg)**
        for (job_id, file_path) in &video_jobs {
            let file_name = file_name_from_path(file_path);
            app_log_info!(
                "🔄 WORKER {}: Processing video {} individually",
                worker_id,
                file_name
            );

            // **FIXED: Use global semaphore to limit concurrent video processing**
            let video_semaphore = get_video_semaphore();
            let _permit = match video_semaphore.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    app_log_error!(
                        "❌ WORKER {}: Failed to acquire video semaphore: {}",
                        worker_id,
                        e
                    );
                    let sem_err = "Failed to acquire video processing permit".to_string();
                    if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &sem_err) {
                        app_log_error!("❌ WORKER {}: Failed to schedule retry after video semaphore error: {}", worker_id, retry_err);
                        let _ = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&sem_err),
                            Some(0),
                            Some(&vec![sem_err.clone()]),
                            None,
                        );
                    }
                    continue;
                }
            };

            app_log_info!(
                "🎬 WORKER {}: Acquired video processing permit for {} (permits: {}/{})",
                worker_id,
                file_name,
                MAX_CONCURRENT_VIDEOS - video_semaphore.available_permits(),
                MAX_CONCURRENT_VIDEOS
            );

            let video_start = std::time::Instant::now();

            app_log_info!(
                "🎬 WORKER {}: Starting in-memory video processing for {}",
                worker_id,
                file_name
            );

            let result = if video_service.is_ffmpeg_available() {
                embedding_service
                    .index_video_file_with_mode(file_path, None, true, Some(app_handle.clone()))
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Err("FFmpeg not available for video processing".to_string())
            };

            let video_time = video_start.elapsed();
            app_log_info!(
                "⏱️ WORKER {}: Video {} processed in {:.2}ms",
                worker_id,
                file_name,
                video_time.as_millis()
            );

            // Update video job result
            match result {
                Ok(_) => {
                    consecutive_errors = 0; // Reset on success
                    if let Err(e) = sqlite_service.update_job_progress(
                        job_id,
                        "completed",
                        Some("Video completed"),
                        Some(1),
                        None,
                        None,
                    ) {
                        app_log_error!(
                            "❌ WORKER {}: Failed to mark video job as completed: {}",
                            worker_id,
                            e
                        );
                    } else {
                        if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                            emit_event_with_retry(&app_handle, "job_completed", &job_data).await;
                        }
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;

                    // **NEW: Check if error is retryable and schedule automatic retry**
                    let error_type = categorize_error(&e);
                    if error_type == "temporary" {
                        // Schedule automatic retry with exponential backoff
                        if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &e) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to schedule retry for video job: {}",
                                worker_id,
                                retry_err
                            );
                            // Fallback: mark as failed
                            let _ = sqlite_service.update_job_progress(
                                job_id,
                                "failed",
                                Some(&format!("Video failed: {}", e)),
                                Some(0),
                                Some(&vec![e.clone()]),
                                None,
                            );
                        }
                    } else {
                        // Permanent error - mark as failed immediately
                        if let Err(e) = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&format!("Video failed: {}", e)),
                            Some(0),
                            Some(&vec![e.clone()]),
                            None,
                        ) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to mark video job as failed: {}",
                                worker_id,
                                e
                            );
                        }
                    }
                }
            }

            app_log_info!(
                "🎬 WORKER {}: Released video processing permit for {}",
                worker_id,
                file_name
            );
        }

        // **Process audio jobs for transcription individually**
        for (job_id, file_path) in &audio_jobs {
            let file_name = file_name_from_path(file_path);
            app_log_info!(
                "🔄 WORKER {}: Processing audio {} for transcription",
                worker_id,
                file_name
            );

            // **Use global semaphore to limit concurrent transcription processing**
            let transcription_semaphore = get_transcription_semaphore();
            let _permit = match transcription_semaphore.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    app_log_error!(
                        "❌ WORKER {}: Failed to acquire transcription semaphore: {}",
                        worker_id,
                        e
                    );
                    let sem_err = "Failed to acquire transcription processing permit".to_string();
                    if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &sem_err) {
                        app_log_error!("❌ WORKER {}: Failed to schedule retry after transcription semaphore error: {}", worker_id, retry_err);
                        let _ = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&sem_err),
                            Some(0),
                            Some(&vec![sem_err.clone()]),
                            None,
                        );
                    }
                    continue;
                }
            };

            app_log_info!(
                "🎤 WORKER {}: Acquired transcription processing permit for {} (permits: {}/{})",
                worker_id,
                file_name,
                MAX_CONCURRENT_TRANSCRIPTIONS - transcription_semaphore.available_permits(),
                MAX_CONCURRENT_TRANSCRIPTIONS
            );

            let transcription_start = std::time::Instant::now();

            // Update job status to processing with transcription progress
            if let Err(e) = sqlite_service.update_job_progress(
                job_id,
                "running",
                Some("Starting transcription"),
                None,
                None,
                None,
            ) {
                app_log_error!(
                    "❌ WORKER {}: Failed to update transcription job status: {}",
                    worker_id,
                    e
                );
            }

            app_log_info!(
                "🎤 WORKER {}: Starting audio transcription for {}",
                worker_id,
                file_name
            );

            // Get audio service from app state through embedding service
            let result = if let Some(audio_service_arc) = &embedding_service.audio_service {
                // Convert string path to Path
                let audio_path = std::path::Path::new(file_path);

                // Get mutable access to audio service
                let mut audio_service = audio_service_arc.lock().await;

                // Ensure Whisper model is loaded before transcription
                let model_check_result = if !audio_service.is_available() {
                    match crate::services::download_service::DownloadService::get_whisper_status() {
                        crate::services::download_service::WhisperStatus::Ready => {
                            app_log_info!(
                                "🎤 WORKER {}: Whisper model files found, loading model...",
                                worker_id
                            );
                            match audio_service.load_model().await {
                                Ok(_) => {
                                    app_log_info!(
                                        "✅ WORKER {}: Whisper model loaded successfully",
                                        worker_id
                                    );
                                    Ok(())
                                }
                                Err(e) => {
                                    app_log_error!(
                                        "❌ WORKER {}: Failed to load Whisper model: {}",
                                        worker_id,
                                        e
                                    );
                                    Err(format!("Failed to load Whisper model: {}", e))
                                }
                            }
                        }
                        _ => {
                            let error =
                                "Whisper model not ready. Please wait for download to complete.";
                            app_log_error!("❌ WORKER {}: {}", worker_id, error);
                            Err(error.to_string())
                        }
                    }
                } else {
                    Ok(()) // Model already loaded
                };

                // Only proceed if model is ready
                match model_check_result {
                    Ok(_) => {
                        // Validate audio file first
                        match audio_service.validate_audio_file(audio_path) {
                            Ok(_) => {
                                app_log_info!(
                                    "✅ WORKER {}: Audio file validation passed for {}",
                                    worker_id,
                                    file_name
                                );

                                // Update progress: validation complete
                                let _ = sqlite_service.update_job_progress(
                                    job_id,
                                    "running",
                                    Some("Validation complete, starting transcription"),
                                    None,
                                    None,
                                    None,
                                );

                                // Perform transcription
                                audio_service
                                    .transcribe_file(audio_path)
                                    .await
                                    .map_err(|e| e.to_string())
                            }
                            Err(e) => {
                                app_log_error!(
                                    "❌ WORKER {}: Audio validation failed for {}: {}",
                                    worker_id,
                                    file_name,
                                    e
                                );
                                Err(format!("Audio validation failed: {}", e))
                            }
                        }
                    }
                    Err(model_error) => {
                        // Model loading failed, return the error
                        Err(model_error)
                    }
                }
            } else {
                app_log_error!(
                    "❌ WORKER {}: AudioService not available for transcription",
                    worker_id
                );
                Err("AudioService not available".to_string())
            };

            let transcription_time = transcription_start.elapsed();
            app_log_info!(
                "⏱️ WORKER {}: Audio {} transcription processed in {:.2}ms",
                worker_id,
                file_name,
                transcription_time.as_millis()
            );

            // Update audio job result
            match result {
                Ok(transcription_result) => {
                    consecutive_errors = 0; // Reset on success

                    app_log_info!(
                        "✅ WORKER {}: Transcription completed for {}: {} segments, language: {:?}",
                        worker_id,
                        file_name,
                        transcription_result.segments.len(),
                        transcription_result.language
                    );

                    // Store transcription result (for now just log, later we'll integrate with database)
                    app_log_debug!("📝 TRANSCRIPTION: {}", transcription_result.text);

                    if let Err(e) = sqlite_service.update_job_progress(
                        job_id,
                        "completed",
                        Some("Transcription completed"),
                        Some(1),
                        None,
                        None,
                    ) {
                        app_log_error!(
                            "❌ WORKER {}: Failed to mark transcription job as completed: {}",
                            worker_id,
                            e
                        );
                    } else {
                        if let Ok(job_data) = sqlite_service.get_job_by_id(job_id) {
                            emit_event_with_retry(&app_handle, "job_completed", &job_data).await;
                        }
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;

                    app_log_error!(
                        "❌ WORKER {}: Transcription failed for {}: {}",
                        worker_id,
                        file_name,
                        e
                    );

                    // **Check if error is retryable and schedule automatic retry**
                    let error_type = categorize_error(&e);
                    if error_type == "temporary" {
                        // Schedule automatic retry with exponential backoff
                        if let Err(retry_err) = sqlite_service.schedule_job_retry(job_id, &e) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to schedule retry for transcription job: {}",
                                worker_id,
                                retry_err
                            );
                            // Fallback: mark as failed
                            let _ = sqlite_service.update_job_progress(
                                job_id,
                                "failed",
                                Some(&format!("Transcription failed: {}", e)),
                                Some(0),
                                Some(&vec![e.to_string()]),
                                None,
                            );
                        }
                    } else {
                        // Permanent error - mark as failed immediately
                        if let Err(update_err) = sqlite_service.update_job_progress(
                            job_id,
                            "failed",
                            Some(&format!("Transcription failed: {}", e)),
                            Some(0),
                            Some(&vec![e.to_string()]),
                            None,
                        ) {
                            app_log_error!(
                                "❌ WORKER {}: Failed to mark transcription job as failed: {}",
                                worker_id,
                                update_err
                            );
                        }
                    }
                }
            }

            app_log_info!(
                "🎤 WORKER {}: Released transcription processing permit for {}",
                worker_id,
                file_name
            );
        }

        // Adaptive delay between batches based on workload
        let delay = if consecutive_errors > 0 {
            // Longer delay if we had errors
            std::time::Duration::from_millis(500)
        } else if !batch_jobs.is_empty() || !video_jobs.is_empty() || !audio_jobs.is_empty() {
            // Short delay if we just processed jobs
            std::time::Duration::from_millis(50)
        } else {
            // Medium delay if we had no jobs
            std::time::Duration::from_millis(200)
        };

        tokio::time::sleep(delay).await;
    }
}

// **PUBLIC CONSTANTS FOR MAIN.RS**

/// Get the worker count for background processing
pub fn get_worker_count() -> usize {
    WORKER_COUNT
}

/// Clear search index
#[tauri::command]
pub async fn clear_search_index(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!("🗑️ CLEAR INDEX: Starting index cleanup");

    // Clear SQLite index
    match state.sqlite_service.clear_index() {
        Ok(_) => {
            app_log_info!("✅ CLEAR INDEX: Successfully cleared SQLite index");

            // Emit event to notify frontend
            emit_event_with_retry(&app_handle, "index_cleared", &()).await;

            Ok("Successfully cleared search index".to_string())
        }
        Err(e) => {
            app_log_error!("❌ CLEAR INDEX: Failed to clear SQLite index: {}", e);
            Err(format!("Failed to clear search index: {}", e))
        }
    }
}

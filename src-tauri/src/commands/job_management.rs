use tauri::{AppHandle, Emitter, State};
use crate::services::startup::AppState;
use crate::{app_log_info, app_log_error, app_log_warn};
use crate::commands::indexing::{is_queue_processing_paused, set_queue_processing_paused};

// **SIMPLIFIED JOB MANAGEMENT COMMANDS**

/// Get all jobs with optional limit and status filter
#[tauri::command]
pub async fn get_jobs(
    limit: Option<usize>, 
    status: Option<String>,
    state: State<'_, AppState>
) -> Result<Vec<serde_json::Value>, String> {
    let result = match &status {
        Some(status_filter) => {
            app_log_info!("📋 JOBS: Getting jobs with status: {}", status_filter);
            state.sqlite_service.get_jobs_by_status(status_filter)
        }
        None => {
            app_log_info!("📋 JOBS: Getting jobs with limit: {:?}", limit);
            state.sqlite_service.get_jobs(limit)
        }
    };
    
    match result {
        Ok(jobs) => {
            app_log_info!("✅ JOBS: Retrieved {} jobs", jobs.len());
            Ok(jobs)
        }
        Err(e) => {
            let error_str = e.to_string();
            
            // **NEW: Handle "jobs table not found" error specifically**
            if error_str.contains("no such table: jobs") || error_str.contains("jobs") && error_str.contains("not found") {
                app_log_warn!("⚠️ JOBS: Detected missing jobs table error, attempting recovery");
                
                match state.sqlite_service.recover_from_jobs_table_error() {
                    Ok(_) => {
                        app_log_info!("✅ JOBS: Recovery successful, retrying get_jobs");
                        
                        // Retry the original operation
                        let retry_result = match &status {
                            Some(status_filter) => state.sqlite_service.get_jobs_by_status(status_filter),
                            None => state.sqlite_service.get_jobs(limit),
                        };
                        
                        match retry_result {
                            Ok(jobs) => {
                                app_log_info!("✅ JOBS: Retrieved {} jobs after recovery", jobs.len());
                                Ok(jobs)
                            }
                            Err(retry_e) => {
                                app_log_error!("❌ JOBS: Failed to get jobs even after recovery: {}", retry_e);
                                Err(format!("Failed to get jobs after recovery: {}", retry_e))
                            }
                        }
                    }
                    Err(recovery_e) => {
                        app_log_error!("❌ JOBS: Recovery failed: {}", recovery_e);
                        Err(format!("Jobs table missing and recovery failed: {}. Please restart the application.", recovery_e))
                    }
                }
            } else {
                app_log_error!("❌ JOBS: Failed to get jobs: {}", e);
                Err(format!("Failed to get jobs: {}", e))
            }
        }
    }
}

/// Simplified job queue management (no user-facing retry complexity)
#[tauri::command]
pub async fn manage_job_queue(
    app_handle: AppHandle,
    action: String,
    job_id: Option<String>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 BACKEND: manage_job_queue called with action: '{}', job_id: {:?}", action, job_id);
    match action.as_str() {
        "status" => {
            app_log_info!("🔍 QUEUE STATUS: Checking queue status");
            match state.sqlite_service.get_jobs_by_status("pending") {
                Ok(pending_jobs) => {
                    let running_jobs = state.sqlite_service.get_jobs_by_status("running").unwrap_or_default();
                    let completed_jobs = state.sqlite_service.get_jobs_by_status("completed").unwrap_or_default();
                    let failed_jobs = state.sqlite_service.get_jobs_by_status("failed").unwrap_or_default();
                    
                    Ok(serde_json::json!({
                        "pending": pending_jobs.len(),
                        "running": running_jobs.len(),
                        "completed": completed_jobs.len(),
                        "failed": failed_jobs.len(),
                        "paused": is_queue_processing_paused()
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ QUEUE STATUS: Failed to get queue status: {}", e);
                    Err(format!("Failed to get queue status: {}", e))
                }
            }
        }
        "stop" => {
            app_log_warn!("⏸️ QUEUE: Stop requested via manage_job_queue");
            set_queue_processing_paused(true);
            if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
                "paused": true,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })) {
                app_log_error!("Failed to emit queue_processing_changed event: {}", e);
            }
            Ok(serde_json::json!({
                "message": "Queue processing paused",
                "paused": true
            }))
        }
        "resume" => {
            app_log_info!("▶️ QUEUE: Resume requested via manage_job_queue");
            set_queue_processing_paused(false);
            if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
                "paused": false,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })) {
                app_log_error!("Failed to emit queue_processing_changed event: {}", e);
            }
            Ok(serde_json::json!({
                "message": "Queue processing resumed",
                "paused": false
            }))
        }
        "clear" => {
            app_log_info!("🗑️ CLEAR QUEUE: Clearing all pending and running jobs");
            // Pause workers first so they stop pulling new work while queue is cleared.
            set_queue_processing_paused(true);
            
            // Clean up temp files for all pending/running jobs before clearing
            if let Ok(pending_jobs) = state.sqlite_service.get_jobs_by_status("pending") {
                for job in pending_jobs {
                    let job_path = job["target_path"].as_str().unwrap_or("");
                    let job_id_str = job["id"].as_str().unwrap_or("unknown");
                    if !job_path.is_empty() {
                        cleanup_job_temp_files(job_id_str, job_path).await;
                    }
                }
            }
            
            if let Ok(running_jobs) = state.sqlite_service.get_jobs_by_status("running") {
                for job in running_jobs {
                    let job_path = job["target_path"].as_str().unwrap_or("");
                    let job_id_str = job["id"].as_str().unwrap_or("unknown");
                    if !job_path.is_empty() {
                        cleanup_job_temp_files(job_id_str, job_path).await;
                    }
                }
            }
            
            match state.sqlite_service.clear_jobs_queue() {
                Ok(deleted_count) => {
                    app_log_info!("✅ CLEAR QUEUE: Successfully cleared {} jobs", deleted_count);
                    
                    // Emit event to notify frontend that jobs were cleared
                    if let Err(e) = app_handle.emit("jobs_cleared", serde_json::json!({
                        "deleted_count": deleted_count,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })) {
                        app_log_error!("Failed to emit jobs_cleared event: {}", e);
                    }
                    if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
                        "paused": true,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })) {
                        app_log_error!("Failed to emit queue_processing_changed event: {}", e);
                    }
                    
                    Ok(serde_json::json!({
                        "message": format!("Cleared {} jobs from the queue", deleted_count),
                        "deleted_count": deleted_count
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ CLEAR QUEUE: Failed to clear queue: {}", e);
                    Err(format!("Failed to clear queue: {}", e))
                }
            }
        }
        "clear_all" => {
            app_log_warn!("🗑️ CLEAR ALL JOBS: Deleting all jobs regardless of status");
            set_queue_processing_paused(true);

            match state.sqlite_service.clear_all_jobs() {
                Ok(deleted_count) => {
                    app_log_info!("✅ CLEAR ALL JOBS: Successfully deleted {} jobs", deleted_count);

                    if let Err(e) = app_handle.emit("jobs_cleared", serde_json::json!({
                        "deleted_count": deleted_count,
                        "scope": "all",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })) {
                        app_log_error!("Failed to emit jobs_cleared event: {}", e);
                    }
                    if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
                        "paused": true,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })) {
                        app_log_error!("Failed to emit queue_processing_changed event: {}", e);
                    }

                    Ok(serde_json::json!({
                        "message": format!("Deleted all {} jobs", deleted_count),
                        "deleted_count": deleted_count,
                        "scope": "all",
                        "paused": true
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ CLEAR ALL JOBS: Failed to clear all jobs: {}", e);
                    Err(format!("Failed to clear all jobs: {}", e))
                }
            }
        }
        "cancel" => {
            let job_id = job_id.ok_or("Job ID required for cancel action")?;
            app_log_info!("🛑 JOBS: Cancelling job: {}", job_id);
            
            // Clean up temp files for this job before cancelling
            if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
                let job_path = job_data["target_path"].as_str().unwrap_or("");
                if !job_path.is_empty() {
                    cleanup_job_temp_files(&job_id, job_path).await;
                }
            }
            
            match state.sqlite_service.cancel_job(&job_id) {
                Ok(_) => {
                    app_log_info!("✅ JOBS: Job cancelled successfully: {}", job_id);
                    
                    // Emit job cancelled event
                    if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
                        if let Err(e) = app_handle.emit("job_updated", &job_data) {
                            app_log_error!("Failed to emit job cancellation event: {}", e);
                        }
                    }
                    
                    Ok(serde_json::json!({
                        "message": format!("Job {} cancelled successfully", job_id),
                        "job_id": job_id
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ JOBS: Failed to cancel job: {}", e);
                    Err(format!("Failed to cancel job: {}", e))
                }
            }
        }
        "retry" => {
            let job_id = job_id.ok_or("Job ID required for retry action")?;
            app_log_info!("🔄 JOBS: Manual retry requested for job: {}", job_id);
            
            // Use manual retry method which resets retry count
            match state.sqlite_service.manual_retry_job(&job_id) {
                Ok(_) => {
                    app_log_info!("✅ JOBS: Job {} reset for manual retry", job_id);
                    
                    // Emit job updated event
                    if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
                        if let Err(e) = app_handle.emit("job_updated", &job_data) {
                            app_log_error!("Failed to emit job retry event: {}", e);
                        }
                    }
                    
                    Ok(serde_json::json!({
                        "message": "Job scheduled for retry",
                        "job_id": job_id
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ JOBS: Failed to retry job: {}", e);
                    Err(format!("Failed to retry job: {}", e))
                }
            }
        }
        _ => Err(format!("Unknown queue action: {}", action))
    }
}

#[tauri::command]
pub async fn set_queue_processing(
    app_handle: AppHandle,
    paused: bool,
) -> Result<serde_json::Value, String> {
    if paused {
        app_log_warn!("⏸️ QUEUE: set_queue_processing called with paused=true");
    } else {
        app_log_info!("▶️ QUEUE: set_queue_processing called with paused=false");
    }

    set_queue_processing_paused(paused);
    if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
        "paused": paused,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })) {
        app_log_error!("Failed to emit queue_processing_changed event: {}", e);
    }

    Ok(serde_json::json!({
        "paused": paused
    }))
}

#[tauri::command]
pub async fn stop_and_clear_queue(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    app_log_warn!("🛑🧹 QUEUE: stop_and_clear_queue requested");
    set_queue_processing_paused(true);

    match state.sqlite_service.clear_jobs_queue() {
        Ok(deleted_count) => {
            if let Err(e) = app_handle.emit("jobs_cleared", serde_json::json!({
                "deleted_count": deleted_count,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })) {
                app_log_error!("Failed to emit jobs_cleared event: {}", e);
            }
            if let Err(e) = app_handle.emit("queue_processing_changed", serde_json::json!({
                "paused": true,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })) {
                app_log_error!("Failed to emit queue_processing_changed event: {}", e);
            }

            Ok(serde_json::json!({
                "message": format!("Queue paused and {} jobs cleared", deleted_count),
                "deleted_count": deleted_count,
                "paused": true
            }))
        }
        Err(e) => Err(format!("Failed to clear queue: {}", e)),
    }
}

/// Dedicated retry command to avoid parameter serialization issues
#[tauri::command]
pub async fn retry_job(
    app_handle: AppHandle,
    job_id: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 BACKEND: retry_job called with job_id: '{}'", job_id);
    
    // Use manual retry method which resets retry count
    match state.sqlite_service.manual_retry_job(&job_id) {
        Ok(_) => {
            app_log_info!("✅ JOBS: Job {} reset for manual retry", job_id);
            
            // Emit job updated event
            if let Ok(job_data) = state.sqlite_service.get_job_by_id(&job_id) {
                if let Err(e) = app_handle.emit("job_updated", &job_data) {
                    app_log_error!("Failed to emit job retry event: {}", e);
                }
            }
            
            Ok(serde_json::json!({
                "message": "Job scheduled for retry",
                "job_id": job_id
            }))
        }
        Err(e) => {
            app_log_error!("❌ JOBS: Failed to retry job: {}", e);
            Err(format!("Failed to retry job: {}", e))
        }
    }
}

/// Bulk job operations (simplified - no user-facing retry)
#[tauri::command]
pub async fn bulk_job_operations(
    action: String,
    days_old: Option<i64>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    match action.as_str() {
        "cancel_all_pending" => {
            app_log_info!("🛑 CANCEL ALL PENDING JOBS: Cancelling all pending jobs");
            
            match state.sqlite_service.get_jobs_by_status("pending") {
                Ok(pending_jobs) => {
                    app_log_info!("📋 PENDING JOBS: {} jobs pending", pending_jobs.len());
                    
                    let mut success_count = 0;
                    for job in pending_jobs {
                        // Clean up temp files before cancelling
                        let job_path = job["target_path"].as_str().unwrap_or("");
                        let job_id_str = job["id"].as_str().unwrap_or("unknown");
                        
                        if !job_path.is_empty() {
                            cleanup_job_temp_files(job_id_str, job_path).await;
                        }
                        
                        match state.sqlite_service.cancel_job(job_id_str) {
                            Ok(_) => {
                                app_log_info!("✅ CANCEL SUCCESS: Job {} cancelled", job_id_str);
                                success_count += 1;
                            }
                            Err(e) => {
                                app_log_error!("❌ CANCEL FAILURE: Failed to cancel job {}: {}", job_id_str, e);
                            }
                        }
                    }
                    
                    Ok(serde_json::json!({
                        "message": format!("Successfully cancelled {} pending jobs", success_count),
                        "cancelled_count": success_count
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ CANCEL ALL PENDING JOBS: Failed to get pending jobs: {}", e);
                    Err(format!("Failed to get pending jobs: {}", e))
                }
            }
        }
        "cleanup_old" => {
            let days = days_old.unwrap_or(30);
            app_log_info!("🧹 JOBS: Cleaning up jobs older than {} days", days);
            
            match state.sqlite_service.cleanup_old_jobs(days) {
                Ok(count) => {
                    app_log_info!("✅ JOBS: Cleaned up {} old jobs", count);
                    Ok(serde_json::json!({
                        "message": format!("Cleaned up {} old jobs", count),
                        "cleaned_count": count
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ JOBS: Failed to clean up old jobs: {}", e);
                    Err(format!("Failed to clean up old jobs: {}", e))
                }
            }
        }
        "cleanup_temp_files" => {
            app_log_info!("🧹 TEMP CLEANUP: Starting comprehensive temp file cleanup");
            
            match cleanup_all_temp_files().await {
                Ok(count) => {
                    app_log_info!("✅ TEMP CLEANUP: Cleaned up {} temp directories", count);
                    Ok(serde_json::json!({
                        "message": format!("Cleaned up {} temp directories", count),
                        "cleaned_count": count
                    }))
                }
                Err(e) => {
                    app_log_error!("❌ TEMP CLEANUP: Failed to clean temp files: {}", e);
                    Err(format!("Failed to clean temp files: {}", e))
                }
            }
        }
        _ => Err(format!("Unknown bulk operation: {}", action))
    }
}

// **AUTOMATIC RETRY & CLEANUP HELPERS**

/// Clean up temp files for a specific job
async fn cleanup_job_temp_files(job_id: &str, job_path: &str) {
    app_log_info!("🧹 CLEANUP: Cleaning temp files for job: {}", job_id);
    
    // Get temp directory for this job/file
    if let Ok(temp_dir) = get_job_temp_directory(job_id, job_path) {
        if temp_dir.exists() {
            match std::fs::remove_dir_all(&temp_dir) {
                Ok(_) => {
                    app_log_info!("✅ CLEANUP: Removed temp directory: {}", temp_dir.display());
                }
                Err(e) => {
                    app_log_error!("❌ CLEANUP: Failed to remove temp directory {}: {}", temp_dir.display(), e);
                }
            }
        }
    }
}

/// Clean up all orphaned temp files
async fn cleanup_all_temp_files() -> Result<usize, String> {
    use crate::utils::path_utils::get_app_data_dir;
    
    let app_data_dir = get_app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let temp_base_dir = app_data_dir.join("temp");
    
    if !temp_base_dir.exists() {
        return Ok(0);
    }
    
    let mut cleaned_count = 0;
    
    // Clean up video processing temp directories
    if let Ok(entries) = std::fs::read_dir(&temp_base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_dir() {
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Clean video processing temp dirs older than 1 hour
                if dir_name.starts_with("video_processing_") {
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let age = std::time::SystemTime::now()
                                .duration_since(modified)
                                .unwrap_or_default();
                            
                            if age > std::time::Duration::from_secs(3600) { // 1 hour
                                match std::fs::remove_dir_all(&path) {
                                    Ok(_) => {
                                        app_log_info!("🧹 CLEANUP: Removed old temp dir: {}", path.display());
                                        cleaned_count += 1;
                                    }
                                    Err(e) => {
                                        app_log_error!("❌ CLEANUP: Failed to remove temp dir {}: {}", path.display(), e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(cleaned_count)
}

/// Get temp directory for a specific job
fn get_job_temp_directory(job_id: &str, job_path: &str) -> Result<std::path::PathBuf, String> {
    use crate::utils::path_utils::get_app_data_dir;
    
    let app_data_dir = get_app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    // For video files, use the same pattern as video_service.rs
    if job_path.ends_with(".mp4") || job_path.ends_with(".mov") || job_path.ends_with(".avi") {
        // Use process ID as a simple way to identify temp dirs
        // In production, we might want to use job_id instead
        let temp_dir = app_data_dir.join("temp").join(format!("video_processing_{}", std::process::id()));
        Ok(temp_dir)
    } else {
        // For other files, create a job-specific temp dir
        let temp_dir = app_data_dir.join("temp").join(format!("job_{}", job_id));
        Ok(temp_dir)
    }
} 

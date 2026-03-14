use crate::services::watched_folder_service::{WatchedFolder, WatchedFolderScanResult};
use crate::services::startup::AppState;
use crate::{app_log_error, app_log_info};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn add_watched_folder(
    path: String,
    recursive: Option<bool>,
    auto_transcribe_videos: Option<bool>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<WatchedFolder, String> {
    let recursive = recursive.unwrap_or(true);
    let auto_transcribe_videos = auto_transcribe_videos.unwrap_or(true);

    match state.watched_folder_service.add_watched_folder(
        &path,
        recursive,
        auto_transcribe_videos,
    ) {
        Ok(folder) => {
            let _ = app_handle.emit("watched_folder_updated", &folder);
            app_log_info!("👀 WATCH COMMAND: Added watched folder {}", folder.path);

            // Kick off an immediate forced backfill scan so users see indexing start right away.
            let folder_id = folder.id.clone();
            let watched_folder_service = state.watched_folder_service.clone();
            let sqlite_service = state.sqlite_service.clone();
            let scan_handle = app_handle.clone();
            tokio::spawn(async move {
                if let Err(e) = watched_folder_service
                    .scan_watched_folder_by_id(&folder_id, &sqlite_service, Some(&scan_handle), true)
                    .await
                {
                    app_log_error!(
                        "❌ WATCH COMMAND: Immediate scan failed for watched folder {}: {}",
                        folder_id,
                        e
                    );
                }
            });

            Ok(folder)
        }
        Err(e) => {
            app_log_error!("❌ WATCH COMMAND: Failed to add watched folder: {}", e);
            Err(format!("Failed to add watched folder: {}", e))
        }
    }
}

#[tauri::command]
pub async fn remove_watched_folder(
    folder_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.watched_folder_service.remove_watched_folder(&folder_id) {
        Ok(_) => {
            let _ = app_handle.emit(
                "watched_folder_updated",
                serde_json::json!({
                    "id": folder_id,
                    "removed": true
                }),
            );
            Ok("Watched folder removed".to_string())
        }
        Err(e) => {
            app_log_error!("❌ WATCH COMMAND: Failed to remove watched folder: {}", e);
            Err(format!("Failed to remove watched folder: {}", e))
        }
    }
}

#[tauri::command]
pub async fn list_watched_folders(state: State<'_, AppState>) -> Result<Vec<WatchedFolder>, String> {
    state
        .watched_folder_service
        .list_watched_folders()
        .map_err(|e| format!("Failed to list watched folders: {}", e))
}

#[tauri::command]
pub async fn set_watched_folder_enabled(
    folder_id: String,
    enabled: bool,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state
        .watched_folder_service
        .set_watched_folder_enabled(&folder_id, enabled)
    {
        Ok(_) => {
            if let Ok(Some(folder)) = state.watched_folder_service.get_watched_folder_by_id(&folder_id)
            {
                let _ = app_handle.emit("watched_folder_updated", folder);
            }

            if enabled {
                let watched_folder_service = state.watched_folder_service.clone();
                let sqlite_service = state.sqlite_service.clone();
                let scan_handle = app_handle.clone();
                let folder_id_for_scan = folder_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = watched_folder_service
                        .scan_watched_folder_by_id(
                            &folder_id_for_scan,
                            &sqlite_service,
                            Some(&scan_handle),
                            true,
                        )
                        .await
                    {
                        app_log_error!(
                            "❌ WATCH COMMAND: Immediate scan after resume failed for {}: {}",
                            folder_id_for_scan,
                            e
                        );
                    }
                });
            }
            Ok("Watched folder updated".to_string())
        }
        Err(e) => Err(format!("Failed to update watched folder: {}", e)),
    }
}

#[tauri::command]
pub async fn trigger_watched_folder_backfill(
    folder_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<WatchedFolderScanResult, String> {
    state
        .watched_folder_service
        .scan_watched_folder_by_id(
            &folder_id,
            &state.sqlite_service,
            Some(&app_handle),
            true,
        )
        .await
        .map_err(|e| format!("Failed to trigger watched folder backfill: {}", e))
}

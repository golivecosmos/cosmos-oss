use crate::services::drive_service::{DriveInfo, DriveStatus};
use crate::services::startup::AppState;
use crate::{app_log_info, app_log_debug, app_log_warn, app_log_error};
use tauri::State;

/// Get all currently connected external drives
#[tauri::command]
pub async fn get_connected_drives(
    state: State<'_, AppState>
) -> Result<Vec<DriveInfo>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting connected drives");
    
    match state.drive_service.detect_connected_drives().await {
        Ok(drives) => {
            app_log_info!("✅ DRIVE COMMAND: Found {} connected drives", drives.len());
            
            // Update database with current drive statuses
            for drive in &drives {
                if let Err(e) = state.sqlite_service.update_drive_status(
                    &drive.uuid, 
                    "connected", 
                    Some(&drive.mount_path)
                ) {
                    app_log_warn!("⚠️ DRIVE: Failed to update status for {}: {}", drive.name, e);
                }
            }
            
            Ok(drives)
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to get connected drives: {}", e);
            Err(format!("Failed to get connected drives: {}", e))
        }
    }
}

/// Get information about a specific drive
#[tauri::command]
pub async fn get_drive_info(
    drive_uuid: String,
    state: State<'_, AppState>
) -> Result<Option<DriveInfo>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting info for drive: {}", drive_uuid);
    
    match state.drive_service.get_drive_info(&drive_uuid).await {
        Some(drive_info) => {
            app_log_info!("✅ DRIVE COMMAND: Found drive info for: {}", drive_info.name);
            Ok(Some(drive_info))
        }
        None => {
            app_log_warn!("⚠️ DRIVE COMMAND: Drive not found: {}", drive_uuid);
            Ok(None)
        }
    }
}

/// Force refresh the drive list
#[tauri::command]
pub async fn refresh_drives(
    state: State<'_, AppState>
) -> Result<Vec<DriveInfo>, String> {
    app_log_info!("🔄 DRIVE COMMAND: Force refreshing drives");
    
    match state.drive_service.refresh_drives().await {
        Ok(drives) => {
            app_log_info!("✅ DRIVE COMMAND: Refreshed {} drives", drives.len());
            Ok(drives)
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to refresh drives: {}", e);
            Err(format!("Failed to refresh drives: {}", e))
        }
    }
}

/// Get all drives (connected and disconnected)
#[tauri::command]
pub async fn get_all_drives(
    state: State<'_, AppState>
) -> Result<Vec<DriveInfo>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting all drives");
    
    let drives = state.drive_service.get_all_drives().await;
    app_log_info!("✅ DRIVE COMMAND: Found {} total drives", drives.len());
    
    Ok(drives)
}

/// Update drive status
#[tauri::command]
pub async fn update_drive_status(
    drive_uuid: String,
    status: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    app_log_info!("🔄 DRIVE COMMAND: Updating drive {} status to: {}", drive_uuid, status);
    
    let drive_status = match status.as_str() {
        "connected" => DriveStatus::Connected,
        "disconnected" => DriveStatus::Disconnected,
        "indexing" => DriveStatus::Indexing,
        _ => return Err(format!("Invalid drive status: {}", status)),
    };
    
    match state.drive_service.update_drive_status(&drive_uuid, drive_status).await {
        Ok(_) => {
            app_log_info!("✅ DRIVE COMMAND: Updated drive status successfully");
            Ok(())
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to update drive status: {}", e);
            Err(format!("Failed to update drive status: {}", e))
        }
    }
}

/// Get drive information for a specific path
#[tauri::command]
pub async fn get_drive_for_path(
    path: String,
    state: State<'_, AppState>
) -> Result<Option<DriveInfo>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting drive info for path: {}", path);
    
    match state.drive_service.get_drive_for_path(&path).await {
        Some(drive_info) => {
            app_log_info!("✅ DRIVE COMMAND: Found drive {} for path", drive_info.name);
            Ok(Some(drive_info))
        }
        None => {
            app_log_debug!("📝 DRIVE COMMAND: No drive found for path: {}", path);
            Ok(None)
        }
    }
}

/// Test if a path is on an external drive
#[tauri::command]
pub async fn is_path_on_external_drive(
    path: String,
    state: State<'_, AppState>
) -> Result<bool, String> {
    app_log_debug!("🔍 DRIVE COMMAND: Checking if path is on external drive: {}", path);
    
    match state.drive_service.get_drive_for_path(&path).await {
        Some(drive_info) => {
            let is_external = drive_info.is_removable;
            app_log_debug!("✅ DRIVE COMMAND: Path is on {} drive: {}", 
                if is_external { "external" } else { "internal" }, drive_info.name);
            Ok(is_external)
        }
        None => {
            app_log_debug!("📝 DRIVE COMMAND: Path is not on any tracked drive");
            Ok(false)
        }
    }
}

/// Get list of files indexed from a specific drive
#[tauri::command]
pub async fn get_drive_indexed_files(
    drive_uuid: String,
    _state: State<'_, AppState>
) -> Result<Vec<String>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting indexed files for drive: {}", drive_uuid);
    
    // TODO: Implement this method in SqliteVectorService
    // For now, return empty list
    app_log_warn!("⚠️ DRIVE COMMAND: get_drive_indexed_files not yet implemented");
    Ok(vec![])
}

/// Get drive statistics
#[tauri::command]
pub async fn get_drive_stats(
    drive_uuid: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    app_log_info!("📊 DRIVE COMMAND: Getting stats for drive: {}", drive_uuid);
    
    match state.drive_service.get_drive_info(&drive_uuid).await {
        Some(drive_info) => {
            let stats = serde_json::json!({
                "uuid": drive_info.uuid,
                "name": drive_info.name,
                "total_space": drive_info.total_space,
                "free_space": drive_info.free_space,
                "indexed_files_count": drive_info.indexed_files_count,
                "total_size_indexed": drive_info.total_size_indexed,
                "status": drive_info.status,
                "last_seen": drive_info.last_seen,
            });
            
            app_log_info!("✅ DRIVE COMMAND: Got stats for drive: {}", drive_info.name);
            Ok(stats)
        }
        None => {
            app_log_warn!("⚠️ DRIVE COMMAND: Drive not found for stats: {}", drive_uuid);
            Err(format!("Drive not found: {}", drive_uuid))
        }
    }
}

/// Update drive custom name and physical location
#[tauri::command]
pub async fn update_drive_metadata(
    drive_uuid: String,
    custom_name: Option<String>,
    physical_location: Option<String>,
    state: State<'_, AppState>
) -> Result<(), String> {
    app_log_info!("🔄 DRIVE COMMAND: Updating metadata for drive: {} (name: {:?}, location: {:?})", 
        drive_uuid, custom_name, physical_location);
    
    match state.sqlite_service.update_drive_metadata(
        &drive_uuid, 
        custom_name.as_deref(), 
        physical_location.as_deref()
    ) {
        Ok(_) => {
            app_log_info!("✅ DRIVE COMMAND: Updated drive metadata successfully");
            Ok(())
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to update drive metadata: {}", e);
            Err(format!("Failed to update drive metadata: {}", e))
        }
    }
}

/// Delete drive from database
#[tauri::command]
pub async fn delete_drive_from_database(
    drive_uuid: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    app_log_info!("🗑️ DRIVE COMMAND: Deleting drive from database: {}", drive_uuid);
    
    match state.sqlite_service.delete_drive(&drive_uuid) {
        Ok(_) => {
            app_log_info!("✅ DRIVE COMMAND: Deleted drive successfully: {}", drive_uuid);
            Ok(())
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to delete drive: {}", e);
            Err(format!("Failed to delete drive: {}", e))
        }
    }
}

/// Get all drives with their metadata (including custom names and locations)
#[tauri::command]
pub async fn get_all_drives_with_metadata(
    state: State<'_, AppState>
) -> Result<Vec<serde_json::Value>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting all drives with metadata");
    
    match state.sqlite_service.get_all_drives() {
        Ok(drives) => {
            app_log_info!("✅ DRIVE COMMAND: Found {} drives with metadata", drives.len());
            Ok(drives)
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to get drives with metadata: {}", e);
            Err(format!("Failed to get drives with metadata: {}", e))
        }
    }
}

/// Get drive metadata by UUID
#[tauri::command]
pub async fn get_drive_metadata(
    drive_uuid: String,
    state: State<'_, AppState>
) -> Result<Option<serde_json::Value>, String> {
    app_log_info!("🔍 DRIVE COMMAND: Getting metadata for drive: {}", drive_uuid);
    
    match state.sqlite_service.get_drive_by_uuid(&drive_uuid) {
        Ok(drive) => {
            if drive.is_some() {
                app_log_info!("✅ DRIVE COMMAND: Found metadata for drive: {}", drive_uuid);
            } else {
                app_log_warn!("⚠️ DRIVE COMMAND: No metadata found for drive: {}", drive_uuid);
            }
            Ok(drive)
        }
        Err(e) => {
            app_log_error!("❌ DRIVE COMMAND: Failed to get drive metadata: {}", e);
            Err(format!("Failed to get drive metadata: {}", e))
        }
    }
}


/// Sync detected drives with database (ensures drives are stored with metadata)
#[tauri::command]
pub async fn sync_drives_to_database(
    state: State<'_, AppState>
) -> Result<(), String> {
    app_log_info!("🔄 DRIVE COMMAND: Syncing drives to database");
    
    match state.drive_service.detect_connected_drives().await {
        Ok(connected_drives) => {
            // Get all drives from database to check for disconnected ones
            let db_drives = match state.sqlite_service.get_all_drives() {
                Ok(drives) => drives,
                Err(e) => {
                    app_log_warn!("⚠️ DRIVE SYNC: Failed to get drives from database: {}", e);
                    Vec::new()
                }
            };
            
            // Create a set of connected drive UUIDs for quick lookup
            let connected_uuids: std::collections::HashSet<String> = 
                connected_drives.iter().map(|d| d.uuid.clone()).collect();
            
            // Update status for connected drives and ensure they exist in database
            for drive in &connected_drives {
                // Check if drive exists in database
                match state.sqlite_service.get_drive_by_uuid(&drive.uuid) {
                    Ok(Some(_)) => {
                        // Drive exists, update its status
                        if let Err(e) = state.sqlite_service.update_drive_status(
                            &drive.uuid, 
                            "connected", 
                            Some(&drive.mount_path)
                        ) {
                            app_log_warn!("⚠️ DRIVE SYNC: Failed to update status for {}: {}", drive.name, e);
                        }
                    }
                    Ok(None) => {
                        // Drive doesn't exist, insert it
                        if let Err(e) = state.sqlite_service.add_drive(
                            &drive.uuid,
                            &drive.name,
                            &drive.mount_path,
                            drive.is_removable
                        ) {
                            app_log_warn!("⚠️ DRIVE SYNC: Failed to add new drive {}: {}", drive.name, e);
                        } else {
                            app_log_info!("✅ DRIVE SYNC: Added new drive {} to database", drive.name);
                        }
                    }
                    Err(e) => {
                        app_log_warn!("⚠️ DRIVE SYNC: Failed to check if drive {} exists: {}", drive.name, e);
                    }
                }
            }
            
            // Mark disconnected drives in database
            for db_drive in db_drives {
                if let Some(uuid) = db_drive.get("uuid").and_then(|v| v.as_str()) {
                    if !connected_uuids.contains(uuid) {
                        if let Err(e) = state.sqlite_service.update_drive_status(
                            uuid, 
                            "disconnected", 
                            None
                        ) {
                            app_log_warn!("⚠️ DRIVE SYNC: Failed to mark drive {} as disconnected: {}", uuid, e);
                        }
                    }
                }
            }
            
            app_log_info!("✅ DRIVE SYNC: Synchronized {} connected drives", connected_drives.len());
            Ok(())
        }
        Err(e) => {
            app_log_error!("❌ DRIVE SYNC: Failed to detect drives: {}", e);
            Err(format!("Failed to sync drives: {}", e))
        }
    }
}
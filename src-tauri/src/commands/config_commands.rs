use tauri::State;
use crate::services::startup::AppState;
use crate::{app_log_info, app_log_error};

/// Get current configuration information
#[tauri::command]
pub async fn get_config_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 CONFIG: Getting configuration information");
    
    let config_service = state.sqlite_service.get_database_service().get_config_service();
    let config = config_service.lock().unwrap();
    
    let db_path = config.get_db_path().map_err(|e| e.to_string())?;
    
    Ok(serde_json::json!({
        "db_path": db_path.to_string_lossy()
    }))
}


/// Set custom database path
#[tauri::command]
pub async fn set_custom_db_path(
    path: Option<String>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    app_log_info!("📁 CONFIG: Setting custom DB path: {:?}", path);
    
    let config_service = state.sqlite_service.get_database_service().get_config_service();
    let mut config = config_service.lock().unwrap();
    
    match config.set_custom_db_path(path.clone()) {
        Ok(_) => {
            app_log_info!("✅ CONFIG: Custom DB path updated successfully");
            Ok(serde_json::json!({
                "success": true,
                "message": "Custom database path updated successfully",
                "custom_db_path": path
            }))
        }
        Err(e) => {
            app_log_error!("❌ CONFIG: Failed to set custom DB path: {}", e);
            Err(format!("Failed to set custom database path: {}", e))
        }
    }
}

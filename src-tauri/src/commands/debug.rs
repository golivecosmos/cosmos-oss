use tauri::State;
use crate::services::startup::AppState;
use crate::{app_log_info, app_log_error};

#[cfg(debug_assertions)]
#[tauri::command]
pub async fn get_sqlite_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    match state.sqlite_service.get_stats() {
        Ok(stats) => Ok(stats),
        Err(e) => {
            app_log_error!("Failed to get SQLite stats: {}", e);
            Err(format!("Failed to get SQLite stats: {}", e))
        }
    }
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub async fn get_sqlite_stats(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Err("Debug commands disabled in production".to_string())
}

#[cfg(debug_assertions)]
#[tauri::command]
pub async fn recreate_sqlite_virtual_table(state: State<'_, AppState>) -> Result<String, String> {
    match state.sqlite_service.recreate_virtual_table() {
        Ok(_) => {
            app_log_info!("✅ SQLite virtual table recreated successfully");
            Ok("Virtual table recreated successfully".to_string())
        }
        Err(e) => {
            app_log_error!("❌ Failed to recreate virtual table: {}", e);
            Err(format!("Failed to recreate virtual table: {}", e))
        }
    }
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub async fn recreate_sqlite_virtual_table(_state: State<'_, AppState>) -> Result<String, String> {
    Err("Debug commands disabled in production".to_string())
}

#[cfg(debug_assertions)]
#[tauri::command]
pub async fn get_database_schema_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    match state.sqlite_service.get_schema_info() {
        Ok(info) => {
            app_log_info!("📊 SCHEMA INFO: Retrieved schema information");
            Ok(info)
        }
        Err(e) => {
            app_log_error!("❌ Failed to get schema info: {}", e);
            Err(format!("Failed to get schema information: {}", e))
        }
    }
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub async fn get_database_schema_info(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Err("Debug commands disabled in production".to_string())
}

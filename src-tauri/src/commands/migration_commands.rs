use crate::services::startup::AppState;
use crate::{app_log_info, app_log_error};
use tauri::State;

/// Get migration status and history
#[tauri::command]
pub async fn get_migration_info(
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 MIGRATION COMMAND: Getting migration info");
    
    let connection = state.sqlite_service.get_database_service().get_connection();
    let db = connection.lock().unwrap();
    
    match crate::services::migration_service::get_migration_info(&db) {
        Ok(info) => {
            app_log_info!("✅ MIGRATION COMMAND: Retrieved migration info successfully");
            Ok(info)
        }
        Err(e) => {
            app_log_error!("❌ MIGRATION COMMAND: Failed to get migration info: {}", e);
            Err(format!("Failed to get migration info: {}", e))
        }
    }
}

// Note: Database encryption is now automatic - no manual migration needed
// Schema migrations still run automatically during database initialization
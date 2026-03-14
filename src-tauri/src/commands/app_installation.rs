use crate::services::app_installation_service::{
    AppInstallRequest, AppInstallResponse, AppInstallationService, InstalledApp,
};
use crate::services::startup::AppState;
use crate::{app_log_error, app_log_info};
use anyhow::Result;
use serde_json;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const GOOGLE_GENERATIVE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

fn should_validate_google_api_key(app_name: &str) -> bool {
    app_name.eq_ignore_ascii_case("Google Gemini")
}

fn looks_like_google_api_key(api_key: &str) -> bool {
    let trimmed = api_key.trim();
    trimmed.starts_with("AIza") && trimmed.len() >= 20
}

fn categorize_google_api_error(
    status: reqwest::StatusCode,
    message: &str,
    context: &str,
) -> String {
    match status.as_u16() {
        400 => format!(
            "{} failed: invalid request. Verify the API key and try again. Details: {}",
            context, message
        ),
        401 => "Google API key is invalid. Open App Store, reinstall/update Google Gemini with a valid key.".to_string(),
        403 => "Google API access forbidden. Enable Generative Language API in Google Cloud and verify this key has access.".to_string(),
        429 => "Google API rate limit exceeded during key validation. Wait and retry, or increase project quota.".to_string(),
        500..=599 => format!("Google API is temporarily unavailable ({}). Please retry shortly.", status),
        _ => format!("{} failed ({}): {}", context, status, message),
    }
}

async fn validate_google_api_key(api_key: &str) -> Result<(), String> {
    if !looks_like_google_api_key(api_key) {
        return Err(
            "API key format looks invalid. Google API keys usually start with 'AIza'.".to_string(),
        );
    }

    let client = reqwest::Client::new();
    let response = tokio::time::timeout(
        Duration::from_secs(12),
        client
            .get(&format!("{}/models", GOOGLE_GENERATIVE_BASE_URL))
            .header("x-goog-api-key", api_key)
            .send(),
    )
    .await
    .map_err(|_| {
        "Timed out while validating Google API key. Check your network and try again.".to_string()
    })?
    .map_err(|e| format!("Google API key validation request failed: {}", e))?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let error_data: serde_json::Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"error": {"message": "Unknown error"}}));
    let message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown error");
    Err(categorize_google_api_error(
        status,
        message,
        "Google API key validation",
    ))
}

/// Install an app with configuration
#[tauri::command]
pub async fn install_app(
    request: AppInstallRequest,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<AppInstallResponse, String> {
    let app_name = request.app_name.clone();
    if should_validate_google_api_key(&app_name) {
        if let Some(api_key) = request.api_key.as_deref() {
            validate_google_api_key(api_key).await?;
        } else {
            return Err("Google Gemini installation requires an API key.".to_string());
        }
    }

    let app_service = AppInstallationService::new(state.sqlite_service.get_database_service());

    let result = app_service.install_app(request);

    // If installation was successful, emit an event to notify the frontend
    if let Ok(response) = &result {
        if response.success {
            app_log_info!("🔔 Emitting app_installed event for: {}", app_name);
            if let Err(e) = app_handle.emit(
                "app_installed",
                serde_json::json!({
                    "app_name": app_name,
                    "app_id": response.app_id,
                    "message": response.message
                }),
            ) {
                app_log_error!("Failed to emit app_installed event: {}", e);
            } else {
                app_log_info!("✅ Successfully emitted app_installed event");
            }
        }
    }

    result.map_err(|e| e.to_string())
}

/// Get all installed apps
#[tauri::command]
pub async fn get_installed_apps(state: State<'_, AppState>) -> Result<Vec<InstalledApp>, String> {
    let app_service = AppInstallationService::new(state.sqlite_service.get_database_service());

    match app_service.get_installed_apps() {
        Ok(apps) => Ok(apps),
        Err(e) => {
            println!("🔍 APP COMMAND: Error loading apps: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get app by ID
#[tauri::command]
pub async fn get_app_by_id(
    app_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<InstalledApp>, String> {
    let app_service = AppInstallationService::new(state.sqlite_service.get_database_service());

    app_service.get_app_by_id(app_id).map_err(|e| e.to_string())
}

/// Uninstall an app
#[tauri::command]
pub async fn uninstall_app(
    app_id: i64,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<AppInstallResponse, String> {
    let app_service = AppInstallationService::new(state.sqlite_service.get_database_service());

    // Capture app name before uninstall so event payload is accurate.
    let app_name_before_uninstall = app_service
        .get_app_by_id(app_id)
        .ok()
        .flatten()
        .map(|app| app.app_name);

    let result = app_service.uninstall_app(app_id);

    // If uninstallation was successful, emit an event to notify the frontend
    if let Ok(response) = &result {
        if response.success {
            if let Some(app_name) = app_name_before_uninstall {
                app_log_info!("🔔 Emitting app_uninstalled event for: {}", app_name);
                if let Err(e) = app_handle.emit(
                    "app_uninstalled",
                    serde_json::json!({
                        "app_name": app_name,
                        "app_id": app_id,
                        "message": response.message
                    }),
                ) {
                    app_log_error!("Failed to emit app_uninstalled event: {}", e);
                } else {
                    app_log_info!("✅ Successfully emitted app_uninstalled event");
                }
            }
        }
    }

    match result {
        Ok(response) => {
            println!(
                "🔍 UNINSTALL COMMAND: Successfully uninstalled app, response: {:?}",
                response
            );
            Ok(response)
        }
        Err(e) => {
            println!("🔍 UNINSTALL COMMAND: Error uninstalling app: {}", e);
            Err(e.to_string())
        }
    }
}

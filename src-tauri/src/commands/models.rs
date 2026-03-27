use crate::services::download_service::{DownloadProgress, DownloadService};
use crate::services::startup::AppState;
use crate::{app_log_error, app_log_info};
use tauri::{Emitter, State};

/// Check the status of AI models
#[tauri::command]
pub async fn check_models_status(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 MODEL STATUS: Checking model availability");

    let models_available = DownloadService::are_models_available();

    let missing_models = match DownloadService::check_missing_models() {
        Ok(missing) => missing.into_iter().map(|m| m.name).collect::<Vec<String>>(),
        Err(e) => {
            app_log_error!("Failed to check missing models: {}", e);
            return Err(format!("Failed to check missing models: {}", e));
        }
    };

    let status = serde_json::json!({
        "models_available": models_available,
        "missing_models": missing_models,
        "total_missing": missing_models.len(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    app_log_info!(
        "📊 MODEL STATUS: Available = {}, Missing = {}",
        models_available,
        missing_models.len()
    );

    Ok(status)
}

/// Clear existing models and re-download them
#[tauri::command]
pub async fn clear_and_redownload_models(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!("🧹 CLEAR MODELS: Clearing existing model files and re-downloading");

    let download_service = &state.download_service;

    // Clear existing model files
    match DownloadService::clear_existing_models() {
        Ok(_) => {
            app_log_info!("✅ Existing model files cleared");
        }
        Err(e) => {
            app_log_error!("Failed to clear existing models: {}", e);
        }
    }

    // Create progress callback that emits events to frontend
    let progress_callback = {
        let app_handle = app_handle.clone();
        move |progress: DownloadProgress| {
            if let Err(e) = app_handle.emit("download_progress", &progress) {
                app_log_error!("Failed to emit download progress: {}", e);
            }
        }
    };

    // Download all models
    match download_service
        .download_all_missing_models(progress_callback)
        .await
    {
        Ok(_) => {
            app_log_info!("✅ MODEL DOWNLOAD: All models downloaded successfully");

            // Automatically reload models after successful download
            app_log_info!("🔄 AUTO RELOAD: Reloading models after re-download...");
            match state.model_service.reload_clip_model().await {
                Ok(_) => {
                    app_log_info!("✅ AUTO RELOAD: Models reloaded successfully after re-download");
                    Ok("All models downloaded and loaded successfully".to_string())
                }
                Err(e) => {
                    app_log_error!(
                        "❌ AUTO RELOAD: Failed to reload models after re-download: {}",
                        e
                    );
                    // Still return success since download worked, just mention reload issue
                    Ok(format!(
                        "Models downloaded successfully, but failed to reload: {}",
                        e
                    ))
                }
            }
        }
        Err(e) => {
            app_log_error!("❌ MODEL DOWNLOAD: Failed to download models: {}", e);
            Err(format!("Failed to download models: {}", e))
        }
    }
}

/// Download missing AI models
#[tauri::command]
pub async fn download_models(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    app_log_info!("🚀 MODEL DOWNLOAD: Starting model download process");

    let download_service = &state.download_service;

    // Check if models are already available
    if DownloadService::are_models_available() {
        app_log_info!("All models are already available, skipping download");
        return Ok("All models are already available".to_string());
    }

    // Create progress callback that emits events to frontend
    let progress_callback = {
        let app_handle = app_handle.clone();
        move |progress: DownloadProgress| {
            if let Err(e) = app_handle.emit("download_progress", &progress) {
                app_log_error!("Failed to emit download progress: {}", e);
            }
        }
    };

    // Download all missing models with timeout
    let download_result = tokio::time::timeout(
        std::time::Duration::from_secs(300), // 5 minute timeout
        download_service.download_all_missing_models(progress_callback),
    )
    .await;

    match download_result {
        Ok(Ok(_)) => {
            app_log_info!("✅ MODEL DOWNLOAD: All models downloaded successfully");
            Ok("All models downloaded successfully".to_string())
        }
        Ok(Err(e)) => {
            if e.to_string() == DownloadService::DOWNLOAD_ALREADY_IN_PROGRESS_MESSAGE {
                app_log_info!("ℹ️ MODEL DOWNLOAD: Another download is already in progress");
                return Ok("Model download already in progress".to_string());
            }
            app_log_error!("❌ MODEL DOWNLOAD: Failed to download models: {}", e);
            Err(format!("Failed to download models: {}", e))
        }
        Err(_) => {
            app_log_error!("❌ MODEL DOWNLOAD: Download timed out after 5 minutes");
            Err(
                "Download timed out. Please check your internet connection and try again."
                    .to_string(),
            )
        }
    }
}

/// Reload AI models
#[tauri::command]
pub async fn reload_models(state: State<'_, AppState>) -> Result<String, String> {
    app_log_info!("🔄 RELOAD MODELS: Attempting to reload AI models");

    let model_service = &state.model_service;

    // Try to reload the CLIP model
    match model_service.reload_clip_model().await {
        Ok(_) => {
            app_log_info!("✅ RELOAD SUCCESS: AI models reloaded successfully");
            Ok("Models reloaded successfully".to_string())
        }
        Err(e) => {
            app_log_error!("❌ RELOAD FAILED: Failed to reload models: {}", e);
            Err(format!("Failed to reload models: {}", e))
        }
    }
}

/// Debug model status and file system
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn debug_model_status(state: State<'_, AppState>) -> Result<String, String> {
    app_log_info!("🔧 DEBUG: Checking detailed model status");

    let model_service = &state.model_service;

    // Check if model is currently loaded
    let is_loaded = model_service.is_model_loaded();
    app_log_info!("🔧 Model loaded: {}", is_loaded);

    // **NEW: Check download service status first**
    let models_available = DownloadService::are_models_available();
    app_log_info!(
        "🔧 Download service says models available: {}",
        models_available
    );

    // **NEW: Check what models are missing**
    match DownloadService::check_missing_models() {
        Ok(missing) => {
            app_log_info!("🔧 Missing models count: {}", missing.len());
            for model in &missing {
                app_log_info!(
                    "🔧 Missing: {} at {}",
                    model.name,
                    model.destination_path.display()
                );
            }
        }
        Err(e) => {
            app_log_error!("🔧 Failed to check missing models: {}", e);
        }
    }

    // Check file system
    match crate::utils::path_utils::get_app_data_dir() {
        Ok(app_dir) => {
            let models_dir = app_dir.join("models");
            app_log_info!("🔧 Models directory: {}", models_dir.display());
            app_log_info!("🔧 Models directory exists: {}", models_dir.exists());

            // **NEW: Check the new simplified structure**
            let text_model_dir = models_dir.join("nomic-embed-text-v1.5");
            let vision_model_dir = models_dir.join("nomic-embed-vision-v1.5");

            app_log_info!(
                "🔧 Text model directory: {} - {}",
                text_model_dir.display(),
                if text_model_dir.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );
            app_log_info!(
                "🔧 Vision model directory: {} - {}",
                vision_model_dir.display(),
                if vision_model_dir.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );

            // Check individual files in the new structure
            let text_model_onnx = text_model_dir.join("onnx").join("model.onnx");
            let text_config = text_model_dir.join("config.json");
            let text_tokenizer = text_model_dir.join("tokenizer.json");

            let vision_model_onnx = vision_model_dir.join("onnx").join("model.onnx");
            let vision_preprocessor = vision_model_dir.join("preprocessor_config.json");

            app_log_info!(
                "🔧 Text model ONNX: {} - {}",
                text_model_onnx.display(),
                if text_model_onnx.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );
            app_log_info!(
                "🔧 Text config: {} - {}",
                text_config.display(),
                if text_config.exists() { "✅" } else { "❌" }
            );
            app_log_info!(
                "🔧 Text tokenizer: {} - {}",
                text_tokenizer.display(),
                if text_tokenizer.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );
            app_log_info!(
                "🔧 Vision model ONNX: {} - {}",
                vision_model_onnx.display(),
                if vision_model_onnx.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );
            app_log_info!(
                "🔧 Vision preprocessor: {} - {}",
                vision_preprocessor.display(),
                if vision_preprocessor.exists() {
                    "✅"
                } else {
                    "❌"
                }
            );

            // **NEW: Check old structure for comparison**
            let old_onnx_dir = models_dir.join("onnx");
            app_log_info!(
                "🔧 Old onnx directory: {} - {}",
                old_onnx_dir.display(),
                if old_onnx_dir.exists() { "✅" } else { "❌" }
            );

            if old_onnx_dir.exists() {
                let old_text_model = old_onnx_dir.join("text_model.onnx");
                let old_vision_model = old_onnx_dir.join("vision_model.onnx");

                app_log_info!(
                    "🔧 Old text model: {} - {}",
                    old_text_model.display(),
                    if old_text_model.exists() {
                        "✅"
                    } else {
                        "❌"
                    }
                );
                app_log_info!(
                    "🔧 Old vision model: {} - {}",
                    old_vision_model.display(),
                    if old_vision_model.exists() {
                        "✅"
                    } else {
                        "❌"
                    }
                );
            }
        }
        Err(e) => {
            app_log_error!("🔧 Failed to get app data dir: {}", e);
        }
    }

    Ok(format!(
        "Debug complete - check logs for details. Model loaded: {}, Files available: {}",
        is_loaded, models_available
    ))
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub async fn debug_model_status(_state: State<'_, AppState>) -> Result<String, String> {
    Err("Debug commands disabled in production".to_string())
}

use crate::services::audio_service::TranscriptionResult;
use crate::AppState;
use crate::{app_log_debug, app_log_error, app_log_info};
use anyhow::Result;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

/// Transcribe an audio file to text (simplified)
#[tauri::command]
pub async fn transcribe_audio_file(
    file_path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<TranscriptionResult, String> {
    app_log_info!("🎤 Starting audio transcription for: {}", file_path);
    app_log_debug!("🔍 Raw input path: '{}'", file_path);

    // Clean up the file path - handle asset URLs that may have working directory prefix
    let clean_path = if let Some(asset_start) = file_path.find("asset:/localhost/") {
        let asset_url = &file_path[asset_start..];
        let asset_path = &asset_url["asset:/localhost/".len()..];
        app_log_debug!(
            "🔧 Found asset:/localhost/ pattern, decoding: '{}'",
            asset_path
        );
        percent_encoding::percent_decode_str(asset_path)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode URL: {}", e))?
            .to_string()
    } else if let Some(asset_start) = file_path.find("asset://localhost/") {
        let asset_url = &file_path[asset_start..];
        let asset_path = &asset_url["asset://localhost/".len()..];
        app_log_debug!(
            "🔧 Found asset://localhost/ pattern, decoding: '{}'",
            asset_path
        );
        percent_encoding::percent_decode_str(asset_path)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode URL: {}", e))?
            .to_string()
    } else if let Some(file_path_without_protocol) = file_path.strip_prefix("file://") {
        app_log_debug!(
            "🔧 Matched file:// pattern, decoding: '{}'",
            file_path_without_protocol
        );
        percent_encoding::percent_decode_str(file_path_without_protocol)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode URL: {}", e))?
            .to_string()
    } else {
        app_log_debug!("🔧 No URL pattern matched, using path as-is");
        file_path.clone()
    };

    app_log_debug!("🔧 Cleaned path: {} -> {}", file_path, clean_path);

    let path = Path::new(&clean_path);
    app_log_debug!("🔍 Final path for service: {:?}", path);

    // Emit transcription started event
    let transcription_started_data = serde_json::json!({
        "file_path": clean_path,
        "status": "started"
    });
    app_log_info!("🔔 Emitting transcription_started event for: {}", clean_path);
    if let Err(e) = app_handle.emit("transcription_started", &transcription_started_data) {
        app_log_error!("Failed to emit transcription_started event: {}", e);
    } else {
        app_log_info!("✅ Successfully emitted transcription_started event");
    }

    let mut audio_service = state.audio_service.lock().await;

    // Perform transcription
    match audio_service.transcribe_file(path).await {
        Ok(result) => {
            app_log_info!("✅ Transcription completed successfully for: {}", file_path);
            app_log_debug!("📝 Transcription result: {} characters", result.text.len());

            // Store the transcription in the database
            match state
                .sqlite_service
                .store_transcription(&result, &clean_path)
            {
                Ok(transcription_id) => {
                    app_log_info!(
                        "💾 Transcription stored in database with ID: {}",
                        transcription_id
                    );
                }
                Err(e) => {
                    app_log_error!("⚠️ Failed to store transcription in database: {}", e);
                    // Don't fail the whole operation if storage fails
                }
            }

            // Emit transcription completed event
            let transcription_completed_data = serde_json::json!({
                "file_path": clean_path,
                "status": "completed",
                "transcription": {
                    "text": result.text,
                    "duration": result.duration,
                    "language": result.language,
                    "segments_count": result.segments.len()
                }
            });
            app_log_info!("🔔 Emitting transcription_completed event for: {}", clean_path);
            if let Err(e) = app_handle.emit("transcription_completed", &transcription_completed_data) {
                app_log_error!("Failed to emit transcription_completed event: {}", e);
            } else {
                app_log_info!("✅ Successfully emitted transcription_completed event");
            }

            Ok(result)
        }
        Err(e) => {
            let error = format!("Transcription failed: {}", e);
            app_log_error!("{}", error);

            // Emit transcription failed event
            let transcription_failed_data = serde_json::json!({
                "file_path": clean_path,
                "status": "failed",
                "error": error
            });
            app_log_info!("🔔 Emitting transcription_failed event for: {}", clean_path);
            if let Err(emit_err) = app_handle.emit("transcription_failed", &transcription_failed_data) {
                app_log_error!("Failed to emit transcription_failed event: {}", emit_err);
            } else {
                app_log_info!("✅ Successfully emitted transcription_failed event");
            }

            Err(error)
        }
    }
}

/// Validate that an audio file can be processed
#[tauri::command]
pub async fn validate_audio_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    app_log_debug!("🔍 Validating audio file: {}", file_path);

    let path = Path::new(&file_path);
    let audio_service = state.audio_service.lock().await;

    match audio_service.validate_audio_file(path) {
        Ok(_) => {
            app_log_debug!("✅ Audio file validation passed: {}", file_path);
            Ok(true)
        }
        Err(e) => {
            app_log_debug!("❌ Audio file validation failed: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get transcription for a specific file
#[tauri::command]
pub async fn get_transcription_by_path(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    app_log_debug!("🔍 Getting transcription for: {}", file_path);

    // Clean the file path the same way as in transcribe_file
    let clean_path = if let Some(asset_path) = file_path.strip_prefix("asset://localhost/") {
        app_log_debug!("🔧 Matched asset:// pattern, decoding: '{}'", asset_path);
        percent_encoding::percent_decode_str(asset_path)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode URL: {}", e))?
            .to_string()
    } else if let Some(file_path_without_protocol) = file_path.strip_prefix("file://") {
        app_log_debug!(
            "🔧 Matched file:// pattern, decoding: '{}'",
            file_path_without_protocol
        );
        percent_encoding::percent_decode_str(file_path_without_protocol)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode URL: {}", e))?
            .to_string()
    } else {
        app_log_debug!("🔧 No URL pattern matched, using path as-is");
        file_path.clone()
    };

    app_log_debug!("🔧 Cleaned path: {} -> {}", file_path, clean_path);

    match state.sqlite_service.get_transcription_by_path(&clean_path) {
        Ok(transcription) => {
            if transcription.is_some() {
                app_log_debug!("✅ Found transcription for: {}", clean_path);
            } else {
                app_log_debug!("❌ No transcription found for: {}", clean_path);
            }
            Ok(transcription)
        }
        Err(e) => {
            let error = format!("Failed to get transcription: {}", e);
            app_log_error!("{}", error);
            Err(error)
        }
    }
}

/// Get whisper model status for transcription
#[tauri::command]
pub async fn is_whisper_model_available() -> Result<String, String> {
    app_log_debug!("🔍 Checking whisper model status");

    use crate::services::download_service::DownloadService;
    let status = DownloadService::get_whisper_status();

    let status_str = match status {
        crate::services::download_service::WhisperStatus::NotAvailable => "not_available",
        crate::services::download_service::WhisperStatus::Downloading => "downloading",
        crate::services::download_service::WhisperStatus::Ready => "ready",
        crate::services::download_service::WhisperStatus::Failed(_) => "failed",
    };

    app_log_debug!("✅ Whisper model status: {}", status_str);
    Ok(status_str.to_string())
}

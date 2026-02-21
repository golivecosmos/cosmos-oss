use tauri::command;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct ThumbnailResult {
    pub success: bool,
    pub data: Option<String>, // base64 encoded JPEG
    pub error: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[command]
pub async fn generate_video_thumbnail(
    file_path: String,
    timestamp_seconds: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
) -> ThumbnailResult {
    let width = width.unwrap_or(320);
    let height = height.unwrap_or(180);
    let timestamp = timestamp_seconds.unwrap_or(1.0);

    // Try cache first - this enables offline preview support!
    match crate::thumbnail_cache::try_get_cached_thumbnail(&file_path, timestamp, width, height).await {
        Ok(Some(jpeg_data)) => {
            // Found in cache - return even if file is offline
            let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &jpeg_data);
            return ThumbnailResult {
                success: true,
                data: Some(base64_data),
                error: None,
                width: Some(width),
                height: Some(height),
            };
        }
        Ok(None) => {
            // Not in cache, continue to check file
        }
        Err(e) => {
            eprintln!("Cache check error: {}", e);
            // Continue anyway
        }
    }

    // Not in cache - check if file exists
    if !Path::new(&file_path).exists() {
        return ThumbnailResult {
            success: false,
            data: None,
            error: Some(format!("File not found: {}", file_path)),
            width: None,
            height: None,
        };
    }

    // File exists and not cached - generate new thumbnail
    generate_ffmpeg_thumbnail_impl(file_path, timestamp, width, height).await
}

async fn generate_ffmpeg_thumbnail_impl(
    file_path: String,
    timestamp: f64,
    width: u32,
    height: u32,
) -> ThumbnailResult {
    // Use cached thumbnail generation with file system persistence
    match crate::thumbnail_cache::get_cached_thumbnail(&file_path, timestamp, width, height).await {
        Ok(jpeg_data) => {
            let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &jpeg_data);
            ThumbnailResult {
                success: true,
                data: Some(base64_data),
                error: None,
                width: Some(width),
                height: Some(height),
            }
        }
        Err(error) => ThumbnailResult {
            success: false,
            data: None,
            error: Some(error),
            width: None,
            height: None,
        }
    }
}




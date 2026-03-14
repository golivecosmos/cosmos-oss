use crate::services::startup::AppState;
use crate::{app_log_error, app_log_info};
use dirs;
use serde_json::json;
use std::process::Command;
use tauri::{command, State};

/// Copy text to system clipboard
#[command]
pub async fn copy_to_clipboard(text: String) -> Result<String, String> {
    app_log_info!("📋 CLIPBOARD: Copying text to clipboard");

    #[cfg(target_os = "macos")]
    {
        match Command::new("pbcopy")
            .arg(&text)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    use std::io::Write;
                    let mut stdin = stdin;
                    if let Err(e) = stdin.write_all(text.as_bytes()) {
                        app_log_error!("Failed to write to pbcopy stdin: {}", e);
                        return Err(format!("Failed to copy to clipboard: {}", e));
                    }
                }
                match child.wait() {
                    Ok(status) if status.success() => {
                        app_log_info!("✅ Text copied to clipboard successfully");
                        Ok("Text copied to clipboard".to_string())
                    }
                    Ok(_) => {
                        app_log_error!("pbcopy command failed");
                        Err("Failed to copy to clipboard".to_string())
                    }
                    Err(e) => {
                        app_log_error!("Failed to wait for pbcopy: {}", e);
                        Err(format!("Failed to copy to clipboard: {}", e))
                    }
                }
            }
            Err(e) => {
                app_log_error!("Failed to spawn pbcopy: {}", e);
                Err(format!("Failed to copy to clipboard: {}", e))
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        match Command::new("clip").arg(&text).output() {
            Ok(output) if output.status.success() => {
                app_log_info!("✅ Text copied to clipboard successfully");
                Ok("Text copied to clipboard".to_string())
            }
            Ok(_) => {
                app_log_error!("clip command failed");
                Err("Failed to copy to clipboard".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to execute clip command: {}", e);
                Err(format!("Failed to copy to clipboard: {}", e))
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Try xclip first, then xsel as fallback
        let result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .arg(&text)
            .output();
        match result {
            Ok(output) if output.status.success() => {
                app_log_info!("✅ Text copied to clipboard successfully (xclip)");
                Ok("Text copied to clipboard".to_string())
            }
            _ => {
                // Fallback to xsel
                match Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .arg(&text)
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        app_log_info!("✅ Text copied to clipboard successfully (xsel)");
                        Ok("Text copied to clipboard".to_string())
                    }
                    _ => {
                        app_log_error!("Failed to copy to clipboard - no suitable tool found");
                        Err("Failed to copy to clipboard - install xclip or xsel".to_string())
                    }
                }
            }
        }
    }
}

/// Show file in system file manager
#[command]
pub async fn show_in_file_manager(path: String) -> Result<String, String> {
    app_log_info!("📁 FILE MANAGER: Showing file in manager: {}", path);
    if !std::path::Path::new(&path).exists() {
        return Err("File does not exist".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        match Command::new("open").args(["-R", &path]).spawn() {
            Ok(_) => {
                app_log_info!("✅ File shown in Finder successfully");
                Ok("File shown in Finder".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open Finder: {}", e);
                Err(format!("Failed to show file in Finder: {}", e))
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        match Command::new("explorer").args(["/select,", &path]).spawn() {
            Ok(_) => {
                app_log_info!("✅ File shown in Explorer successfully");
                Ok("File shown in Explorer".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open Explorer: {}", e);
                Err(format!("Failed to show file in Explorer: {}", e))
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Get the directory containing the file
        let parent_dir = std::path::Path::new(&path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("/"))
            .to_string_lossy();
        match Command::new("xdg-open").arg(&*parent_dir).spawn() {
            Ok(_) => {
                app_log_info!("✅ Directory opened in file manager successfully");
                Ok("Directory opened in file manager".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open file manager: {}", e);
                Err(format!("Failed to show file in file manager: {}", e))
            }
        }
    }
}

/// Open file with default system application
#[command]
pub async fn open_with_default_app(path: String) -> Result<String, String> {
    app_log_info!("🚀 OPEN: Opening file with default app: {}", path);
    if !std::path::Path::new(&path).exists() {
        return Err("File does not exist".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        match Command::new("open").arg(&path).spawn() {
            Ok(_) => {
                app_log_info!("✅ File opened with default app successfully");
                Ok("File opened with default application".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open file: {}", e);
                Err(format!("Failed to open file: {}", e))
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        match Command::new("cmd").args(["/c", "start", "", &path]).spawn() {
            Ok(_) => {
                app_log_info!("✅ File opened with default app successfully");
                Ok("File opened with default application".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open file: {}", e);
                Err(format!("Failed to open file: {}", e))
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        match Command::new("xdg-open").arg(&path).spawn() {
            Ok(_) => {
                app_log_info!("✅ File opened with default app successfully");
                Ok("File opened with default application".to_string())
            }
            Err(e) => {
                app_log_error!("Failed to open file: {}", e);
                Err(format!("Failed to open file: {}", e))
            }
        }
    }
}

/// Check if FFmpeg is available
#[tauri::command]
pub fn is_ffmpeg_available(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.video_service.is_ffmpeg_available())
}

/// Get system information (internal function)
#[tauri::command]
pub async fn get_system_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let models_loaded = state.embedding_service.is_semantic_search_available();
    let ffmpeg_available = state.video_service.is_ffmpeg_available();

    // Get SQLite stats
    let sqlite_stats = match state.sqlite_service.get_stats() {
        Ok(stats) => stats,
        Err(_) => json!({}),
    };

    let db_service = state.sqlite_service.get_database_service();
    let connection = db_service.get_connection();
    let db = connection.lock().unwrap();

    // Count unique images (excluding video frames)
    let unique_images = db.query_row(
        "SELECT COUNT(DISTINCT file_path) FROM images WHERE (source_type != 'video_frame' OR source_type IS NULL) AND (mime_type LIKE 'image/%' OR metadata LIKE '%\"width\":%')",
        [],
        |row| row.get::<_, i32>(0)
    ).unwrap_or(0);

    // Count unique videos
    let unique_videos = db.query_row(
        "SELECT COUNT(DISTINCT file_path) FROM images WHERE source_type = 'video_frame' OR mime_type LIKE 'video/%'",
        [],
        |row| row.get::<_, i32>(0)
    ).unwrap_or(0);

    let total_unique_files = unique_images + unique_videos;

    let files_with_embeddings = db
        .query_row(
            "SELECT COUNT(*) FROM images WHERE embedding IS NOT NULL",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0);

    // Get OS information
    let os_info = os_info::get();
    let os = os_info.os_type().to_string();
    let os_version = os_info.version().to_string();
    let arch = std::env::consts::ARCH.to_string();

    Ok(json!({
        "os": os,
        "os_version": os_version,
        "arch": arch,
        "app_version": env!("CARGO_PKG_VERSION"),
        "models_loaded": models_loaded,
        "ffmpeg_available": ffmpeg_available,
        "sqlite_stats": sqlite_stats,
        "actual_file_counts": {
            "total_unique_files": total_unique_files,
            "unique_images": unique_images,
            "unique_videos": unique_videos,
            "files_with_embeddings": files_with_embeddings,
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Get application state information
#[tauri::command]
pub async fn get_app_state_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let models_loaded = state.embedding_service.is_semantic_search_available();
    let ffmpeg_available = state.video_service.is_ffmpeg_available();

    // Get indexed count from SQLite
    let indexed_count = match state.sqlite_service.get_image_count() {
        Ok(count) => count,
        Err(_) => 0,
    };

    let info = json!({
        "models_loaded": models_loaded,
        "ffmpeg_available": ffmpeg_available,
        "indexed_count": indexed_count,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(info)
}

/// Cancel download
#[tauri::command]
pub async fn cancel_download(_state: State<'_, AppState>) -> Result<String, String> {
    app_log_info!("🛑 CANCEL DOWNLOAD: User requested download cancellation");

    // For now, we'll just log this. In a full implementation, you'd want to:
    // 1. Set a cancellation flag in the download service
    // 2. Stop any ongoing downloads
    // 3. Clean up partial files

    app_log_info!("⚠️ Download cancellation requested - downloads will complete current file");
    Ok("Download cancellation requested".to_string())
}

/// Get the user's desktop directory path
#[tauri::command]
pub async fn get_desktop_path() -> Result<String, String> {
    match dirs::desktop_dir() {
        Some(desktop_path) => {
            let path_str = desktop_path.to_string_lossy().to_string();
            app_log_info!("✅ Desktop path: {}", path_str);
            Ok(path_str)
        }
        None => {
            app_log_error!("Failed to get desktop directory");
            Err("Failed to get desktop directory".to_string())
        }
    }
}

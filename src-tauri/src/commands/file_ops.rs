use crate::models::embedding::ImageVectorDataResponse;
use crate::models::file_item::{FileItem, FileMetadata};
use crate::services::file_service::FilePreviewResult;
use crate::services::startup::AppState;
use crate::utils::logger;
use base64::{engine::general_purpose, Engine};
use serde_json;
use std::collections::HashMap;
use tauri::State;
use tokio::fs;

// Import logging macros from crate root
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};

/// Check if a file exists at the given path
#[tauri::command]
pub fn file_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

/// Check if the given path is a directory
#[tauri::command]
pub fn is_directory(path: String, state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.file_service.is_directory(&path))
}

/// Get the path to the current log file
#[tauri::command]
pub fn get_log_file_path() -> Result<String, String> {
    let logger = logger::LOGGER.get_or_init(|| logger::AppLogger::new());
    Ok(logger.get_log_file_path().to_string_lossy().to_string())
}

/// List contents of a directory
#[tauri::command]
pub fn list_directory_contents(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileItem>, String> {
    state
        .file_service
        .list_directory(&path)
        .map_err(|e| format!("Failed to list directory: {}", e))
}

/// Get metadata for a file
#[tauri::command]
pub fn get_file_metadata(path: String, state: State<'_, AppState>) -> Result<FileMetadata, String> {
    state
        .file_service
        .get_file_metadata(&path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))
}

/// Read file content as text
#[tauri::command]
pub fn read_file_content(path: String, state: State<'_, AppState>) -> Result<String, String> {
    state
        .file_service
        .read_file_content(&path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

/// Read file preview content with optional byte limit.
#[tauri::command]
pub fn read_file_preview(
    path: String,
    max_bytes: Option<usize>,
    state: State<'_, AppState>,
) -> Result<FilePreviewResult, String> {
    state
        .file_service
        .read_file_preview(&path, max_bytes)
        .map_err(|e| format!("Failed to read file preview: {}", e))
}

/// Read file as base64 encoded string
#[tauri::command]
pub async fn read_file_as_base64(path: String) -> Result<String, String> {
    app_log_info!("📄 READ BINARY: Reading file as base64: {}", path);

    // Check if file exists first
    if !std::path::Path::new(&path).exists() {
        app_log_error!("❌ READ BINARY: File does not exist: {}", path);
        return Err(format!("File does not exist: {}", path));
    }

    // Check if it's a file (not a directory)
    if !std::path::Path::new(&path).is_file() {
        app_log_error!("❌ READ BINARY: Path is not a file: {}", path);
        return Err(format!("Path is not a file: {}", path));
    }

    // Get file metadata for debugging
    match std::fs::metadata(&path) {
        Ok(metadata) => {
            app_log_info!("📄 READ BINARY: File size: {} bytes", metadata.len());
        }
        Err(e) => {
            app_log_warn!("⚠️ READ BINARY: Could not get file metadata: {}", e);
        }
    }

    match fs::read(&path).await {
        Ok(bytes) => {
            let base64_content = general_purpose::STANDARD.encode(&bytes);
            app_log_info!(
                "✅ READ BINARY: Successfully read {} bytes from {}",
                bytes.len(),
                path
            );
            Ok(base64_content)
        }
        Err(e) => {
            app_log_error!("❌ READ BINARY: Failed to read file {}: {}", path, e);
            Err(format!("Failed to read file: {}", e))
        }
    }
}

/// List directory contents
#[tauri::command]
pub fn list_directory(path: String, state: State<'_, AppState>) -> Result<Vec<FileItem>, String> {
    state
        .file_service
        .list_directory(&path)
        .map_err(|e| format!("Failed to list directory: {}", e))
}

/// List directory contents recursively
#[tauri::command]
pub fn list_directory_recursive(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileItem>, String> {
    state
        .file_service
        .list_directory_recursive(&path)
        .map_err(|e| format!("Failed to list directory recursively: {}", e))
}

/// Get all indexed files
#[tauri::command]
pub async fn get_indexed_files(
    state: State<'_, AppState>,
) -> Result<Vec<ImageVectorDataResponse>, String> {
    Ok(load_all_indexed_files(&state))
}

/// Helper to group files by file_path (for video frames)
fn group_files_by_file_path(files: Vec<ImageVectorDataResponse>) -> Vec<ImageVectorDataResponse> {
    let mut grouped_files = Vec::new();
    let mut video_frames: std::collections::HashMap<String, Vec<ImageVectorDataResponse>> =
        std::collections::HashMap::new();
    for file in files {
        let metadata = serde_json::from_str::<serde_json::Value>(&file.metadata)
            .unwrap_or_else(|_| serde_json::json!({}));
        let is_video_frame = metadata["source_type"] == "video_frame";
        if is_video_frame {
            let video_path = file.file_path.clone();
            video_frames
                .entry(video_path)
                .or_insert_with(Vec::new)
                .push(file);
        } else {
            grouped_files.push(file);
        }
    }
    for (_, frames) in video_frames {
        if !frames.is_empty() {
            grouped_files.push(frames[0].clone());
        }
    }
    grouped_files
}

/// Get indexed files grouped by file path
#[tauri::command]
pub async fn get_indexed_files_grouped(
    state: State<'_, AppState>,
) -> Result<Vec<ImageVectorDataResponse>, String> {
    Ok(group_files_by_file_path(load_all_indexed_files(&state)))
}

/// Helper to count unique files by file_path grouping logic
fn count_unique_files_by_file_path(files: Vec<ImageVectorDataResponse>) -> usize {
    group_files_by_file_path(files).len()
}

/// Get indexed files grouped and paginated
#[tauri::command]
pub async fn get_indexed_files_grouped_paginated(
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<ImageVectorDataResponse>, String> {
    // First group the files
    let grouped_results = group_files_by_file_path(load_all_indexed_files(&state));

    // Then apply pagination to the grouped results
    let start = offset.min(grouped_results.len());
    let end = (offset + limit).min(grouped_results.len());
    let paginated_results = grouped_results
        .into_iter()
        .skip(start)
        .take(end - start)
        .collect();

    Ok(paginated_results)
}

/// Get count of indexed files
#[tauri::command]
pub async fn get_indexed_count(state: State<'_, AppState>) -> Result<usize, String> {
    match state.sqlite_service.get_semantic_file_count() {
        Ok(count) => Ok(count),
        Err(e) => {
            app_log_warn!(
                "⚠️ INDEX COUNT: Failed semantic count query ({}), using fallback aggregation",
                e
            );
            Ok(count_unique_files_by_file_path(load_all_indexed_files(&state)))
        }
    }
}

fn load_all_indexed_files(state: &State<'_, AppState>) -> Vec<ImageVectorDataResponse> {
    let mut combined = Vec::new();

    match state.sqlite_service.get_all_images() {
        Ok(mut image_rows) => combined.append(&mut image_rows),
        Err(e) => {
            app_log_warn!("⚠️ INDEXED FILES: Failed to load image rows: {}", e);
        }
    }

    match state.sqlite_service.get_all_text_file_entries() {
        Ok(mut text_rows) => combined.append(&mut text_rows),
        Err(e) => {
            app_log_warn!("⚠️ INDEXED FILES: Failed to load text rows: {}", e);
        }
    }

    // Keep one representative per file path.
    let mut by_path: HashMap<String, ImageVectorDataResponse> = HashMap::new();
    for file in combined {
        by_path
            .entry(file.file_path.clone())
            .and_modify(|existing| {
                if file.updated_at > existing.updated_at {
                    *existing = file.clone();
                }
            })
            .or_insert(file);
    }

    let mut deduped: Vec<ImageVectorDataResponse> = by_path.into_values().collect();
    deduped.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    deduped
}

/// Get index db directory
#[tauri::command]
pub fn get_indexed_directory(state: State<'_, AppState>) -> Result<(String, bool), String> {
    let dir = state.sqlite_service.get_db_path()?;
    Ok((dir.0.display().to_string(), dir.1))
}

// Set index db directory
#[tauri::command]
pub async fn set_indexed_directory(
    state: State<'_, AppState>,
    is_set_default: bool,
    new_dir: String,
) -> Result<String, String> {
    app_log_info!("🔧 SET_INDEX_DIR: Starting set_indexed_directory command");
    app_log_info!(
        "🔧 SET_INDEX_DIR: is_set_default = {}, new_dir = '{}'",
        is_set_default,
        new_dir
    );

    //List directory to check access
    if is_set_default {
        app_log_info!("🔧 SET_INDEX_DIR: Setting to default directory");
        match state.sqlite_service.set_db_path(None) {
            Ok(file_path) => {
                app_log_info!(
                    "✅ SET_INDEX_DIR: Successfully set to default: {}",
                    file_path
                );
                Ok(file_path)
            }
            Err(e) => {
                app_log_error!("❌ SET_INDEX_DIR: Failed to set default directory: {}", e);
                Err(e)
            }
        }
    } else {
        app_log_info!("🔧 SET_INDEX_DIR: Setting to custom directory: {}", new_dir);

        // Check directory access first
        match state.file_service.list_directory(&new_dir) {
            Ok(_) => {
                app_log_info!(
                    "✅ SET_INDEX_DIR: Directory access confirmed for: {}",
                    new_dir
                );

                match state.sqlite_service.set_db_path(Some(&new_dir)) {
                    Ok(file_path) => {
                        app_log_info!(
                            "✅ SET_INDEX_DIR: Successfully set database path: {}",
                            file_path
                        );
                        Ok(file_path)
                    }
                    Err(e) => {
                        app_log_error!("❌ SET_INDEX_DIR: Failed to set database path: {}", e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                app_log_error!(
                    "❌ SET_INDEX_DIR: Directory access failed for '{}': {}",
                    new_dir,
                    e
                );
                Err("Cosmos does not have access to this directory.".to_string())
            }
        }
    }
}

/// Clean stale entries from the index
#[tauri::command]
pub async fn clean_stale_entries(state: State<'_, AppState>) -> Result<String, String> {
    app_log_info!("🧹 CLEANUP: Starting to clean stale entries");

    // Get all indexed files from SQLite
    let indexed_files = match state.sqlite_service.get_all_images() {
        Ok(files) => files,
        Err(e) => {
            app_log_error!("❌ CLEANUP: Failed to get indexed files: {}", e);
            return Err(format!("Failed to get indexed files: {}", e));
        }
    };

    let mut removed_count = 0;
    let total_files = indexed_files.len();

    for file in indexed_files {
        // Check if file still exists
        if !std::path::Path::new(&file.file_path).exists() {
            match state.sqlite_service.delete_image_vector(&file.id) {
                Ok(_) => {
                    app_log_debug!("✅ Removed stale entry: {}", file.file_path);
                    removed_count += 1;
                }
                Err(e) => {
                    app_log_error!("❌ Failed to remove stale entry {}: {}", file.file_path, e);
                }
            }
        }
    }

    app_log_info!(
        "✅ CLEANUP: Removed {} stale entries out of {} total files",
        removed_count,
        total_files
    );
    Ok(format!("Removed {} stale entries", removed_count))
}

use crate::constants::{
    is_supported_image_extension, is_supported_text_extension, is_supported_video_extension,
};
use crate::services::database_service::DatabaseService;
use crate::services::sqlite_service::SqliteVectorService;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use rusqlite::OptionalExtension;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Emitter;
use uuid::Uuid;

const DEFAULT_SCAN_INTERVAL_SECONDS: u64 = 5;

#[derive(Debug, Clone, serde::Serialize)]
pub struct WatchedFolder {
    pub id: String,
    pub path: String,
    pub recursive: bool,
    pub enabled: bool,
    pub auto_transcribe_videos: bool,
    pub status: String,
    pub last_scan_at: Option<String>,
    pub last_event_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WatchedFolderScanResult {
    pub folder_id: String,
    pub path: String,
    pub scanned_files: usize,
    pub queued_files: usize,
    pub unchanged_files: usize,
    pub removed_files: usize,
    pub failed_files: usize,
    pub status: String,
    pub scanned_at: String,
}

pub struct WatchedFolderService {
    db_service: Arc<DatabaseService>,
}

impl WatchedFolderService {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    fn normalize_folder_path(path: &str) -> Result<String> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Folder path cannot be empty"));
        }

        let path_buf = PathBuf::from(trimmed);
        if path_buf.exists() {
            let canonical = path_buf.canonicalize()?;
            return Ok(canonical.to_string_lossy().to_string());
        }

        Ok(path_buf.to_string_lossy().to_string())
    }

    pub fn add_watched_folder(
        &self,
        path: &str,
        recursive: bool,
        auto_transcribe_videos: bool,
    ) -> Result<WatchedFolder> {
        let normalized_path = Self::normalize_folder_path(path)?;
        let folder_path = Path::new(&normalized_path);

        if !folder_path.exists() {
            return Err(anyhow!("Folder does not exist: {}", normalized_path));
        }
        if !folder_path.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", normalized_path));
        }

        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let existing: Option<(String, bool, bool)> = db
            .query_row(
                "SELECT id, recursive, auto_transcribe_videos
                 FROM watched_folders
                 WHERE path = ?1",
                rusqlite::params![normalized_path],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1, row.get::<_, i64>(2)? == 1)),
            )
            .optional()?;

        let id = match existing {
            Some((existing_id, _, _)) => {
                db.execute(
                    "UPDATE watched_folders
                     SET recursive = ?1,
                         enabled = 1,
                         auto_transcribe_videos = ?2,
                         status = 'watching',
                         updated_at = ?3
                     WHERE id = ?4",
                    rusqlite::params![
                        if recursive { 1 } else { 0 },
                        if auto_transcribe_videos { 1 } else { 0 },
                        now,
                        existing_id
                    ],
                )?;
                existing_id
            }
            None => {
                let new_id = format!("wf_{}", Uuid::new_v4().simple());
                db.execute(
                    "INSERT INTO watched_folders (
                        id, path, recursive, enabled, auto_transcribe_videos,
                        status, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, 1, ?4, 'watching', ?5, ?5)",
                    rusqlite::params![
                        new_id,
                        normalized_path,
                        if recursive { 1 } else { 0 },
                        if auto_transcribe_videos { 1 } else { 0 },
                        now
                    ],
                )?;
                new_id
            }
        };

        app_log_info!("👀 WATCH: Added/updated watched folder {}", normalized_path);
        self.get_watched_folder_by_id(&id)?
            .ok_or_else(|| anyhow!("Failed to load watched folder after insert"))
    }

    pub fn remove_watched_folder(&self, folder_id: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.execute(
            "DELETE FROM watched_folders WHERE id = ?1",
            rusqlite::params![folder_id],
        )?;
        app_log_info!("🗑️ WATCH: Removed watched folder {}", folder_id);
        Ok(())
    }

    pub fn set_watched_folder_enabled(&self, folder_id: &str, enabled: bool) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "UPDATE watched_folders
             SET enabled = ?1,
                 status = CASE WHEN ?1 = 1 THEN 'watching' ELSE 'paused' END,
                 updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![if enabled { 1 } else { 0 }, now, folder_id],
        )?;
        Ok(())
    }

    pub fn list_watched_folders(&self) -> Result<Vec<WatchedFolder>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, path, recursive, enabled, auto_transcribe_videos, status,
                    last_scan_at, last_event_at, created_at, updated_at
             FROM watched_folders
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![], |row| {
            Ok(WatchedFolder {
                id: row.get(0)?,
                path: row.get(1)?,
                recursive: row.get::<_, i64>(2)? == 1,
                enabled: row.get::<_, i64>(3)? == 1,
                auto_transcribe_videos: row.get::<_, i64>(4)? == 1,
                status: row.get(5)?,
                last_scan_at: row.get(6)?,
                last_event_at: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut folders = Vec::new();
        for row in rows {
            folders.push(row?);
        }
        Ok(folders)
    }

    pub fn get_watched_folder_by_id(&self, folder_id: &str) -> Result<Option<WatchedFolder>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.query_row(
            "SELECT id, path, recursive, enabled, auto_transcribe_videos, status,
                    last_scan_at, last_event_at, created_at, updated_at
             FROM watched_folders
             WHERE id = ?1",
            rusqlite::params![folder_id],
            |row| {
                Ok(WatchedFolder {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    recursive: row.get::<_, i64>(2)? == 1,
                    enabled: row.get::<_, i64>(3)? == 1,
                    auto_transcribe_videos: row.get::<_, i64>(4)? == 1,
                    status: row.get(5)?,
                    last_scan_at: row.get(6)?,
                    last_event_at: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub async fn scan_all_watched_folders(
        &self,
        sqlite_service: &Arc<SqliteVectorService>,
        app_handle: Option<&tauri::AppHandle>,
        force_scan: bool,
    ) -> Result<Vec<WatchedFolderScanResult>> {
        let folders = self.list_watched_folders()?;
        let enabled_folders: Vec<WatchedFolder> =
            folders.into_iter().filter(|folder| folder.enabled).collect();

        let mut results = Vec::with_capacity(enabled_folders.len());
        for folder in enabled_folders {
            match self
                .scan_watched_folder(&folder, sqlite_service, app_handle, force_scan)
                .await
            {
                Ok(result) => results.push(result),
                Err(e) => {
                    app_log_error!(
                        "❌ WATCH: Failed scanning watched folder {}: {}",
                        folder.path,
                        e
                    );
                }
            }
        }

        Ok(results)
    }

    pub async fn scan_watched_folder_by_id(
        &self,
        folder_id: &str,
        sqlite_service: &Arc<SqliteVectorService>,
        app_handle: Option<&tauri::AppHandle>,
        force_scan: bool,
    ) -> Result<WatchedFolderScanResult> {
        let folder = self
            .get_watched_folder_by_id(folder_id)?
            .ok_or_else(|| anyhow!("Watched folder not found: {}", folder_id))?;

        self.scan_watched_folder(&folder, sqlite_service, app_handle, force_scan)
            .await
    }

    async fn scan_watched_folder(
        &self,
        folder: &WatchedFolder,
        sqlite_service: &Arc<SqliteVectorService>,
        app_handle: Option<&tauri::AppHandle>,
        force_scan: bool,
    ) -> Result<WatchedFolderScanResult> {
        if !force_scan && !self.should_scan_folder(folder)? {
            return Ok(WatchedFolderScanResult {
                folder_id: folder.id.clone(),
                path: folder.path.clone(),
                scanned_files: 0,
                queued_files: 0,
                unchanged_files: 0,
                removed_files: 0,
                failed_files: 0,
                status: "skipped".to_string(),
                scanned_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        self.update_folder_status(&folder.id, "scanning", None)?;

        let scan_started_at = chrono::Utc::now().to_rfc3339();
        let scan_started_unix_ms = chrono::Utc::now().timestamp_millis();
        let mut scanned_files = 0usize;
        let mut queued_files = 0usize;
        let mut unchanged_files = 0usize;
        let mut failed_files = 0usize;

        let discovered_files = self.discover_indexable_files(&folder.path, folder.recursive)?;
        if discovered_files.is_empty() {
            app_log_debug!(
                "👀 WATCH: No supported indexable files found in {} (recursive={})",
                folder.path,
                folder.recursive
            );
        }
        for file_path in discovered_files {
            scanned_files += 1;
            let metadata = match std::fs::metadata(&file_path) {
                Ok(metadata) => metadata,
                Err(e) => {
                    failed_files += 1;
                    app_log_warn!(
                        "⚠️ WATCH: Failed to read metadata for {}: {}",
                        file_path.display(),
                        e
                    );
                    continue;
                }
            };

            let file_size = metadata.len() as i64;
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|time| {
                    time.duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|duration| duration.as_millis() as i64)
                })
                .unwrap_or(0);
            let signature = format!("{}:{}", file_size, modified_ms);
            let file_path_str = file_path.to_string_lossy().to_string();

            let previous_signature = self.get_file_signature(&folder.id, &file_path_str)?;
            let is_changed = previous_signature
                .as_deref()
                .map(|sig| sig != signature)
                .unwrap_or(true);

            let mut job_id: Option<String> = None;
            if is_changed {
                match sqlite_service.create_job("file", &file_path_str, Some(1)) {
                    Ok(created_job_id) => {
                        queued_files += 1;
                        job_id = Some(created_job_id.clone());

                        if let Some(handle) = app_handle {
                            let payload = serde_json::json!({
                                "folder_id": folder.id,
                                "file_path": file_path_str,
                                "job_id": created_job_id,
                                "event_type": "queued"
                            });
                            let _ = handle.emit("watched_folder_activity", payload);
                        }
                    }
                    Err(e) => {
                        failed_files += 1;
                        app_log_error!(
                            "❌ WATCH: Failed to create indexing job for {}: {}",
                            file_path_str,
                            e
                        );
                    }
                }
            } else {
                unchanged_files += 1;
            }

            self.upsert_file_state(
                &folder.id,
                &file_path_str,
                file_size,
                modified_ms,
                &signature,
                scan_started_unix_ms,
                job_id.as_deref(),
            )?;
        }

        let removed_files = self.remove_stale_file_state(&folder.id, scan_started_unix_ms)?;
        let last_event_at = if queued_files > 0 {
            Some(scan_started_at.clone())
        } else {
            None
        };
        self.update_folder_status(&folder.id, "watching", last_event_at.as_deref())?;
        self.set_last_scan_at(&folder.id, &scan_started_at)?;

        let result = WatchedFolderScanResult {
            folder_id: folder.id.clone(),
            path: folder.path.clone(),
            scanned_files,
            queued_files,
            unchanged_files,
            removed_files,
            failed_files,
            status: "completed".to_string(),
            scanned_at: scan_started_at.clone(),
        };

        if let Some(handle) = app_handle {
            let _ = handle.emit("watched_folder_scan_progress", &result);
            if let Ok(updated_folder) = self.get_watched_folder_by_id(&folder.id) {
                if let Some(updated_folder) = updated_folder {
                    let _ = handle.emit("watched_folder_updated", updated_folder);
                }
            }
        }

        app_log_info!(
            "👀 WATCH: Scanned {} (scanned={}, queued={}, unchanged={}, removed={}, failed={})",
            folder.path,
            scanned_files,
            queued_files,
            unchanged_files,
            removed_files,
            failed_files
        );

        Ok(result)
    }

    fn should_scan_folder(&self, folder: &WatchedFolder) -> Result<bool> {
        if folder.last_scan_at.is_none() {
            return Ok(true);
        }

        let last_scan = folder
            .last_scan_at
            .as_ref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&chrono::Utc));
        let Some(last_scan) = last_scan else {
            return Ok(true);
        };

        let elapsed = chrono::Utc::now()
            .signed_duration_since(last_scan)
            .num_seconds()
            .max(0) as u64;
        Ok(elapsed >= DEFAULT_SCAN_INTERVAL_SECONDS)
    }

    fn discover_indexable_files(&self, folder_path: &str, recursive: bool) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let max_files = crate::commands::indexing::MAX_FILES_PER_SCAN;

        if recursive {
            for entry in walkdir::WalkDir::new(folder_path)
                .follow_links(false) // Prevent symlink loops (macOS /System, etc.)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if files.len() >= max_files {
                    app_log_warn!(
                        "⚠️ WATCH: Hit scan cap of {} files in {}",
                        max_files,
                        folder_path
                    );
                    break;
                }

                if !entry.file_type().is_file() {
                    continue;
                }

                let file_name = entry.file_name().to_string_lossy().to_string();
                if is_hidden_or_system_name(&file_name) {
                    continue;
                }

                let path = entry.path().to_path_buf();
                if is_supported_indexable_path(&path) {
                    files.push(path);
                }
            }
        } else {
            for entry in std::fs::read_dir(folder_path)? {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = entry.file_name().to_string_lossy().to_string();
                if is_hidden_or_system_name(&file_name) {
                    continue;
                }

                if is_supported_indexable_path(&path) {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    fn get_file_signature(
        &self,
        folder_id: &str,
        file_path: &str,
    ) -> Result<Option<String>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.query_row(
            "SELECT content_sig
             FROM watched_folder_file_state
             WHERE watched_folder_id = ?1 AND file_path = ?2",
            rusqlite::params![folder_id, file_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    fn upsert_file_state(
        &self,
        folder_id: &str,
        file_path: &str,
        file_size: i64,
        mtime_unix_ms: i64,
        content_sig: &str,
        last_seen_at_ms: i64,
        last_index_job_id: Option<&str>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        db.execute(
            "INSERT INTO watched_folder_file_state (
                watched_folder_id, file_path, file_size, mtime_unix_ms, content_sig, last_seen_at, last_index_job_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT (watched_folder_id, file_path) DO UPDATE SET
                file_size = excluded.file_size,
                mtime_unix_ms = excluded.mtime_unix_ms,
                content_sig = excluded.content_sig,
                last_seen_at = excluded.last_seen_at,
                last_index_job_id = COALESCE(excluded.last_index_job_id, watched_folder_file_state.last_index_job_id)",
            rusqlite::params![
                folder_id,
                file_path,
                file_size,
                mtime_unix_ms,
                content_sig,
                last_seen_at_ms,
                last_index_job_id
            ],
        )?;
        Ok(())
    }

    fn remove_stale_file_state(&self, folder_id: &str, last_seen_at_ms: i64) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let removed = db.execute(
            "DELETE FROM watched_folder_file_state
             WHERE watched_folder_id = ?1
               AND last_seen_at < ?2",
            rusqlite::params![folder_id, last_seen_at_ms],
        )?;
        Ok(removed)
    }

    fn update_folder_status(
        &self,
        folder_id: &str,
        status: &str,
        last_event_at: Option<&str>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "UPDATE watched_folders
             SET status = ?1,
                 last_event_at = COALESCE(?2, last_event_at),
                 updated_at = ?3
             WHERE id = ?4",
            rusqlite::params![status, last_event_at, now, folder_id],
        )?;
        Ok(())
    }

    fn set_last_scan_at(&self, folder_id: &str, last_scan_at: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "UPDATE watched_folders
             SET last_scan_at = ?1,
                 updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![last_scan_at, now, folder_id],
        )?;
        Ok(())
    }
}

/// Base poll interval in seconds (when queue is small or empty).
const POLL_INTERVAL_BASE_SECS: u64 = 10;
/// Backoff poll interval when queue is under heavy load (>10K pending jobs).
const POLL_INTERVAL_LOADED_SECS: u64 = 60;
/// Pending job threshold that triggers backoff.
const QUEUE_LOAD_THRESHOLD: i64 = 10_000;

pub async fn run_watched_folder_monitor_loop(
    watched_folder_service: Arc<WatchedFolderService>,
    sqlite_service: Arc<SqliteVectorService>,
    app_handle: tauri::AppHandle,
) {
    app_log_info!("👀 WATCH: Starting watched folder monitor loop (adaptive interval)");

    loop {
        // Adaptive backoff: check queue pressure and adjust poll interval.
        let poll_secs = match sqlite_service.get_pending_job_count() {
            Ok(count) if count >= QUEUE_LOAD_THRESHOLD => {
                app_log_debug!(
                    "👀 WATCH: Queue loaded ({} pending), backing off to {}s poll",
                    count,
                    POLL_INTERVAL_LOADED_SECS
                );
                POLL_INTERVAL_LOADED_SECS
            }
            _ => POLL_INTERVAL_BASE_SECS,
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(poll_secs)).await;

        match watched_folder_service
            .scan_all_watched_folders(&sqlite_service, Some(&app_handle), false)
            .await
        {
            Ok(results) => {
                if !results.is_empty() {
                    app_log_debug!(
                        "👀 WATCH: Completed watched folder polling cycle for {} folder(s)",
                        results.len()
                    );
                }
            }
            Err(e) => {
                app_log_warn!("⚠️ WATCH: Monitor loop scan error: {}", e);
            }
        }
    }
}

fn is_hidden_or_system_name(name: &str) -> bool {
    name.starts_with('.')
        || name.ends_with(".app")
        || name.ends_with(".framework")
        || name.ends_with(".xpc")
        || name.ends_with(".bundle")
        || name.ends_with(".plugin")
        || name.ends_with(".kext")
        || name.ends_with(".dSYM")
        || name == "DS_Store"
        || name == "Thumbs.db"
        || name == "desktop.ini"
        || crate::commands::indexing::EXCLUDED_DIR_NAMES.contains(&name)
}

fn is_supported_indexable_path(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if extension.is_empty() {
        return false;
    }

    is_supported_image_extension(&extension)
        || is_supported_video_extension(&extension)
        || is_supported_text_extension(&extension)
}

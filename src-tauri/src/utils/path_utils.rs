use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use dirs;

/// Recursively copy a directory and its contents
fn copy_directory_recursive(src: &Path, dst: &Path) -> Result<()> {
    // Create destination directory
    fs::create_dir_all(dst)
        .map_err(|e| anyhow!("Failed to create destination directory: {}", e))?;

    // Read source directory
    let entries = fs::read_dir(src)
        .map_err(|e| anyhow!("Failed to read source directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Recursively copy subdirectory
            copy_directory_recursive(&src_path, &dst_path)?;
        } else {
            // Copy file
            fs::copy(&src_path, &dst_path)
                .map_err(|e| anyhow!("Failed to copy file {:?} to {:?}: {}", src_path, dst_path, e))?;
        }
    }

    Ok(())
}

/// Migrate data from old app directory to new one
/// This should be called early in app startup, before any other data operations
/// Check if this is a migration scenario (desktop-docs exists)
pub fn is_migration_needed() -> bool {
    if let Some(data_local_dir) = dirs::data_local_dir() {
        data_local_dir.join("desktop-docs").exists()
    } else {
        false
    }
}

pub fn migrate_app_data_if_needed() -> Result<()> {
    let data_local_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Could not determine data directory"))?;

    let old_dir = data_local_dir.join("desktop-docs");
    let new_dir = data_local_dir.join("cosmos");

    // Simple logic: if desktop-docs exists, migrate it
    if old_dir.exists() {
        log::info!("Found desktop-docs directory, migrating to cosmos");

        // Remove cosmos directory if it exists (shouldn't exist in migration scenario)
        if new_dir.exists() {
            log::info!("Removing existing cosmos directory to allow migration");
            if let Err(e) = fs::remove_dir_all(&new_dir) {
                log::error!("Failed to remove existing cosmos directory: {}", e);
            }
        }

        // Attempt atomic rename first
        match fs::rename(&old_dir, &new_dir) {
            Ok(_) => {
                log::info!("Successfully migrated app data directory using atomic rename");
                Ok(())
            },
            Err(e) => {
                log::error!("Failed to migrate with rename: {}", e);
                log::info!("Attempting fallback copy-and-delete migration...");

                // Fallback: copy and delete
                match copy_directory_recursive(&old_dir, &new_dir) {
                    Ok(_) => {
                        log::info!("Successfully copied data to new directory");
                        if let Err(e) = fs::remove_dir_all(&old_dir) {
                            log::warn!("Failed to remove old directory after copy: {}", e);
                        }
                        Ok(())
                    },
                    Err(copy_err) => {
                        log::error!("Failed to copy directory: {}", copy_err);
                        Err(anyhow!("Data migration failed: rename failed ({}), copy failed ({})", e, copy_err))
                    }
                }
            }
        }
    } else {
        log::debug!("No desktop-docs directory found, starting fresh");
        Ok(())
    }
}

/// Get the application data directory (post-migration)
pub fn get_app_data_dir() -> Result<PathBuf> {
    // Check for environment variable first (for testing)
    if let Ok(env_path) = std::env::var("COSMOS_APP_DATA_DIR") {
        return Ok(PathBuf::from(env_path));
    }

    dirs::data_local_dir()
        .map(|p| p.join("cosmos"))
        .ok_or_else(|| anyhow!("Could not determine application data directory"))
}

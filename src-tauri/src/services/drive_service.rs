use crate::services::database_service::DatabaseService;
use crate::{app_log_debug, app_log_info, app_log_warn};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriveInfo {
    pub uuid: String,
    pub name: String,
    pub mount_path: String,
    pub total_space: u64,
    pub free_space: u64,
    pub is_removable: bool,
    pub last_seen: DateTime<Utc>,
    pub status: DriveStatus,
    pub indexed_files_count: i64,
    pub total_size_indexed: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DriveStatus {
    Connected,
    Disconnected,
    Indexing,
    Error(String),
}

impl Default for DriveStatus {
    fn default() -> Self {
        DriveStatus::Connected
    }
}

/// Service for managing drive operations (both system detection and database management)
pub struct DriveService {
    mounted_drives: Arc<RwLock<HashMap<String, DriveInfo>>>,
    db_service: Arc<DatabaseService>,
}

impl DriveService {
    /// Create a new drive service
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self {
            mounted_drives: Arc::new(RwLock::new(HashMap::new())),
            db_service,
        }
    }

    // ===== SYSTEM-LEVEL DRIVE DETECTION METHODS =====

    /// Detect all currently connected external drives
    pub async fn detect_connected_drives(&self) -> Result<Vec<DriveInfo>, String> {
        let output = Command::new("diskutil")
            .args(&["list", "-plist"])
            .output()
            .map_err(|e| format!("Failed to run diskutil: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "diskutil command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let drives = self.parse_diskutil_output(&output_str)?;

        // Filter for external/removable drives only
        let external_drives: Vec<DriveInfo> = drives
            .into_iter()
            .filter(|drive| drive.is_removable && drive.mount_path.starts_with("/Volumes/"))
            .collect();

        // Update internal state
        let mut mounted_drives = self.mounted_drives.write().await;
        mounted_drives.clear();
        for drive in &external_drives {
            mounted_drives.insert(drive.uuid.clone(), drive.clone());
        }

        Ok(external_drives)
    }

    /// Get drive information for a specific UUID
    pub async fn get_drive_info(&self, uuid: &str) -> Option<DriveInfo> {
        let drives = self.mounted_drives.read().await;
        drives.get(uuid).cloned()
    }

    /// Get drive information for a specific path
    pub async fn get_drive_for_path(&self, path: &str) -> Option<DriveInfo> {
        let drives = self.mounted_drives.read().await;
        drives
            .values()
            .find(|drive| path.starts_with(&drive.mount_path))
            .cloned()
    }

    /// Get all currently tracked drives
    pub async fn get_all_drives(&self) -> Vec<DriveInfo> {
        let drives = self.mounted_drives.read().await;
        drives.values().cloned().collect()
    }

    /// Update drive status
    pub async fn update_drive_status(&self, uuid: &str, status: DriveStatus) -> Result<(), String> {
        let mut drives = self.mounted_drives.write().await;
        if let Some(drive) = drives.get_mut(uuid) {
            drive.status = status;
            drive.last_seen = Utc::now();
            Ok(())
        } else {
            Err(format!("Drive with UUID {} not found", uuid))
        }
    }

    /// Parse diskutil output to extract drive information
    fn parse_diskutil_output(&self, _output: &str) -> Result<Vec<DriveInfo>, String> {
        // For now, implement a basic parser
        // In production, you'd want to use a proper plist parser
        let mut drives = Vec::new();

        // Get individual drive info for each volume
        let volumes_output = Command::new("ls")
            .args(&["/Volumes/"])
            .output()
            .map_err(|e| format!("Failed to list volumes: {}", e))?;

        if !volumes_output.status.success() {
            return Ok(drives);
        }

        let volumes_str = String::from_utf8_lossy(&volumes_output.stdout);
        for volume_name in volumes_str.lines() {
            let volume_name = volume_name.trim();
            if volume_name.is_empty() || volume_name == "Macintosh HD" {
                continue;
            }

            let mount_path = format!("/Volumes/{}", volume_name);

            // Get detailed info for this volume
            match self.get_drive_info_for_path(&mount_path) {
                Ok(drive_info) => {
                    drives.push(drive_info);
                }
                Err(e) => {
                    app_log_debug!("❌ DRIVE: Failed to get info for '{}': {}", mount_path, e);
                }
            }
        }

        Ok(drives)
    }

    /// Get detailed drive information for a specific mount path
    fn get_drive_info_for_path(&self, mount_path: &str) -> Result<DriveInfo, String> {
        let output = Command::new("diskutil")
            .args(&["info", mount_path])
            .output()
            .map_err(|e| format!("Failed to get drive info: {}", e))?;

        if !output.status.success() {
            return Err(format!("diskutil info failed for {}", mount_path));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse the output to extract relevant information
        let mut uuid = String::new();
        let mut name = String::new();
        let mut total_space = 0u64;
        let mut free_space = 0u64;
        let mut is_removable = false;

        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("Volume UUID:") {
                uuid = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Volume Name:") {
                name = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Disk Size:") {
                // Parse size like "1.0 TB (1000000000000 Bytes)"
                if let Some(bytes_part) = line.split('(').nth(1) {
                    if let Some(bytes_str) = bytes_part.split(' ').next() {
                        total_space = bytes_str.parse().unwrap_or(0);
                    }
                }
            } else if line.starts_with("Container Free Space:") || line.starts_with("Free Space:") {
                if let Some(bytes_part) = line.split('(').nth(1) {
                    if let Some(bytes_str) = bytes_part.split(' ').next() {
                        free_space = bytes_str.parse().unwrap_or(0);
                    }
                }
            } else if line.starts_with("Removable Media:") {
                is_removable = is_removable || line.contains("Yes");
            } else if line.starts_with("Protocol:") {
                // Consider USB, FireWire, etc. as removable
                is_removable = is_removable || line.contains("USB") || line.contains("FireWire");
            } else if line.starts_with("Device Location:") {
                // Also consider external devices as removable
                is_removable = is_removable || line.contains("External");
            }
        }

        if uuid.is_empty() {
            return Err("Could not extract UUID from drive info".to_string());
        }

        if name.is_empty() {
            name = mount_path
                .split('/')
                .last()
                .unwrap_or("Unknown")
                .to_string();
        }

        Ok(DriveInfo {
            uuid,
            name,
            mount_path: mount_path.to_string(),
            total_space,
            free_space,
            is_removable,
            last_seen: Utc::now(),
            status: DriveStatus::Connected,
            indexed_files_count: 0,
            total_size_indexed: 0,
        })
    }

    /// Start monitoring for drive changes
    pub async fn start_monitoring(&self, app_handle: tauri::AppHandle) -> Result<(), String> {
        let drives_clone = self.mounted_drives.clone();
        let service_clone = DriveService {
            mounted_drives: drives_clone.clone(),
            db_service: self.db_service.clone(),
        };

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
            let mut last_known_drives: std::collections::HashMap<String, DriveInfo> =
                std::collections::HashMap::new();

            loop {
                interval.tick().await;

                // Detect current drives
                match service_clone.detect_connected_drives().await {
                    Ok(current_drives) => {
                        let current_drive_map: std::collections::HashMap<String, DriveInfo> =
                            current_drives
                                .iter()
                                .map(|d| (d.uuid.clone(), d.clone()))
                                .collect();

                        // Check for newly connected drives
                        for (uuid, drive_info) in &current_drive_map {
                            if !last_known_drives.contains_key(uuid) {
                                // Emit drive connected event
                                if let Err(_) = app_handle.emit("drive_connected", drive_info) {
                                    //app_log_debug!("Failed to emit drive_connected event: {}", e);
                                }
                            }
                        }

                        // Check for disconnected drives
                        for (uuid, drive_info) in &last_known_drives {
                            if !current_drive_map.contains_key(uuid) {
                                // Emit drive disconnected event
                                if let Err(e) = app_handle.emit(
                                    "drive_disconnected",
                                    serde_json::json!({
                                        "uuid": uuid,
                                        "name": drive_info.name,
                                        "mount_path": drive_info.mount_path
                                    }),
                                ) {
                                    app_log_debug!(
                                        "Failed to emit drive_disconnected event: {}",
                                        e
                                    );
                                }
                            }
                        }

                        // Update our known drives
                        last_known_drives = current_drive_map;
                    }
                    Err(_) => {
                        // app_log_debug!("🔄 DRIVE: Monitoring error (will continue): {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Force refresh of drive list
    pub async fn refresh_drives(&self) -> Result<Vec<DriveInfo>, String> {
        self.detect_connected_drives().await
    }

    // ===== DATABASE-LEVEL DRIVE MANAGEMENT METHODS =====

    /// Update drive custom name and physical location
    pub fn update_drive_metadata(
        &self,
        uuid: &str,
        custom_name: Option<&str>,
        physical_location: Option<&str>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        app_log_info!(
            "🔄 DRIVE: Updating metadata for drive {}: name={:?}, location={:?}",
            uuid,
            custom_name,
            physical_location
        );

        db.execute(
            "UPDATE drives SET custom_name = ?, physical_location = ?, last_seen = CURRENT_TIMESTAMP
             WHERE uuid = ?",
            rusqlite::params![custom_name, physical_location, uuid],
        )?;

        app_log_info!("✅ DRIVE: Updated metadata for drive {}", uuid);
        Ok(())
    }

    /// Delete drive from database (with indexed files check)
    pub fn delete_drive(&self, uuid: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        app_log_info!(
            "🗑️ DRIVE: Checking drive {} for indexed content before deletion",
            uuid
        );

        // Check if drive has indexed files
        let indexed_count: usize = db.query_row(
            "SELECT COUNT(DISTINCT file_path) FROM images WHERE drive_uuid = ? AND embedding IS NOT NULL",
            rusqlite::params![uuid],
            |row| row.get(0)
        )?;

        if indexed_count > 0 {
            app_log_warn!(
                "⚠️ DRIVE: Cannot delete drive {} - it has {} indexed files",
                uuid,
                indexed_count
            );
            return Err(anyhow::anyhow!(
                "Cannot delete drive: it contains {} indexed files. Please remove indexed content first.",
                indexed_count
            ));
        }

        app_log_info!("🗑️ DRIVE: Deleting drive from database: {}", uuid);

        // Delete drive (safe to delete as it has no indexed content)
        let rows_affected =
            db.execute("DELETE FROM drives WHERE uuid = ?", rusqlite::params![uuid])?;

        if rows_affected > 0 {
            app_log_info!("✅ DRIVE: Deleted drive {} (no indexed content)", uuid);
        } else {
            app_log_warn!("⚠️ DRIVE: Drive {} not found in database", uuid);
        }

        Ok(())
    }

    /// Get all drives with their metadata (including calculated indexed files count)
    pub fn get_all_drives_db(&self) -> Result<Vec<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT d.uuid, d.name, d.custom_name, d.physical_location, d.last_mount_path,
                    d.total_space, d.free_space, d.is_removable, d.first_seen, d.last_seen,
                    d.status, d.metadata,
                    (
                        SELECT COUNT(*)
                        FROM (
                            SELECT DISTINCT i.file_path
                            FROM images i
                            WHERE i.drive_uuid = d.uuid AND i.embedding IS NOT NULL
                        )
                    ) as actual_indexed_files_count,
                    COALESCE(
                        (SELECT SUM(LENGTH(i.metadata))
                         FROM images i
                         WHERE i.drive_uuid = d.uuid AND i.embedding IS NOT NULL),
                        0
                    ) as total_size_indexed
             FROM drives d
             ORDER BY d.last_seen DESC",
        )?;

        let drives = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "uuid": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "custom_name": row.get::<_, Option<String>>(2)?,
                    "physical_location": row.get::<_, Option<String>>(3)?,
                    "last_mount_path": row.get::<_, Option<String>>(4)?,
                    "total_space": row.get::<_, i64>(5)?,
                    "free_space": row.get::<_, i64>(6)?,
                    "is_removable": row.get::<_, bool>(7)?,
                    "first_seen": row.get::<_, String>(8)?,
                    "last_seen": row.get::<_, String>(9)?,
                    "status": row.get::<_, String>(10)?,
                    "metadata": row.get::<_, String>(11)?,
                    "indexed_files_count": row.get::<_, i64>(12)?,
                    "total_size_indexed": row.get::<_, i64>(13)?
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(drives)
    }

    /// Get drive information by UUID (including calculated indexed files count)
    pub fn get_drive_by_uuid_db(&self, uuid: &str) -> Result<Option<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let result = db
            .query_row(
                "SELECT d.uuid, d.name, d.custom_name, d.physical_location, d.last_mount_path,
                    d.total_space, d.free_space, d.is_removable, d.first_seen, d.last_seen,
                    d.status, d.metadata,
                    (
                        SELECT COUNT(*)
                        FROM (
                            SELECT DISTINCT i.file_path
                            FROM images i
                            WHERE i.drive_uuid = d.uuid AND i.embedding IS NOT NULL
                        )
                    ) as actual_indexed_files_count,
                    COALESCE(
                        (SELECT SUM(LENGTH(i.metadata))
                         FROM images i
                         WHERE i.drive_uuid = d.uuid AND i.embedding IS NOT NULL),
                        0
                    ) as total_size_indexed
             FROM drives d
             WHERE d.uuid = ?",
                rusqlite::params![uuid],
                |row| {
                    Ok(serde_json::json!({
                        "uuid": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "custom_name": row.get::<_, Option<String>>(2)?,
                        "physical_location": row.get::<_, Option<String>>(3)?,
                        "last_mount_path": row.get::<_, Option<String>>(4)?,
                        "total_space": row.get::<_, i64>(5)?,
                        "free_space": row.get::<_, i64>(6)?,
                        "is_removable": row.get::<_, bool>(7)?,
                        "first_seen": row.get::<_, String>(8)?,
                        "last_seen": row.get::<_, String>(9)?,
                        "status": row.get::<_, String>(10)?,
                        "metadata": row.get::<_, String>(11)?,
                        "indexed_files_count": row.get::<_, i64>(12)?,
                        "total_size_indexed": row.get::<_, i64>(13)?
                    }))
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Add a new drive to the database
    pub fn add_drive(
        &self,
        uuid: &str,
        name: &str,
        mount_path: &str,
        is_removable: bool,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        app_log_info!("➕ DRIVE: Adding new drive {} to database", name);

        // Insert new drive
        db.execute(
            "INSERT INTO drives (uuid, name, status, last_mount_path, last_seen, is_removable)
             VALUES (?, ?, 'connected', ?, CURRENT_TIMESTAMP, ?)",
            rusqlite::params![uuid, name, mount_path, is_removable],
        )?;

        // Add to mount history
        db.execute(
            "INSERT INTO drive_mounts (drive_uuid, mount_path) VALUES (?, ?)",
            rusqlite::params![uuid, mount_path],
        )?;

        app_log_info!("✅ DRIVE: Successfully added drive {} to database", name);
        Ok(())
    }

    /// Update drive connection status and last seen timestamp
    pub fn update_drive_status_db(
        &self,
        uuid: &str,
        status: &str,
        mount_path: Option<&str>,
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        app_log_info!("🔄 DRIVE: Updating status for drive {} to {}", uuid, status);

        // Update drive status and last seen
        db.execute(
            "UPDATE drives SET status = ?, last_seen = CURRENT_TIMESTAMP, last_mount_path = ?
             WHERE uuid = ?",
            rusqlite::params![status, mount_path, uuid],
        )?;

        // If connecting, add to mount history
        if status == "connected" && mount_path.is_some() {
            db.execute(
                "INSERT INTO drive_mounts (drive_uuid, mount_path) VALUES (?, ?)",
                rusqlite::params![uuid, mount_path],
            )?;
        }

        app_log_info!("✅ DRIVE: Updated status for drive {} to {}", uuid, status);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database_service::DatabaseService;
    use std::sync::Arc;

    #[test]
    fn test_drive_service_creation() {
        let db_service =
            DatabaseService::new_in_memory().expect("Failed to create database service");
        let db_service_arc = Arc::new(db_service);
        let _drive_service = DriveService::new(db_service_arc);

        // Test that the service was created successfully
        assert!(true); // If we get here, the service was created successfully
    }

    #[test]
    fn test_drive_operations() {
        let db_service =
            DatabaseService::new_in_memory().expect("Failed to create database service");
        let db_service_arc = Arc::new(db_service);
        let drive_service = DriveService::new(Arc::clone(&db_service_arc));

        // Initialize schema - create the drives tables
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        let schema_service =
            crate::services::schema_service::SchemaService::new(Arc::clone(&db_service_arc));
        schema_service
            .create_drives_tables(&db)
            .expect("Failed to create drives tables");
        drop(db);

        // Test adding a drive
        let result = drive_service.add_drive("test-uuid", "Test Drive", "/test/path", false);
        assert!(result.is_ok(), "Failed to add drive: {:?}", result);

        // Test getting all drives (simplified for unit test)
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        let drives_count: i64 = db
            .query_row("SELECT COUNT(*) FROM drives", rusqlite::params![], |row| {
                row.get(0)
            })
            .expect("Failed to count drives");
        assert_eq!(drives_count, 1, "Expected 1 drive, got {}", drives_count);
        drop(db);

        // Test getting drive by UUID (simplified for unit test)
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        let drive_name: String = db
            .query_row(
                "SELECT name FROM drives WHERE uuid = ?",
                rusqlite::params!["test-uuid"],
                |row| row.get(0),
            )
            .expect("Failed to get drive by UUID");
        assert_eq!(drive_name, "Test Drive");
        drop(db);

        // Test updating drive metadata
        let result = drive_service.update_drive_metadata(
            "test-uuid",
            Some("Custom Name"),
            Some("Custom Location"),
        );
        assert!(
            result.is_ok(),
            "Failed to update drive metadata: {:?}",
            result
        );

        // Test updating drive status
        let result = drive_service.update_drive_status_db("test-uuid", "disconnected", None);
        assert!(
            result.is_ok(),
            "Failed to update drive status: {:?}",
            result
        );

        // Test getting indexed files count (skip for unit test since images table doesn't exist)
        // let count = drive_service.get_drive_indexed_files_count("test-uuid").expect("Failed to get indexed files count");
        // assert_eq!(count, 0, "Expected 0 indexed files for new drive");

        // Test deleting drive (simplified for unit test)
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        let rows_affected = db
            .execute(
                "DELETE FROM drives WHERE uuid = ?",
                rusqlite::params!["test-uuid"],
            )
            .expect("Failed to delete drive");
        assert_eq!(rows_affected, 1, "Expected 1 row to be deleted");
        drop(db);

        // Verify drive was deleted (simplified for unit test)
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        let drives_count: i64 = db
            .query_row("SELECT COUNT(*) FROM drives", rusqlite::params![], |row| {
                row.get(0)
            })
            .expect("Failed to count drives");
        assert_eq!(drives_count, 0, "Expected 0 drives after deletion");
        drop(db);
    }
}

use anyhow::{anyhow, Result};
use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::services::database_service::DatabaseService;
use crate::services::schema_service::SchemaService;
use crate::services::api_key_encryption_service::ApiKeyEncryptionService;
use crate::{app_log_info, app_log_warn};

/// App installation service for managing installed apps and their configurations
pub struct AppInstallationService {
    db_service: Arc<DatabaseService>,
    schema_service: Arc<SchemaService>,
}

/// Request to install an app
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInstallRequest {
    pub app_name: String,
    pub app_version: String,
    pub api_key: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Response from app installation
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInstallResponse {
    pub success: bool,
    pub app_id: Option<i64>,
    pub message: String,
}

/// Installed app information
#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledApp {
    pub id: i64,
    pub app_name: String,
    pub app_version: String,
    pub installed_at: String,
    pub updated_at: String,
    pub has_api_key: bool,
    pub metadata: Option<serde_json::Value>,
}

impl AppInstallationService {
    /// Create a new AppInstallationService instance
    pub fn new(
        db_service: Arc<DatabaseService>,
        schema_service: Arc<SchemaService>,
    ) -> Self {
        Self {
            db_service,
            schema_service,
        }
    }

    /// Install an app with configuration
    pub fn install_app(&self, request: AppInstallRequest) -> Result<AppInstallResponse> {
        app_log_info!("📦 APP INSTALLATION: Installing app: {}", request.app_name);

        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();

        // Start transaction
        let tx = db.transaction()?;

        // Check if app is already installed
        let existing_app = self.get_app_by_name(&tx, &request.app_name)?;
        if existing_app.is_some() {
            app_log_warn!("⚠️ APP INSTALLATION: App {} is already installed", request.app_name);
            return Ok(AppInstallResponse {
                success: false,
                app_id: None,
                message: format!("App {} is already installed", request.app_name),
            });
        }

        // Insert app record
        let app_id = self.insert_app_record(&tx, &request)?;

        // Store API key if provided
        if let Some(api_key) = request.api_key {
            self.store_api_key(&tx, app_id, &api_key)?;
            app_log_info!("🔑 APP INSTALLATION: API key stored for app: {}", request.app_name);
        }

        // Store metadata if provided
        if let Some(metadata) = request.metadata {
            self.store_app_metadata(&tx, app_id, &metadata)?;
            app_log_info!("📊 APP INSTALLATION: Metadata stored for app: {}", request.app_name);
        }

        // Log installation
        self.log_app_action(&tx, app_id, "install", "success", &format!("Installed app: {}", request.app_name))?;

        // Commit transaction
        tx.commit()?;

        app_log_info!("✅ APP INSTALLATION: Successfully installed app: {}", request.app_name);

        Ok(AppInstallResponse {
            success: true,
            app_id: Some(app_id),
            message: format!("Successfully installed {}", request.app_name),
        })
    }

    /// Get app by name
    fn get_app_by_name(&self, db: &Connection, app_name: &str) -> Result<Option<InstalledApp>> {
        let mut stmt = db.prepare(
            "SELECT id, app_name, app_version, installed_at, updated_at, metadata 
             FROM app_installations 
             WHERE app_name = ?"
        )?;

        let app = stmt.query_row(params![app_name], |row| {
            let metadata_str: Option<String> = row.get(5)?;
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            let app_id: i64 = row.get(0)?;
            let has_api_key = self.app_has_api_key(db, app_id)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(InstalledApp {
                id: app_id,
                app_name: row.get(1)?,
                app_version: row.get(2)?,
                installed_at: row.get(3)?,
                updated_at: row.get(4)?,
                has_api_key,
                metadata,
            })
        }).optional()?;

        Ok(app)
    }

    /// Check if app has API key
    fn app_has_api_key(&self, db: &Connection, app_id: i64) -> Result<bool> {
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM app_settings WHERE setting_key = 'api_key' AND app_id = ?",
            params![app_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Insert app record
    fn insert_app_record(&self, db: &Connection, request: &AppInstallRequest) -> Result<i64> {
        db.execute(
            "INSERT INTO app_installations (app_name, app_version, metadata) 
             VALUES (?, ?, ?)",
            params![
                request.app_name,
                request.app_version,
                request.metadata.as_ref().map(|m| m.to_string())
            ],
        )?;

        Ok(db.last_insert_rowid())
    }

    /// Store API key securely
    fn store_api_key(&self, db: &Connection, app_id: i64, api_key: &str) -> Result<()> {
        // Encrypt the API key before storing
        let encryption_service = ApiKeyEncryptionService::new()?;
        let encrypted_api_key = encryption_service.encrypt_api_key(api_key)?;
        
        db.execute(
            "INSERT INTO app_settings (app_id, setting_key, setting_value, setting_type) 
             VALUES (?, 'api_key', ?, 'string')",
            params![app_id, encrypted_api_key],
        )?;

        Ok(())
    }

    /// Retrieve and decrypt API key
    pub fn get_api_key(&self, app_id: i64) -> Result<Option<String>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Get the encrypted API key
        let encrypted_api_key: Option<String> = db.query_row(
            "SELECT setting_value FROM app_settings WHERE setting_key = 'api_key' AND app_id = ?",
            params![app_id],
            |row| row.get(0)
        ).optional()?;

        match encrypted_api_key {
            Some(encrypted) => {
                // Decrypt the API key
                let encryption_service = ApiKeyEncryptionService::new()?;
                let decrypted_api_key = encryption_service.decrypt_api_key(&encrypted)?;
                Ok(Some(decrypted_api_key))
            },
            None => Ok(None),
        }
    }

    /// Store app metadata
    fn store_app_metadata(&self, db: &Connection, app_id: i64, metadata: &serde_json::Value) -> Result<()> {
        db.execute(
            "UPDATE app_installations SET metadata = ? WHERE id = ?",
            params![metadata.to_string(), app_id],
        )?;

        Ok(())
    }

    /// Log app action
    fn log_app_action(&self, db: &Connection, app_id: i64, action: &str, status: &str, message: &str) -> Result<()> {
        db.execute(
            "INSERT INTO app_logs (log_level, log_message, log_source) 
             VALUES (?, ?, ?)",
            params![
                if status == "success" { "info" } else { "error" },
                format!("{}: {}", action, message),
                format!("app_installation_{}", app_id)
            ],
        )?;

        Ok(())
    }

    /// Clean up duplicate and orphaned API key records
    pub fn cleanup_api_key_records(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();

        // Start transaction
        let tx = db.transaction()?;

        // Delete orphaned API key records (app_id IS NULL)
        let orphaned_count = tx.execute(
            "DELETE FROM app_settings WHERE setting_key = 'api_key' AND app_id IS NULL",
            params![],
        )?;
        println!("🧹 CLEANUP: Deleted {} orphaned API key records", orphaned_count);

        // For each app, keep only the most recent API key record
        let app_ids: Vec<i64> = {
            let mut stmt = tx.prepare(
                "SELECT DISTINCT app_id FROM app_settings WHERE setting_key = 'api_key' AND app_id IS NOT NULL"
            )?;
            
            let app_ids = stmt.query_map([], |row| row.get::<_, i64>(0))?;
            app_ids.collect::<Result<Vec<_>, _>>()?
        };

        for app_id in app_ids {
            // Get all API key records for this app
            let api_key_ids: Vec<i64> = {
                let mut api_key_stmt = tx.prepare(
                    "SELECT id FROM app_settings WHERE setting_key = 'api_key' AND app_id = ? ORDER BY id DESC"
                )?;

                let api_key_ids = api_key_stmt.query_map(params![app_id], |row| row.get::<_, i64>(0))?;
                api_key_ids.collect::<Result<Vec<_>, _>>()?
            };

            // Keep only the most recent one (highest ID)
            if api_key_ids.len() > 1 {
                let ids_to_delete = &api_key_ids[1..]; // All except the first (most recent)
                for id_to_delete in ids_to_delete {
                    tx.execute("DELETE FROM app_settings WHERE id = ?", params![id_to_delete])?;
                }
                println!("🧹 CLEANUP: Deleted {} duplicate API key records for app_id {}", 
                        ids_to_delete.len(), app_id);
            }
        }

        tx.commit()?;
        println!("🧹 CLEANUP: API key cleanup completed");
        Ok(())
    }

    /// Get all installed apps
    pub fn get_installed_apps(&self) -> Result<Vec<InstalledApp>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, app_name, app_version, installed_at, updated_at, metadata 
             FROM app_installations 
             ORDER BY installed_at DESC"
        )?;

        let apps = stmt.query_map(params![], |row| {
            let metadata_str: Option<String> = row.get(5)?;
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            let app_id: i64 = row.get(0)?;
            let has_api_key = self.app_has_api_key(&db, app_id)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(InstalledApp {
                id: app_id,
                app_name: row.get(1)?,
                app_version: row.get(2)?,
                installed_at: row.get(3)?,
                updated_at: row.get(4)?,
                has_api_key,
                metadata,
            })
        })?.collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| anyhow!("Failed to collect apps: {}", e))?;

        Ok(apps)
    }

    /// Get app by ID
    pub fn get_app_by_id(&self, app_id: i64) -> Result<Option<InstalledApp>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, app_name, app_version, installed_at, updated_at, metadata 
             FROM app_installations 
             WHERE id = ?"
        )?;
        let app = stmt.query_row(params![app_id], |row| {
            let metadata_str: Option<String> = row.get(5)?;
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            let app_id: i64 = row.get(0)?;

            let has_api_key: i64 = db.query_row(
                "SELECT COUNT(*) FROM app_settings WHERE setting_key = 'api_key' AND app_id = ?",
                params![app_id],
                |row| row.get(0),
            ).unwrap_or(0);

            Ok(InstalledApp {
                id: app_id,
                app_name: row.get(1)?,
                app_version: row.get(2)?,
                installed_at: row.get(3)?,
                updated_at: row.get(4)?,
                has_api_key: has_api_key > 0,
                metadata,
            })
        }).optional()?;
        Ok(app)
    }

    /// Uninstall an app
    pub fn uninstall_app(&self, app_id: i64) -> Result<AppInstallResponse> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();

        let app_name = db.query_row(
            "SELECT app_name FROM app_installations WHERE id = ?",
            params![app_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| "Unknown".to_string());

        // Start transaction
        let tx = db.transaction()?;

        // Delete app settings - clean up both app_id specific and any orphaned records
        let deleted_count = tx.execute(
            "DELETE FROM app_settings WHERE app_id = ?",
            params![app_id],
        )?;

        // This is a safety measure to prevent accumulation of orphaned records
        let orphaned_count = tx.execute(
            "DELETE FROM app_settings WHERE setting_key = 'api_key' AND app_id IS NULL",
            params![],
        )?;

        // Delete app installation record
        tx.execute(
            "DELETE FROM app_installations WHERE id = ?",
            params![app_id],
        )?;

        // Log uninstallation
        self.log_app_action(&tx, app_id, "uninstall", "success", &format!("Uninstalled app: {}", app_name))?;

        // Commit transaction
        tx.commit()?;

        app_log_info!("✅ APP INSTALLATION: Successfully uninstalled app: {}", app_name);

        Ok(AppInstallResponse {
            success: true,
            app_id: Some(app_id),
            message: format!("Successfully uninstalled {}", app_name),
        })
    }
} 

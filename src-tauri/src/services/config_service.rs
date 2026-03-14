use crate::utils::path_utils;
use crate::{app_log_debug, app_log_info, app_log_warn};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User configuration data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Custom database path (if set by user)
    pub custom_db_path: Option<String>,

    /// Config file metadata
    pub metadata: ConfigMetadata,
}

/// Config file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Default for UserConfig {
    fn default() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            custom_db_path: None,
            metadata: ConfigMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: now.clone(),
                updated_at: now,
            },
        }
    }
}

/// Configuration service to replace user.db operations
pub struct ConfigService {
    config_path: PathBuf,
    config: UserConfig,
}

impl ConfigService {
    /// Create a new config service
    pub fn new() -> Result<Self> {
        let app_data_dir = path_utils::get_app_data_dir()?;
        let config_path = app_data_dir.join(".config.json");

        let config = Self::load_or_create_config(&config_path)?;

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Load existing config or create default
    fn load_or_create_config(config_path: &PathBuf) -> Result<UserConfig> {
        if config_path.exists() {
            app_log_debug!(
                "📋 CONFIG: Loading existing config from: {}",
                config_path.display()
            );

            match fs::read_to_string(config_path) {
                Ok(content) => {
                    match serde_json::from_str::<UserConfig>(&content) {
                        Ok(mut config) => {
                            // Update metadata
                            config.metadata.updated_at = chrono::Utc::now().to_rfc3339();
                            app_log_info!("✅ CONFIG: Successfully loaded existing config");
                            return Ok(config);
                        }
                        Err(e) => {
                            app_log_warn!(
                                "⚠️ CONFIG: Failed to parse config file, creating new one: {}",
                                e
                            );
                            // Backup corrupted config
                            let backup_path = config_path.with_extension("json.backup");
                            let _ = fs::copy(config_path, backup_path);
                        }
                    }
                }
                Err(e) => {
                    app_log_warn!("⚠️ CONFIG: Failed to read config file: {}", e);
                }
            }
        }

        // Create new config
        app_log_info!(
            "🆕 CONFIG: Creating new config file at: {}",
            config_path.display()
        );
        let config = UserConfig::default();

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Save new config
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(config_path, content)?;

        app_log_info!("✅ CONFIG: Created new config file successfully");
        Ok(config)
    }

    /// Save current config to disk
    pub fn save(&self) -> Result<()> {
        app_log_debug!(
            "💾 CONFIG: Saving config to: {}",
            self.config_path.display()
        );

        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, content)?;

        app_log_debug!("✅ CONFIG: Config saved successfully");
        Ok(())
    }

    /// Get the database path (custom or default)
    pub fn get_db_path(&self) -> Result<PathBuf> {
        match &self.config.custom_db_path {
            Some(custom_path) => {
                let path = PathBuf::from(custom_path);
                app_log_debug!("📁 CONFIG: Using custom DB path: {}", path.display());
                Ok(path)
            }
            None => {
                let default_path =
                    path_utils::get_app_data_dir()?.join(crate::constants::DATABASE_FILENAME);
                app_log_debug!(
                    "📁 CONFIG: Using default DB path: {}",
                    default_path.display()
                );
                Ok(default_path)
            }
        }
    }

    /// Get custom database path
    pub fn get_custom_db_path(&self) -> Option<String> {
        self.config.custom_db_path.clone()
    }

    /// Set custom database path
    pub fn set_custom_db_path(&mut self, path: Option<String>) -> Result<()> {
        self.config.custom_db_path = path;
        self.config.metadata.updated_at = chrono::Utc::now().to_rfc3339();
        self.save()?;

        if let Some(ref path) = self.config.custom_db_path {
            app_log_info!("📁 CONFIG: Set custom DB path: {}", path);
        } else {
            app_log_info!("📁 CONFIG: Cleared custom DB path, using default");
        }

        Ok(())
    }

    /// Remove old user.db file if it exists
    pub fn cleanup_old_user_db() -> Result<()> {
        let app_data_dir = path_utils::get_app_data_dir()?;
        let user_db_path = app_data_dir.join(".user.db");

        if user_db_path.exists() {
            app_log_info!("🗑️ CONFIG: Removing old user.db file");
            if let Err(e) = fs::remove_file(&user_db_path) {
                app_log_warn!("⚠️ CONFIG: Failed to remove user.db: {}", e);
            } else {
                app_log_info!("✅ CONFIG: Successfully removed old user.db file");
            }
        }

        Ok(())
    }

    /// Remove old vector_search.db file if it exists
    pub fn cleanup_old_vector_db() -> Result<()> {
        let app_data_dir = path_utils::get_app_data_dir()?;
        let old_db_path = app_data_dir.join("vector_search.db");

        if old_db_path.exists() {
            app_log_info!("🗑️ CONFIG: Removing old vector_search.db file");
            if let Err(e) = fs::remove_file(&old_db_path) {
                app_log_warn!("⚠️ CONFIG: Failed to remove vector_search.db: {}", e);
            } else {
                app_log_info!("✅ CONFIG: Successfully removed old vector_search.db file");
            }
        }

        Ok(())
    }
}

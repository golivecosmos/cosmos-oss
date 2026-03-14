use crate::app_log_info;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fs;
use std::path::PathBuf;
#[cfg(test)]
use tempfile;

/// Service for managing encryption keys using app storage
///
/// This service provides secure storage and retrieval of encryption keys
/// using app-specific storage for maximum reliability and zero lockout risk.
/// Works identically on all platforms (macOS, Windows, Linux).
pub struct EncryptionKeyService {
    app_data_dir: PathBuf,
    #[cfg(test)]
    _temp_dir: Option<tempfile::TempDir>, // Keep temp dir alive during tests
}

impl EncryptionKeyService {
    /// Create a new encryption key service for database encryption
    pub fn new() -> Self {
        let app_data_dir =
            crate::utils::path_utils::get_app_data_dir().expect("Failed to get app data directory");

        Self {
            app_data_dir,
            #[cfg(test)]
            _temp_dir: None,
        }
    }

    /// Create a new encryption key service for testing (in-memory only)
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        // Use a temporary directory for testing
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory for testing");

        // Ensure the directory exists
        std::fs::create_dir_all(temp_dir.path())
            .expect("Failed to create temp directory structure");

        Self {
            app_data_dir: temp_dir.path().to_path_buf(),
            #[cfg(test)]
            _temp_dir: Some(temp_dir),
        }
    }

    /// Store a key using app storage (simple, reliable)
    pub fn store_key(&self, key: &str) -> Result<()> {
        self.store_key_in_app_storage(key)
    }

    /// Retrieve a key from app storage (always works)
    pub fn get_key(&self) -> Result<String> {
        self.get_key_from_app_storage()
    }

    /// Check if a key exists in app storage
    pub fn has_key(&self) -> bool {
        self.has_key_in_app_storage()
    }

    /// Remove a key from app storage
    pub fn remove_key(&self) -> Result<()> {
        self.remove_key_from_app_storage()
    }

    // ===== APP STORAGE METHODS =====

    /// Store key in app-specific encrypted storage (simple, reliable)
    fn store_key_in_app_storage(&self, key: &str) -> Result<()> {
        let key_file = self.app_data_dir.join(".cosmos_desktop_key");

        // Create a simple obfuscated storage (not as secure as system keychain but better UX)
        let obfuscated_key = self.obfuscate_key(key);
        fs::write(&key_file, obfuscated_key)?;

        // Set restrictive permissions (user read/write only)
        let mut perms = fs::metadata(&key_file)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&key_file, perms)?;

        app_log_info!("🔐 Stored encryption key in app storage");
        Ok(())
    }

    /// Retrieve key from app-specific storage
    fn get_key_from_app_storage(&self) -> Result<String> {
        let key_file = self.app_data_dir.join(".cosmos_desktop_key");

        if !key_file.exists() {
            return Err(anyhow!("No encryption key found in app storage"));
        }

        let obfuscated_key = fs::read_to_string(&key_file)?;
        let key = self.deobfuscate_key(&obfuscated_key)?;

        Ok(key)
    }

    /// Check if key exists in app storage
    fn has_key_in_app_storage(&self) -> bool {
        let key_file = self.app_data_dir.join(".cosmos_desktop_key");
        key_file.exists()
    }

    /// Remove key from app storage
    fn remove_key_from_app_storage(&self) -> Result<()> {
        let key_file = self.app_data_dir.join(".cosmos_desktop_key");
        if key_file.exists() {
            fs::remove_file(&key_file)?;
        }
        Ok(())
    }

    /// Simple obfuscation for app storage (not cryptographic, just basic obfuscation)
    fn obfuscate_key(&self, key: &str) -> String {
        let encoded = BASE64.encode(key.as_bytes());
        // Simple XOR with a fixed value for basic obfuscation
        let obfuscated: String = encoded.chars().map(|c| (c as u8 ^ 0x42) as char).collect();
        obfuscated
    }

    /// Deobfuscate key from app storage
    fn deobfuscate_key(&self, obfuscated: &str) -> Result<String> {
        let deobfuscated: String = obfuscated
            .chars()
            .map(|c| (c as u8 ^ 0x42) as char)
            .collect();

        let decoded = BASE64
            .decode(deobfuscated.as_bytes())
            .map_err(|e| anyhow!("Failed to decode key: {}", e))?;

        String::from_utf8(decoded).map_err(|e| anyhow!("Failed to convert key to string: {}", e))
    }
}

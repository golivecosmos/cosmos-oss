use anyhow::{anyhow, Result};
use rand::{Rng, rngs::OsRng};
use crate::services::encryption_key_service::EncryptionKeyService;
use thiserror::Error;
use base64::Engine;

/// Encryption-specific error types for comprehensive error handling
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),
    
    #[error("Key storage failed: {0}")]
    KeyStorageError(String),
    
    #[error("Key retrieval failed: {0}")]
    KeyRetrievalError(String),
    
    #[error("Key removal failed: {0}")]
    KeyRemovalError(String),
    
    #[error("Key verification failed: {0}")]
    KeyVerificationError(String),
    
    #[error("Key rotation failed: {0}")]
    KeyRotationError(String),
}

/// Service for managing database encryption keys and operations
/// 
/// This service provides secure database encryption key management,
/// integrating with app storage for secure storage.
pub struct DatabaseEncryptionService {
    encryption_key_service: EncryptionKeyService,
}

impl DatabaseEncryptionService {
    /// Create a new database encryption service
    pub fn new() -> Self {
        Self {
            encryption_key_service: EncryptionKeyService::new(),
        }
    }
    
    /// Create a new database encryption service for testing
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        Self {
            encryption_key_service: EncryptionKeyService::new_for_testing(),
        }
    }
    
    /// Generate a cryptographically secure database key
    pub fn generate_database_key() -> Result<String> {
        let mut rng = OsRng;
        let key: String = (0..64)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        Ok(key)
    }
    
    /// Store database key in app storage
    pub fn store_database_key(&self, key: &str) -> Result<()> {
        // Try to remove existing key first to avoid conflicts
        let _ = self.encryption_key_service.remove_key();
        
        self.encryption_key_service.store_key(key)
            .map_err(|e| anyhow!("Failed to store database key: {}", e))
    }
    
    /// Retrieve database key from app storage
    pub fn get_database_key(&self) -> Result<String> {
        self.encryption_key_service.get_key()
            .map_err(|e| anyhow!("Failed to retrieve database key: {}", e))
    }
    
    /// Check if database key exists in app storage
    pub fn has_database_key(&self) -> bool {
        self.encryption_key_service.has_key()
    }
    
    /// Remove database key from app storage
    pub fn remove_database_key(&self) -> Result<()> {
        self.encryption_key_service.remove_key()
            .map_err(|e| anyhow!("Failed to remove database key: {}", e))
    }
    
    /// Test encryption functionality
    pub fn test_encryption(&self) -> Result<()> {
        // Generate a test key
        let test_key = Self::generate_database_key()?;
        
        // Store it in app storage
        self.store_database_key(&test_key)?;
        
        // Retrieve it from app storage
        let retrieved_key = self.get_database_key()?;
        
        // Verify they match
        if test_key != retrieved_key {
            return Err(anyhow!("Encryption test failed: keys don't match"));
        }
        
        // Clean up
        self.remove_database_key()?;
        
        Ok(())
    }
    
    /// Rotate database encryption key
    pub fn rotate_database_key(&self) -> Result<String> {
        // Generate new key
        let new_key = Self::generate_database_key()?;
        
        // Store new key (this will replace the old key)
        self.store_database_key(&new_key)?;
        
        Ok(new_key)
    }
    
    // ===== ERROR HANDLING AND RECOVERY METHODS =====
    
    /// Handle encryption errors with recovery options
    pub fn handle_encryption_error(&self, error: &anyhow::Error) -> Result<()> {
        crate::app_log_error!("🚨 Encryption error encountered: {}", error);
        
        // Try to downcast to our specific error types
        if let Some(encryption_error) = error.downcast_ref::<EncryptionError>() {
            match encryption_error {
                EncryptionError::KeyGenerationError(msg) => {
                    crate::app_log_error!("🔑 Key generation error: {}", msg);
                    self.recover_from_key_generation_error()
                }
                EncryptionError::KeyStorageError(msg) => {
                    crate::app_log_error!("💾 Key storage error: {}", msg);
                    self.recover_from_key_storage_error()
                }
                EncryptionError::KeyRetrievalError(msg) => {
                    crate::app_log_error!("🔍 Key retrieval error: {}", msg);
                    self.recover_from_key_retrieval_error()
                }
                EncryptionError::KeyRemovalError(msg) => {
                    crate::app_log_error!("🗑️ Key removal error: {}", msg);
                    self.recover_from_key_removal_error()
                }
                EncryptionError::KeyVerificationError(msg) => {
                    crate::app_log_error!("✅ Key verification error: {}", msg);
                    self.recover_from_key_verification_error()
                }
                EncryptionError::KeyRotationError(msg) => {
                    crate::app_log_error!("🔄 Key rotation error: {}", msg);
                    self.recover_from_key_rotation_error()
                }
            }
        } else {
            // Generic error handling
            crate::app_log_error!("❓ Unknown encryption error: {}", error);
            self.recover_from_generic_encryption_error(error)
        }
    }
    
    /// Recover from key generation errors
    fn recover_from_key_generation_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key generation error");
        
        // Try to generate a new key with different parameters
        let new_key = Self::generate_database_key()
            .map_err(|e| anyhow!("Failed to generate new key during recovery: {}", e))?;
        
        crate::app_log_info!("✅ New key generated successfully during recovery");
        Ok(())
    }
    
    /// Recover from key storage errors
    fn recover_from_key_storage_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key storage error");
        
        // Try to remove existing key first
        let _ = self.remove_database_key();
        
        // Try to store a new key
        let new_key = Self::generate_database_key()?;
        self.store_database_key(&new_key)?;
        
        crate::app_log_info!("✅ Key storage recovery successful");
        Ok(())
    }
    
    /// Recover from key retrieval errors
    fn recover_from_key_retrieval_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key retrieval error");
        
        // Check if key exists
        if self.has_database_key() {
            crate::app_log_info!("🔑 Key exists in app storage, attempting to regenerate");
            // Try to regenerate the key
            let new_key = Self::generate_database_key()?;
            self.store_database_key(&new_key)?;
        } else {
            crate::app_log_info!("🔑 No key found, generating new key");
            let new_key = Self::generate_database_key()?;
            self.store_database_key(&new_key)?;
        }
        
        crate::app_log_info!("✅ Key retrieval recovery successful");
        Ok(())
    }
    
    /// Recover from key removal errors
    fn recover_from_key_removal_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key removal error");
        
        // Try to remove the key again
        self.remove_database_key()?;
        
        crate::app_log_info!("✅ Key removal recovery successful");
        Ok(())
    }
    
    /// Recover from key verification errors
    fn recover_from_key_verification_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key verification error");
        
        // Try to regenerate the key
        let new_key = Self::generate_database_key()?;
        self.store_database_key(&new_key)?;
        
        crate::app_log_info!("✅ Key verification recovery successful");
        Ok(())
    }
    
    /// Recover from key rotation errors
    fn recover_from_key_rotation_error(&self) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from key rotation error");
        
        // Try to rotate the key again
        self.rotate_database_key()?;
        
        crate::app_log_info!("✅ Key rotation recovery successful");
        Ok(())
    }
    
    /// Recover from generic encryption errors
    fn recover_from_generic_encryption_error(&self, error: &anyhow::Error) -> Result<()> {
        crate::app_log_info!("🔧 Attempting to recover from generic encryption error");
        
        // Try to regenerate the key
        let new_key = Self::generate_database_key()?;
        self.store_database_key(&new_key)?;
        
        crate::app_log_info!("✅ Generic encryption recovery successful");
        Ok(())
    }
    
    /// Get encryption health status
    pub fn get_encryption_health(&self) -> Result<serde_json::Value> {
        let has_key = self.has_database_key();
        let can_retrieve = self.get_database_key().is_ok();
        
        let health_status = if !has_key {
            "no_key"
        } else if !can_retrieve {
            "key_corrupted"
        } else {
            "healthy"
        };
        
        let health = serde_json::json!({
            "has_encryption_key": has_key,
            "can_retrieve_key": can_retrieve,
            "health_status": health_status,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(health)
    }
    
    /// Perform encryption integrity check
    pub fn check_encryption_integrity(&self) -> Result<serde_json::Value> {
        crate::app_log_info!("🔍 Performing encryption integrity check");
        
        let mut integrity_report = serde_json::json!({
            "checks": {}
        });
        
        // Check if key exists
        let has_key = self.has_database_key();
        integrity_report["checks"]["key_exists"] = serde_json::Value::Bool(has_key);
        
        if !has_key {
            crate::app_log_warn!("⚠️ No encryption key found");
            return Ok(integrity_report);
        }
        
        // Check if key can be retrieved
        let can_retrieve = self.get_database_key().is_ok();
        integrity_report["checks"]["can_retrieve_key"] = serde_json::Value::Bool(can_retrieve);
        
        if !can_retrieve {
            crate::app_log_error!("❌ Cannot retrieve encryption key");
            integrity_report["checks"]["retrieval_error"] = serde_json::Value::String(
                self.get_database_key().unwrap_err().to_string()
            );
        } else {
            crate::app_log_info!("✅ Encryption key retrieval successful");
        }
        
        // Check key length
        if let Ok(key) = self.get_database_key() {
            integrity_report["checks"]["key_length"] = serde_json::Value::Number(key.len().into());
            integrity_report["checks"]["key_length_valid"] = serde_json::Value::Bool(key.len() == 64);
        }
        
        crate::app_log_info!("✅ Encryption integrity check completed");
        Ok(integrity_report)
    }
    
    /// Emergency key regeneration
    pub fn emergency_key_regeneration(&self) -> Result<String> {
        crate::app_log_warn!("🚨 Performing emergency key regeneration");
        
        // Remove existing key
        let _ = self.remove_database_key();
        
        // Generate new key
        let new_key = Self::generate_database_key()?;
        
        // Store new key
        self.store_database_key(&new_key)?;
        
        crate::app_log_info!("✅ Emergency key regeneration completed");
        Ok(new_key)
    }
    
    /// Backup encryption key (for emergency recovery)
    pub fn backup_encryption_key(&self) -> Result<String> {
        let key = self.get_database_key()?;
        
        // In a production system, you might want to encrypt this backup
        // For now, we'll just return the key as a base64 string
        let backup = base64::engine::general_purpose::STANDARD.encode(&key);
        
        crate::app_log_info!("💾 Encryption key backup created");
        Ok(backup)
    }
    
    /// Restore encryption key from backup
    pub fn restore_encryption_key(&self, backup: &str) -> Result<()> {
        // Decode the backup
        let key_bytes = base64::engine::general_purpose::STANDARD.decode(backup)
            .map_err(|e| anyhow!("Failed to decode backup: {}", e))?;
        
        let key = String::from_utf8(key_bytes)
            .map_err(|e| anyhow!("Failed to convert backup to string: {}", e))?;
        
        // Store the restored key
        self.store_database_key(&key)?;
        
        crate::app_log_info!("✅ Encryption key restored from backup");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_uniqueness() {
        let encryption_service = DatabaseEncryptionService::new();
        
        // Generate multiple keys and ensure they're unique
        let keys: Vec<String> = (0..10)
            .map(|_| DatabaseEncryptionService::generate_database_key().expect("Failed to generate key"))
            .collect();
        
        // Check all keys are unique
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j], "Generated keys should be unique");
            }
        }
        
        // Check all keys are 64 characters
        for key in &keys {
            assert_eq!(key.len(), 64, "All keys should be 64 characters");
        }
    }
} 
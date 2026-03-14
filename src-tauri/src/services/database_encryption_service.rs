use crate::services::encryption_key_service::EncryptionKeyService;
use anyhow::{anyhow, Result};
use rand::{rngs::OsRng, Rng};

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

        self.encryption_key_service
            .store_key(key)
            .map_err(|e| anyhow!("Failed to store database key: {}", e))
    }

    /// Retrieve database key from app storage
    pub fn get_database_key(&self) -> Result<String> {
        self.encryption_key_service
            .get_key()
            .map_err(|e| anyhow!("Failed to retrieve database key: {}", e))
    }

    /// Check if database key exists in app storage
    pub fn has_database_key(&self) -> bool {
        self.encryption_key_service.has_key()
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
            .map(|_| {
                DatabaseEncryptionService::generate_database_key().expect("Failed to generate key")
            })
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

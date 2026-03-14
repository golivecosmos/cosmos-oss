use crate::services::encryption_key_service::EncryptionKeyService;
use crate::{app_log_error, app_log_info};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::{rngs::OsRng, Rng};
use sha2::Digest;

/// Service for encrypting and decrypting API keys using AES-GCM
///
/// This service provides secure encryption and decryption of API keys
/// using AES-256-GCM for authenticated encryption.
pub struct ApiKeyEncryptionService {
    key: Aes256Gcm,
}

impl ApiKeyEncryptionService {
    /// Create a new API key encryption service
    pub fn new() -> Result<Self> {
        let encryption_key_service = EncryptionKeyService::new();

        // Get or generate the API key encryption key
        let key_bytes = Self::get_or_generate_api_key_encryption_key(&encryption_key_service)?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { key: cipher })
    }

    /// Create a new API key encryption service for testing
    #[cfg(test)]
    pub fn new_for_testing() -> Result<Self> {
        let encryption_key_service = EncryptionKeyService::new_for_testing();

        // Generate a new key for testing
        let key_bytes = Self::generate_api_key_encryption_key()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { key: cipher })
    }

    /// Encrypt an API key
    pub fn encrypt_api_key(&self, api_key: &str) -> Result<String> {
        // Generate a random nonce
        let mut rng = OsRng;
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the API key
        let ciphertext = self
            .key
            .encrypt(nonce, api_key.as_bytes())
            .map_err(|e| anyhow!("Failed to encrypt API key: {}", e))?;

        // Combine nonce and ciphertext
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        // Encode as base64
        let encoded = BASE64.encode(&combined);

        app_log_info!("🔐 API key encrypted successfully");
        Ok(encoded)
    }

    /// Decrypt an API key
    pub fn decrypt_api_key(&self, encrypted_api_key: &str) -> Result<String> {
        // Decode from base64
        let combined = BASE64
            .decode(encrypted_api_key)
            .map_err(|e| anyhow!("Failed to decode encrypted API key: {}", e))?;

        if combined.len() < 12 {
            return Err(anyhow!("Invalid encrypted API key format"));
        }

        // Extract nonce and ciphertext
        let nonce_bytes = &combined[..12];
        let ciphertext = &combined[12..];

        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the API key
        let plaintext = self
            .key
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Failed to decrypt API key: {}", e))?;

        let api_key = String::from_utf8(plaintext)
            .map_err(|e| anyhow!("Failed to convert decrypted API key to string: {}", e))?;

        app_log_info!("🔐 API key decrypted successfully");
        Ok(api_key)
    }

    /// Get or generate the API key encryption key
    fn get_or_generate_api_key_encryption_key(
        encryption_key_service: &EncryptionKeyService,
    ) -> Result<[u8; 32]> {
        // Try to get existing key
        if encryption_key_service.has_key() {
            match encryption_key_service.get_key() {
                Ok(key_str) => {
                    // The key service returns the key as a string, we need to derive 32 bytes from it
                    // Use SHA-256 to derive exactly 32 bytes from the stored key
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(key_str.as_bytes());
                    let key_bytes = hasher.finalize();

                    let mut key_array = [0u8; 32];
                    key_array.copy_from_slice(&key_bytes);

                    app_log_info!("🔐 Retrieved and derived encryption key from app storage");
                    return Ok(key_array);
                }
                Err(e) => {
                    app_log_error!("Failed to retrieve stored encryption key: {}", e);
                    // Fall through to generate new key
                }
            }
        }

        // Generate new key
        let new_key = Self::generate_api_key_encryption_key()?;

        // Store the new key as a string (the encryption service will handle the derivation)
        let key_str = BASE64.encode(&new_key);
        encryption_key_service
            .store_key(&key_str)
            .map_err(|e| anyhow!("Failed to store new encryption key: {}", e))?;

        app_log_info!("🔐 Generated and stored new API key encryption key");
        Ok(new_key)
    }

    /// Generate a cryptographically secure encryption key
    fn generate_api_key_encryption_key() -> Result<[u8; 32]> {
        let mut rng = OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes);
        Ok(key_bytes)
    }

    /// Test encryption and decryption functionality
    #[cfg(test)]
    pub fn test_encryption(&self) -> Result<()> {
        let test_api_key = "test_api_key_12345";

        // Encrypt
        let encrypted = self.encrypt_api_key(test_api_key)?;

        // Decrypt
        let decrypted = self.decrypt_api_key(&encrypted)?;

        // Verify
        if test_api_key != decrypted {
            return Err(anyhow!(
                "Encryption test failed: decrypted value doesn't match original"
            ));
        }

        app_log_info!("✅ API key encryption test passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_encryption() {
        let service = ApiKeyEncryptionService::new_for_testing().unwrap();

        let test_api_key = "sk-test-api-key-123456789";

        // Test encryption and decryption
        let encrypted = service.encrypt_api_key(test_api_key).unwrap();
        let decrypted = service.decrypt_api_key(&encrypted).unwrap();

        assert_eq!(test_api_key, decrypted);
    }

    #[test]
    fn test_encryption_test_method() {
        let service = ApiKeyEncryptionService::new_for_testing().unwrap();
        service.test_encryption().unwrap();
    }

    #[test]
    fn test_different_keys_produce_different_encryptions() {
        let service1 = ApiKeyEncryptionService::new_for_testing().unwrap();
        let service2 = ApiKeyEncryptionService::new_for_testing().unwrap();

        let test_api_key = "sk-test-api-key-123456789";

        let encrypted1 = service1.encrypt_api_key(test_api_key).unwrap();
        let encrypted2 = service2.encrypt_api_key(test_api_key).unwrap();

        // Should be different due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        let decrypted1 = service1.decrypt_api_key(&encrypted1).unwrap();
        let decrypted2 = service2.decrypt_api_key(&encrypted2).unwrap();

        assert_eq!(decrypted1, decrypted2);
        assert_eq!(decrypted1, test_api_key);
    }
}

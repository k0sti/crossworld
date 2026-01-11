//! Key management for Nostr accounts
//!
//! Handles generation, import, export, and storage of Nostr keypairs.

use crate::{Error, NostrAccount, Result};
use nostr::prelude::*;
use std::path::PathBuf;

/// Key storage filename
const KEY_FILE_NAME: &str = "nostr-keys.json";

/// Manages Nostr keypair operations
#[derive(Debug, Clone)]
pub struct KeyManager {
    /// Directory for storing keys
    config_dir: PathBuf,
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyManager {
    /// Create a new KeyManager using the default config directory
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("crossworld"))
            .unwrap_or_else(|| PathBuf::from(".crossworld"));

        Self { config_dir }
    }

    /// Create a KeyManager with a custom config directory
    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Get the path to the key file
    fn key_file_path(&self) -> PathBuf {
        self.config_dir.join(KEY_FILE_NAME)
    }

    /// Generate a new random keypair and create an account
    pub fn generate_account(&self) -> Result<NostrAccount> {
        let keys = Keys::generate();
        Ok(NostrAccount::from_keys(keys))
    }

    /// Import an account from an nsec string
    ///
    /// # Arguments
    /// * `nsec` - The nsec1... bech32 encoded private key
    pub fn import_from_nsec(&self, nsec: &str) -> Result<NostrAccount> {
        let keys = Keys::parse(nsec).map_err(|e| Error::InvalidKey(e.to_string()))?;
        Ok(NostrAccount::from_keys(keys))
    }

    /// Import an account from a hex-encoded private key
    ///
    /// # Arguments
    /// * `hex_key` - The 64-character hex-encoded private key
    pub fn import_from_hex(&self, hex_key: &str) -> Result<NostrAccount> {
        let secret_key =
            SecretKey::from_hex(hex_key).map_err(|e| Error::InvalidKey(e.to_string()))?;
        let keys = Keys::new(secret_key);
        Ok(NostrAccount::from_keys(keys))
    }

    /// Save an account's private key to the config directory
    ///
    /// # Security Warning
    /// This stores the private key unencrypted on disk. Only use for development
    /// or when the user explicitly requests persistence.
    pub fn save_account(&self, account: &NostrAccount) -> Result<()> {
        // Ensure config directory exists
        std::fs::create_dir_all(&self.config_dir)?;

        let key_data = KeyFileData {
            nsec: account.nsec(),
            npub: account.npub(),
            hex_pubkey: account.public_key_hex(),
        };

        let json = serde_json::to_string_pretty(&key_data)?;
        std::fs::write(self.key_file_path(), json)?;

        Ok(())
    }

    /// Load a saved account from the config directory
    pub fn load_account(&self) -> Result<Option<NostrAccount>> {
        let path = self.key_file_path();
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)?;
        let key_data: KeyFileData = serde_json::from_str(&json)?;

        let account = self.import_from_nsec(&key_data.nsec)?;
        Ok(Some(account))
    }

    /// Delete the saved key file
    pub fn delete_saved_keys(&self) -> Result<()> {
        let path = self.key_file_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Check if there are saved keys
    pub fn has_saved_keys(&self) -> bool {
        self.key_file_path().exists()
    }
}

/// Data structure for storing keys to disk
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct KeyFileData {
    nsec: String,
    npub: String,
    hex_pubkey: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn temp_key_manager() -> KeyManager {
        let dir = temp_dir().join(format!("crossworld-test-{}", std::process::id()));
        KeyManager::with_config_dir(dir)
    }

    #[test]
    fn test_generate_account() {
        let km = temp_key_manager();
        let account = km.generate_account().unwrap();

        // Should have valid keys
        assert!(account.npub().starts_with("npub1"));
        assert!(account.nsec().starts_with("nsec1"));
        assert_eq!(account.public_key_hex().len(), 64);
    }

    #[test]
    fn test_import_nsec() {
        let km = temp_key_manager();

        // Generate a key first
        let original = km.generate_account().unwrap();
        let nsec = original.nsec();

        // Import it
        let imported = km.import_from_nsec(&nsec).unwrap();

        // Should have the same public key
        assert_eq!(original.npub(), imported.npub());
        assert_eq!(original.public_key_hex(), imported.public_key_hex());
    }

    #[test]
    fn test_invalid_nsec() {
        let km = temp_key_manager();
        let result = km.import_from_nsec("invalid-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load() {
        let km = temp_key_manager();

        // Generate and save
        let original = km.generate_account().unwrap();
        km.save_account(&original).unwrap();

        // Load it back
        let loaded = km.load_account().unwrap().unwrap();

        // Should be the same
        assert_eq!(original.npub(), loaded.npub());
        assert_eq!(original.nsec(), loaded.nsec());

        // Cleanup
        km.delete_saved_keys().unwrap();
    }

    #[test]
    fn test_load_nonexistent() {
        let km = temp_key_manager();
        let result = km.load_account().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_has_saved_keys() {
        let km = temp_key_manager();

        // Initially no keys
        assert!(!km.has_saved_keys());

        // Save some
        let account = km.generate_account().unwrap();
        km.save_account(&account).unwrap();
        assert!(km.has_saved_keys());

        // Delete them
        km.delete_saved_keys().unwrap();
        assert!(!km.has_saved_keys());
    }
}

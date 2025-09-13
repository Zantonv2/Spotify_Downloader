use crate::errors::{AppError, Result};
use crate::utils::{encrypt_data, decrypt_data};
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use dirs;

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedData {
    data: String,
    salt: String,
}

pub struct SecureStorage {
    storage_path: PathBuf,
    master_key: [u8; 32],
}

impl SecureStorage {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| AppError::IoError("Could not find data directory".to_string()))?
            .join("spotify-downloader");
        
        fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::IoError(format!("Failed to create data directory: {}", e)))?;

        let storage_path = data_dir.join("secure_storage.json");
        
        // Generate or load master key
        let master_key = Self::get_or_create_master_key(&data_dir)?;

        Ok(Self {
            storage_path,
            master_key,
        })
    }

    fn get_or_create_master_key(data_dir: &PathBuf) -> Result<[u8; 32]> {
        let key_path = data_dir.join("master.key");
        
        if key_path.exists() {
            let key_data = fs::read(&key_path)
                .map_err(|e| AppError::IoError(format!("Failed to read master key: {}", e)))?;
            
            if key_data.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_data);
                Ok(key)
            } else {
                // Invalid key file, regenerate
                Self::generate_master_key(&key_path)
            }
        } else {
            Self::generate_master_key(&key_path)
        }
    }

    fn generate_master_key(key_path: &PathBuf) -> Result<[u8; 32]> {
        use rand::RngCore;
        
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        
        fs::write(key_path, &key)
            .map_err(|e| AppError::IoError(format!("Failed to write master key: {}", e)))?;
        
        Ok(key)
    }

    pub async fn store_api_key(&self, service: &str, api_key: &str) -> Result<()> {
        let encrypted_data = encrypt_data(api_key, &self.master_key)?;
        
        let mut storage = self.load_storage().await?;
        storage.insert(service.to_string(), encrypted_data);
        
        self.save_storage(&storage).await?;
        Ok(())
    }

    pub async fn get_api_key(&self, service: &str) -> Result<Option<String>> {
        let storage = self.load_storage().await?;
        
        if let Some(encrypted_data) = storage.get(service) {
            let decrypted = decrypt_data(encrypted_data, &self.master_key)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    pub async fn remove_api_key(&self, service: &str) -> Result<()> {
        let mut storage = self.load_storage().await?;
        storage.remove(service);
        self.save_storage(&storage).await?;
        Ok(())
    }

    pub async fn list_services(&self) -> Result<Vec<String>> {
        let storage = self.load_storage().await?;
        Ok(storage.keys().cloned().collect())
    }

    async fn load_storage(&self) -> Result<std::collections::HashMap<String, String>> {
        if self.storage_path.exists() {
            let data = fs::read_to_string(&self.storage_path)
                .map_err(|e| AppError::IoError(format!("Failed to read storage file: {}", e)))?;
            
            let storage: std::collections::HashMap<String, String> = serde_json::from_str(&data)
                .map_err(|e| AppError::ParseError(format!("Failed to parse storage file: {}", e)))?;
            
            Ok(storage)
        } else {
            Ok(std::collections::HashMap::new())
        }
    }

    async fn save_storage(&self, storage: &std::collections::HashMap<String, String>) -> Result<()> {
        let data = serde_json::to_string_pretty(storage)
            .map_err(|e| AppError::Json(e))?;
        
        fs::write(&self.storage_path, data)
            .map_err(|e| AppError::IoError(format!("Failed to write storage file: {}", e)))?;
        
        Ok(())
    }
}

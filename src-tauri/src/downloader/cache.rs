use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::errors::Result;

/// Cache entry for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: u64,
    pub expires_at: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl_seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            data,
            created_at: now,
            expires_at: now + ttl_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at
    }
}

/// Cache for metadata and search results
pub struct MetadataCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry<serde_json::Value>>>>,
    default_ttl: Duration,
}

impl MetadataCache {
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(default_ttl_seconds),
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: serde_json::Value) {
        let entry = CacheEntry::new(value, self.default_ttl.as_secs());
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    pub async fn set_with_ttl(&self, key: String, value: serde_json::Value, ttl_seconds: u64) {
        let entry = CacheEntry::new(value, ttl_seconds);
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    pub async fn remove(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    pub async fn clear_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    pub async fn clear_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

/// File-based cache for downloaded content
pub struct FileCache {
    cache_dir: PathBuf,
    max_size_bytes: u64,
    current_size: Arc<RwLock<u64>>,
}

impl FileCache {
    pub fn new(cache_dir: PathBuf, max_size_mb: u64) -> Result<Self> {
        let max_size_bytes = max_size_mb * 1024 * 1024;
        
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)?;
        
        Ok(Self {
            cache_dir,
            max_size_bytes,
            current_size: Arc::new(RwLock::new(0)),
        })
    }

    pub fn get_cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.cache", key))
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.get_cache_path(key);
        if path.exists() {
            match tokio::fs::read(&path).await {
                Ok(data) => Some(data),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub async fn set(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let path = self.get_cache_path(key);
        
        // Check if we need to make space
        self.ensure_space(data.len() as u64).await?;
        
        // Write the file
        tokio::fs::write(&path, &data).await?;
        
        // Update size tracking
        let mut current_size = self.current_size.write().await;
        *current_size += data.len() as u64;
        
        Ok(())
    }

    pub async fn remove(&self, key: &str) -> Result<()> {
        let path = self.get_cache_path(key);
        if path.exists() {
            let metadata = tokio::fs::metadata(&path).await?;
            tokio::fs::remove_file(&path).await?;
            
            // Update size tracking
            let mut current_size = self.current_size.write().await;
            *current_size = current_size.saturating_sub(metadata.len());
        }
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let entries = tokio::fs::read_dir(&self.cache_dir).await?;
        let mut entries = entries;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().map_or(false, |ext| ext == "cache") {
                tokio::fs::remove_file(entry.path()).await?;
            }
        }
        
        let mut current_size = self.current_size.write().await;
        *current_size = 0;
        
        Ok(())
    }

    async fn ensure_space(&self, needed_bytes: u64) -> Result<()> {
        let current_size = self.current_size.read().await;
        
        if *current_size + needed_bytes <= self.max_size_bytes {
            return Ok(());
        }
        
        drop(current_size);
        
        // Remove oldest files until we have enough space
        let mut entries: Vec<_> = {
            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&self.cache_dir).await?;
            
            while let Some(entry) = read_dir.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(created) = metadata.created() {
                        entries.push((entry.path(), created, metadata.len()));
                    }
                }
            }
            entries
        };
        
        // Sort by creation time (oldest first)
        entries.sort_by_key(|(_, created, _)| *created);
        
        let mut current_size = self.current_size.write().await;
        let mut freed_bytes = 0u64;
        
        for (path, _, size) in entries {
            if *current_size + needed_bytes - freed_bytes <= self.max_size_bytes {
                break;
            }
            
            if tokio::fs::remove_file(&path).await.is_ok() {
                freed_bytes += size;
            }
        }
        
        *current_size = current_size.saturating_sub(freed_bytes);
        Ok(())
    }

    pub async fn get_size(&self) -> u64 {
        *self.current_size.read().await
    }

    pub fn get_max_size(&self) -> u64 {
        self.max_size_bytes
    }
}

/// Combined cache manager
pub struct CacheManager {
    pub metadata: MetadataCache,
    pub files: FileCache,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf, max_file_cache_mb: u64, metadata_ttl_seconds: u64) -> Result<Self> {
        Ok(Self {
            metadata: MetadataCache::new(metadata_ttl_seconds),
            files: FileCache::new(cache_dir, max_file_cache_mb)?,
        })
    }

    pub async fn cleanup_expired(&self) -> Result<()> {
        self.metadata.clear_expired().await;
        Ok(())
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            metadata_entries: self.metadata.size().await,
            file_cache_size_bytes: self.files.get_size().await,
            file_cache_max_bytes: self.files.get_max_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub metadata_entries: usize,
    pub file_cache_size_bytes: u64,
    pub file_cache_max_bytes: u64,
}

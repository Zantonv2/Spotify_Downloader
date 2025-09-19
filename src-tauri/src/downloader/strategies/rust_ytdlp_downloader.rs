use super::super::super::downloader::{Downloader, DownloadTask, DownloadProgress};
use super::super::super::errors::Result;
use super::super::super::config::AppConfig;
use super::super::http_pool::HttpPoolManager;
use super::super::cache::CacheManager;
use super::super::ytdlp_rust::RustYtDlpDownloader as YtDlpDownloader;
use std::sync::Arc;
use std::path::PathBuf;

pub struct RustYtDlpDownloader {
    downloader: YtDlpDownloader,
}

impl RustYtDlpDownloader {
    pub fn new(config: AppConfig) -> Result<Self> {
        // Initialize HTTP pool
        let http_pool = HttpPoolManager::new(10, 30)
            .map_err(|e| {
                log::error!("❌ HTTP pool creation failed: {}", e);
                e
            })?;
        
        // Initialize cache
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| {
                log::warn!("⚠️ Could not get system cache dir, using ./cache");
                PathBuf::from("./cache")
            })
            .join("spotify_downloader");
        
        let cache = Arc::new(CacheManager::new(cache_dir, 1024, 3600)
            .map_err(|e| {
                log::error!("❌ Cache creation failed: {}", e);
                e
            })?);
        
        let downloader = YtDlpDownloader::new(http_pool.get_pool(), cache, config);
        
        log::info!("✅ Rust downloader initialized");
        Ok(Self { downloader })
    }

    pub fn with_proxy(mut self, proxy_url: &str, config: AppConfig) -> Result<Self> {
        // Recreate with proxy
        let http_pool = HttpPoolManager::new(10, 30)?
            .with_proxy(proxy_url)?;
        
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("./cache"))
            .join("spotify_downloader");
        let cache = Arc::new(CacheManager::new(cache_dir, 1024, 3600)?);
        
        self.downloader = YtDlpDownloader::new(http_pool.get_pool(), cache, config);
        Ok(self)
    }
}

#[async_trait::async_trait]
impl Downloader for RustYtDlpDownloader {
    async fn download(&self, task: &DownloadTask) -> Result<()> {
        let start_time = std::time::Instant::now();
        log::info!("🚀 [RUST] Started Downloading: {} - {}", task.track_info.artist, task.track_info.title);
        log::info!("📁 [RUST] Output path: {:?}", task.output_path);
        log::info!("🔗 [RUST] URL: {}", task.track_info.url);
        log::info!("📊 [RUST] Task ID: {}", task.id);
        
        let result = self.downloader.download(task).await;
        
        let duration = start_time.elapsed();
        match &result {
            Ok(_) => log::info!("✅ [RUST] Successfully downloaded: {} - {} (took {:.2} seconds)", 
                               task.track_info.artist, task.track_info.title, duration.as_secs_f64()),
            Err(e) => log::error!("❌ [RUST] Failed to download: {} - {} - Error: {} (took {:.2} seconds)", 
                                 task.track_info.artist, task.track_info.title, e, duration.as_secs_f64()),
        }
        
        result
    }

    async fn pause(&self, task_id: &str) -> Result<()> {
        self.downloader.pause(task_id).await
    }

    async fn resume(&self, task_id: &str) -> Result<()> {
        self.downloader.resume(task_id).await
    }

    async fn cancel(&self, task_id: &str) -> Result<()> {
        self.downloader.cancel(task_id).await
    }

    async fn get_progress(&self, task_id: &str) -> Result<DownloadProgress> {
        self.downloader.get_progress(task_id).await
    }

    fn supports_format(&self, format: &str) -> bool {
        self.downloader.supports_format(format)
    }

    fn get_name(&self) -> &str {
        self.downloader.get_name()
    }
}

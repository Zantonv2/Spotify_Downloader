use std::sync::Arc;
use std::time::Duration;
use reqwest::{Client, ClientBuilder, Proxy};
use serde::{Deserialize, Serialize};
use crate::errors::Result;

/// HTTP connection pool for efficient downloads
pub struct HttpPool {
    client: Client,
    max_connections: usize,
    timeout: Duration,
}

impl HttpPool {
    pub fn new(max_connections: usize, timeout_seconds: u64) -> Result<Self> {
        let timeout = Duration::from_secs(timeout_seconds);
        
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(max_connections)
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .gzip(true)
            .brotli(true)
            .http1_title_case_headers()
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()?;

        Ok(Self {
            client,
            max_connections,
            timeout,
        })
    }

    pub fn with_proxy(mut self, proxy_url: &str) -> Result<Self> {
        let proxy = Proxy::all(proxy_url)?;
        self.client = ClientBuilder::new()
            .pool_max_idle_per_host(self.max_connections)
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .timeout(self.timeout)
            .connect_timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .gzip(true)
            .brotli(true)
            .http1_title_case_headers()
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .proxy(proxy)
            .build()?;
        Ok(self)
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        mut progress_callback: F,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(u64, u64, f32) + Send + 'static,
    {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(2);

        for attempt in 1..=MAX_RETRIES {
            log::info!("🌐 [HTTP] Attempt {} of {} for URL: {}", attempt, MAX_RETRIES, url);
            
            // Try different approaches based on attempt number
            let result = match attempt {
                1 => self.try_download_with_headers(url, &mut progress_callback).await,
                2 => self.try_download_simple(url, &mut progress_callback).await,
                _ => self.try_download_with_headers(url, &mut progress_callback).await,
            };
            
            match result {
                Ok(data) => {
                    log::info!("✅ [HTTP] Download successful on attempt {} with method {}", attempt, 
                        if attempt == 2 { "simple" } else { "with headers" });
                    return Ok(data);
                }
                Err(e) => {
                    log::warn!("⚠️ [HTTP] Attempt {} failed: {}", attempt, e);
                    
                    if attempt < MAX_RETRIES {
                        log::info!("🔄 [HTTP] Retrying in {:?}...", RETRY_DELAY);
                        tokio::time::sleep(RETRY_DELAY).await;
                    } else {
                        log::error!("❌ [HTTP] All {} attempts failed", MAX_RETRIES);
                        return Err(e);
                    }
                }
            }
        }

        unreachable!()
    }

    async fn try_download_with_headers<F>(
        &self,
        url: &str,
        progress_callback: &mut F,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(u64, u64, f32) + Send + 'static,
    {
        let response = self.client
            .get(url)
            .header("Accept", "audio/webm,audio/*,*/*;q=0.9")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Range", "bytes=0-")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::AppError::DownloadError(
                format!("HTTP error: {}", response.status())
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        let mut data = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            data.extend_from_slice(&chunk);
            
            let progress = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };
            
            progress_callback(downloaded, total_size, progress);
        }

        Ok(data)
    }

    async fn try_download_simple<F>(
        &self,
        url: &str,
        progress_callback: &mut F,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(u64, u64, f32) + Send + 'static,
    {
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::AppError::DownloadError(
                format!("HTTP error: {}", response.status())
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        let mut data = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            data.extend_from_slice(&chunk);
            
            let progress = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };
            
            progress_callback(downloaded, total_size, progress);
        }

        Ok(data)
    }
}

/// Global HTTP pool manager
pub struct HttpPoolManager {
    pool: Arc<HttpPool>,
}

impl HttpPoolManager {
    pub fn new(max_connections: usize, timeout_seconds: u64) -> Result<Self> {
        let pool = HttpPool::new(max_connections, timeout_seconds)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub fn with_proxy(mut self, proxy_url: &str) -> Result<Self> {
        let pool = HttpPool::new(self.pool.max_connections, self.pool.timeout.as_secs())?
            .with_proxy(proxy_url)?;
        self.pool = Arc::new(pool);
        Ok(self)
    }

    pub fn get_pool(&self) -> Arc<HttpPool> {
        self.pool.clone()
    }
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress_percentage: f32,
    pub speed_bytes_per_sec: Option<u64>,
    pub estimated_remaining_secs: Option<u64>,
}

impl DownloadProgress {
    pub fn new(downloaded: u64, total: u64) -> Self {
        let progress = if total > 0 {
            (downloaded as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Self {
            downloaded_bytes: downloaded,
            total_bytes: total,
            progress_percentage: progress,
            speed_bytes_per_sec: None,
            estimated_remaining_secs: None,
        }
    }

    pub fn with_speed(mut self, speed: u64, _elapsed_secs: u64) -> Self {
        self.speed_bytes_per_sec = Some(speed);
        
        if speed > 0 && self.total_bytes > self.downloaded_bytes {
            let remaining_bytes = self.total_bytes - self.downloaded_bytes;
            self.estimated_remaining_secs = Some(remaining_bytes / speed);
        }
        
        self
    }
}

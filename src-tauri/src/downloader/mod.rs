pub mod manager;
pub mod strategies;

use serde::{Deserialize, Serialize};
use crate::errors::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub track_info: crate::api::TrackInfo,
    pub output_path: PathBuf,
    pub status: DownloadStatus,
    pub progress: f32,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub order: u32, // Track order in playlist/queue
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Processing,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub task_id: String,
    pub status: DownloadStatus,
    pub progress: f32,
    pub current_speed: Option<u64>, // bytes per second
    pub estimated_time_remaining: Option<u64>, // seconds
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

#[async_trait::async_trait]
pub trait Downloader {
    async fn download(&self, task: &DownloadTask) -> Result<()>;
    async fn pause(&self, task_id: &str) -> Result<()>;
    async fn resume(&self, task_id: &str) -> Result<()>;
    async fn cancel(&self, task_id: &str) -> Result<()>;
    async fn get_progress(&self, task_id: &str) -> Result<DownloadProgress>;
    fn supports_format(&self, format: &str) -> bool;
    fn get_name(&self) -> &str;
}

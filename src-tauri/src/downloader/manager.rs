use crate::downloader::{Downloader, DownloadTask, DownloadStatus, DownloadProgress};
use crate::downloader::strategies::{PythonDownloader, RustYtDlpDownloader};
use crate::errors::Result;
use crate::config::AppConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::utils::generate_download_id;

pub struct DownloadManager {
    tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    downloaders: Vec<Arc<dyn Downloader + Send + Sync>>,
    max_concurrent: usize,
    active_downloads: Arc<Mutex<usize>>,
    config: Arc<Mutex<AppConfig>>,
}

impl DownloadManager {
    pub fn new(max_concurrent: usize, config: AppConfig) -> Self {
        let mut downloaders: Vec<Arc<dyn Downloader + Send + Sync>> = Vec::new();
        
        // Add Rust yt-dlp downloader (primary) FIRST
        log::info!("🔧 Initializing Rust yt-dlp downloader...");
        match RustYtDlpDownloader::new(config.clone()) {
            Ok(rust_downloader) => {
                downloaders.push(Arc::new(rust_downloader));
                log::info!("✅ Rust yt-dlp downloader initialized successfully");
            },
            Err(e) => {
                log::error!("❌ Failed to initialize Rust yt-dlp downloader: {}", e);
                log::warn!("⚠️ Using Python fallback only");
            }
        }
        
        // Add Python downloader (fallback) SECOND
        downloaders.push(Arc::new(PythonDownloader::new()));
        
        log::info!("📊 {} downloaders available", downloaders.len());
        
        let manager = Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            downloaders,
            max_concurrent,
            active_downloads: Arc::new(Mutex::new(0)),
            config: Arc::new(Mutex::new(config)),
        };
        
        // Start background progress updater
        manager.start_progress_updater();
        
        // Start background queue processor
        manager.start_queue_processor();
        
        manager
    }

    pub fn add_downloader(&mut self, downloader: Arc<dyn Downloader + Send + Sync>) {
        self.downloaders.push(downloader);
    }

    pub async fn add_download(&self, track_info: crate::api::TrackInfo, output_path: std::path::PathBuf) -> Result<String> {
        self.add_download_with_auto_start(track_info, output_path, true).await
    }

    pub async fn add_download_with_auto_start(&self, track_info: crate::api::TrackInfo, output_path: std::path::PathBuf, auto_start: bool) -> Result<String> {
        self.add_download_with_order(track_info, output_path, auto_start, 0).await
    }

    pub async fn add_download_with_order(&self, track_info: crate::api::TrackInfo, output_path: std::path::PathBuf, auto_start: bool, order: u32) -> Result<String> {
        // Check if file already exists and is valid
        if output_path.exists() {
            if let Ok(file_metadata) = std::fs::metadata(&output_path) {
                if file_metadata.len() > 1024 { // At least 1KB
                    // Additional verification: check if it's a valid audio file
                    if self.verify_audio_file(&output_path).await {
                        log::info!("✅ [VERIFY] File already exists and is valid: {:?}", output_path);
                        return Ok("already_downloaded".to_string());
                    } else {
                        log::warn!("⚠️ [VERIFY] File exists but appears corrupted, will re-download: {:?}", output_path);
                        // Remove corrupted file
                        let _ = std::fs::remove_file(&output_path);
                    }
                }
            }
        }

        let task_id = generate_download_id();
        let task = DownloadTask {
            id: task_id.clone(),
            track_info,
            output_path,
            status: DownloadStatus::Pending,
            progress: 0.0,
            error: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            order,
        };

        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(task_id.clone(), task);
        }

        // Background processor will handle queue processing automatically

        Ok(task_id)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<DownloadTask>> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.get(task_id).cloned())
    }

    pub async fn get_all_tasks(&self) -> Result<Vec<DownloadTask>> {
        let tasks = self.tasks.lock().await;
        let mut task_list: Vec<DownloadTask> = tasks.values().cloned().collect();
        
        // Sort by status priority, then by creation time
        task_list.sort_by(|a, b| {
            // First, sort by status priority
            let status_priority = |status: &DownloadStatus| -> u8 {
                match status {
                    DownloadStatus::Downloading => 1,
                    DownloadStatus::Pending => 2,
                    DownloadStatus::Paused => 3,
                    DownloadStatus::Processing => 4,
                    DownloadStatus::Completed => 5,
                    DownloadStatus::Failed => 6,
                    DownloadStatus::Cancelled => 7,
                }
            };
            
            status_priority(&a.status).cmp(&status_priority(&b.status))
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        
        // Renumber tasks based on their position in the sorted list
        for (index, task) in task_list.iter_mut().enumerate() {
            task.order = (index + 1) as u32;
        }
        
        Ok(task_list)
    }

    pub async fn get_next_individual_order(&self) -> u32 {
        // Get all tasks to determine the next order number
        let all_tasks = match self.get_all_tasks().await {
            Ok(tasks) => tasks,
            Err(_) => return 1, // Default to 1 if we can't get tasks
        };
        
        // Count pending and downloading tasks to get the next position
        let active_count = all_tasks.iter()
            .filter(|task| matches!(task.status, DownloadStatus::Pending | DownloadStatus::Downloading))
            .count();
        
        // Return the next position (1-based indexing)
        (active_count + 1) as u32
    }

    async fn renumber_queue(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        
        // Get all pending tasks sorted by creation time
        let mut pending_tasks: Vec<(String, chrono::DateTime<chrono::Utc>)> = tasks
            .iter()
            .filter(|(_, task)| task.status == DownloadStatus::Pending)
            .map(|(id, task)| (id.clone(), task.created_at))
            .collect();
        
        // Sort by creation time to maintain order
        pending_tasks.sort_by(|a, b| a.1.cmp(&b.1));
        
        // Renumber the tasks sequentially
        for (index, (task_id, _)) in pending_tasks.iter().enumerate() {
            if let Some(task) = tasks.get_mut(task_id) {
                task.order = (index + 1) as u32;
            }
        }
        
        Ok(())
    }

    pub async fn pause_download(&self, task_id: &str) -> Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                if task.status == DownloadStatus::Downloading {
                    task.status = DownloadStatus::Paused;
                }
            }
        }

        // Try to pause in all downloaders
        for downloader in &self.downloaders {
            let _ = downloader.pause(task_id).await;
        }

        Ok(())
    }

    pub async fn resume_download(&self, task_id: &str) -> Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                if task.status == DownloadStatus::Paused {
                    task.status = DownloadStatus::Pending;
                }
            }
        }

        self.process_queue().await?;
        Ok(())
    }

    pub async fn cancel_download(&self, task_id: &str) -> Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.status = DownloadStatus::Cancelled;
            }
        }

        // Try to cancel in all downloaders
        for downloader in &self.downloaders {
            let _ = downloader.cancel(task_id).await;
        }

        Ok(())
    }

    pub async fn remove_download(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        tasks.remove(task_id);
        Ok(())
    }

    pub async fn reorder_queue(&self, _task_ids: Vec<String>) -> Result<()> {
        // For now, we'll just return Ok since the HashMap doesn't maintain order
        // In a real implementation, we'd use a VecDeque or similar ordered structure
        Ok(())
    }

    pub async fn get_progress(&self, task_id: &str) -> Result<Option<DownloadProgress>> {
        // Try to get progress from downloaders first
        for downloader in &self.downloaders {
            if let Ok(progress) = downloader.get_progress(task_id).await {
                return Ok(Some(progress));
            }
        }

        // Fallback to task status
        let tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get(task_id) {
            Ok(Some(DownloadProgress {
                task_id: task.id.clone(),
                status: task.status.clone(),
                progress: task.progress,
                current_speed: None,
                estimated_time_remaining: None,
                downloaded_bytes: None,
                total_bytes: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_task_progress(&self, task_id: &str, progress: f32, status: Option<DownloadStatus>) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.progress = progress;
            if let Some(new_status) = status {
                task.status = new_status;
            }
        }
        Ok(())
    }

    fn start_progress_updater(&self) {
        let tasks = self.tasks.clone();
        let downloaders = self.downloaders.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000)); // Update every second
            
            loop {
                interval.tick().await;
                
                // Get all downloading tasks
                let downloading_tasks: Vec<String> = {
                    let tasks = tasks.lock().await;
                    tasks.values()
                        .filter(|task| task.status == DownloadStatus::Downloading)
                        .map(|task| task.id.clone())
                        .collect()
                };
                
                // Update progress for each downloading task
                for task_id in downloading_tasks {
                    for downloader in &downloaders {
                        if let Ok(progress) = downloader.get_progress(&task_id).await {
                            let mut tasks = tasks.lock().await;
                            if let Some(task) = tasks.get_mut(&task_id) {
                                task.progress = progress.progress;
                                if progress.status != task.status {
                                    task.status = progress.status.clone();
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn start_queue_processor(&self) {
        let tasks = self.tasks.clone();
        let active_downloads = self.active_downloads.clone();
        let max_concurrent = self.max_concurrent;
        let downloaders = self.downloaders.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100)); // Check every 100ms
            loop {
                interval.tick().await;
                
                // Check if auto-start downloads is enabled
                let auto_start_enabled = {
                    let config_guard = config.lock().await;
                    config_guard.ui.auto_start_downloads
                };
                
                // Always process the queue to continue downloads that were manually started
                // Only skip if auto-start is disabled AND there are no active downloads
                let active_count = *active_downloads.lock().await;
                if !auto_start_enabled && active_count == 0 {
                    continue; // Skip processing if auto-start is disabled and no active downloads
                }
                
                // Process as many pending downloads as possible up to max_concurrent
                loop {
                    let active_count = *active_downloads.lock().await;
                    if active_count >= max_concurrent {
                        log::debug!("🔄 [AUTO-QUEUE] Max concurrent reached ({}/{})", active_count, max_concurrent);
                        break; // No more capacity
                    }

                    let tasks_guard = tasks.lock().await;
                    let pending_tasks: Vec<String> = tasks_guard
                        .values()
                        .filter(|task| task.status == crate::downloader::DownloadStatus::Pending)
                        .map(|task| task.id.clone())
                        .collect();
                    drop(tasks_guard);
                    
                    if pending_tasks.is_empty() {
                        log::debug!("🔄 [AUTO-QUEUE] No pending tasks found");
                        break; // No more pending tasks
                    }

                    // Only start new downloads if auto-start is enabled OR if there are already active downloads
                    if !auto_start_enabled && active_count == 0 {
                        break; // Don't start new downloads if auto-start is disabled and no active downloads
                    }

                    let next_task_id = pending_tasks[0].clone();
                    log::info!("🔄 [AUTO-QUEUE] Background processor starting download: {}", next_task_id);
                    
                    // Update task status to downloading
                    if let Err(e) = Self::update_task_status_static(&tasks, &next_task_id, crate::downloader::DownloadStatus::Downloading, None).await {
                        log::error!("Failed to update task status: {}", e);
                        break;
                    }

                    // Increment active downloads counter
                    {
                        let mut active_count = active_downloads.lock().await;
                        *active_count += 1;
                    }

                    // Start the download in a separate task
                    let tasks_clone = tasks.clone();
                    let downloaders_clone = downloaders.clone();
                    let active_downloads_clone = active_downloads.clone();
                    let max_concurrent_clone = max_concurrent;
                    let next_task_id_clone = next_task_id.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::process_single_download_static(tasks_clone, downloaders_clone, max_concurrent_clone, active_downloads_clone, next_task_id_clone).await {
                            log::error!("Background download failed: {}", e);
                        }
                    });
                }
            }
        });
    }

    pub async fn process_queue(&self) -> Result<()> {
        // Process as many pending downloads as possible up to max_concurrent
        let mut processed_count = 0;
        
        loop {
            let active_count = *self.active_downloads.lock().await;
            if active_count >= self.max_concurrent {
                log::info!("📊 Queue processing: Max concurrent downloads reached ({}/{})", active_count, self.max_concurrent);
                break;
            }

            let tasks_guard = self.tasks.lock().await;
            let pending_tasks: Vec<String> = tasks_guard
                .values()
                .filter(|task| task.status == crate::downloader::DownloadStatus::Pending)
                .map(|task| task.id.clone())
                .collect();
            drop(tasks_guard);

            if pending_tasks.is_empty() {
                log::info!("📊 Queue processing: No pending tasks found");
                break;
            }

            // Start the first pending task
            let task_id = pending_tasks[0].clone();
            log::info!("🔄 [QUEUE] Processing download: {}", task_id);
            
            self.start_download(task_id).await?;
            
            processed_count += 1;
            log::info!("📊 Queue processing: Started {} downloads", processed_count);
        }
        
        log::info!("📊 Queue processing completed: {} downloads started", processed_count);
        Ok(())
    }

    fn find_suitable_downloader(&self, task: &DownloadTask) -> Option<&Arc<dyn Downloader + Send + Sync>> {
        let format = task.track_info.format.as_deref().unwrap_or("mp3");
        log::info!("🔍 Finding suitable downloader for format: {}", format);
        log::info!("📋 Available downloaders: {:?}", self.downloaders.iter().map(|d| d.get_name()).collect::<Vec<_>>());
        
        // Use first suitable downloader (Rust is first, Python is fallback)
        for downloader in &self.downloaders {
            if downloader.supports_format(format) {
                log::info!("✅ Using {} downloader for: {} - {}", downloader.get_name(), task.track_info.artist, task.track_info.title);
                return Some(downloader);
            } else {
                log::info!("❌ {} doesn't support format: {}", downloader.get_name(), format);
            }
        }
        
        // Last resort: return first available
        log::warn!("⚠️ No suitable downloader found, using first available");
        self.downloaders.first()
    }

    pub async fn start_download(&self, task_id: String) -> Result<()> {
        // Check if we have capacity
        let active_count = *self.active_downloads.lock().await;
        if active_count >= self.max_concurrent {
            return Err(crate::errors::AppError::DownloadError("Maximum concurrent downloads reached".to_string()));
        }

        // Get the task
        let task = {
            let tasks = self.tasks.lock().await;
            tasks.get(&task_id).cloned()
        };

        if let Some(mut task) = task {
            if task.status != crate::downloader::DownloadStatus::Pending {
                // If task is already completed, that's fine - don't return an error
                if task.status == crate::downloader::DownloadStatus::Completed {
                    return Ok(());
                }
                // If task is already downloading, that's also fine - don't return an error
                if task.status == crate::downloader::DownloadStatus::Downloading {
                    return Ok(());
                }
                return Err(crate::errors::AppError::DownloadError("Task is not in pending status".to_string()));
            }

            // Update task status
            task.status = crate::downloader::DownloadStatus::Downloading;
            task.started_at = Some(chrono::Utc::now());

            // Update the task in the manager
            {
                let mut tasks = self.tasks.lock().await;
                tasks.insert(task_id.clone(), task);
            }

            // Increment active downloads counter
            {
                let mut active_count = self.active_downloads.lock().await;
                *active_count += 1;
            }

            // Start the actual download in a separate task
            let tasks = self.tasks.clone();
            let downloaders = self.downloaders.clone();
            let max_concurrent = self.max_concurrent;
            let active_downloads = self.active_downloads.clone();
            let task_id_clone = task_id.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::process_single_download_static(tasks, downloaders, max_concurrent, active_downloads, task_id_clone).await {
                    log::error!("Download failed: {}", e);
                }
            });

            Ok(())
        } else {
            Err(crate::errors::AppError::DownloadError("Task not found".to_string()))
        }
    }

    async fn process_single_download_static(
        tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
        downloaders: Vec<Arc<dyn Downloader + Send + Sync>>,
        max_concurrent: usize,
        active_downloads: Arc<Mutex<usize>>,
        task_id: String,
    ) -> Result<()> {
        // Find a suitable downloader
        let task = {
            let tasks = tasks.lock().await;
            tasks.get(&task_id).cloned()
        };

        if let Some(task) = task {
            let downloader = downloaders.first()
                .ok_or_else(|| crate::errors::AppError::DownloadError("No suitable downloader found".to_string()))?;
            
            // Execute the download
            if let Err(e) = downloader.download(&task).await {
                // Update task status to failed
                Self::update_task_status_static(&tasks, &task_id, crate::downloader::DownloadStatus::Failed, Some(e.to_string())).await?;
                return Err(e);
            }

            // Update task status to completed
            Self::update_task_status_static(&tasks, &task_id, crate::downloader::DownloadStatus::Completed, None).await?;
        }

        // Decrement active downloads counter
        {
            let mut active_count = active_downloads.lock().await;
            *active_count = active_count.saturating_sub(1);
            log::info!("📊 [QUEUE] Download completed. Active downloads: {}", *active_count);
        }

        // Log that a slot is now available for the next download
        let active_count = *active_downloads.lock().await;
        let tasks_guard = tasks.lock().await;
        let pending_count = tasks_guard
            .values()
            .filter(|task| task.status == crate::downloader::DownloadStatus::Pending)
            .count();
        drop(tasks_guard);

        if pending_count > 0 {
            log::info!("🔄 [AUTO-QUEUE] {} pending tasks available, {} active downloads. Background processor will handle them.", pending_count, active_count);
        }

        Ok(())
    }

    async fn process_single_download(&self, task_id: String) -> Result<()> {
        // Find a suitable downloader
        let task = {
            let tasks = self.tasks.lock().await;
            tasks.get(&task_id).cloned()
        };

        if let Some(task) = task {
            let downloader = self.find_suitable_downloader(&task)
                .ok_or_else(|| crate::errors::AppError::DownloadError("No suitable downloader found".to_string()))?;
            
            // Execute the download
            if let Err(e) = downloader.download(&task).await {
                // Update task status to failed
                self.update_task_status(&task_id, crate::downloader::DownloadStatus::Failed, Some(e.to_string())).await?;
                return Err(e);
            }

            // Update task status to completed
            self.update_task_status(&task_id, crate::downloader::DownloadStatus::Completed, None).await?;
        }

        // Decrement active downloads counter
        {
            let mut active_count = self.active_downloads.lock().await;
            *active_count = active_count.saturating_sub(1);
        }

        // Process the queue to start more downloads
        self.process_queue().await?;

        Ok(())
    }

    pub async fn update_task_status(&self, task_id: &str, status: crate::downloader::DownloadStatus, error: Option<String>) -> Result<()> {
        Self::update_task_status_static(&self.tasks, task_id, status, error).await
    }

    async fn update_task_status_static(
        tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
        task_id: &str,
        status: crate::downloader::DownloadStatus,
        error: Option<String>,
    ) -> Result<()> {
        let is_completed = status == crate::downloader::DownloadStatus::Completed;
        let is_failed = status == crate::downloader::DownloadStatus::Failed;
        
        {
            let mut tasks = tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.status = status;
                if let Some(err) = error {
                    task.error = Some(err);
                }
                if is_completed {
                    task.progress = 100.0;
                    task.completed_at = Some(chrono::Utc::now());
                }
            }
        }
        
        // If a task was completed or failed, renumber the remaining pending tasks
        if is_completed || is_failed {
            // We need to renumber the queue, but we need access to the DownloadManager instance
            // For now, we'll handle this in the process_queue function
        }
        
        Ok(())
    }

    async fn process_queue_static(
        tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
        downloaders: Vec<Arc<dyn Downloader + Send + Sync>>,
        max_concurrent: usize,
        active_downloads: Arc<Mutex<usize>>,
    ) -> Result<()> {
        let active_count = *active_downloads.lock().await;
        if active_count >= max_concurrent {
            return Ok(());
        }

        let tasks_guard = tasks.lock().await;
        let pending_tasks: Vec<String> = tasks_guard
            .values()
            .filter(|task| task.status == crate::downloader::DownloadStatus::Pending)
            .map(|task| task.id.clone())
            .collect();

        drop(tasks_guard);

        for task_id in pending_tasks {
            let active_count = *active_downloads.lock().await;
            if active_count >= max_concurrent {
                break;
            }

            // Update task status to downloading
            Self::update_task_status_static(&tasks, &task_id, crate::downloader::DownloadStatus::Downloading, None).await?;

            // Increment active downloads counter
            {
                let mut active_count = active_downloads.lock().await;
                *active_count += 1;
            }

            // Start the download in a separate task
            let tasks_clone = tasks.clone();
            let downloaders_clone = downloaders.clone();
            let active_downloads_clone = active_downloads.clone();
            let task_id_clone = task_id.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::process_single_download_static(tasks_clone, downloaders_clone, max_concurrent, active_downloads_clone, task_id_clone).await {
                    log::error!("Download failed: {}", e);
                }
            });
        }

        Ok(())
    }

    // Automatic queue processor removed to prevent race conditions
    // Use process_queue() method for manual queue processing instead

    async fn verify_audio_file(&self, file_path: &std::path::Path) -> bool {
        // Check file extension
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            if !matches!(ext, "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "ape") {
                return false;
            }
        } else {
            return false;
        }

        // Try to read the file with ffprobe to verify it's a valid audio file
        let output = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(file_path)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    // Check if the output contains audio stream information
                    let output_str = String::from_utf8_lossy(&result.stdout);
                    output_str.contains("codec_type=audio")
                } else {
                    false
                }
            }
            Err(_) => {
                // If ffprobe is not available, fall back to basic file size check
                if let Ok(metadata) = std::fs::metadata(file_path) {
                    metadata.len() > 1024 // At least 1KB
                } else {
                    false
                }
            }
        }
    }

    pub async fn verify_downloads(&self, output_directory: &std::path::Path) -> Result<DownloadVerification> {
        log::info!("🔍 [VERIFICATION] Starting download verification...");
        
        let tasks = self.tasks.lock().await;
        let mut total_tasks = 0;
        let mut completed_tasks = 0;
        let mut actual_files = 0;
        let mut missing_files = Vec::new();
        let mut oversized_files = Vec::new();
        
        // Count tasks and check their status
        for (task_id, task) in tasks.iter() {
            total_tasks += 1;
            
            if task.status == DownloadStatus::Completed {
                completed_tasks += 1;
                
                // Check if file actually exists
                if task.output_path.exists() {
                    actual_files += 1;
                    
                    // Check file size
                    if let Ok(metadata) = std::fs::metadata(&task.output_path) {
                        let file_size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                        if file_size_mb > 60.0 {
                            oversized_files.push((task_id.clone(), file_size_mb));
                        }
                    }
                } else {
                    missing_files.push(task_id.clone());
                }
            }
        }
        
        let verification = DownloadVerification {
            total_tasks,
            completed_tasks,
            actual_files,
            missing_files,
            oversized_files,
        };
        
        log::info!("📊 [VERIFICATION] Results:");
        log::info!("   Total tasks: {}", verification.total_tasks);
        log::info!("   Completed tasks: {}", verification.completed_tasks);
        log::info!("   Actual files: {}", verification.actual_files);
        log::info!("   Missing files: {}", verification.missing_files.len());
        log::info!("   Oversized files: {}", verification.oversized_files.len());
        
        if !verification.missing_files.is_empty() {
            log::warn!("⚠️ [VERIFICATION] Missing files: {:?}", verification.missing_files);
        }
        
        if !verification.oversized_files.is_empty() {
            log::warn!("⚠️ [VERIFICATION] Oversized files (likely albums):");
            for (task_id, size) in &verification.oversized_files {
                log::warn!("   {}: {:.1}MB", task_id, size);
            }
        }
        
        Ok(verification)
    }
}

#[derive(Debug, Clone)]
pub struct DownloadVerification {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub actual_files: usize,
    pub missing_files: Vec<String>,
    pub oversized_files: Vec<(String, f64)>, // (task_id, size_mb)
}

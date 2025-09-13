use serde::{Deserialize, Serialize};
use crate::api::TrackInfo;
use crate::errors::{AppError, Result};
use crate::utils::execute_python_script_with_ffmpeg;
use crate::commands::get_ffmpeg_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: Option<usize>,
    pub platforms: Option<Vec<String>>,
    pub deep_search: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResult {
    pub tracks: Vec<TrackInfo>,
    pub total: usize,
    pub sources_used: Vec<String>,
    pub deduplicated: bool,
}

pub struct SearchManager {
    // No longer need individual clients - we use Python processor
}

impl SearchManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<UnifiedSearchResult> {
        // Use Python processor for search
        let platforms = match &query.platforms {
            Some(platforms) if !platforms.is_empty() => platforms.clone(),
            _ => vec![
                "youtube".to_string(),
                "soundcloud".to_string(),
                "bandcamp".to_string(),
                "vimeo".to_string()
            ]
        };
        
        let search_request = serde_json::json!({
            "action": "search",
            "query": query.query,
            "limit": query.limit.unwrap_or(3),  // Reduced from 10 to 3 for faster search
            "deep_search": query.deep_search,
            "platforms": platforms
        });

        // Get FFmpeg path and use it when calling Python script
        let ffmpeg_path = get_ffmpeg_path().await?;
        let result = execute_python_script_with_ffmpeg("../python_processor/audio_processor.py", search_request, ffmpeg_path).await?;
        
        if let Some(error) = result.get("error") {
            return Err(AppError::ApiError(error.as_str().unwrap_or("Search failed").to_string()));
        }

        let tracks: Vec<TrackInfo> = result
            .get("tracks")
            .and_then(|t| t.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| serde_json::from_value(t.clone()).ok())
            .collect();

        let search_type = result.get("search_type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");
        
        Ok(UnifiedSearchResult {
            tracks: tracks.clone(),
            total: result.get("total").and_then(|t| t.as_u64()).unwrap_or(0) as usize,
            sources_used: vec![search_type.to_string()],
            deduplicated: search_type == "deep",
        })
    }

    pub async fn deep_search(&self, query: String, limit: usize) -> Result<UnifiedSearchResult> {
        self.search(&SearchQuery {
            query,
            limit: Some(limit),
            platforms: None,
            deep_search: true,
        }).await
    }

    pub async fn quick_search(&self, query: String, limit: usize) -> Result<UnifiedSearchResult> {
        self.search(&SearchQuery {
            query,
            limit: Some(limit),
            platforms: None,
            deep_search: false,
        }).await
    }

    pub fn get_available_sources(&self) -> Vec<String> {
        vec![
            "youtube".to_string(),
            "soundcloud".to_string(),
            "bandcamp".to_string(),
            "vimeo".to_string(),
        ]
    }
}

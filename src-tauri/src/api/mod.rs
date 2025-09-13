// API modules removed - using direct HTTP clients in metadata providers

use serde::{Deserialize, Serialize};
// Result import removed - not used

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<u64>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub thumbnail_url: Option<String>,
    pub source: String,
    pub url: String,
    pub isrc: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub composer: Option<String>,
    pub quality: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub tracks: Vec<TrackInfo>,
    pub total: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataInfo {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub album_artist: Option<String>,
    pub composer: Option<String>,
    pub isrc: Option<String>,
    pub cover_art_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration: Option<u64>,
    pub lyrics: Option<String>,
}

// ApiClient trait removed - not needed

// embedder, spotify, musicbrainz modules removed - using providers instead
pub mod lyrics;
pub mod providers;

use serde::{Deserialize, Serialize};
use crate::errors::Result;
use std::path::PathBuf;

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
    pub lyrics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtInfo {
    pub url: String,
    pub data: Option<Vec<u8>>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CoverArtData {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[async_trait::async_trait]
pub trait MetadataEmbedder {
    async fn embed_metadata(&self, file_path: &PathBuf, metadata: &MetadataInfo) -> Result<()>;
    async fn embed_cover_art(&self, file_path: &PathBuf, cover_art: &CoverArtInfo) -> Result<()>;
    async fn embed_lyrics(&self, file_path: &PathBuf, lyrics: &str) -> Result<()>;
    async fn read_metadata(&self, file_path: &PathBuf) -> Result<MetadataInfo>;
    fn supports_format(&self, format: &str) -> bool;
}

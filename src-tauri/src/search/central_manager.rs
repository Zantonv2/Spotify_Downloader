use crate::search::{SearchManager, SearchQuery, UnifiedSearchResult};
use crate::api::{spotify::SpotifyClient, musicbrainz::MusicBrainzClient};
use crate::config::AppConfig;
use crate::errors::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CentralSearchManager {
    search_manager: Arc<Mutex<SearchManager>>,
}

impl CentralSearchManager {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let mut search_manager = SearchManager::new();

        // Add Spotify client if configured
        if config.api_keys.spotify_client_id.is_some() && config.api_keys.spotify_client_secret.is_some() {
            match SpotifyClient::new(&config.api_keys) {
                Ok(spotify_client) => {
                    search_manager.add_client(
                        "spotify".to_string(),
                        Box::new(spotify_client),
                    );
                }
                Err(e) => {
                    log::warn!("Failed to initialize Spotify client: {}", e);
                }
            }
        }

        // Add MusicBrainz client
        let musicbrainz_client = MusicBrainzClient::new(&config.api_keys);
        search_manager.add_client(
            "musicbrainz".to_string(),
            Box::new(musicbrainz_client),
        );

        Ok(Self {
            search_manager: Arc::new(Mutex::new(search_manager)),
        })
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<UnifiedSearchResult> {
        let mut manager = self.search_manager.lock().await;
        manager.search(query).await
    }

    pub async fn get_available_sources(&self) -> Vec<String> {
        let manager = self.search_manager.lock().await;
        manager.get_available_sources()
    }
}

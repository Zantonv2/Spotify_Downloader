use crate::errors::Result;
use reqwest::{Client, Proxy};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub struct LyricsProvider {
    client: reqwest::Client,
    api_keys: HashMap<String, String>,
}

impl LyricsProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_keys: HashMap::new(),
        }
    }

    pub fn new_with_proxy(proxy_url: Option<String>) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(false)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

        if let Some(proxy_url) = proxy_url {
            if let Ok(proxy) = Proxy::all(&proxy_url) {
                builder = builder.proxy(proxy);
            }
        }

        Self {
            client: builder.build().unwrap_or_else(|_| Client::new()),
            api_keys: HashMap::new(),
        }
    }

    pub fn set_api_key(&mut self, service: &str, key: String) {
        self.api_keys.insert(service.to_string(), key);
    }

    pub async fn search_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>> {
        log::info!("🎵 Searching for lyrics: {} - {}", artist, title);
        
        // Run all lyrics providers in parallel for much faster results
        let search_future = async {
            let lrclib_future = self.try_lrclib(artist, title);
            let lyrics_ovh_future = self.try_lyrics_ovh(artist, title);
            let musixmatch_future = self.try_musixmatch(artist, title);
            let genius_future = self.try_genius(artist, title);
            
            // Wait for all providers to complete
            let (lrclib_result, lyrics_ovh_result, musixmatch_result, genius_result) = tokio::join!(
                lrclib_future,
                lyrics_ovh_future,
                musixmatch_future,
                genius_future
            );
            
            // Check results in order of preference
            let providers = vec![
                ("LRC Lib", lrclib_result),
                ("Lyrics.ovh", lyrics_ovh_result),
                ("Musixmatch", musixmatch_result),
                ("Genius", genius_result),
            ];

            for (provider_name, result) in providers {
                match result {
                    Ok(Some(lyrics)) => {
                        log::info!("✅ Found lyrics from {}: {} characters", provider_name, lyrics.len());
                        return Ok(Some(lyrics));
                    }
                    Ok(None) => {
                        log::debug!("⚠️ {}: No lyrics found", provider_name);
                        continue;
                    }
                    Err(e) => {
                        log::debug!("❌ {} error: {}", provider_name, e);
                        continue;
                    }
                }
            }

            log::warn!("No lyrics found from any provider");
            Ok(None)
        };

        // Reduced timeout from 30s to 10s since we're running in parallel
        match tokio::time::timeout(Duration::from_secs(10), search_future).await {
            Ok(result) => result,
            Err(_) => {
                log::warn!("Lyrics search timed out after 10 seconds");
                Ok(None)
            }
        }
    }

    async fn try_lrclib(&self, artist: &str, title: &str) -> Result<Option<String>> {
        let url = format!(
            "https://lrclib.net/api/search?q={} {}",
            urlencoding::encode(artist),
            urlencoding::encode(title)
        );

        log::info!("🔍 LRC Lib URL: {}", url);
        let response = self.client.get(&url).send().await;
        
        match response {
            Ok(response) => {
                log::info!("📡 LRC Lib response status: {}", response.status());
                if response.status().is_success() {
                    let json: Value = response.json().await?;
                    log::debug!("📄 LRC Lib JSON response: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                    
                    if let Some(tracks) = json.as_array() {
                        log::info!("🎵 Found {} tracks from LRC Lib", tracks.len());
                        if let Some(track) = tracks.first() {
                            log::debug!("🎵 First track: {}", serde_json::to_string_pretty(track).unwrap_or_default());
                            if let Some(lyrics) = track["syncedLyrics"].as_str() {
                                if !lyrics.is_empty() {
                                    log::debug!("✅ LRC Lib: Found lyrics with {} characters", lyrics.len());
                                    return Ok(Some(lyrics.to_string()));
                                } else {
                                    log::debug!("⚠️ LRC Lib: Empty lyrics");
                                }
                            } else {
                                log::debug!("⚠️ LRC Lib: No syncedLyrics field");
                            }
                        } else {
                            log::debug!("⚠️ LRC Lib: No tracks in response");
                        }
                    } else {
                        log::debug!("⚠️ LRC Lib: No data array in response");
                    }
                } else {
                    log::debug!("❌ LRC Lib returned status: {}", response.status());
                }
            }
            Err(e) => {
                log::debug!("❌ LRC Lib request failed: {}", e);
            }
        }
        
        Ok(None)
    }

    async fn try_lyrics_ovh(&self, artist: &str, title: &str) -> Result<Option<String>> {
        let url = format!(
            "https://api.lyrics.ovh/v1/{}/{}",
            urlencoding::encode(artist),
            urlencoding::encode(title)
        );

        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let json: Value = response.json().await?;
            if let Some(lyrics) = json["lyrics"].as_str() {
                if !lyrics.is_empty() && !lyrics.contains("Sorry, we don't have lyrics for this song yet.") {
                    return Ok(Some(lyrics.to_string()));
                }
            }
        }

        Ok(None)
    }

    async fn try_musixmatch(&self, artist: &str, title: &str) -> Result<Option<String>> {
        if let Some(api_key) = self.api_keys.get("musixmatch") {
            let url = format!(
                "https://api.musixmatch.com/ws/1.1/matcher.lyrics.get?apikey={}&q_track={}&q_artist={}",
                api_key,
                urlencoding::encode(title),
                urlencoding::encode(artist)
            );

            let response = self.client.get(&url).send().await?;
            
            if response.status().is_success() {
                let json: Value = response.json().await?;
                if let Some(lyrics) = json["message"]["body"]["lyrics"]["lyrics_body"].as_str() {
                    if !lyrics.is_empty() && !lyrics.contains("******* This Lyrics is NOT for Commercial use *******") {
                        return Ok(Some(lyrics.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn try_genius(&self, artist: &str, title: &str) -> Result<Option<String>> {
        if let Some(access_token) = self.api_keys.get("genius") {
            // First search for the song
            let search_url = format!(
                "https://api.genius.com/search?q={} {}",
                urlencoding::encode(artist),
                urlencoding::encode(title)
            );

            let response = self.client
                .get(&search_url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if response.status().is_success() {
                let json: Value = response.json().await?;
                if let Some(hits) = json["response"]["hits"].as_array() {
                    if let Some(hit) = hits.first() {
                        if let Some(song_id) = hit["result"]["id"].as_u64() {
                            // Get lyrics for the song
                            let lyrics_url = format!("https://api.genius.com/songs/{}", song_id);
                            let lyrics_response = self.client
                                .get(&lyrics_url)
                                .header("Authorization", format!("Bearer {}", access_token))
                                .send()
                                .await?;
                            
                            if lyrics_response.status().is_success() {
                                let lyrics_json: Value = lyrics_response.json().await?;
                                if let Some(lyrics) = lyrics_json["response"]["song"]["lyrics"]["plain"].as_str() {
                                    if !lyrics.is_empty() {
                                        return Ok(Some(lyrics.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

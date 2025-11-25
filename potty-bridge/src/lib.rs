// Module declarations
mod auth;
mod error;
mod player;
mod spotify;
mod types;

// Re-exports for UniFFI
pub use error::PottyError;
pub use types::{
    PottyAlbum, PottyArtist, PottyDelegate, PottyPlayerEvent, PottyPlaylist, PottySearchResults,
    PottySearchType, PottyTrack,
};

use auth::AuthManager;
use librespot_core::authentication::Credentials;
use librespot_core::{Session, SessionConfig};
use player::PlayerManager;
use spotify::SpotifyApiManager;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// Main client for the Potty Spotify bridge
#[derive(uniffi::Object)]
pub struct PottyClient {
    runtime: Arc<Runtime>,
    session: Arc<Mutex<Option<Session>>>,
    player: Arc<Mutex<Option<PlayerManager>>>,
    spotify_api: Arc<Mutex<Option<SpotifyApiManager>>>,
    delegate: Arc<Mutex<Option<Box<dyn PottyDelegate>>>>,
}

#[uniffi::export]
impl PottyClient {
    /// Creates a new PottyClient instance
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let _ = env_logger::builder().try_init();
        
        // Create a persistent Tokio runtime for all async operations
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        
        Arc::new(Self {
            runtime: Arc::new(runtime),
            session: Arc::new(Mutex::new(None)),
            player: Arc::new(Mutex::new(None)),
            spotify_api: Arc::new(Mutex::new(None)),
            delegate: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Sets the delegate for player events
    pub fn set_delegate(&self, delegate: Box<dyn PottyDelegate>) {
        *self.delegate.lock().unwrap() = Some(delegate);
    }
    
    /// Authenticates with Spotify and initializes the session
    pub fn login(&self) -> Result<String, PottyError> {
        let _guard = self.runtime.enter();
        
        // Authenticate via OAuth
        let oauth_token = AuthManager::authenticate()?;
        
        // Create and connect librespot session
        let session_config = SessionConfig {
            client_id: AuthManager::client_id().to_string(),
            ..Default::default()
        };
        
        let credentials = Credentials::with_access_token(&oauth_token.access_token);
        let session = Session::new(session_config, None);
        
        self.runtime.block_on(async {
            session
                .connect(credentials, true)
                .await
                .map_err(|e| PottyError::AuthenticationFailed(e.to_string()))
        })?;
        
        // Initialize player
        let player = PlayerManager::new(
            session.clone(),
            self.runtime.clone(),
            self.delegate.clone(),
        )?;
        
        // Initialize Spotify Web API
        let spotify_api = SpotifyApiManager::new(
            oauth_token.access_token.clone(),
            oauth_token.refresh_token.clone(),
            AuthManager::scopes(),
            self.runtime.clone(),
        );
        
        // Store instances
        *self.session.lock().unwrap() = Some(session);
        *self.player.lock().unwrap() = Some(player);
        *self.spotify_api.lock().unwrap() = Some(spotify_api);
        
        Ok(oauth_token.access_token)
    }
    
    // ========== Playback Controls ==========
    
    /// Plays a track by Spotify URI
    pub fn play_uri(&self, uri: String) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.play_uri(&uri)
    }
    
    /// Pauses playback
    pub fn pause(&self) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.pause();
        Ok(())
    }
    
    /// Resumes playback
    pub fn play(&self) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.play();
        Ok(())
    }
    
    /// Stops playback
    pub fn stop(&self) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.stop();
        Ok(())
    }
    
    /// Seeks to a position in the current track
    pub fn seek(&self, position_ms: u32) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.seek(position_ms);
        Ok(())
    }
    
    /// Sets the playback volume (0-65535, where 65535 is 100%)
    pub fn set_volume(&self, volume: u16) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.set_volume(volume);
        Ok(())
    }
    
    /// Gets the current playback volume (0-65535, where 65535 is 100%)
    pub fn get_volume(&self) -> Result<u16, PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        Ok(player.get_volume())
    }
    
    // ========== Spotify Web API ==========
    
    /// Searches for tracks on Spotify with pagination support
    /// 
    /// # Arguments
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return (max 50)
    /// * `offset` - The offset for pagination (0-based)
    pub fn search(&self, query: String, limit: u32, offset: u32) -> Result<Vec<PottyTrack>, PottyError> {
        let _guard = self.runtime.enter();
        
        let api_guard = self.spotify_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        api.search(&query, limit, offset)
    }
    
    /// Advanced search with filters and type selection
    /// 
    /// # Arguments
    /// * `query` - The base search query string
    /// * `search_type` - Type of results to return (All, Track, Album, Artist)
    /// * `artist_filter` - Optional artist name filter
    /// * `album_filter` - Optional album name filter
    /// * `track_filter` - Optional track name filter
    /// * `limit` - Maximum number of results per type (max 50)
    /// * `offset` - The offset for pagination (0-based)
    pub fn advanced_search(
        &self,
        query: String,
        search_type: PottySearchType,
        artist_filter: Option<String>,
        album_filter: Option<String>,
        track_filter: Option<String>,
        limit: u32,
        offset: u32,
    ) -> Result<PottySearchResults, PottyError> {
        let _guard = self.runtime.enter();
        
        let api_guard = self.spotify_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        api.advanced_search(
            &query,
            &search_type,
            artist_filter.as_deref(),
            album_filter.as_deref(),
            track_filter.as_deref(),
            limit,
            offset,
        )
    }
    
    /// Gets the user's liked/saved songs
    pub fn get_liked_songs(&self) -> Result<Vec<PottyTrack>, PottyError> {
        let _guard = self.runtime.enter();
        
        let api_guard = self.spotify_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        api.get_liked_songs()
    }
    
    /// Gets the user's playlists
    pub fn get_user_playlists(&self) -> Result<Vec<PottyPlaylist>, PottyError> {
        let _guard = self.runtime.enter();
        
        let api_guard = self.spotify_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        api.get_user_playlists()
    }
    
    /// Gets tracks from a specific playlist
    pub fn get_playlist_tracks(&self, playlist_id: String) -> Result<Vec<PottyTrack>, PottyError> {
        let _guard = self.runtime.enter();
        
        let api_guard = self.spotify_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        api.get_playlist_tracks(&playlist_id)
    }
}

// UniFFI scaffolding
uniffi::setup_scaffolding!("potty_bridge");

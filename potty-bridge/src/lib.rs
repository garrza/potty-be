// Module declarations - clean, organized structure
mod domain;
mod managers;
mod error;

// Re-exports for UniFFI - domain types only
pub use domain::*;
pub use error::PottyError;

use managers::{AuthManager, MetadataManager, PlaybackManager, WebApiManager};
use librespot_core::authentication::Credentials;
use librespot_core::{Session, SessionConfig};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// Main client for the Potty Spotify bridge
/// 
/// This is a thin facade that delegates to specialized managers
#[derive(uniffi::Object)]
pub struct PottyClient {
    runtime: Arc<Runtime>,
    session: Arc<Mutex<Option<Session>>>,
    playback: Arc<Mutex<Option<PlaybackManager>>>,
    web_api: Arc<Mutex<Option<WebApiManager>>>,
    metadata: Arc<Mutex<Option<MetadataManager>>>,
    delegate: Arc<Mutex<Option<Box<dyn PlayerDelegate>>>>,
}

#[uniffi::export]
impl PottyClient {
    /// Creates a new PottyClient instance
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let _ = env_logger::builder().try_init();
        
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        
        Arc::new(Self {
            runtime: Arc::new(runtime),
            session: Arc::new(Mutex::new(None)),
            playback: Arc::new(Mutex::new(None)),
            web_api: Arc::new(Mutex::new(None)),
            metadata: Arc::new(Mutex::new(None)),
            delegate: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Sets the delegate for player events
    pub fn set_delegate(&self, delegate: Box<dyn PlayerDelegate>) {
        *self.delegate.lock().unwrap() = Some(delegate);
    }
    
    /// Authenticates with Spotify and initializes the session
    pub fn login(&self) -> Result<String, PottyError> {
        let _guard = self.runtime.enter();
        
        let oauth_token = AuthManager::authenticate()?;
        
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
        
        // Initialize managers
        let playback = PlaybackManager::new(
            session.clone(),
            self.runtime.clone(),
            self.delegate.clone(),
        )?;
        
        let web_api = WebApiManager::new(
            oauth_token.access_token.clone(),
            oauth_token.refresh_token.clone(),
            AuthManager::scopes(),
            self.runtime.clone(),
        );
        
        let metadata = MetadataManager::new(
            session.clone(),
            self.runtime.clone(),
        );
        
        // Store instances
        *self.session.lock().unwrap() = Some(session);
        *self.playback.lock().unwrap() = Some(playback);
        *self.web_api.lock().unwrap() = Some(web_api);
        *self.metadata.lock().unwrap() = Some(metadata);
        
        Ok(oauth_token.access_token)
    }
    
    // ========== Playback Controls ==========
    
    pub fn play_uri(&self, uri: String) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.play_uri(&uri)
    }
    
    pub fn pause(&self) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.pause();
        Ok(())
    }
    
    pub fn play(&self) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.play();
        Ok(())
    }
    
    pub fn stop(&self) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.stop();
        Ok(())
    }
    
    pub fn seek(&self, position_ms: u32) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.seek(position_ms);
        Ok(())
    }
    
    pub fn set_volume(&self, volume: u16) -> Result<(), PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        mgr.set_volume(volume);
        Ok(())
    }
    
    pub fn get_volume(&self) -> Result<u16, PottyError> {
        let guard = self.playback.lock().unwrap();
        let mgr = guard.as_ref().ok_or(PottyError::NotConnected)?;
        Ok(mgr.get_volume())
    }
    
    // ========== Spotify Web API ==========
    
    pub fn search(&self, query: String, limit: u32, offset: u32) -> Result<Vec<Track>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.search(&query, limit, offset)
    }
    
    pub fn advanced_search(
        &self,
        query: String,
        search_type: SearchType,
        artist_filter: Option<String>,
        album_filter: Option<String>,
        track_filter: Option<String>,
        limit: u32,
        offset: u32,
    ) -> Result<SearchResults, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
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
    
    pub fn get_liked_songs(&self, offset: u32, limit: u32) -> Result<Vec<Track>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_liked_songs(offset, limit)
    }
    
    pub fn get_user_playlists(&self) -> Result<Vec<Playlist>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_user_playlists()
    }

    pub fn get_recently_played(&self, limit: u32) -> Result<Vec<Track>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_recently_played(limit)
    }

    pub fn get_new_releases(&self, limit: u32, offset: u32) -> Result<Vec<Album>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_new_releases(limit, offset)
    }

    pub fn get_featured_playlists(&self, limit: u32, offset: u32) -> Result<Vec<Playlist>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_featured_playlists(limit, offset)
    }
    
    pub fn get_playlist_tracks(&self, playlist_id: String) -> Result<Vec<Track>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_playlist_tracks(&playlist_id)
    }
    
    pub fn get_user_saved_albums(&self, offset: u32, limit: u32) -> Result<Vec<Album>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_user_saved_albums(offset, limit)
    }
    
    pub fn get_user_followed_artists(&self) -> Result<Vec<Artist>, PottyError> {
        let _guard = self.runtime.enter();
        let api_guard = self.web_api.lock().unwrap();
        let api = api_guard.as_ref().ok_or(PottyError::NotConnected)?;
        api.get_user_followed_artists()
    }
    
    // ========== Librespot Metadata API ==========
    
    pub fn get_artist_metadata(&self, artist_uri: String) -> Result<ArtistMetadata, PottyError> {
        let _guard = self.runtime.enter();
        let metadata_guard = self.metadata.lock().unwrap();
        let metadata = metadata_guard.as_ref().ok_or(PottyError::NotConnected)?;
        metadata.get_artist_metadata(&artist_uri)
    }
    
    pub fn get_album_metadata(&self, album_uri: String) -> Result<AlbumMetadata, PottyError> {
        let _guard = self.runtime.enter();
        let metadata_guard = self.metadata.lock().unwrap();
        let metadata = metadata_guard.as_ref().ok_or(PottyError::NotConnected)?;
        metadata.get_album_metadata(&album_uri)
    }
}

// UniFFI scaffolding
uniffi::setup_scaffolding!("potty_bridge");

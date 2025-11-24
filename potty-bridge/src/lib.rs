use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use librespot_oauth::OAuthClientBuilder;
use librespot_core::{Session, SessionConfig, SpotifyUri};
use librespot_core::authentication::Credentials;
use librespot_playback::player::{Player, PlayerEvent};
use librespot_playback::config::{PlayerConfig, AudioFormat};
use librespot_playback::mixer::{self, MixerConfig};
use librespot_playback::audio_backend;
use uniffi;
use thiserror::Error;
use log::{info, error};
use rspotify::{AuthCodeSpotify, Token};
use rspotify::prelude::*;
use rspotify::model::{SearchType, SearchResult};
use chrono::Duration as ChronoDuration;

#[derive(Debug, Error, uniffi::Error)]
pub enum PottyError {
    #[error("Generic error: {0}")]
    Generic(String),
    #[error("Not connected")]
    NotConnected,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PottyTrack {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub uri: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum PottyPlayerEvent {
    Playing { track_id: String },
    Paused { track_id: String },
    Stopped { track_id: String },
    EndOfTrack { track_id: String },
    VolumeChanged { volume: u16 },
    Unknown,
}

#[uniffi::export(callback_interface)]
pub trait PottyDelegate: Send + Sync {
    fn on_player_event(&self, event: PottyPlayerEvent);
}

#[derive(uniffi::Object)]
pub struct PottyClient {
    session: Arc<Mutex<Option<Session>>>,
    player: Arc<Mutex<Option<Arc<Player>>>>,
    spotify_api: Arc<Mutex<Option<AuthCodeSpotify>>>,
    delegate: Arc<Mutex<Option<Box<dyn PottyDelegate>>>>,
}

#[uniffi::export]
impl PottyClient {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let _ = env_logger::builder().try_init();
        
        Arc::new(Self {
            session: Arc::new(Mutex::new(None)),
            player: Arc::new(Mutex::new(None)),
            spotify_api: Arc::new(Mutex::new(None)),
            delegate: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_delegate(&self, delegate: Box<dyn PottyDelegate>) {
        *self.delegate.lock().unwrap() = Some(delegate);
    }

    pub async fn login(&self) -> Result<String, PottyError> {
        let client_id = "65b708073fc0480ea92a077233ca87bd"; 
        
        let port = match TcpListener::bind("127.0.0.1:0") {
            Ok(socket) => socket.local_addr().map(|addr| addr.port()).unwrap_or(8888),
            Err(_) => 8888,
        };
        
        let redirect_uri = format!("http://127.0.0.1:{}/login", port);

        let scopes = vec![
            "playlist-modify-private",
            "playlist-read-private",
            "streaming",
            "user-read-email",
            "user-read-private",
            "user-modify-playback-state",
        ];

        let client = OAuthClientBuilder::new(client_id, &redirect_uri, scopes.clone())
            .open_in_browser()
            .build()
            .map_err(|e| PottyError::Generic(e.to_string()))?;

        let oauth_token = client.get_access_token_async().await.map_err(|e| PottyError::Generic(e.to_string()))?;

        let session_config = SessionConfig {
            client_id: client_id.to_string(),
            ..Default::default()
        };

        let credentials = Credentials::with_access_token(&oauth_token.access_token);
        
        let session = Session::new(session_config, None);
        session.connect(credentials, true).await.map_err(|e| PottyError::Generic(e.to_string()))?;
        
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();
        let backend = audio_backend::find(None).unwrap();
        
        let mixer_factory_opt = mixer::find(Some("softvol"));
        let factory = mixer_factory_opt.ok_or(PottyError::Generic("No mixer found".to_string()))?;
        let mixer = factory(MixerConfig::default());

        let mixer = match mixer {
             Ok(m) => m,
             Err(_) => return Err(PottyError::Generic("Failed to create mixer".to_string())),
        };

        let player = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || backend(None, audio_format),
        );

        // Start Event Loop
        let mut event_channel = player.get_player_event_channel();
        
        // We need to access the delegate in the loop.
        // But we only have Box<dyn PottyDelegate>. Box is unique.
        // We can't clone Box<dyn Trait>.
        // This implies we cannot share the delegate easily if it's a Box.
        // However, we CAN clone the foreign callback handle if UniFFI allows it?
        // Usually callback interfaces are passed as `Box<dyn Trait>`.
        
        // If I need to share it (store it AND use it in thread), I probably need `Arc`.
        // If UniFFI doesn't support `Arc` for callback interfaces, I'm stuck.
        // BUT, `ncspot` doesn't use UniFFI.
        
        // Let's assume UniFFI supports `Box` and I can keep it in a Mutex.
        // To use it in the loop, I have to lock the Mutex every time.
        // `delegate_store` is `Arc<Mutex<Option<Box<dyn PottyDelegate>>>>`.
        let delegate_store = self.delegate.clone();
        
        tokio::spawn(async move {
            while let Some(event) = event_channel.recv().await {
                // We lock and use the delegate.
                // We can't clone the delegate out of the lock if it's a Box.
                // So we must keep the lock while using it.
                {
                    let guard = delegate_store.lock().unwrap();
                    if let Some(delegate) = guard.as_ref() {
                        let potty_event = match event {
                            PlayerEvent::Playing { track_id, .. } => PottyPlayerEvent::Playing { track_id: track_id.to_uri() },
                            PlayerEvent::Paused { track_id, .. } => PottyPlayerEvent::Paused { track_id: track_id.to_uri() },
                            PlayerEvent::Stopped { track_id, .. } => PottyPlayerEvent::Stopped { track_id: track_id.to_uri() },
                            PlayerEvent::EndOfTrack { track_id, .. } => PottyPlayerEvent::EndOfTrack { track_id: track_id.to_uri() },
                            PlayerEvent::VolumeChanged { volume } => PottyPlayerEvent::VolumeChanged { volume },
                            _ => PottyPlayerEvent::Unknown,
                        };
                        delegate.on_player_event(potty_event);
                    }
                }
            }
        });

        *self.session.lock().unwrap() = Some(session);
        *self.player.lock().unwrap() = Some(player);

        // Initialize rspotify
        let token = Token {
            access_token: oauth_token.access_token.clone(),
            refresh_token: Some(oauth_token.refresh_token.clone()),
            expires_in: ChronoDuration::seconds(3600),
            expires_at: Some(chrono::Utc::now() + ChronoDuration::seconds(3600)),
            scopes: std::collections::HashSet::from_iter(scopes.into_iter().map(|s| s.to_string())),
        };

        let spotify = AuthCodeSpotify::from_token(token);
        *self.spotify_api.lock().unwrap() = Some(spotify);

        Ok(oauth_token.access_token)
    }

    pub fn play_uri(&self, uri: String) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        
        let spotify_uri = SpotifyUri::from_uri(&uri).map_err(|e| PottyError::Generic(e.to_string()))?;
        player.load(spotify_uri, true, 0);
        Ok(())
    }

    pub fn pause(&self) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.pause();
        Ok(())
    }
    
    pub fn play(&self) -> Result<(), PottyError> {
        let player_guard = self.player.lock().unwrap();
        let player = player_guard.as_ref().ok_or(PottyError::NotConnected)?;
        player.play();
        Ok(())
    }

    pub async fn search(&self, query: String) -> Result<Vec<PottyTrack>, PottyError> {
        // Get clone of API to use in async block
        let api = {
            let api_guard = self.spotify_api.lock().unwrap();
            api_guard.clone().ok_or(PottyError::NotConnected)?
        };

        let result = api.search(&query, SearchType::Track, None, None, Some(20), None).await
            .map_err(|e| PottyError::Generic(e.to_string()))?;

        match result {
            SearchResult::Tracks(page) => {
                let tracks = page.items.into_iter().map(|t| {
                    PottyTrack {
                        id: t.id.map(|id| id.to_string()).unwrap_or_default(),
                        name: t.name,
                        artist: t.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
                        album: t.album.name,
                        uri: t.external_urls.get("spotify").cloned().unwrap_or_default(),
                        duration_ms: t.duration.num_milliseconds() as u32,
                    }
                }).collect();
                Ok(tracks)
            },
            _ => Ok(vec![])
        }
    }
}

uniffi::setup_scaffolding!("potty_bridge");

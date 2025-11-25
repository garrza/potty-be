use crate::error::PottyError;
use crate::types::{PottyDelegate, PottyPlayerEvent};
use librespot_core::{Session, SpotifyUri};
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{self, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// Volume constant: 100% in Spotify's 16-bit scale
const MAX_VOLUME: u16 = 65535;

/// Manages Spotify playback using librespot
pub struct PlayerManager {
    player: Arc<Player>,
}

impl PlayerManager {
    /// Creates a new player manager with the given session
    pub fn new(
        session: Session,
        runtime: Arc<Runtime>,
        delegate: Arc<Mutex<Option<Box<dyn PottyDelegate>>>>,
    ) -> Result<Self, PottyError> {
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();
        
        let backend = audio_backend::find(None)
            .ok_or_else(|| PottyError::PlaybackError("No audio backend found".to_string()))?;
        
        let mixer = Self::create_mixer()?;
        
        // Set volume to maximum (users control via macOS volume)
        mixer.set_volume(MAX_VOLUME);
        
        // Player::new already returns Arc<Player>
        let player = Player::new(
            player_config,
            session,
            mixer.get_soft_volume(),
            move || backend(None, audio_format),
        );
        
        // Spawn event loop on the runtime
        Self::spawn_event_loop(Arc::clone(&player), runtime, delegate);
        
        Ok(Self { player })
    }
    
    /// Creates and configures the audio mixer
    fn create_mixer() -> Result<Arc<dyn librespot_playback::mixer::Mixer>, PottyError> {
        let mixer_factory = mixer::find(Some("softvol"))
            .ok_or_else(|| PottyError::PlaybackError("No mixer found".to_string()))?;
        
        mixer_factory(MixerConfig::default())
            .map_err(|_| PottyError::PlaybackError("Failed to create mixer".to_string()))
    }
    
    /// Spawns the player event loop
    fn spawn_event_loop(
        player: Arc<Player>,
        runtime: Arc<Runtime>,
        delegate: Arc<Mutex<Option<Box<dyn PottyDelegate>>>>,
    ) {
        let mut event_channel = player.get_player_event_channel();
        
        runtime.spawn(async move {
            while let Some(event) = event_channel.recv().await {
                if let Some(delegate) = delegate.lock().unwrap().as_ref() {
                    let potty_event = Self::convert_player_event(event);
                    delegate.on_player_event(potty_event);
                }
            }
        });
    }
    
    /// Converts librespot PlayerEvent to PottyPlayerEvent
    fn convert_player_event(event: PlayerEvent) -> PottyPlayerEvent {
        match event {
            PlayerEvent::Playing { track_id, .. } => {
                PottyPlayerEvent::Playing { track_id: track_id.to_uri() }
            }
            PlayerEvent::Paused { track_id, .. } => {
                PottyPlayerEvent::Paused { track_id: track_id.to_uri() }
            }
            PlayerEvent::Stopped { track_id, .. } => {
                PottyPlayerEvent::Stopped { track_id: track_id.to_uri() }
            }
            PlayerEvent::EndOfTrack { track_id, .. } => {
                PottyPlayerEvent::EndOfTrack { track_id: track_id.to_uri() }
            }
            PlayerEvent::VolumeChanged { volume } => {
                PottyPlayerEvent::VolumeChanged { volume }
            }
            _ => PottyPlayerEvent::Unknown,
        }
    }
    
    /// Plays a track by URI
    pub fn play_uri(&self, uri: &str) -> Result<(), PottyError> {
        let spotify_uri = SpotifyUri::from_uri(uri)
            .map_err(|e| PottyError::PlaybackError(e.to_string()))?;
        
        self.player.load(spotify_uri, true, 0);
        Ok(())
    }
    
    /// Pauses playback
    pub fn pause(&self) {
        self.player.pause();
    }
    
    /// Resumes playback
    pub fn play(&self) {
        self.player.play();
    }
    
    /// Stops playback
    pub fn stop(&self) {
        self.player.stop();
    }
    
    /// Seeks to a position in milliseconds
    pub fn seek(&self, position_ms: u32) {
        self.player.seek(position_ms);
    }
}

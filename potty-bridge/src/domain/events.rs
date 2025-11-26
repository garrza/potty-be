/// Player events that can be sent to the Swift UI
#[derive(Debug, Clone, uniffi::Enum)]
pub enum PlayerEvent {
    Playing { track_id: String },
    Paused { track_id: String },
    Stopped { track_id: String },
    EndOfTrack { track_id: String },
    VolumeChanged { volume: u16 },
    Unknown,
}

/// Callback interface for player events
#[uniffi::export(callback_interface)]
pub trait PlayerDelegate: Send + Sync {
    fn on_player_event(&self, event: PlayerEvent);
}


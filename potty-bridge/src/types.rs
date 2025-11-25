/// Represents a Spotify track with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct PottyTrack {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub uri: String,
    pub duration_ms: u32,
}

/// Represents a Spotify playlist with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct PottyPlaylist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub uri: String,
    pub track_count: u32,
    pub image_url: String,
}

/// Represents a Spotify album with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct PottyAlbum {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub uri: String,
    pub release_date: String,
    pub total_tracks: u32,
    pub image_url: String,
}

/// Represents a Spotify artist with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct PottyArtist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub genres: Vec<String>,
    pub image_url: String,
    pub followers: u32,
}

/// Search filter type
#[derive(Debug, Clone, uniffi::Enum)]
pub enum PottySearchType {
    All,
    Track,
    Album,
    Artist,
}

/// Unified search result
#[derive(Debug, Clone, uniffi::Record)]
pub struct PottySearchResults {
    pub tracks: Vec<PottyTrack>,
    pub albums: Vec<PottyAlbum>,
    pub artists: Vec<PottyArtist>,
}

/// Player events that can be sent to the Swift UI
#[derive(Debug, Clone, uniffi::Enum)]
pub enum PottyPlayerEvent {
    Playing { track_id: String },
    Paused { track_id: String },
    Stopped { track_id: String },
    EndOfTrack { track_id: String },
    VolumeChanged { volume: u16 },
    Unknown,
}

/// Callback interface for player events
#[uniffi::export(callback_interface)]
pub trait PottyDelegate: Send + Sync {
    fn on_player_event(&self, event: PottyPlayerEvent);
}


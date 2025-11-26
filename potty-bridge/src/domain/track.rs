/// Represents a Spotify track with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub uri: String,
    pub duration_ms: u32,
    pub image_url: String,
}


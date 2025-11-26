/// Represents a Spotify playlist with metadata
#[derive(Debug, Clone, uniffi::Record)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub uri: String,
    pub track_count: u32,
    pub image_url: String,
}


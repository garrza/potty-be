use super::artist::ArtistSimple;

/// Represents a Spotify album with basic metadata (from Web API)
#[derive(Debug, Clone, uniffi::Record)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub uri: String,
    pub release_date: String,
    pub total_tracks: u32,
    pub image_url: String,
}

/// Represents extended album metadata (from Librespot)
#[derive(Debug, Clone, uniffi::Record)]
pub struct AlbumMetadata {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub artists: Vec<ArtistSimple>,
    pub album_type: String,
    pub label: String,
    pub release_date: String,
    pub popularity: i32,
    pub image_url: String,
    pub total_tracks: u32,
    pub tracks: Vec<String>, // Track URIs
    pub copyrights: Vec<String>,
}


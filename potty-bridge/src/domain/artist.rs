/// Represents a Spotify artist with basic metadata (from Web API)
#[derive(Debug, Clone, uniffi::Record)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub genres: Vec<String>,
    pub image_url: String,
    pub followers: u32,
}

/// Simple artist info for nested structures
#[derive(Debug, Clone, uniffi::Record)]
pub struct ArtistSimple {
    pub id: String,
    pub name: String,
    pub uri: String,
}

/// Represents extended artist metadata (from Librespot)
#[derive(Debug, Clone, uniffi::Record)]
pub struct ArtistMetadata {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub popularity: i32,
    pub genres: Vec<String>,
    pub image_url: String,
    pub followers: u32,
    
    // Top tracks
    pub top_tracks: Vec<String>, // Track URIs
    
    // Discography
    pub albums: Vec<String>,        // Album URIs (current releases)
    pub singles: Vec<String>,       // Single URIs (current releases)
    pub compilations: Vec<String>,  // Compilation URIs (current releases)
    pub appears_on: Vec<String>,    // Album URIs where artist appears
    
    // Biography
    pub biography: String,
    pub biography_portraits: Vec<String>, // Image URLs
    
    // Activity period
    pub activity_period: String, // "1990-2005" or "2010s" etc.
    
    // Related artists
    pub related_artists: Vec<ArtistSimple>,
}


use crate::error::PottyError;
use crate::domain::{AlbumMetadata, ArtistMetadata, ArtistSimple};
use librespot_core::{Session, SpotifyUri};
use librespot_metadata::{Album, Artist, Metadata};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Manages Librespot metadata operations
pub struct MetadataManager {
    session: Session,
    runtime: Arc<Runtime>,
}

impl MetadataManager {
    /// Creates a new metadata manager
    pub fn new(session: Session, runtime: Arc<Runtime>) -> Self {
        Self { session, runtime }
    }
    
    /// Fetches full artist metadata from Librespot
    pub fn get_artist_metadata(&self, artist_uri: &str) -> Result<ArtistMetadata, PottyError> {
        let session = self.session.clone();
        let uri = SpotifyUri::from_uri(artist_uri)
            .map_err(|e| PottyError::Generic(format!("Invalid artist URI: {}", e)))?;
        
        self.runtime.block_on(async move {
            let artist = Artist::get(&session, &uri)
                .await
                .map_err(|e| PottyError::Generic(format!("Failed to fetch artist metadata: {}", e)))?;
            
            Ok(Self::convert_artist(artist))
        })
    }
    
    /// Fetches full album metadata from Librespot
    pub fn get_album_metadata(&self, album_uri: &str) -> Result<AlbumMetadata, PottyError> {
        let session = self.session.clone();
        let uri = SpotifyUri::from_uri(album_uri)
            .map_err(|e| PottyError::Generic(format!("Invalid album URI: {}", e)))?;
        
        self.runtime.block_on(async move {
            let album = Album::get(&session, &uri)
                .await
                .map_err(|e| PottyError::Generic(format!("Failed to fetch album metadata: {}", e)))?;
            
            Ok(Self::convert_album(album))
        })
    }
    
    /// Converts Librespot Artist to ArtistMetadata
    fn convert_artist(artist: Artist) -> ArtistMetadata {
        // Extract top tracks (use global or first available country)
        let top_tracks = artist
            .top_tracks
            .for_country("")
            .iter()
            .map(|uri| uri.to_uri())
            .collect();
        
        // Get current album releases (no duplicates)
        let albums: Vec<String> = artist.albums_current()
            .map(|uri| uri.to_uri())
            .collect();
        
        let singles: Vec<String> = artist.singles_current()
            .map(|uri| uri.to_uri())
            .collect();
        
        let compilations: Vec<String> = artist.compilations_current()
            .map(|uri| uri.to_uri())
            .collect();
        
        let appears_on: Vec<String> = artist.appears_on_albums_current()
            .map(|uri| uri.to_uri())
            .collect();
        
        // Extract biography
        let (biography, biography_portraits) = artist.biographies.first()
            .map(|bio| {
                let portraits = bio.portraits
                    .iter()
                    .map(|img| Self::image_url_from_id(&img.id))
                    .collect();
                (bio.text.clone(), portraits)
            })
            .unwrap_or_default();
        
        // Format activity period
        let activity_period = Self::format_activity_period(&artist.activity_periods);
        
        // Extract image URL
        let image_url = artist.portraits
            .first()
            .map(|img| Self::image_url_from_id(&img.id))
            .unwrap_or_default();
        
        // Convert related artists
        let related_artists = artist.related
            .iter()
            .map(|a| ArtistSimple {
                id: Self::extract_id(&a.id),
                name: a.name.clone(),
                uri: a.id.to_uri(),
            })
            .collect();
        
        ArtistMetadata {
            id: Self::extract_id(&artist.id),
            name: artist.name,
            uri: artist.id.to_uri(),
            popularity: artist.popularity,
            genres: vec![], // Librespot metadata doesn't include genres
            image_url,
            followers: 0, // Not available in Librespot metadata
            top_tracks,
            albums,
            singles,
            compilations,
            appears_on,
            biography,
            biography_portraits,
            activity_period,
            related_artists,
        }
    }
    
    /// Converts Librespot Album to AlbumMetadata
    fn convert_album(album: Album) -> AlbumMetadata {
        // Extract tracks
        let tracks: Vec<String> = album.tracks()
            .map(|uri| uri.to_uri())
            .collect();
        
        // Extract artists
        let artists = album.artists
            .iter()
            .map(|a| ArtistSimple {
                id: Self::extract_id(&a.id),
                name: a.name.clone(),
                uri: a.id.to_uri(),
            })
            .collect();
        
        // Extract image URL
        let image_url = album.covers
            .first()
            .map(|img| Self::image_url_from_id(&img.id))
            .unwrap_or_default();
        
        // Extract copyrights
        let copyrights = album.copyrights
            .iter()
            .map(|c| c.text.clone())
            .collect();
        
        AlbumMetadata {
            id: Self::extract_id(&album.id),
            name: album.name,
            uri: album.id.to_uri(),
            artists,
            album_type: format!("{:?}", album.album_type),
            label: album.label,
            release_date: format!("{:?}", album.date),
            popularity: album.popularity,
            image_url,
            total_tracks: tracks.len() as u32,
            tracks,
            copyrights,
        }
    }
    
    /// Converts Spotify image file ID to CDN URL
    fn image_url_from_id(file_id: &librespot_core::FileId) -> String {
        format!("https://i.scdn.co/image/{}", file_id.to_base16())
    }
    
    /// Formats activity periods into a readable string
    fn format_activity_period(periods: &librespot_metadata::artist::ActivityPeriods) -> String {
        use librespot_metadata::artist::ActivityPeriod;
        
        periods.first()
            .map(|period| match period {
                ActivityPeriod::Timespan { start_year, end_year } => {
                    if let Some(end) = end_year {
                        format!("{}-{}", start_year, end)
                    } else {
                        format!("{}-PRESENT", start_year)
                    }
                }
                ActivityPeriod::Decade(decade) => format!("{}s", decade),
            })
            .unwrap_or_else(|| String::from("UNKNOWN"))
    }
    
    /// Extracts the base62 ID string from a SpotifyUri
    fn extract_id(uri: &SpotifyUri) -> String {
        match uri {
            SpotifyUri::Artist { id } => id.to_base62(),
            SpotifyUri::Album { id } => id.to_base62(),
            SpotifyUri::Track { id } => id.to_base62(),
            SpotifyUri::Playlist { id, .. } => id.to_base62(),
            SpotifyUri::Show { id } => id.to_base62(),
            SpotifyUri::Episode { id } => id.to_base62(),
            _ => String::new(),
        }
    }
}


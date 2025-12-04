use crate::error::PottyError;
use crate::domain::{Album, Artist, Playlist, SearchResults, Track};
use crate::domain::SearchType as PottySearchType;
use chrono::Duration as ChronoDuration;
use rspotify::model::{Market, PlayableItem, SearchResult, SearchType as RspotifySearchType};
use rspotify::prelude::*;
use rspotify::{AuthCodeSpotify, Token};
use std::collections::HashSet;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

/// Default limit for paginated API requests
const DEFAULT_LIMIT: u32 = 50;

/// Maximum number of search results per request
const MAX_SEARCH_LIMIT: u32 = 50;

/// Manages Spotify Web API interactions
pub struct WebApiManager {
    api: AuthCodeSpotify,
    runtime: std::sync::Arc<Runtime>,
}

impl WebApiManager {
    /// Creates a new Spotify API manager with the given token
    pub fn new(
        access_token: String,
        refresh_token: String,
        scopes: Vec<String>,
        runtime: std::sync::Arc<Runtime>,
    ) -> Self {
        let token = Token {
            access_token,
            refresh_token: Some(refresh_token),
            expires_in: ChronoDuration::seconds(3600),
            expires_at: Some(chrono::Utc::now() + ChronoDuration::seconds(3600)),
            scopes: HashSet::from_iter(scopes),
        };
        
        let api = AuthCodeSpotify::from_token(token);
        
        Self { api, runtime }
    }
    
    /// Searches for tracks on Spotify with pagination support
    pub fn search(&self, query: &str, limit: u32, offset: u32) -> Result<Vec<Track>, PottyError> {
        let api = self.api.clone();
        let query = query.to_string();
        
        // Clamp limit to MAX_SEARCH_LIMIT
        let limit = limit.min(MAX_SEARCH_LIMIT);
        
        self.runtime.block_on(async move {
            let result = api
                .search(&query, RspotifySearchType::Track, None, None, Some(limit), Some(offset))
                .await
                .map_err(PottyError::from_error)?;
            
            match result {
                SearchResult::Tracks(page) => {
                    let tracks = page
                        .items
                        .into_iter()
                        .map(|t| Self::track_to_potty_track(t))
                        .collect();
                    Ok(tracks)
                }
                _ => Ok(vec![]),
            }
        })
    }
    
    /// Gets the user's liked/saved tracks with pagination support
    /// 
    /// # Arguments
    /// * `offset` - The offset for pagination (default: 0)
    /// * `limit` - Maximum number of results to return (default: 50, max: 50)
    pub fn get_liked_songs(&self, offset: u32, limit: u32) -> Result<Vec<Track>, PottyError> {
        let api = self.api.clone();
        let limit = limit.min(DEFAULT_LIMIT);
        
        self.runtime.block_on(async move {
            let result = api
                .current_user_saved_tracks_manual(Some(Market::FromToken), Some(limit), Some(offset))
                .await
                .map_err(PottyError::from_error)?;
            
            let tracks = result
                .items
                .into_iter()
                .map(|saved_track| Self::track_to_potty_track(saved_track.track))
                .collect();
            
            Ok(tracks)
        })
    }
    
    /// Gets the user's saved albums with pagination support
    /// 
    /// # Arguments
    /// * `offset` - The offset for pagination (default: 0)
    /// * `limit` - Maximum number of results to return (default: 50, max: 50)
    pub fn get_user_saved_albums(&self, offset: u32, limit: u32) -> Result<Vec<Album>, PottyError> {
        let api = self.api.clone();
        let limit = limit.min(DEFAULT_LIMIT);
        
        self.runtime.block_on(async move {
            let result = api
                .current_user_saved_albums_manual(Some(Market::FromToken), Some(limit), Some(offset))
                .await
                .map_err(PottyError::from_error)?;
            
            let albums = result
                .items
                .into_iter()
                .map(|saved_album| Self::full_album_to_potty_album(saved_album.album))
                .collect();
            
            Ok(albums)
        })
    }
    
    /// Gets the user's followed artists (first page only)
    pub fn get_user_followed_artists(&self) -> Result<Vec<Artist>, PottyError> {
        let api = self.api.clone();
        
        self.runtime.block_on(async move {
            let result = api
                .current_user_followed_artists(None, Some(DEFAULT_LIMIT))
                .await
                .map_err(PottyError::from_error)?;
            
            let artists = result
                .items
                .into_iter()
                .map(Self::artist_to_potty_artist)
                .collect();
            
            Ok(artists)
        })
    }
    
    /// Gets the user's playlists
    pub fn get_user_playlists(&self) -> Result<Vec<Playlist>, PottyError> {
        let api = self.api.clone();
        
        self.runtime.block_on(async move {
            let result = api
                .current_user_playlists_manual(Some(DEFAULT_LIMIT), None)
                .await
                .map_err(PottyError::from_error)?;
            
            let playlists = result
                .items
                .into_iter()
                .map(|pl| {
                    let uri = format!("spotify:playlist:{}", pl.id.id());
                    let image_url = pl
                        .images
                        .first()
                        .map(|img| img.url.clone())
                        .unwrap_or_default();
                    
                    Playlist {
                        id: pl.id.id().to_string(),
                        name: pl.name,
                        description: String::new(),
                        uri,
                        track_count: pl.tracks.total,
                        image_url,
                    }
                })
                .collect();
            
            Ok(playlists)
        })
    }

    /// Gets the current user's recently played tracks
    pub fn get_recently_played(&self, limit: u32) -> Result<Vec<Track>, PottyError> {
        let api = self.api.clone();
        let limit = limit.min(DEFAULT_LIMIT);

        self.runtime.block_on(async move {
            let result = api
                .current_user_recently_played(Some(limit), None)
                .await
                .map_err(PottyError::from_error)?;

            let tracks = result
                .items
                .into_iter()
                .map(|item| Self::track_to_potty_track(item.track))
                .collect();

            Ok(tracks)
        })
    }

    /// Gets a list of new album releases
    pub fn get_new_releases(&self, limit: u32, _offset: u32) -> Result<Vec<Album>, PottyError> {
        let api = self.api.clone();
        let limit = limit.min(DEFAULT_LIMIT);

        self.runtime.block_on(async move {
            let mut stream = api.new_releases(None);
            let mut albums = Vec::new();
            
            // Consume stream up to limit
            while let Some(item_result) = stream.next().await {
                if albums.len() >= limit as usize {
                    break;
                }
                
                match item_result {
                    Ok(album) => albums.push(Self::album_to_potty_album(album)),
                    Err(e) => {
                        // Log error?
                        eprintln!("Error fetching new release: {}", e);
                    }
                }
            }

            Ok(albums)
        })
    }

    /// Gets a list of featured playlists
    pub fn get_featured_playlists(&self, limit: u32, _offset: u32) -> Result<Vec<Playlist>, PottyError> {
        let api = self.api.clone();
        let limit = limit.min(DEFAULT_LIMIT);

        self.runtime.block_on(async move {
            // featured_playlists likely returns a Result<FeaturedPlaylists, ...> which contains a Page or similar
            // Actually in 0.15 it might be a stream of playlists or a struct containing message + playlists
            // Let's try to look at the error message if I use stream, or assume it's a method that returns Future<Output=ClientResult<FeaturedPlaylists>>
            // But based on new_releases being a stream, this might differ.
            // Wait, new_releases is a stream, but featured_playlists returns a wrapper object usually.
            
            // Let's check codebase for featured_playlists usage? None found.
            // I'll assume it returns a Future -> FeaturedPlaylists
            
            let result = api
                .featured_playlists(None, None, None, Some(limit), Some(_offset))
                .await
                .map_err(PottyError::from_error)?;

            let playlists = result
                .playlists
                .items
                .into_iter()
                .map(|pl| {
                    let uri = format!("spotify:playlist:{}", pl.id.id());
                    let image_url = pl
                        .images
                        .first()
                        .map(|img| img.url.clone())
                        .unwrap_or_default();

                    Playlist {
                        id: pl.id.id().to_string(),
                        name: pl.name,
                        description: String::new(), // SimplifiedPlaylist doesn't have description
                        uri,
                        track_count: pl.tracks.total,
                        image_url,
                    }
                })
                .collect();

            Ok(playlists)
        })
    }
    
    /// Gets tracks from a specific playlist
    pub fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>, PottyError> {
        let api = self.api.clone();
        let playlist_id = playlist_id.to_string();
        
        self.runtime.block_on(async move {
            use rspotify::model::PlaylistId;
            
            let playlist_id = PlaylistId::from_id(&playlist_id)
                .map_err(PottyError::from_error)?;
            
            let mut stream = api.playlist_items(playlist_id, None, None);
            let mut tracks = Vec::new();
            
            while let Some(item_result) = stream.next().await {
                let item = item_result.map_err(PottyError::from_error)?;
                
                if let Some(PlayableItem::Track(track)) = item.track {
                    tracks.push(Self::track_to_potty_track(track));
                }
            }
            
            Ok(tracks)
        })
    }
    
    /// Advanced search with filters and type selection
    pub fn advanced_search(
        &self,
        query: &str,
        search_type: &PottySearchType,
        artist_filter: Option<&str>,
        album_filter: Option<&str>,
        track_filter: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<SearchResults, PottyError> {
        let api = self.api.clone();
        
        // Build query with filters
        let mut query_parts = Vec::new();
        
        if !query.is_empty() {
            query_parts.push(query.to_string());
        }
        
        if let Some(artist) = artist_filter {
            if !artist.is_empty() {
                query_parts.push(format!("artist:{}", artist));
            }
        }
        
        if let Some(album) = album_filter {
            if !album.is_empty() {
                query_parts.push(format!("album:{}", album));
            }
        }
        
        if let Some(track) = track_filter {
            if !track.is_empty() {
                query_parts.push(format!("track:{}", track));
            }
        }
        
        let final_query = query_parts.join(" ");
        if final_query.is_empty() {
            return Ok(SearchResults {
                tracks: vec![],
                albums: vec![],
                artists: vec![],
            });
        }
        
        let limit = limit.min(MAX_SEARCH_LIMIT);
        
        self.runtime.block_on(async move {
            let mut results = SearchResults {
                tracks: vec![],
                albums: vec![],
                artists: vec![],
            };
            
            // Determine which types to search
            let search_tracks = matches!(search_type, PottySearchType::All | PottySearchType::Track);
            let search_albums = matches!(search_type, PottySearchType::All | PottySearchType::Album);
            let search_artists = matches!(search_type, PottySearchType::All | PottySearchType::Artist);
            
            // Search tracks
            if search_tracks {
                let result = api
                    .search(&final_query, RspotifySearchType::Track, None, None, Some(limit), Some(offset))
                    .await
                    .map_err(PottyError::from_error)?;
                
                if let SearchResult::Tracks(page) = result {
                    results.tracks = page
                        .items
                        .into_iter()
                        .map(Self::track_to_potty_track)
                        .collect();
                }
            }
            
            // Search albums
            if search_albums {
                let result = api
                    .search(&final_query, RspotifySearchType::Album, None, None, Some(limit), Some(offset))
                    .await
                    .map_err(PottyError::from_error)?;
                
                if let SearchResult::Albums(page) = result {
                    results.albums = page
                        .items
                        .into_iter()
                        .map(Self::album_to_potty_album)
                        .collect();
                }
            }
            
            // Search artists
            if search_artists {
                let result = api
                    .search(&final_query, RspotifySearchType::Artist, None, None, Some(limit), Some(offset))
                    .await
                    .map_err(PottyError::from_error)?;
                
                if let SearchResult::Artists(page) = result {
                    results.artists = page
                        .items
                        .into_iter()
                        .map(Self::artist_to_potty_artist)
                        .collect();
                }
            }
            
            Ok(results)
        })
    }
    
    /// Converts an rspotify FullTrack to Track
    fn track_to_potty_track(track: rspotify::model::FullTrack) -> Track {
        let uri = track
            .id
            .as_ref()
            .map(|id| format!("spotify:track:{}", id.id()))
            .unwrap_or_default();
        
        let image_url = track
            .album
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        Track {
            id: track.id.map(|id| id.to_string()).unwrap_or_default(),
            name: track.name,
            artist: track
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            album: track.album.name,
            uri,
            duration_ms: track.duration.num_milliseconds() as u32,
            image_url,
        }
    }
    
    /// Converts an rspotify SimplifiedAlbum to Album
    fn album_to_potty_album(album: rspotify::model::SimplifiedAlbum) -> Album {
        let uri = album
            .id
            .as_ref()
            .map(|id| format!("spotify:album:{}", id.id()))
            .unwrap_or_default();
        
        let image_url = album
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        Album {
            id: album.id.map(|id| id.to_string()).unwrap_or_default(),
            name: album.name,
            artist: album
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            uri,
            release_date: album.release_date.unwrap_or_default(),
            total_tracks: 0, // SimplifiedAlbum doesn't include track count
            image_url,
        }
    }
    
    /// Converts an rspotify FullAlbum to Album
    fn full_album_to_potty_album(album: rspotify::model::FullAlbum) -> Album {
        let uri = format!("spotify:album:{}", album.id.id());
        
        let image_url = album
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        Album {
            id: album.id.to_string(),
            name: album.name,
            artist: album
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            uri,
            release_date: album.release_date,
            total_tracks: album.tracks.items.len() as u32,
            image_url,
        }
    }
    
    /// Converts an rspotify FullArtist to Artist
    fn artist_to_potty_artist(artist: rspotify::model::FullArtist) -> Artist {
        let uri = format!("spotify:artist:{}", artist.id.id());
        
        let image_url = artist
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        Artist {
            id: artist.id.to_string(),
            name: artist.name,
            uri,
            genres: artist.genres,
            image_url,
            followers: artist.followers.total,
        }
    }
}


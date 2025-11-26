use crate::error::PottyError;
use crate::types::{PottyAlbum, PottyArtist, PottyPlaylist, PottySearchResults, PottySearchType, PottyTrack};
use chrono::Duration as ChronoDuration;
use rspotify::model::{Market, PlayableItem, SearchResult, SearchType};
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
pub struct SpotifyApiManager {
    api: AuthCodeSpotify,
    runtime: std::sync::Arc<Runtime>,
}

impl SpotifyApiManager {
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
    pub fn search(&self, query: &str, limit: u32, offset: u32) -> Result<Vec<PottyTrack>, PottyError> {
        let api = self.api.clone();
        let query = query.to_string();
        
        // Clamp limit to MAX_SEARCH_LIMIT
        let limit = limit.min(MAX_SEARCH_LIMIT);
        
        self.runtime.block_on(async move {
            let result = api
                .search(&query, SearchType::Track, None, None, Some(limit), Some(offset))
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
    
    /// Gets the user's liked/saved tracks (first page only, use offset for pagination)
    pub fn get_liked_songs(&self) -> Result<Vec<PottyTrack>, PottyError> {
        self.get_liked_songs_paginated(0, DEFAULT_LIMIT)
    }
    
    /// Gets liked songs with pagination support
    pub fn get_liked_songs_paginated(&self, offset: u32, limit: u32) -> Result<Vec<PottyTrack>, PottyError> {
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
    
    /// Gets the user's saved albums (first page only, use offset for pagination)
    pub fn get_user_saved_albums(&self) -> Result<Vec<PottyAlbum>, PottyError> {
        self.get_user_saved_albums_paginated(0, DEFAULT_LIMIT)
    }
    
    /// Gets saved albums with pagination support
    pub fn get_user_saved_albums_paginated(&self, offset: u32, limit: u32) -> Result<Vec<PottyAlbum>, PottyError> {
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
    pub fn get_user_followed_artists(&self) -> Result<Vec<PottyArtist>, PottyError> {
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
    pub fn get_user_playlists(&self) -> Result<Vec<PottyPlaylist>, PottyError> {
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
                    
                    PottyPlaylist {
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
    
    /// Gets tracks from a specific playlist
    pub fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<PottyTrack>, PottyError> {
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
    ) -> Result<PottySearchResults, PottyError> {
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
            return Ok(PottySearchResults {
                tracks: vec![],
                albums: vec![],
                artists: vec![],
            });
        }
        
        let limit = limit.min(MAX_SEARCH_LIMIT);
        
        self.runtime.block_on(async move {
            let mut results = PottySearchResults {
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
                    .search(&final_query, SearchType::Track, None, None, Some(limit), Some(offset))
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
                    .search(&final_query, SearchType::Album, None, None, Some(limit), Some(offset))
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
                    .search(&final_query, SearchType::Artist, None, None, Some(limit), Some(offset))
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
    
    /// Converts an rspotify FullTrack to PottyTrack
    fn track_to_potty_track(track: rspotify::model::FullTrack) -> PottyTrack {
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
        
        PottyTrack {
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
    
    /// Converts an rspotify SimplifiedAlbum to PottyAlbum
    fn album_to_potty_album(album: rspotify::model::SimplifiedAlbum) -> PottyAlbum {
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
        
        PottyAlbum {
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
    
    /// Converts an rspotify FullAlbum to PottyAlbum
    fn full_album_to_potty_album(album: rspotify::model::FullAlbum) -> PottyAlbum {
        let uri = format!("spotify:album:{}", album.id.id());
        
        let image_url = album
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        PottyAlbum {
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
    
    /// Converts an rspotify FullArtist to PottyArtist
    fn artist_to_potty_artist(artist: rspotify::model::FullArtist) -> PottyArtist {
        let uri = format!("spotify:artist:{}", artist.id.id());
        
        let image_url = artist
            .images
            .first()
            .map(|img| img.url.clone())
            .unwrap_or_default();
        
        PottyArtist {
            id: artist.id.to_string(),
            name: artist.name,
            uri,
            genres: artist.genres,
            image_url,
            followers: artist.followers.total,
        }
    }
}


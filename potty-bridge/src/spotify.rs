use crate::error::PottyError;
use crate::types::{PottyPlaylist, PottyTrack};
use chrono::Duration as ChronoDuration;
use rspotify::model::{Market, PlayableItem, SearchResult, SearchType};
use rspotify::prelude::*;
use rspotify::{AuthCodeSpotify, Token};
use std::collections::HashSet;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

/// Default limit for paginated API requests
const DEFAULT_LIMIT: u32 = 50;

/// Default number of search results
const SEARCH_LIMIT: u32 = 20;

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
    
    /// Searches for tracks on Spotify
    pub fn search(&self, query: &str) -> Result<Vec<PottyTrack>, PottyError> {
        let api = self.api.clone();
        let query = query.to_string();
        
        self.runtime.block_on(async move {
            let result = api
                .search(&query, SearchType::Track, None, None, Some(SEARCH_LIMIT), None)
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
    
    /// Gets the user's liked/saved tracks
    pub fn get_liked_songs(&self) -> Result<Vec<PottyTrack>, PottyError> {
        let api = self.api.clone();
        
        self.runtime.block_on(async move {
            let result = api
                .current_user_saved_tracks_manual(Some(Market::FromToken), Some(DEFAULT_LIMIT), None)
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
    
    /// Converts an rspotify FullTrack to PottyTrack
    fn track_to_potty_track(track: rspotify::model::FullTrack) -> PottyTrack {
        let uri = track
            .id
            .as_ref()
            .map(|id| format!("spotify:track:{}", id.id()))
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
        }
    }
}


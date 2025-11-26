use super::{Album, Artist, Track};

/// Search filter type
#[derive(Debug, Clone, uniffi::Enum)]
pub enum SearchType {
    All,
    Track,
    Album,
    Artist,
}

/// Unified search result
#[derive(Debug, Clone, uniffi::Record)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}


// Domain models - business entities and types

pub mod track;
pub mod artist;
pub mod album;
pub mod playlist;
pub mod search;
pub mod events;

// Re-exports for convenience
pub use track::Track;
pub use artist::{Artist, ArtistMetadata, ArtistSimple};
pub use album::{Album, AlbumMetadata};
pub use playlist::Playlist;
pub use search::{SearchType, SearchResults};
pub use events::{PlayerEvent, PlayerDelegate};


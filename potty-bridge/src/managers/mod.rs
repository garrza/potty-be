// Business logic managers - each handles a specific domain

mod auth_manager;
mod playback_manager;
mod metadata_manager;
mod web_api_manager;

pub use auth_manager::AuthManager;
pub use playback_manager::PlaybackManager;
pub use metadata_manager::MetadataManager;
pub use web_api_manager::WebApiManager;


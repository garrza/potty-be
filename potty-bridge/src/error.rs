use thiserror::Error;

/// Core error type for the Potty bridge
#[derive(Debug, Error, uniffi::Error)]
pub enum PottyError {
    #[error("Generic error: {0}")]
    Generic(String),
    
    #[error("Not connected to Spotify")]
    NotConnected,
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Playback error: {0}")]
    PlaybackError(String),
}

impl PottyError {
    /// Convert any error type to PottyError
    pub fn from_error<E: std::error::Error>(err: E) -> Self {
        PottyError::Generic(err.to_string())
    }
}


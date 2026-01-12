use crate::error::PottyError;
use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use std::net::TcpListener;

/// OAuth configuration constants
const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const DEFAULT_PORT: u16 = 8888;

/// OAuth scopes required for the application
const SCOPES: &[&str] = &[
    "playlist-modify-private",
    "playlist-read-private",
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-modify-playback-state",
    "user-library-read",
    "user-follow-read",
    "user-read-recently-played",
];

/// Manages OAuth authentication flow
pub struct AuthManager;

impl AuthManager {
    /// Initiates OAuth flow and returns access token
    pub fn authenticate() -> Result<OAuthToken, PottyError> {
        let port = Self::find_available_port().unwrap_or(DEFAULT_PORT);
        let redirect_uri = format!("http://127.0.0.1:{}/login", port);
        
        let client = Self::build_oauth_client(&redirect_uri)?;
        
        client
            .get_access_token()
            .map_err(|e| PottyError::AuthenticationFailed(e.to_string()))
    }
    
    /// Finds an available port for the OAuth callback
    fn find_available_port() -> Option<u16> {
        TcpListener::bind("127.0.0.1:0")
            .ok()?
            .local_addr()
            .ok()
            .map(|addr| addr.port())
    }
    
    /// Builds OAuth client with configured parameters
    fn build_oauth_client(redirect_uri: &str) -> Result<OAuthClient, PottyError> {
        OAuthClientBuilder::new(CLIENT_ID, redirect_uri, SCOPES.to_vec())
            .open_in_browser()
            .build()
            .map_err(|e| PottyError::AuthenticationFailed(e.to_string()))
    }
    
    /// Returns the client ID
    pub fn client_id() -> &'static str {
        CLIENT_ID
    }
    
    /// Returns the OAuth scopes
    pub fn scopes() -> Vec<String> {
        SCOPES.iter().map(|s| s.to_string()).collect()
    }
}


use std::net::TcpListener;
use std::sync::Arc;
use librespot_oauth::OAuthClientBuilder;
use uniffi;
use thiserror::Error;

#[derive(Debug, Error, uniffi::Error)]
pub enum PottyError {
    #[error("Generic error: {0}")]
    Generic(String),
}

#[derive(uniffi::Object)]
pub struct PottyClient {}

#[uniffi::export]
impl PottyClient {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub async fn login(&self) -> Result<String, PottyError> {
        let client_id = "65b708073fc0480ea92a077233ca87bd"; // ncspot client ID
        
        let port = match TcpListener::bind("127.0.0.1:0") {
            Ok(socket) => socket.local_addr().map(|addr| addr.port()).unwrap_or(8888),
            Err(_) => 8888,
        };
        
        let redirect_uri = format!("http://127.0.0.1:{}/login", port);

        let scopes = vec![
            "playlist-modify",
            "playlist-modify-private",
            "playlist-modify-public",
            "playlist-read",
            "playlist-read-collaborative",
            "playlist-read-private",
            "streaming",
            "user-follow-modify",
            "user-follow-read",
            "user-library-modify",
            "user-library-read",
            "user-modify",
            "user-modify-playback-state",
            "user-modify-private",
            "user-personalized",
            "user-read-currently-playing",
            "user-read-email",
            "user-read-play-history",
            "user-read-playback-position",
            "user-read-playback-state",
            "user-read-private",
            "user-read-recently-played",
            "user-top-read",
        ];

        let client = OAuthClientBuilder::new(client_id, &redirect_uri, scopes)
            .open_in_browser()
            .build()
            .map_err(|e| PottyError::Generic(e.to_string()))?;

        let token = client.get_access_token_async().await.map_err(|e| PottyError::Generic(e.to_string()))?;

        Ok(token.access_token)
    }
}

uniffi::setup_scaffolding!("potty_bridge");

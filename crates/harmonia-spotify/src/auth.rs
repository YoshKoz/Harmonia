use anyhow::Result;
use rspotify::prelude::*;
use rspotify::{AuthCodePkceSpotify, Config, Credentials, OAuth, scopes};
use std::path::PathBuf;
use tracing::{info, error};

/// Harmonia's Spotify client ID (you must register your own app at https://developer.spotify.com).
/// This placeholder must be replaced with a real client ID.
const CLIENT_ID: &str = "YOUR_SPOTIFY_CLIENT_ID";
const REDIRECT_URI: &str = "http://127.0.0.1:8898/callback";

/// Manages Spotify OAuth PKCE authentication.
pub struct SpotifyAuth {
    spotify: AuthCodePkceSpotify,
    cache_path: PathBuf,
}

impl SpotifyAuth {
    /// Create a new SpotifyAuth with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        let config = Config {
            token_cached: true,
            cache_path: cache_dir.join("token_cache.json"),
            ..Default::default()
        };

        let creds = Credentials::new_pkce(CLIENT_ID);
        let oauth = OAuth {
            redirect_uri: REDIRECT_URI.to_string(),
            scopes: scopes!(
                "user-library-read",
                "playlist-read-private",
                "playlist-read-collaborative",
                "streaming",
                "user-read-playback-state",
                "user-modify-playback-state",
                "user-read-currently-playing"
            ),
            ..Default::default()
        };

        let spotify = AuthCodePkceSpotify::with_config(creds, oauth, config);

        Ok(Self {
            spotify,
            cache_path: cache_dir,
        })
    }

    /// Attempt to load cached token, or start a new auth flow.
    pub async fn authenticate(&mut self) -> Result<()> {
        // Try to use cached token
        let token = self.spotify.read_token_cache(true).await;
        if let Ok(Some(token)) = token {
            *self.spotify.get_token().lock().await.unwrap() = Some(token);
            // Try a simple API call to validate
            if self.spotify.current_user().await.is_ok() {
                info!("Spotify: authenticated from cache");
                return Ok(());
            }
        }

        // Start OAuth PKCE flow
        let auth_url = self.spotify.get_authorize_url(None)?;
        info!("Opening browser for Spotify login");

        // Open browser
        if let Err(e) = open::that(&auth_url) {
            error!("Failed to open browser: {e}. Please navigate to:\n{auth_url}");
        }

        // Wait for redirect callback
        let code = wait_for_callback().await?;
        self.spotify.request_token(&code).await?;

        info!("Spotify: authenticated successfully");
        Ok(())
    }

    /// Get a reference to the authenticated Spotify client.
    pub fn client(&self) -> &AuthCodePkceSpotify {
        &self.spotify
    }

    /// Check if we have a valid session.
    pub async fn is_authenticated(&self) -> bool {
        self.spotify.current_user().await.is_ok()
    }
}

/// Start a tiny HTTP server on 127.0.0.1:8898 to receive the OAuth callback.
async fn wait_for_callback() -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:8898").await?;
    info!("Waiting for Spotify callback on http://127.0.0.1:8898/callback");

    let (mut stream, _) = listener.accept().await?;
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the code from the query string
    let code = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| {
            url::form_urlencoded::parse(
                path.split('?').nth(1).unwrap_or("").as_bytes()
            )
            .find(|(key, _): &(std::borrow::Cow<str>, std::borrow::Cow<str>)| key == "code")
            .map(|(_, val)| val.to_string())
        })
        .ok_or_else(|| anyhow::anyhow!("No authorization code in callback"))?;

    // Send a response
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h1>Success!</h1><p>You can close this tab and return to Harmonia.</p>\
        <script>window.close();</script></body></html>";
    stream.write_all(response.as_bytes()).await?;

    Ok(code)
}

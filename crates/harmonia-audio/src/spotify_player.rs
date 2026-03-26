use anyhow::Result;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::{error, info, warn};

use crate::{PlaybackEngine, PlaybackEvent};
use harmonia_core::models::PlaybackState;

enum SpotifyCmd {
    /// One-time credential login.  Password is used once and dropped; librespot
    /// saves an encrypted blob to the cache directory.
    Login { username: String, password: String },
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(u32),
    Shutdown,
}

struct SpotifyShared {
    state: PlaybackState,
    position_ms: u64,
    duration_ms: u64,
}

/// Spotify streaming engine backed by librespot.
///
/// Credentials are never stored in config.  On first use run:
///   `harmonia --spotify-login`
/// librespot caches an encrypted blob in `cache_dir`; all subsequent launches
/// load from that cache automatically.
pub struct SpotifyPlayer {
    shared: Arc<Mutex<SpotifyShared>>,
    cmd_tx: UnboundedSender<SpotifyCmd>,
}

impl SpotifyPlayer {
    pub fn new(cache_dir: PathBuf, event_tx: Sender<PlaybackEvent>) -> Self {
        let (cmd_tx, cmd_rx) = unbounded_channel::<SpotifyCmd>();
        let shared = Arc::new(Mutex::new(SpotifyShared {
            state: PlaybackState::Stopped,
            position_ms: 0,
            duration_ms: 0,
        }));
        let shared_clone = shared.clone();

        std::thread::Builder::new()
            .name("harmonia-spotify".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("spotify tokio runtime");
                rt.block_on(spotify_task(cmd_rx, event_tx, shared_clone, cache_dir));
            })
            .expect("spawn spotify thread");

        Self { shared, cmd_tx }
    }

    /// Send a one-time login command.  Only needed when no cached credentials
    /// exist.  The password is dropped immediately after Spotify confirms auth.
    pub fn login(&self, username: String, password: String) {
        let _ = self.cmd_tx.send(SpotifyCmd::Login { username, password });
    }
}

impl PlaybackEngine for SpotifyPlayer {
    fn load(&mut self, uri: &str) -> Result<()> {
        let _ = self.cmd_tx.send(SpotifyCmd::Load(uri.to_string()));
        Ok(())
    }
    fn play(&mut self)  { let _ = self.cmd_tx.send(SpotifyCmd::Play); }
    fn pause(&mut self) { let _ = self.cmd_tx.send(SpotifyCmd::Pause); }
    fn stop(&mut self)  { let _ = self.cmd_tx.send(SpotifyCmd::Stop); }
    fn seek(&mut self, ms: u64) { let _ = self.cmd_tx.send(SpotifyCmd::Seek(ms as u32)); }
    fn volume(&self) -> f32        { 1.0 } // system volume
    fn set_volume(&mut self, _: f32) {}
    fn position_ms(&self) -> u64   { self.shared.lock().position_ms }
    fn duration_ms(&self) -> u64   { self.shared.lock().duration_ms }
    fn state(&self) -> PlaybackState { self.shared.lock().state }
}

impl Drop for SpotifyPlayer {
    fn drop(&mut self) { let _ = self.cmd_tx.send(SpotifyCmd::Shutdown); }
}

// ─── async task ─────────────────────────────────────────────────────────────

async fn spotify_task(
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<SpotifyCmd>,
    event_tx: Sender<PlaybackEvent>,
    shared: Arc<Mutex<SpotifyShared>>,
    cache_dir: PathBuf,
) {
    use librespot_core::authentication::Credentials;
    use librespot_core::cache::Cache;
    use librespot_core::config::SessionConfig;
    use librespot_core::session::Session;
    use librespot_core::spotify_id::SpotifyId;
    use librespot_playback::audio_backend;
    use librespot_playback::config::{AudioFormat, PlayerConfig};
    use librespot_playback::mixer::NoOpVolume;
    use librespot_playback::player::{Player, PlayerEvent};

    let _ = std::fs::create_dir_all(&cache_dir);

    let cache = match Cache::new(Some(&cache_dir), None, None, None) {
        Ok(c) => c,
        Err(e) => {
            error!("Spotify cache error: {e}");
            let _ = event_tx.send(PlaybackEvent::Error(format!("Spotify cache: {e}")));
            return;
        }
    };

    // Helper: open a session with given credentials
    let open_session = |creds: Credentials, cache: Cache| async move {
        Session::connect(SessionConfig::default(), creds, Some(cache), false).await
    };

    // Try auto-login from cache
    let mut active: Option<(Player, tokio::sync::mpsc::UnboundedReceiver<PlayerEvent>)> = None;

    match cache.credentials() {
        Some(creds) => {
            match open_session(creds, cache.clone()).await {
                Ok((session, _)) => {
                    info!("Spotify: connected from cache");
                    let backend = audio_backend::find(None).expect("audio backend");
                    let (p, ev) = Player::new(
                        PlayerConfig::default(),
                        session,
                        Box::new(NoOpVolume),
                        move || backend(None, AudioFormat::default()),
                    );
                    active = Some((p, ev));
                }
                Err(e) => {
                    error!("Spotify cached-auth failed: {e}");
                    let _ = event_tx.send(PlaybackEvent::Error(
                        "Spotify login expired. Run: harmonia --spotify-login".into(),
                    ));
                }
            }
        }
        None => {
            info!("Spotify: no cache. Run `harmonia --spotify-login` to authenticate.");
            let _ = event_tx.send(PlaybackEvent::Error(
                "Spotify not logged in. Run: harmonia --spotify-login".into(),
            ));
        }
    }

    // Main loop
    loop {
        if let Some((ref mut player, ref mut events)) = active {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        SpotifyCmd::Load(uri) => {
                            match SpotifyId::from_uri(&uri) {
                                Ok(id) => {
                                    player.load(id, true, 0);
                                    shared.lock().state = PlaybackState::Loading;
                                    let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Loading));
                                }
                                Err(e) => {
                                    warn!("Bad Spotify URI: {e:?}");
                                    let _ = event_tx.send(PlaybackEvent::Error(format!("Bad URI: {e:?}")));
                                }
                            }
                        }
                        SpotifyCmd::Play  => player.play(),
                        SpotifyCmd::Pause => player.pause(),
                        SpotifyCmd::Stop  => {
                            player.stop();
                            let mut s = shared.lock();
                            s.state = PlaybackState::Stopped;
                            s.position_ms = 0;
                            drop(s);
                            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Stopped));
                        }
                        SpotifyCmd::Seek(ms) => player.seek(ms),
                        SpotifyCmd::Login { username, password } => {
                            // Re-auth (e.g. expired session)
                            let creds = Credentials::with_password(&username, &password);
                            if let Ok((session, _)) = open_session(creds, cache.clone()).await {
                                let backend = audio_backend::find(None).expect("audio backend");
                                let (p, ev) = Player::new(
                                    PlayerConfig::default(),
                                    session,
                                    Box::new(NoOpVolume),
                                    move || backend(None, AudioFormat::default()),
                                );
                                active = Some((p, ev));
                                info!("Spotify re-login successful");
                            }
                        }
                        SpotifyCmd::Shutdown => return,
                    }
                }
                Some(ev) = events.recv() => {
                    on_player_event(ev, &shared, &event_tx);
                }
                else => return,
            }
        } else {
            // Not connected — wait for Login or Shutdown
            match cmd_rx.recv().await {
                Some(SpotifyCmd::Login { username, password }) => {
                    let creds = Credentials::with_password(&username, &password);
                    match open_session(creds, cache.clone()).await {
                        Ok((session, _)) => {
                            info!("Spotify login successful — credentials cached");
                            let backend = audio_backend::find(None).expect("audio backend");
                            let (p, ev) = Player::new(
                                PlayerConfig::default(),
                                session,
                                Box::new(NoOpVolume),
                                move || backend(None, AudioFormat::default()),
                            );
                            active = Some((p, ev));
                        }
                        Err(e) => {
                            error!("Spotify login failed: {e}");
                            let _ = event_tx.send(PlaybackEvent::Error(
                                format!("Spotify login failed: {e}"),
                            ));
                        }
                    }
                }
                Some(SpotifyCmd::Shutdown) | None => return,
                _ => {} // ignore playback commands until connected
            }
        }
    }
}

fn on_player_event(
    event: librespot_playback::player::PlayerEvent,
    shared: &Arc<Mutex<SpotifyShared>>,
    event_tx: &Sender<PlaybackEvent>,
) {
    use librespot_playback::player::PlayerEvent;
    match event {
        PlayerEvent::Playing { position_ms, duration_ms, .. } => {
            {
                let mut s = shared.lock();
                s.state = PlaybackState::Playing;
                s.position_ms = position_ms as u64;
                s.duration_ms = duration_ms as u64;
            }
            let _ = event_tx.send(PlaybackEvent::TrackLoaded { duration_ms: duration_ms as u64 });
            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Playing));
        }
        PlayerEvent::Paused { position_ms, duration_ms, .. } => {
            {
                let mut s = shared.lock();
                s.state = PlaybackState::Paused;
                s.position_ms = position_ms as u64;
                s.duration_ms = duration_ms as u64;
            }
            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Paused));
        }
        PlayerEvent::Stopped { .. } => {
            shared.lock().state = PlaybackState::Stopped;
            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Stopped));
        }
        PlayerEvent::EndOfTrack { .. } => {
            shared.lock().state = PlaybackState::Stopped;
            let _ = event_tx.send(PlaybackEvent::TrackFinished);
            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Stopped));
        }
        PlayerEvent::Unavailable { .. } => {
            let _ = event_tx.send(PlaybackEvent::Error(
                "Track unavailable (Premium required)".into(),
            ));
        }
        _ => {}
    }
}

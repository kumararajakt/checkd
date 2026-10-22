//! Google Calendar sync.
//!
//! The sync is **local-first**: SQLite is the source of truth and Google is an
//! optional mirror. Nothing here talks to the network unless the user
//! connected a Google account in the settings.
//!
//! The module has three parts:
//!
//! * [`auth`] — the OAuth consent flow, the token store and the account
//!   lookup. The client id, the client secret and the tokens live in the
//!   operating system's keyring, never in the database.
//! * [`google`] — the Calendar REST client.
//! * this file — the [`SyncManager`], which ties them to the database and
//!   owns the connection state.
//!
//! ## The local bookkeeping
//!
//! Every local write has to take part in the sync, so the `sync_links` table
//! is at the same time
//!
//! * the **outbox** — one row per local entity, `dirty = 1` while it has
//!   changes that still need to be pushed to Google,
//! * the **id mapping** — the Google calendar id / event id the local
//!   entity belongs to (`remote_id`, `None` until it exists remotely), and
//! * the **tombstone log** — a deleted entity keeps its row with
//!   `deleted = 1` so the remote object can be removed and so a pull cannot
//!   resurrect it.
//!
//! The rows are written by the normal mutation paths in `db.rs`, so the
//! outbox is a side effect of using the app — there is no separate "sync
//! this" step the user could forget.

pub mod auth;
pub mod credentials;
pub mod google;

use std::net::TcpListener;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::db::Db;
use crate::sync::auth::{OAuthClient, SecretStore, Tokens};
use crate::sync::credentials::ResolvedClient;
use crate::sync::google::{GoogleApi, GoogleClient};

/// What a sync link points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEntity {
    /// A list — mirrored as a Google Calendar.
    List,
    /// A task — mirrored as a Google Event.
    Task,
}

impl SyncEntity {
    /// The value stored in `sync_links.entity_type`.
    pub fn as_str(self) -> &'static str {
        match self {
            SyncEntity::List => "list",
            SyncEntity::Task => "task",
        }
    }

    /// Parses a stored `entity_type`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "list" => Some(SyncEntity::List),
            "task" => Some(SyncEntity::Task),
            _ => None,
        }
    }
}

/// One row of the outbox.
#[derive(Debug, Clone)]
pub struct SyncLink {
    pub entity: SyncEntity,
    /// The local id — the list id or the task id.
    pub entity_id: String,
    /// The Google calendar id / event id. `None` until it exists remotely.
    pub remote_id: Option<String>,
    /// The local change still has to be pushed.
    pub dirty: bool,
    /// The entity was deleted locally; the remote object still has to go.
    pub deleted: bool,
    /// When the local change happened (unix milliseconds).
    pub updated_at: i64,
    /// When the last push of this entity succeeded (unix milliseconds).
    pub synced_at: i64,
}

/// Key used in `sync_state` for "the user connected a Google account".
pub const STATE_ENABLED: &str = "enabled";
/// Key used in `sync_state` for the connected Google account id.
pub const STATE_ACCOUNT: &str = "account";
/// Key used in `sync_state` for the time of the last successful sync.
pub const STATE_SYNCED_AT: &str = "synced_at";
/// Key used in `sync_state` for the message of the last failed sync.
pub const STATE_ERROR: &str = "last_error";
/// Key used in `sync_state` for the number of checkd calendars on the Google side.
pub const STATE_CALENDARS: &str = "remote_calendars";

/// The name of the keyring entries.
const KEYRING_SERVICE: &str = "checkd";
/// The keyring entry with the refresh token.
const REFRESH_TOKEN_NAME: &str = "refresh-token";
/// The keyring entry with the current access token.
const ACCESS_TOKEN_NAME: &str = "access-token";

/// What the settings dialog needs to know about the sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// The OAuth client was found — the user can connect.
    pub configured: bool,
    /// A Google account is connected.
    pub connected: bool,
    /// The connected account.
    pub account: Option<String>,
    /// Local changes that are waiting to be pushed.
    pub pending: i64,
    /// When the last sync ran (unix milliseconds).
    pub last_sync_at: Option<i64>,
    /// The message of the last failed sync.
    pub last_error: Option<String>,
}

/// The OAuth client the app found — the secret itself never leaves the
/// keyring or the environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatus {
    /// Whether a client was found at all.
    pub configured: bool,
    /// The client id.
    pub client_id: Option<String>,
    /// A hint that shows the secret is there without showing it.
    pub secret_hint: Option<String>,
    /// Where the client came from — an environment variable or a `.env` file.
    pub source: Option<String>,
    /// The file the app reads when the environment says nothing.
    pub env_file: Option<String>,
}

/// Owns the connection to Google: the credentials, the tokens and the status.
pub struct SyncManager {
    secrets: Arc<dyn SecretStore>,
    api: Arc<dyn GoogleClient>,
    /// The OAuth client, read from the environment at startup.
    client: std::sync::Mutex<Option<ResolvedClient>>,
    /// The app-data directory — where the `.env` file lives.
    app_data: std::sync::Mutex<Option<std::path::PathBuf>>,
}

impl SyncManager {
    /// A manager that keeps the secrets in the OS keyring.
    pub fn new() -> Self {
        SyncManager::with_stores(
            Arc::new(auth::SafeStore::new(KEYRING_SERVICE)),
            Arc::new(GoogleApi::new()),
        )
    }

    /// The directory the secrets fall back to when the system has no keyring.
    pub fn set_fallback_dir(&self, dir: std::path::PathBuf) {
        self.secrets.set_fallback_dir(dir);
    }

    /// A manager with the given stores — the tests use this to stay off the
    /// network and out of the real keyring.
    pub fn with_stores(secrets: Arc<dyn SecretStore>, api: Arc<dyn GoogleClient>) -> Self {
        SyncManager {
            secrets,
            api,
            client: std::sync::Mutex::new(None),
            app_data: std::sync::Mutex::new(None),
        }
    }

    /// Reads the OAuth client from the environment (or a `.env` file).
    ///
    /// The app asks for no credentials — the user brings them through the
    /// environment.
    pub fn load_client(&self, app_data_dir: Option<std::path::PathBuf>) -> Result<(), String> {
        let dirs = credentials::search_dirs(app_data_dir.clone());

        {
            let mut app_data = self.app_data.lock().unwrap_or_else(|err| err.into_inner());
            *app_data = app_data_dir;
        }

        let resolved = credentials::resolve_client(&dirs)?;

        let mut client = self.client.lock().unwrap_or_else(|err| err.into_inner());
        *client = resolved;

        Ok(())
    }

    /// The OAuth client the app found.
    pub fn resolved_client(&self) -> Option<ResolvedClient> {
        self.client
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

    /// The state of the sync, read from the keyring and the database.
    pub fn status(&self, db: &Db) -> SyncStatus {
        let mut last_error = db
            .sync_state(STATE_ERROR)
            .ok()
            .flatten()
            .filter(|message| !message.is_empty());

        let refresh_token = self.refresh_token().unwrap_or_else(|err| {
            last_error.get_or_insert(err);
            None
        });

        SyncStatus {
            configured: self.resolved_client().is_some(),
            connected: refresh_token.is_some(),
            account: db
                .sync_state(STATE_ACCOUNT)
                .ok()
                .flatten()
                .filter(|account| !account.is_empty()),
            pending: db.pending_count().unwrap_or(0),
            last_sync_at: db
                .sync_state(STATE_SYNCED_AT)
                .ok()
                .flatten()
                .and_then(|value| value.parse().ok()),
            last_error,
        }
    }

    /// The OAuth client the app found — the secret itself is not sent back.
    pub fn credentials_status(&self) -> Result<CredentialsStatus, String> {
        match self.resolved_client() {
            Some(resolved) => Ok(CredentialsStatus {
                configured: true,
                client_id: Some(resolved.client.client_id),
                secret_hint: Some(secret_hint(&resolved.client.client_secret)),
                source: Some(resolved.source.describe()),
                env_file: self.env_file(),
            }),
            None => Ok(CredentialsStatus {
                configured: false,
                client_id: None,
                secret_hint: None,
                source: None,
                env_file: self.env_file(),
            }),
        }
    }

    /// The `.env` file the app reads when the environment says nothing.
    pub fn env_file(&self) -> Option<String> {
        let app_data = self
            .app_data
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()?;

        Some(
            credentials::env_file_in(&app_data)
                .display()
                .to_string(),
        )
    }

    /// Runs the consent flow and stores the tokens.
    ///
    /// The browser is opened on the Google consent screen; the answer comes
    /// back to a loopback listener on `127.0.0.1`.
    pub async fn sign_in<R: tauri::Runtime>(
        &self,
        app: AppHandle<R>,
        db: &Db,
    ) -> Result<SyncStatus, String> {
        let client = self
            .client()?
            .ok_or("No Google OAuth client found — set CHECKD_GOOGLE_CLIENT_ID and CHECKD_GOOGLE_CLIENT_SECRET in the environment.")?;

        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();
        let redirect_uri = format!("http://127.0.0.1:{port}");
        let state = uuid::Uuid::new_v4().to_string();

        let url = auth::consent_url(&client.client_id, &redirect_uri, &state);
        open_browser(&app, &url)?;

        let expected = state;
        let code =
            tauri::async_runtime::spawn_blocking(move || auth::wait_for_code(listener, &expected))
                .await
                .map_err(|e| e.to_string())??;

        let tokens = auth::exchange_code(&client, &code, &redirect_uri).await?;
        self.store_tokens(&tokens)?;

        if let Some(email) = auth::account_email(&tokens.access_token).await.ok() {
            db.set_sync_state(STATE_ACCOUNT, &email)?;
        }
        db.set_sync_state(STATE_ENABLED, "1")?;
        db.set_sync_state(STATE_ERROR, "")?;

        Ok(self.status(db))
    }

    /// Gives the access back to Google and forgets the tokens.
    pub async fn sign_out(&self, db: &Db) -> Result<SyncStatus, String> {
        if let Some(token) = self.refresh_token().unwrap_or(None) {
            // Best effort — the local tokens go either way.
            let _ = auth::revoke(&token).await;
        }
        let _ = self.secrets.delete(REFRESH_TOKEN_NAME);
        let _ = self.secrets.delete(ACCESS_TOKEN_NAME);

        db.set_sync_state(STATE_ENABLED, "0")?;
        db.clear_sync_state(STATE_ACCOUNT)?;
        db.set_sync_state(STATE_ERROR, "")?;

        Ok(self.status(db))
    }

    /// Talks to Google.
    ///
    /// The push and the pull arrive with the sync engine; for now this proves
    /// that the token works and counts the checkd calendars on the Google side.
    pub async fn sync_now(&self, db: &Db) -> Result<SyncStatus, String> {
        match self.run_sync(db).await {
            Ok(()) => {
                db.set_sync_state(STATE_ERROR, "")?;
                Ok(self.status(db))
            }
            Err(err) => {
                db.set_sync_state(STATE_ERROR, &err)?;
                Err(err)
            }
        }
    }

    /// The work of one sync run.
    async fn run_sync(&self, db: &Db) -> Result<(), String> {
        let token = self.access_token().await?;
        let calendars = self.api.list_calendars(&token).await?;

        db.set_sync_state(STATE_CALENDARS, &calendars.len().to_string())?;
        db.set_sync_state(
            STATE_SYNCED_AT,
            &chrono::Utc::now().timestamp_millis().to_string(),
        )?;

        Ok(())
    }

    /// The OAuth client the app found.
    fn client(&self) -> Result<Option<OAuthClient>, String> {
        Ok(self.resolved_client().map(|resolved| resolved.client))
    }

    /// The refresh token of the connected account.
    fn refresh_token(&self) -> Result<Option<String>, String> {
        Ok(self.secrets.get(REFRESH_TOKEN_NAME)?)
    }

    /// An access token that is good for at least another minute — the old one
    /// is refreshed when it is about to expire.
    async fn access_token(&self) -> Result<String, String> {
        if let Ok(Some(raw)) = self.secrets.get(ACCESS_TOKEN_NAME) {
            if let Ok(tokens) = serde_json::from_str::<Tokens>(&raw) {
                if tokens.is_fresh() {
                    return Ok(tokens.access_token);
                }
            }
        }

        let refresh_token = self
            .refresh_token()?
            .ok_or("No Google account is connected.")?;
        let client = self
            .client()?
            .ok_or("No Google OAuth client found — set CHECKD_GOOGLE_CLIENT_ID and CHECKD_GOOGLE_CLIENT_SECRET in the environment.")?;

        let tokens = auth::refresh(&client, &refresh_token).await?;
        self.store_tokens(&tokens)?;

        Ok(tokens.access_token)
    }

    /// Puts the tokens into the keyring.
    fn store_tokens(&self, tokens: &Tokens) -> Result<(), String> {
        let raw = serde_json::to_string(tokens).map_err(|e| e.to_string())?;
        self.secrets.set(ACCESS_TOKEN_NAME, &raw)?;

        // A refreshed token comes without a refresh token — the old one stays
        // valid, so it is kept.
        if let Some(refresh_token) = &tokens.refresh_token {
            self.secrets.set(REFRESH_TOKEN_NAME, refresh_token)?;
        }

        Ok(())
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        SyncManager::new()
    }
}

/// Opens the consent screen in the user's browser.
fn open_browser<R: tauri::Runtime>(app: &AppHandle<R>, url: &str) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// A hint that shows the secret is there without showing it.
fn secret_hint(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 10 {
        return "••••".to_string();
    }

    let head: String = chars[..6].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::auth::MemoryStore;
    use crate::sync::credentials::ClientSource;

    #[test]
    fn the_secret_hint_hides_the_middle() {
        assert_eq!(secret_hint("short"), "••••");
        assert_eq!(secret_hint("GOCSPX-abcdefghijkl"), "GOCSPX…ijkl");
    }

    /// A manager with a client, the way the environment would provide one.
    fn manager_with_client() -> SyncManager {
        let manager = SyncManager::with_stores(
            Arc::new(MemoryStore::default()),
            Arc::new(google::GoogleApi::new()),
        );

        {
            let mut client = manager.client.lock().unwrap_or_else(|err| err.into_inner());
            *client = Some(ResolvedClient {
                client: OAuthClient {
                    client_id: "123.apps.googleusercontent.com".to_string(),
                    client_secret: "GOCSPX-abcdefghijkl".to_string(),
                },
                source: ClientSource::Environment("CHECKD_GOOGLE_CLIENT_ID".to_string()),
            });
        }

        manager
    }

    #[test]
    fn the_client_from_the_environment_is_reported() {
        let manager = manager_with_client();

        let status = manager.credentials_status().unwrap();
        assert!(status.configured);
        assert_eq!(
            status.client_id.as_deref(),
            Some("123.apps.googleusercontent.com")
        );
        assert_eq!(status.secret_hint.as_deref(), Some("GOCSPX…ijkl"));
        assert_eq!(
            status.source.as_deref(),
            Some("CHECKD_GOOGLE_CLIENT_ID (environment)")
        );
    }

    #[test]
    fn without_a_client_nothing_is_configured() {
        let manager = SyncManager::with_stores(
            Arc::new(MemoryStore::default()),
            Arc::new(google::GoogleApi::new()),
        );

        let status = manager.credentials_status().unwrap();
        assert!(!status.configured);
        assert_eq!(status.client_id, None);
        assert_eq!(status.source, None);
    }

    #[test]
    fn the_env_file_of_the_app_data_dir_is_named() {
        let manager = SyncManager::with_stores(
            Arc::new(MemoryStore::default()),
            Arc::new(google::GoogleApi::new()),
        );

        // Without an app-data directory there is nothing to point at.
        assert_eq!(manager.env_file(), None);

        manager
            .load_client(Some(std::path::PathBuf::from("/tmp/checkd-app-data")))
            .ok();

        assert_eq!(
            manager.env_file().as_deref(),
            Some("/tmp/checkd-app-data/.env")
        );
        assert_eq!(
            manager.credentials_status().unwrap().env_file.as_deref(),
            Some("/tmp/checkd-app-data/.env")
        );
    }
}

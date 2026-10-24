pub mod auth;
pub mod credentials;
pub mod google;
pub mod merge;
pub mod payload;

use std::collections::HashSet;
use std::net::TcpListener;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use crate::db::Db;
use crate::sync::auth::{OAuthClient, SecretStore, Tokens};
use crate::sync::credentials::ResolvedClient;
use crate::sync::google::{GoogleApi, GoogleClient};
use crate::sync::merge::{
    handle_local_absent, merge_list, merge_task, ListAction, LocalAbsent, LocalState, TaskAction,
};
use crate::sync::payload::{list_calendar, task_event, ListPayload, TaskPayload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEntity {
    List,
    Task,
}

impl SyncEntity {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncEntity::List => "list",
            SyncEntity::Task => "task",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "list" => Some(SyncEntity::List),
            "task" => Some(SyncEntity::Task),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncLink {
    pub entity: SyncEntity,
    pub entity_id: String,
    pub parent_id: Option<String>,
    pub remote_id: Option<String>,
    pub dirty: bool,
    pub deleted: bool,
    pub updated_at: i64,
    pub synced_at: i64,
}

pub const STATE_ENABLED: &str = "enabled";
pub const STATE_ACCOUNT: &str = "account";
pub const STATE_SYNCED_AT: &str = "synced_at";
pub const STATE_ERROR: &str = "last_error";
pub const STATE_CALENDARS: &str = "remote_calendars";
pub const STATE_CALENDAR_LIST_TOKEN: &str = "calendar_list_token";

pub fn event_token_key(calendar_id: &str) -> String {
    format!("events_token:{calendar_id}")
}

fn is_not_found(err: &str) -> bool {
    err.contains("404")
        || err.contains("410")
        || err.contains("Not Found")
        || err.contains("gone on the Google side")
        || err.contains("Resource has been deleted")
}

const KEYRING_SERVICE: &str = "checkd";
const REFRESH_TOKEN_NAME: &str = "refresh-token";
const ACCESS_TOKEN_NAME: &str = "access-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub configured: bool,
    pub connected: bool,
    pub account: Option<String>,
    pub pending: i64,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatus {
    pub configured: bool,
    pub client_id: Option<String>,
    pub secret_hint: Option<String>,
    pub source: Option<String>,
    pub env_file: Option<String>,
}

pub struct SyncManager {
    secrets: Arc<dyn SecretStore>,
    api: Arc<dyn GoogleClient>,
    client: std::sync::Mutex<Option<ResolvedClient>>,
    app_data: std::sync::Mutex<Option<std::path::PathBuf>>,
    app: std::sync::Mutex<Option<AppHandle>>,
}

impl SyncManager {
    pub fn new() -> Self {
        SyncManager::with_stores(
            Arc::new(auth::SafeStore::new(KEYRING_SERVICE)),
            Arc::new(GoogleApi::new()),
        )
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        let mut app = self.app.lock().unwrap_or_else(|err| err.into_inner());
        *app = Some(handle);
    }

    pub fn set_fallback_dir(&self, dir: std::path::PathBuf) {
        self.secrets.set_fallback_dir(dir);
    }

    pub fn with_stores(secrets: Arc<dyn SecretStore>, api: Arc<dyn GoogleClient>) -> Self {
        SyncManager {
            secrets,
            api,
            client: std::sync::Mutex::new(None),
            app_data: std::sync::Mutex::new(None),
            app: std::sync::Mutex::new(None),
        }
    }

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

    pub fn resolved_client(&self) -> Option<ResolvedClient> {
        self.client
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

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

    pub async fn sign_out(&self, db: &Db) -> Result<SyncStatus, String> {
        if let Some(token) = self.refresh_token().unwrap_or(None) {
            let _ = auth::revoke(&token).await;
        }
        let _ = self.secrets.delete(REFRESH_TOKEN_NAME);
        let _ = self.secrets.delete(ACCESS_TOKEN_NAME);

        db.set_sync_state(STATE_ENABLED, "0")?;
        db.clear_sync_state(STATE_ACCOUNT)?;
        db.set_sync_state(STATE_ERROR, "")?;

        Ok(self.status(db))
    }

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

    async fn run_sync(&self, db: &Db) -> Result<(), String> {
        let token = self.access_token().await?;

        self.pull(db, &token).await?;
        let push_error = self.push(db, &token).await?;

        let calendar_list = self.api.list_calendars_delta(&token, None).await?;
        let calendars = calendar_list.calendars;

        db.set_sync_state(STATE_CALENDARS, &calendars.len().to_string())?;
        if let Some(token_val) = calendar_list.next_sync_token {
            db.set_sync_state(STATE_CALENDAR_LIST_TOKEN, &token_val)?;
        }
        db.set_sync_state(
            STATE_SYNCED_AT,
            &chrono::Utc::now().timestamp_millis().to_string(),
        )?;

        if let Some(app) = self.app.lock().unwrap_or_else(|err| err.into_inner()).clone() {
            let _ = app.emit(crate::notify::DATA_CHANGED, ());
        }

        if let Some(err) = push_error {
            return Err(err);
        }

        Ok(())
    }

    async fn pull(&self, db: &Db, token: &str) -> Result<(), String> {
        let saved_cal_token = db.sync_state(STATE_CALENDAR_LIST_TOKEN)?.filter(|s| !s.is_empty());
        let is_full_cal_sync = saved_cal_token.is_none();
        let cal_list = self
            .api
            .list_calendars_delta(token, saved_cal_token.as_deref())
            .await?;

        let mut seen_list_ids = HashSet::new();

        for remote_cal in &cal_list.calendars {
            if remote_cal.deleted {
                if let Some(link) = db.link_by_remote_id(SyncEntity::List, &remote_cal.id)? {
                    if !link.deleted && handle_local_absent(link.dirty) == LocalAbsent::Delete {
                        db.remove_remote_deleted_list(&link.entity_id)?;
                    }
                }
                continue;
            }

            if !remote_cal.hidden {
                let _ = self.api.hide_calendar(token, &remote_cal.id).await;
            }

            let Some(desc) = remote_cal.description.as_deref() else {
                continue;
            };
            let Some(list_payload) = ListPayload::parse(desc) else {
                continue;
            };

            let list_id = list_payload.id.clone();
            seen_list_ids.insert(list_id.clone());

            let local_list = db.sync_list(&list_id)?;
            let link = db.link(SyncEntity::List, &list_id)?;
            let link_deleted = link.as_ref().map(|l| l.deleted).unwrap_or(false);
            let link_dirty = link.as_ref().map(|l| l.dirty).unwrap_or(false);
            let local_state = LocalState {
                exists: local_list.is_some(),
                updated_at: local_list.as_ref().map(|l| l.updated_at),
                dirty: link_dirty,
                deleted: link_deleted,
            };

            match merge_list(&list_payload, Some(&local_state), link_deleted, link_dirty) {
                ListAction::Create { list, .. } | ListAction::Update { list, .. } => {
                    db.apply_remote_list(&list, &remote_cal.id)?;
                }
                ListAction::Ignore => {
                    if link.as_ref().and_then(|l| l.remote_id.as_ref()).is_none() {
                        db.set_remote_id(SyncEntity::List, &list_id, &remote_cal.id)?;
                    }
                }
                ListAction::Tombstone => {
                    if link.as_ref().and_then(|l| l.remote_id.as_ref()).is_none() {
                        db.set_remote_id(SyncEntity::List, &list_id, &remote_cal.id)?;
                    }
                    continue;
                }
            }

            if db.sync_list(&list_id)?.is_none() {
                continue;
            }

            let evt_token_key = event_token_key(&remote_cal.id);
            let saved_evt_token = db.sync_state(&evt_token_key)?.filter(|s| !s.is_empty());
            let is_full_evt_sync = saved_evt_token.is_none();
            let event_list = self
                .api
                .list_events_delta(token, &remote_cal.id, saved_evt_token.as_deref())
                .await?;

            let mut seen_task_ids = HashSet::new();

            for remote_evt in &event_list.events {
                if remote_evt.status.as_deref() == Some("cancelled") {
                    let task_id = remote_evt
                        .ical_uid
                        .clone()
                        .or_else(|| {
                            db.link_by_remote_id(SyncEntity::Task, &remote_evt.id)
                                .ok()
                                .flatten()
                                .map(|l| l.entity_id)
                        });
                    if let Some(tid) = task_id {
                        let tlink = db.link(SyncEntity::Task, &tid)?;
                        let dirty = tlink.as_ref().map(|l| l.dirty).unwrap_or(false);
                        let deleted = tlink.as_ref().map(|l| l.deleted).unwrap_or(false);
                        if !deleted && handle_local_absent(dirty) == LocalAbsent::Delete {
                            db.remove_remote_deleted_task(&tid)?;
                        }
                    }
                    continue;
                }

                let Some(desc) = remote_evt.description.as_deref() else {
                    continue;
                };
                let Some(mut task_payload) = TaskPayload::parse(desc) else {
                    continue;
                };
                task_payload.list_id = list_id.clone();

                let task_id = task_payload.id.clone();
                seen_task_ids.insert(task_id.clone());

                let local_task = db.sync_task(&task_id)?;
                let tlink = db.link(SyncEntity::Task, &task_id)?;
                let tlink_deleted = tlink.as_ref().map(|l| l.deleted).unwrap_or(false);
                let tlink_dirty = tlink.as_ref().map(|l| l.dirty).unwrap_or(false);
                let tlocal_state = LocalState {
                    exists: local_task.is_some(),
                    updated_at: local_task.as_ref().map(|t| t.updated_at),
                    dirty: tlink_dirty,
                    deleted: tlink_deleted,
                };

                match merge_task(&task_payload, Some(&tlocal_state), tlink_deleted, tlink_dirty) {
                    TaskAction::Create { task, .. } | TaskAction::Update { task, .. } => {
                        db.apply_remote_task(&task, &remote_evt.id)?;
                    }
                    TaskAction::Ignore | TaskAction::Tombstone => {
                        if tlink.as_ref().and_then(|l| l.remote_id.as_ref()).is_none() {
                            db.set_remote_id(SyncEntity::Task, &task_id, &remote_evt.id)?;
                        }
                    }
                }
            }

            if is_full_evt_sync {
                for local_tid in db.task_ids_of(&list_id)? {
                    if !seen_task_ids.contains(&local_tid) {
                        let tlink = db.link(SyncEntity::Task, &local_tid)?;
                        let dirty = tlink.as_ref().map(|l| l.dirty).unwrap_or(false);
                        if handle_local_absent(dirty) == LocalAbsent::Delete {
                            db.remove_remote_deleted_task(&local_tid)?;
                        }
                    }
                }
            }

            if let Some(next_evt_token) = event_list.next_sync_token {
                db.set_sync_state(&evt_token_key, &next_evt_token)?;
            }
        }

        if is_full_cal_sync {
            for local_lid in db.all_list_ids()? {
                if !seen_list_ids.contains(&local_lid) {
                    let link = db.link(SyncEntity::List, &local_lid)?;
                    let dirty = link.as_ref().map(|l| l.dirty).unwrap_or(false);
                    if handle_local_absent(dirty) == LocalAbsent::Delete {
                        db.remove_remote_deleted_list(&local_lid)?;
                    }
                }
            }
        }

        if let Some(next_cal_token) = cal_list.next_sync_token {
            db.set_sync_state(STATE_CALENDAR_LIST_TOKEN, &next_cal_token)?;
        }

        Ok(())
    }

    async fn push(&self, db: &Db, token: &str) -> Result<Option<String>, String> {
        let links = db.pending_links()?;
        let mut first_error: Option<String> = None;

        for link in links {
            let result = match link.entity {
                SyncEntity::List => self.push_list(db, token, &link).await,
                SyncEntity::Task => self.push_task(db, token, &link).await,
            };

            if let Err(err) = result {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }

        Ok(first_error)
    }

    async fn push_list(&self, db: &Db, token: &str, link: &SyncLink) -> Result<(), String> {
        if link.deleted {
            if let Some(remote_id) = link.remote_id.as_deref() {
                match self.api.delete_calendar(token, remote_id).await {
                    Ok(()) => {}
                    Err(err) if is_not_found(&err) => {}
                    Err(err) => return Err(err),
                }
            }
            db.drop_link(SyncEntity::List, &link.entity_id)?;
            return Ok(());
        }

        let Some(list) = db.sync_list(&link.entity_id)? else {
            db.drop_link(SyncEntity::List, &link.entity_id)?;
            return Ok(());
        };

        let payload = list_calendar(&list);

        let remote_id = match link.remote_id.as_deref() {
            Some(existing_id) => match self.api.update_calendar(token, existing_id, &payload).await {
                Ok(updated) => updated.id,
                Err(err) if is_not_found(&err) => {
                    self.api.create_calendar(token, &payload).await?.id
                }
                Err(err) => return Err(err),
            },
            None => self.api.create_calendar(token, &payload).await?.id,
        };

        db.set_remote_id(SyncEntity::List, &link.entity_id, &remote_id)?;
        db.mark_synced(SyncEntity::List, &link.entity_id)?;
        Ok(())
    }

    async fn push_task(&self, db: &Db, token: &str, link: &SyncLink) -> Result<(), String> {
        if link.deleted {
            if let (Some(remote_id), Some(parent_id)) =
                (link.remote_id.as_deref(), link.parent_id.as_deref())
            {
                let parent_link = db.link(SyncEntity::List, parent_id)?;
                if let Some(plink) = parent_link {
                    if !plink.deleted {
                        if let Some(cal_id) = plink.remote_id.as_deref() {
                            match self.api.delete_event(token, cal_id, remote_id).await {
                                Ok(()) => {}
                                Err(err) if is_not_found(&err) => {}
                                Err(err) => return Err(err),
                            }
                        }
                    }
                }
            }
            db.drop_link(SyncEntity::Task, &link.entity_id)?;
            return Ok(());
        }

        let Some(task) = db.sync_task(&link.entity_id)? else {
            db.drop_link(SyncEntity::Task, &link.entity_id)?;
            return Ok(());
        };

        let parent_link = db.link(SyncEntity::List, &task.list_id)?;
        let Some(cal_id) = parent_link.and_then(|l| l.remote_id) else {
            return Err("The list for this task has not been synced to Google yet.".to_string());
        };

        let payload = task_event(&task);

        let remote_id = match link.remote_id.as_deref() {
            Some(existing_id) => {
                match self
                    .api
                    .update_event(token, &cal_id, existing_id, &payload)
                    .await
                {
                    Ok(updated) => updated.id,
                    Err(err) if is_not_found(&err) => {
                        self.api.create_event(token, &cal_id, &payload).await?.id
                    }
                    Err(err) => return Err(err),
                }
            }
            None => self.api.create_event(token, &cal_id, &payload).await?.id,
        };

        db.set_remote_id(SyncEntity::Task, &link.entity_id, &remote_id)?;
        db.mark_synced(SyncEntity::Task, &link.entity_id)?;
        Ok(())
    }

    fn client(&self) -> Result<Option<OAuthClient>, String> {
        Ok(self.resolved_client().map(|resolved| resolved.client))
    }

    fn refresh_token(&self) -> Result<Option<String>, String> {
        Ok(self.secrets.get(REFRESH_TOKEN_NAME)?)
    }

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

    fn store_tokens(&self, tokens: &Tokens) -> Result<(), String> {
        let raw = serde_json::to_string(tokens).map_err(|e| e.to_string())?;
        self.secrets.set(ACCESS_TOKEN_NAME, &raw)?;

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

fn open_browser<R: tauri::Runtime>(app: &AppHandle<R>, url: &str) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

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

pub mod db;
pub mod notify;
pub mod repeat;
pub mod sync;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WindowEvent};

use crate::db::{AppData, Db, Repeat, Task};
use crate::sync::{CredentialsStatus, SyncManager, SyncStatus};

const APP_ICON: tauri::image::Image<'static> = tauri::include_image!("icons/32x32.png");

static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn set_close_to_tray(enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
fn export_data(path: String, contents: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(&target, contents).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_data(db: State<'_, Db>) -> Result<AppData, String> {
    db.load()
}

#[tauri::command]
fn create_list(db: State<'_, Db>, id: String, name: String, color: String) -> Result<(), String> {
    db.create_list(&id, &name, &color)
}

#[tauri::command]
fn rename_list(db: State<'_, Db>, id: String, name: String) -> Result<(), String> {
    db.rename_list(&id, &name)
}

#[tauri::command]
fn delete_list(db: State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_list(&id)
}

#[tauri::command]
fn set_list_color(db: State<'_, Db>, id: String, color: String) -> Result<(), String> {
    db.set_list_color(&id, &color)
}

#[tauri::command]
fn create_task(db: State<'_, Db>, id: String, list_id: String, title: String) -> Result<(), String> {
    db.create_task(&id, &list_id, &title)
}

#[tauri::command]
fn rename_task(db: State<'_, Db>, id: String, title: String) -> Result<(), String> {
    db.rename_task(&id, &title)
}

#[tauri::command]
fn set_task_due(
    db: State<'_, Db>,
    id: String,
    due_date: Option<String>,
    due_time: Option<String>,
) -> Result<(), String> {
    db.set_task_due(&id, due_date.as_deref(), due_time.as_deref())
}

#[tauri::command]
fn set_task_description(db: State<'_, Db>, id: String, description: String) -> Result<(), String> {
    db.set_task_description(&id, &description)
}

#[tauri::command]
fn set_task_repeat(db: State<'_, Db>, id: String, repeat: Option<Repeat>) -> Result<Task, String> {
    db.set_task_repeat(&id, repeat.as_ref())
}

#[tauri::command]
fn set_task_completed(db: State<'_, Db>, id: String, completed: bool) -> Result<Task, String> {
    db.set_task_completed(&id, completed)
}

#[tauri::command]
fn delete_task(db: State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_task(&id)
}

#[tauri::command]
fn google_status(sync: State<'_, SyncManager>, db: State<'_, Db>) -> SyncStatus {
    sync.status(&db)
}

#[tauri::command]
fn google_credentials_status(sync: State<'_, SyncManager>) -> Result<CredentialsStatus, String> {
    sync.credentials_status()
}

#[tauri::command]
async fn google_sign_in<R: tauri::Runtime>(
    app: AppHandle<R>,
    sync: State<'_, SyncManager>,
    db: State<'_, Db>,
) -> Result<SyncStatus, String> {
    sync.sign_in(app, &db).await
}

#[tauri::command]
async fn google_sign_out(
    sync: State<'_, SyncManager>,
    db: State<'_, Db>,
) -> Result<SyncStatus, String> {
    sync.sign_out(&db).await
}

#[tauri::command]
async fn google_sync_now(
    sync: State<'_, SyncManager>,
    db: State<'_, Db>,
) -> Result<SyncStatus, String> {
    sync.sync_now(&db).await
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn builder<R: tauri::Runtime>(db: Option<Db>, sync: SyncManager) -> tauri::Builder<R> {
    let builder = tauri::Builder::<R>::new()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(sync)
        .invoke_handler(tauri::generate_handler![
            set_close_to_tray,
            export_data,
            load_data,
            create_list,
            rename_list,
            delete_list,
            set_list_color,
            create_task,
            rename_task,
            set_task_due,
            set_task_description,
            set_task_repeat,
            set_task_completed,
            delete_task,
            google_status,
            google_credentials_status,
            google_sign_in,
            google_sign_out,
            google_sync_now,
        ]);

    match db {
        Some(db) => builder.manage(db),
        None => builder,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut b = builder::<tauri::Wry>(None, SyncManager::new());
    #[cfg(desktop)]
    {
        b = b
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                show_main_window(app);
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ));
    }
    b.setup(|app| {
            app.manage(Db::open(app.handle())?);
            app.state::<SyncManager>()
                .set_app_handle(app.handle().clone());

            let data_dir = app.path().app_data_dir().ok();

            if let Some(dir) = data_dir.clone() {
                app.state::<SyncManager>().set_fallback_dir(dir);
            }

            match app.state::<SyncManager>().load_client(data_dir) {
                Ok(()) => {
                    if let Some(resolved) = app.state::<SyncManager>().resolved_client() {
                        eprintln!(
                            "Google OAuth client: {} (from {})",
                            resolved.client.client_id,
                            resolved.source.describe()
                        );
                    }
                }
                Err(err) => eprintln!("could not read the Google OAuth client: {err}"),
            }

            notify::spawn(app.handle().clone());

            let show = MenuItem::with_id(app, "show", "Show checkd", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit checkd", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let tray = TrayIconBuilder::with_id("main")
                .icon(APP_ICON.clone())
                .tooltip("checkd")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app);

            if let Err(err) = tray {
                eprintln!("could not create tray icon: {err}");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if CLOSE_TO_TRAY.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::{MonthRepeat, MonthWeek, RepeatEnd, RepeatUnit};
    use crate::sync::auth::{MemoryStore, SecretStore};
    use crate::sync::google::{BoxFuture, GoogleClient, RemoteCalendar, RemoteEvent};
    use std::sync::Arc;
    use tauri::ipc::InvokeResponseBody;
    use tauri::test::{get_ipc_response, mock_context, noop_assets, MockRuntime};
    use tauri::webview::InvokeRequest;

    #[derive(Debug, Default)]
    struct FakeGoogle {
        calendars: std::sync::Mutex<Vec<RemoteCalendar>>,
        events: std::sync::Mutex<std::collections::HashMap<String, Vec<RemoteEvent>>>,
        next_id: std::sync::atomic::AtomicUsize,
    }

    impl FakeGoogle {
        fn fresh_id(&self, prefix: &str) -> String {
            let n = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            format!("{prefix}-{n}")
        }
    }

    impl GoogleClient for FakeGoogle {
        fn list_calendars<'a>(
            &'a self,
            access_token: &'a str,
        ) -> BoxFuture<'a, Result<Vec<RemoteCalendar>, String>> {
            Box::pin(async move {
                Ok(self
                    .list_calendars_delta(access_token, None)
                    .await?
                    .calendars)
            })
        }

        fn list_calendars_delta<'a>(
            &'a self,
            _access_token: &'a str,
            _sync_token: Option<&'a str>,
        ) -> BoxFuture<'a, Result<crate::sync::google::CalendarList, String>> {
            let cals = self
                .calendars
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            Box::pin(async move {
                Ok(crate::sync::google::CalendarList {
                    calendars: cals,
                    next_sync_token: Some("token".to_string()),
                })
            })
        }

        fn list_events<'a>(
            &'a self,
            access_token: &'a str,
            calendar_id: &'a str,
        ) -> BoxFuture<'a, Result<Vec<RemoteEvent>, String>> {
            Box::pin(async move {
                Ok(self
                    .list_events_delta(access_token, calendar_id, None)
                    .await?
                    .events)
            })
        }

        fn list_events_delta<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
            _sync_token: Option<&'a str>,
        ) -> BoxFuture<'a, Result<crate::sync::google::EventList, String>> {
            let evts = self
                .events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(calendar_id)
                .cloned()
                .unwrap_or_default();
            Box::pin(async move {
                Ok(crate::sync::google::EventList {
                    events: evts,
                    next_sync_token: Some("etoken".to_string()),
                })
            })
        }

        fn create_calendar<'a>(
            &'a self,
            _access_token: &'a str,
            calendar: &'a crate::sync::payload::CalendarPayload,
        ) -> BoxFuture<'a, Result<RemoteCalendar, String>> {
            let id = self.fresh_id("cal");
            let remote = RemoteCalendar {
                id,
                summary: Some(calendar.summary.clone()),
                description: Some(calendar.description.clone()),
                location: Some(calendar.location.clone()),
                hidden: true,
                deleted: false,
            };
            self.calendars
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(remote.clone());
            Box::pin(async move { Ok(remote) })
        }

        fn update_calendar<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
            calendar: &'a crate::sync::payload::CalendarPayload,
        ) -> BoxFuture<'a, Result<RemoteCalendar, String>> {
            let mut cals = self.calendars.lock().unwrap_or_else(|e| e.into_inner());
            let found = cals.iter_mut().find(|c| c.id == calendar_id && !c.deleted);
            let result = match found {
                Some(c) => {
                    c.summary = Some(calendar.summary.clone());
                    c.description = Some(calendar.description.clone());
                    c.location = Some(calendar.location.clone());
                    Ok(c.clone())
                }
                None => Err("404 Not Found".to_string()),
            };
            Box::pin(async move { result })
        }

        fn delete_calendar<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
        ) -> BoxFuture<'a, Result<(), String>> {
            let mut cals = self.calendars.lock().unwrap_or_else(|e| e.into_inner());
            let before = cals.len();
            cals.retain(|c| c.id != calendar_id);
            let result = if cals.len() == before {
                Err("404 Not Found".to_string())
            } else {
                self.events
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(calendar_id);
                Ok(())
            };
            Box::pin(async move { result })
        }

        fn hide_calendar<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
        ) -> BoxFuture<'a, Result<(), String>> {
            let mut cals = self.calendars.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(c) = cals.iter_mut().find(|c| c.id == calendar_id) {
                c.hidden = true;
            }
            Box::pin(async { Ok(()) })
        }

        fn create_event<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
            event: &'a crate::sync::payload::EventPayload,
        ) -> BoxFuture<'a, Result<RemoteEvent, String>> {
            let id = self.fresh_id("evt");
            let remote = RemoteEvent {
                id,
                ical_uid: Some(event.ical_uid.clone()),
                summary: Some(event.summary.clone()),
                description: Some(event.description.clone()),
                status: Some(event.status.clone()),
                updated: None,
            };
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(calendar_id.to_string())
                .or_default()
                .push(remote.clone());
            Box::pin(async move { Ok(remote) })
        }

        fn update_event<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
            event_id: &'a str,
            event: &'a crate::sync::payload::EventPayload,
        ) -> BoxFuture<'a, Result<RemoteEvent, String>> {
            let mut all_events = self.events.lock().unwrap_or_else(|e| e.into_inner());
            let found = all_events
                .get_mut(calendar_id)
                .and_then(|list| list.iter_mut().find(|e| e.id == event_id));
            let result = match found {
                Some(e) => {
                    e.ical_uid = Some(event.ical_uid.clone());
                    e.summary = Some(event.summary.clone());
                    e.description = Some(event.description.clone());
                    e.status = Some(event.status.clone());
                    Ok(e.clone())
                }
                None => Err("404 Not Found".to_string()),
            };
            Box::pin(async move { result })
        }

        fn delete_event<'a>(
            &'a self,
            _access_token: &'a str,
            calendar_id: &'a str,
            event_id: &'a str,
        ) -> BoxFuture<'a, Result<(), String>> {
            let mut all_events = self.events.lock().unwrap_or_else(|e| e.into_inner());
            let result = match all_events.get_mut(calendar_id) {
                Some(list) => {
                    let before = list.len();
                    list.retain(|e| e.id != event_id);
                    if list.len() == before {
                        Err("404 Not Found".to_string())
                    } else {
                        Ok(())
                    }
                }
                None => Err("404 Not Found".to_string()),
            };
            Box::pin(async move { result })
        }
    }

    struct Ipc {
        _app: tauri::App<MockRuntime>,
        webview: tauri::WebviewWindow<MockRuntime>,
        key: String,
        google: Arc<FakeGoogle>,
        secrets: Arc<MemoryStore>,
    }

    impl Ipc {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("checkd-test-ipc-{name}.db"));
            let _ = std::fs::remove_file(&path);
            let db = Db::open_path(&path).expect("open temp db");

            let google = Arc::new(FakeGoogle::default());
            let secrets = Arc::new(MemoryStore::default());

            let app = builder::<MockRuntime>(
                Some(db),
                SyncManager::with_stores(secrets.clone(), google.clone()),
            )
            .build(mock_context(noop_assets()))
            .expect("build app");
            let key = app.invoke_key().to_string();
            let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
                .build()
                .expect("build webview");

            Ipc {
                _app: app,
                webview,
                key,
                google,
                secrets,
            }
        }

        fn connect(&self) {
            let tokens = crate::sync::auth::Tokens {
                access_token: "fake-token".to_string(),
                refresh_token: Some("fake-refresh".to_string()),
                expires_at: chrono::Utc::now().timestamp() + 3600,
            };
            self.secrets
                .set("access-token", &serde_json::to_string(&tokens).unwrap())
                .unwrap();
            self.secrets.set("refresh-token", "fake-refresh").unwrap();
        }

        fn call(&self, cmd: &str, body: serde_json::Value) -> Result<InvokeResponseBody, serde_json::Value> {
            get_ipc_response(
                &self.webview,
                InvokeRequest {
                    cmd: cmd.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: self.key.clone(),
                },
            )
        }
    }

    #[test]
    fn frontend_commands_round_trip_through_sqlite() {
        let ipc = Ipc::new("roundtrip");

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "Groceries", "color": "#3b82f6" }),
        )
        .expect("create_list");

        ipc.call(
            "create_task",
            serde_json::json!({ "id": "t1", "listId": "l1", "title": "Milk" }),
        )
        .expect("create_task");

        ipc.call(
            "set_task_due",
            serde_json::json!({ "id": "t1", "dueDate": "2026-10-21", "dueTime": "18:00" }),
        )
        .expect("set_task_due");

        ipc.call(
            "set_task_repeat",
            serde_json::json!({
                "id": "t1",
                "repeat": {
                    "unit": "month",
                    "interval": 1,
                    "start": "2026-10-21",
                    "time": "09:00",
                    "weekdays": [],
                    "monthRepeat": { "type": "weekday", "weekday": 3, "week": "last" },
                    "end": { "type": "on", "date": "2027-01-01" }
                }
            }),
        )
        .expect("set_task_repeat");

        ipc.call(
            "set_task_due",
            serde_json::json!({ "id": "t1", "dueDate": null, "dueTime": null }),
        )
        .expect("clear due date");

        let response = ipc.call("load_data", serde_json::json!({})).expect("load_data");
        let data: AppData = response.deserialize().expect("deserialize app data");

        assert_eq!(data.lists.len(), 1);
        assert_eq!(data.lists[0].name, "Groceries");
        assert_eq!(data.lists[0].tasks.len(), 1);

        let task = &data.lists[0].tasks[0];
        assert_eq!(task.title, "Milk");
        assert_eq!(task.due_date, None);
        assert_eq!(task.due_time, None);

        let rule = task.repeat.as_ref().expect("repeat rule");
        assert_eq!(rule.unit, RepeatUnit::Month);
        assert_eq!(rule.interval, 1);
        assert_eq!(rule.time.as_deref(), Some("09:00"));
        assert!(matches!(
            &rule.month_repeat,
            Some(MonthRepeat::Weekday {
                week: MonthWeek::Last,
                ..
            })
        ));
        assert!(matches!(
            &rule.end,
            RepeatEnd::On { date } if date == "2027-01-01"
        ));
    }

    #[test]
    fn deleting_a_list_through_ipc_removes_its_tasks() {
        let ipc = Ipc::new("cascade");

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "List", "color": "#3b82f6" }),
        )
        .expect("create_list");
        ipc.call(
            "create_task",
            serde_json::json!({ "id": "t1", "listId": "l1", "title": "Milk" }),
        )
        .expect("create_task");
        ipc.call("delete_list", serde_json::json!({ "id": "l1" })).expect("delete_list");

        let response = ipc.call("load_data", serde_json::json!({})).expect("load_data");
        let data: AppData = response.deserialize().expect("deserialize app data");
        assert!(data.lists.is_empty());
    }

    #[test]
    fn setting_the_time_keeps_the_due_date() {
        let ipc = Ipc::new("time-modal");

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "Home", "color": "#3b82f6" }),
        )
        .expect("create_list");

        ipc.call(
            "create_task",
            serde_json::json!({ "id": "t1", "listId": "l1", "title": "Call mum" }),
        )
        .expect("create_task");

        ipc.call(
            "set_task_due",
            serde_json::json!({ "id": "t1", "dueDate": "2026-10-21", "dueTime": null }),
        )
        .expect("set_task_due");

        ipc.call(
            "set_task_due",
            serde_json::json!({ "id": "t1", "dueDate": "2026-10-21", "dueTime": "17:45" }),
        )
        .expect("set_task_due");

        let response = ipc.call("load_data", serde_json::json!({})).expect("load_data");
        let data: AppData = response.deserialize().expect("deserialize app data");
        let task = &data.lists[0].tasks[0];

        assert_eq!(task.due_date.as_deref(), Some("2026-10-21"));
        assert_eq!(task.due_time.as_deref(), Some("17:45"));
    }

    #[test]
    fn completing_a_repeating_task_through_ipc_advances_it() {
        let ipc = Ipc::new("complete");

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "Home", "color": "#3b82f6" }),
        )
        .expect("create_list");

        ipc.call(
            "create_task",
            serde_json::json!({ "id": "t1", "listId": "l1", "title": "Water plants" }),
        )
        .expect("create_task");

        ipc.call(
            "set_task_repeat",
            serde_json::json!({
                "id": "t1",
                "repeat": {
                    "unit": "day",
                    "interval": 1,
                    "start": "2030-01-01",
                    "time": "09:00",
                    "weekdays": [],
                    "monthRepeat": null,
                    "end": { "type": "never" }
                }
            }),
        )
        .expect("set_task_repeat");

        let response = ipc
            .call(
                "set_task_completed",
                serde_json::json!({ "id": "t1", "completed": true }),
            )
            .expect("set_task_completed");
        let task: Task = response.deserialize().expect("deserialize task");

        assert!(!task.completed);
        assert_eq!(task.next_occurrence.as_deref(), Some("2030-01-02"));
        assert_eq!(task.occurrence_index, 1);
    }

    #[test]
    fn the_sync_commands_start_out_disconnected() {
        let ipc = Ipc::new("sync");

        let response = ipc.call("google_status", serde_json::json!({})).expect("google_status");
        let status: SyncStatus = response.deserialize().expect("deserialize status");
        assert!(!status.configured);
        assert!(!status.connected);
        assert_eq!(status.pending, 0);

        let response = ipc
            .call("google_credentials_status", serde_json::json!({}))
            .expect("google_credentials_status");
        let credentials: CredentialsStatus = response.deserialize().expect("deserialize credentials");
        assert!(!credentials.configured);
        assert_eq!(credentials.client_id, None);

        let error = ipc
            .call("google_sync_now", serde_json::json!({}))
            .expect_err("sync without a connection");
        let message = error.to_string();
        assert!(
            message.contains("No Google account is connected"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn local_changes_show_up_as_pending() {
        let ipc = Ipc::new("pending");

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "Groceries", "color": "#3b82f6" }),
        )
        .expect("create_list");

        let response = ipc.call("google_status", serde_json::json!({})).expect("google_status");
        let status: SyncStatus = response.deserialize().expect("deserialize status");
        assert_eq!(status.pending, 1);
    }

    #[test]
    fn sync_now_pushes_lists_and_tasks_updates_and_deletes() {
        let ipc = Ipc::new("push");
        ipc.connect();

        ipc.call(
            "create_list",
            serde_json::json!({ "id": "l1", "name": "Groceries", "color": "#3b82f6" }),
        )
        .expect("create_list");
        ipc.call(
            "create_task",
            serde_json::json!({ "id": "t1", "listId": "l1", "title": "Milk" }),
        )
        .expect("create_task");

        let response = ipc
            .call("google_sync_now", serde_json::json!({}))
            .expect("google_sync_now");
        let status: SyncStatus = response.deserialize().expect("status");
        assert_eq!(status.pending, 0);

        {
            let cals = ipc.google.calendars.lock().unwrap();
            assert_eq!(cals.len(), 1);
            assert_eq!(cals[0].summary.as_deref(), Some("Groceries"));
            let cal_id = cals[0].id.clone();

            let evts = ipc.google.events.lock().unwrap();
            let list_evts = evts.get(&cal_id).expect("events for cal");
            assert_eq!(list_evts.len(), 1);
            assert_eq!(list_evts[0].summary.as_deref(), Some("Milk"));
            assert_eq!(list_evts[0].ical_uid.as_deref(), Some("t1"));
        }

        ipc.call(
            "rename_task",
            serde_json::json!({ "id": "t1", "title": "Oat milk" }),
        )
        .expect("rename_task");

        let response = ipc
            .call("google_sync_now", serde_json::json!({}))
            .expect("google_sync_now");
        let status: SyncStatus = response.deserialize().expect("status");
        assert_eq!(status.pending, 0);

        {
            let cals = ipc.google.calendars.lock().unwrap();
            let cal_id = cals[0].id.clone();
            let evts = ipc.google.events.lock().unwrap();
            let list_evts = evts.get(&cal_id).expect("events for cal");
            assert_eq!(list_evts.len(), 1);
            assert_eq!(list_evts[0].summary.as_deref(), Some("Oat milk"));
        }

        ipc.call("delete_task", serde_json::json!({ "id": "t1" }))
            .expect("delete_task");
        ipc.call("google_sync_now", serde_json::json!({}))
            .expect("google_sync_now");

        {
            let cals = ipc.google.calendars.lock().unwrap();
            let cal_id = cals[0].id.clone();
            let evts = ipc.google.events.lock().unwrap();
            let list_evts = evts.get(&cal_id).expect("events for cal");
            assert!(list_evts.is_empty());
        }

        ipc.call("delete_list", serde_json::json!({ "id": "l1" }))
            .expect("delete_list");
        ipc.call("google_sync_now", serde_json::json!({}))
            .expect("google_sync_now");

        assert!(ipc.google.calendars.lock().unwrap().is_empty());
    }

    #[test]
    fn sync_now_pulls_remote_lists_and_tasks_into_sqlite() {
        let ipc = Ipc::new("pull");
        ipc.connect();

        let list_payload = crate::sync::payload::list_payload_json(&crate::db::SyncList {
            id: "remote-l1".to_string(),
            name: "Work".to_string(),
            color: "#ef4444".to_string(),
            position: 1,
            updated_at: 5_000_000_000_000,
        });
        let task_payload = crate::sync::payload::task_payload_json(&crate::db::SyncTask {
            id: "remote-t1".to_string(),
            list_id: "remote-l1".to_string(),
            title: "Ship sync".to_string(),
            description: "Two-way".to_string(),
            due_date: Some("2026-10-25".to_string()),
            due_time: Some("14:00".to_string()),
            repeat: None,
            completed: false,
            next_occurrence: None,
            occurrence_index: 0,
            position: 1,
            updated_at: 5_000_000_000_000,
        });

        ipc.google.calendars.lock().unwrap().push(RemoteCalendar {
            id: "gcal-1".to_string(),
            summary: Some("Work".to_string()),
            description: Some(list_payload),
            location: Some("checkd".to_string()),
            hidden: true,
            deleted: false,
        });
        ipc.google
            .events
            .lock()
            .unwrap()
            .insert(
                "gcal-1".to_string(),
                vec![RemoteEvent {
                    id: "gevt-1".to_string(),
                    ical_uid: Some("remote-t1".to_string()),
                    summary: Some("Ship sync".to_string()),
                    description: Some(task_payload),
                    status: Some("confirmed".to_string()),
                    updated: None,
                }],
            );

        ipc.call("google_sync_now", serde_json::json!({}))
            .expect("google_sync_now");

        let response = ipc.call("load_data", serde_json::json!({})).expect("load_data");
        let data: AppData = response.deserialize().expect("app data");
        assert_eq!(data.lists.len(), 1);
        assert_eq!(data.lists[0].id, "remote-l1");
        assert_eq!(data.lists[0].name, "Work");
        assert_eq!(data.lists[0].color, "#ef4444");
        assert_eq!(data.lists[0].tasks.len(), 1);
        assert_eq!(data.lists[0].tasks[0].id, "remote-t1");
        assert_eq!(data.lists[0].tasks[0].title, "Ship sync");
        assert_eq!(data.lists[0].tasks[0].description, "Two-way");
        assert_eq!(
            data.lists[0].tasks[0].due_date.as_deref(),
            Some("2026-10-25")
        );
    }
}

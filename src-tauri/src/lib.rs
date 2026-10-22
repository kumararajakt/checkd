// Add Tauri commands here: https://tauri.app/develop/calling-rust/
// Remember to register each one in `tauri::generate_handler![...]` below.
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

/// When enabled, closing the window hides it instead of quitting the app.
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn set_close_to_tray(enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::Relaxed);
}

/// Writes the exported app data to `path`, creating parent directories if needed.
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

/// Sets (or clears) the repeat rule and returns the updated task — the active
/// occurrence is anchored to the rule.
#[tauri::command]
fn set_task_repeat(db: State<'_, Db>, id: String, repeat: Option<Repeat>) -> Result<Task, String> {
    db.set_task_repeat(&id, repeat.as_ref())
}

/// Marks a task done (or not done) and returns its updated state — a
/// repeating task advances to its next occurrence.
#[tauri::command]
fn set_task_completed(db: State<'_, Db>, id: String, completed: bool) -> Result<Task, String> {
    db.set_task_completed(&id, completed)
}

#[tauri::command]
fn delete_task(db: State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_task(&id)
}

/// The state of the Google sync — the settings dialog reads this.
#[tauri::command]
fn google_status(sync: State<'_, SyncManager>, db: State<'_, Db>) -> SyncStatus {
    sync.status(&db)
}

/// The OAuth client the app found in the environment — the secret itself is
/// never sent back.
#[tauri::command]
fn google_credentials_status(sync: State<'_, SyncManager>) -> Result<CredentialsStatus, String> {
    sync.credentials_status()
}

/// Runs the consent flow: the browser opens on Google's screen and the
/// answer comes back to a loopback listener.
#[tauri::command]
async fn google_sign_in<R: tauri::Runtime>(
    app: AppHandle<R>,
    sync: State<'_, SyncManager>,
    db: State<'_, Db>,
) -> Result<SyncStatus, String> {
    sync.sign_in(app, &db).await
}

/// Gives the access back to Google and forgets the tokens.
#[tauri::command]
async fn google_sign_out(
    sync: State<'_, SyncManager>,
    db: State<'_, Db>,
) -> Result<SyncStatus, String> {
    sync.sign_out(&db).await
}

/// Talks to Google now.
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

/// Builds the app with the shared plugins and command handlers.
///
/// `db` is managed up-front; pass `None` to open the real database in
/// `setup` (it needs the app handle to resolve the app-data dir).
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
    builder::<tauri::Wry>(None, SyncManager::new())
        .setup(|app| {
            app.manage(Db::open(app.handle())?);

            let data_dir = app.path().app_data_dir().ok();

            // Systems without a keyring keep the secrets in the app-data
            // directory instead.
            if let Some(dir) = data_dir.clone() {
                app.state::<SyncManager>().set_fallback_dir(dir);
            }

            // The OAuth client comes from the environment — the app asks for
            // no credentials.
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

            // Reminders keep firing while the window is hidden in the tray.
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

            // Not every desktop has a tray host — the app must still start.
            if let Err(err) = tray {
                eprintln!("could not create tray icon: {err}");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if CLOSE_TO_TRAY.load(Ordering::Relaxed) {
                    // Keep the app running in the tray.
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // No tray residency: closing the window quits the app.
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
    use crate::sync::auth::MemoryStore;
    use crate::sync::google::{BoxFuture, GoogleClient, RemoteCalendar, RemoteEvent};
    use std::sync::Arc;
    use tauri::ipc::InvokeResponseBody;
    use tauri::test::{get_ipc_response, mock_context, noop_assets, MockRuntime};
    use tauri::webview::InvokeRequest;

    /// A Google client that never talks to the network.
    #[derive(Debug, Default)]
    struct FakeGoogle;

    impl GoogleClient for FakeGoogle {
        fn list_calendars<'a>(
            &'a self,
            _access_token: &'a str,
        ) -> BoxFuture<'a, Result<Vec<RemoteCalendar>, String>> {
            Box::pin(async {
                Ok(vec![RemoteCalendar {
                    id: "cal-1".to_string(),
                    summary: Some("Groceries".to_string()),
                    location: Some("checkd".to_string()),
                    hidden: true,
                }])
            })
        }

        fn list_events<'a>(
            &'a self,
            _access_token: &'a str,
            _calendar_id: &'a str,
        ) -> BoxFuture<'a, Result<Vec<RemoteEvent>, String>> {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    /// A mock app + webview that speaks the same IPC protocol as the real one.
    struct Ipc {
        _app: tauri::App<MockRuntime>,
        webview: tauri::WebviewWindow<MockRuntime>,
        key: String,
    }

    impl Ipc {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("checkd-test-ipc-{name}.db"));
            let _ = std::fs::remove_file(&path);
            let db = Db::open_path(&path).expect("open temp db");

            let app = builder::<MockRuntime>(
                Some(db),
                SyncManager::with_stores(Arc::new(MemoryStore::default()), Arc::new(FakeGoogle)),
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
            }
        }

        /// Calls a command the way the frontend's `invoke` does.
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

    /// The payloads below are exactly what `src/db.ts` sends.
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

        // Null clears the optional columns.
        ipc.call(
            "set_task_due",
            serde_json::json!({ "id": "t1", "dueDate": null, "dueTime": null }),
        )
        .expect("clear due date");

        // The frontend reads everything back with load_data.
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

    /// The time modal writes the picked time together with the date the task
    /// already had — the date must survive the write.
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

        // The modal sends the picked time along with the date it already had.
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

    /// Completing a repeating task through the IPC advances the series.
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

        // The completed occurrence moved on to the next day. The rule starts
        // in the future, so the test does not depend on today's date.
        assert!(!task.completed);
        assert_eq!(task.next_occurrence.as_deref(), Some("2030-01-02"));
        assert_eq!(task.occurrence_index, 1);
    }

    /// The sync commands answer before a Google account is connected — and
    /// the credentials never leave the environment.
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

        // A sync without a connection fails with a readable message.
        let error = ipc
            .call("google_sync_now", serde_json::json!({}))
            .expect_err("sync without a connection");
        let message = error.to_string();
        assert!(
            message.contains("No Google account is connected"),
            "unexpected error: {message}"
        );
    }

    /// Local changes wait in the outbox until they are pushed.
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
}

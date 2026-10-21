// Add Tauri commands here: https://tauri.app/develop/calling-rust/
// Remember to register each one in `tauri::generate_handler![...]` below.
pub mod db;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State, WindowEvent};

use crate::db::{AppData, Db, Repeat};

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

#[tauri::command]
fn set_task_repeat(db: State<'_, Db>, id: String, repeat: Option<Repeat>) -> Result<(), String> {
    db.set_task_repeat(&id, repeat.as_ref())
}

#[tauri::command]
fn delete_task(db: State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_task(&id)
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
fn builder<R: tauri::Runtime>(db: Option<Db>) -> tauri::Builder<R> {
    let builder = tauri::Builder::<R>::new()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            delete_task,
        ]);

    match db {
        Some(db) => builder.manage(db),
        None => builder,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    builder::<tauri::Wry>(None)
        .setup(|app| {
            app.manage(Db::open(app.handle())?);

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
    use tauri::ipc::InvokeResponseBody;
    use tauri::test::{get_ipc_response, mock_context, noop_assets, MockRuntime};
    use tauri::webview::InvokeRequest;

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

            let app = builder::<MockRuntime>(Some(db))
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
}

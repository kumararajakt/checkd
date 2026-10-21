// Add Tauri commands here: https://tauri.app/develop/calling-rust/
// Remember to register each one in `tauri::generate_handler![...]` below.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

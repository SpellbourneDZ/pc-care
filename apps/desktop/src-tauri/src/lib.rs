mod api;
use tauri::Manager;
#[path = "../domain/mod.rs"]
pub mod domain;
#[path = "../win/mod.rs"]
pub mod win;

pub fn run() {
    tauri::Builder::default()
        .manage(domain::engine::Engine::new(cfg!(debug_assertions)))
        .setup(|app| {
            #[cfg(not(feature = "qa-fixtures"))]
            let root = app.path().app_config_dir().ok();
            #[cfg(feature = "qa-fixtures")]
            let root = win::adapters::preferences::qa_directory();
            app.manage(win::adapters::preferences::SettingsStore::new(root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api::operation_api])
        .run(tauri::generate_context!())
        .expect("PC Care could not start");
}

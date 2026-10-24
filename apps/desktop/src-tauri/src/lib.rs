mod api;
#[path = "../domain/mod.rs"]
pub mod domain;
#[path = "../win/mod.rs"]
pub mod win;

pub fn run() {
    tauri::Builder::default()
        .manage(domain::engine::Engine::new(cfg!(debug_assertions)))
        .invoke_handler(tauri::generate_handler![api::operation_api])
        .run(tauri::generate_context!())
        .expect("PC Care could not start");
}

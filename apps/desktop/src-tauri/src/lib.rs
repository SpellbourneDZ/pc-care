#[path = "../domain/mod.rs"]
pub mod domain;
#[path = "../win/mod.rs"]
pub mod win;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("PC Care could not start");
}

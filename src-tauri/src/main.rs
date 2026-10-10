mod commands;

use std::{fs, sync::Mutex};

use hungerstrike_core::Store;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            fs::create_dir_all(&directory)?;
            let store = Store::open(&directory.join("hungerstrike.sqlite"))?;
            app.manage(commands::AppStore(Mutex::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_text,
            commands::confirm_reviewed,
            commands::get_snapshot,
            commands::inventory_action
        ])
        .run(tauri::generate_context!())
        .expect("HungerStrike could not start");
}

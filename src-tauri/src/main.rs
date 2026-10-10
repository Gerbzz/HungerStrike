mod commands;
mod ocr;

use std::{fs, sync::Mutex};

use hungerstrike_core::Store;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            fs::create_dir_all(&directory)?;
            let store = Store::open(&directory.join("hungerstrike.sqlite"))?;
            app.manage(commands::AppStore(Mutex::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_text,
            commands::scan_photo,
            commands::confirm_reviewed,
            commands::get_snapshot,
            commands::inventory_action,
            commands::get_meals,
            commands::get_budget,
            commands::save_budget,
            commands::edit_category,
            commands::export_backup_file,
            commands::preview_backup,
            commands::apply_backup,
            commands::undo_last_replace
        ])
        .run(tauri::generate_context!())
        .expect("HungerStrike could not start");
}

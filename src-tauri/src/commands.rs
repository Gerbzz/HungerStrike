use std::sync::Mutex;

use crate::ocr;
use hungerstrike_core::{
    confirm_receipt, parse_text, record_inventory_action, review, snapshot, AppSnapshot,
    ConfirmOutcome, InventoryAction, ReceiptDraft, ReviewedReceipt, Store,
};
use serde::Serialize;
use tauri::State;

pub struct AppStore(pub Mutex<Store>);

#[tauri::command]
pub fn import_text(raw: String) -> ReceiptDraft {
    parse_text(&raw)
}

#[derive(Serialize)]
pub struct PhotoScan {
    pub text: String,
    pub draft: ReceiptDraft,
}

#[tauri::command]
pub fn scan_photo(bytes: Vec<u8>) -> Result<PhotoScan, String> {
    let text =
        ocr::extract_local_image(&bytes, &ocr::installed_engine()).map_err(|e| e.to_string())?;
    let draft = parse_text(&text);
    Ok(PhotoScan { text, draft })
}

#[tauri::command]
pub fn confirm_reviewed(
    reviewed: ReceiptDraft,
    state: State<'_, AppStore>,
) -> Result<ConfirmOutcome, String> {
    let reviewed: ReviewedReceipt = review(reviewed).map_err(|e| e.to_string())?;
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    confirm_receipt(&mut store, reviewed)
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppStore>) -> Result<AppSnapshot, String> {
    let store = state.0.lock().map_err(|e| e.to_string())?;
    snapshot(&store)
}

#[tauri::command]
pub fn inventory_action(
    action: InventoryAction,
    state: State<'_, AppStore>,
) -> Result<AppSnapshot, String> {
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    record_inventory_action(&mut store, action)?;
    snapshot(&store)
}

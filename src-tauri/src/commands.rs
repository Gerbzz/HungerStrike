use std::{io::Write, sync::Mutex};

use crate::ocr;
use hungerstrike_core::{
    apply_import, budget_view, confirm_receipt, export_backup, parse_text, preview_import,
    recategorize_item, record_inventory_action, restore_last_import, review, set_budget, snapshot,
    suggest_meals, AppSnapshot, ApplyOutcome, BudgetView, ConfirmOutcome, ImportPreview,
    InventoryAction, MealSuggestion, Period, ReceiptDraft, RecipeCollection, ReplaceOrReconcile,
    ReviewedReceipt, Store,
};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

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

#[tauri::command]
pub fn get_meals(today: String, state: State<'_, AppStore>) -> Result<Vec<MealSuggestion>, String> {
    let store = state.0.lock().map_err(|e| e.to_string())?;
    let inventory = snapshot(&store)?;
    let recipes: RecipeCollection = serde_json::from_str(include_str!("../../assets/recipes.json"))
        .map_err(|e| e.to_string())?;
    suggest_meals(&inventory, &recipes.recipes, &today)
}

#[tauri::command]
pub fn get_budget(
    reference_date: String,
    period: Period,
    week_start: u8,
    state: State<'_, AppStore>,
) -> Result<BudgetView, String> {
    let store = state.0.lock().map_err(|e| e.to_string())?;
    budget_view(&store, &reference_date, period, week_start)
}

#[tauri::command]
pub fn save_budget(
    period: Period,
    cents: Option<i64>,
    state: State<'_, AppStore>,
) -> Result<(), String> {
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    set_budget(&mut store, period, cents)
}

#[tauri::command]
pub fn edit_category(
    line_id: String,
    category: String,
    state: State<'_, AppStore>,
) -> Result<AppSnapshot, String> {
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    recategorize_item(&mut store, &line_id, &category)?;
    snapshot(&store)
}

#[tauri::command]
pub fn export_backup_file(
    app: AppHandle,
    state: State<'_, AppStore>,
) -> Result<Option<String>, String> {
    let bytes = {
        let store = state.0.lock().map_err(|e| e.to_string())?;
        export_backup(&store)?.to_bytes()?
    };
    let chosen = app
        .dialog()
        .file()
        .add_filter("HungerStrike backup", &["json"])
        .blocking_save_file();
    let Some(chosen) = chosen else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or("backup destination has no parent folder")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary.write_all(&bytes).map_err(|e| e.to_string())?;
    temporary.flush().map_err(|e| e.to_string())?;
    temporary
        .persist_noclobber(&path)
        .map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub fn preview_backup(bytes: Vec<u8>, state: State<'_, AppStore>) -> Result<ImportPreview, String> {
    let store = state.0.lock().map_err(|e| e.to_string())?;
    preview_import(&store, &bytes)
}

#[derive(Serialize)]
pub struct BackupApplyResult {
    outcome: ApplyOutcome,
    snapshot: AppSnapshot,
}

#[tauri::command]
pub fn apply_backup(
    bytes: Vec<u8>,
    mode: ReplaceOrReconcile,
    state: State<'_, AppStore>,
) -> Result<BackupApplyResult, String> {
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    let outcome = apply_import(&mut store, &bytes, mode)?;
    Ok(BackupApplyResult {
        outcome,
        snapshot: snapshot(&store)?,
    })
}

#[tauri::command]
pub fn undo_last_replace(state: State<'_, AppStore>) -> Result<AppSnapshot, String> {
    let mut store = state.0.lock().map_err(|e| e.to_string())?;
    restore_last_import(&mut store)?;
    snapshot(&store)
}

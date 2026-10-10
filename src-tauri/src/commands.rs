use std::sync::Mutex;

use crate::ocr;
use hungerstrike_core::{
    budget_view, confirm_receipt, parse_text, recategorize_item, record_inventory_action, review,
    set_budget, snapshot, suggest_meals, AppSnapshot, BudgetView, ConfirmOutcome, InventoryAction,
    MealSuggestion, Period, ReceiptDraft, RecipeCollection, ReviewedReceipt, Store,
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

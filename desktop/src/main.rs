mod ocr;
mod platform;
mod review_hints;
mod scene;
mod theme;
mod ui;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use hungerstrike_core::{
    add_recipe_missing_to_shopping, add_recipes_missing_to_shopping, apply_import, budget_view,
    confirm_receipt, confirm_separate_purchase, cook_recipe, correct_receipt, estimate_expiry,
    export_backup, matching_source_receipt, parse_text, parse_user_date, preview_import,
    record_inventory_action, remove_shopping_item, restore_last_import, review, set_budget,
    set_ui_theme, snapshot, ui_theme, AppSnapshot, ImportPreview, InventoryAction, Period,
    ReceiptDraft, Recipe, RecipeCollection, ReplaceOrReconcile, Store,
};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Kitchen,
    Receipt,
    Meals,
    Spending,
    Data,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MealFilter {
    CookNow,
    UseSoon,
    PlanNext,
}

#[derive(Clone, Copy)]
enum ReceiptCommit {
    New,
    Separate,
    Correct,
}

enum AppAction {
    Navigate(Tab),
    SelectZone(String),
    SelectLot(String),
    Inventory(InventoryAction),
    CookRecipe(Recipe),
    AddMissingToShopping(Recipe),
    AddMissingBatch(Vec<Recipe>),
    RemoveShopping {
        name: String,
        unit: String,
    },
    ParseReceipt,
    StartOcr(Vec<u8>),
    /// Estimate expiry for one draft line (`Some(index)`) or all undated lines.
    EstimateExpiry(Option<usize>),
    Commit(ReceiptCommit),
    CancelReceipt,
    PreviewBackup(Vec<u8>),
    ApplyBackup(ReplaceOrReconcile),
    UndoBackup,
    SetTheme(theme::ThemePref),
}

struct OcrTask {
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<Result<String, ocr::OcrError>>,
}

#[derive(Resource)]
struct AppState {
    store: Mutex<Store>,
    snapshot: AppSnapshot,
    tab: Tab,
    draft: Option<ReceiptDraft>,
    raw_text: String,
    message: String,
    zone: String,
    lot_id: String,
    quantity: String,
    expiry_text: String,
    /// Set for the isolated synthetic capture routes; shows a persistent label.
    synthetic_banner: bool,
    /// Saved theme choice (Light by default).
    theme_pref: theme::ThemePref,
    /// Operating-system preference reported by the window system, if any.
    system_dark: Option<bool>,
    /// Theme in effect this frame.
    dark: bool,
    /// Theme the egui style was last built for.
    applied_dark: Option<bool>,
    category: String,
    reference_date: String,
    week_start: u8,
    weekly_budget: i64,
    monthly_budget: i64,
    weekly_budget_text: String,
    monthly_budget_text: String,
    backup_bytes: Option<Vec<u8>>,
    backup_preview: Option<ImportPreview>,
    source_match: Option<(String, String)>,
    separate_ack: bool,
    ocr_task: Option<OcrTask>,
    style_ready: bool,
    startup_complete: bool,
    show_about: bool,
    meal_filter: MealFilter,
    selected_recipe_id: Option<String>,
    batch_recipe_ids: HashSet<String>,
    hold_launch_for_capture: bool,
    dormouse_logo: Option<bevy_egui::egui::TextureHandle>,
    pwds_logo: Option<bevy_egui::egui::TextureHandle>,
    scene_dirty: bool,
    camera_yaw: f32,
    camera_distance: f32,
}

impl AppState {
    fn new(store: Store, today: String) -> Result<Self, String> {
        let snapshot = snapshot(&store)?;
        let weekly_budget = budget_view(&store, &today, Period::Week, 0)
            .ok()
            .and_then(|view| view.budget_cents)
            .unwrap_or(0);
        let theme_pref = theme::ThemePref::parse(ui_theme(&store)?.as_deref());
        let monthly_budget = budget_view(&store, &today, Period::Month, 0)
            .ok()
            .and_then(|view| view.budget_cents)
            .unwrap_or(0);
        Ok(Self {
            store: Mutex::new(store),
            snapshot,
            tab: Tab::Kitchen,
            draft: None,
            raw_text: String::new(),
            message: String::new(),
            zone: "all".into(),
            lot_id: String::new(),
            quantity: "1".into(),
            expiry_text: String::new(),
            synthetic_banner: false,
            theme_pref,
            system_dark: None,
            dark: theme::resolve_dark(theme_pref, None),
            applied_dark: None,
            category: String::new(),
            reference_date: today,
            week_start: 0,
            weekly_budget,
            monthly_budget,
            weekly_budget_text: format!("${}.{:02}", weekly_budget / 100, weekly_budget % 100),
            monthly_budget_text: format!("${}.{:02}", monthly_budget / 100, monthly_budget % 100),
            backup_bytes: None,
            backup_preview: None,
            source_match: None,
            separate_ack: false,
            ocr_task: None,
            style_ready: false,
            startup_complete: false,
            show_about: false,
            meal_filter: MealFilter::CookNow,
            selected_recipe_id: None,
            batch_recipe_ids: HashSet::new(),
            hold_launch_for_capture: false,
            dormouse_logo: None,
            pwds_logo: None,
            scene_dirty: true,
            camera_yaw: 0.36,
            camera_distance: 8.8,
        })
    }

    fn refresh(&mut self) -> Result<(), String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        self.snapshot = snapshot(&store)?;
        if !self.lot_id.is_empty() {
            if let Some(lot) = self.snapshot.lots.iter().find(|lot| lot.id == self.lot_id) {
                self.zone = lot.zone.clone();
            } else {
                self.lot_id.clear();
            }
        }
        if let Ok(view) = budget_view(&store, &self.reference_date, Period::Week, self.week_start) {
            self.weekly_budget = view.budget_cents.unwrap_or(0);
            self.weekly_budget_text = format!(
                "${}.{:02}",
                self.weekly_budget / 100,
                self.weekly_budget % 100
            );
        }
        if let Ok(view) = budget_view(&store, &self.reference_date, Period::Month, self.week_start)
        {
            self.monthly_budget = view.budget_cents.unwrap_or(0);
            self.monthly_budget_text = format!(
                "${}.{:02}",
                self.monthly_budget / 100,
                self.monthly_budget % 100
            );
        }
        self.scene_dirty = true;
        Ok(())
    }
    fn export_backup_bytes(&self) -> Result<Vec<u8>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        export_backup(&store)?.to_bytes()
    }
    fn clear_receipt(&mut self) {
        if let Some(task) = self.ocr_task.take() {
            task.cancel.store(true, Ordering::Relaxed);
        }
        self.draft = None;
        self.raw_text.clear();
        self.source_match = None;
        self.separate_ack = false;
    }

    fn set_draft_from_text(&mut self, text: String) -> Result<(), String> {
        let draft = parse_text(&text);
        self.source_match = {
            let store = self.store.lock().map_err(|e| e.to_string())?;
            matching_source_receipt(&store, &draft.source_sha256)?
        };
        self.raw_text = text;
        self.draft = Some(draft);
        self.separate_ack = false;
        Ok(())
    }

    /// Mark every purchase line in the draft as checked. The review screen
    /// does this one line at a time; synthetic capture routes and tests call
    /// it to stand in for the person doing the review.
    /// Recompute the theme in effect from the saved choice and the OS.
    fn update_theme(&mut self) {
        let dark = theme::resolve_dark(self.theme_pref, self.system_dark);
        if dark != self.dark {
            self.dark = dark;
            self.scene_dirty = true;
        }
    }

    fn mark_all_reviewed(&mut self) {
        if let Some(draft) = self.draft.as_mut() {
            for item in &mut draft.items {
                item.reviewed = true;
            }
        }
    }

    /// Fill a clearly flagged expiry estimate for one draft line, or for every
    /// food line that has no date yet. Never overwrites a date a person typed.
    fn estimate_draft_expiry(&mut self, index: Option<usize>) -> Result<usize, String> {
        let draft = self.draft.as_mut().ok_or("no receipt to review")?;
        let purchase_date = draft
            .purchase_date
            .clone()
            .filter(|date| parse_user_date(date).is_ok())
            .ok_or("enter the purchase date first; estimates count from it")?;
        let mut filled = 0;
        for (at, item) in draft.items.iter_mut().enumerate() {
            if index.is_some_and(|wanted| wanted != at) {
                continue;
            }
            if index.is_none() && item.expiry_date.is_some() {
                continue;
            }
            if let Some(estimate) =
                estimate_expiry(&item.name, &item.category, None, &purchase_date)
            {
                item.expiry_date = Some(estimate.date);
                item.expiry_estimated = true;
                filled += 1;
            }
        }
        Ok(filled)
    }

    fn poll_ocr(&mut self) {
        if !self
            .ocr_task
            .as_ref()
            .is_some_and(|task| task.handle.is_finished())
        {
            return;
        }
        let task = self.ocr_task.take().expect("finished OCR task");
        match task.handle.join() {
            Ok(Ok(text)) => {
                self.message = self
                    .set_draft_from_text(text)
                    .map(|_| {
                        "OCR draft ready. Check every line against the photo before confirming."
                            .into()
                    })
                    .unwrap_or_else(|e| e);
            }
            Ok(Err(error)) => self.message = format!("{error}. Paste or type the receipt instead."),
            Err(_) => {
                self.message =
                    "Offline OCR stopped unexpectedly. Paste or type the receipt instead.".into()
            }
        }
    }

    fn dispatch(&mut self, action: AppAction) -> Result<(), String> {
        match action {
            AppAction::Navigate(tab) => self.tab = tab,
            AppAction::SelectZone(zone) => {
                if !["all", "fridge", "freezer", "pantry"].contains(&zone.as_str()) {
                    return Err("unknown kitchen zone".into());
                }
                self.zone = zone;
                self.tab = Tab::Kitchen;
                self.lot_id.clear();
                self.scene_dirty = true;
            }
            AppAction::SelectLot(id) => {
                let lot = self
                    .snapshot
                    .lots
                    .iter()
                    .find(|lot| lot.id == id)
                    .ok_or("stock lot not found")?;
                self.zone = lot.zone.clone();
                self.expiry_text = lot.expiry_date.clone().unwrap_or_default();
                self.lot_id = id;
                self.quantity = "1".into();
                self.tab = Tab::Kitchen;
                self.scene_dirty = true;
            }
            AppAction::Inventory(action) => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    record_inventory_action(&mut store, action)?;
                }
                self.refresh()?;
                self.message = "Inventory updated.".into();
            }
            AppAction::CookRecipe(recipe) => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    cook_recipe(&mut store, &recipe, &self.reference_date)?;
                }
                self.refresh()?;
                self.message = format!("Cooked {}. Confirmed stock was updated.", recipe.name);
            }
            AppAction::AddMissingToShopping(recipe) => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    add_recipe_missing_to_shopping(&mut store, &recipe, &self.reference_date)?;
                }
                self.refresh()?;
                self.message = format!(
                    "Missing ingredients for {} added to your local shopping list.",
                    recipe.name
                );
            }
            AppAction::AddMissingBatch(recipes) => {
                let before = self.snapshot.shopping.clone();
                let shopping = {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    add_recipes_missing_to_shopping(&mut store, &recipes, &self.reference_date)?
                };
                self.refresh()?;
                self.meal_filter = MealFilter::PlanNext;
                self.selected_recipe_id = None;
                self.message = if shopping == before {
                    "Shopping list unchanged; any missing groceries are already listed.".into()
                } else {
                    format!(
                        "Combined grocery needs for {} selected meals saved. Larger existing shopping quantities were retained.",
                        recipes.len()
                    )
                };
            }
            AppAction::RemoveShopping { name, unit } => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    remove_shopping_item(&mut store, &name, &unit)?;
                }
                self.refresh()?;
                self.message = "Shopping item removed.".into();
            }
            AppAction::ParseReceipt => {
                self.set_draft_from_text(self.raw_text.clone())?;
                self.message = "Review every field, including the date and uncertain lines.".into();
            }
            AppAction::StartOcr(bytes) => {
                if self.ocr_task.is_some() {
                    return Err(
                        "OCR is already running; cancel it before starting another image".into(),
                    );
                }
                let cancel = Arc::new(AtomicBool::new(false));
                let thread_cancel = cancel.clone();
                let handle = std::thread::spawn(move || {
                    ocr::extract_local_image_with_cancel(&bytes, &thread_cancel)
                });
                self.ocr_task = Some(OcrTask { cancel, handle });
                self.message = "Reading the photo on this computer. You can cancel.".into();
            }
            AppAction::EstimateExpiry(index) => {
                let filled = self.estimate_draft_expiry(index)?;
                self.message = if filled == 0 {
                    "No estimate available; the expiry stays unknown until you enter it.".into()
                } else {
                    format!("Estimated expiry added to {filled} line(s). Estimates are labelled; check the package.")
                };
            }
            AppAction::Commit(mode) => {
                let draft = self.draft.clone().ok_or("no receipt to confirm")?;
                if matches!(mode, ReceiptCommit::Separate) && !self.separate_ack {
                    return Err("acknowledge that this is a separate purchase first".into());
                }
                let unchecked = draft.items.iter().filter(|item| !item.reviewed).count();
                if unchecked > 0 {
                    return Err(format!(
                        "check every line before confirming ({unchecked} not checked yet)"
                    ));
                }
                let reviewed = review(draft).map_err(|e| e.to_string())?;
                let outcome = {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    match mode {
                        ReceiptCommit::New => confirm_receipt(&mut store, reviewed),
                        ReceiptCommit::Separate => confirm_separate_purchase(&mut store, reviewed),
                        ReceiptCommit::Correct => correct_receipt(&mut store, reviewed),
                    }
                }?;
                self.refresh()?;
                self.clear_receipt();
                self.message = format!("Receipt: {outcome:?}");
            }
            AppAction::CancelReceipt => {
                self.clear_receipt();
                self.message = "Temporary receipt review and OCR cleared.".into();
            }
            AppAction::PreviewBackup(bytes) => {
                let preview = {
                    let store = self.store.lock().map_err(|e| e.to_string())?;
                    preview_import(&store, &bytes)?
                };
                self.backup_bytes = Some(bytes);
                self.backup_preview = Some(preview);
                self.message = "Backup preview loaded; review before applying.".into();
            }
            AppAction::ApplyBackup(mode) => {
                let bytes = self
                    .backup_bytes
                    .as_deref()
                    .ok_or("open a backup preview first")?;
                let result = {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    apply_import(&mut store, bytes, mode)?
                };
                self.refresh()?;
                self.backup_bytes = None;
                self.backup_preview = None;
                self.message = format!("Backup: {result:?}");
            }
            AppAction::SetTheme(pref) => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    set_ui_theme(&mut store, pref.as_str())?;
                }
                self.theme_pref = pref;
                self.update_theme();
                self.message = format!("Theme set to {}.", pref.label());
            }
            AppAction::UndoBackup => {
                {
                    let mut store = self.store.lock().map_err(|e| e.to_string())?;
                    restore_last_import(&mut store)?;
                }
                self.refresh()?;
                self.message = "Replacement undone.".into();
            }
        }
        Ok(())
    }
}

fn seed_unknown_visual_capture(state: &mut AppState) -> Result<(), String> {
    state.raw_text = include_str!("../../tests/fixtures/visual-unknown-items.txt").into();
    state.dispatch(AppAction::ParseReceipt)?;
    let draft = state.draft.as_mut().ok_or("visual fixture did not parse")?;
    for item in &mut draft.items {
        item.category = match item.name.as_str() {
            "apple juice" => "food.beverage",
            "fresh tomatoes" => "food.produce",
            name => return Err(format!("unexpected visual fixture item: {name}")),
        }
        .into();
        // A person has explicitly resolved the category of each unfamiliar
        // synthetic line before the demo writes inventory.
        item.uncertainty.retain(|flag| flag != "item unrecognized");
    }
    state.mark_all_reviewed();
    state.dispatch(AppAction::Commit(ReceiptCommit::New))?;
    let juice_id = state
        .snapshot
        .lots
        .iter()
        .find(|lot| lot.name == "apple juice")
        .ok_or("visual fixture juice lot missing")?
        .id
        .clone();
    state.dispatch(AppAction::Inventory(InventoryAction::Move {
        lot_id: juice_id.clone(),
        zone: "fridge".into(),
    }))?;
    state.dispatch(AppAction::SelectZone("fridge".into()))?;
    state.dispatch(AppAction::SelectLot(juice_id))?;
    state.message = "SYNTHETIC TEST DATA - unknown product shapes in isolated local profile".into();
    Ok(())
}

fn main() {
    let path = platform::data_path();
    std::fs::create_dir_all(path.parent().expect("data folder")).expect("create local data folder");
    // Earlier macOS/Linux builds kept data under the working directory. Copy
    // it to the per-OS location once; the original files are left in place.
    let migrated_from = match platform::legacy_data_path() {
        Some(legacy) => match platform::migrate_legacy_data(&legacy, &path) {
            Ok(true) => Some(legacy),
            Ok(false) => None,
            Err(error) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("HungerStrike could not migrate local data")
                    .set_description(format!(
                        "{error}\n\nThe original kitchen at {} was kept. No new kitchen will be opened. Close any older HungerStrike instance and retry. Destination: {}",
                        legacy.display(), path.display()
                    ))
                    .show();
                std::process::exit(1);
            }
        },
        None => None,
    };
    let (store, recovery) = match Store::open_or_recover(&path) {
        Ok(opened) => opened,
        Err(error) => {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("HungerStrike could not open local data")
                .set_description(format!("{error}\n\nData folder: {}", path.display()))
                .show();
            std::process::exit(1);
        }
    };
    let demo = std::env::var("HUNGERSTRIKE_CAPTURE_DEMO").ok();
    let today = if demo.is_some() {
        "2026-10-04".into()
    } else {
        chrono::Local::now().date_naive().to_string()
    };
    let mut state = AppState::new(store, today).expect("read local inventory");
    if let Some(note) = recovery {
        state.message = format!(
            "Your saved kitchen could not be read ({}), so a fresh one was started. The old file was kept, not deleted, as {} in the HungerStrike data folder.",
            note.reason,
            note.preserved_copy
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
    }
    if let Some(legacy) = migrated_from {
        state.message = format!(
            "Your kitchen was copied from {} to {}. The original files were left where they were.",
            legacy.display(),
            path.display()
        );
    }
    state.hold_launch_for_capture = demo.as_deref() == Some("launch");
    state.synthetic_banner = demo.is_some();
    // Capture routes can preview a theme without saving it.
    if let Ok(theme) = std::env::var("HUNGERSTRIKE_CAPTURE_THEME") {
        state.theme_pref = theme::ThemePref::parse(Some(theme.as_str()));
        state.update_theme();
    }
    if demo.as_deref() == Some("receipt-photo") {
        // Runs the bundled OCR on the SYNTHETIC labelled photo fixture; the
        // draft is shown for review and nothing is written.
        state
            .dispatch(AppAction::StartOcr(
                include_bytes!("../../tests/images/synthetic-receipt-photo.jpg").to_vec(),
            ))
            .expect("start synthetic OCR");
        state
            .dispatch(AppAction::Navigate(Tab::Receipt))
            .expect("show receipt");
    } else if demo.as_deref() == Some("receipt") {
        state.raw_text =
            include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt").into();
        state
            .dispatch(AppAction::ParseReceipt)
            .expect("parse demo receipt");
        state
            .dispatch(AppAction::Navigate(Tab::Receipt))
            .expect("show receipt");
        state.message = "SYNTHETIC TEST DATA — review only; no inventory write".into();
    } else if demo.as_deref() == Some("unknowns") {
        seed_unknown_visual_capture(&mut state).expect("seed synthetic unknown-item capture");
    } else if matches!(
        demo.as_deref(),
        Some(
            "kitchen"
                | "pantry"
                | "freezer"
                | "all"
                | "selected"
                | "meals"
                | "spending"
                | "about"
                | "plan-next"
                | "plan-added"
                | "cooked"
                | "data"
        )
    ) {
        state.raw_text =
            include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt").into();
        state
            .dispatch(AppAction::ParseReceipt)
            .expect("parse synthetic stock");
        if let Some(draft) = state.draft.as_mut() {
            for item in &mut draft.items {
                if item.name == "spinach" {
                    item.expiry_date = Some("2026-10-06".into());
                }
                if item.name == "milk 2%" {
                    item.expiry_date = Some("2026-10-07".into());
                }
                if item.name == "large eggs" {
                    // Synthetic opt-in estimate so captures show the label.
                    item.expiry_date = Some("2026-10-24".into());
                    item.expiry_estimated = true;
                }
            }
        }
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .expect("seed synthetic stock");
        state.raw_text = include_str!("../../core/tests/fixtures/receipts/R002-refund.txt").into();
        state
            .dispatch(AppAction::ParseReceipt)
            .expect("parse synthetic second receipt");
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .expect("seed synthetic spending");
        {
            let mut store = state.store.lock().expect("synthetic store lock");
            set_budget(&mut store, Period::Week, Some(4000)).expect("synthetic weekly budget");
            set_budget(&mut store, Period::Month, Some(12000)).expect("synthetic monthly budget");
        }
        state.refresh().expect("refresh synthetic state");
        if matches!(demo.as_deref(), Some("plan-added" | "cooked")) {
            let collection: RecipeCollection =
                serde_json::from_str(include_str!("../../assets/recipes.json"))
                    .expect("read bundled synthetic recipe");
            let id = if demo.as_deref() == Some("plan-added") {
                "tomato-pasta"
            } else {
                "egg-spinach-scramble"
            };
            let recipe = collection
                .recipes
                .into_iter()
                .find(|recipe| recipe.id == id)
                .expect("find bundled synthetic recipe");
            state
                .dispatch(if demo.as_deref() == Some("plan-added") {
                    AppAction::AddMissingToShopping(recipe)
                } else {
                    AppAction::CookRecipe(recipe)
                })
                .expect("apply synthetic recipe action");
        }
        let tab = match demo.as_deref() {
            Some("meals" | "plan-next" | "plan-added") => Tab::Meals,
            Some("spending") => Tab::Spending,
            Some("data") => Tab::Data,
            _ => Tab::Kitchen,
        };
        state
            .dispatch(AppAction::Navigate(tab))
            .expect("show synthetic page");
        if tab == Tab::Kitchen {
            state
                .dispatch(AppAction::SelectZone(
                    match demo.as_deref() {
                        Some("pantry") => "pantry",
                        Some("freezer") => "freezer",
                        Some("all") => "all",
                        _ => "fridge",
                    }
                    .into(),
                ))
                .expect("show synthetic storage");
            if matches!(demo.as_deref(), Some("selected" | "cooked")) {
                if let Some(lot) = state
                    .snapshot
                    .lots
                    .iter()
                    .find(|lot| {
                        lot.zone == "fridge"
                            && (demo.as_deref() != Some("cooked") || lot.name == "large eggs")
                    })
                    .map(|lot| lot.id.clone())
                {
                    state
                        .dispatch(AppAction::SelectLot(lot))
                        .expect("show selected synthetic lot");
                }
            }
        }
        state.message = "SYNTHETIC TEST DATA — application-rendered verification capture".into();
        state.show_about = demo.as_deref() == Some("about");
        if matches!(demo.as_deref(), Some("plan-next" | "plan-added")) {
            state.meal_filter = MealFilter::PlanNext;
            state.selected_recipe_id = if demo.as_deref() == Some("plan-added") {
                Some("tomato-pasta".into())
            } else {
                None
            };
        }
    }
    let resolution = std::env::var("HUNGERSTRIKE_CAPTURE_SIZE")
        .ok()
        .and_then(|size| {
            let (width, height) = size.split_once('x')?;
            Some((width.parse::<u32>().ok()?, height.parse::<u32>().ok()?))
        })
        .filter(|(width, height)| (1000..=1920).contains(width) && (700..=1200).contains(height))
        .unwrap_or((1440, 900));
    let assets_dir = std::env::var_os("HUNGERSTRIKE_ASSETS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .expect("locate local executable")
                .parent()
                .expect("executable directory")
                .join("assets")
        });
    platform::start_system_theme_watch();
    App::new()
        .insert_resource(state)
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets_dir.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "HungerStrike · local kitchen".into(),
                        resolution: resolution.into(),
                        resize_constraints: bevy::window::WindowResizeConstraints {
                            min_width: 1000.0,
                            min_height: 680.0,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                }),
            EguiPlugin::default(),
            MeshPickingPlugin,
        ))
        .add_observer(scene::lot_clicked)
        .add_systems(Startup, scene::setup)
        .add_systems(
            Update,
            (
                sync_system_theme,
                scene::apply_scene_theme,
                scene::camera_controls,
                scene::sync_visuals,
                scene::animate_visuals,
                capture_once,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, draw_ui)
        .run();
}

/// Track the operating-system theme: bevy records it on the primary window
/// when the window is created and sends a message when it changes.
fn sync_system_theme(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut changes: MessageReader<bevy::window::WindowThemeChanged>,
    mut state: ResMut<AppState>,
) {
    let mut system_dark = state.system_dark;
    if let Some(dark) = platform::portal_system_dark() {
        // Linux: the desktop portal reports the colour scheme that winit
        // does not expose on X11.
        system_dark = Some(dark);
    } else if system_dark.is_none() {
        if let Some(theme) = windows.iter().next().and_then(|window| window.window_theme) {
            system_dark = Some(theme == bevy::window::WindowTheme::Dark);
        }
    }
    for change in changes.read() {
        system_dark = Some(change.theme == bevy::window::WindowTheme::Dark);
    }
    if system_dark != state.system_dark {
        state.system_dark = system_dark;
        state.update_theme();
    }
}

fn draw_ui(mut contexts: EguiContexts, mut state: ResMut<AppState>) -> Result {
    state.update_theme();
    state.poll_ocr();
    ui::draw(contexts.ctx_mut()?, &mut state);
    Ok(())
}

fn capture_once(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<AppState>,
    mut captured: Local<bool>,
) {
    let delay = std::env::var("HUNGERSTRIKE_CAPTURE_DELAY")
        .ok()
        .and_then(|secs| secs.parse::<f32>().ok())
        .unwrap_or(2.0)
        .clamp(1.0, 60.0);
    // Wait for any OCR job so a receipt-photo capture shows the real draft.
    if *captured || time.elapsed_secs() < delay || state.ocr_task.is_some() {
        return;
    }
    if let Some(path) = std::env::var_os("HUNGERSTRIKE_CAPTURE_PATH") {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(PathBuf::from(path)))
            .observe(exit_after_capture);
        *captured = true;
    }
}

/// With `HUNGERSTRIKE_CAPTURE_EXIT=1`, quit once the capture has been written
/// (the save observer writes the file synchronously). Used by headless CI.
fn exit_after_capture(_captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    if std::env::var("HUNGERSTRIKE_CAPTURE_EXIT").as_deref() == Ok("1") {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hungerstrike_core::DraftItem;

    const RECEIPT: &str = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");

    #[test]
    fn unknown_visual_fixture_is_corrected_before_isolated_inventory_seed() {
        let raw = include_str!("../../tests/fixtures/visual-unknown-items.txt");
        let draft = parse_text(raw);
        assert_eq!(
            draft
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["apple juice", "fresh tomatoes"]
        );
        assert!(review(draft).is_err(), "unknown categories need review");

        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-04".into()).unwrap();
        seed_unknown_visual_capture(&mut state).unwrap();
        assert_eq!(state.snapshot.receipt_count, 1);
        assert_eq!(state.snapshot.cash_flow_cents, 728);
        assert_eq!(state.zone, "fridge");
        assert_eq!(state.snapshot.lots.len(), 2);
        assert!(state
            .snapshot
            .lots
            .iter()
            .all(|lot| lot.zone == "fridge" && lot.expiry_date.is_none()));
        assert_eq!(
            state
                .snapshot
                .lots
                .iter()
                .find(|lot| lot.id == state.lot_id)
                .map(|lot| lot.name.as_str()),
            Some("apple juice")
        );
    }

    #[test]
    fn cancel_clears_temporary_ocr_and_review_state() {
        let store = Store::open_in_memory().unwrap();
        let mut state = AppState::new(store, "2026-10-03".into()).unwrap();
        state.raw_text = "VISA ENDING 1234".into();
        state.draft = Some(parse_text(
            "DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29",
        ));
        state.dispatch(AppAction::CancelReceipt).unwrap();
        assert!(state.raw_text.is_empty());
        assert!(state.draft.is_none());
        assert_eq!(state.snapshot.receipt_count, 0);
    }

    #[test]
    fn app_actions_cover_zone_picking_list_selection_navigation_and_stock_use() {
        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-03".into()).unwrap();
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        assert_eq!(state.snapshot.receipt_count, 0);
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        let banana = state
            .snapshot
            .lots
            .iter()
            .find(|lot| lot.name == "bananas")
            .unwrap()
            .id
            .clone();
        state.scene_dirty = false;
        state
            .dispatch(AppAction::SelectZone("fridge".into()))
            .unwrap();
        assert_eq!(state.zone, "fridge");
        assert!(state.scene_dirty);
        state
            .dispatch(AppAction::SelectLot(banana.clone()))
            .unwrap();
        assert_eq!(state.lot_id, banana);
        state.dispatch(AppAction::Navigate(Tab::Meals)).unwrap();
        assert_eq!(state.tab, Tab::Meals);
        state
            .dispatch(AppAction::Inventory(InventoryAction::Consume {
                lot_id: banana.clone(),
                quantity: "0.25".into(),
            }))
            .unwrap();
        assert_eq!(
            state
                .snapshot
                .lots
                .iter()
                .find(|lot| lot.id == banana)
                .unwrap()
                .quantity,
            "1"
        );
        state
            .dispatch(AppAction::Inventory(InventoryAction::Move {
                lot_id: banana.clone(),
                zone: "pantry".into(),
            }))
            .unwrap();
        assert_eq!(state.zone, "pantry");
        assert_eq!(state.lot_id, banana);
        assert_eq!(
            state
                .snapshot
                .lots
                .iter()
                .find(|lot| lot.id == banana)
                .unwrap()
                .zone,
            "pantry"
        );
    }

    #[test]
    fn app_actions_keep_receipt_pending_until_confirm_and_cancel_does_not_write() {
        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-03".into()).unwrap();
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        assert!(state.draft.is_some());
        assert_eq!(state.snapshot.receipt_count, 0);
        state.dispatch(AppAction::CancelReceipt).unwrap();
        assert!(state.raw_text.is_empty());
        assert_eq!(state.snapshot.receipt_count, 0);
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        assert_eq!(state.snapshot.receipt_count, 1);
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        assert!(state.source_match.is_some());
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        assert_eq!(state.snapshot.receipt_count, 1);
    }

    #[test]
    fn app_separate_purchase_requires_acknowledgment_and_new_identity() {
        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-03".into()).unwrap();
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        let draft = state.draft.as_mut().unwrap();
        draft.id = "R001-SECOND".into();
        draft.purchase_date = Some("2026-10-04".into());
        for (index, item) in draft.items.iter_mut().enumerate() {
            item.line_id = format!("R001-SECOND-L{:02}", index + 1);
        }
        state.mark_all_reviewed();
        assert!(state
            .dispatch(AppAction::Commit(ReceiptCommit::Separate))
            .is_err());
        assert_eq!(state.snapshot.receipt_count, 1);
        state.separate_ack = true;
        state.mark_all_reviewed();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::Separate))
            .unwrap();
        assert_eq!(state.snapshot.receipt_count, 2);
        assert_eq!(
            state.snapshot.receipt_edits[1].action,
            "separate_purchase_confirmed"
        );
    }

    #[test]
    fn app_backup_preview_replace_and_undo_follow_state_actions() {
        let mut source =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-03".into()).unwrap();
        source.raw_text = RECEIPT.into();
        source.dispatch(AppAction::ParseReceipt).unwrap();
        source.mark_all_reviewed();
        source
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        let backup = source.export_backup_bytes().unwrap();
        let mut target =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-03".into()).unwrap();
        assert!(target
            .dispatch(AppAction::PreviewBackup(b"invalid".to_vec()))
            .is_err());
        assert_eq!(target.snapshot.receipt_count, 0);
        target.dispatch(AppAction::PreviewBackup(backup)).unwrap();
        assert_eq!(target.backup_preview.as_ref().unwrap().incoming_receipts, 1);
        target
            .dispatch(AppAction::ApplyBackup(ReplaceOrReconcile::Replace))
            .unwrap();
        assert_eq!(target.snapshot.receipt_count, 1);
        target.dispatch(AppAction::UndoBackup).unwrap();
        assert_eq!(target.snapshot.receipt_count, 0);
    }

    fn load_photo(state: &mut AppState, bytes: &[u8]) {
        state.dispatch(AppAction::StartOcr(bytes.to_vec())).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while state.ocr_task.is_some() {
            state.poll_ocr();
            assert!(
                std::time::Instant::now() < deadline,
                "OCR worker did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// What a person does on the review screen for the SYNTHETIC R101 photo:
    /// correct each line against the image and tick it as checked.
    fn correct_r101_draft(state: &mut AppState) {
        let draft = state.draft.as_mut().expect("OCR draft");
        draft.id = "R101".into();
        draft.purchase_date = Some("2026-10-12".into());
        let lines = [
            ("bananas", "1.25", "lb", 86, "food.produce"),
            ("milk 2%", "1", "gal", 429, "food.dairy"),
            ("large eggs", "12", "egg", 349, "food.dairy"),
            ("spinach", "5", "oz", 299, "food.produce"),
            ("rice", "2", "lb", 379, "food.pantry"),
        ];
        draft.items = lines
            .iter()
            .enumerate()
            .map(
                |(index, (name, quantity, unit, cents, category))| DraftItem {
                    line_id: format!("R101-L{:02}", index + 1),
                    raw: String::new(),
                    name: (*name).into(),
                    quantity: Some((*quantity).into()),
                    unit: Some((*unit).into()),
                    gross_cents: Some(*cents),
                    discount_cents: 0,
                    net_cents: Some(*cents),
                    category: (*category).into(),
                    expiry_date: None,
                    expiry_estimated: false,
                    uncertainty: Vec::new(),
                    reviewed: false,
                },
            )
            .collect();
        draft.refunds.clear();
        draft.unallocated_adjustments.clear();
        draft.known_subtotal_cents = 1542;
        draft.printed_subtotal_cents = Some(1542);
        draft.printed_tax_cents = Some(0);
        draft.printed_total_cents = Some(1542);
        draft.uncertainty.clear();
    }

    #[test]
    fn bundled_ocr_draft_needs_line_review_and_repeat_photo_is_held_as_duplicate() {
        let photo = include_bytes!("../../tests/images/synthetic-receipt-clean.png");
        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-12".into()).unwrap();
        load_photo(&mut state, photo);
        let draft = state.draft.clone().expect("bundled OCR returns a draft");
        assert!(
            draft.items.iter().all(|item| item.expiry_date.is_none()),
            "a receipt never provides expiry dates"
        );
        assert!(state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .is_err());
        assert_eq!(state.snapshot.receipt_count, 0);

        correct_r101_draft(&mut state);
        assert!(review(state.draft.clone().unwrap()).is_ok());
        // Totals reconcile, but nothing is saved until every line is checked.
        let error = state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap_err();
        assert!(error.contains("check every line"), "{error}");
        assert_eq!(state.snapshot.receipt_count, 0);
        for index in 0..4 {
            state.draft.as_mut().unwrap().items[index].reviewed = true;
        }
        assert!(state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap_err()
            .contains("1 not checked"));

        // Estimated expiry is opt-in, labelled and dated from purchase.
        state.dispatch(AppAction::EstimateExpiry(Some(1))).unwrap();
        let milk = &state.draft.as_ref().unwrap().items[1];
        assert_eq!(milk.expiry_date.as_deref(), Some("2026-10-19"));
        assert!(milk.expiry_estimated);
        assert!(state.draft.as_ref().unwrap().items[0].expiry_date.is_none());

        state.draft.as_mut().unwrap().items[4].reviewed = true;
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        assert_eq!(
            (state.snapshot.receipt_count, state.snapshot.cash_flow_cents),
            (1, 1542)
        );
        let milk = state
            .snapshot
            .lots
            .iter()
            .find(|lot| lot.name == "milk 2%")
            .unwrap();
        assert!(milk.expiry_estimated);

        // Importing the same photo again is recognised and does not double-spend.
        load_photo(&mut state, photo);
        assert!(state.source_match.is_some());
        correct_r101_draft(&mut state);
        state.mark_all_reviewed();
        // The re-reviewed draft differs from the saved one (it has no milk
        // estimate), so it is refused rather than silently merged or charged.
        let error = state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap_err();
        assert!(error.contains("already imported"), "{error}");
        assert_eq!(
            (state.snapshot.receipt_count, state.snapshot.cash_flow_cents),
            (1, 1542)
        );
        // An identical re-review is accepted as a no-op duplicate.
        state.dispatch(AppAction::EstimateExpiry(Some(1))).unwrap();
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        assert_eq!(state.message, "Receipt: Duplicate");
        assert_eq!(
            (state.snapshot.receipt_count, state.snapshot.cash_flow_cents),
            (1, 1542)
        );
    }

    #[test]
    fn corrupt_photo_and_cancelled_ocr_never_write() {
        let mut state =
            AppState::new(Store::open_in_memory().unwrap(), "2026-10-12".into()).unwrap();
        load_photo(
            &mut state,
            include_bytes!("../../tests/images/synthetic-receipt-truncated.png"),
        );
        assert!(state.draft.is_none());
        assert!(state.message.contains("not a readable PNG or JPEG"));
        assert_eq!(state.snapshot.receipt_count, 0);

        state
            .dispatch(AppAction::StartOcr(
                include_bytes!("../../tests/images/synthetic-receipt-photo.jpg").to_vec(),
            ))
            .unwrap();
        assert!(
            state.dispatch(AppAction::StartOcr(Vec::new())).is_err(),
            "only one OCR job at a time"
        );
        state.dispatch(AppAction::CancelReceipt).unwrap();
        assert!(state.ocr_task.is_none());
        assert!(state.draft.is_none());
        assert_eq!(state.snapshot.receipt_count, 0);
    }

    #[test]
    fn batch_feedback_distinguishes_unchanged_and_retained_quantities() {
        let mut store = Store::open_in_memory().unwrap();
        confirm_receipt(&mut store, review(parse_text(RECEIPT)).unwrap()).unwrap();
        let mut state = AppState::new(store, "2026-10-04".into()).unwrap();
        let banana_meal = |id: &str, quantity: &str| Recipe {
            id: id.into(),
            name: id.into(),
            servings: 1,
            ingredients: vec![hungerstrike_core::RecipeIngredient {
                name: "banana".into(),
                quantity: quantity.into(),
                unit: "lb".into(),
            }],
            steps: vec!["Slice the banana.".into()],
        };

        state
            .dispatch(AppAction::AddMissingBatch(vec![banana_meal(
                "covered", "0.25",
            )]))
            .unwrap();
        assert!(state.snapshot.shopping.is_empty());
        assert_eq!(
            state.message,
            "Shopping list unchanged; any missing groceries are already listed."
        );
        assert!(state.meal_filter == MealFilter::PlanNext);

        state
            .dispatch(AppAction::AddMissingBatch(vec![banana_meal(
                "larger", "1.5",
            )]))
            .unwrap();
        assert_eq!(state.snapshot.shopping[0].quantity, "0.25");
        assert!(state.message.contains("saved"));
        assert!(state
            .message
            .contains("Larger existing shopping quantities"));

        state
            .dispatch(AppAction::AddMissingBatch(vec![banana_meal(
                "smaller", "1.4",
            )]))
            .unwrap();
        assert_eq!(state.snapshot.shopping[0].quantity, "0.25");
        assert_eq!(
            state.message,
            "Shopping list unchanged; any missing groceries are already listed."
        );
    }

    #[test]
    fn theme_choice_applies_immediately_persists_and_follows_system() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inventory.sqlite");
        {
            let mut state =
                AppState::new(Store::open(&path).unwrap(), "2026-10-18".into()).unwrap();
            assert_eq!(state.theme_pref, theme::ThemePref::Light);
            assert!(!state.dark, "light is the default");
            state.system_dark = Some(true);
            state.update_theme();
            assert!(!state.dark, "Light ignores the system preference");
            state
                .dispatch(AppAction::SetTheme(theme::ThemePref::Dark))
                .unwrap();
            assert!(state.dark, "switching applies without a restart");
            state
                .dispatch(AppAction::SetTheme(theme::ThemePref::System))
                .unwrap();
            assert!(state.dark, "System follows a dark OS");
            state.system_dark = Some(false);
            state.update_theme();
            assert!(!state.dark, "System follows a light OS");
        }
        let reopened = AppState::new(Store::open(&path).unwrap(), "2026-10-18".into()).unwrap();
        assert_eq!(reopened.theme_pref, theme::ThemePref::System);
        assert!(!reopened.dark, "no OS report yet: System starts light");
    }
}

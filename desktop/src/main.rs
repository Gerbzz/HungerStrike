mod ocr;
mod scene;
mod ui;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use hungerstrike_core::{
    apply_import, budget_view, confirm_receipt, confirm_separate_purchase, correct_receipt,
    export_backup, matching_source_receipt, parse_text, preview_import, record_inventory_action,
    restore_last_import, review, set_budget, snapshot, AppSnapshot, ImportPreview, InventoryAction,
    Period, ReceiptDraft, ReplaceOrReconcile, Store,
};
use std::{
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
    ParseReceipt,
    StartOcr(Vec<u8>),
    Commit(ReceiptCommit),
    CancelReceipt,
    PreviewBackup(Vec<u8>),
    ApplyBackup(ReplaceOrReconcile),
    UndoBackup,
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
    category: String,
    reference_date: String,
    week_start: u8,
    weekly_budget: i64,
    monthly_budget: i64,
    backup_bytes: Option<Vec<u8>>,
    backup_preview: Option<ImportPreview>,
    source_match: Option<(String, String)>,
    separate_ack: bool,
    ocr_task: Option<OcrTask>,
    style_ready: bool,
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
            category: String::new(),
            reference_date: today,
            week_start: 0,
            weekly_budget,
            monthly_budget,
            backup_bytes: None,
            backup_preview: None,
            source_match: None,
            separate_ack: false,
            ocr_task: None,
            style_ready: false,
            scene_dirty: true,
            camera_yaw: 0.65,
            camera_distance: 12.0,
        })
    }

    fn refresh(&mut self) -> Result<(), String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        self.snapshot = snapshot(&store)?;
        if let Ok(view) = budget_view(&store, &self.reference_date, Period::Week, self.week_start) {
            self.weekly_budget = view.budget_cents.unwrap_or(0);
        }
        if let Ok(view) = budget_view(&store, &self.reference_date, Period::Month, self.week_start)
        {
            self.monthly_budget = view.budget_cents.unwrap_or(0);
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
                    .map(|_| "OCR is ready. Verify every field before confirmation.".into())
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
                let engine = ocr::installed_engine();
                let handle = std::thread::spawn(move || {
                    ocr::extract_local_image_with_cancel(&bytes, &engine, &thread_cancel)
                });
                self.ocr_task = Some(OcrTask { cancel, handle });
                self.message = "Reading local image. You can cancel this operation.".into();
            }
            AppAction::Commit(mode) => {
                let draft = self.draft.clone().ok_or("no receipt to confirm")?;
                if matches!(mode, ReceiptCommit::Separate) && !self.separate_ack {
                    return Err("acknowledge that this is a separate purchase first".into());
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

fn data_path() -> PathBuf {
    let directory = std::env::var_os("HUNGERSTRIKE_DATA_DIR")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    directory.join("HungerStrike").join("inventory.sqlite")
}

fn main() {
    let path = data_path();
    std::fs::create_dir_all(path.parent().expect("data folder")).expect("create local data folder");
    let store = Store::open(&path).expect("open local inventory");
    let demo = std::env::var("HUNGERSTRIKE_CAPTURE_DEMO").ok();
    let today = if demo.is_some() {
        "2026-10-04".into()
    } else {
        chrono::Local::now().date_naive().to_string()
    };
    let mut state = AppState::new(store, today).expect("read local inventory");
    if demo.as_deref() == Some("receipt") {
        state.raw_text =
            include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt").into();
        state
            .dispatch(AppAction::ParseReceipt)
            .expect("parse demo receipt");
        state
            .dispatch(AppAction::Navigate(Tab::Receipt))
            .expect("show receipt");
        state.message = "SYNTHETIC TEST DATA — review only; no inventory write".into();
    } else if matches!(demo.as_deref(), Some("kitchen" | "meals" | "spending")) {
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
            }
        }
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .expect("seed synthetic stock");
        state.raw_text = include_str!("../../core/tests/fixtures/receipts/R002-refund.txt").into();
        state
            .dispatch(AppAction::ParseReceipt)
            .expect("parse synthetic second receipt");
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .expect("seed synthetic spending");
        {
            let mut store = state.store.lock().expect("synthetic store lock");
            set_budget(&mut store, Period::Week, Some(4000)).expect("synthetic weekly budget");
            set_budget(&mut store, Period::Month, Some(12000)).expect("synthetic monthly budget");
        }
        state.refresh().expect("refresh synthetic state");
        let tab = match demo.as_deref() {
            Some("meals") => Tab::Meals,
            Some("spending") => Tab::Spending,
            _ => Tab::Kitchen,
        };
        state
            .dispatch(AppAction::Navigate(tab))
            .expect("show synthetic page");
        if tab == Tab::Kitchen {
            state
                .dispatch(AppAction::SelectZone("fridge".into()))
                .expect("show synthetic fridge");
        }
        state.message = "SYNTHETIC TEST DATA — application-rendered verification capture".into();
    }
    App::new()
        .insert_resource(state)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "HungerStrike · local kitchen".into(),
                    resolution: (1440, 900).into(),
                    ..default()
                }),
                ..default()
            }),
            EguiPlugin::default(),
            MeshPickingPlugin,
        ))
        .add_systems(Startup, scene::setup)
        .add_systems(
            Update,
            (scene::camera_controls, scene::sync_visuals, capture_once),
        )
        .add_systems(EguiPrimaryContextPass, draw_ui)
        .run();
}

fn draw_ui(mut contexts: EguiContexts, mut state: ResMut<AppState>) -> Result {
    state.poll_ocr();
    ui::draw(contexts.ctx_mut()?, &mut state);
    Ok(())
}

fn capture_once(mut commands: Commands, time: Res<Time>, mut captured: Local<bool>) {
    if *captured || time.elapsed_secs() < 2.0 {
        return;
    }
    if let Some(path) = std::env::var_os("HUNGERSTRIKE_CAPTURE_PATH") {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(PathBuf::from(path)));
        *captured = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");

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
        state
            .dispatch(AppAction::SelectLot(banana.clone()))
            .unwrap();
        assert_eq!(state.zone, "pantry");
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
        state
            .dispatch(AppAction::Commit(ReceiptCommit::New))
            .unwrap();
        assert_eq!(state.snapshot.receipt_count, 1);
        state.raw_text = RECEIPT.into();
        state.dispatch(AppAction::ParseReceipt).unwrap();
        assert!(state.source_match.is_some());
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
        assert!(state
            .dispatch(AppAction::Commit(ReceiptCommit::Separate))
            .is_err());
        assert_eq!(state.snapshot.receipt_count, 1);
        state.separate_ack = true;
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
}

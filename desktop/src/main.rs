mod ocr;
mod scene;
mod ui;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use hungerstrike_core::{
    budget_view, parse_text, snapshot, AppSnapshot, ImportPreview, Period, ReceiptDraft, Store,
};
use std::{path::PathBuf, sync::Mutex};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Kitchen,
    Receipt,
    Meals,
    Spending,
    Data,
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
    scene_dirty: bool,
    camera_yaw: f32,
    camera_distance: f32,
}

impl AppState {
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
    fn clear_receipt(&mut self) {
        self.draft = None;
        self.raw_text.clear();
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
    let snapshot = snapshot(&store).expect("read local inventory");
    let today = chrono::Local::now().date_naive().to_string();
    let weekly_budget = budget_view(&store, &today, Period::Week, 0)
        .ok()
        .and_then(|view| view.budget_cents)
        .unwrap_or(0);
    let monthly_budget = budget_view(&store, &today, Period::Month, 0)
        .ok()
        .and_then(|view| view.budget_cents)
        .unwrap_or(0);
    let demo_receipt =
        std::env::var("HUNGERSTRIKE_CAPTURE_DEMO").ok().as_deref() == Some("receipt");
    let sample = if demo_receipt {
        include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt")
    } else {
        ""
    };
    let state = AppState {
        store: Mutex::new(store),
        snapshot,
        tab: if demo_receipt {
            Tab::Receipt
        } else {
            Tab::Kitchen
        },
        draft: if demo_receipt {
            Some(parse_text(sample))
        } else {
            None
        },
        raw_text: sample.into(),
        message: if demo_receipt {
            "SYNTHETIC TEST DATA · review only; no inventory write".into()
        } else {
            String::new()
        },
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
        scene_dirty: true,
        camera_yaw: 0.65,
        camera_distance: 12.0,
    };
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

    #[test]
    fn cancel_clears_temporary_ocr_and_review_state() {
        let store = Store::open_in_memory().unwrap();
        let current = snapshot(&store).unwrap();
        let mut state = AppState {
            store: Mutex::new(store),
            snapshot: current,
            tab: Tab::Receipt,
            draft: Some(parse_text(
                "DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29",
            )),
            raw_text: "VISA ENDING 1234".into(),
            message: String::new(),
            zone: "all".into(),
            lot_id: String::new(),
            quantity: String::new(),
            category: String::new(),
            reference_date: "2026-10-03".into(),
            week_start: 0,
            weekly_budget: 0,
            monthly_budget: 0,
            backup_bytes: None,
            backup_preview: None,
            scene_dirty: false,
            camera_yaw: 0.0,
            camera_distance: 10.0,
        };
        state.clear_receipt();
        assert!(state.raw_text.is_empty());
        assert!(state.draft.is_none());
        assert_eq!(state.snapshot.receipt_count, 0);
    }
}

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{CategoryEdit, LotView, ReceiptDraft, ReceiptEdit, ShoppingItem};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CostEvent {
    pub kind: String,
    pub amount_cents: i64,
    #[serde(default)]
    pub cost_unknown: bool,
    #[serde(default)]
    pub lot_id: Option<String>,
    #[serde(default)]
    pub quantity: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct State {
    pub schema_version: u32,
    pub receipts: Vec<ReceiptDraft>,
    pub lots: Vec<LotView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shopping: Vec<ShoppingItem>,
    pub cost_events: Vec<CostEvent>,
    pub refund_without_stock_reversal_cents: i64,
    #[serde(default)]
    pub weekly_budget_cents: Option<i64>,
    #[serde(default)]
    pub monthly_budget_cents: Option<i64>,
    /// Display theme setting: "light", "dark" or "system". Absent means light.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_theme: Option<String>,
    #[serde(default)]
    pub category_edits: Vec<CategoryEdit>,
    #[serde(default)]
    pub receipt_edits: Vec<ReceiptEdit>,
}

#[derive(Serialize, Deserialize)]
struct RestorePoint {
    previous_backup_json: String,
    applied_state_sha256: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            receipts: Vec::new(),
            lots: Vec::new(),
            shopping: Vec::new(),
            cost_events: Vec::new(),
            refund_without_stock_reversal_cents: 0,
            weekly_budget_cents: None,
            monthly_budget_cents: None,
            ui_theme: None,
            category_edits: Vec::new(),
            receipt_edits: Vec::new(),
        }
    }
}

pub struct Store {
    connection: Connection,
}

/// Describes a local database that could not be read and was set aside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryNote {
    /// Where the unreadable file (and any journal) was moved. It is never
    /// deleted, so it can be inspected or restored by hand.
    pub preserved_copy: PathBuf,
    pub reason: String,
}

fn looks_corrupt(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    [
        "not a database",
        "malformed",
        "corrupt",
        "file is encrypted",
    ]
    .iter()
    .any(|marker| error.contains(marker))
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        Self::initialize(connection)
    }

    /// Open the local database, recovering from a corrupt file.
    ///
    /// SQLite's rollback journal already makes each save atomic: an
    /// interrupted write is rolled back on the next open. This handles the
    /// remaining case where the file itself is unreadable (damaged bytes or an
    /// unparseable payload). The damaged file and its journal are renamed
    /// next to the original, never deleted, and a fresh store is created.
    /// Lock/busy errors and data from a newer app version are returned as
    /// errors instead, because setting those aside would hide good data.
    pub fn open_or_recover(path: &Path) -> Result<(Self, Option<RecoveryNote>), String> {
        let reason = match Self::open(path) {
            Ok(store) => match store.load_payload() {
                Ok(raw) => match serde_json::from_str::<State>(&raw) {
                    Ok(state) if state.schema_version == 1 => return Ok((store, None)),
                    Ok(state) => {
                        return Err(format!(
                            "unsupported local data version {}; use the newer HungerStrike build",
                            state.schema_version
                        ))
                    }
                    Err(error) => format!("saved data could not be read ({error})"),
                },
                Err(error) if looks_corrupt(&error) => error,
                Err(error) => return Err(error),
            },
            Err(error) if looks_corrupt(&error) => error,
            Err(error) => return Err(error),
        };
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("inventory");
        let mut preserved = path.with_file_name(format!("{stem}.unreadable-{stamp}.sqlite"));
        let mut counter = 1;
        while preserved.exists() {
            preserved = path.with_file_name(format!("{stem}.unreadable-{stamp}-{counter}.sqlite"));
            counter += 1;
        }
        std::fs::rename(path, &preserved)
            .map_err(|e| format!("could not set aside unreadable data: {e}"))?;
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut sidecar = path.as_os_str().to_owned();
            sidecar.push(suffix);
            let sidecar = PathBuf::from(sidecar);
            if sidecar.exists() {
                let mut target = preserved.as_os_str().to_owned();
                target.push(suffix);
                std::fs::rename(&sidecar, PathBuf::from(target))
                    .map_err(|e| format!("could not set aside {suffix} file: {e}"))?;
            }
        }
        let store = Self::open(path)?;
        Ok((
            store,
            Some(RecoveryNote {
                preserved_copy: preserved,
                reason,
            }),
        ))
    }

    fn load_payload(&self) -> Result<String, String> {
        self.connection
            .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let connection = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::initialize(connection)
    }

    pub fn open_read_only(path: &Path) -> Result<Self, String> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }

    fn initialize(connection: Connection) -> Result<Self, String> {
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS app_state (
                   id INTEGER PRIMARY KEY CHECK (id = 1),
                   payload TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS restore_points (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   backup_json TEXT NOT NULL
                 );",
            )
            .map_err(|e| e.to_string())?;
        let initial = serde_json::to_string(&State::default()).map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO app_state (id, payload) VALUES (1, ?1)",
                params![initial],
            )
            .map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }

    pub(crate) fn load(&self) -> Result<State, String> {
        let raw: String = self
            .connection
            .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        let state: State = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if state.schema_version != 1 {
            return Err(format!(
                "unsupported local data version {}",
                state.schema_version
            ));
        }
        Ok(state)
    }

    pub(crate) fn save(&mut self, state: &State) -> Result<(), String> {
        let mut safe = state.clone();
        for receipt in &mut safe.receipts {
            for item in &mut receipt.items {
                item.raw.clear();
            }
            receipt.unallocated_adjustments.clear();
        }
        let raw = serde_json::to_string(&safe).map_err(|e| e.to_string())?;
        let transaction = self.connection.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE app_state SET payload = ?1 WHERE id = 1",
                params![raw],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }

    pub(crate) fn replace_with_restore(
        &mut self,
        state: &State,
        backup_json: &str,
    ) -> Result<(), String> {
        let raw = serde_json::to_string(state).map_err(|e| e.to_string())?;
        let point = RestorePoint {
            previous_backup_json: backup_json.into(),
            applied_state_sha256: format!("{:x}", Sha256::digest(raw.as_bytes())),
        };
        let point_json = serde_json::to_string(&point).map_err(|e| e.to_string())?;
        let transaction = self.connection.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "INSERT INTO restore_points (backup_json) VALUES (?1)",
                params![point_json],
            )
            .map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE app_state SET payload = ?1 WHERE id = 1",
                params![raw],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }

    pub(crate) fn latest_restore_point(&self) -> Result<Option<(i64, String, String)>, String> {
        let row: Option<(i64, String)> = self
            .connection
            .query_row(
                "SELECT id, backup_json FROM restore_points ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        row.map(|(id, raw)| {
            let point: RestorePoint = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            Ok((id, point.previous_backup_json, point.applied_state_sha256))
        })
        .transpose()
    }

    pub(crate) fn apply_restore_point(
        &mut self,
        id: i64,
        state: &State,
        expected_current_sha256: &str,
    ) -> Result<(), String> {
        let raw = serde_json::to_string(state).map_err(|e| e.to_string())?;
        let transaction = self.connection.transaction().map_err(|e| e.to_string())?;
        let current: String = transaction
            .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        if format!("{:x}", Sha256::digest(current.as_bytes())) != expected_current_sha256 {
            return Err(
                "local data changed after replacement; undo would discard newer work".into(),
            );
        }
        transaction
            .execute(
                "UPDATE app_state SET payload = ?1 WHERE id = 1",
                params![raw],
            )
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM restore_points WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }
}

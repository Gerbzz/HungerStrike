use std::path::Path;

use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::model::{CategoryEdit, LotView, ReceiptDraft};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CostEvent {
    pub kind: String,
    pub amount_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct State {
    pub schema_version: u32,
    pub receipts: Vec<ReceiptDraft>,
    pub lots: Vec<LotView>,
    pub cost_events: Vec<CostEvent>,
    pub refund_without_stock_reversal_cents: i64,
    #[serde(default)]
    pub weekly_budget_cents: Option<i64>,
    #[serde(default)]
    pub monthly_budget_cents: Option<i64>,
    #[serde(default)]
    pub category_edits: Vec<CategoryEdit>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            receipts: Vec::new(),
            lots: Vec::new(),
            cost_events: Vec::new(),
            refund_without_stock_reversal_cents: 0,
            weekly_budget_cents: None,
            monthly_budget_cents: None,
            category_edits: Vec::new(),
        }
    }
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        Self::initialize(connection)
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
        let raw = serde_json::to_string(state).map_err(|e| e.to_string())?;
        let transaction = self.connection.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE app_state SET payload = ?1 WHERE id = 1",
                params![raw],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }
}

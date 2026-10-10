use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    PendingConfirmation,
    NeedsResolution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftItem {
    pub line_id: String,
    pub raw: String,
    pub name: String,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub gross_cents: Option<i64>,
    pub discount_cents: i64,
    pub net_cents: Option<i64>,
    pub category: String,
    pub expiry_date: Option<String>,
    pub uncertainty: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundLine {
    pub name: String,
    pub category: String,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub amount_cents: i64,
    pub original_line_id: Option<String>,
    #[serde(default)]
    pub inventory_quantity_to_reverse: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptDraft {
    pub id: String,
    pub purchase_date: Option<String>,
    pub source_sha256: String,
    pub items: Vec<DraftItem>,
    pub refunds: Vec<RefundLine>,
    pub unallocated_adjustments: Vec<String>,
    #[serde(default)]
    pub unallocated_cents: Option<i64>,
    #[serde(default)]
    pub accepted_unknowns: bool,
    pub known_subtotal_cents: i64,
    pub printed_subtotal_cents: Option<i64>,
    pub printed_tax_cents: Option<i64>,
    pub printed_total_cents: Option<i64>,
    pub status: DraftStatus,
    pub uncertainty: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewedReceipt {
    pub draft: ReceiptDraft,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewError {
    NeedsResolution,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "receipt requires correction before confirmation")
    }
}

impl std::error::Error for ReviewError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmOutcome {
    Accepted,
    Duplicate,
    Corrected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InventoryAction {
    Consume { lot_id: String, quantity: String },
    Discard { lot_id: String, quantity: String },
    Move { lot_id: String, zone: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LotView {
    pub id: String,
    pub source_receipt_id: String,
    pub name: String,
    pub category: String,
    pub quantity: String,
    pub unit: String,
    pub remaining_cost_cents: i64,
    #[serde(default)]
    pub cost_unknown: bool,
    pub zone: String,
    pub expiry_date: Option<String>,
    pub purchase_date: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub receipt_count: usize,
    pub cash_flow_cents: i64,
    pub food_spending_cents: i64,
    pub household_goods_cents: i64,
    pub tax_cents: i64,
    pub consumed_cost_cents: i64,
    pub discarded_cost_cents: i64,
    pub unknown_consumption_count: usize,
    pub unknown_discard_count: usize,
    pub remaining_food_cost_cents: i64,
    pub unpriced_lot_count: usize,
    pub refund_without_stock_reversal_cents: i64,
    pub lots: Vec<LotView>,
    pub receipt_items: Vec<ReceiptItemView>,
    pub category_edits: Vec<CategoryEdit>,
    pub receipt_edits: Vec<ReceiptEdit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptEdit {
    pub sequence: usize,
    pub receipt_id: String,
    pub action: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub before_total_cents: Option<i64>,
    pub after_total_cents: Option<i64>,
    pub fields_changed: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptItemView {
    pub receipt_id: String,
    pub line_id: String,
    pub name: String,
    pub category: String,
    pub net_cents: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryEdit {
    pub sequence: usize,
    pub receipt_id: String,
    pub line_id: String,
    pub from: String,
    pub to: String,
}

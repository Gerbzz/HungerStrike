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
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub amount_cents: i64,
    pub original_line_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptDraft {
    pub id: String,
    pub purchase_date: Option<String>,
    pub source_sha256: String,
    pub items: Vec<DraftItem>,
    pub refunds: Vec<RefundLine>,
    pub unallocated_adjustments: Vec<String>,
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

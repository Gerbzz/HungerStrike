use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    inventory::decimal,
    model::ShoppingItem,
    receipt::review,
    store::{State, Store},
};

const FORMAT: &str = "hungerstrike-local-backup";
const VERSION: u32 = 1;
const MAX_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortableBackup {
    format: String,
    version: u32,
    checksum_sha256: String,
    payload: State,
}

impl PortableBackup {
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplaceOrReconcile {
    Replace,
    Reconcile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOutcome {
    Imported,
    Duplicate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportPreview {
    pub incoming_receipts: usize,
    pub existing_receipts: usize,
    pub duplicate_receipts: usize,
    pub conflicting_identifiers: usize,
    pub incoming_lots: usize,
    pub replace_will_create_restore_point: bool,
}

fn checksum(state: &State) -> Result<String, String> {
    let payload = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(&payload)))
}

fn sanitize(mut state: State) -> State {
    for receipt in &mut state.receipts {
        for item in &mut receipt.items {
            item.raw.clear();
        }
        receipt.unallocated_adjustments.clear();
    }
    state
}

fn make_backup(state: State) -> Result<PortableBackup, String> {
    let payload = sanitize(state);
    Ok(PortableBackup {
        format: FORMAT.into(),
        version: VERSION,
        checksum_sha256: checksum(&payload)?,
        payload,
    })
}

pub fn export_backup(store: &Store) -> Result<PortableBackup, String> {
    make_backup(store.load()?)
}

fn validate_state(state: &State) -> Result<(), String> {
    if state.schema_version != 1 {
        return Err("unsupported state version".into());
    }
    if state.weekly_budget_cents.is_some_and(|value| value < 0)
        || state.monthly_budget_cents.is_some_and(|value| value < 0)
    {
        return Err("invalid budget in backup".into());
    }
    let mut receipt_ids = HashSet::new();
    let mut source_hashes = HashMap::new();
    for receipt in &state.receipts {
        if receipt.items.iter().any(|item| !item.raw.is_empty())
            || !receipt.unallocated_adjustments.is_empty()
        {
            return Err("backup contains raw receipt text".into());
        }
        let checked = review(receipt.clone()).map_err(|e| e.to_string())?;
        if checked.draft.known_subtotal_cents != receipt.known_subtotal_cents {
            return Err("backup receipt accounting is not canonical".into());
        }
        if !receipt_ids.insert(receipt.id.as_str()) {
            return Err("duplicate receipt in backup".into());
        }
        if let Some(first_id) = source_hashes.get(receipt.source_sha256.as_str()) {
            if !state.receipt_edits.iter().any(|edit| {
                edit.receipt_id == receipt.id
                    && edit.action == "separate_purchase_confirmed"
                    && edit.related_receipt_id.as_deref() == Some(*first_id)
            }) {
                return Err("matching receipt source lacks separate-purchase audit".into());
            }
        } else {
            source_hashes.insert(receipt.source_sha256.as_str(), receipt.id.as_str());
        }
    }
    let mut lot_ids = HashSet::new();
    for lot in &state.lots {
        if !lot_ids.insert(lot.id.as_str())
            || !receipt_ids.contains(lot.source_receipt_id.as_str())
            || !lot.category.starts_with("food.")
            || decimal(&lot.quantity)? < rust_decimal::Decimal::ZERO
            || lot.remaining_cost_cents < 0
        {
            return Err("invalid inventory lot in backup".into());
        }
    }
    let mut shopping_keys = HashSet::new();
    for item in &state.shopping {
        if item.name.trim().is_empty()
            || item.unit.trim().is_empty()
            || decimal(&item.quantity)? <= rust_decimal::Decimal::ZERO
            || !shopping_keys.insert((
                item.name.trim().to_lowercase(),
                item.unit.trim().to_lowercase(),
            ))
        {
            return Err("invalid shopping item in backup".into());
        }
    }
    if state.cost_events.iter().any(|event| event.amount_cents < 0) {
        return Err("invalid cost event in backup".into());
    }
    Ok(())
}

fn parse_backup(bytes: &[u8]) -> Result<PortableBackup, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("backup is empty or too large".into());
    }
    let backup: PortableBackup = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if backup.format != FORMAT || backup.version != VERSION {
        return Err("unsupported backup format or version".into());
    }
    if checksum(&backup.payload)? != backup.checksum_sha256 {
        return Err("backup checksum mismatch".into());
    }
    validate_state(&backup.payload)?;
    Ok(backup)
}

pub fn preview_import(store: &Store, bytes: &[u8]) -> Result<ImportPreview, String> {
    let incoming = parse_backup(bytes)?;
    let existing = store.load()?;
    let mut duplicate_receipts = 0;
    let mut conflicting_identifiers = 0;
    for receipt in &incoming.payload.receipts {
        if existing
            .receipts
            .iter()
            .any(|old| old.id == receipt.id && old.source_sha256 == receipt.source_sha256)
        {
            duplicate_receipts += 1;
        } else if existing
            .receipts
            .iter()
            .any(|old| old.id == receipt.id || old.source_sha256 == receipt.source_sha256)
        {
            conflicting_identifiers += 1;
        }
    }
    Ok(ImportPreview {
        incoming_receipts: incoming.payload.receipts.len(),
        existing_receipts: existing.receipts.len(),
        duplicate_receipts,
        conflicting_identifiers,
        incoming_lots: incoming.payload.lots.len(),
        replace_will_create_restore_point: true,
    })
}

fn merge_shopping(existing: &mut State, incoming: Vec<ShoppingItem>) -> Result<bool, String> {
    let mut changed = false;
    for item in incoming {
        if let Some(current) = existing.shopping.iter_mut().find(|current| {
            current.name.eq_ignore_ascii_case(&item.name)
                && current.unit.eq_ignore_ascii_case(&item.unit)
        }) {
            if decimal(&item.quantity)? > decimal(&current.quantity)? {
                current.quantity = item.quantity;
                changed = true;
            }
        } else {
            existing.shopping.push(item);
            changed = true;
        }
    }
    Ok(changed)
}

pub fn apply_import(
    store: &mut Store,
    bytes: &[u8],
    mode: ReplaceOrReconcile,
) -> Result<ApplyOutcome, String> {
    let incoming = parse_backup(bytes)?.payload;
    let mut existing = store.load()?;
    match mode {
        ReplaceOrReconcile::Replace => {
            let restore = make_backup(existing)?.to_bytes()?;
            let restore = String::from_utf8(restore).map_err(|e| e.to_string())?;
            store.replace_with_restore(&incoming, &restore)?;
            Ok(ApplyOutcome::Imported)
        }
        ReplaceOrReconcile::Reconcile => {
            let overlap = incoming
                .receipts
                .iter()
                .filter(|receipt| {
                    existing.receipts.iter().any(|old| {
                        old.id == receipt.id && old.source_sha256 == receipt.source_sha256
                    })
                })
                .count();
            if overlap == incoming.receipts.len() {
                if merge_shopping(&mut existing, incoming.shopping)? {
                    validate_state(&existing)?;
                    store.save(&existing)?;
                    return Ok(ApplyOutcome::Imported);
                }
                return Ok(ApplyOutcome::Duplicate);
            }
            if overlap > 0 {
                return Err("partial receipt overlap needs explicit replacement".into());
            }
            if incoming.receipts.iter().any(|receipt| {
                existing
                    .receipts
                    .iter()
                    .any(|old| old.id == receipt.id || old.source_sha256 == receipt.source_sha256)
            }) {
                return Err(
                    "receipt identifier or source conflict; use explicit replacement after review"
                        .into(),
                );
            }
            if incoming
                .lots
                .iter()
                .any(|lot| existing.lots.iter().any(|old| old.id == lot.id))
            {
                return Err("inventory lot identifier conflict".into());
            }
            existing.receipts.extend(incoming.receipts);
            existing.lots.extend(incoming.lots);
            merge_shopping(&mut existing, incoming.shopping)?;
            existing.cost_events.extend(incoming.cost_events);
            existing.refund_without_stock_reversal_cents = existing
                .refund_without_stock_reversal_cents
                .checked_add(incoming.refund_without_stock_reversal_cents)
                .ok_or("refund adjustment overflow")?;
            for mut edit in incoming.category_edits {
                edit.sequence = existing.category_edits.len() + 1;
                existing.category_edits.push(edit);
            }
            for mut edit in incoming.receipt_edits {
                edit.sequence = existing.receipt_edits.len() + 1;
                existing.receipt_edits.push(edit);
            }
            if existing.weekly_budget_cents.is_none() {
                existing.weekly_budget_cents = incoming.weekly_budget_cents;
            }
            if existing.monthly_budget_cents.is_none() {
                existing.monthly_budget_cents = incoming.monthly_budget_cents;
            }
            validate_state(&existing)?;
            store.save(&existing)?;
            Ok(ApplyOutcome::Imported)
        }
    }
}

pub fn restore_last_import(store: &mut Store) -> Result<(), String> {
    let (id, raw, expected_current_sha256) = store
        .latest_restore_point()?
        .ok_or("no local restore point exists")?;
    let previous = parse_backup(raw.as_bytes())?;
    store.apply_restore_point(id, &previous.payload, &expected_current_sha256)
}

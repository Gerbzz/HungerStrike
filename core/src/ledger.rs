use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use crate::{
    inventory::{allocated_cost, decimal},
    model::{
        AppSnapshot, CategoryEdit, ConfirmOutcome, LotView, ReceiptDraft, ReceiptEdit,
        ReceiptItemView, ReviewedReceipt,
    },
    receipt::review,
    store::{State, Store},
};

fn sanitized(mut draft: ReceiptDraft) -> ReceiptDraft {
    for item in &mut draft.items {
        item.raw.clear();
    }
    draft.unallocated_adjustments.clear();
    draft
}

fn digest(draft: &ReceiptDraft) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(draft).map_err(|e| e.to_string())?)
    ))
}

fn record_edit(
    state: &mut State,
    before: Option<&ReceiptDraft>,
    after: &ReceiptDraft,
    action: &str,
) -> Result<(), String> {
    let mut fields = Vec::new();
    if let Some(old) = before {
        if old.purchase_date != after.purchase_date {
            fields.push("purchase_date".into());
        }
        if old.source_sha256 != after.source_sha256 {
            fields.push("source_fingerprint".into());
        }
        if old.items.len() != after.items.len() {
            fields.push("item_count".into());
        }
        if old.refunds.len() != after.refunds.len() {
            fields.push("refund_count".into());
        }
        if old.printed_subtotal_cents != after.printed_subtotal_cents {
            fields.push("printed_subtotal_cents".into());
        }
        if old.printed_tax_cents != after.printed_tax_cents {
            fields.push("printed_tax_cents".into());
        }
        if old.printed_total_cents != after.printed_total_cents {
            fields.push("printed_total_cents".into());
        }
        if old.unallocated_cents != after.unallocated_cents {
            fields.push("unallocated_cents".into());
        }
        if old.accepted_unknowns != after.accepted_unknowns {
            fields.push("accepted_unknowns".into());
        }
        for (index, (a, b)) in old.items.iter().zip(&after.items).enumerate() {
            if a.name != b.name {
                fields.push(format!("items[{index}].name"));
            }
            if a.quantity != b.quantity {
                fields.push(format!("items[{index}].quantity"));
            }
            if a.unit != b.unit {
                fields.push(format!("items[{index}].unit"));
            }
            if a.gross_cents != b.gross_cents {
                fields.push(format!("items[{index}].gross_cents"));
            }
            if a.discount_cents != b.discount_cents {
                fields.push(format!("items[{index}].discount_cents"));
            }
            if a.net_cents != b.net_cents {
                fields.push(format!("items[{index}].net_cents"));
            }
            if a.category != b.category {
                fields.push(format!("items[{index}].category"));
            }
            if a.expiry_date != b.expiry_date {
                fields.push(format!("items[{index}].expiry_date"));
            }
        }
        for (index, (a, b)) in old.refunds.iter().zip(&after.refunds).enumerate() {
            if a.name != b.name {
                fields.push(format!("refunds[{index}].name"));
            }
            if a.quantity != b.quantity {
                fields.push(format!("refunds[{index}].quantity"));
            }
            if a.unit != b.unit {
                fields.push(format!("refunds[{index}].unit"));
            }
            if a.amount_cents != b.amount_cents {
                fields.push(format!("refunds[{index}].amount_cents"));
            }
            if a.category != b.category {
                fields.push(format!("refunds[{index}].category"));
            }
            if a.original_line_id != b.original_line_id {
                fields.push(format!("refunds[{index}].original_line_id"));
            }
            if a.inventory_quantity_to_reverse != b.inventory_quantity_to_reverse {
                fields.push(format!("refunds[{index}].inventory_quantity_to_reverse"));
            }
        }
    } else {
        fields.push("reviewed_receipt".into());
    }
    state.receipt_edits.push(ReceiptEdit {
        sequence: state.receipt_edits.len() + 1,
        receipt_id: after.id.clone(),
        action: action.into(),
        before_sha256: before.map(digest).transpose()?.unwrap_or_default(),
        after_sha256: digest(after)?,
        before_total_cents: before.and_then(|d| d.printed_total_cents),
        after_total_cents: after.printed_total_cents,
        fields_changed: fields,
        related_receipt_id: None,
    });
    Ok(())
}

fn apply_receipt(state: &mut State, draft: &ReceiptDraft) -> Result<(), String> {
    let date = draft
        .purchase_date
        .as_deref()
        .ok_or("purchase date missing")?;
    for item in &draft.items {
        if !item.category.starts_with("food.") {
            continue;
        }
        let quantity = decimal(item.quantity.as_deref().ok_or("food quantity missing")?)?;
        if quantity <= Decimal::ZERO {
            return Err("food quantity must be positive".into());
        }
        if state.lots.iter().any(|lot| lot.id == item.line_id) {
            return Err("duplicate lot identifier".into());
        }
        state.lots.push(LotView {
            id: item.line_id.clone(),
            source_receipt_id: draft.id.clone(),
            name: item.name.clone(),
            category: item.category.clone(),
            quantity: quantity.normalize().to_string(),
            unit: item.unit.clone().ok_or("food unit missing")?,
            remaining_cost_cents: item.net_cents.unwrap_or(0),
            cost_unknown: item.net_cents.is_none(),
            zone: default_zone(&item.category).into(),
            expiry_date: item.expiry_date.clone(),
            purchase_date: date.into(),
        });
    }
    for refund in &draft.refunds {
        let reversal = decimal(
            refund
                .inventory_quantity_to_reverse
                .as_deref()
                .unwrap_or("0"),
        )?;
        if reversal < Decimal::ZERO {
            return Err("refund stock reversal cannot be negative".into());
        }
        let reviewed_quantity = refund.quantity.as_deref().map(decimal).transpose()?;
        if reversal > reviewed_quantity.unwrap_or(Decimal::ZERO) {
            return Err("refund reversal exceeds reviewed refund quantity".into());
        }
        let mut reversed_cost = 0;
        if let Some(original) = &refund.original_line_id {
            let lot = state
                .lots
                .iter_mut()
                .find(|lot| &lot.id == original)
                .ok_or("linked refund lot not found")?;
            if lot.name != refund.name
                || !lot.category.starts_with("food.")
                || refund.unit.as_deref() != Some(lot.unit.as_str())
            {
                return Err("refund does not match original food lot and unit".into());
            }
            if reversal > Decimal::ZERO {
                let available = decimal(&lot.quantity)?;
                reversed_cost = allocated_cost(lot.remaining_cost_cents, available, reversal)?;
                lot.quantity = (available - reversal).normalize().to_string();
                lot.remaining_cost_cents = lot
                    .remaining_cost_cents
                    .checked_sub(reversed_cost)
                    .ok_or("refund cost overflow")?;
            }
        } else if reversal > Decimal::ZERO {
            return Err("refund stock reversal needs an original lot".into());
        }
        state.refund_without_stock_reversal_cents = state
            .refund_without_stock_reversal_cents
            .checked_add(refund.amount_cents)
            .and_then(|v| v.checked_add(reversed_cost))
            .ok_or("refund adjustment overflow")?;
    }
    Ok(())
}

pub(crate) fn default_zone(category: &str) -> &'static str {
    match category {
        "food.dairy" | "food.produce" => "fridge",
        "food.meat" => "freezer",
        _ => "pantry",
    }
}

pub fn confirm_receipt(
    store: &mut Store,
    receipt: ReviewedReceipt,
) -> Result<ConfirmOutcome, String> {
    let draft = sanitized(review(receipt.draft).map_err(|e| e.to_string())?.draft);
    let mut state = store.load()?;
    if let Some(existing) = state
        .receipts
        .iter()
        .find(|existing| existing.source_sha256 == draft.source_sha256)
    {
        let incoming = digest(&draft)?;
        let last_reviewed = state
            .receipt_edits
            .iter()
            .rev()
            .find(|edit| edit.receipt_id == existing.id)
            .map(|edit| edit.after_sha256.as_str());
        if digest(existing)? == incoming || last_reviewed == Some(incoming.as_str()) {
            return Ok(ConfirmOutcome::Duplicate);
        }
        return Err("receipt source already imported. Correct that receipt, or review a unique receipt ID and deliberately confirm a separate purchase".into());
    }
    if state
        .receipts
        .iter()
        .any(|existing| existing.id == draft.id)
    {
        return Err("receipt identifier already exists; use correct_receipt".into());
    }
    apply_receipt(&mut state, &draft)?;
    record_edit(&mut state, None, &draft, "confirmed_review")?;
    state.receipts.push(draft);
    store.save(&state)?;
    Ok(ConfirmOutcome::Accepted)
}

pub fn matching_source_receipt(
    store: &Store,
    source_sha256: &str,
) -> Result<Option<(String, String)>, String> {
    Ok(store
        .load()?
        .receipts
        .iter()
        .find(|receipt| receipt.source_sha256 == source_sha256)
        .map(|receipt| {
            (
                receipt.id.clone(),
                receipt.purchase_date.clone().unwrap_or_default(),
            )
        }))
}

pub fn confirm_separate_purchase(
    store: &mut Store,
    receipt: ReviewedReceipt,
) -> Result<ConfirmOutcome, String> {
    let draft = sanitized(review(receipt.draft).map_err(|e| e.to_string())?.draft);
    let mut state = store.load()?;
    let related = state
        .receipts
        .iter()
        .find(|existing| existing.source_sha256 == draft.source_sha256)
        .ok_or("no matching source was imported; use ordinary confirmation")?;
    let related_id = related.id.clone();
    if state
        .receipts
        .iter()
        .any(|existing| existing.id == draft.id)
    {
        return Err("separate purchase needs a unique reviewed receipt ID".into());
    }
    if draft
        .items
        .iter()
        .any(|item| state.lots.iter().any(|lot| lot.id == item.line_id))
    {
        return Err("separate purchase needs unique item line IDs".into());
    }
    apply_receipt(&mut state, &draft)?;
    record_edit(&mut state, None, &draft, "separate_purchase_confirmed")?;
    if let Some(edit) = state.receipt_edits.last_mut() {
        edit.related_receipt_id = Some(related_id);
    }
    state.receipts.push(draft);
    store.save(&state)?;
    Ok(ConfirmOutcome::Accepted)
}

pub fn correct_receipt(
    store: &mut Store,
    receipt: ReviewedReceipt,
) -> Result<ConfirmOutcome, String> {
    let draft = sanitized(review(receipt.draft).map_err(|e| e.to_string())?.draft);
    let mut state = store.load()?;
    let index = state
        .receipts
        .iter()
        .position(|old| old.id == draft.id)
        .ok_or("receipt to correct not found")?;
    let old = state.receipts[index].clone();
    if digest(&old)? == digest(&draft)? {
        return Ok(ConfirmOutcome::Duplicate);
    }
    if old.source_sha256 != draft.source_sha256
        && state
            .receipts
            .iter()
            .enumerate()
            .any(|(i, other)| i != index && other.source_sha256 == draft.source_sha256)
    {
        return Err("corrected source is already imported by another receipt".into());
    }
    if state.receipts.iter().enumerate().any(|(i, other)| {
        i != index
            && other.refunds.iter().any(|r| {
                r.original_line_id
                    .as_ref()
                    .is_some_and(|id| old.items.iter().any(|item| &item.line_id == id))
            })
    }) {
        return Err(
            "cannot correct receipt after linked refund; resolve stock history first".into(),
        );
    }
    let stock_used = old
        .items
        .iter()
        .filter(|item| item.category.starts_with("food."))
        .try_fold(false, |used, item| {
            let lot = state
                .lots
                .iter()
                .find(|lot| lot.id == item.line_id)
                .ok_or("original lot missing")?;
            Ok::<bool, String>(
                used || decimal(&lot.quantity)?
                    != decimal(
                        item.quantity
                            .as_deref()
                            .ok_or("original quantity missing")?,
                    )?
                    || lot.remaining_cost_cents != item.net_cents.unwrap_or(0)
                    || lot.cost_unknown != item.net_cents.is_none(),
            )
        })?;
    if stock_used {
        let safe_fields = old.items.len() == draft.items.len()
            && old.refunds.is_empty()
            && draft.refunds.is_empty()
            && old.printed_subtotal_cents == draft.printed_subtotal_cents
            && old.printed_tax_cents == draft.printed_tax_cents
            && old.printed_total_cents == draft.printed_total_cents
            && old.unallocated_cents == draft.unallocated_cents
            && old.accepted_unknowns == draft.accepted_unknowns
            && old.items.iter().zip(&draft.items).all(|(a, b)| {
                a.line_id == b.line_id
                    && a.quantity == b.quantity
                    && a.unit == b.unit
                    && a.gross_cents == b.gross_cents
                    && a.discount_cents == b.discount_cents
                    && a.net_cents == b.net_cents
                    && a.category == b.category
            });
        if !safe_fields {
            return Err(
                "cost or quantity correction after stock use requires manual reconciliation".into(),
            );
        }
        let date = draft
            .purchase_date
            .as_deref()
            .ok_or("purchase date missing")?;
        for item in &draft.items {
            if !item.category.starts_with("food.") {
                continue;
            }
            let lot = state
                .lots
                .iter_mut()
                .find(|lot| lot.id == item.line_id)
                .ok_or("original lot missing")?;
            lot.name = item.name.clone();
            lot.expiry_date = item.expiry_date.clone();
            lot.purchase_date = date.into();
        }
        state.receipts[index] = draft.clone();
        record_edit(&mut state, Some(&old), &draft, "corrected_review")?;
        store.save(&state)?;
        return Ok(ConfirmOutcome::Corrected);
    }
    for item in &old.items {
        if !item.category.starts_with("food.") {
            continue;
        }
        let lot = state
            .lots
            .iter()
            .find(|lot| lot.id == item.line_id)
            .ok_or("original lot missing")?;
        if decimal(&lot.quantity)?
            != decimal(
                item.quantity
                    .as_deref()
                    .ok_or("original quantity missing")?,
            )?
            || lot.remaining_cost_cents != item.net_cents.unwrap_or(0)
            || lot.cost_unknown != item.net_cents.is_none()
        {
            return Err("cannot correct receipt after inventory use or stock adjustment".into());
        }
    }
    if old.refunds.iter().any(|refund| {
        refund
            .inventory_quantity_to_reverse
            .as_deref()
            .is_some_and(|q| q != "0")
    }) || draft.refunds.iter().any(|refund| {
        refund
            .inventory_quantity_to_reverse
            .as_deref()
            .is_some_and(|q| q != "0")
    }) {
        return Err("correction of stock reversing refunds requires manual reconciliation".into());
    }
    let old_unlinked = old
        .refunds
        .iter()
        .try_fold(0_i64, |sum, r| sum.checked_add(r.amount_cents))
        .ok_or("refund adjustment overflow")?;
    state.refund_without_stock_reversal_cents = state
        .refund_without_stock_reversal_cents
        .checked_sub(old_unlinked)
        .ok_or("refund adjustment overflow")?;
    state.lots.retain(|lot| lot.source_receipt_id != old.id);
    apply_receipt(&mut state, &draft)?;
    state.receipts[index] = draft.clone();
    record_edit(&mut state, Some(&old), &draft, "corrected_review")?;
    store.save(&state)?;
    Ok(ConfirmOutcome::Corrected)
}

pub fn snapshot(store: &Store) -> Result<AppSnapshot, String> {
    let state = store.load()?;
    let mut cash_flow_cents = 0_i64;
    let mut food_spending_cents = 0_i64;
    let mut household_goods_cents = 0_i64;
    let mut tax_cents = 0_i64;
    for receipt in &state.receipts {
        cash_flow_cents = cash_flow_cents
            .checked_add(receipt.printed_total_cents.unwrap_or(0))
            .ok_or("cash flow overflow")?;
        tax_cents = tax_cents
            .checked_add(receipt.printed_tax_cents.unwrap_or(0))
            .ok_or("tax total overflow")?;
        for item in &receipt.items {
            if item.category.starts_with("food.") {
                food_spending_cents = food_spending_cents
                    .checked_add(item.net_cents.unwrap_or(0))
                    .ok_or("food total overflow")?;
            } else if item.category.starts_with("household.") {
                household_goods_cents = household_goods_cents
                    .checked_add(item.net_cents.unwrap_or(0))
                    .ok_or("household total overflow")?;
            }
        }
        for refund in &receipt.refunds {
            if refund.category.starts_with("food.") {
                food_spending_cents = food_spending_cents
                    .checked_add(refund.amount_cents)
                    .ok_or("food refund overflow")?;
            } else if refund.category.starts_with("household.") {
                household_goods_cents = household_goods_cents
                    .checked_add(refund.amount_cents)
                    .ok_or("household refund overflow")?;
            }
        }
    }
    let consumed_cost_cents = state
        .cost_events
        .iter()
        .filter(|event| event.kind == "consume")
        .map(|event| event.amount_cents)
        .try_fold(0_i64, |sum, n| sum.checked_add(n))
        .ok_or("consumed cost overflow")?;
    let discarded_cost_cents = state
        .cost_events
        .iter()
        .filter(|event| event.kind == "discard")
        .map(|event| event.amount_cents)
        .try_fold(0_i64, |sum, n| sum.checked_add(n))
        .ok_or("discarded cost overflow")?;
    let unknown_consumption_count = state
        .cost_events
        .iter()
        .filter(|e| e.kind == "consume" && e.cost_unknown)
        .count();
    let unknown_discard_count = state
        .cost_events
        .iter()
        .filter(|e| e.kind == "discard" && e.cost_unknown)
        .count();
    let remaining_food_cost_cents = state
        .lots
        .iter()
        .filter(|lot| lot.category.starts_with("food."))
        .map(|lot| lot.remaining_cost_cents)
        .try_fold(0_i64, |sum, n| sum.checked_add(n))
        .ok_or("remaining cost overflow")?;
    let receipt_items = state
        .receipts
        .iter()
        .flat_map(|receipt| {
            receipt.items.iter().map(|item| ReceiptItemView {
                receipt_id: receipt.id.clone(),
                line_id: item.line_id.clone(),
                name: item.name.clone(),
                category: item.category.clone(),
                net_cents: item.net_cents,
            })
        })
        .collect();
    Ok(AppSnapshot {
        receipt_count: state.receipts.len(),
        cash_flow_cents,
        food_spending_cents,
        household_goods_cents,
        tax_cents,
        consumed_cost_cents,
        discarded_cost_cents,
        unknown_consumption_count,
        unknown_discard_count,
        remaining_food_cost_cents,
        unpriced_lot_count: state.lots.iter().filter(|lot| lot.cost_unknown).count(),
        refund_without_stock_reversal_cents: state.refund_without_stock_reversal_cents,
        lots: state.lots,
        shopping: state.shopping,
        receipt_items,
        category_edits: state.category_edits,
        receipt_edits: state.receipt_edits,
    })
}

pub fn recategorize_item(store: &mut Store, line_id: &str, category: &str) -> Result<(), String> {
    let new_is_food = category.starts_with("food.");
    if !new_is_food && !category.starts_with("household.") {
        return Err("category must begin food. or household.".into());
    }
    let mut state = store.load()?;
    let (receipt_index, item_index) = state
        .receipts
        .iter()
        .enumerate()
        .find_map(|(ri, receipt)| {
            receipt
                .items
                .iter()
                .position(|item| item.line_id == line_id)
                .map(|ii| (ri, ii))
        })
        .ok_or("receipt item not found")?;
    let receipt_id = state.receipts[receipt_index].id.clone();
    let purchase_date = state.receipts[receipt_index]
        .purchase_date
        .clone()
        .ok_or("purchase date missing")?;
    let item = state.receipts[receipt_index].items[item_index].clone();
    if item.category == category {
        return Ok(());
    }
    let old_is_food = item.category.starts_with("food.");
    if old_is_food && !new_is_food {
        if state
            .receipts
            .iter()
            .flat_map(|receipt| &receipt.refunds)
            .any(|refund| refund.original_line_id.as_deref() == Some(line_id))
        {
            return Err("cannot change category after a linked refund".into());
        }
        let index = state
            .lots
            .iter()
            .position(|lot| lot.id == line_id)
            .ok_or("food lot missing")?;
        let lot = &state.lots[index];
        if decimal(&lot.quantity)? != decimal(item.quantity.as_deref().ok_or("quantity missing")?)?
            || lot.remaining_cost_cents != item.net_cents.unwrap_or(0)
            || lot.cost_unknown != item.net_cents.is_none()
        {
            return Err("cannot change food category after stock use or adjustment".into());
        }
        state.lots.remove(index);
    } else if !old_is_food && new_is_food {
        if state.lots.iter().any(|lot| lot.id == line_id) {
            return Err("lot identifier already exists".into());
        }
        state.lots.push(LotView {
            id: item.line_id.clone(),
            source_receipt_id: receipt_id.clone(),
            name: item.name.clone(),
            category: category.into(),
            quantity: item.quantity.clone().ok_or("quantity missing")?,
            unit: item.unit.clone().ok_or("unit missing")?,
            remaining_cost_cents: item.net_cents.unwrap_or(0),
            cost_unknown: item.net_cents.is_none(),
            zone: default_zone(category).into(),
            expiry_date: item.expiry_date.clone(),
            purchase_date,
        });
    } else if old_is_food {
        let lot = state
            .lots
            .iter_mut()
            .find(|lot| lot.id == line_id)
            .ok_or("food lot missing")?;
        lot.category = category.into();
    }
    state.receipts[receipt_index].items[item_index].category = category.into();
    state.category_edits.push(CategoryEdit {
        sequence: state.category_edits.len() + 1,
        receipt_id,
        line_id: line_id.into(),
        from: item.category,
        to: category.into(),
    });
    store.save(&state)
}

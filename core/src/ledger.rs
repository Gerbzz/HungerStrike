use rust_decimal::Decimal;

use crate::{
    inventory::{allocated_cost, decimal},
    model::{AppSnapshot, ConfirmOutcome, LotView, ReviewedReceipt},
    receipt::review,
    store::Store,
};

fn default_zone(category: &str) -> &'static str {
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
    review(receipt.draft.clone()).map_err(|e| e.to_string())?;
    let draft = receipt.draft;
    let mut state = store.load()?;
    if state
        .receipts
        .iter()
        .any(|existing| existing.source_sha256 == draft.source_sha256)
    {
        return Ok(ConfirmOutcome::Duplicate);
    }
    if state
        .receipts
        .iter()
        .any(|existing| existing.id == draft.id)
    {
        return Err("receipt identifier already exists with different content".into());
    }
    let date = draft
        .purchase_date
        .clone()
        .ok_or_else(|| "purchase date missing".to_string())?;
    for item in &draft.items {
        if !item.category.starts_with("food.") {
            continue;
        }
        let quantity = item
            .quantity
            .as_deref()
            .ok_or_else(|| "food quantity missing".to_string())?;
        let parsed_quantity = decimal(quantity)?;
        if parsed_quantity <= Decimal::ZERO {
            return Err("food quantity must be positive".into());
        }
        let cost = item
            .net_cents
            .ok_or_else(|| "food cost missing".to_string())?;
        state.lots.push(LotView {
            id: item.line_id.clone(),
            source_receipt_id: draft.id.clone(),
            name: item.name.clone(),
            category: item.category.clone(),
            quantity: parsed_quantity.normalize().to_string(),
            unit: item
                .unit
                .clone()
                .ok_or_else(|| "food unit missing".to_string())?,
            remaining_cost_cents: cost,
            zone: default_zone(&item.category).to_string(),
            expiry_date: item.expiry_date.clone(),
            purchase_date: date.clone(),
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
        let mut reversed_cost = 0;
        if let Some(original) = &refund.original_line_id {
            let lot = state
                .lots
                .iter_mut()
                .find(|lot| &lot.id == original)
                .ok_or_else(|| "linked refund lot not found".to_string())?;
            if lot.name != refund.name || !lot.category.starts_with("food.") {
                return Err("refund does not match original food lot".into());
            }
            if reversal > Decimal::ZERO {
                let available = decimal(&lot.quantity)?;
                reversed_cost = allocated_cost(lot.remaining_cost_cents, available, reversal)?;
                lot.quantity = (available - reversal).normalize().to_string();
                lot.remaining_cost_cents -= reversed_cost;
            }
        } else if reversal > Decimal::ZERO {
            return Err("refund stock reversal needs an original lot".into());
        }
        state.refund_without_stock_reversal_cents += refund.amount_cents + reversed_cost;
    }
    state.receipts.push(draft);
    store.save(&state)?;
    Ok(ConfirmOutcome::Accepted)
}

pub fn snapshot(store: &Store) -> Result<AppSnapshot, String> {
    let state = store.load()?;
    let mut cash_flow_cents = 0;
    let mut food_spending_cents = 0;
    let mut household_goods_cents = 0;
    let mut tax_cents = 0;
    for receipt in &state.receipts {
        cash_flow_cents += receipt.printed_total_cents.unwrap_or(0);
        tax_cents += receipt.printed_tax_cents.unwrap_or(0);
        for item in &receipt.items {
            if item.category.starts_with("food.") {
                food_spending_cents += item.net_cents.unwrap_or(0);
            } else if item.category.starts_with("household.") {
                household_goods_cents += item.net_cents.unwrap_or(0);
            }
        }
        for refund in &receipt.refunds {
            if refund
                .original_line_id
                .as_ref()
                .and_then(|id| state.lots.iter().find(|lot| &lot.id == id))
                .is_some_and(|lot| lot.category.starts_with("food."))
            {
                food_spending_cents += refund.amount_cents;
            }
        }
    }
    let consumed_cost_cents = state
        .cost_events
        .iter()
        .filter(|event| event.kind == "consume")
        .map(|event| event.amount_cents)
        .sum();
    let discarded_cost_cents = state
        .cost_events
        .iter()
        .filter(|event| event.kind == "discard")
        .map(|event| event.amount_cents)
        .sum();
    let remaining_food_cost_cents = state
        .lots
        .iter()
        .filter(|lot| lot.category.starts_with("food."))
        .map(|lot| lot.remaining_cost_cents)
        .sum();
    Ok(AppSnapshot {
        receipt_count: state.receipts.len(),
        cash_flow_cents,
        food_spending_cents,
        household_goods_cents,
        tax_cents,
        consumed_cost_cents,
        discarded_cost_cents,
        remaining_food_cost_cents,
        refund_without_stock_reversal_cents: state.refund_without_stock_reversal_cents,
        lots: state.lots,
    })
}

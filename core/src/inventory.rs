use std::str::FromStr;

use rust_decimal::{prelude::ToPrimitive, Decimal, RoundingStrategy};

use crate::{
    model::InventoryAction,
    store::{CostEvent, Store},
};

pub(crate) fn decimal(value: &str) -> Result<Decimal, String> {
    Decimal::from_str(value).map_err(|_| format!("invalid quantity: {value}"))
}

pub(crate) fn allocated_cost(cost: i64, available: Decimal, taken: Decimal) -> Result<i64, String> {
    if available <= Decimal::ZERO || taken < Decimal::ZERO || taken > available {
        return Err("quantity exceeds available stock".into());
    }
    if taken == available {
        return Ok(cost);
    }
    (Decimal::from(cost) * taken / available)
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .ok_or_else(|| "cost allocation overflow".into())
}

pub fn record_inventory_action(store: &mut Store, action: InventoryAction) -> Result<(), String> {
    let mut state = store.load()?;
    let is_consume = matches!(&action, InventoryAction::Consume { .. });
    match action {
        InventoryAction::Consume { lot_id, quantity }
        | InventoryAction::Discard { lot_id, quantity } => {
            let kind = if is_consume { "consume" } else { "discard" };
            let lot = state
                .lots
                .iter_mut()
                .find(|lot| lot.id == lot_id)
                .ok_or_else(|| "lot not found".to_string())?;
            let available = decimal(&lot.quantity)?;
            let taken = decimal(&quantity)?;
            if taken <= Decimal::ZERO {
                return Err("quantity must be positive".into());
            }
            let cost = allocated_cost(lot.remaining_cost_cents, available, taken)?;
            lot.quantity = (available - taken).normalize().to_string();
            lot.remaining_cost_cents -= cost;
            state.cost_events.push(CostEvent {
                kind: kind.to_string(),
                amount_cents: cost,
            });
        }
        InventoryAction::Move { lot_id, zone } => {
            if !["fridge", "freezer", "pantry"].contains(&zone.as_str()) {
                return Err("unknown kitchen zone".into());
            }
            let lot = state
                .lots
                .iter_mut()
                .find(|lot| lot.id == lot_id)
                .ok_or_else(|| "lot not found".to_string())?;
            lot.zone = zone;
        }
    }
    store.save(&state)
}

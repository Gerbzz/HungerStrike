use hungerstrike_core::{
    confirm_receipt, parse_text, record_inventory_action, review, snapshot, ConfirmOutcome,
    InventoryAction, Store,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/receipts/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn r001() -> hungerstrike_core::ReviewedReceipt {
    review(parse_text(&fixture("R001-purchase.txt"))).unwrap()
}

fn r002_reversing_milk(quantity: &str) -> hungerstrike_core::ReviewedReceipt {
    let mut draft = parse_text(&fixture("R002-refund.txt"));
    draft.refunds[0].original_line_id = Some("R001-L02".into());
    draft.refunds[0].inventory_quantity_to_reverse = Some(quantity.into());
    review(draft).unwrap()
}

#[test]
fn confirmations_are_atomic_idempotent_and_reconcile_to_fixture_totals() {
    let mut store = Store::open_in_memory().unwrap();
    assert_eq!(
        confirm_receipt(&mut store, r001()).unwrap(),
        ConfirmOutcome::Accepted
    );
    assert_eq!(
        confirm_receipt(&mut store, r002_reversing_milk("1")).unwrap(),
        ConfirmOutcome::Accepted
    );
    assert_eq!(
        confirm_receipt(&mut store, r001()).unwrap(),
        ConfirmOutcome::Duplicate
    );
    let before_actions = snapshot(&store).unwrap();
    assert_eq!(before_actions.receipt_count, 2);
    assert_eq!(before_actions.cash_flow_cents, 2725);
    assert_eq!(before_actions.food_spending_cents, 1711);
    assert_eq!(before_actions.household_goods_cents, 948);
    assert_eq!(before_actions.tax_cents, 66);
    assert_eq!(
        before_actions
            .lots
            .iter()
            .find(|x| x.id == "R001-L02")
            .unwrap()
            .quantity,
        "0"
    );

    record_inventory_action(
        &mut store,
        InventoryAction::Consume {
            lot_id: "R001-L03".into(),
            quantity: "6".into(),
        },
    )
    .unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Discard {
            lot_id: "R001-L05".into(),
            quantity: "2".into(),
        },
    )
    .unwrap();
    let after = snapshot(&store).unwrap();
    assert_eq!(after.cash_flow_cents, 2725);
    assert_eq!(after.consumed_cost_cents, 175);
    assert_eq!(after.discarded_cost_cents, 120);
    assert_eq!(after.remaining_food_cost_cents, 1416);
    assert_eq!(
        after
            .lots
            .iter()
            .find(|x| x.id == "R001-L03")
            .unwrap()
            .quantity,
        "6"
    );
    assert_eq!(
        after
            .lots
            .iter()
            .find(|x| x.id == "R001-L05")
            .unwrap()
            .quantity,
        "3"
    );
}

#[test]
fn refund_after_consumption_does_not_remove_unavailable_stock() {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, r001()).unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Consume {
            lot_id: "R001-L02".into(),
            quantity: "1".into(),
        },
    )
    .unwrap();
    confirm_receipt(&mut store, r002_reversing_milk("0")).unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(
        state
            .lots
            .iter()
            .find(|x| x.id == "R001-L02")
            .unwrap()
            .quantity,
        "0"
    );
    assert_eq!(state.consumed_cost_cents, 429);
    assert_eq!(state.refund_without_stock_reversal_cents, -429);
}

#[test]
fn unlinked_refund_still_reduces_food_spending_without_changing_stock() {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, r001()).unwrap();
    let refund = review(parse_text(&fixture("R002-refund.txt"))).unwrap();
    confirm_receipt(&mut store, refund).unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(state.food_spending_cents, 1711);
    assert_eq!(state.refund_without_stock_reversal_cents, -429);
    assert_eq!(
        state
            .lots
            .iter()
            .find(|x| x.id == "R001-L02")
            .unwrap()
            .quantity,
        "1"
    );
}

#[test]
fn invalid_refund_reversal_rolls_back_entire_receipt() {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, r001()).unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Consume {
            lot_id: "R001-L02".into(),
            quantity: "1".into(),
        },
    )
    .unwrap();
    assert!(confirm_receipt(&mut store, r002_reversing_milk("1")).is_err());
    let state = snapshot(&store).unwrap();
    assert_eq!(state.receipt_count, 1);
    assert_eq!(state.cash_flow_cents, 1769);
}

#[test]
fn confirmed_receipts_survive_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.db");
    {
        let mut store = Store::open(&path).unwrap();
        confirm_receipt(&mut store, r001()).unwrap();
    }
    let reopened = Store::open(&path).unwrap();
    assert_eq!(snapshot(&reopened).unwrap().cash_flow_cents, 1769);
}

#[test]
fn failed_sqlite_write_preserves_previous_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.db");
    {
        let mut store = Store::open(&path).unwrap();
        confirm_receipt(&mut store, r001()).unwrap();
    }
    let mut read_only = Store::open_read_only(&path).unwrap();
    assert!(confirm_receipt(&mut read_only, r002_reversing_milk("1")).is_err());
    let reopened = Store::open(&path).unwrap();
    let state = snapshot(&reopened).unwrap();
    assert_eq!(state.receipt_count, 1);
    assert_eq!(state.cash_flow_cents, 1769);
}

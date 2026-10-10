use hungerstrike_core::{
    confirm_receipt, confirm_separate_purchase, correct_receipt, export_backup, parse_text,
    preview_import, recategorize_item, record_inventory_action, review, snapshot, suggest_meals,
    ConfirmOutcome, InventoryAction, RecipeCollection, Store,
};

const ORDINARY: &str = "ACME GROCERY\n123 MAIN ST\nDATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29\nVISA ENDING 1234\n";

#[test]
fn ordinary_receipt_does_not_turn_metadata_or_payment_into_items() {
    let draft = parse_text(ORDINARY);
    assert_eq!(draft.purchase_date.as_deref(), Some("2026-10-03"));
    assert_eq!(draft.items.len(), 1);
    assert_eq!(draft.items[0].name, "milk 2%");
    assert!(draft.items.iter().all(|item| !item.raw.contains("1234")));
}

#[test]
fn quantity_first_market_layout_is_reviewable_and_reconciles() {
    let raw = "FRESH MARKET\nDATE: 10/03/2026\n2 MILK 2% 8.58\n1.25 LB BANANAS @ 0.59 0.74\nSUB TOTAL 9.32\nSALES TAX 0.00\nTOTAL DUE 9.32";
    let draft = parse_text(raw);
    assert_eq!(draft.purchase_date.as_deref(), Some("2026-10-03"));
    assert_eq!(draft.items.len(), 2);
    assert_eq!(
        (
            draft.items[0].name.as_str(),
            draft.items[0].quantity.as_deref(),
            draft.items[0].unit.as_deref()
        ),
        ("milk 2%", Some("2"), Some("each"))
    );
    assert_eq!(
        (
            draft.items[1].name.as_str(),
            draft.items[1].quantity.as_deref(),
            draft.items[1].unit.as_deref()
        ),
        ("bananas", Some("1.25"), Some("lb"))
    );
    assert_eq!(draft.known_subtotal_cents, 932);
    assert!(review(draft).is_ok());
}

#[test]
fn quantity_first_at_price_layout_and_incomplete_line_are_not_omitted() {
    let draft = parse_text("DATE 2026-10-03\n2 @ 1.79 PASTA 3.58\nPASTA DISC -1.79\n3 CHKN BRST\nSUBTOTAL 1.79\nTAX 0.00\nTOTAL 1.79");
    assert_eq!(draft.items.len(), 2);
    assert_eq!(
        (
            draft.items[0].name.as_str(),
            draft.items[0].quantity.as_deref(),
            draft.items[0].net_cents
        ),
        ("pasta", Some("2"), Some(179))
    );
    assert_eq!(draft.items[1].name, "chicken breast");
    assert_eq!(draft.items[1].quantity.as_deref(), Some("3"));
    assert_eq!(draft.items[1].net_cents, None);
    assert!(draft.items[1]
        .uncertainty
        .iter()
        .any(|x| x.contains("amount")));
    assert!(review(draft).is_err());
}

#[test]
fn short_quantity_first_line_is_visible_for_manual_resolution() {
    let draft = parse_text("DATE 2026-10-03\n2 MILK\nSUBTOTAL 0.00\nTAX 0.00\nTOTAL 0.00");
    assert_eq!(draft.items.len(), 1);
    assert_eq!(draft.items[0].quantity.as_deref(), Some("2"));
    assert_eq!(draft.items[0].net_cents, None);
    assert!(review(draft).is_err());
}

#[test]
fn common_dollar_amounts_and_unfamiliar_product_remain_reviewable() {
    let draft = parse_text(
        "MARKET\nDATE 2026-10-03\nBREAD 1 EA $3.49\nSUBTOTAL $3.49\nTAX $0.00\nTOTAL $3.49",
    );
    assert_eq!(draft.items.len(), 1);
    assert_eq!(draft.items[0].name, "bread");
    assert_eq!(draft.items[0].net_cents, Some(349));
    assert_eq!(draft.items[0].category, "unclassified");
    assert!(review(draft).is_err());
}

#[test]
fn payment_footer_filter_keeps_legitimate_nonfood_product_lines() {
    let draft = parse_text("DATE 2026-10-03\nCARDAMOM 1 EA 3.49\nVISA GIFT CARD 1 EA 25.00\nSUBTOTAL 28.49\nTAX 0.00\nTOTAL 28.49\nVISA ENDING 1234");
    assert_eq!(draft.items.len(), 2);
    assert_eq!(draft.items[0].name, "cardamom");
    assert_eq!(draft.items[1].name, "visa gift card");
}

#[test]
fn count_unit_is_not_assumed_to_mean_eggs_for_every_product() {
    let draft =
        parse_text("DATE 2026-10-03\nTISSUES 12 CT 3.49\nSUBTOTAL 3.49\nTAX 0.00\nTOTAL 3.49");
    assert_eq!(draft.items[0].unit.as_deref(), Some("count"));
}

#[test]
fn implausible_quantity_is_rejected_before_cost_allocation() {
    let mut draft = parse_text(
        "FIXTURE ID BIGQ DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29",
    );
    draft.items[0].quantity = Some("999999999999999999".into());
    assert!(review(draft).is_err());
}

#[test]
fn oversized_money_never_panics_or_wraps() {
    let raw =
        "DATE 2026-10-03\nMILK 2% 1 GAL 92233720368547758.08\nSUBTOTAL 0.00\nTAX 0.00\nTOTAL 0.00";
    let result = std::panic::catch_unwind(|| parse_text(raw));
    assert!(result.is_ok());
    assert!(review(result.unwrap()).is_err());
}

#[test]
fn refund_reversal_cannot_exceed_reviewed_refund_quantity() {
    let mut store = Store::open_in_memory().unwrap();
    let mut buy = parse_text(
        "FIXTURE ID BUY DATE 2026-10-03\nMILK 2% 2 GAL 8.58\nSUBTOTAL 8.58\nTAX 0.00\nTOTAL 8.58",
    );
    buy.items[0].line_id = "BUY-L01".into();
    confirm_receipt(&mut store, review(buy).unwrap()).unwrap();
    let mut refund = parse_text("FIXTURE ID REF DATE 2026-10-03\nRETURN MILK 2% 1 GAL -4.29\nSUBTOTAL -4.29\nTAX 0.00\nTOTAL -4.29");
    refund.refunds[0].original_line_id = Some("BUY-L01".into());
    refund.refunds[0].inventory_quantity_to_reverse = Some("2".into());
    assert!(
        review(refund.clone()).is_err()
            || confirm_receipt(&mut store, review(refund).unwrap()).is_err()
    );
    assert_eq!(snapshot(&store).unwrap().lots[0].quantity, "2");
}

#[test]
fn raw_payment_text_is_not_persisted() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("private.db");
    let mut store = Store::open(&db).unwrap();
    let mut draft = parse_text(ORDINARY);
    draft.id = "PRIVATE".into();
    draft.items[0].raw.push_str(" VISA ENDING 1234");
    assert_eq!(
        confirm_receipt(&mut store, review(draft).unwrap()).unwrap(),
        ConfirmOutcome::Accepted
    );
    drop(store);
    let bytes = std::fs::read(db).unwrap();
    assert!(!bytes
        .windows(b"VISA ENDING 1234".len())
        .any(|w| w == b"VISA ENDING 1234"));
}

#[test]
fn explicit_correction_replaces_spending_atomically_and_records_audit() {
    let mut store = Store::open_in_memory().unwrap();
    let source =
        "FIXTURE ID EDIT DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29";
    let original = review(parse_text(source)).unwrap();
    assert_eq!(
        confirm_receipt(&mut store, original).unwrap(),
        ConfirmOutcome::Accepted
    );
    let mut changed = parse_text(source);
    changed.items[0].name = "milk corrected".into();
    changed.items[0].gross_cents = Some(399);
    changed.items[0].net_cents = Some(399);
    changed.printed_subtotal_cents = Some(399);
    changed.printed_total_cents = Some(399);
    let changed = review(changed).unwrap();
    assert!(confirm_receipt(&mut store, changed.clone()).is_err());
    assert_eq!(snapshot(&store).unwrap().cash_flow_cents, 429);
    assert_eq!(
        correct_receipt(&mut store, changed.clone()).unwrap(),
        ConfirmOutcome::Corrected
    );
    let state = snapshot(&store).unwrap();
    assert_eq!(
        (
            state.receipt_count,
            state.cash_flow_cents,
            state.food_spending_cents
        ),
        (1, 399, 399)
    );
    assert_eq!(state.lots[0].name, "milk corrected");
    assert_eq!(state.receipt_edits.len(), 2);
    assert!(state.receipt_edits[1]
        .fields_changed
        .contains(&"items[0].net_cents".into()));
    assert_eq!(
        correct_receipt(&mut store, changed).unwrap(),
        ConfirmOutcome::Duplicate
    );
}

#[test]
fn whitespace_variation_is_same_import() {
    let mut store = Store::open_in_memory().unwrap();
    let original =
        "FIXTURE ID W DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29";
    let variant = " FIXTURE  ID W DATE 2026-10-03 \r\n MILK 2%   1 GAL 4.29\r\nSUBTOTAL 4.29\r\nTAX 0.00\r\nTOTAL 4.29\r\n";
    confirm_receipt(&mut store, review(parse_text(original)).unwrap()).unwrap();
    assert_eq!(
        confirm_receipt(&mut store, review(parse_text(variant)).unwrap()).unwrap(),
        ConfirmOutcome::Duplicate
    );
    assert_eq!(snapshot(&store).unwrap().receipt_count, 1);
}

#[test]
fn repeated_source_needs_deliberate_unique_identity_and_is_audited() {
    let source = "DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29";
    let mut store = Store::open_in_memory().unwrap();
    let mut first = parse_text(source);
    first.id = "FIRST".into();
    first.items[0].line_id = "FIRST-L01".into();
    confirm_receipt(&mut store, review(first).unwrap()).unwrap();

    let mut second = parse_text(source);
    second.id = "SECOND".into();
    second.purchase_date = Some("2026-10-04".into());
    second.items[0].line_id = "SECOND-L01".into();
    assert!(confirm_receipt(&mut store, review(second.clone()).unwrap()).is_err());
    assert_eq!(snapshot(&store).unwrap().receipt_count, 1);
    assert_eq!(
        confirm_separate_purchase(&mut store, review(second.clone()).unwrap()).unwrap(),
        ConfirmOutcome::Accepted
    );
    assert!(confirm_separate_purchase(&mut store, review(second).unwrap()).is_err());
    let snap = snapshot(&store).unwrap();
    assert_eq!(
        (snap.receipt_count, snap.cash_flow_cents, snap.lots.len()),
        (2, 858, 2)
    );
    assert!(serde_json::to_value(&snap.receipt_edits[0])
        .unwrap()
        .get("related_receipt_id")
        .is_none());
    assert_eq!(snap.receipt_edits[1].action, "separate_purchase_confirmed");
    assert_eq!(
        snap.receipt_edits[1].related_receipt_id.as_deref(),
        Some("FIRST")
    );

    let backup = export_backup(&store).unwrap().to_bytes().unwrap();
    let empty = Store::open_in_memory().unwrap();
    assert_eq!(
        preview_import(&empty, &backup).unwrap().incoming_receipts,
        2
    );
}

#[test]
fn separate_purchase_rejects_default_or_colliding_identity() {
    let source = "DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29";
    let mut store = Store::open_in_memory().unwrap();
    let first = parse_text(source);
    confirm_receipt(&mut store, review(first.clone()).unwrap()).unwrap();
    assert!(confirm_separate_purchase(&mut store, review(first.clone()).unwrap()).is_err());
    let mut colliding_lines = first;
    colliding_lines.id = "NEW".into();
    assert!(confirm_separate_purchase(&mut store, review(colliding_lines).unwrap()).is_err());
    assert_eq!(snapshot(&store).unwrap().receipt_count, 1);
}

#[test]
fn unknown_cost_requires_explicit_reconciliation_and_meal_cost_stays_unknown() {
    let mut store = Store::open_in_memory().unwrap();
    let mut draft = parse_text(
        "FIXTURE ID U DATE 2026-10-03\nMILK 2% 1 GAL ??\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29",
    );
    assert!(review(draft.clone()).is_err());
    draft.accepted_unknowns = true;
    draft.unallocated_cents = Some(429);
    confirm_receipt(&mut store, review(draft).unwrap()).unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(state.unpriced_lot_count, 1);
    let recipes: RecipeCollection = serde_json::from_str(r#"{"recipes":[{"id":"milk","name":"Milk","servings":1,"ingredients":[{"name":"milk 2%","quantity":"1","unit":"gal"}],"steps":[]}]}"#).unwrap();
    let meals = suggest_meals(&state, &recipes.recipes, "2026-10-04").unwrap();
    assert!(meals[0].estimate_cents.is_none());
}

#[test]
fn unknown_cost_item_can_be_recategorized_before_stock_use() {
    let mut store = Store::open_in_memory().unwrap();
    let mut draft = parse_text(
        "FIXTURE ID U2 DATE 2026-10-03\nMILK 2% 1 GAL ??\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29",
    );
    draft.accepted_unknowns = true;
    draft.unallocated_cents = Some(429);
    confirm_receipt(&mut store, review(draft).unwrap()).unwrap();
    recategorize_item(&mut store, "U2-L01", "household.misc").unwrap();
    assert_eq!(snapshot(&store).unwrap().unpriced_lot_count, 0);
    recategorize_item(&mut store, "U2-L01", "food.pantry").unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(state.unpriced_lot_count, 1);
    assert_eq!(state.cash_flow_cents, 429);
    assert_eq!(state.category_edits.len(), 2);
}

#[test]
fn malformed_fixture_can_be_reconciled_with_explicit_adjustment() {
    let raw = include_str!("fixtures/receipts/R003-uncertain.txt");
    let mut draft = parse_text(raw);
    let onion = draft
        .items
        .iter_mut()
        .find(|item| item.name == "green onion")
        .unwrap();
    onion.quantity = Some("1".into());
    onion.unit = Some("each".into());
    onion.gross_cents = Some(89);
    onion.net_cents = Some(89);
    draft.unallocated_cents = Some(-189);
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, review(draft).unwrap()).unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(
        (state.cash_flow_cents, state.food_spending_cents),
        (983, 1172)
    );
    let budget =
        hungerstrike_core::budget_view(&store, "2026-10-05", hungerstrike_core::Period::Week, 0)
            .unwrap();
    assert_eq!(budget.reconciliation_cents, -189);
}

#[test]
fn exact_import_remains_duplicate_after_later_category_edit() {
    let mut store = Store::open_in_memory().unwrap();
    let source =
        "FIXTURE ID CAT DATE 2026-10-03\nMILK 2% 1 GAL 4.29\nSUBTOTAL 4.29\nTAX 0.00\nTOTAL 4.29";
    confirm_receipt(&mut store, review(parse_text(source)).unwrap()).unwrap();
    recategorize_item(&mut store, "CAT-L01", "food.pantry").unwrap();
    assert_eq!(
        confirm_receipt(&mut store, review(parse_text(source)).unwrap()).unwrap(),
        ConfirmOutcome::Duplicate
    );
    assert_eq!(snapshot(&store).unwrap().food_spending_cents, 429);
}

#[test]
fn payment_footer_variation_does_not_duplicate_purchase() {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(
        &mut store,
        review(parse_text(ORDINARY).tap_id("P")).unwrap(),
    )
    .unwrap();
    let variant = ORDINARY.replace("VISA ENDING 1234", "VISA ENDING 5678");
    assert_eq!(
        confirm_receipt(
            &mut store,
            review(parse_text(&variant).tap_id("P")).unwrap()
        )
        .unwrap(),
        ConfirmOutcome::Duplicate
    );
}

trait SetId {
    fn tap_id(self, id: &str) -> Self;
}
impl SetId for hungerstrike_core::ReceiptDraft {
    fn tap_id(mut self, id: &str) -> Self {
        self.id = id.into();
        self
    }
}

#[test]
fn correction_after_partial_consumption_is_blocked_without_changing_totals() {
    let mut store = Store::open_in_memory().unwrap();
    let source =
        "FIXTURE ID USE DATE 2026-10-03\nMILK 2% 2 GAL 8.00\nSUBTOTAL 8.00\nTAX 0.00\nTOTAL 8.00";
    confirm_receipt(&mut store, review(parse_text(source)).unwrap()).unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Consume {
            lot_id: "USE-L01".into(),
            quantity: "0.5".into(),
        },
    )
    .unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Discard {
            lot_id: "USE-L01".into(),
            quantity: "0.5".into(),
        },
    )
    .unwrap();
    let mut changed = parse_text(source);
    changed.items[0].gross_cents = Some(1000);
    changed.items[0].net_cents = Some(1000);
    changed.printed_subtotal_cents = Some(1000);
    changed.printed_total_cents = Some(1000);
    assert!(correct_receipt(&mut store, review(changed).unwrap()).is_err());
    let state = snapshot(&store).unwrap();
    assert_eq!((state.receipt_count, state.cash_flow_cents), (1, 800));
    assert_eq!(
        (
            state.consumed_cost_cents,
            state.discarded_cost_cents,
            state.remaining_food_cost_cents
        ),
        (200, 200, 400)
    );
    assert_eq!(state.lots[0].quantity, "1");
}

#[test]
fn nonfinancial_correction_after_use_preserves_cost_history() {
    let mut store = Store::open_in_memory().unwrap();
    let source =
        "FIXTURE ID LABEL DATE 2026-10-03\nMILK 2% 2 GAL 8.00\nSUBTOTAL 8.00\nTAX 0.00\nTOTAL 8.00";
    confirm_receipt(&mut store, review(parse_text(source)).unwrap()).unwrap();
    record_inventory_action(
        &mut store,
        InventoryAction::Consume {
            lot_id: "LABEL-L01".into(),
            quantity: "1".into(),
        },
    )
    .unwrap();
    let mut changed = parse_text(source);
    changed.items[0].name = "milk 2% corrected".into();
    changed.items[0].expiry_date = Some("2026-10-12".into());
    assert_eq!(
        correct_receipt(&mut store, review(changed).unwrap()).unwrap(),
        ConfirmOutcome::Corrected
    );
    let state = snapshot(&store).unwrap();
    assert_eq!(
        (state.consumed_cost_cents, state.remaining_food_cost_cents),
        (400, 400)
    );
    assert_eq!(state.lots[0].quantity, "1");
    assert_eq!(state.lots[0].name, "milk 2% corrected");
    assert_eq!(state.lots[0].expiry_date.as_deref(), Some("2026-10-12"));
}

use hungerstrike_core::{parse_text, review, DraftStatus};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/receipts/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn r001_parses_weighted_bogo_and_nonfood_without_expiry() {
    let draft = parse_text(&fixture("R001-purchase.txt"));
    assert_eq!(draft.id, "R001");
    assert_eq!(draft.purchase_date.as_deref(), Some("2026-10-03"));
    assert_eq!(draft.known_subtotal_cents, 1741);
    assert_eq!(draft.printed_total_cents, Some(1769));
    assert_eq!(draft.status, DraftStatus::PendingConfirmation);
    let bananas = draft.items.iter().find(|x| x.name == "bananas").unwrap();
    assert_eq!(
        (bananas.quantity.as_deref(), bananas.unit.as_deref()),
        (Some("1.25"), Some("lb"))
    );
    let pasta = draft.items.iter().find(|x| x.name == "pasta").unwrap();
    assert_eq!(
        (pasta.gross_cents, pasta.discount_cents, pasta.net_cents),
        (Some(358), -179, Some(179))
    );
    let soap = draft.items.iter().find(|x| x.name == "dish soap").unwrap();
    assert_eq!(soap.category, "household.cleaning");
    assert!(draft.items.iter().all(|x| x.expiry_date.is_none()));
}

#[test]
fn r002_retains_refund_separately_from_purchases() {
    let draft = parse_text(&fixture("R002-refund.txt"));
    assert_eq!(draft.known_subtotal_cents, 918);
    assert_eq!(draft.printed_total_cents, Some(956));
    assert_eq!(draft.refunds.len(), 1);
    assert_eq!(draft.refunds[0].amount_cents, -429);
    assert_eq!(draft.refunds[0].name, "milk 2%");
    let yogurt = draft.items.iter().find(|x| x.name == "yogurt").unwrap();
    assert_eq!(
        (yogurt.gross_cents, yogurt.discount_cents, yogurt.net_cents),
        (Some(375), -75, Some(300))
    );
}

#[test]
fn r003_marks_malformed_lines_and_total_mismatch_pending() {
    let draft = parse_text(&fixture("R003-uncertain.txt"));
    assert_eq!(draft.known_subtotal_cents, 1083);
    assert_eq!(draft.printed_subtotal_cents, Some(983));
    assert_eq!(draft.status, DraftStatus::NeedsResolution);
    assert!(draft.uncertainty.iter().any(|x| x.contains("mismatch")));
    let onions = draft
        .items
        .iter()
        .find(|x| x.name == "green onion")
        .unwrap();
    assert!(onions.quantity.is_none());
    assert!(onions.net_cents.is_none());
    assert!(draft.items.iter().all(|x| x.expiry_date.is_none()));
}

#[test]
fn review_accepts_reconciled_receipt_and_blocks_unresolved_one() {
    assert!(review(parse_text(&fixture("R001-purchase.txt"))).is_ok());
    assert!(review(parse_text(&fixture("R003-uncertain.txt"))).is_err());
}

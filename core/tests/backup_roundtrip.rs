use hungerstrike_core::{
    apply_import, confirm_receipt, export_backup, parse_text, preview_import, restore_last_import,
    review, snapshot, ApplyOutcome, ReplaceOrReconcile, Store,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/receipts/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn seed(name: &str) -> Store {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, review(parse_text(&fixture(name))).unwrap()).unwrap();
    store
}

#[test]
fn versioned_backup_omits_raw_payment_text_and_round_trips() {
    let mut source = Store::open_in_memory().unwrap();
    let mut receipt = parse_text(&fixture("R001-purchase.txt"));
    receipt.items[0].raw = "PAYMENT CARD 4111 1111 1111 1111".into();
    confirm_receipt(&mut source, review(receipt).unwrap()).unwrap();
    let bytes = export_backup(&source).unwrap().to_bytes().unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("hungerstrike-local-backup"));
    assert!(!text.contains("PAYMENT CARD"));
    assert!(!text.contains("4111"));
    let mut target = Store::open_in_memory().unwrap();
    let preview = preview_import(&target, &bytes).unwrap();
    assert_eq!(preview.incoming_receipts, 1);
    assert_eq!(
        apply_import(&mut target, &bytes, ReplaceOrReconcile::Replace).unwrap(),
        ApplyOutcome::Imported
    );
    assert_eq!(snapshot(&target).unwrap().cash_flow_cents, 1769);
}

#[test]
fn corrupt_or_incompatible_backup_never_changes_current_data() {
    let mut target = seed("R001-purchase.txt");
    let bytes = export_backup(&seed("R002-refund.txt"))
        .unwrap()
        .to_bytes()
        .unwrap();
    let corrupt = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("food.produce", "food.meat")
        .into_bytes();
    assert!(preview_import(&target, &corrupt).is_err());
    assert!(apply_import(&mut target, &corrupt, ReplaceOrReconcile::Replace).is_err());
    let future = String::from_utf8(bytes)
        .unwrap()
        .replace("\"version\": 1", "\"version\": 999")
        .into_bytes();
    assert!(apply_import(&mut target, &future, ReplaceOrReconcile::Replace).is_err());
    assert_eq!(snapshot(&target).unwrap().cash_flow_cents, 1769);
}

#[test]
fn replace_has_a_local_restore_point_and_reconcile_is_duplicate_safe() {
    let mut target = seed("R001-purchase.txt");
    let second = export_backup(&seed("R002-refund.txt"))
        .unwrap()
        .to_bytes()
        .unwrap();
    assert_eq!(
        apply_import(&mut target, &second, ReplaceOrReconcile::Reconcile).unwrap(),
        ApplyOutcome::Imported
    );
    assert_eq!(snapshot(&target).unwrap().cash_flow_cents, 2725);
    assert_eq!(
        apply_import(&mut target, &second, ReplaceOrReconcile::Reconcile).unwrap(),
        ApplyOutcome::Duplicate
    );
    let first = export_backup(&seed("R001-purchase.txt"))
        .unwrap()
        .to_bytes()
        .unwrap();
    assert_eq!(
        apply_import(&mut target, &first, ReplaceOrReconcile::Replace).unwrap(),
        ApplyOutcome::Imported
    );
    assert_eq!(snapshot(&target).unwrap().cash_flow_cents, 1769);
    restore_last_import(&mut target).unwrap();
    assert_eq!(snapshot(&target).unwrap().cash_flow_cents, 2725);
}

#[test]
fn failed_replace_write_preserves_the_existing_database() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inventory.sqlite");
    {
        let mut writable = Store::open(&path).unwrap();
        confirm_receipt(
            &mut writable,
            review(parse_text(&fixture("R001-purchase.txt"))).unwrap(),
        )
        .unwrap();
    }
    let bytes = export_backup(&seed("R002-refund.txt"))
        .unwrap()
        .to_bytes()
        .unwrap();
    let mut read_only = Store::open_read_only(&path).unwrap();
    assert!(apply_import(&mut read_only, &bytes, ReplaceOrReconcile::Replace).is_err());
    let reopened = Store::open(&path).unwrap();
    assert_eq!(snapshot(&reopened).unwrap().cash_flow_cents, 1769);
}

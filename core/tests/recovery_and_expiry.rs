//! Local database recovery, reopening, and estimated-expiry persistence.
//! All receipts are the SYNTHETIC fixtures in tests/fixtures/receipts.

use hungerstrike_core::{
    confirm_receipt, estimate_expiry, parse_text, record_inventory_action, review, snapshot,
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

#[test]
fn healthy_database_reopens_without_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    {
        let (mut store, note) = Store::open_or_recover(&path).unwrap();
        assert!(note.is_none());
        confirm_receipt(&mut store, r001()).unwrap();
    }
    for _ in 0..3 {
        let (store, note) = Store::open_or_recover(&path).unwrap();
        assert!(note.is_none());
        assert_eq!(snapshot(&store).unwrap().cash_flow_cents, 1769);
    }
}

#[test]
fn corrupt_file_is_preserved_and_a_fresh_store_is_usable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let garbage = b"SYNTHETIC CORRUPT DATABASE BYTES - not sqlite".repeat(200);
    std::fs::write(&path, &garbage).unwrap();
    let (mut store, note) = Store::open_or_recover(&path).unwrap();
    let note = note.expect("corruption must be reported");
    assert!(note.preserved_copy.exists());
    assert_eq!(std::fs::read(&note.preserved_copy).unwrap(), garbage);
    assert!(!note.reason.is_empty());
    assert_eq!(snapshot(&store).unwrap().receipt_count, 0);
    confirm_receipt(&mut store, r001()).unwrap();
    drop(store);
    let (reopened, again) = Store::open_or_recover(&path).unwrap();
    assert!(again.is_none(), "recovered store must reopen cleanly");
    assert_eq!(snapshot(&reopened).unwrap().receipt_count, 1);
}

#[test]
fn unreadable_payload_is_preserved_not_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    {
        let (mut store, _) = Store::open_or_recover(&path).unwrap();
        confirm_receipt(&mut store, r001()).unwrap();
    }
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE app_state SET payload = '{truncated' WHERE id = 1",
                [],
            )
            .unwrap();
    }
    let (store, note) = Store::open_or_recover(&path).unwrap();
    let note = note.expect("unreadable payload must be reported");
    assert_eq!(snapshot(&store).unwrap().receipt_count, 0);
    let preserved = rusqlite::Connection::open(&note.preserved_copy).unwrap();
    let payload: String = preserved
        .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(payload, "{truncated");
}

#[test]
fn interrupted_uncommitted_save_is_rolled_back_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    {
        let (mut store, _) = Store::open_or_recover(&path).unwrap();
        confirm_receipt(&mut store, r001()).unwrap();
    }
    {
        // A writer that dies before COMMIT: the transaction is abandoned.
        let mut connection = rusqlite::Connection::open(&path).unwrap();
        let transaction = connection.transaction().unwrap();
        transaction
            .execute(
                "UPDATE app_state SET payload = 'half-written' WHERE id = 1",
                [],
            )
            .unwrap();
        drop(transaction);
    }
    let (store, note) = Store::open_or_recover(&path).unwrap();
    assert!(note.is_none());
    assert_eq!(snapshot(&store).unwrap().cash_flow_cents, 1769);
}

#[test]
fn data_from_a_newer_version_is_not_set_aside() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    {
        Store::open_or_recover(&path).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE app_state SET payload = replace(payload, '\"schema_version\":1', '\"schema_version\":9') WHERE id = 1",
                [],
            )
            .unwrap();
    }
    let error = Store::open_or_recover(&path).err().expect("newer data");
    assert!(error.contains("unsupported local data version 9"));
    assert!(path.exists());
    let siblings = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(siblings, 1, "nothing may be renamed for a newer version");
}

#[test]
fn receipts_never_carry_expiry_and_estimates_stay_labelled() {
    let draft = parse_text(&fixture("R001-purchase.txt"));
    assert!(draft
        .items
        .iter()
        .all(|item| item.expiry_date.is_none() && !item.expiry_estimated));

    let mut draft = draft;
    let date = draft.purchase_date.clone().unwrap();
    let milk = draft
        .items
        .iter_mut()
        .find(|item| item.name == "milk 2%")
        .unwrap();
    let estimate = estimate_expiry(&milk.name, &milk.category, Some("fridge"), &date).unwrap();
    milk.expiry_date = Some(estimate.date.clone());
    milk.expiry_estimated = true;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let lot_id = {
        let (mut store, _) = Store::open_or_recover(&path).unwrap();
        confirm_receipt(&mut store, review(draft).unwrap()).unwrap();
        let lots = snapshot(&store).unwrap().lots;
        let milk = lots.iter().find(|lot| lot.name == "milk 2%").unwrap();
        assert_eq!(milk.expiry_date.as_deref(), Some(estimate.date.as_str()));
        assert!(milk.expiry_estimated);
        let spinach = lots.iter().find(|lot| lot.name == "spinach").unwrap();
        assert!(spinach.expiry_date.is_none() && !spinach.expiry_estimated);
        milk.id.clone()
    };
    let (mut store, _) = Store::open_or_recover(&path).unwrap();
    let reopened = snapshot(&store).unwrap();
    assert!(
        reopened
            .lots
            .iter()
            .find(|lot| lot.id == lot_id)
            .unwrap()
            .expiry_estimated
    );

    // A person reads the carton and enters the real date.
    assert!(record_inventory_action(
        &mut store,
        InventoryAction::SetExpiry {
            lot_id: lot_id.clone(),
            expiry_date: Some("10/12/2026".into()),
            estimated: false,
        },
    )
    .is_err());
    record_inventory_action(
        &mut store,
        InventoryAction::SetExpiry {
            lot_id: lot_id.clone(),
            expiry_date: Some("2026-10-12".into()),
            estimated: false,
        },
    )
    .unwrap();
    let lot = snapshot(&store)
        .unwrap()
        .lots
        .into_iter()
        .find(|lot| lot.id == lot_id)
        .unwrap();
    assert_eq!(lot.expiry_date.as_deref(), Some("2026-10-12"));
    assert!(!lot.expiry_estimated);

    // Clearing the date returns it to unknown.
    record_inventory_action(
        &mut store,
        InventoryAction::SetExpiry {
            lot_id: lot_id.clone(),
            expiry_date: None,
            estimated: true,
        },
    )
    .unwrap();
    let lot = snapshot(&store)
        .unwrap()
        .lots
        .into_iter()
        .find(|lot| lot.id == lot_id)
        .unwrap();
    assert!(lot.expiry_date.is_none() && !lot.expiry_estimated);
}

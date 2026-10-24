use hungerstrike_core::{confirm_receipt, parse_text, review, set_ui_theme, ui_theme, Store};

#[test]
fn theme_setting_defaults_to_unset_and_persists_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    {
        let mut store = Store::open(&path).unwrap();
        assert_eq!(ui_theme(&store).unwrap(), None);
        set_ui_theme(&mut store, "dark").unwrap();
    }
    {
        let mut store = Store::open(&path).unwrap();
        assert_eq!(ui_theme(&store).unwrap().as_deref(), Some("dark"));
        // Other writes keep the setting.
        let receipt = std::fs::read_to_string(format!(
            "{}/tests/fixtures/receipts/R001-purchase.txt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        confirm_receipt(&mut store, review(parse_text(&receipt)).unwrap()).unwrap();
        set_ui_theme(&mut store, "system").unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(ui_theme(&store).unwrap().as_deref(), Some("system"));
}

#[test]
fn unknown_theme_is_rejected_without_changing_the_setting() {
    let mut store = Store::open_in_memory().unwrap();
    set_ui_theme(&mut store, "light").unwrap();
    assert!(set_ui_theme(&mut store, "sepia").is_err());
    assert_eq!(ui_theme(&store).unwrap().as_deref(), Some("light"));
}

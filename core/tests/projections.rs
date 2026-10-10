use hungerstrike_core::{
    budget_view, confirm_receipt, parse_text, recategorize_item, record_inventory_action, review,
    set_budget, snapshot, suggest_meals, InventoryAction, Period, RecipeCollection, Store,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/receipts/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn category_edits_are_audited_and_reproject_food_without_changing_cash_flow() {
    let mut store = seeded();
    recategorize_item(&mut store, "R001-L04", "household.grocery").unwrap();
    let state = snapshot(&store).unwrap();
    assert_eq!(state.cash_flow_cents, 2725);
    assert_eq!(state.food_spending_cents, 1532);
    assert_eq!(state.household_goods_cents, 1127);
    assert!(!state.lots.iter().any(|lot| lot.id == "R001-L04"));
    assert_eq!(state.category_edits.len(), 1);
    assert_eq!(state.category_edits[0].from, "food.pantry");
    assert_eq!(state.category_edits[0].to, "household.grocery");
    recategorize_item(&mut store, "R001-L04", "food.pantry").unwrap();
    assert!(snapshot(&store)
        .unwrap()
        .lots
        .iter()
        .any(|lot| lot.id == "R001-L04"));
    assert!(recategorize_item(&mut store, "R001-L03", "household.grocery").is_err());
}

fn seeded() -> Store {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(
        &mut store,
        review(parse_text(&fixture("R001-purchase.txt"))).unwrap(),
    )
    .unwrap();
    let mut second = parse_text(&fixture("R002-refund.txt"));
    second.refunds[0].original_line_id = Some("R001-L02".into());
    second.refunds[0].inventory_quantity_to_reverse = Some("1".into());
    confirm_receipt(&mut store, review(second).unwrap()).unwrap();
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
    store
}

#[test]
fn original_recipes_show_complete_and_partial_costs_from_confirmed_lots() {
    let recipes: RecipeCollection =
        serde_json::from_str(include_str!("../../assets/recipes.json")).unwrap();
    let state = snapshot(&seeded()).unwrap();
    let results = suggest_meals(&state, &recipes.recipes, "2026-10-10").unwrap();
    let by_id = |id: &str| results.iter().find(|meal| meal.id == id).unwrap();
    assert_eq!(by_id("egg-spinach-scramble").estimate_cents, Some(118));
    assert_eq!(by_id("apple-yogurt-bowl").estimate_cents, Some(137));
    assert_eq!(by_id("tomato-pasta").estimate_cents, Some(90));
    assert!(!by_id("tomato-pasta").available);
    assert_eq!(by_id("chicken-rice-skillet").estimate_cents, Some(47));
    assert!(!by_id("chicken-rice-skillet").available);
    assert!(by_id("egg-spinach-scramble").expiry_unknown);
}

#[test]
fn only_confirmed_user_expiry_marks_an_ingredient_soon_expiring() {
    let recipes: RecipeCollection =
        serde_json::from_str(include_str!("../../assets/recipes.json")).unwrap();
    let mut state = snapshot(&seeded()).unwrap();
    state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "large eggs")
        .unwrap()
        .expiry_date = Some("2026-10-12".into());
    let results = suggest_meals(&state, &recipes.recipes, "2026-10-10").unwrap();
    let scramble = results
        .iter()
        .find(|meal| meal.id == "egg-spinach-scramble")
        .unwrap();
    assert!(scramble.soon_expiring.contains(&"large eggs".to_string()));
}

#[test]
fn week_start_changes_grouping_and_monthly_budget_uses_cash_flow_once() {
    let mut store = seeded();
    let monday = budget_view(&store, "2026-10-04", Period::Week, 0).unwrap();
    let sunday = budget_view(&store, "2026-10-04", Period::Week, 6).unwrap();
    assert_eq!(monday.cash_flow_cents, 2725);
    assert_eq!(sunday.cash_flow_cents, 956);
    set_budget(&mut store, Period::Month, Some(3000)).unwrap();
    let month = budget_view(&store, "2026-10-04", Period::Month, 0).unwrap();
    assert_eq!(month.cash_flow_cents, 2725);
    assert_eq!(month.food_spending_cents, 1711);
    assert_eq!(month.household_goods_cents, 948);
    assert_eq!(month.tax_cents, 66);
    assert_eq!(month.refunds_cents, -429);
    assert_eq!(month.remaining_budget_cents, Some(275));
    assert_eq!(snapshot(&store).unwrap().discarded_cost_cents, 120);
}

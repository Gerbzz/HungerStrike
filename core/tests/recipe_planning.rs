use hungerstrike_core::{
    add_recipe_missing_to_shopping, add_recipes_missing_to_shopping, apply_import, confirm_receipt,
    cook_recipe, export_backup, parse_text, record_inventory_action, review, snapshot,
    suggest_meals, InventoryAction, Recipe, RecipeCollection, RecipeIngredient, ReplaceOrReconcile,
    Store,
};

const RECIPES: &str = include_str!("../../assets/recipes.json");
const RECEIPT: &str = include_str!("fixtures/receipts/R001-purchase.txt");

#[test]
fn bundled_pantry_recipes_have_unique_ids_and_positive_ingredients() {
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    assert_eq!(collection.recipes.len(), 10);
    let mut ids = std::collections::HashSet::new();
    for meal in &collection.recipes {
        assert!(ids.insert(&meal.id));
        assert!(!meal.ingredients.is_empty());
        assert!(meal.ingredients.iter().all(|item| {
            item.quantity.parse::<rust_decimal::Decimal>().unwrap() > rust_decimal::Decimal::ZERO
        }));
    }
}

fn seeded() -> Store {
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, review(parse_text(RECEIPT)).unwrap()).unwrap();
    store
}

#[test]
fn recipe_matches_confirmed_food_across_freezer_and_pantry() {
    let mut store = seeded();
    for (name, zone) in [("large eggs", "freezer"), ("spinach", "pantry")] {
        let lot_id = snapshot(&store)
            .unwrap()
            .lots
            .iter()
            .find(|lot| lot.name == name)
            .unwrap()
            .id
            .clone();
        record_inventory_action(
            &mut store,
            InventoryAction::Move {
                lot_id,
                zone: zone.into(),
            },
        )
        .unwrap();
    }
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let meal = collection
        .recipes
        .iter()
        .find(|recipe| recipe.id == "egg-spinach-scramble")
        .unwrap();
    let result = suggest_meals(
        &snapshot(&store).unwrap(),
        std::slice::from_ref(meal),
        "2026-10-04",
    )
    .unwrap();
    assert!(result[0].available);
    assert_eq!(result[0].consumption.len(), 2);
}

#[test]
fn plan_next_prioritizes_fewer_missing_ingredients() {
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let suggested = suggest_meals(
        &snapshot(&seeded()).unwrap(),
        &collection.recipes,
        "2026-10-04",
    )
    .unwrap();
    let tomato = suggested
        .iter()
        .position(|meal| meal.id == "tomato-pasta")
        .unwrap();
    let chicken = suggested
        .iter()
        .position(|meal| meal.id == "chicken-rice-skillet")
        .unwrap();
    assert!(tomato < chicken);
}

#[test]
fn plan_next_prefers_more_usable_stock_when_missing_counts_tie() {
    let mut nearly_ready = recipe("spinach", "6", "oz");
    nearly_ready.id = "nearly-ready".into();
    nearly_ready.name = "Z nearly ready".into();
    let mut less_ready = recipe("spinach", "10", "oz");
    less_ready.id = "less-ready".into();
    less_ready.name = "A less ready".into();

    let suggestions = suggest_meals(
        &snapshot(&seeded()).unwrap(),
        &[less_ready, nearly_ready],
        "2026-10-04",
    )
    .unwrap();
    assert_eq!(suggestions[0].id, "nearly-ready");
    assert_eq!(suggestions[0].stock_coverage_percent, 83);
    assert_eq!(suggestions[1].stock_coverage_percent, 50);
}

#[test]
fn empty_recipe_cannot_be_suggested_as_ready() {
    let mut empty = recipe("spinach", "1", "oz");
    empty.ingredients.clear();
    assert_eq!(
        suggest_meals(&snapshot(&seeded()).unwrap(), &[empty], "2026-10-04").unwrap_err(),
        "recipe has no ingredients"
    );
}

#[test]
fn canned_bean_recipe_matches_a_counted_pantry_lot() {
    let mut state = snapshot(&seeded()).unwrap();
    let mut beans = state.lots[0].clone();
    beans.id = "synthetic-beans".into();
    beans.name = "canned black beans".into();
    beans.category = "food.pantry".into();
    beans.zone = "pantry".into();
    beans.quantity = "1".into();
    beans.unit = "each".into();
    beans.expiry_date = None;
    state.lots.push(beans);

    let meal = recipe("canned beans", "1", "can");
    let suggestion = suggest_meals(&state, &[meal], "2026-10-04")
        .unwrap()
        .remove(0);
    assert!(suggestion.available);
    assert_eq!(suggestion.stock_coverage_percent, 100);
    assert_eq!(suggestion.consumption[0].lot_id, "synthetic-beans");
}

#[test]
fn canned_beans_accept_tins_but_not_dry_names_or_mass_units() {
    let mut state = snapshot(&seeded()).unwrap();
    let mut beans = state.lots[0].clone();
    beans.id = "synthetic-tin".into();
    beans.name = "canned kidney beans".into();
    beans.category = "food.pantry".into();
    beans.zone = "pantry".into();
    beans.quantity = "1".into();
    beans.unit = "tin".into();
    beans.expiry_date = None;
    state.lots.push(beans);
    let meal = recipe("canned beans", "1", "can");

    assert!(suggest_meals(&state, std::slice::from_ref(&meal), "2026-10-04").unwrap()[0].available);
    state.lots.last_mut().unwrap().name = "dry kidney beans".into();
    assert!(
        !suggest_meals(&state, std::slice::from_ref(&meal), "2026-10-04").unwrap()[0].available
    );
    let beans = state.lots.last_mut().unwrap();
    beans.name = "canned beans".into();
    beans.unit = "lb".into();
    assert!(!suggest_meals(&state, &[meal], "2026-10-04").unwrap()[0].available);
}

#[test]
fn stock_coverage_averages_fractions_across_ingredients() {
    let mut meal = recipe("spinach", "6", "oz");
    meal.ingredients.push(RecipeIngredient {
        name: "pasta".into(),
        quantity: "1".into(),
        unit: "each".into(),
    });
    let result = suggest_meals(&snapshot(&seeded()).unwrap(), &[meal], "2026-10-04")
        .unwrap()
        .remove(0);
    assert!(!result.available);
    assert_eq!(result.stock_coverage_percent, 91);
    assert_eq!(result.ingredients[0].missing_quantity, "1");
    assert_eq!(result.ingredients[1].missing_quantity, "0");
}

fn recipe(name: &str, quantity: &str, unit: &str) -> Recipe {
    Recipe {
        id: "test-meal".into(),
        name: "Test meal".into(),
        servings: 1,
        ingredients: vec![RecipeIngredient {
            name: name.into(),
            quantity: quantity.into(),
            unit: unit.into(),
        }],
        steps: vec!["Prepare and cook safely.".into()],
    }
}

#[test]
fn insufficient_stock_stays_in_plan_next_and_cannot_be_cooked() {
    let mut store = seeded();
    let meal = recipe("spinach", "6", "oz");
    let suggestion = suggest_meals(&snapshot(&store).unwrap(), &[meal.clone()], "2026-10-04")
        .unwrap()
        .remove(0);
    assert!(!suggestion.available);
    assert_eq!(suggestion.ingredients[0].missing_quantity, "1");
    assert!(cook_recipe(&mut store, &meal, "2026-10-04").is_err());
    assert_eq!(
        snapshot(&store)
            .unwrap()
            .lots
            .iter()
            .find(|lot| lot.name == "spinach")
            .unwrap()
            .quantity,
        "5"
    );
}

#[test]
fn aliases_and_compatible_units_use_stock_but_incompatible_units_do_not() {
    let state = snapshot(&seeded()).unwrap();
    let meal = recipe("baby spinach", "0.125", "lb");
    let suggestion = suggest_meals(&state, &[meal], "2026-10-04")
        .unwrap()
        .remove(0);
    assert!(suggestion.available);
    assert_eq!(suggestion.ingredients[0].stock_used_quantity, "0.125");
    assert_eq!(suggestion.consumption[0].quantity, "2");
    assert_eq!(suggestion.consumption[0].unit, "oz");
    let incompatible = recipe("spinach", "1", "gal");
    assert!(!suggest_meals(&state, &[incompatible], "2026-10-04").unwrap()[0].available);
}

#[test]
fn usable_soon_expiring_stock_ranks_ahead_of_undated_stock() {
    let mut pasta = recipe("pasta", "1", "each");
    pasta.id = "pasta".into();
    pasta.name = "A pasta".into();
    let mut spinach = recipe("spinach", "1", "oz");
    spinach.id = "spinach".into();
    spinach.name = "Z spinach".into();
    let mut state = snapshot(&seeded()).unwrap();
    state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "spinach")
        .unwrap()
        .expiry_date = Some("2026-10-05".into());
    let suggestions =
        suggest_meals(&state, &[pasta.clone(), spinach.clone()], "2026-10-04").unwrap();
    assert_eq!(suggestions[0].id, "spinach");
    assert!(suggestions[0]
        .soon_expiring
        .contains(&"spinach".to_string()));
    state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "spinach")
        .unwrap()
        .expiry_date = Some("2026-10-03".into());
    let suggestions = suggest_meals(&state, &[pasta, spinach], "2026-10-04").unwrap();
    assert_eq!(suggestions[0].id, "pasta");
    assert!(
        !suggestions
            .iter()
            .find(|meal| meal.id == "spinach")
            .unwrap()
            .available
    );
}

#[test]
fn missing_quantities_are_saved_once_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stock.sqlite");
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let meal = collection
        .recipes
        .iter()
        .find(|recipe| recipe.id == "tomato-pasta")
        .unwrap();
    {
        let mut store = Store::open(&path).unwrap();
        confirm_receipt(&mut store, review(parse_text(RECEIPT)).unwrap()).unwrap();
        add_recipe_missing_to_shopping(&mut store, meal, "2026-10-04").unwrap();
        add_recipe_missing_to_shopping(&mut store, meal, "2026-10-04").unwrap();
    }
    let state = snapshot(&Store::open(&path).unwrap()).unwrap();
    assert_eq!(state.shopping.len(), 1);
    assert_eq!(state.shopping[0].name, "tomatoes");
    assert_eq!(state.shopping[0].quantity, "0.25");
    assert_eq!(state.shopping[0].unit, "lb");
}

#[test]
fn selected_meals_share_stock_and_repeat_saves_are_idempotent() {
    let mut store = seeded();
    let recipes: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|id| Recipe {
            id: id.into(),
            name: id.into(),
            servings: 1,
            ingredients: vec![RecipeIngredient {
                name: "spinach".into(),
                quantity: "3".into(),
                unit: "oz".into(),
            }],
            steps: vec!["Cook spinach.".into()],
        })
        .collect();
    assert!(
        suggest_meals(&snapshot(&store).unwrap(), &recipes, "2026-10-04")
            .unwrap()
            .iter()
            .all(|meal| meal.available)
    );
    let shopping = add_recipes_missing_to_shopping(&mut store, &recipes, "2026-10-04").unwrap();
    assert_eq!(shopping.len(), 1);
    assert_eq!(shopping[0].name, "spinach");
    assert_eq!(shopping[0].quantity, "1");
    assert_eq!(shopping[0].unit, "oz");
    let repeated = add_recipes_missing_to_shopping(&mut store, &recipes, "2026-10-04").unwrap();
    assert_eq!(repeated, shopping);
}

#[test]
fn selected_pantry_meals_sum_shared_missing_rice_without_duplicate_rows() {
    let mut store = seeded();
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let recipes: Vec<_> = collection
        .recipes
        .into_iter()
        .filter(|recipe| ["chicken-rice-skillet", "bean-rice-bowl"].contains(&recipe.id.as_str()))
        .collect();
    let shopping = add_recipes_missing_to_shopping(&mut store, &recipes, "2026-10-04").unwrap();
    let rice: Vec<_> = shopping.iter().filter(|item| item.name == "rice").collect();
    assert_eq!(rice.len(), 1);
    assert_eq!(rice[0].quantity, "0.5");
    assert_eq!(rice[0].unit, "lb");
    let repeated = add_recipes_missing_to_shopping(&mut store, &recipes, "2026-10-04").unwrap();
    assert_eq!(repeated, shopping);
}

#[test]
fn selected_meals_convert_compatible_units_before_subtracting_stock() {
    let mut store = seeded();
    let recipes: Vec<_> = [("pounds", "1", "lb"), ("ounces", "8", "oz")]
        .into_iter()
        .map(|(id, quantity, unit)| Recipe {
            id: id.into(),
            name: id.into(),
            servings: 1,
            ingredients: vec![RecipeIngredient {
                name: "bananas".into(),
                quantity: quantity.into(),
                unit: unit.into(),
            }],
            steps: vec!["Slice bananas.".into()],
        })
        .collect();
    let shopping = add_recipes_missing_to_shopping(&mut store, &recipes, "2026-10-04").unwrap();
    assert_eq!(shopping.len(), 1);
    assert_eq!(shopping[0].name, "bananas");
    assert_eq!(shopping[0].quantity, "0.25");
    assert_eq!(shopping[0].unit, "lb");
}

#[test]
fn shopping_list_survives_backup_reconciliation_with_matching_receipts() {
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let meal = collection
        .recipes
        .iter()
        .find(|recipe| recipe.id == "tomato-pasta")
        .unwrap();
    let mut source = seeded();
    add_recipe_missing_to_shopping(&mut source, meal, "2026-10-04").unwrap();
    let backup = export_backup(&source).unwrap().to_bytes().unwrap();
    let mut target = seeded();
    apply_import(&mut target, &backup, ReplaceOrReconcile::Reconcile).unwrap();
    assert_eq!(snapshot(&target).unwrap().shopping.len(), 1);
    assert_eq!(snapshot(&target).unwrap().receipt_count, 1);
}

#[test]
fn explicit_cook_consumes_all_previewed_lots_once() {
    let mut store = seeded();
    let collection: RecipeCollection = serde_json::from_str(RECIPES).unwrap();
    let meal = collection
        .recipes
        .iter()
        .find(|recipe| recipe.id == "egg-spinach-scramble")
        .unwrap();
    let preview = suggest_meals(&snapshot(&store).unwrap(), &[meal.clone()], "2026-10-04")
        .unwrap()
        .remove(0);
    assert!(preview.available);
    assert_eq!(preview.consumption.len(), 2);
    let before = snapshot(&store).unwrap();
    assert_eq!(before.consumed_cost_cents, 0);
    cook_recipe(&mut store, meal, "2026-10-04").unwrap();
    let after = snapshot(&store).unwrap();
    assert_eq!(
        after
            .lots
            .iter()
            .find(|lot| lot.name == "large eggs")
            .unwrap()
            .quantity,
        "10"
    );
    assert_eq!(
        after
            .lots
            .iter()
            .find(|lot| lot.name == "spinach")
            .unwrap()
            .quantity,
        "4"
    );
    assert!(after.consumed_cost_cents > 0);
}

#[test]
fn user_entered_unpadded_dates_use_chronological_lot_order() {
    let mut draft = parse_text(RECEIPT);
    draft
        .items
        .iter_mut()
        .find(|item| item.name == "spinach")
        .unwrap()
        .expiry_date = Some("2026-2-10".into());
    let mut store = Store::open_in_memory().unwrap();
    confirm_receipt(&mut store, review(draft).unwrap()).unwrap();

    let mut state = snapshot(&store).unwrap();
    let earlier = state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "spinach")
        .unwrap();
    earlier.quantity = "1".into();
    let mut later = earlier.clone();
    later.id = "later-spinach".into();
    later.expiry_date = Some("2026-10-05".into());
    state.lots.push(later);

    let suggestion = suggest_meals(&state, &[recipe("spinach", "1", "oz")], "2026-02-01")
        .unwrap()
        .remove(0);
    assert_eq!(suggestion.consumption[0].lot_id, "R001-L05");
}

#[test]
fn user_entered_unpadded_date_ranks_earlier_meal_first() {
    let mut state = snapshot(&seeded()).unwrap();
    state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "spinach")
        .unwrap()
        .expiry_date = Some("2026-2-10".into());
    state
        .lots
        .iter_mut()
        .find(|lot| lot.name == "pasta")
        .unwrap()
        .expiry_date = Some("2026-10-05".into());
    let mut early = recipe("spinach", "1", "oz");
    early.id = "early".into();
    early.name = "Z early".into();
    let mut late = recipe("pasta", "1", "each");
    late.id = "late".into();
    late.name = "A late".into();

    let suggestions = suggest_meals(&state, &[late, early], "2026-02-01").unwrap();
    assert_eq!(suggestions[0].id, "early");
}

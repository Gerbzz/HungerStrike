use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    inventory::{allocated_cost, decimal},
    ledger::snapshot,
    model::{AppSnapshot, LotView, ShoppingItem},
    store::{CostEvent, Store},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeCollection {
    pub recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub servings: u32,
    pub ingredients: Vec<RecipeIngredient>,
    pub steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngredientEstimate {
    pub name: String,
    pub required_quantity: String,
    pub unit: String,
    pub stock_used_quantity: String,
    pub estimated_cost_cents: i64,
    pub cost_unknown: bool,
    pub missing_quantity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MealSuggestion {
    pub id: String,
    pub name: String,
    pub servings: u32,
    pub steps: Vec<String>,
    pub available: bool,
    pub estimate_cents: Option<i64>,
    pub estimate_status: String,
    pub ingredients: Vec<IngredientEstimate>,
    pub consumption: Vec<LotUse>,
    pub soon_expiring: Vec<String>,
    pub expiry_unknown: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LotUse {
    pub lot_id: String,
    pub name: String,
    pub zone: String,
    pub quantity: String,
    pub unit: String,
    pub expiry_date: Option<String>,
}

#[derive(Clone)]
struct WorkingLot {
    lot: LotView,
    quantity: Decimal,
    cost_cents: i64,
}

fn canonical_name(name: &str) -> String {
    let normalized = name.trim().to_lowercase();
    match normalized.as_str() {
        "eggs" | "egg" | "large egg" | "large eggs" => "large eggs".into(),
        "baby spinach" | "spinach" => "spinach".into(),
        "tomato" | "tomatoes" => "tomatoes".into(),
        "apple" | "apples" => "apples".into(),
        "banana" | "bananas" => "bananas".into(),
        "white rice" | "rice" => "rice".into(),
        "pasta noodles" | "pasta" => "pasta".into(),
        _ => normalized,
    }
}

fn unit_scale(unit: &str) -> (String, Decimal) {
    match unit.trim().to_lowercase().as_str() {
        "lb" | "lbs" | "pound" | "pounds" => ("mass_oz".into(), Decimal::from(16)),
        "oz" | "ounce" | "ounces" => ("mass_oz".into(), Decimal::ONE),
        "gal" | "gallon" | "gallons" => ("volume_fl_oz".into(), Decimal::from(128)),
        "fl oz" | "fluid ounce" | "fluid ounces" => ("volume_fl_oz".into(), Decimal::ONE),
        "each" | "ea" | "ct" | "count" => ("each".into(), Decimal::ONE),
        "egg" | "eggs" => ("egg".into(), Decimal::ONE),
        other => (other.into(), Decimal::ONE),
    }
}

pub fn suggest_meals(
    snapshot: &AppSnapshot,
    recipes: &[Recipe],
    today: &str,
) -> Result<Vec<MealSuggestion>, String> {
    let today = NaiveDate::parse_from_str(today, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let mut suggestions = Vec::new();
    for recipe in recipes {
        let mut stock: Vec<WorkingLot> = snapshot
            .lots
            .iter()
            .filter(|lot| lot.category.starts_with("food."))
            .filter_map(|lot| {
                let quantity = decimal(&lot.quantity).ok()?;
                if quantity <= Decimal::ZERO {
                    return None;
                }
                if lot
                    .expiry_date
                    .as_deref()
                    .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
                    .is_some_and(|date| date < today)
                {
                    return None;
                }
                Some(WorkingLot {
                    lot: lot.clone(),
                    quantity,
                    cost_cents: lot.remaining_cost_cents,
                })
            })
            .collect();
        stock.sort_by_key(|entry| {
            (
                entry.lot.expiry_date.is_none(),
                entry.lot.expiry_date.clone(),
            )
        });
        let mut estimates = Vec::new();
        let mut consumption = Vec::new();
        let mut soon_expiring = Vec::new();
        let mut expiry_unknown = false;
        let mut total_cents = 0_i64;
        let mut estimate_unknown = false;
        let mut available = true;
        for ingredient in &recipe.ingredients {
            let required = decimal(&ingredient.quantity)?;
            if required <= Decimal::ZERO {
                return Err("recipe quantity must be positive".into());
            }
            let (required_unit, required_scale) = unit_scale(&ingredient.unit);
            let mut remaining = required * required_scale;
            let mut cost_cents = 0_i64;
            let mut ingredient_cost_unknown = false;
            for entry in stock.iter_mut().filter(|entry| {
                canonical_name(&entry.lot.name) == canonical_name(&ingredient.name)
                    && unit_scale(&entry.lot.unit).0 == required_unit
            }) {
                if remaining <= Decimal::ZERO {
                    break;
                }
                if entry.quantity <= Decimal::ZERO {
                    continue;
                }
                let (_, stock_scale) = unit_scale(&entry.lot.unit);
                let taken = remaining.min(entry.quantity * stock_scale) / stock_scale;
                if entry.lot.cost_unknown {
                    ingredient_cost_unknown = true;
                    estimate_unknown = true;
                }
                let cost = allocated_cost(entry.cost_cents, entry.quantity, taken)?;
                entry.quantity -= taken;
                entry.cost_cents -= cost;
                remaining -= taken * stock_scale;
                cost_cents += cost;
                consumption.push(LotUse {
                    lot_id: entry.lot.id.clone(),
                    name: entry.lot.name.clone(),
                    zone: entry.lot.zone.clone(),
                    quantity: taken.normalize().to_string(),
                    unit: entry.lot.unit.clone(),
                    expiry_date: entry.lot.expiry_date.clone(),
                });
                match entry
                    .lot
                    .expiry_date
                    .as_deref()
                    .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
                {
                    Some(date) if date <= today + Duration::days(3) => {
                        if !soon_expiring.contains(&ingredient.name) {
                            soon_expiring.push(ingredient.name.clone());
                        }
                    }
                    Some(_) => {}
                    None => expiry_unknown = true,
                }
            }
            if remaining > Decimal::ZERO {
                available = false;
            }
            total_cents = total_cents
                .checked_add(cost_cents)
                .ok_or("meal estimate overflow")?;
            estimates.push(IngredientEstimate {
                name: ingredient.name.clone(),
                required_quantity: required.normalize().to_string(),
                unit: ingredient.unit.clone(),
                stock_used_quantity: (required - remaining / required_scale)
                    .normalize()
                    .to_string(),
                estimated_cost_cents: cost_cents,
                cost_unknown: ingredient_cost_unknown,
                missing_quantity: (remaining / required_scale).normalize().to_string(),
            });
        }
        suggestions.push(MealSuggestion {
            id: recipe.id.clone(),
            name: recipe.name.clone(),
            servings: recipe.servings,
            steps: recipe.steps.clone(),
            available,
            estimate_cents: if estimate_unknown {
                None
            } else {
                Some(total_cents)
            },
            estimate_status: if estimate_unknown {
                "unknown_cost"
            } else if available {
                "complete"
            } else {
                "partial_missing_ingredient"
            }
            .into(),
            ingredients: estimates,
            consumption,
            soon_expiring,
            expiry_unknown,
        });
    }
    suggestions.sort_by_key(|meal| {
        let missing_count = meal
            .ingredients
            .iter()
            .filter(|ingredient| ingredient.missing_quantity != "0")
            .count();
        let soonest_expiry = meal
            .consumption
            .iter()
            .filter_map(|lot| lot.expiry_date.as_deref())
            .min()
            .unwrap_or("9999-12-31")
            .to_string();
        (
            !meal.available,
            if meal.available {
                meal.soon_expiring.is_empty()
            } else {
                false
            },
            if meal.available { 0 } else { missing_count },
            if meal.available {
                soonest_expiry
            } else {
                String::new()
            },
            meal.name.clone(),
        )
    });
    Ok(suggestions)
}

pub fn add_recipe_missing_to_shopping(
    store: &mut Store,
    recipe: &Recipe,
    today: &str,
) -> Result<Vec<ShoppingItem>, String> {
    let suggestion = suggest_meals(&snapshot(store)?, std::slice::from_ref(recipe), today)?
        .pop()
        .ok_or("recipe has no suggestion")?;
    let mut state = store.load()?;
    for ingredient in suggestion.ingredients {
        let missing = decimal(&ingredient.missing_quantity)?;
        if missing <= Decimal::ZERO {
            continue;
        }
        let (unit_kind, unit_factor) = unit_scale(&ingredient.unit);
        if let Some(existing) = state.shopping.iter_mut().find(|item| {
            canonical_name(&item.name) == canonical_name(&ingredient.name)
                && unit_scale(&item.unit).0 == unit_kind
        }) {
            let (_, existing_factor) = unit_scale(&existing.unit);
            let existing_quantity = decimal(&existing.quantity)?;
            existing.quantity = existing_quantity
                .max(missing * unit_factor / existing_factor)
                .normalize()
                .to_string();
        } else {
            state.shopping.push(ShoppingItem {
                name: ingredient.name,
                quantity: missing.normalize().to_string(),
                unit: ingredient.unit,
            });
        }
    }
    let shopping = state.shopping.clone();
    store.save(&state)?;
    Ok(shopping)
}

pub fn remove_shopping_item(store: &mut Store, name: &str, unit: &str) -> Result<(), String> {
    let mut state = store.load()?;
    let before = state.shopping.len();
    state
        .shopping
        .retain(|item| !(canonical_name(&item.name) == canonical_name(name) && item.unit == unit));
    if state.shopping.len() == before {
        return Err("shopping item not found".into());
    }
    store.save(&state)
}

pub fn cook_recipe(
    store: &mut Store,
    recipe: &Recipe,
    today: &str,
) -> Result<MealSuggestion, String> {
    if recipe.ingredients.is_empty() {
        return Err("recipe has no ingredients".into());
    }
    let suggestion = suggest_meals(&snapshot(store)?, std::slice::from_ref(recipe), today)?
        .pop()
        .ok_or("recipe has no suggestion")?;
    if !suggestion.available {
        return Err("not enough usable stock for this recipe".into());
    }
    let mut state = store.load()?;
    for use_lot in &suggestion.consumption {
        let lot = state
            .lots
            .iter_mut()
            .find(|lot| lot.id == use_lot.lot_id)
            .ok_or("recipe stock lot no longer exists")?;
        let available = decimal(&lot.quantity)?;
        let taken = decimal(&use_lot.quantity)?;
        let cost = allocated_cost(lot.remaining_cost_cents, available, taken)?;
        lot.quantity = (available - taken).normalize().to_string();
        lot.remaining_cost_cents -= cost;
        state.cost_events.push(CostEvent {
            kind: "consume".into(),
            amount_cents: cost,
            cost_unknown: lot.cost_unknown,
            lot_id: Some(lot.id.clone()),
            quantity: Some(taken.normalize().to_string()),
        });
    }
    store.save(&state)?;
    Ok(suggestion)
}

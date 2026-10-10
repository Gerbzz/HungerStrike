use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    inventory::{allocated_cost, decimal},
    model::{AppSnapshot, LotView},
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
    pub soon_expiring: Vec<String>,
    pub expiry_unknown: bool,
}

#[derive(Clone)]
struct WorkingLot {
    lot: LotView,
    quantity: Decimal,
    cost_cents: i64,
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
            let mut remaining = required;
            let mut cost_cents = 0_i64;
            let mut ingredient_cost_unknown = false;
            for entry in stock.iter_mut().filter(|entry| {
                entry.lot.name == ingredient.name && entry.lot.unit == ingredient.unit
            }) {
                if remaining <= Decimal::ZERO {
                    break;
                }
                if entry.quantity <= Decimal::ZERO {
                    continue;
                }
                let taken = remaining.min(entry.quantity);
                if entry.lot.cost_unknown {
                    ingredient_cost_unknown = true;
                    estimate_unknown = true;
                }
                let cost = allocated_cost(entry.cost_cents, entry.quantity, taken)?;
                entry.quantity -= taken;
                entry.cost_cents -= cost;
                remaining -= taken;
                cost_cents += cost;
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
                stock_used_quantity: (required - remaining).normalize().to_string(),
                estimated_cost_cents: cost_cents,
                cost_unknown: ingredient_cost_unknown,
                missing_quantity: remaining.normalize().to_string(),
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
            soon_expiring,
            expiry_unknown,
        });
    }
    suggestions.sort_by_key(|meal| {
        (
            !meal.available,
            meal.soon_expiring.is_empty(),
            meal.name.clone(),
        )
    });
    Ok(suggestions)
}

//! Rough, clearly labelled shelf-life estimates.
//!
//! Grocery receipts do not print expiry or best-by dates. HungerStrike never
//! invents one silently: an estimate is only created when a person asks for
//! it, it is stored with `expiry_estimated = true`, and every screen labels it
//! as an estimate. These are planning hints for unopened food stored as
//! indicated, not food-safety guidance; the package date always wins.

use chrono::{Days, NaiveDate};

/// A suggested expiry date and the reasoning shown to the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpiryEstimate {
    pub date: String,
    pub days: u64,
    pub basis: String,
}

/// Keyword table: (substring in the lower-case item name, zone, days).
/// The first match wins, so more specific words come first.
const BY_NAME: &[(&str, &str, u64)] = &[
    ("spinach", "fridge", 5),
    ("lettuce", "fridge", 5),
    ("salad", "fridge", 4),
    ("berr", "fridge", 4),
    ("banana", "pantry", 5),
    ("avocado", "pantry", 4),
    ("tomato", "pantry", 5),
    ("green onion", "fridge", 7),
    ("onion", "pantry", 30),
    ("potato", "pantry", 30),
    ("apple", "fridge", 28),
    ("milk", "fridge", 7),
    ("yogurt", "fridge", 14),
    ("egg", "fridge", 21),
    ("cheese", "fridge", 21),
    ("butter", "fridge", 30),
    ("chicken", "fridge", 2),
    ("beef", "fridge", 3),
    ("pork", "fridge", 3),
    ("fish", "fridge", 2),
    ("bread", "pantry", 5),
    ("juice", "fridge", 7),
    ("rice", "pantry", 365),
    ("pasta", "pantry", 365),
    ("oat", "pantry", 270),
    ("bean", "pantry", 365),
    ("flour", "pantry", 180),
];

/// Fallback by spending category when no name keyword matches.
fn by_category(category: &str) -> Option<u64> {
    match category {
        "food.produce" => Some(5),
        "food.dairy" => Some(7),
        "food.meat" => Some(2),
        "food.beverage" => Some(14),
        "food.pantry" => Some(180),
        _ => None,
    }
}

/// Estimate an expiry date for an item bought on `purchase_date`.
///
/// Freezer storage extends the estimate to 90 days for perishables. Returns
/// `None` for non-food categories or an unreadable purchase date: in that
/// case the expiry stays unknown rather than guessed.
pub fn estimate_expiry(
    name: &str,
    category: &str,
    zone: Option<&str>,
    purchase_date: &str,
) -> Option<ExpiryEstimate> {
    if !category.starts_with("food.") {
        return None;
    }
    let bought = NaiveDate::parse_from_str(purchase_date, "%Y-%m-%d").ok()?;
    let lower = name.to_ascii_lowercase();
    let (mut days, mut basis) = match BY_NAME.iter().find(|(word, _, _)| lower.contains(word)) {
        Some((word, usual_zone, days)) => (
            *days,
            format!("typical for {word} kept in the {usual_zone}"),
        ),
        None => (
            by_category(category)?,
            format!("typical for {}", category.trim_start_matches("food.")),
        ),
    };
    if zone == Some("freezer") && days < 90 {
        days = 90;
        basis = "typical for frozen food".into();
    }
    let date = bought.checked_add_days(Days::new(days))?;
    Some(ExpiryEstimate {
        date: date.to_string(),
        days,
        basis: format!("Estimate: {days} days after purchase, {basis}. Check the package."),
    })
}

/// Validate a user-entered date. Returns the canonical `YYYY-MM-DD` form.
pub fn parse_user_date(input: &str) -> Result<String, String> {
    NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d")
        .map(|date| date.to_string())
        .map_err(|_| "use a date like 2026-10-21".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_are_dated_from_purchase_and_explain_themselves() {
        let milk = estimate_expiry("milk 2%", "food.dairy", Some("fridge"), "2026-10-03").unwrap();
        assert_eq!(milk.date, "2026-10-10");
        assert!(milk.basis.starts_with("Estimate:"));
        assert!(milk.basis.contains("Check the package"));
        let rice = estimate_expiry("rice", "food.pantry", None, "2026-10-03").unwrap();
        assert_eq!(rice.days, 365);
    }

    #[test]
    fn freezer_extends_perishables_and_unknowns_stay_unknown() {
        let chicken =
            estimate_expiry("chicken breast", "food.meat", Some("freezer"), "2026-10-03").unwrap();
        assert_eq!(chicken.days, 90);
        assert_eq!(
            estimate_expiry("dish soap", "household.cleaning", None, "2026-10-03"),
            None
        );
        assert_eq!(
            estimate_expiry("mystery", "unclassified", None, "2026-10-03"),
            None
        );
        assert_eq!(
            estimate_expiry("milk", "food.dairy", None, "not a date"),
            None
        );
    }

    #[test]
    fn user_dates_are_validated() {
        assert_eq!(parse_user_date(" 2026-10-21 ").unwrap(), "2026-10-21");
        assert!(parse_user_date("10/21/2026").is_err());
        assert!(parse_user_date("2026-02-30").is_err());
    }
}

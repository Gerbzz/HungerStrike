use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::store::Store;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Period {
    Week,
    Month,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BudgetView {
    pub period: Period,
    pub start_date: String,
    pub end_date: String,
    pub receipt_count: usize,
    pub budget_cents: Option<i64>,
    pub remaining_budget_cents: Option<i64>,
    pub cash_flow_cents: i64,
    pub food_spending_cents: i64,
    pub household_goods_cents: i64,
    pub tax_cents: i64,
    pub gross_items_cents: i64,
    pub discounts_cents: i64,
    pub refunds_cents: i64,
    pub reconciliation_cents: i64,
}

pub fn set_budget(store: &mut Store, period: Period, cents: Option<i64>) -> Result<(), String> {
    if cents.is_some_and(|value| value < 0) {
        return Err("budget cannot be negative".into());
    }
    let mut state = store.load()?;
    match period {
        Period::Week => state.weekly_budget_cents = cents,
        Period::Month => state.monthly_budget_cents = cents,
    }
    store.save(&state)
}

pub fn budget_view(
    store: &Store,
    reference_date: &str,
    period: Period,
    week_start: u8,
) -> Result<BudgetView, String> {
    macro_rules! add_cents {
        ($field:expr, $value:expr) => {
            $field = $field
                .checked_add($value)
                .ok_or("budget accounting overflow")?
        };
    }
    if week_start > 6 {
        return Err("week start must be 0 (Monday) through 6 (Sunday)".into());
    }
    let date = NaiveDate::parse_from_str(reference_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let (start, end) = match period {
        Period::Week => {
            let weekday = date.weekday().num_days_from_monday() as i64;
            let offset = (weekday - i64::from(week_start)).rem_euclid(7);
            let start = date - Duration::days(offset);
            (start, start + Duration::days(6))
        }
        Period::Month => {
            let start =
                NaiveDate::from_ymd_opt(date.year(), date.month(), 1).ok_or("invalid month")?;
            let (year, month) = if date.month() == 12 {
                (date.year() + 1, 1)
            } else {
                (date.year(), date.month() + 1)
            };
            let next = NaiveDate::from_ymd_opt(year, month, 1).ok_or("invalid next month")?;
            (start, next - Duration::days(1))
        }
    };
    let state = store.load()?;
    let budget = match period {
        Period::Week => state.weekly_budget_cents,
        Period::Month => state.monthly_budget_cents,
    };
    let mut view = BudgetView {
        period,
        start_date: start.to_string(),
        end_date: end.to_string(),
        receipt_count: 0,
        budget_cents: budget,
        remaining_budget_cents: None,
        cash_flow_cents: 0,
        food_spending_cents: 0,
        household_goods_cents: 0,
        tax_cents: 0,
        gross_items_cents: 0,
        discounts_cents: 0,
        refunds_cents: 0,
        reconciliation_cents: 0,
    };
    for receipt in &state.receipts {
        let purchase = NaiveDate::parse_from_str(
            receipt
                .purchase_date
                .as_deref()
                .ok_or("receipt date missing")?,
            "%Y-%m-%d",
        )
        .map_err(|e| e.to_string())?;
        if purchase < start || purchase > end {
            continue;
        }
        view.receipt_count += 1;
        add_cents!(
            view.cash_flow_cents,
            receipt.printed_total_cents.ok_or("receipt total missing")?
        );
        add_cents!(
            view.tax_cents,
            receipt.printed_tax_cents.ok_or("receipt tax missing")?
        );
        for item in &receipt.items {
            add_cents!(view.gross_items_cents, item.gross_cents.unwrap_or(0));
            add_cents!(view.discounts_cents, item.discount_cents);
            if item.category.starts_with("food.") {
                add_cents!(view.food_spending_cents, item.net_cents.unwrap_or(0));
            }
            if item.category.starts_with("household.") {
                add_cents!(view.household_goods_cents, item.net_cents.unwrap_or(0));
            }
        }
        for refund in &receipt.refunds {
            add_cents!(view.refunds_cents, refund.amount_cents);
            if refund.category.starts_with("food.") {
                add_cents!(view.food_spending_cents, refund.amount_cents);
            }
            if refund.category.starts_with("household.") {
                add_cents!(view.household_goods_cents, refund.amount_cents);
            }
        }
    }
    let allocated = view
        .gross_items_cents
        .checked_add(view.discounts_cents)
        .and_then(|v| v.checked_add(view.refunds_cents))
        .and_then(|v| v.checked_add(view.tax_cents))
        .ok_or("budget reconciliation overflow")?;
    view.reconciliation_cents = view
        .cash_flow_cents
        .checked_sub(allocated)
        .ok_or("budget reconciliation overflow")?;
    view.remaining_budget_cents = budget
        .map(|value| {
            value
                .checked_sub(view.cash_flow_cents)
                .ok_or("remaining budget overflow")
        })
        .transpose()?;
    Ok(view)
}

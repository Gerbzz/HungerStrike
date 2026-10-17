//! Plain-language explanations of why a receipt draft cannot be confirmed
//! yet. `hungerstrike_core::review` remains the authority; these hints only
//! point the person at the field to fix.

use hungerstrike_core::{DraftItem, ReceiptDraft};

fn valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

pub fn line_problems(item: &DraftItem) -> Vec<&'static str> {
    let mut problems = Vec::new();
    if item.name.trim().is_empty() {
        problems.push("add a name");
    }
    match item.quantity.as_deref().map(|q| q.trim().parse::<f64>()) {
        Some(Ok(q)) if q > 0.0 && q <= 1_000_000.0 => {}
        _ => problems.push("quantity must be a positive number"),
    }
    if item.unit.as_deref().is_none_or(|u| u.trim().is_empty()) {
        problems.push("add a unit");
    }
    if !(item.category.starts_with("food.") || item.category.starts_with("household.")) {
        problems.push("choose a food or household category");
    }
    if item.gross_cents.is_none() {
        problems.push("price unknown");
    }
    if item.discount_cents > 0 {
        problems.push("discount must be zero or negative");
    }
    if item.net_cents.is_some_and(|net| net < 0) {
        problems.push("discount is larger than the price");
    }
    if item.expiry_date.as_deref().is_some_and(|d| !valid_date(d)) {
        problems.push("expiry must look like 2026-10-21");
    }
    problems
}

pub fn receipt_problems(draft: &ReceiptDraft) -> Vec<String> {
    let mut problems = Vec::new();
    if draft.id.trim().is_empty() {
        problems.push("Give the receipt an ID.".to_string());
    }
    if draft
        .purchase_date
        .as_deref()
        .is_none_or(|d| !valid_date(d))
    {
        problems.push("Enter the purchase date as YYYY-MM-DD.".to_string());
    }
    if draft.items.is_empty() && draft.refunds.is_empty() {
        problems.push("Add at least one line.".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    if draft.items.iter().any(|item| !seen.insert(&item.line_id)) {
        problems.push("Two lines share the same line ID.".to_string());
    }
    let items: Option<i64> = draft
        .items
        .iter()
        .try_fold(0_i64, |sum, item| sum.checked_add(item.net_cents?));
    let refunds: i64 = draft.refunds.iter().map(|refund| refund.amount_cents).sum();
    match (items, draft.printed_subtotal_cents) {
        (Some(items), Some(printed)) => {
            let counted = items + refunds + draft.unallocated_cents.unwrap_or(0);
            if counted != printed {
                problems.push(format!(
                    "Lines add up to {} but the printed subtotal is {}.",
                    crate::ui::money(counted),
                    crate::ui::money(printed)
                ));
            }
        }
        (_, None) => problems.push("Enter the printed subtotal.".to_string()),
        (None, _) => {
            if !draft.accepted_unknowns || draft.unallocated_cents.is_none() {
                problems.push(
                    "Some prices are unknown: enter them, or accept unknown costs with an explicit reconciliation amount."
                        .to_string(),
                );
            }
        }
    }
    match draft.printed_tax_cents {
        None => problems.push("Enter the printed tax (0.00 if none).".to_string()),
        Some(tax) if tax < 0 => problems.push("Tax cannot be negative.".to_string()),
        _ => {}
    }
    match (
        draft.printed_subtotal_cents,
        draft.printed_tax_cents,
        draft.printed_total_cents,
    ) {
        (Some(subtotal), Some(tax), Some(total)) if subtotal + tax != total => {
            problems.push(format!(
                "Subtotal plus tax is {} but the printed total is {}.",
                crate::ui::money(subtotal + tax),
                crate::ui::money(total)
            ))
        }
        (_, _, None) => problems.push("Enter the printed total.".to_string()),
        _ => {}
    }
    if !draft.unallocated_adjustments.is_empty() && draft.unallocated_cents.is_none() {
        problems.push(
            "Resolve or remove the unassigned adjustments, or enter a reconciliation amount."
                .to_string(),
        );
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use hungerstrike_core::{parse_text, review};

    const RECEIPT: &str = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");

    #[test]
    fn reconciled_fixture_has_no_hints() {
        let draft = parse_text(RECEIPT);
        assert!(review(draft.clone()).is_ok());
        assert!(receipt_problems(&draft).is_empty());
        assert!(draft
            .items
            .iter()
            .all(|item| line_problems(item).is_empty()));
    }

    #[test]
    fn hints_name_the_field_to_fix() {
        let mut draft = parse_text(RECEIPT);
        draft.items[1].gross_cents = Some(500);
        draft.items[1].net_cents = Some(500);
        draft.items[2].category = "unclassified".into();
        draft.items[3].quantity = Some("zero".into());
        draft.purchase_date = Some("10/03/2026".into());
        assert!(review(draft.clone()).is_err());
        let problems = receipt_problems(&draft);
        assert!(problems.iter().any(|p| p.contains("purchase date")));
        assert!(problems
            .iter()
            .any(|p| p.contains("Lines add up to $18.12") && p.contains("$17.41")));
        assert_eq!(
            line_problems(&draft.items[2]),
            vec!["choose a food or household category"]
        );
        assert_eq!(
            line_problems(&draft.items[3]),
            vec!["quantity must be a positive number"]
        );
    }
}

use sha2::{Digest, Sha256};

use crate::model::{
    DraftItem, DraftStatus, ReceiptDraft, RefundLine, ReviewError, ReviewedReceipt,
};

fn cents(token: &str) -> Option<i64> {
    let negative = token.starts_with('-');
    let value = token.strip_prefix('-').unwrap_or(token);
    let (whole, fraction) = value.split_once('.')?;
    if fraction.len() != 2
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let value = whole.parse::<i64>().ok()?.checked_mul(100)? + fraction.parse::<i64>().ok()?;
    Some(if negative { -value } else { value })
}

fn final_amount(line: &str) -> Option<i64> {
    line.split_whitespace().last().and_then(cents)
}

fn quantity_and_unit(line: &str) -> (Option<String>, Option<String>) {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.contains(&"??") {
        return (None, None);
    }
    for pair in words.windows(2) {
        let quantity = pair[0];
        if quantity.parse::<rust_decimal::Decimal>().is_err() {
            continue;
        }
        let unit = match pair[1] {
            "LB" => "lb",
            "GAL" => "gal",
            "CT" => "egg",
            "OZ" => "oz",
            "BTL" => "bottle",
            "PACK" => "pack",
            "@" => "each",
            _ => continue,
        };
        return (Some(quantity.to_string()), Some(unit.to_string()));
    }
    (None, None)
}

fn name_category(line: &str) -> (&'static str, &'static str) {
    match line {
        s if s.starts_with("BANANAS") => ("bananas", "food.produce"),
        s if s.starts_with("MILK 2%") => ("milk 2%", "food.dairy"),
        s if s.starts_with("EGGS LG") => ("large eggs", "food.dairy"),
        s if s.starts_with("PASTA") => ("pasta", "food.pantry"),
        s if s.starts_with("SPINACH") => ("spinach", "food.produce"),
        s if s.starts_with("DISH SOAP") => ("dish soap", "household.cleaning"),
        s if s.starts_with("APPLES") => ("apples", "food.produce"),
        s if s.starts_with("RICE") => ("rice", "food.pantry"),
        s if s.starts_with("YOGURT") => ("yogurt", "food.dairy"),
        s if s.starts_with("PAPER TOWELS") => ("paper towels", "household.paper"),
        s if s.starts_with("CHKN BRST") => ("chicken breast", "food.meat"),
        s if s.starts_with("TOMATOES") => ("tomatoes", "food.produce"),
        s if s.starts_with("CHEESE SHRD") => ("shredded cheese", "food.dairy"),
        s if s.starts_with("GRN ONION") => ("green onion", "food.produce"),
        _ => ("unknown item", "unclassified"),
    }
}

pub fn parse_text(raw: &str) -> ReceiptDraft {
    let source_sha256 = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let mut draft = ReceiptDraft {
        id: String::new(),
        purchase_date: None,
        source_sha256,
        items: Vec::new(),
        refunds: Vec::new(),
        unallocated_adjustments: Vec::new(),
        known_subtotal_cents: 0,
        printed_subtotal_cents: None,
        printed_tax_cents: None,
        printed_total_cents: None,
        status: DraftStatus::PendingConfirmation,
        uncertainty: Vec::new(),
    };

    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.starts_with("FIXTURE ID ") {
            let words: Vec<&str> = line.split_whitespace().collect();
            draft.id = words.get(2).copied().unwrap_or("").to_string();
            draft.purchase_date = words.get(4).map(|s| s.to_string());
        } else if line.starts_with("SUBTOTAL") {
            draft.printed_subtotal_cents = final_amount(line);
        } else if line.starts_with("TAX ") {
            draft.printed_tax_cents = final_amount(line);
        } else if line.starts_with("TOTAL") {
            draft.printed_total_cents = final_amount(line);
        } else if line.starts_with("RETURN ") {
            let amount = final_amount(line).unwrap_or(0);
            let (quantity, unit) = quantity_and_unit(line);
            let (name, category) = name_category(line.trim_start_matches("RETURN "));
            draft.refunds.push(RefundLine {
                name: name.to_string(),
                category: category.to_string(),
                quantity,
                unit,
                amount_cents: amount,
                original_line_id: None,
                inventory_quantity_to_reverse: None,
            });
            draft.known_subtotal_cents += amount;
        } else if line.contains(" DISC") {
            let amount = final_amount(line);
            let target = draft
                .items
                .iter_mut()
                .rev()
                .find(|item| line.contains(item.raw.split_whitespace().next().unwrap_or("~")));
            if let (Some(amount), Some(item)) = (amount, target) {
                item.discount_cents += amount;
                item.net_cents = item.gross_cents.map(|gross| gross + item.discount_cents);
                draft.known_subtotal_cents += amount;
            } else {
                draft.unallocated_adjustments.push(line.to_string());
                draft.uncertainty.push("unallocated discount".to_string());
            }
        } else if line.starts_with("SAVE ") {
            draft.unallocated_adjustments.push(line.to_string());
            draft
                .uncertainty
                .push("unparsed savings amount".to_string());
        } else if line.starts_with("SYNTHETIC")
            || line.starts_with("PUBLIX-STYLE")
            || line.starts_with("All prices")
            || line.starts_with("END SYNTHETIC")
            || line.is_empty()
        {
            continue;
        } else {
            let (name, category) = name_category(line);
            let (quantity, unit) = quantity_and_unit(line);
            let amount = if line.contains("??") {
                None
            } else {
                final_amount(line)
            };
            let mut uncertainty = Vec::new();
            if quantity.is_none() {
                uncertainty.push("quantity unknown".to_string());
            }
            if amount.is_none() {
                uncertainty.push("line amount unknown".to_string());
            }
            if name == "unknown item" {
                uncertainty.push("item unrecognized".to_string());
            }
            draft.known_subtotal_cents += amount.unwrap_or(0);
            draft.items.push(DraftItem {
                line_id: format!("{}-L{:02}", draft.id, draft.items.len() + 1),
                raw: line.to_string(),
                name: name.to_string(),
                quantity,
                unit,
                gross_cents: amount,
                discount_cents: 0,
                net_cents: amount,
                category: category.to_string(),
                expiry_date: None,
                uncertainty: uncertainty.clone(),
            });
            draft.uncertainty.extend(uncertainty);
        }
    }
    if draft.id.is_empty() {
        draft
            .uncertainty
            .push("receipt identifier unknown".to_string());
    }
    if let Some(printed) = draft.printed_subtotal_cents {
        if printed != draft.known_subtotal_cents {
            draft.uncertainty.push("subtotal mismatch".to_string());
        }
    } else {
        draft.uncertainty.push("subtotal missing".to_string());
    }
    if draft.printed_total_cents
        != draft
            .printed_subtotal_cents
            .zip(draft.printed_tax_cents)
            .map(|(subtotal, tax)| subtotal + tax)
    {
        draft.uncertainty.push("total mismatch".to_string());
    }
    if !draft.uncertainty.is_empty() {
        draft.status = DraftStatus::NeedsResolution;
    }
    draft
}

pub fn review(draft: ReceiptDraft) -> Result<ReviewedReceipt, ReviewError> {
    let item_total: Option<i64> = draft.items.iter().try_fold(0_i64, |sum, item| {
        let amount = item.net_cents?;
        item.quantity.as_ref()?;
        item.unit.as_ref()?;
        sum.checked_add(amount)
    });
    let refund_total: i64 = draft.refunds.iter().map(|item| item.amount_cents).sum();
    if draft.status != DraftStatus::PendingConfirmation
        || !draft.uncertainty.is_empty()
        || draft.id.is_empty()
        || draft.purchase_date.is_none()
        || !draft.unallocated_adjustments.is_empty()
        || draft
            .refunds
            .iter()
            .any(|refund| refund.category == "unclassified")
        || item_total.and_then(|n| n.checked_add(refund_total)) != draft.printed_subtotal_cents
        || draft
            .printed_subtotal_cents
            .zip(draft.printed_tax_cents)
            .map(|(subtotal, tax)| subtotal + tax)
            != draft.printed_total_cents
    {
        return Err(ReviewError::NeedsResolution);
    }
    Ok(ReviewedReceipt { draft })
}

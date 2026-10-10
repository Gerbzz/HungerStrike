use sha2::{Digest, Sha256};

use crate::model::{
    DraftItem, DraftStatus, ReceiptDraft, RefundLine, ReviewError, ReviewedReceipt,
};

fn cents(token: &str) -> Option<i64> {
    let token = token.strip_prefix('$').unwrap_or(token);
    let negative = token.starts_with('-');
    let value = token.strip_prefix('-').unwrap_or(token);
    let value = value.strip_prefix('$').unwrap_or(value);
    let (whole, fraction) = value.split_once('.')?;
    if fraction.len() != 2
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let value = whole
        .parse::<i64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(fraction.parse::<i64>().ok()?)?;
    if value > 100_000_000 {
        return None;
    }
    Some(if negative { -value } else { value })
}

fn final_amount(line: &str) -> Option<i64> {
    line.split_whitespace().last().and_then(cents)
}

fn quantity_and_unit(line: &str) -> (Option<String>, Option<String>) {
    let words: Vec<&str> = line.split_whitespace().collect();
    for pair in words.windows(2) {
        let quantity = pair[0];
        if quantity.parse::<rust_decimal::Decimal>().is_err() {
            continue;
        }
        let unit = match pair[1].to_ascii_uppercase().as_str() {
            "LB" => "lb",
            "GAL" => "gal",
            "CT" => "count",
            "OZ" => "oz",
            "BTL" => "bottle",
            "PACK" => "pack",
            "@" | "EA" => "each",
            _ => continue,
        };
        return (Some(quantity.to_string()), Some(unit.to_string()));
    }
    (None, None)
}

fn name_category(line: &str) -> (String, String) {
    let upper = line.to_ascii_uppercase();
    let known = match upper.as_str() {
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
        _ => ("", ""),
    };
    if !known.0.is_empty() {
        return (known.0.into(), known.1.into());
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    let end = words
        .iter()
        .position(|word| word.parse::<rust_decimal::Decimal>().is_ok() || *word == "??")
        .unwrap_or_else(|| words.len().saturating_sub(1));
    (
        words[..end].join(" ").to_ascii_lowercase(),
        "unclassified".into(),
    )
}

fn specific_unit(name: &str, unit: Option<String>) -> Option<String> {
    if name == "large eggs" && unit.as_deref() == Some("count") {
        Some("egg".into())
    } else {
        unit
    }
}

fn is_metadata(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    let card_footer = ["VISA ", "MASTERCARD ", "AMEX ", "CARD "]
        .iter()
        .any(|prefix| upper.starts_with(prefix))
        && ["ENDING", "****", "AUTH", "APPROVAL", "DEBIT", "CREDIT"]
            .iter()
            .any(|marker| upper.contains(marker));
    upper.is_empty()
        || upper.starts_with("SYNTHETIC")
        || upper.starts_with("PUBLIX-STYLE")
        || upper.starts_with("ALL PRICES")
        || upper.starts_with("END SYNTHETIC")
        || card_footer
        || upper.starts_with("PAYMENT")
        || upper.starts_with("AUTH")
        || upper.starts_with("CREDIT CARD")
        || upper.starts_with("DEBIT CARD")
        || upper.starts_with("CARD NUMBER")
        || upper.starts_with("CARD NO")
        || upper.starts_with("CUSTOMER ")
        || upper.starts_with("MEMBER ")
        || upper.starts_with("EMAIL ")
        || upper.starts_with("ADDRESS ")
        || upper.starts_with("CASH ")
        || upper.starts_with("CHANGE ")
        || upper.starts_with("THANK")
        || (upper.starts_with("STORE ") && quantity_and_unit(line).0.is_none())
        || (upper.starts_with("TEL ") && quantity_and_unit(line).0.is_none())
        || (upper.starts_with("PHONE ") && quantity_and_unit(line).0.is_none())
        || upper.starts_with("LOYALTY")
        || upper.starts_with("REWARDS")
        || upper.starts_with("TOTAL SAVINGS")
        || upper.starts_with("TOTAL DISCOUNT")
        || upper.starts_with("TAXABLE ")
        || upper.starts_with("TAX EXEMPT")
        || upper.starts_with("HTTP")
        || upper.starts_with("WWW.")
        || (line.chars().next().is_some_and(|c| c.is_ascii_digit()) && !line.contains(" @ "))
}

pub fn parse_text(raw: &str) -> ReceiptDraft {
    let normalized = raw
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !is_metadata(line))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_uppercase();
    let source_sha256 = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    let mut draft = ReceiptDraft {
        id: format!("OCR-{}", &source_sha256[..12]),
        purchase_date: None,
        source_sha256,
        items: Vec::new(),
        refunds: Vec::new(),
        unallocated_adjustments: Vec::new(),
        unallocated_cents: None,
        accepted_unknowns: false,
        known_subtotal_cents: 0,
        printed_subtotal_cents: None,
        printed_tax_cents: None,
        printed_total_cents: None,
        status: DraftStatus::PendingConfirmation,
        uncertainty: Vec::new(),
    };

    for raw_line in raw.lines() {
        let cleaned = raw_line.split_whitespace().collect::<Vec<_>>().join(" ");
        let line = cleaned.as_str();
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("FIXTURE ID ") {
            let words: Vec<&str> = line.split_whitespace().collect();
            draft.id = words.get(2).copied().unwrap_or("").to_string();
            draft.purchase_date = words.get(4).map(|s| s.to_string());
        } else if upper.starts_with("DATE ") || upper.starts_with("DATE:") {
            draft.purchase_date = line.split_whitespace().last().map(str::to_string);
        } else if upper.starts_with("SUBTOTAL") {
            draft.printed_subtotal_cents = final_amount(line);
        } else if upper.starts_with("TAX ") {
            draft.printed_tax_cents = final_amount(line);
        } else if upper.starts_with("TOTAL") && !is_metadata(line) {
            draft.printed_total_cents = final_amount(line);
        } else if upper.starts_with("RETURN ") {
            let amount = final_amount(line).unwrap_or(0);
            let (quantity, unit) = quantity_and_unit(line);
            let (name, category) =
                name_category(line.split_once(' ').map(|(_, body)| body).unwrap_or(""));
            let unit = specific_unit(&name, unit);
            draft.refunds.push(RefundLine {
                name: name.to_string(),
                category: category.to_string(),
                quantity,
                unit,
                amount_cents: amount,
                original_line_id: None,
                inventory_quantity_to_reverse: None,
            });
            if let Some(sum) = draft.known_subtotal_cents.checked_add(amount) {
                draft.known_subtotal_cents = sum;
            } else {
                draft.uncertainty.push("subtotal overflow".into());
            }
        } else if upper.contains(" DISC") {
            let amount = final_amount(line);
            let target = draft.items.iter_mut().rev().find(|item| {
                upper.contains(
                    &item
                        .raw
                        .split_whitespace()
                        .next()
                        .unwrap_or("~")
                        .to_ascii_uppercase(),
                )
            });
            if let (Some(amount), Some(item)) = (amount, target) {
                item.discount_cents = item.discount_cents.checked_add(amount).unwrap_or(i64::MAX);
                item.net_cents = item
                    .gross_cents
                    .and_then(|gross| gross.checked_add(item.discount_cents));
                if let Some(sum) = draft.known_subtotal_cents.checked_add(amount) {
                    draft.known_subtotal_cents = sum;
                } else {
                    draft.uncertainty.push("subtotal overflow".into());
                }
            } else {
                draft.unallocated_adjustments.push(line.to_string());
                draft.uncertainty.push("unallocated discount".to_string());
            }
        } else if upper.starts_with("SAVE ") {
            draft.unallocated_adjustments.push(line.to_string());
            draft
                .uncertainty
                .push("unparsed savings amount".to_string());
        } else if is_metadata(line) {
            continue;
        } else if final_amount(line).is_some()
            || line.contains("??")
            || quantity_and_unit(line).0.is_some()
        {
            let (name, category) = name_category(line);
            let (quantity, unit) = quantity_and_unit(line);
            let unit = specific_unit(&name, unit);
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
            if category == "unclassified" {
                uncertainty.push("item unrecognized".to_string());
            }
            if let Some(sum) = draft.known_subtotal_cents.checked_add(amount.unwrap_or(0)) {
                draft.known_subtotal_cents = sum;
            } else {
                draft.uncertainty.push("subtotal overflow".into());
            }
            draft.items.push(DraftItem {
                line_id: format!("{}-L{:02}", draft.id, draft.items.len() + 1),
                raw: line.to_string(),
                name,
                quantity,
                unit,
                gross_cents: amount,
                discount_cents: 0,
                net_cents: amount,
                category,
                expiry_date: None,
                uncertainty: uncertainty.clone(),
            });
            draft.uncertainty.extend(uncertainty);
        }
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
            .and_then(|(subtotal, tax)| subtotal.checked_add(tax))
    {
        draft.uncertainty.push("total mismatch".to_string());
    }
    if !draft.uncertainty.is_empty() {
        draft.status = DraftStatus::NeedsResolution;
    }
    draft
}

pub fn review(draft: ReceiptDraft) -> Result<ReviewedReceipt, ReviewError> {
    const LIMIT: i64 = 100_000_000;
    let mut ids = std::collections::HashSet::new();
    let item_total: Option<i64> = draft.items.iter().try_fold(0_i64, |sum, item| {
        let gross = item.gross_cents;
        let amount = item.net_cents;
        let quantity = item
            .quantity
            .as_ref()?
            .parse::<rust_decimal::Decimal>()
            .ok()?;
        if item.name.trim().is_empty()
            || !ids.insert(item.line_id.as_str())
            || item.line_id.trim().is_empty()
            || item
                .expiry_date
                .as_deref()
                .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
            || !["food.", "household."]
                .iter()
                .any(|prefix| item.category.starts_with(prefix))
            || item.unit.as_ref()?.trim().is_empty()
            || quantity <= rust_decimal::Decimal::ZERO
            || quantity > rust_decimal::Decimal::from(1_000_000)
            || item.discount_cents > 0
            || item.gross_cents.is_some_and(|v| v > LIMIT)
            || item.net_cents.is_some_and(|v| v > LIMIT)
            || item.discount_cents < -LIMIT
        {
            return None;
        }
        match (gross, amount) {
            (Some(gross), Some(amount))
                if gross >= 0
                    && amount >= 0
                    && gross.checked_add(item.discount_cents) == Some(amount) =>
            {
                sum.checked_add(amount)
            }
            (None, None) if draft.accepted_unknowns => Some(sum),
            _ => None,
        }
    });
    let refund_total: Option<i64> = draft.refunds.iter().try_fold(0_i64, |sum, refund| {
        if refund.name.trim().is_empty()
            || !["food.", "household."]
                .iter()
                .any(|prefix| refund.category.starts_with(prefix))
            || refund.amount_cents >= 0
            || refund.amount_cents < -LIMIT
        {
            return None;
        }
        if let Some(quantity) = &refund.quantity {
            if quantity
                .parse::<rust_decimal::Decimal>()
                .ok()
                .is_none_or(|q| {
                    q <= rust_decimal::Decimal::ZERO || q > rust_decimal::Decimal::from(1_000_000)
                })
            {
                return None;
            }
        }
        if let Some(reverse) = &refund.inventory_quantity_to_reverse {
            let reverse = reverse.parse::<rust_decimal::Decimal>().ok()?;
            let quantity = refund
                .quantity
                .as_deref()?
                .parse::<rust_decimal::Decimal>()
                .ok()?;
            if reverse < rust_decimal::Decimal::ZERO
                || reverse > quantity
                || reverse > rust_decimal::Decimal::from(1_000_000)
                || refund.unit.is_none()
            {
                return None;
            }
        }
        sum.checked_add(refund.amount_cents)
    });
    if draft.id.trim().is_empty()
        || draft.items.is_empty() && draft.refunds.is_empty()
        || (draft.items.iter().any(|item| item.net_cents.is_none())
            && (!draft.accepted_unknowns || draft.unallocated_cents.is_none()))
        || draft
            .unallocated_cents
            .is_some_and(|v| v.unsigned_abs() > LIMIT as u64)
        || draft
            .printed_subtotal_cents
            .is_some_and(|v| v.unsigned_abs() > LIMIT as u64)
        || draft.printed_tax_cents.is_some_and(|v| v > LIMIT)
        || draft
            .printed_total_cents
            .is_some_and(|v| v.unsigned_abs() > LIMIT as u64)
        || draft
            .purchase_date
            .as_deref()
            .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
            .is_none()
        || (!draft.unallocated_adjustments.is_empty() && draft.unallocated_cents.is_none())
        || item_total
            .and_then(|n| refund_total.and_then(|r| n.checked_add(r)))
            .and_then(|n| n.checked_add(draft.unallocated_cents.unwrap_or(0)))
            != draft.printed_subtotal_cents
        || draft.printed_tax_cents.is_none_or(|tax| tax < 0)
        || draft
            .printed_subtotal_cents
            .zip(draft.printed_tax_cents)
            .and_then(|(subtotal, tax)| subtotal.checked_add(tax))
            != draft.printed_total_cents
    {
        return Err(ReviewError::NeedsResolution);
    }
    let mut draft = draft;
    draft.known_subtotal_cents = item_total
        .and_then(|items| refund_total.and_then(|refunds| items.checked_add(refunds)))
        .ok_or(ReviewError::NeedsResolution)?;
    draft.status = DraftStatus::PendingConfirmation;
    Ok(ReviewedReceipt { draft })
}

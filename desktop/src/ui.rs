use std::io::Write;

use bevy_egui::egui::{self, RichText};
use hungerstrike_core::{
    apply_import, budget_view, confirm_receipt, correct_receipt, export_backup, parse_text,
    preview_import, recategorize_item, record_inventory_action, restore_last_import, review,
    set_budget, suggest_meals, DraftItem, InventoryAction, Period, RecipeCollection, RefundLine,
    ReplaceOrReconcile,
};

use crate::{ocr, AppState, Tab};

fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    format!("{sign}${}.{:02}", absolute / 100, absolute % 100)
}

fn option_text(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) {
    let mut text = value.clone().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.text_edit_singleline(&mut text).changed() {
            *value = if text.trim().is_empty() {
                None
            } else {
                Some(text.clone())
            };
        }
    });
}

fn option_cents(ui: &mut egui::Ui, label: &str, value: &mut Option<i64>) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut known = value.is_some();
        if ui.checkbox(&mut known, "known").changed() {
            *value = if known { Some(0) } else { None };
        }
        if let Some(value) = value.as_mut() {
            ui.add(egui::DragValue::new(value).speed(1).suffix(" cents"));
        }
    });
}

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("HungerStrike");
            ui.separator();
            for (label, tab) in [
                ("Kitchen", Tab::Kitchen),
                ("Receipt", Tab::Receipt),
                ("Meals", Tab::Meals),
                ("Spending", Tab::Spending),
                ("Data", Tab::Data),
            ] {
                if ui.selectable_label(state.tab == tab, label).clicked() {
                    state.tab = tab;
                }
            }
            ui.separator();
            ui.label(RichText::new("LOCAL ONLY · Rust desktop preview").small());
        });
    });
    egui::SidePanel::left("controls")
        .min_width(500.0)
        .max_width(620.0)
        .show(ctx, |ui| {
            if !state.message.is_empty() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(&state.message);
                });
                ui.add_space(6.0);
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match state.tab {
                    Tab::Kitchen => kitchen(ui, state),
                    Tab::Receipt => receipt(ui, state),
                    Tab::Meals => meals(ui, state),
                    Tab::Spending => spending(ui, state),
                    Tab::Data => data(ui, state),
                });
        });
    egui::TopBottomPanel::bottom("scene_help").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Click a 3D appliance to filter stock. Arrow keys orbit and zoom.");
            if ui.button("Rotate left").clicked() {
                state.camera_yaw -= 0.35;
            }
            if ui.button("Rotate right").clicked() {
                state.camera_yaw += 0.35;
            }
            if ui.button("Zoom +").clicked() {
                state.camera_distance = (state.camera_distance - 1.0).max(5.5);
            }
            if ui.button("Zoom out").clicked() {
                state.camera_distance = (state.camera_distance + 1.0).min(18.0);
            }
        });
    });
}

fn kitchen(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Kitchen stock");
    ui.label("The 3D fridge, freezer and pantry show confirmed stock. The list provides the same zone selection and stock actions by keyboard.");
    ui.horizontal(|ui| {
        for zone in ["all", "fridge", "freezer", "pantry"] {
            if ui.selectable_label(state.zone == zone, zone).clicked() {
                state.zone = zone.into();
            }
        }
    });
    ui.separator();
    let lots = state.snapshot.lots.clone();
    let visible: Vec<_> = lots
        .into_iter()
        .filter(|lot| state.zone == "all" || lot.zone == state.zone)
        .collect();
    if visible.is_empty() {
        ui.label("No confirmed stock in this zone.");
    }
    for lot in &visible {
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(state.lot_id == lot.id, &lot.name)
                .clicked()
            {
                state.lot_id = lot.id.clone();
                state.quantity = "1".into();
            }
            ui.label(format!("{} {} · {}", lot.quantity, lot.unit, lot.zone));
            ui.label(if lot.cost_unknown {
                "cost unknown".into()
            } else {
                money(lot.remaining_cost_cents)
            });
            ui.label(format!(
                "expiry {}",
                lot.expiry_date.as_deref().unwrap_or("unknown")
            ));
        });
    }
    if let Some(lot) = state
        .snapshot
        .lots
        .iter()
        .find(|lot| lot.id == state.lot_id)
    {
        ui.separator();
        ui.strong(format!(
            "Selected: {} · {} {}",
            lot.name, lot.quantity, lot.unit
        ));
        ui.horizontal(|ui| {
            ui.label("Quantity");
            ui.text_edit_singleline(&mut state.quantity);
        });
        let mut action = None;
        ui.horizontal(|ui| {
            if ui.button("Consume").clicked() {
                action = Some(InventoryAction::Consume {
                    lot_id: state.lot_id.clone(),
                    quantity: state.quantity.clone(),
                });
            }
            if ui.button("Discard").clicked() {
                action = Some(InventoryAction::Discard {
                    lot_id: state.lot_id.clone(),
                    quantity: state.quantity.clone(),
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label("Move to");
            for zone in ["fridge", "freezer", "pantry"] {
                if ui.button(zone).clicked() {
                    action = Some(InventoryAction::Move {
                        lot_id: state.lot_id.clone(),
                        zone: zone.into(),
                    });
                }
            }
        });
        if let Some(action) = action {
            let result = state
                .store
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut store| record_inventory_action(&mut store, action));
            state.message = match result {
                Ok(()) => {
                    let _ = state.refresh();
                    "Inventory updated.".into()
                }
                Err(e) => e,
            };
        }
    }
    ui.separator();
    ui.heading("Spending categories");
    ui.label("Choose a confirmed line to change its category. Cross-group moves after stock use or linked refunds are blocked.");
    let lines = state.snapshot.receipt_items.clone();
    for item in &lines {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    state.category.starts_with(&format!("{}|", item.line_id)),
                    &item.name,
                )
                .clicked()
            {
                state.category = format!("{}|{}", item.line_id, item.category);
            }
            ui.label(format!(
                "{} · {}",
                item.net_cents
                    .map(money)
                    .unwrap_or_else(|| "cost unknown".into()),
                item.category
            ));
        });
    }
    if let Some((line_id, category)) = state.category.clone().split_once('|') {
        let line_id = line_id.to_string();
        let mut new_category = category.to_string();
        ui.horizontal(|ui| {
            ui.label("Category");
            ui.text_edit_singleline(&mut new_category);
        });
        state.category = format!("{line_id}|{new_category}");
        if ui.button("Apply category").clicked() {
            let result = state
                .store
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut store| recategorize_item(&mut store, &line_id, &new_category));
            state.message = match result {
                Ok(()) => {
                    let _ = state.refresh();
                    "Category updated and audited.".into()
                }
                Err(e) => e,
            };
        }
    }
}

fn receipt(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Receipt review");
    ui.label(
        "Import text or a local PNG/JPEG. Nothing changes inventory until you review and confirm.",
    );
    ui.add(
        egui::TextEdit::multiline(&mut state.raw_text)
            .desired_rows(6)
            .desired_width(ui.available_width())
            .hint_text("Paste receipt text here"),
    );
    ui.horizontal(|ui| {
        if ui.button("Parse text").clicked() {
            state.draft = Some(parse_text(&state.raw_text));
            state.message = "Review every field, including the date and uncertain lines.".into();
        }
        if ui.button("Open photo · offline OCR").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Receipt photo", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                let result = std::fs::read(path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        ocr::extract_local_image(&bytes, &ocr::installed_engine())
                            .map_err(|e| e.to_string())
                    });
                match result {
                    Ok(text) => {
                        state.draft = Some(parse_text(&text));
                        state.raw_text = text;
                        state.message =
                            "OCR text is local and temporary. Verify every field.".into();
                    }
                    Err(error) => {
                        state.message = format!("{error}. Paste or type the receipt instead.")
                    }
                }
            }
        }
        if ui.button("Cancel and clear").clicked() {
            state.clear_receipt();
            state.message = "Temporary receipt text cleared.".into();
        }
    });
    let Some(mut draft) = state.draft.take() else {
        return;
    };
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Receipt ID");
        ui.text_edit_singleline(&mut draft.id);
    });
    option_text(ui, "Purchase date YYYY-MM-DD", &mut draft.purchase_date);
    if !draft.uncertainty.is_empty() {
        ui.label(format!(
            "Source uncertainty: {}",
            draft.uncertainty.join(", ")
        ));
    }
    option_cents(ui, "Printed subtotal", &mut draft.printed_subtotal_cents);
    option_cents(ui, "Printed tax", &mut draft.printed_tax_cents);
    option_cents(ui, "Printed total", &mut draft.printed_total_cents);
    ui.label("Purchase date never sets an expiry date.");
    let mut remove = None;
    ui.separator();
    ui.strong("Purchased items");
    for (index, item) in draft.items.iter_mut().enumerate() {
        egui::CollapsingHeader::new(format!("{} · {}", index + 1, item.name))
            .default_open(!item.uncertainty.is_empty())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut item.name);
                });
                option_text(ui, "Quantity", &mut item.quantity);
                option_text(ui, "Unit", &mut item.unit);
                ui.horizontal(|ui| {
                    ui.label("Category");
                    ui.text_edit_singleline(&mut item.category);
                });
                option_cents(ui, "Gross amount", &mut item.gross_cents);
                ui.horizontal(|ui| {
                    ui.label("Discount cents (negative)");
                    ui.add(egui::DragValue::new(&mut item.discount_cents));
                });
                item.net_cents = item
                    .gross_cents
                    .and_then(|gross| gross.checked_add(item.discount_cents));
                ui.label(format!(
                    "Net: {}",
                    item.net_cents
                        .map(money)
                        .unwrap_or_else(|| "unknown".into())
                ));
                option_text(ui, "Expiry YYYY-MM-DD (optional)", &mut item.expiry_date);
                if !item.uncertainty.is_empty() {
                    ui.label(format!(
                        "Source uncertainty: {}",
                        item.uncertainty.join(", ")
                    ));
                }
                if ui.button("Remove item").clicked() {
                    remove = Some(index);
                }
            });
    }
    if let Some(index) = remove {
        draft.items.remove(index);
    }
    if ui.button("Add item").clicked() {
        let mut index = 1;
        while draft
            .items
            .iter()
            .any(|item| item.line_id == format!("{}-L{index:02}", draft.id))
        {
            index += 1;
        }
        draft.items.push(DraftItem {
            line_id: format!("{}-L{index:02}", draft.id),
            raw: String::new(),
            name: String::new(),
            quantity: Some("1".into()),
            unit: Some("each".into()),
            gross_cents: Some(0),
            discount_cents: 0,
            net_cents: Some(0),
            category: "food.pantry".into(),
            expiry_date: None,
            uncertainty: vec!["added during review".into()],
        });
    }
    ui.separator();
    ui.strong("Refunds");
    let mut remove_refund = None;
    for (index, refund) in draft.refunds.iter_mut().enumerate() {
        egui::CollapsingHeader::new(format!("Refund {} · {}", index + 1, refund.name))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut refund.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Category");
                    ui.text_edit_singleline(&mut refund.category);
                });
                option_text(ui, "Refunded quantity", &mut refund.quantity);
                option_text(ui, "Unit", &mut refund.unit);
                ui.horizontal(|ui| {
                    ui.label("Amount cents (negative)");
                    ui.add(egui::DragValue::new(&mut refund.amount_cents));
                });
                option_text(
                    ui,
                    "Original lot ID (optional)",
                    &mut refund.original_line_id,
                );
                option_text(
                    ui,
                    "Stock quantity to reverse",
                    &mut refund.inventory_quantity_to_reverse,
                );
                if ui.button("Remove refund").clicked() {
                    remove_refund = Some(index);
                }
            });
    }
    if let Some(index) = remove_refund {
        draft.refunds.remove(index);
    }
    if ui.button("Add refund").clicked() {
        draft.refunds.push(RefundLine {
            name: String::new(),
            category: "food.pantry".into(),
            quantity: Some("1".into()),
            unit: Some("each".into()),
            amount_cents: -1,
            original_line_id: None,
            inventory_quantity_to_reverse: None,
        });
    }
    ui.separator();
    ui.label("Unassigned source adjustments (review, edit, or remove false positives):");
    let mut remove_adjustment = None;
    for (index, line) in draft.unallocated_adjustments.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.text_edit_singleline(line);
            if ui.button("Remove").clicked() {
                remove_adjustment = Some(index);
            }
        });
    }
    if let Some(index) = remove_adjustment {
        draft.unallocated_adjustments.remove(index);
    }
    if ui.button("Add adjustment note").clicked() {
        draft.unallocated_adjustments.push(String::new());
    }
    option_cents(
        ui,
        "Explicit unallocated reconciliation",
        &mut draft.unallocated_cents,
    );
    ui.checkbox(
        &mut draft.accepted_unknowns,
        "Accept unknown item costs with explicit reconciliation",
    );
    let valid = review(draft.clone()).is_ok();
    if !valid {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Needs correction: item fields, refund limits, subtotal and total must reconcile.",
        );
    }
    let mut confirmed = false;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(valid, egui::Button::new("Confirm new receipt"))
            .clicked()
        {
            let result = review(draft.clone())
                .map_err(|e| e.to_string())
                .and_then(|reviewed| {
                    state
                        .store
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|mut store| confirm_receipt(&mut store, reviewed))
                });
            state.message = match result {
                Ok(outcome) => {
                    confirmed = true;
                    let _ = state.refresh();
                    format!("Receipt: {outcome:?}")
                }
                Err(e) => e,
            };
        }
        if ui
            .add_enabled(valid, egui::Button::new("Correct existing receipt"))
            .clicked()
        {
            let result = review(draft.clone())
                .map_err(|e| e.to_string())
                .and_then(|reviewed| {
                    state
                        .store
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|mut store| correct_receipt(&mut store, reviewed))
                });
            state.message = match result {
                Ok(outcome) => {
                    confirmed = true;
                    let _ = state.refresh();
                    format!("Receipt: {outcome:?}")
                }
                Err(e) => e,
            };
        }
    });
    if confirmed {
        state.clear_receipt();
    } else {
        state.draft = Some(draft);
    }
}

fn meals(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Meals from stock");
    ui.horizontal(|ui| {
        ui.label("Today YYYY-MM-DD");
        ui.text_edit_singleline(&mut state.reference_date);
    });
    let recipes: Result<RecipeCollection, _> =
        serde_json::from_str(include_str!("../../assets/recipes.json"));
    let result = recipes.map_err(|e| e.to_string()).and_then(|collection| {
        suggest_meals(&state.snapshot, &collection.recipes, &state.reference_date)
    });
    match result {
        Ok(meals) => {
            for meal in meals {
                egui::CollapsingHeader::new(&meal.name).show(ui, |ui| {
                    ui.label(format!(
                        "{} servings · {}",
                        meal.servings,
                        if meal.available {
                            "ingredients available"
                        } else {
                            "ingredients missing"
                        }
                    ));
                    ui.label(format!(
                        "Estimated stock cost: {} · {}",
                        meal.estimate_cents
                            .map(money)
                            .unwrap_or_else(|| "unknown".into()),
                        meal.estimate_status
                    ));
                    if !meal.soon_expiring.is_empty() {
                        ui.label(format!("Use soon: {}", meal.soon_expiring.join(", ")));
                    }
                    if meal.expiry_unknown {
                        ui.label("Some expiry dates are unknown.");
                    }
                    for ingredient in meal.ingredients {
                        ui.label(format!(
                            "{}: use {} {} · missing {} · {}",
                            ingredient.name,
                            ingredient.stock_used_quantity,
                            ingredient.unit,
                            ingredient.missing_quantity,
                            if ingredient.cost_unknown {
                                "cost unknown".into()
                            } else {
                                money(ingredient.estimated_cost_cents)
                            }
                        ));
                    }
                    for step in meal.steps {
                        ui.label(step);
                    }
                });
            }
        }
        Err(error) => {
            ui.label(error);
        }
    }
}

fn spending(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Spending");
    ui.label(format!(
        "Cash paid / refunded: {}",
        money(state.snapshot.cash_flow_cents)
    ));
    ui.label(format!(
        "Food lines and refunds with known allocation: {}",
        money(state.snapshot.food_spending_cents)
    ));
    ui.label(format!(
        "Household lines and refunds: {}",
        money(state.snapshot.household_goods_cents)
    ));
    ui.label(format!("Tax: {}", money(state.snapshot.tax_cents)));
    ui.label(format!(
        "Consumed food cost: {} ({} unknown-cost events)",
        money(state.snapshot.consumed_cost_cents),
        state.snapshot.unknown_consumption_count
    ));
    ui.label(format!(
        "Discarded food cost: {} ({} unknown-cost events)",
        money(state.snapshot.discarded_cost_cents),
        state.snapshot.unknown_discard_count
    ));
    ui.label(format!(
        "Refund adjustment not linked to stock reversal: {}",
        money(state.snapshot.refund_without_stock_reversal_cents)
    ));
    ui.label(format!(
        "Unpriced food lots: {}",
        state.snapshot.unpriced_lot_count
    ));
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Reference date YYYY-MM-DD");
        ui.text_edit_singleline(&mut state.reference_date);
    });
    egui::ComboBox::from_label("Week starts")
        .selected_text(
            [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ][state.week_start as usize],
        )
        .show_ui(ui, |ui| {
            for (index, day) in [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut state.week_start, index as u8, *day);
            }
        });
    for (period, label) in [(Period::Week, "Weekly"), (Period::Month, "Monthly")] {
        let budget_input = if matches!(period, Period::Week) {
            &mut state.weekly_budget
        } else {
            &mut state.monthly_budget
        };
        ui.horizontal(|ui| {
            ui.strong(format!("{label} budget cents"));
            ui.add(egui::DragValue::new(budget_input).range(0..=100_000_000));
            if ui.button("Save").clicked() {
                let result = state
                    .store
                    .lock()
                    .map_err(|e| e.to_string())
                    .and_then(|mut store| set_budget(&mut store, period, Some(*budget_input)));
                state.message = result
                    .map(|_| format!("{label} budget saved locally."))
                    .unwrap_or_else(|e| e);
            }
        });
        let result = state
            .store
            .lock()
            .map_err(|e| e.to_string())
            .and_then(|store| budget_view(&store, &state.reference_date, period, state.week_start));
        match result {
            Ok(view) => {
                ui.label(format!(
                    "{} to {} · {} receipt(s)",
                    view.start_date, view.end_date, view.receipt_count
                ));
                ui.label(format!(
                    "Cash {} · food {} · household {} · tax {}",
                    money(view.cash_flow_cents),
                    money(view.food_spending_cents),
                    money(view.household_goods_cents),
                    money(view.tax_cents)
                ));
                ui.label(format!(
                    "Discounts {} · refunds {} · unallocated reconciliation {}",
                    money(view.discounts_cents),
                    money(view.refunds_cents),
                    money(view.reconciliation_cents)
                ));
                ui.label(format!(
                    "Remaining budget: {}",
                    view.remaining_budget_cents
                        .map(money)
                        .unwrap_or_else(|| "not set".into())
                ));
            }
            Err(error) => {
                ui.label(error);
            }
        }
        ui.separator();
    }
}

fn data(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Local data");
    ui.label("A backup is a local JSON file. Review its preview before reconciling or replacing the current data.");
    if ui.button("Export backup").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON backup", &["json"])
            .save_file()
        {
            let result = state
                .store
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|store| export_backup(&store)?.to_bytes())
                .and_then(|bytes| {
                    let parent = path.parent().ok_or("backup path has no parent")?;
                    let mut temp =
                        tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
                    temp.write_all(&bytes).map_err(|e| e.to_string())?;
                    temp.persist(&path).map_err(|e| e.to_string())?;
                    Ok(())
                });
            state.message = result
                .map(|_| format!("Saved backup to {}", path.display()))
                .unwrap_or_else(|e| e);
        }
    }
    if ui.button("Open backup for preview").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON backup", &["json"])
            .pick_file()
        {
            let result = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    state
                        .store
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|store| {
                            preview_import(&store, &bytes).map(|preview| (bytes, preview))
                        })
                });
            match result {
                Ok((bytes, preview)) => {
                    state.backup_bytes = Some(bytes);
                    state.backup_preview = Some(preview);
                    state.message = format!("Preview loaded from {}", path.display());
                }
                Err(error) => state.message = error,
            }
        }
    }
    if let Some(preview) = &state.backup_preview {
        ui.label(format!(
            "Incoming: {} receipts, {} lots; existing: {} receipts",
            preview.incoming_receipts, preview.incoming_lots, preview.existing_receipts
        ));
        ui.label(format!(
            "Duplicates: {} · ID conflicts: {}",
            preview.duplicate_receipts, preview.conflicting_identifiers
        ));
        let mut mode = None;
        ui.horizontal(|ui| {
            if ui.button("Reconcile nonoverlapping data").clicked() {
                mode = Some(ReplaceOrReconcile::Reconcile);
            }
            if ui
                .button("Replace current data · local restore point")
                .clicked()
            {
                mode = Some(ReplaceOrReconcile::Replace);
            }
        });
        if let (Some(mode), Some(bytes)) = (mode, state.backup_bytes.as_ref()) {
            let result = state
                .store
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut store| apply_import(&mut store, bytes, mode));
            state.message = match result {
                Ok(outcome) => {
                    let _ = state.refresh();
                    state.backup_bytes = None;
                    state.backup_preview = None;
                    format!("Backup: {outcome:?}")
                }
                Err(error) => error,
            };
        }
    }
    if ui.button("Undo last replacement").clicked() {
        let result = state
            .store
            .lock()
            .map_err(|e| e.to_string())
            .and_then(|mut store| restore_last_import(&mut store));
        state.message = match result {
            Ok(()) => {
                let _ = state.refresh();
                "Replacement undone.".into()
            }
            Err(error) => error,
        };
    }
    ui.separator();
    ui.label(format!(
        "Confirmed receipts: {} · receipt revision events: {} · category edits: {}",
        state.snapshot.receipt_count,
        state.snapshot.receipt_edits.len(),
        state.snapshot.category_edits.len()
    ));
    egui::CollapsingHeader::new("Receipt revision history").show(ui, |ui| {
        for edit in &state.snapshot.receipt_edits {
            ui.label(format!(
                "#{} {} · {} · {} → {}",
                edit.sequence,
                edit.receipt_id,
                edit.action,
                edit.before_total_cents
                    .map(money)
                    .unwrap_or_else(|| "new".into()),
                edit.after_total_cents
                    .map(money)
                    .unwrap_or_else(|| "unknown".into())
            ));
            ui.label(format!("Changed: {}", edit.fields_changed.join(", ")));
        }
    });
    egui::CollapsingHeader::new("Category edit history").show(ui, |ui| {
        for edit in &state.snapshot.category_edits {
            ui.label(format!(
                "#{} {} / {}: {} → {}",
                edit.sequence, edit.receipt_id, edit.line_id, edit.from, edit.to
            ));
        }
    });
    ui.label("Offline photo OCR requires a separately installed local Tesseract executable. Manual text entry always remains available.");
}

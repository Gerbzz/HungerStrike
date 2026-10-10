use std::io::Write;

use bevy_egui::egui::{self, RichText};
use hungerstrike_core::{
    budget_view, recategorize_item, review, set_budget, suggest_meals, DraftItem, InventoryAction,
    Period, RecipeCollection, RefundLine, ReplaceOrReconcile,
};

use crate::{ocr, AppAction, AppState, ReceiptCommit, Tab};

fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    format!("{sign}${}.{:02}", absolute / 100, absolute % 100)
}

fn metric(ui: &mut egui::Ui, label: &str, value: String, color: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(38, 47, 51))
        .show(ui, |ui| {
            ui.set_min_width(112.0);
            ui.label(RichText::new(value).size(22.0).strong().color(color));
            ui.label(RichText::new(label).size(13.0));
        });
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
    if !state.style_ready {
        let mut style = (*ctx.style()).clone();
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(17.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(17.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(14.0));
        style.spacing.item_spacing = egui::vec2(10.0, 9.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.visuals.panel_fill = egui::Color32::from_rgb(24, 29, 34);
        style.visuals.window_fill = egui::Color32::from_rgb(31, 37, 42);
        ctx.set_style(style);
        state.style_ready = true;
    }
    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        ui.set_min_height(54.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("HungerStrike")
                    .size(25.0)
                    .strong()
                    .color(egui::Color32::from_rgb(152, 220, 179)),
            );
            ui.separator();
            for (label, tab) in [
                ("Kitchen", Tab::Kitchen),
                ("Receipt", Tab::Receipt),
                ("Meals", Tab::Meals),
                ("Spending", Tab::Spending),
                ("Data", Tab::Data),
            ] {
                if ui
                    .selectable_label(state.tab == tab, RichText::new(label).size(17.0))
                    .clicked()
                {
                    let _ = state.dispatch(AppAction::Navigate(tab));
                }
            }
            ui.separator();
            ui.label(
                RichText::new("PRIVATE · LOCAL DATA")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(162, 188, 178)),
            );
        });
    });
    if state.tab == Tab::Kitchen {
        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(450.0)
            .min_width(390.0)
            .max_width(600.0)
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(24, 29, 34))
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                if !state.message.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(&state.message);
                    });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| kitchen(ui, state));
            });
        egui::TopBottomPanel::bottom("scene_help").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Select a storage zone or shelf item. Arrow keys orbit and zoom.");
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
        egui::Area::new("zone_legend".into())
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-20.0, 70.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(27, 34, 40, 245))
                    .show(ui, |ui| {
                        ui.strong("OPEN STORAGE");
                        for (zone, label, color) in [
                            ("fridge", "Fridge", egui::Color32::from_rgb(99, 200, 194)),
                            ("freezer", "Freezer", egui::Color32::from_rgb(138, 189, 247)),
                            ("pantry", "Pantry", egui::Color32::from_rgb(235, 173, 105)),
                        ] {
                            let count = state
                                .snapshot
                                .lots
                                .iter()
                                .filter(|lot| lot.zone == zone && lot.quantity != "0")
                                .count();
                            if ui
                                .selectable_label(
                                    state.zone == zone,
                                    RichText::new(format!("{label}  {count}")).color(color),
                                )
                                .clicked()
                            {
                                let _ = state.dispatch(AppAction::SelectZone(zone.into()));
                            }
                        }
                    });
            });
    } else {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(24, 29, 34))
                    .inner_margin(egui::Margin::same(26)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(1040.0);
                        if !state.message.is_empty() {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.label(&state.message);
                            });
                            ui.add_space(12.0);
                        }
                        match state.tab {
                            Tab::Kitchen => {}
                            Tab::Receipt => receipt(ui, state),
                            Tab::Meals => meals(ui, state),
                            Tab::Spending => spending(ui, state),
                            Tab::Data => data(ui, state),
                        }
                    });
            });
    }
}

fn kitchen(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Your kitchen");
    ui.label("Confirmed food stock in your fridge, freezer and pantry.");
    let today = chrono::NaiveDate::parse_from_str(&state.reference_date, "%Y-%m-%d").ok();
    let stocked = state
        .snapshot
        .lots
        .iter()
        .filter(|lot| lot.quantity != "0")
        .count();
    let soon =
        state
            .snapshot
            .lots
            .iter()
            .filter(|lot| {
                lot.quantity != "0"
                    && today
                        .zip(lot.expiry_date.as_deref().and_then(|date| {
                            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
                        }))
                        .is_some_and(|(today, expiry)| {
                            (0..=3).contains(&expiry.signed_duration_since(today).num_days())
                        })
            })
            .count();
    ui.horizontal(|ui| {
        metric(
            ui,
            "stock lots",
            stocked.to_string(),
            egui::Color32::from_rgb(153, 225, 185),
        );
        metric(
            ui,
            "use soon",
            soon.to_string(),
            egui::Color32::from_rgb(242, 188, 108),
        );
        metric(
            ui,
            "stock value",
            money(state.snapshot.remaining_food_cost_cents),
            egui::Color32::from_rgb(155, 199, 237),
        );
    });
    ui.add_space(10.0);
    ui.strong("Browse a storage zone");
    ui.horizontal_wrapped(|ui| {
        for (zone, label) in [
            ("all", "All stock"),
            ("fridge", "Fridge"),
            ("freezer", "Freezer"),
            ("pantry", "Pantry"),
        ] {
            if ui.selectable_label(state.zone == zone, label).clicked() {
                let _ = state.dispatch(AppAction::SelectZone(zone.into()));
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
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong(if state.zone == "all" {
                "Your kitchen is ready"
            } else {
                "This storage zone is empty"
            });
            ui.label("Add a receipt, correct the scanned lines, then confirm to place food here.");
            if ui.button("Review a receipt").clicked() {
                let _ = state.dispatch(AppAction::Navigate(Tab::Receipt));
            }
        });
    }
    for lot in &visible {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(ui.available_width().min(390.0));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(state.lot_id == lot.id, RichText::new(&lot.name).strong())
                    .clicked()
                {
                    let _ = state.dispatch(AppAction::SelectLot(lot.id.clone()));
                }
                ui.label(format!("{} {}", lot.quantity, lot.unit));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} storage", lot.zone));
                ui.label(if lot.cost_unknown {
                    "cost unknown".into()
                } else {
                    money(lot.remaining_cost_cents)
                });
                ui.label(format!(
                    "use by {}",
                    lot.expiry_date.as_deref().unwrap_or("unknown")
                ));
            });
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
            if let Err(error) = state.dispatch(AppAction::Inventory(action)) {
                state.message = error;
            }
        }
    }
    ui.separator();
    egui::CollapsingHeader::new("Edit spending categories").show(ui, |ui| {
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
    });
}

fn receipt(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Review a receipt");
    ui.label(
        "Paste text or open a local PNG/JPEG. Review the lines and totals before anything enters inventory.",
    );
    if state.ocr_task.is_some() {
        ui.colored_label(
            egui::Color32::from_rgb(242, 188, 108),
            "Offline OCR is running. Cancel and clear stops the engine.",
        );
    }
    egui::ScrollArea::vertical()
        .id_salt("receipt_source_scroll")
        .max_height(170.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.raw_text)
                    .desired_width(ui.available_width())
                    .hint_text("Paste receipt text here"),
            );
        });
    ui.horizontal(|ui| {
        if ui.button("Parse text").clicked() {
            if let Err(error) = state.dispatch(AppAction::ParseReceipt) {
                state.message = error;
            }
        }
        if ui
            .add_enabled(
                state.ocr_task.is_none(),
                egui::Button::new("Open photo · offline OCR"),
            )
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Receipt photo", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                let result = std::fs::metadata(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|metadata| {
                        if metadata.len() > ocr::MAX_IMAGE_BYTES as u64 {
                            Err("Receipt photo is over the 8 MB local limit".into())
                        } else {
                            std::fs::read(&path).map_err(|e| e.to_string())
                        }
                    })
                    .and_then(|bytes| state.dispatch(AppAction::StartOcr(bytes)));
                if let Err(error) = result {
                    state.message = format!("{error}. Paste or type the receipt instead.");
                }
            }
        }
        if ui.button("Cancel and clear").clicked() {
            let _ = state.dispatch(AppAction::CancelReceipt);
        }
    });
    let Some(mut draft) = state.draft.take() else {
        return;
    };
    ui.add_space(14.0);
    ui.separator();
    ui.heading("1. Check receipt details");
    ui.horizontal(|ui| {
        metric(
            ui,
            "purchase lines",
            draft.items.len().to_string(),
            egui::Color32::from_rgb(153, 225, 185),
        );
        metric(
            ui,
            "printed total",
            draft
                .printed_total_cents
                .map(money)
                .unwrap_or_else(|| "unknown".into()),
            egui::Color32::from_rgb(155, 199, 237),
        );
        metric(
            ui,
            "source flags",
            draft.uncertainty.len().to_string(),
            egui::Color32::from_rgb(242, 188, 108),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Receipt ID");
        let previous_id = draft.id.clone();
        if ui.text_edit_singleline(&mut draft.id).changed() {
            for item in &mut draft.items {
                if let Some(suffix) = item.line_id.strip_prefix(&format!("{previous_id}-L")) {
                    item.line_id = format!("{}-L{}", draft.id, suffix);
                }
            }
            state.separate_ack = false;
        }
    });
    let previous_date = draft.purchase_date.clone();
    option_text(ui, "Purchase date YYYY-MM-DD", &mut draft.purchase_date);
    if draft.purchase_date != previous_date {
        state.separate_ack = false;
    }
    if let Some((id, date)) = &state.source_match {
        ui.colored_label(egui::Color32::YELLOW,
            format!("Matching source already imported as {id} on {date}. Review the identity and date before choosing a separate purchase."));
        ui.checkbox(
            &mut state.separate_ack,
            "I checked: this is a separate purchase",
        );
    }
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
    let ready = review(draft.clone()).is_ok();
    let mut confirmed_top = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(if ready {
            "Totals reconcile. Check the lines below."
        } else {
            "Correct the lines and totals below."
        });
        if ui
            .add_enabled(ready, egui::Button::new("Confirm reviewed purchase"))
            .clicked()
        {
            state.draft = Some(draft.clone());
            match state.dispatch(AppAction::Commit(ReceiptCommit::New)) {
                Ok(()) => confirmed_top = true,
                Err(error) => state.message = error,
            }
        }
        if state.source_match.is_some()
            && ui
                .add_enabled(
                    ready && state.separate_ack,
                    egui::Button::new("Confirm separate purchase"),
                )
                .clicked()
        {
            state.draft = Some(draft.clone());
            match state.dispatch(AppAction::Commit(ReceiptCommit::Separate)) {
                Ok(()) => confirmed_top = true,
                Err(error) => state.message = error,
            }
        }
    });
    if confirmed_top {
        return;
    }
    let mut remove = None;
    ui.separator();
    ui.heading("2. Purchased items");
    ui.label(
        "Open each line to correct its name, quantity, unit, category, price and optional expiry.",
    );
    for (index, item) in draft.items.iter_mut().enumerate() {
        egui::CollapsingHeader::new(format!("{} · {}", index + 1, item.name))
            .default_open(index < 2 || !item.uncertainty.is_empty())
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
    ui.heading("3. Refunds and adjustments");
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
    if valid {
        ui.colored_label(egui::Color32::from_rgb(153, 225, 185), "Ready to confirm. Inventory and spending will update only when you choose a confirmation action.");
    } else {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Needs correction: item fields, refund limits, subtotal and total must reconcile.",
        );
    }
    ui.heading("4. Confirm your review");
    let mut confirmed = false;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(valid, egui::Button::new("Confirm new receipt"))
            .clicked()
        {
            state.draft = Some(draft.clone());
            match state.dispatch(AppAction::Commit(ReceiptCommit::New)) {
                Ok(()) => confirmed = true,
                Err(error) => state.message = error,
            }
        }
        if state.source_match.is_some()
            && ui
                .add_enabled(
                    valid && state.separate_ack,
                    egui::Button::new("Confirm separate purchase"),
                )
                .clicked()
        {
            state.draft = Some(draft.clone());
            match state.dispatch(AppAction::Commit(ReceiptCommit::Separate)) {
                Ok(()) => confirmed = true,
                Err(error) => state.message = error,
            }
        }
        if ui
            .add_enabled(valid, egui::Button::new("Correct existing receipt"))
            .clicked()
        {
            state.draft = Some(draft.clone());
            match state.dispatch(AppAction::Commit(ReceiptCommit::Correct)) {
                Ok(()) => confirmed = true,
                Err(error) => state.message = error,
            }
        }
    });
    if confirmed {
        state.clear_receipt();
    } else {
        state.draft = Some(draft);
    }
}

fn meals(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Meals from your stock");
    ui.label("Suggestions use confirmed quantities. Estimated cost comes from remaining lot cost; unknown prices stay unknown.");
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
            let ready = meals.iter().filter(|meal| meal.available).count();
            let use_soon = meals
                .iter()
                .filter(|meal| !meal.soon_expiring.is_empty())
                .count();
            ui.horizontal(|ui| {
                metric(
                    ui,
                    "cook now",
                    ready.to_string(),
                    egui::Color32::from_rgb(153, 225, 185),
                );
                metric(
                    ui,
                    "use soon",
                    use_soon.to_string(),
                    egui::Color32::from_rgb(242, 188, 108),
                );
            });
            if state.snapshot.lots.is_empty() {
                ui.label("Confirm a receipt to see which meals you can cook from your own stock.");
            }
            for (index, meal) in meals.into_iter().enumerate() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_min_width(ui.available_width().min(760.0));
                    ui.colored_label(
                        if meal.available {
                            egui::Color32::from_rgb(153, 225, 185)
                        } else {
                            egui::Color32::from_rgb(236, 187, 117)
                        },
                        if meal.available {
                            "READY FROM STOCK"
                        } else {
                            "NEEDS INGREDIENTS"
                        },
                    );
                    egui::CollapsingHeader::new(&meal.name)
                        .default_open(index < 2)
                        .show(ui, |ui| {
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
                });
            }
        }
        Err(error) => {
            ui.label(error);
        }
    }
}

fn spending(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Spending overview");
    ui.label("Receipt cash, item allocation and stock-use costs are shown separately so they do not double count.");
    ui.horizontal(|ui| {
        metric(
            ui,
            "cash flow",
            money(state.snapshot.cash_flow_cents),
            egui::Color32::from_rgb(153, 225, 185),
        );
        metric(
            ui,
            "food lines",
            money(state.snapshot.food_spending_cents),
            egui::Color32::from_rgb(155, 199, 237),
        );
        metric(
            ui,
            "discarded",
            money(state.snapshot.discarded_cost_cents),
            egui::Color32::from_rgb(242, 188, 108),
        );
    });
    if state.snapshot.receipt_count == 0 {
        ui.label("No confirmed purchases yet. Review a receipt to start a local spending history.");
    }
    ui.add_space(8.0);
    ui.heading("Allocation details");
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
    ui.heading("Weekly and monthly budgets");
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
            let result = state.export_backup_bytes().and_then(|bytes| {
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
                .and_then(|bytes| state.dispatch(AppAction::PreviewBackup(bytes)));
            match result {
                Ok(()) => state.message = format!("Preview loaded from {}", path.display()),
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
        if let Some(mode) = mode {
            if let Err(error) = state.dispatch(AppAction::ApplyBackup(mode)) {
                state.message = error;
            }
        }
    }
    if ui.button("Undo last replacement").clicked() {
        if let Err(error) = state.dispatch(AppAction::UndoBackup) {
            state.message = error;
        }
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

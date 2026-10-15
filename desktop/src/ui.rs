use std::io::Write;
use std::sync::Arc;

use bevy_egui::egui::{self, RichText};
use hungerstrike_core::{
    budget_view, recategorize_item, review, set_budget, suggest_meals, DraftItem, InventoryAction,
    MealSuggestion, Period, Recipe, RecipeCollection, RefundLine, ReplaceOrReconcile,
};

use crate::{ocr, AppAction, AppState, MealFilter, ReceiptCommit, Tab};

fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    format!("{sign}${}.{:02}", absolute / 100, absolute % 100)
}

fn brand_texture(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<egui::TextureHandle> {
    let decoded = image::load_from_memory(png).ok()?.into_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    Some(ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw()),
        egui::TextureOptions::LINEAR,
    ))
}

fn launch_screen(ctx: &egui::Context, state: &AppState) {
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(246, 248, 246)))
        .show(ctx, |ui| {
            ui.add_space(((ui.available_height() - 240.0) / 2.0).max(0.0));
            ui.vertical_centered(|ui| {
                if let Some(logo) = &state.dormouse_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(64.0, 64.0))));
                }
                ui.add_space(12.0);
                ui.label(RichText::new("HungerStrike").size(28.0).strong());
                ui.label("Opening your local kitchen");
                ui.add_space(36.0);
                ui.label(
                    RichText::new("CREATED BY")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(84, 103, 105)),
                );
                if let Some(logo) = &state.pwds_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(80.0, 34.0))));
                } else {
                    ui.label("PWDS");
                }
            });
        });
}

fn about_window(ctx: &egui::Context, state: &mut AppState) {
    let dormouse_logo = state.dormouse_logo.clone();
    let pwds_logo = state.pwds_logo.clone();
    egui::Window::new("About HungerStrike")
        .open(&mut state.show_about)
        .resizable(false)
        .collapsible(false)
        .default_width(310.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if let Some(logo) = &dormouse_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(56.0, 56.0))));
                }
                ui.label(RichText::new("HungerStrike").size(23.0).strong());
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("Your kitchen, from the receipt onward.");
                ui.add_space(16.0);
                ui.separator();
                ui.label(
                    RichText::new("CREATED BY")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(84, 103, 105)),
                );
                if let Some(logo) = &pwds_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(80.0, 34.0))));
                } else {
                    ui.label("PWDS");
                }
                ui.hyperlink_to("prowebdevsolutions.com", "https://prowebdevsolutions.com");
                ui.add_space(8.0);
            });
        });
}

fn parse_budget_dollars(input: &str) -> Option<i64> {
    let text = input
        .trim()
        .strip_prefix('$')
        .unwrap_or(input.trim())
        .trim();
    let (dollars, cents) = text.split_once('.').unwrap_or((text, ""));
    let groups: Vec<_> = dollars.split(',').collect();
    let grouped_correctly = groups[0].len() >= 1
        && groups[0].len() <= 3
        && groups[0].bytes().all(|byte| byte.is_ascii_digit())
        && groups
            .iter()
            .skip(1)
            .all(|group| group.len() == 3 && group.bytes().all(|byte| byte.is_ascii_digit()));
    if dollars.is_empty()
        || (groups.len() > 1 && !grouped_correctly)
        || (groups.len() == 1 && !dollars.bytes().all(|byte| byte.is_ascii_digit()))
        || cents.len() > 2
        || !cents.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let dollars = dollars.replace(',', "").parse::<i64>().ok()?;
    let cents = match cents.len() {
        0 => 0,
        1 => cents.parse::<i64>().ok()?.checked_mul(10)?,
        _ => cents.parse::<i64>().ok()?,
    };
    dollars
        .checked_mul(100)?
        .checked_add(cents)
        .filter(|value| *value <= 100_000_000)
}

fn status_badge(ui: &mut egui::Ui, message: &str) {
    if message.starts_with("SYNTHETIC TEST DATA") {
        egui::Frame::default()
            .fill(egui::Color32::from_rgb(231, 243, 239))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("SYNTHETIC TEST DATA")
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(17, 101, 93)),
                );
            });
    } else {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(message);
        });
    }
}

fn dashboard_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.strong(value);
        });
    });
}

fn metric(ui: &mut egui::Ui, label: &str, value: String, color: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::WHITE)
        .inner_margin(egui::Margin::symmetric(4, 5))
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(218, 226, 225),
        ))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_min_width(72.0);
                ui.set_max_width(72.0);
                ui.set_min_height(50.0);
                ui.label(RichText::new(value).size(18.0).strong().color(color));
                ui.label(
                    RichText::new(label)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(84, 103, 105)),
                );
            });
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
        state.dormouse_logo = brand_texture(
            ctx,
            "HungerStrike original dormouse",
            include_bytes!("../../assets/branding/HungerStrike.png"),
        );
        state.pwds_logo = brand_texture(
            ctx,
            "PWDS original mark",
            include_bytes!("../../assets/branding/PWDS-logo.png"),
        );
        // Use the installed Windows UI face when present; retain egui's bundled
        // fallback fonts on other platforms. No font bytes are redistributed.
        let font_root = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
            .join("Fonts");
        let mut heading_family = egui::FontFamily::Proportional;
        if let (Ok(regular), Ok(medium)) = (
            std::fs::read(font_root.join("segoeui.ttf")),
            std::fs::read(font_root.join("seguisb.ttf")),
        ) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "System Sans".into(),
                Arc::new(egui::FontData::from_owned(regular)),
            );
            fonts.font_data.insert(
                "System Sans Medium".into(),
                Arc::new(egui::FontData::from_owned(medium)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "System Sans".into());
            heading_family = egui::FontFamily::Name("Interface Medium".into());
            fonts.families.insert(
                heading_family.clone(),
                vec!["System Sans Medium".into(), "System Sans".into()],
            );
            ctx.set_fonts(fonts);
        }
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(15.0, heading_family.clone()),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(25.0, heading_family),
        );
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
        style.spacing.item_spacing = egui::vec2(9.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.visuals.panel_fill = egui::Color32::from_rgb(246, 248, 246);
        style.visuals.window_fill = egui::Color32::WHITE;
        style.visuals.extreme_bg_color = egui::Color32::WHITE;
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(36, 52, 54));
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(17, 110, 107);
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(232, 238, 236);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(216, 232, 228);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(18, 114, 110);
        ctx.set_style(style);
        state.style_ready = true;
        // Show the launch view while the font atlas is applied on the next pass.
        launch_screen(ctx, state);
        return;
    }
    if state.hold_launch_for_capture {
        launch_screen(ctx, state);
        return;
    }
    if !state.startup_complete {
        if state.scene_dirty {
            launch_screen(ctx, state);
            return;
        }
        state.startup_complete = true;
    }
    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(29, 43, 46))
                .inner_margin(egui::Margin::symmetric(22, 9)),
        )
        .show(ctx, |ui| {
            ui.set_min_height(48.0);
            ui.horizontal_wrapped(|ui| {
                if let Some(logo) = &state.dormouse_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(38.0, 38.0))));
                    ui.add_space(5.0);
                }
                ui.label(
                    RichText::new("HungerStrike")
                        .size(22.0)
                        .strong()
                        .color(egui::Color32::from_rgb(226, 244, 237)),
                );
                ui.add_space(16.0);
                for (label, tab) in [
                    ("Kitchen", Tab::Kitchen),
                    ("Receipt", Tab::Receipt),
                    ("Meals", Tab::Meals),
                    ("Spending", Tab::Spending),
                    ("Data", Tab::Data),
                ] {
                    if ui
                        .selectable_label(
                            state.tab == tab,
                            RichText::new(label)
                                .size(15.0)
                                .color(egui::Color32::from_rgb(229, 239, 237)),
                        )
                        .clicked()
                    {
                        let _ = state.dispatch(AppAction::Navigate(tab));
                    }
                }
                ui.add_space(18.0);
                if ui.button("About").clicked() {
                    state.show_about = true;
                }
                ui.label(
                    RichText::new("PRIVATE · LOCAL DATA")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(162, 188, 178)),
                );
            });
        });
    if state.tab == Tab::Kitchen {
        egui::SidePanel::left("storage_rail")
            .resizable(false)
            .default_width(294.0)
            .min_width(275.0)
            .max_width(305.0)
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(248, 250, 249))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                if !state.message.is_empty() {
                    status_badge(ui, &state.message);
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| kitchen(ui, state));
            });
        egui::SidePanel::right("lot_details")
            .resizable(false)
            .default_width(275.0)
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(248, 250, 249))
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| lot_inspector(ui, state));
            });
        egui::TopBottomPanel::bottom("scene_help")
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(235, 241, 239))
                    .inner_margin(egui::Margin::symmetric(15, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("SELECT  •  door or stocked item").size(12.0));
                    if ui.button("Rotate left").clicked() {
                        state.camera_yaw -= 0.35;
                    }
                    if ui.button("Rotate right").clicked() {
                        state.camera_yaw += 0.35;
                    }
                    if ui.button("Zoom +").clicked() {
                        state.camera_distance = (state.camera_distance - 1.0).max(5.8);
                    }
                    if ui.button("Zoom out").clicked() {
                        state.camera_distance = (state.camera_distance + 1.0).min(15.0);
                    }
                });
            });
        egui::Area::new("scene_caption".into())
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(312.0, 70.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 237))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("KITCHEN  /  STORAGE VIEW")
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::from_rgb(24, 106, 101)),
                        );
                        ui.label(
                            RichText::new(match state.zone.as_str() {
                                "fridge" => "Refrigerator",
                                "freezer" => "Freezer drawer",
                                "pantry" => "Pantry cabinet",
                                _ => "Your kitchen",
                            })
                            .size(19.0)
                            .strong(),
                        );
                        ui.label(
                            RichText::new("Illustrative forms; exact amounts in the list")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(82, 101, 102)),
                        );
                        if let Some(lot) = state.snapshot.lots.iter().find(|lot| {
                            lot.id == state.lot_id
                                && (state.zone == "all" || lot.zone == state.zone)
                        }) {
                            ui.separator();
                            ui.label(
                                RichText::new(format!(
                                    "Selected: {} · {} {}",
                                    lot.name, lot.quantity, lot.unit
                                ))
                                .size(13.0)
                                .strong(),
                            );
                        }
                    });
            });
    } else {
        if state.tab == Tab::Receipt {
            egui::SidePanel::right("receipt_review_summary")
                .resizable(false)
                .default_width(280.0)
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(248, 250, 249))
                        .inner_margin(egui::Margin::same(18)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("RECEIPT / REVIEW")
                            .size(11.0)
                            .strong()
                            .color(egui::Color32::from_rgb(24, 106, 101)),
                    );
                    ui.heading("Check before adding");
                    if let Some(draft) = &state.draft {
                        let ready = review(draft.clone()).is_ok();
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::WHITE)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(if ready {
                                        "Ready for final review"
                                    } else {
                                        "Needs correction"
                                    })
                                    .size(18.0)
                                    .strong()
                                    .color(if ready {
                                        egui::Color32::from_rgb(16, 112, 100)
                                    } else {
                                        egui::Color32::from_rgb(160, 96, 30)
                                    }),
                                );
                                ui.label(format!("{} parsed purchase lines", draft.items.len()));
                                ui.label(format!(
                                    "Printed total: {}",
                                    draft
                                        .printed_total_cents
                                        .map(money)
                                        .unwrap_or_else(|| "unknown".into())
                                ));
                                ui.label(format!("{} source flags", draft.uncertainty.len()));
                            });
                        if state.source_match.is_some() {
                            ui.colored_label(
                                egui::Color32::from_rgb(160, 96, 30),
                                "Matching source exists. Review the duplicate warning.",
                            );
                        }
                    } else {
                        ui.label("Paste receipt text or open a local photo, then parse it.");
                    }
                    ui.separator();
                    ui.strong("Confirm only after checking");
                    ui.label("Date, item names, quantities, units, prices, discounts, tax and totals.");
                    ui.add_space(8.0);
                    ui.label("Purchase date never sets expiry. A scanned draft never writes inventory by itself.");
                });
        }
        if state.tab == Tab::Meals {
            egui::SidePanel::right("meal_details")
                .resizable(false)
                .default_width(340.0)
                .min_width(315.0)
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(248, 250, 249))
                        .inner_margin(egui::Margin::same(18)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| meal_details(ui, state));
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(246, 248, 246))
                    .inner_margin(egui::Margin::same(26)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        if state.tab == Tab::Meals {
                            ui.set_max_width(1300.0);
                        }
                        if !state.message.is_empty() {
                            status_badge(ui, &state.message);
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
    if state.show_about {
        about_window(ctx, state);
    }
}

fn kitchen(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Your kitchen");
    ui.label("Confirmed stock by storage zone.");
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
            egui::Color32::from_rgb(16, 112, 100),
        );
        metric(
            ui,
            "use soon",
            soon.to_string(),
            egui::Color32::from_rgb(160, 96, 30),
        );
        metric(
            ui,
            "stock value",
            money(state.snapshot.remaining_food_cost_cents),
            egui::Color32::from_rgb(45, 99, 135),
        );
    });
    ui.add_space(10.0);
    ui.strong("Browse a storage zone");
    egui::Grid::new("storage_zone_tabs")
        .num_columns(2)
        .spacing(egui::vec2(18.0, 4.0))
        .show(ui, |ui| {
            for (index, (zone, label)) in [
                ("all", "All stock"),
                ("fridge", "Fridge"),
                ("freezer", "Freezer"),
                ("pantry", "Pantry"),
            ]
            .into_iter()
            .enumerate()
            {
                if ui
                    .selectable_label(
                        state.zone == zone,
                        RichText::new(label).color(if state.zone == zone {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(39, 58, 59)
                        }),
                    )
                    .clicked()
                {
                    let _ = state.dispatch(AppAction::SelectZone(zone.into()));
                }
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    ui.separator();
    let lots = state.snapshot.lots.clone();
    let visible: Vec<_> = lots
        .into_iter()
        .filter(|lot| lot.quantity != "0" && (state.zone == "all" || lot.zone == state.zone))
        .collect();
    ui.label(
        RichText::new(format!("{} confirmed stock lot(s) shown", visible.len()))
            .size(12.0)
            .color(egui::Color32::from_rgb(82, 101, 102)),
    );
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
        let selected = state.lot_id == lot.id;
        let row_width = ui.available_width();
        let row = egui::Frame::group(ui.style())
            .fill(if selected {
                egui::Color32::from_rgb(231, 243, 239)
            } else {
                egui::Color32::WHITE
            })
            .stroke(egui::Stroke::new(
                1.0,
                if selected {
                    egui::Color32::from_rgb(17, 110, 107)
                } else {
                    egui::Color32::from_rgb(217, 226, 222)
                },
            ))
            .inner_margin(egui::Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.set_width((row_width - 18.0).max(0.0));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&lot.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} {}", lot.quantity, lot.unit));
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "{} · {} · use by {}",
                        lot.zone,
                        if lot.cost_unknown {
                            "cost unknown".into()
                        } else {
                            money(lot.remaining_cost_cents)
                        },
                        lot.expiry_date.as_deref().unwrap_or("unknown")
                    ))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(82, 101, 102)),
                );
            });
        if ui
            .interact(
                row.response.rect,
                ui.id().with(&lot.id),
                egui::Sense::click(),
            )
            .clicked()
        {
            let _ = state.dispatch(AppAction::SelectLot(lot.id.clone()));
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

fn lot_inspector(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(
        RichText::new("STOCK / DETAILS")
            .size(11.0)
            .strong()
            .color(egui::Color32::from_rgb(24, 106, 101)),
    );
    ui.heading("Selected item");
    let selected = state
        .snapshot
        .lots
        .iter()
        .find(|lot| lot.id == state.lot_id)
        .cloned();
    if let Some(lot) = selected {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::WHITE)
            .show(ui, |ui| {
                ui.label(RichText::new(&lot.name).size(20.0).strong());
                ui.label(format!("{} {}  •  {}", lot.quantity, lot.unit, lot.zone));
                ui.separator();
                ui.label(format!(
                    "Remaining stock cost: {}",
                    if lot.cost_unknown {
                        "unknown".into()
                    } else {
                        money(lot.remaining_cost_cents)
                    }
                ));
                ui.label(format!(
                    "Use by: {}",
                    lot.expiry_date.as_deref().unwrap_or("unknown")
                ));
            });
        ui.add_space(6.0);
        ui.strong("Manage confirmed stock");
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
        ui.separator();
        ui.strong("Move to");
        ui.horizontal_wrapped(|ui| {
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
    } else {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::WHITE)
            .show(ui, |ui| {
                ui.label(RichText::new("Nothing selected yet").strong());
                ui.label("Select stock in the scene or left list.");
            });
    }
    ui.add_space(14.0);
    ui.separator();
    let today = chrono::NaiveDate::parse_from_str(&state.reference_date, "%Y-%m-%d").ok();
    let use_soon: Vec<_> = state
        .snapshot
        .lots
        .iter()
        .filter(|lot| lot.quantity != "0" && (state.zone == "all" || lot.zone == state.zone))
        .filter(|lot| {
            today
                .zip(
                    lot.expiry_date
                        .as_deref()
                        .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
                )
                .is_some_and(|(now, expiry)| {
                    (0..=3).contains(&expiry.signed_duration_since(now).num_days())
                })
        })
        .collect();
    egui::CollapsingHeader::new(format!("Use soon in this view ({})", use_soon.len())).show(
        ui,
        |ui| {
            for lot in use_soon {
                ui.label(format!(
                    "{}  ·  {}",
                    lot.name,
                    lot.expiry_date.as_deref().unwrap_or("unknown")
                ));
            }
            ui.label("Dates are planning cues; check food condition before use.");
        },
    );
}

fn receipt(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Review a receipt");
    ui.label(
        "Paste text or open a local PNG/JPEG. Review the lines and totals before anything enters inventory.",
    );
    if state.ocr_task.is_some() {
        ui.colored_label(
            egui::Color32::from_rgb(160, 96, 30),
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
            egui::Color32::from_rgb(16, 112, 100),
        );
        metric(
            ui,
            "printed total",
            draft
                .printed_total_cents
                .map(money)
                .unwrap_or_else(|| "unknown".into()),
            egui::Color32::from_rgb(45, 99, 135),
        );
        metric(
            ui,
            "source flags",
            draft.uncertainty.len().to_string(),
            egui::Color32::from_rgb(160, 96, 30),
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
        ui.colored_label(egui::Color32::from_rgb(160, 96, 30),
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
    ui.label(if ready {
        "Totals reconcile. Check every line below before confirming at the end."
    } else {
        "Correct the lines and totals below before confirming."
    });
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
        ui.colored_label(egui::Color32::from_rgb(16, 112, 100), "Ready to confirm. Inventory and spending will update only when you choose a confirmation action.");
    } else {
        ui.colored_label(
            egui::Color32::from_rgb(160, 96, 30),
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
    ui.set_max_width(810.0);
    ui.heading("Meals from your kitchen");
    ui.label("Six original recipes bundled locally. Suggestions use confirmed fridge, freezer, and pantry stock.");
    ui.horizontal(|ui| {
        ui.label("Plan for YYYY-MM-DD");
        ui.text_edit_singleline(&mut state.reference_date);
    });
    let (_, suggestions) = match recipe_suggestions(state) {
        Ok(result) => result,
        Err(error) => {
            ui.label(error);
            return;
        }
    };
    let cook_count = suggestions.iter().filter(|meal| meal.available).count();
    let soon_count = suggestions
        .iter()
        .filter(|meal| meal.available && !meal.soon_expiring.is_empty())
        .count();
    let plan_count = suggestions.len() - cook_count;
    ui.horizontal_wrapped(|ui| {
        for (label, filter, count) in [
            ("Cook now", MealFilter::CookNow, cook_count),
            ("Use soon", MealFilter::UseSoon, soon_count),
            ("Plan next", MealFilter::PlanNext, plan_count),
        ] {
            if ui
                .selectable_label(state.meal_filter == filter, format!("{label}  {count}"))
                .clicked()
            {
                state.meal_filter = filter;
                state.selected_recipe_id = None;
            }
        }
    });
    if state.meal_filter == MealFilter::UseSoon {
        ui.label("Dates are user-entered planning cues. Check food condition before cooking.");
    }
    let visible: Vec<_> = suggestions
        .iter()
        .filter(|meal| in_meal_filter(meal, state.meal_filter))
        .collect();
    let selected_id = effective_selected_meal(
        &suggestions,
        state.meal_filter,
        state.selected_recipe_id.as_deref(),
    )
    .map(|meal| meal.id.as_str());
    if visible.is_empty() {
        ui.label(match state.meal_filter {
            MealFilter::CookNow => "No complete recipe from current usable stock. Try Plan next.",
            MealFilter::UseSoon => {
                "No complete recipe uses stock with a known date in the next three days."
            }
            MealFilter::PlanNext => "All bundled recipes are ready from your stock.",
        });
    }
    for meal in visible {
        meal_card(ui, meal, selected_id == Some(meal.id.as_str()), state);
    }
    if state.meal_filter == MealFilter::PlanNext {
        ui.add_space(10.0);
        ui.separator();
        ui.heading("Shopping list");
        ui.label(
            "Saved on this device. Missing grocery prices are unknown until you review a purchase.",
        );
        let shopping = state.snapshot.shopping.clone();
        if shopping.is_empty() {
            ui.label("Choose a planned recipe to add its missing quantities.");
        }
        for item in shopping {
            ui.horizontal(|ui| {
                ui.label(format!("{}  ·  {} {}", item.name, item.quantity, item.unit));
                if ui.button("Remove").clicked() {
                    if let Err(error) = state.dispatch(AppAction::RemoveShopping {
                        name: item.name,
                        unit: item.unit,
                    }) {
                        state.message = error;
                    }
                }
            });
        }
    }
}

fn recipe_suggestions(state: &AppState) -> Result<(Vec<Recipe>, Vec<MealSuggestion>), String> {
    let collection: RecipeCollection =
        serde_json::from_str(include_str!("../../assets/recipes.json"))
            .map_err(|error| error.to_string())?;
    let suggestions = suggest_meals(&state.snapshot, &collection.recipes, &state.reference_date)?;
    Ok((collection.recipes, suggestions))
}

fn in_meal_filter(meal: &MealSuggestion, filter: MealFilter) -> bool {
    match filter {
        MealFilter::CookNow => meal.available,
        MealFilter::UseSoon => meal.available && !meal.soon_expiring.is_empty(),
        MealFilter::PlanNext => !meal.available,
    }
}

fn effective_selected_meal<'a>(
    suggestions: &'a [MealSuggestion],
    filter: MealFilter,
    selected_id: Option<&str>,
) -> Option<&'a MealSuggestion> {
    selected_id
        .and_then(|id| {
            suggestions
                .iter()
                .find(|meal| meal.id == id && in_meal_filter(meal, filter))
        })
        .or_else(|| suggestions.iter().find(|meal| in_meal_filter(meal, filter)))
}

fn meal_card(ui: &mut egui::Ui, meal: &MealSuggestion, selected: bool, state: &mut AppState) {
    let have = meal
        .ingredients
        .iter()
        .filter(|item| item.missing_quantity == "0")
        .count();
    let cost = meal
        .estimate_cents
        .map(money)
        .unwrap_or_else(|| "unknown".into());
    egui::Frame::group(ui.style())
        .fill(if selected {
            egui::Color32::from_rgb(231, 243, 239)
        } else {
            egui::Color32::WHITE
        })
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(217, 226, 222),
        ))
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 18.0).max(250.0));
            ui.set_min_height(82.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&meal.name).size(18.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("View recipe").clicked() {
                        state.selected_recipe_id = Some(meal.id.clone());
                    }
                });
            });
            ui.label(format!(
                "{} serving{}  ·  have {have}/{} ingredients  ·  stock cost {cost}",
                meal.servings,
                if meal.servings == 1 { "" } else { "s" },
                meal.ingredients.len()
            ));
            if !meal.available {
                ui.label("Missing groceries are unpriced");
            } else if !meal.soon_expiring.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(160, 96, 30),
                    format!("Use soon: {}", meal.soon_expiring.join(", ")),
                );
            }
        });
}

fn meal_details(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("MEALS / DETAILS").size(11.0).strong());
    let (recipes, suggestions) = match recipe_suggestions(state) {
        Ok(result) => result,
        Err(error) => {
            ui.label(error);
            return;
        }
    };
    let selected = effective_selected_meal(
        &suggestions,
        state.meal_filter,
        state.selected_recipe_id.as_deref(),
    );
    let Some(meal) = selected else {
        ui.label("Choose a recipe from the list.");
        return;
    };
    let Some(recipe) = recipes.iter().find(|recipe| recipe.id == meal.id).cloned() else {
        return;
    };
    ui.heading(meal.name.as_str());
    ui.label(format!(
        "{} serving{}",
        meal.servings,
        if meal.servings == 1 { "" } else { "s" }
    ));
    ui.label(format!(
        "Recorded stock cost: {}",
        meal.estimate_cents
            .map(money)
            .unwrap_or_else(|| "unknown".into())
    ));
    if !meal.available {
        ui.label("Missing grocery prices are unknown; this is not a purchase estimate.");
    }
    if !meal.soon_expiring.is_empty() {
        ui.colored_label(
            egui::Color32::from_rgb(160, 96, 30),
            format!("Use soon: {}", meal.soon_expiring.join(", ")),
        );
        ui.label("Check dates and food condition before cooking.");
    }
    ui.separator();
    ui.strong("Ingredients");
    for ingredient in &meal.ingredients {
        ui.label(format!(
            "{}  ·  {} {} needed",
            ingredient.name, ingredient.required_quantity, ingredient.unit
        ));
        ui.label(format!(
            "Have {} {}{}",
            ingredient.stock_used_quantity,
            ingredient.unit,
            if ingredient.missing_quantity == "0" {
                String::new()
            } else {
                format!(
                    "  ·  need {} {}",
                    ingredient.missing_quantity, ingredient.unit
                )
            }
        ));
    }
    if meal.available {
        ui.separator();
        ui.strong("Stock to consume when you cook");
        for lot in &meal.consumption {
            ui.label(format!(
                "{}  ·  {} {} from {}",
                lot.name, lot.quantity, lot.unit, lot.zone
            ));
        }
    }
    ui.separator();
    ui.strong("Directions");
    for (index, step) in meal.steps.iter().enumerate() {
        ui.label(format!("{}. {}", index + 1, step));
    }
    ui.add_space(8.0);
    if meal.available {
        if ui.button("Cook and update stock").clicked() {
            if let Err(error) = state.dispatch(AppAction::CookRecipe(recipe)) {
                state.message = error;
            }
        }
    } else {
        if let Ok(store) = state.store.lock() {
            let mut over_budget = false;
            for (period, label) in [(Period::Week, "Weekly"), (Period::Month, "Monthly")] {
                if let Ok(view) =
                    budget_view(&store, &state.reference_date, period, state.week_start)
                {
                    if let Some(remaining) = view.remaining_budget_cents {
                        ui.label(format!("{label} budget remaining: {}", money(remaining)));
                        over_budget |= remaining <= 0;
                    }
                }
            }
            if over_budget {
                ui.label("Review your budget before buying missing items.");
            }
        }
        if ui.button("Add missing to shopping list").clicked() {
            if let Err(error) = state.dispatch(AppAction::AddMissingToShopping(recipe)) {
                state.message = error;
            }
        }
    }
}

fn spending(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Spending overview");
    ui.label("Your confirmed receipts, food allocation and stock-use costs in one local view.");
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        metric(
            ui,
            "cash flow",
            money(state.snapshot.cash_flow_cents),
            egui::Color32::from_rgb(16, 112, 100),
        );
        metric(
            ui,
            "food lines",
            money(state.snapshot.food_spending_cents),
            egui::Color32::from_rgb(45, 99, 135),
        );
        metric(
            ui,
            "discarded",
            money(state.snapshot.discarded_cost_cents),
            egui::Color32::from_rgb(160, 96, 30),
        );
    });
    if state.snapshot.receipt_count == 0 {
        ui.label("No confirmed purchases yet. Review a receipt to start a local spending history.");
    }
    ui.horizontal_wrapped(|ui| {
        for (period, label) in [(Period::Week, "Weekly"), (Period::Month, "Monthly")] {
            let remaining = state
                .store
                .lock()
                .map_err(|error| error.to_string())
                .and_then(|store| {
                    budget_view(&store, &state.reference_date, period, state.week_start)
                })
                .map(|view| {
                    view.remaining_budget_cents
                        .map(money)
                        .unwrap_or_else(|| "not set".into())
                })
                .unwrap_or_else(|_| "unavailable".into());
            ui.strong(format!("{label} remaining: {remaining}"));
        }
    });
    ui.add_space(4.0);
    ui.columns(2, |columns| {
        egui::Frame::group(columns[0].style())
            .fill(egui::Color32::WHITE)
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(215, 225, 223),
            ))
            .show(&mut columns[0], |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.set_min_width(ui.available_width() - 16.0);
                ui.set_min_height(150.0);
                ui.label(
                    RichText::new("PURCHASE ALLOCATION")
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(16, 112, 100)),
                );
                ui.label(
                    RichText::new("Where receipt cash went")
                        .text_style(egui::TextStyle::Heading)
                        .size(18.0),
                );
                ui.add_space(1.0);
                dashboard_row(
                    ui,
                    "Cash paid / refunded",
                    money(state.snapshot.cash_flow_cents),
                );
                ui.separator();
                dashboard_row(
                    ui,
                    "Food lines and refunds",
                    money(state.snapshot.food_spending_cents),
                );
                dashboard_row(
                    ui,
                    "Household lines and refunds",
                    money(state.snapshot.household_goods_cents),
                );
                dashboard_row(ui, "Tax", money(state.snapshot.tax_cents));
                ui.add_space(1.0);
                ui.label(
                    RichText::new(
                        "Allocation of the cash flow above; these amounts are not added again.",
                    )
                    .size(13.0)
                    .color(egui::Color32::from_rgb(91, 106, 106)),
                );
            });
        egui::Frame::group(columns[1].style())
            .fill(egui::Color32::WHITE)
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(215, 225, 223),
            ))
            .show(&mut columns[1], |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.set_min_width(ui.available_width() - 16.0);
                ui.set_min_height(150.0);
                ui.label(
                    RichText::new("STOCK COST EVENTS")
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(45, 99, 135)),
                );
                ui.label(
                    RichText::new("After purchase")
                        .text_style(egui::TextStyle::Heading)
                        .size(18.0),
                );
                ui.add_space(1.0);
                dashboard_row(
                    ui,
                    "Consumed food cost",
                    money(state.snapshot.consumed_cost_cents),
                );
                dashboard_row(
                    ui,
                    "Discarded food cost",
                    money(state.snapshot.discarded_cost_cents),
                );
                ui.separator();
                dashboard_row(
                    ui,
                    "Refund without stock reversal",
                    money(state.snapshot.refund_without_stock_reversal_cents),
                );
                if state.snapshot.refund_without_stock_reversal_cents != 0 {
                    ui.label(
                        RichText::new(
                            "Refund paid back; stock kept. Its quantity and remaining cost stay until used or discarded.",
                        )
                        .size(12.0)
                        .color(egui::Color32::from_rgb(91, 106, 106)),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "Unknown cost: {} consumption, {} discard events · {} unpriced lots",
                        state.snapshot.unknown_consumption_count,
                        state.snapshot.unknown_discard_count,
                        state.snapshot.unpriced_lot_count
                    ))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(91, 106, 106)),
                );
            });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.heading("Weekly and monthly budgets");
        ui.label(
            RichText::new("Scroll for budget controls and full breakdown")
                .size(12.0)
                .color(egui::Color32::from_rgb(91, 106, 106)),
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Period date");
        ui.text_edit_singleline(&mut state.reference_date);
        ui.label("Week starts");
        egui::ComboBox::from_id_salt("week_start")
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
    });
    ui.columns(2, |columns| {
        for (index, (period, label)) in [(Period::Week, "Weekly"), (Period::Month, "Monthly")]
            .into_iter()
            .enumerate()
        {
            let ui = &mut columns[index];
            egui::Frame::group(ui.style())
                .fill(egui::Color32::WHITE)
                .corner_radius(8)
                .inner_margin(egui::Margin::same(8))
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(215, 225, 223)))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.set_min_width(ui.available_width() - 16.0);
                    ui.set_min_height(170.0);
                    ui.label(
                        RichText::new(format!("{} PLAN", label.to_ascii_uppercase()))
                            .size(11.0)
                            .strong()
                            .color(egui::Color32::from_rgb(16, 112, 100)),
                    );
                    ui.label(RichText::new(format!("{label} budget")).text_style(egui::TextStyle::Heading).size(20.0));
                    let (budget_input, budget_text) = if matches!(period, Period::Week) {
                        (&mut state.weekly_budget, &mut state.weekly_budget_text)
                    } else {
                        (&mut state.monthly_budget, &mut state.monthly_budget_text)
                    };
                    ui.horizontal(|ui| {
                        ui.label("Budget (USD)");
                        ui.add_sized([112.0, 28.0], egui::TextEdit::singleline(budget_text).background_color(egui::Color32::from_rgb(241, 246, 244)));
                        if ui.button("Save budget").clicked() {
                            match parse_budget_dollars(budget_text) {
                                Some(cents) => {
                                    let result = state.store.lock().map_err(|e| e.to_string()).and_then(
                                        |mut store| set_budget(&mut store, period, Some(cents)),
                                    );
                                    state.message = result
                                        .map(|_| {
                                            *budget_input = cents;
                                            *budget_text = money(cents);
                                            format!("{label} budget saved locally.")
                                        })
                                        .unwrap_or_else(|e| e);
                                }
                                None => state.message = "Enter a dollar amount from $0.00 to $1,000,000.00 with no more than two decimal places.".into(),
                            }
                        }
                    });
                    let result = state
                        .store
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|store| {
                            budget_view(&store, &state.reference_date, period, state.week_start)
                        });
                    match result {
                        Ok(view) => {
                            ui.label(RichText::new(format!("{} to {} · {} receipt(s)", view.start_date, view.end_date, view.receipt_count)).size(13.0).color(egui::Color32::from_rgb(91, 106, 106)));
                            if let Some(budget) = view.budget_cents.filter(|budget| *budget > 0) {
                                let used = view.cash_flow_cents.max(0);
                                ui.add(egui::ProgressBar::new((used as f32 / budget as f32).clamp(0.0, 1.0)).desired_width(ui.available_width()).fill(egui::Color32::from_rgb(23, 125, 112)));
                                ui.label(format!("{} of {} cash flow against budget", money(view.cash_flow_cents), money(budget)));
                            } else if view.budget_cents.is_none() {
                                ui.label("Set a budget to see progress.");
                            }
                            dashboard_row(ui, "Remaining budget", view.remaining_budget_cents.map(money).unwrap_or_else(|| "not set".into()));
                            dashboard_row(ui, "Cash flow", money(view.cash_flow_cents));
                            egui::CollapsingHeader::new("Full period breakdown")
                                .default_open(false)
                                .show(ui, |ui| {
                                    dashboard_row(ui, "Food", money(view.food_spending_cents));
                                    dashboard_row(ui, "Household", money(view.household_goods_cents));
                                    dashboard_row(ui, "Tax", money(view.tax_cents));
                                    dashboard_row(ui, "Discounts", money(view.discounts_cents));
                                    dashboard_row(ui, "Refunds", money(view.refunds_cents));
                                    dashboard_row(ui, "Unallocated reconciliation", money(view.reconciliation_cents));
                                });
                        }
                        Err(error) => {
                            ui.label(error);
                        }
                    }
                });
        }
    });
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

#[cfg(test)]
mod tests {
    use super::{effective_selected_meal, parse_budget_dollars, MealFilter, MealSuggestion};

    #[test]
    fn default_and_explicit_meal_selection_use_visible_recipe() {
        let meal = |id: &str, available| MealSuggestion {
            id: id.into(),
            name: id.into(),
            servings: 1,
            steps: vec![],
            available,
            estimate_cents: None,
            estimate_status: String::new(),
            ingredients: vec![],
            consumption: vec![],
            soon_expiring: vec![],
            expiry_unknown: true,
        };
        let suggestions = [
            meal("egg-and-spinach", true),
            meal("spinach-pasta", true),
            meal("tomato-pasta", false),
        ];
        let selected = |filter, id| {
            effective_selected_meal(&suggestions, filter, id).map(|meal| meal.id.as_str())
        };

        assert_eq!(selected(MealFilter::CookNow, None), Some("egg-and-spinach"));
        assert_eq!(
            selected(MealFilter::CookNow, Some("spinach-pasta")),
            Some("spinach-pasta")
        );
        assert_eq!(
            selected(MealFilter::CookNow, Some("tomato-pasta")),
            Some("egg-and-spinach")
        );
        assert_eq!(selected(MealFilter::PlanNext, None), Some("tomato-pasta"));
    }

    #[test]
    fn budget_dollar_input_converts_to_exact_integer_cents() {
        assert_eq!(parse_budget_dollars("$40.00"), Some(4_000));
        assert_eq!(parse_budget_dollars("12.3"), Some(1_230));
        assert_eq!(parse_budget_dollars("0.01"), Some(1));
        assert_eq!(parse_budget_dollars("$1,000"), Some(100_000));
        assert_eq!(parse_budget_dollars("$1,00"), None);
        assert_eq!(parse_budget_dollars("5.999"), None);
        assert_eq!(parse_budget_dollars("-2.00"), None);
        assert_eq!(parse_budget_dollars("1000000.01"), None);
    }
}

use std::io::Write;
use std::sync::Arc;

use bevy_egui::egui::{self, RichText};
use hungerstrike_core::{
    budget_view, estimate_expiry, recategorize_item, review, set_budget, suggest_meals, DraftItem,
    InventoryAction, MealSuggestion, Period, ReceiptDraft, Recipe, RecipeCollection, RefundLine,
    ReplaceOrReconcile,
};

use crate::theme::t;
use crate::{ocr, AppAction, AppState, MealFilter, ReceiptCommit, Tab};

const INK: egui::Color32 = egui::Color32::from_rgb(36, 52, 54);
const MUTED: egui::Color32 = egui::Color32::from_rgb(84, 103, 105);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(17, 110, 107);
const ACCENT_SOFT: egui::Color32 = egui::Color32::from_rgb(236, 246, 243);
const ACCENT_BUTTON: egui::Color32 = egui::Color32::from_rgb(196, 230, 222);
const WARN: egui::Color32 = egui::Color32::from_rgb(150, 86, 20);
const CARD: egui::Color32 = egui::Color32::WHITE;
const BORDER: egui::Color32 = egui::Color32::from_rgb(217, 226, 222);

/// White rounded panel used to group related controls.
fn card<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::default()
        .fill(t(CARD))
        .stroke(egui::Stroke::new(1.0_f32, t(BORDER)))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui)
        })
        .inner
}

/// "2026-10-10", "2026-10-10 (estimate)" or "unknown".
fn expiry_label(date: Option<&str>, estimated: bool) -> String {
    match (date, estimated) {
        (Some(date), true) => format!("{date} (estimate)"),
        (Some(date), false) => date.to_string(),
        (None, _) => "unknown".into(),
    }
}

pub(crate) fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    format!("{sign}${}.{:02}", absolute / 100, absolute % 100)
}

fn expiry_before_planning_date(expiry: Option<&str>, planning_date: &str) -> bool {
    let Some(expiry) =
        expiry.and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
    else {
        return false;
    };
    let Ok(planning_date) = chrono::NaiveDate::parse_from_str(planning_date, "%Y-%m-%d") else {
        return false;
    };
    expiry < planning_date
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
        .frame(egui::Frame::default().fill(t(egui::Color32::from_rgb(246, 248, 246))))
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
                        .color(t(egui::Color32::from_rgb(84, 103, 105))),
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
                        .color(t(egui::Color32::from_rgb(84, 103, 105))),
                );
                if let Some(logo) = &pwds_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(80.0, 34.0))));
                } else {
                    ui.label("PWDS");
                }
                ui.hyperlink_to("prowebdevsolutions.com", "https://prowebdevsolutions.com");
                ui.add_space(8.0);
            });
            ui.label(
                RichText::new("Theme (Light, Dark or System) is under Data.")
                    .size(12.0)
                    .color(t(MUTED)),
            );
            egui::CollapsingHeader::new("Credits and licenses")
                .id_salt("about_credits")
                .show(ui, |ui| {
                    for line in [
                        "Receipt OCR: ocrs and rten by Robert Knight (MIT or Apache-2.0).",
                        "OCR models: ocrs text detection and recognition models (CC BY-SA 4.0), trained on HierText and synthetic data.",
                        "3D food models: Poly Haven, by Oliver Harries and others (CC0).",
                        "Built with Bevy and egui (MIT or Apache-2.0) and SQLite (public domain).",
                    ] {
                        ui.label(RichText::new(line).size(12.0));
                    }
                    ui.label(
                        RichText::new("Everything runs on this computer: no account, cloud service or paid API.")
                            .size(12.0)
                            .color(t(MUTED)),
                    );
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
    let grouped_correctly = !groups[0].is_empty()
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
            .fill(t(egui::Color32::from_rgb(231, 243, 239)))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("SYNTHETIC TEST DATA")
                        .size(11.0)
                        .strong()
                        .color(t(egui::Color32::from_rgb(17, 101, 93))),
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
        .fill(t(egui::Color32::WHITE))
        .inner_margin(egui::Margin::symmetric(4, 5))
        .stroke(egui::Stroke::new(
            1.0_f32,
            t(egui::Color32::from_rgb(218, 226, 225)),
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
                        .color(t(egui::Color32::from_rgb(84, 103, 105))),
                );
            });
        });
}

/// egui visuals for the light (original) or dark theme. The light branch
/// sets exactly the values HungerStrike has always used.
fn theme_visuals(dark: bool) -> egui::Visuals {
    crate::theme::set_dark(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = t(egui::Color32::from_rgb(246, 248, 246));
    visuals.window_fill = t(egui::Color32::WHITE);
    // Text fields sit on a faint tint with a visible outline so they read
    // as editable on cards.
    visuals.extreme_bg_color = t(egui::Color32::from_rgb(244, 247, 246));
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, t(BORDER));
    // Hover and focus rings use the accent colour.
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, t(ACCENT));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5_f32, t(ACCENT));
    visuals.override_text_color = Some(t(egui::Color32::from_rgb(36, 52, 54)));
    visuals.selection.bg_fill = crate::theme::accent_fill();
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.inactive.bg_fill = t(egui::Color32::from_rgb(232, 238, 236));
    visuals.widgets.hovered.bg_fill = t(egui::Color32::from_rgb(216, 232, 228));
    visuals.widgets.active.bg_fill = t(egui::Color32::from_rgb(18, 114, 110));
    if dark {
        visuals.widgets.inactive.weak_bg_fill = crate::theme::DARK_SURFACE_2;
        visuals.widgets.hovered.weak_bg_fill = crate::theme::DARK_HOVER;
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0_f32, crate::theme::DARK_BORDER);
        visuals.widgets.noninteractive.bg_fill = crate::theme::DARK_SURFACE;
        visuals.hyperlink_color = crate::theme::DARK_ACCENT;
        visuals.faint_bg_color = crate::theme::DARK_PANEL;
        visuals.text_cursor.stroke = egui::Stroke::new(2.0_f32, crate::theme::DARK_ACCENT);
    }
    visuals
}

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    crate::theme::set_dark(state.dark);
    if state.style_ready && state.applied_dark != Some(state.dark) {
        let mut style = (*ctx.style()).clone();
        style.visuals = theme_visuals(state.dark);
        ctx.set_style(style);
        state.applied_dark = Some(state.dark);
    }
    if !state.style_ready {
        state.dormouse_logo = brand_texture(
            ctx,
            "HungerStrike pantry mark",
            include_bytes!("../../assets/branding/HungerStrike-geometric.png"),
        );
        state.pwds_logo = brand_texture(
            ctx,
            "PWDS original mark",
            include_bytes!("../../assets/branding/PWDS-logo.png"),
        );
        // Use an installed system UI face (Segoe UI on Windows, Arial on
        // macOS, Noto Sans or DejaVu Sans on Linux) when present; egui's
        // bundled fonts stay as the fallback. No font bytes are redistributed.
        let mut heading_family = egui::FontFamily::Proportional;
        if let Some((regular, medium)) = crate::platform::load_ui_fonts() {
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
        style.visuals = theme_visuals(state.dark);
        ctx.set_style(style);
        state.applied_dark = Some(state.dark);
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
    // Compact navigation bar. App and company branding live on the launch
    // screen and in About, so the working view spends its height on content.
    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::default()
                .fill(t(egui::Color32::WHITE))
                .stroke(egui::Stroke::new(1.0_f32, t(BORDER)))
                .inner_margin(egui::Margin::symmetric(16, 6)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (label, tab, hint) in [
                    ("Kitchen", Tab::Kitchen, "Fridge, freezer and pantry stock"),
                    (
                        "Receipts",
                        Tab::Receipt,
                        "Scan or paste a receipt and review it",
                    ),
                    (
                        "Meals",
                        Tab::Meals,
                        "Recipes from what you have, and a shopping list",
                    ),
                    ("Spending", Tab::Spending, "Food spending and budgets"),
                    ("Data", Tab::Data, "Backup and restore"),
                ] {
                    let selected = state.tab == tab;
                    let mut text = RichText::new(label).size(15.0);
                    text = if selected {
                        text.color(egui::Color32::WHITE).strong()
                    } else {
                        text.color(t(INK))
                    };
                    let button = egui::Button::new(text)
                        .fill(if selected {
                            crate::theme::accent_fill()
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(6)
                        .min_size(egui::vec2(84.0, 30.0));
                    if ui.add(button).on_hover_text(hint).clicked() {
                        let _ = state.dispatch(AppAction::Navigate(tab));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new("About").min_size(egui::vec2(64.0, 28.0)))
                        .clicked()
                    {
                        state.show_about = true;
                    }
                    ui.label(
                        RichText::new("Stored on this computer only")
                            .size(12.0)
                            .color(t(MUTED)),
                    );
                    if state.synthetic_banner {
                        egui::Frame::default()
                            .fill(t(egui::Color32::from_rgb(255, 236, 214)))
                            .corner_radius(4)
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("SYNTHETIC TEST DATA")
                                        .size(12.0)
                                        .strong()
                                        .color(t(WARN)),
                                );
                            });
                    }
                });
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
                    .fill(t(egui::Color32::from_rgb(248, 250, 249)))
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
                    .fill(t(egui::Color32::from_rgb(248, 250, 249)))
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
                    .fill(t(egui::Color32::from_rgb(235, 241, 239)))
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
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(312.0, 56.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(t(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 237)))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("KITCHEN  /  STORAGE VIEW")
                                .size(11.0)
                                .strong()
                                .color(t(egui::Color32::from_rgb(24, 106, 101))),
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
                                .color(t(egui::Color32::from_rgb(82, 101, 102))),
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
                        .fill(t(egui::Color32::from_rgb(248, 250, 249)))
                        .inner_margin(egui::Margin::same(18)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("RECEIPT / REVIEW")
                            .size(11.0)
                            .strong()
                            .color(t(egui::Color32::from_rgb(24, 106, 101))),
                    );
                    ui.heading("Check before adding");
                    if let Some(draft) = &state.draft {
                        let ready = review(draft.clone()).is_ok();
                        egui::Frame::group(ui.style())
                            .fill(t(egui::Color32::WHITE))
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
                                        t(egui::Color32::from_rgb(16, 112, 100))
                                    } else {
                                        t(egui::Color32::from_rgb(160, 96, 30))
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
                                t(egui::Color32::from_rgb(160, 96, 30)),
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
                        .fill(t(egui::Color32::from_rgb(248, 250, 249)))
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
                    .fill(t(egui::Color32::from_rgb(246, 248, 246)))
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
    ui.label(format!("Planning date: {}", state.reference_date));
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
            t(egui::Color32::from_rgb(16, 112, 100)),
        );
        metric(
            ui,
            "use soon",
            soon.to_string(),
            t(egui::Color32::from_rgb(160, 96, 30)),
        );
        metric(
            ui,
            "stock value",
            money(state.snapshot.remaining_food_cost_cents),
            t(egui::Color32::from_rgb(45, 99, 135)),
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
                            t(egui::Color32::from_rgb(39, 58, 59))
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
    let mut visible: Vec<_> = lots
        .into_iter()
        .filter(|lot| lot.quantity != "0" && (state.zone == "all" || lot.zone == state.zone))
        .collect();
    // Soonest-dated stock first; undated stock keeps receipt order at the end.
    visible.sort_by_key(|lot| (lot.expiry_date.is_none(), lot.expiry_date.clone()));
    ui.label(
        RichText::new(format!("{} confirmed stock lot(s) shown", visible.len()))
            .size(12.0)
            .color(t(egui::Color32::from_rgb(82, 101, 102))),
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
                t(egui::Color32::from_rgb(231, 243, 239))
            } else {
                t(egui::Color32::WHITE)
            })
            .stroke(egui::Stroke::new(
                1.0_f32,
                if selected {
                    t(egui::Color32::from_rgb(17, 110, 107))
                } else {
                    t(egui::Color32::from_rgb(217, 226, 222))
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
                        expiry_label(lot.expiry_date.as_deref(), lot.expiry_estimated)
                    ))
                    .size(12.0)
                    .color(t(egui::Color32::from_rgb(82, 101, 102))),
                );
                if expiry_before_planning_date(lot.expiry_date.as_deref(), &state.reference_date) {
                    ui.label(
                        RichText::new("Past planning date · excluded from meal suggestions")
                            .size(12.0)
                            .color(t(egui::Color32::from_rgb(151, 77, 28))),
                    );
                }
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
            .color(t(egui::Color32::from_rgb(24, 106, 101))),
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
            .fill(t(egui::Color32::WHITE))
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
                    expiry_label(lot.expiry_date.as_deref(), lot.expiry_estimated)
                ));
                if expiry_before_planning_date(lot.expiry_date.as_deref(), &state.reference_date) {
                    ui.colored_label(
                        t(egui::Color32::from_rgb(151, 77, 28)),
                        "Past planning date. This lot is excluded from meal suggestions.",
                    );
                }
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
        ui.separator();
        ui.strong("Expiry date");
        ui.label(
            RichText::new(if lot.expiry_estimated {
                "Currently an estimate. Enter the date printed on the package to replace it."
            } else if lot.expiry_date.is_some() {
                "Entered by you."
            } else {
                "Unknown. Receipts do not print expiry dates."
            })
            .size(12.0)
            .color(if lot.expiry_estimated {
                t(WARN)
            } else {
                t(MUTED)
            }),
        );
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.expiry_text)
                    .desired_width(108.0)
                    .hint_text("YYYY-MM-DD"),
            );
            if ui
                .button("Save date")
                .on_hover_text("Save the date as entered by you")
                .clicked()
            {
                action = Some(InventoryAction::SetExpiry {
                    lot_id: state.lot_id.clone(),
                    expiry_date: Some(state.expiry_text.clone()),
                    estimated: false,
                });
            }
        });
        ui.horizontal_wrapped(|ui| {
            let estimate = estimate_expiry(
                &lot.name,
                &lot.category,
                Some(lot.zone.as_str()),
                &lot.purchase_date,
            );
            if ui
                .add_enabled(estimate.is_some(), egui::Button::new("Use estimate"))
                .on_hover_text(
                    estimate
                        .as_ref()
                        .map(|estimate| estimate.basis.clone())
                        .unwrap_or_else(|| "No estimate for this kind of item".into()),
                )
                .clicked()
            {
                if let Some(estimate) = estimate {
                    action = Some(InventoryAction::SetExpiry {
                        lot_id: state.lot_id.clone(),
                        expiry_date: Some(estimate.date),
                        estimated: true,
                    });
                }
            }
            if ui
                .add_enabled(lot.expiry_date.is_some(), egui::Button::new("Clear"))
                .clicked()
            {
                action = Some(InventoryAction::SetExpiry {
                    lot_id: state.lot_id.clone(),
                    expiry_date: None,
                    estimated: false,
                });
            }
        });
        if let Some(action) = action {
            match state.dispatch(AppAction::Inventory(action)) {
                Ok(()) => {
                    state.expiry_text = state
                        .snapshot
                        .lots
                        .iter()
                        .find(|lot| lot.id == state.lot_id)
                        .and_then(|lot| lot.expiry_date.clone())
                        .unwrap_or_default();
                }
                Err(error) => state.message = error,
            }
        }
    } else {
        egui::Frame::group(ui.style())
            .fill(t(egui::Color32::WHITE))
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
    let mut use_soon: Vec<_> = use_soon.into_iter().cloned().collect();
    use_soon.sort_by(|a, b| a.expiry_date.cmp(&b.expiry_date));
    egui::CollapsingHeader::new(format!("Use first ({})", use_soon.len()))
        .default_open(true)
        .show(ui, |ui| {
            if use_soon.is_empty() {
                ui.label(
                    RichText::new("Nothing dated within the next 3 days.")
                        .size(13.0)
                        .color(t(MUTED)),
                );
            }
            let mut select = None;
            for lot in &use_soon {
                if ui
                    .link(format!(
                        "{}  ·  {}",
                        lot.name,
                        expiry_label(lot.expiry_date.as_deref(), lot.expiry_estimated)
                    ))
                    .on_hover_text("Select this item")
                    .clicked()
                {
                    select = Some(lot.id.clone());
                }
            }
            if let Some(id) = select {
                let _ = state.dispatch(AppAction::SelectLot(id));
            }
            ui.label(
                RichText::new("Dates are planning cues; check food condition before use.")
                    .size(12.0)
                    .color(t(MUTED)),
            );
        });
}

const CATEGORIES: &[(&str, &str)] = &[
    ("food.produce", "Food · produce"),
    ("food.dairy", "Food · dairy & eggs"),
    ("food.meat", "Food · meat & fish"),
    ("food.pantry", "Food · pantry"),
    ("food.beverage", "Food · drinks"),
    ("food.frozen", "Food · frozen"),
    ("food.other", "Food · other"),
    ("household.cleaning", "Household · cleaning"),
    ("household.paper", "Household · paper"),
    ("household.other", "Household · other"),
];

fn category_label(category: &str) -> String {
    CATEGORIES
        .iter()
        .find(|(value, _)| *value == category)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| {
            if category == "unclassified" {
                "Choose a category".into()
            } else {
                category.to_string()
            }
        })
}

fn category_picker(ui: &mut egui::Ui, id: impl std::hash::Hash, category: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(category_label(category))
        .width(190.0)
        .show_ui(ui, |ui| {
            for (value, label) in CATEGORIES {
                ui.selectable_value(category, (*value).to_string(), *label);
            }
        });
}

/// Parse "$4.29", "4.29", "-1.79" or "4" into cents.
fn parse_money(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed),
    };
    let cents = parse_budget_dollars(body)?;
    Some(if negative { -cents } else { cents })
}

fn money_field(ui: &mut egui::Ui, value: &mut i64) -> egui::Response {
    ui.add(
        egui::DragValue::new(value)
            .speed(1)
            .range(-100_000_000..=100_000_000)
            .custom_formatter(|n, _| money(n as i64))
            .custom_parser(|text| parse_money(text).map(|cents| cents as f64)),
    )
}

fn optional_money(ui: &mut egui::Ui, label: &str, value: &mut Option<i64>) {
    ui.horizontal(|ui| {
        ui.add_sized([118.0, 22.0], egui::Label::new(label));
        let mut known = value.is_some();
        if ui
            .checkbox(&mut known, "known")
            .on_hover_text("Untick when the receipt does not show this amount")
            .changed()
        {
            *value = if known { Some(0) } else { None };
        }
        if let Some(amount) = value.as_mut() {
            money_field(ui, amount);
        } else {
            ui.label(RichText::new("unknown").color(t(MUTED)));
        }
    });
}

fn text_field(ui: &mut egui::Ui, value: &mut Option<String>, width: f32, hint: &str) -> bool {
    let mut text = value.clone().unwrap_or_default();
    let changed = ui
        .add(
            egui::TextEdit::singleline(&mut text)
                .desired_width(width)
                .hint_text(hint),
        )
        .changed();
    if changed {
        *value = if text.trim().is_empty() {
            None
        } else {
            Some(text.trim().to_string())
        };
    }
    changed
}

fn draft_line(
    ui: &mut egui::Ui,
    index: usize,
    item: &mut DraftItem,
    remove: &mut Option<usize>,
    estimate: &mut Option<usize>,
) {
    let problems = crate::review_hints::line_problems(item);
    let border = if !problems.is_empty() {
        t(WARN)
    } else if item.reviewed {
        t(ACCENT)
    } else {
        t(BORDER)
    };
    egui::Frame::default()
        .fill(if item.reviewed { t(ACCENT_SOFT) } else { t(CARD) })
        .stroke(egui::Stroke::new(1.0_f32, border))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.checkbox(&mut item.reviewed, "")
                    .on_hover_text("Tick after comparing this line with the receipt");
                ui.label(RichText::new(format!("{}.", index + 1)).strong().color(t(MUTED)));
                ui.add(
                    egui::TextEdit::singleline(&mut item.name)
                        .desired_width((ui.available_width() - 210.0).max(140.0))
                        .hint_text("Item name")
                        .font(egui::TextStyle::Button),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("Remove")
                        .on_hover_text("Remove this line from the draft")
                        .clicked()
                    {
                        *remove = Some(index);
                    }
                    ui.label(
                        RichText::new(if item.reviewed {
                            "Checked"
                        } else {
                            "Not checked"
                        })
                        .size(12.0)
                        .color(if item.reviewed { t(ACCENT) } else { t(MUTED) }),
                    );
                });
            });
            if !item.raw.trim().is_empty() {
                ui.label(
                    RichText::new(format!("Read from receipt:  {}", item.raw.trim()))
                        .monospace()
                        .size(12.0)
                        .color(t(MUTED)),
                );
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Qty");
                text_field(ui, &mut item.quantity, 56.0, "1");
                ui.label("Unit");
                text_field(ui, &mut item.unit, 64.0, "each");
                ui.add_space(6.0);
                category_picker(ui, ("draft_category", index), &mut item.category);
                ui.add_space(6.0);
                ui.label("Price");
                let mut gross = item.gross_cents.unwrap_or(0);
                if item.gross_cents.is_some() {
                    if money_field(ui, &mut gross).changed() {
                        item.gross_cents = Some(gross);
                    }
                } else if ui
                    .button("Enter price")
                    .on_hover_text("The price could not be read; set it from the receipt")
                    .clicked()
                {
                    item.gross_cents = Some(0);
                }
                ui.label("Discount");
                money_field(ui, &mut item.discount_cents);
                item.net_cents = item
                    .gross_cents
                    .and_then(|gross| gross.checked_add(item.discount_cents));
                ui.label(
                    RichText::new(format!(
                        "= {}",
                        item.net_cents
                            .map(money)
                            .unwrap_or_else(|| "unknown".into())
                    ))
                    .strong(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Expiry");
                let before = item.expiry_date.clone();
                if text_field(ui, &mut item.expiry_date, 104.0, "YYYY-MM-DD")
                    && item.expiry_date != before
                {
                    // A typed date is the person's own, not an estimate.
                    item.expiry_estimated = false;
                }
                if item.category.starts_with("food.")
                    && ui
                        .small_button("Estimate")
                        .on_hover_text(
                            "Suggest a rough date from the purchase date. It is saved as an estimate.",
                        )
                        .clicked()
                {
                    *estimate = Some(index);
                }
                let (note, color) = match (&item.expiry_date, item.expiry_estimated) {
                    (Some(_), true) => (
                        "Estimated, not printed on the receipt. Edit to replace.",
                        t(WARN),
                    ),
                    (Some(_), false) => ("Entered by you", t(MUTED)),
                    (None, _) => ("Optional. Receipts do not print expiry dates.", t(MUTED)),
                };
                ui.label(RichText::new(note).size(12.0).color(color));
            });
            let mut notes: Vec<String> = problems.iter().map(|p| p.to_string()).collect();
            notes.extend(
                item.uncertainty
                    .iter()
                    .filter(|flag| flag.as_str() != "added during review")
                    .map(|flag| format!("scanner flagged: {flag}")),
            );
            if !notes.is_empty() {
                ui.label(RichText::new(notes.join("  ·  ")).size(12.0).color(t(WARN)));
            }
        });
}

fn run_estimate(state: &mut AppState, draft: &mut ReceiptDraft, index: Option<usize>) {
    state.draft = Some(draft.clone());
    let result = state.dispatch(AppAction::EstimateExpiry(index));
    if let Some(updated) = state.draft.take() {
        *draft = updated;
    }
    if let Err(error) = result {
        state.message = error;
    }
}

fn receipt(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(
        RichText::new("RECEIPT")
            .size(11.0)
            .strong()
            .color(t(ACCENT)),
    );
    ui.heading("Add a receipt");
    ui.label(
        RichText::new(
            "Scan a photo or paste the text. Nothing is saved until you have checked every line.",
        )
        .color(t(MUTED)),
    );
    ui.add_space(8.0);
    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let running = state.ocr_task.is_some();
            if ui
                .add_enabled(
                    !running,
                    egui::Button::new("Scan receipt photo…").fill(t(ACCENT_BUTTON)),
                )
                .on_hover_text(format!(
                    "PNG or JPEG up to 8 MB, read on this computer with {}.",
                    ocr::ENGINE_LABEL
                ))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Choose a receipt photo")
                    .add_filter("Receipt photo", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    let result = std::fs::metadata(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|metadata| {
                            if metadata.len() > ocr::MAX_IMAGE_BYTES as u64 {
                                Err("That photo is over the 8 MB limit".into())
                            } else {
                                std::fs::read(&path).map_err(|e| e.to_string())
                            }
                        })
                        .and_then(|bytes| state.dispatch(AppAction::StartOcr(bytes)));
                    if let Err(error) = result {
                        state.message = format!("{error}. You can paste the text instead.");
                    }
                }
            }
            if ui
                .add_enabled(
                    !running && !state.raw_text.trim().is_empty(),
                    egui::Button::new("Read pasted text"),
                )
                .clicked()
            {
                if let Err(error) = state.dispatch(AppAction::ParseReceipt) {
                    state.message = error;
                }
            }
            if ui
                .add_enabled(
                    running || state.draft.is_some() || !state.raw_text.is_empty(),
                    egui::Button::new(if running {
                        "Cancel scan"
                    } else {
                        "Discard draft"
                    }),
                )
                .on_hover_text("Stops any scan and clears the draft. Nothing is saved.")
                .clicked()
            {
                let _ = state.dispatch(AppAction::CancelReceipt);
            }
            if running {
                ui.add(egui::Spinner::new());
                ui.label(RichText::new("Reading the photo on this computer…").color(t(MUTED)));
            }
        });
        egui::CollapsingHeader::new("Receipt text")
            .id_salt("receipt_source_text")
            .default_open(state.draft.is_none())
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("receipt_source_scroll")
                    .max_height(170.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut state.raw_text)
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("Paste receipt text here, one line per item"),
                        );
                    });
            });
    });
    let Some(mut draft) = state.draft.take() else {
        ui.add_space(12.0);
        ui.label(
            RichText::new(
                "Photos and receipt text stay on this computer. Only the reviewed lines are saved.",
            )
            .size(13.0)
            .color(t(MUTED)),
        );
        return;
    };
    ui.add_space(10.0);
    card(ui, |ui| {
        ui.label(RichText::new("Receipt details").strong().size(17.0));
        egui::Grid::new("receipt_detail_grid")
            .num_columns(2)
            .spacing(egui::vec2(14.0, 8.0))
            .show(ui, |ui| {
                ui.label("Receipt ID");
                let previous_id = draft.id.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut draft.id).desired_width(180.0))
                    .changed()
                {
                    for item in &mut draft.items {
                        if let Some(suffix) = item.line_id.strip_prefix(&format!("{previous_id}-L"))
                        {
                            item.line_id = format!("{}-L{}", draft.id, suffix);
                        }
                    }
                    state.separate_ack = false;
                }
                ui.end_row();
                ui.label("Purchase date");
                let previous_date = draft.purchase_date.clone();
                ui.horizontal(|ui| {
                    text_field(ui, &mut draft.purchase_date, 120.0, "YYYY-MM-DD");
                    ui.label(
                        RichText::new("Used for spending. It never sets an expiry date.")
                            .size(12.0)
                            .color(t(MUTED)),
                    );
                });
                if draft.purchase_date != previous_date {
                    state.separate_ack = false;
                }
                ui.end_row();
            });
        optional_money(ui, "Printed subtotal", &mut draft.printed_subtotal_cents);
        optional_money(ui, "Printed tax", &mut draft.printed_tax_cents);
        optional_money(ui, "Printed total", &mut draft.printed_total_cents);
        if let Some((id, date)) = &state.source_match {
            ui.add_space(4.0);
            ui.colored_label(
                t(WARN),
                format!("This receipt matches one already saved as {id} on {date}. Confirming it again is recorded as a duplicate and changes nothing."),
            );
            ui.checkbox(
                &mut state.separate_ack,
                "It really is a separate purchase (I changed the receipt ID and date)",
            );
        }
    });
    ui.add_space(10.0);
    let total_lines = draft.items.len();
    let checked = draft.items.iter().filter(|item| item.reviewed).count();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Items").strong().size(17.0));
        ui.label(
            RichText::new(format!("{checked} of {total_lines} checked")).color(
                if checked == total_lines {
                    t(ACCENT)
                } else {
                    t(WARN)
                },
            ),
        );
    });
    ui.label(
        RichText::new("Compare each line with the receipt, correct it, then tick it.")
            .size(13.0)
            .color(t(MUTED)),
    );
    let mut remove = None;
    let mut estimate = None;
    for (index, item) in draft.items.iter_mut().enumerate() {
        draft_line(ui, index, item, &mut remove, &mut estimate);
        ui.add_space(4.0);
    }
    if let Some(index) = remove {
        draft.items.remove(index);
    }
    let mut estimate_all = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button("Add line").clicked() {
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
                expiry_estimated: false,
                uncertainty: vec!["added during review".into()],
                reviewed: false,
            });
        }
        estimate_all = ui
            .button("Estimate expiry for undated food")
            .on_hover_text("Fills rough, clearly labelled estimates only where no date is set")
            .clicked();
    });
    if estimate_all {
        run_estimate(state, &mut draft, None);
    } else if let Some(index) = estimate {
        run_estimate(state, &mut draft, Some(index));
    }
    ui.add_space(6.0);
    egui::CollapsingHeader::new(format!(
        "Refunds and adjustments ({})",
        draft.refunds.len() + draft.unallocated_adjustments.len()
    ))
    .id_salt("refunds_and_adjustments")
    .default_open(!draft.refunds.is_empty() || !draft.unallocated_adjustments.is_empty())
    .show(ui, |ui| {
        let mut remove_refund = None;
        for (index, refund) in draft.refunds.iter_mut().enumerate() {
            card(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("Refund {}", index + 1)).strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut refund.name)
                            .desired_width(180.0)
                            .hint_text("Item name"),
                    );
                    category_picker(ui, ("refund_category", index), &mut refund.category);
                    ui.label("Amount");
                    money_field(ui, &mut refund.amount_cents);
                    if ui.small_button("Remove").clicked() {
                        remove_refund = Some(index);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Qty");
                    text_field(ui, &mut refund.quantity, 56.0, "1");
                    ui.label("Unit");
                    text_field(ui, &mut refund.unit, 64.0, "each");
                    ui.label("Original line ID");
                    text_field(ui, &mut refund.original_line_id, 120.0, "optional");
                    ui.label("Take back from stock");
                    text_field(ui, &mut refund.inventory_quantity_to_reverse, 56.0, "0");
                });
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
        ui.label("Adjustments the scanner could not attach to a line:");
        let mut remove_adjustment = None;
        for (index, line) in draft.unallocated_adjustments.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(line);
                if ui.small_button("Remove").clicked() {
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
        optional_money(ui, "Reconciliation", &mut draft.unallocated_cents);
        ui.checkbox(
            &mut draft.accepted_unknowns,
            "Accept unknown item prices, balanced by the reconciliation amount",
        );
    });
    ui.add_space(10.0);
    let problems = crate::review_hints::receipt_problems(&draft);
    let valid = review(draft.clone()).is_ok();
    let unchecked = draft.items.iter().filter(|item| !item.reviewed).count();
    let ready = valid && unchecked == 0;
    let mut confirmed = false;
    card(ui, |ui| {
        ui.label(RichText::new("Confirm").strong().size(17.0));
        if ready {
            ui.colored_label(
                t(ACCENT),
                "Everything reconciles and every line is checked. Confirming adds the food to your kitchen and the spending to your history.",
            );
        } else {
            for problem in &problems {
                ui.colored_label(t(WARN), format!("• {problem}"));
            }
            if !valid && problems.is_empty() {
                ui.colored_label(t(WARN), "• Fix the lines marked in amber above.");
            }
            if unchecked > 0 {
                ui.colored_label(
                    t(WARN),
                    format!("• Tick each line after checking it ({unchecked} left)."),
                );
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new("Confirm receipt").fill(t(ACCENT_BUTTON)),
                )
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
                        ready && state.separate_ack,
                        egui::Button::new("Confirm as separate purchase"),
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
                .add_enabled(ready, egui::Button::new("Replace saved receipt with this"))
                .on_hover_text("Corrects a receipt you saved earlier with the same ID")
                .clicked()
            {
                state.draft = Some(draft.clone());
                match state.dispatch(AppAction::Commit(ReceiptCommit::Correct)) {
                    Ok(()) => confirmed = true,
                    Err(error) => state.message = error,
                }
            }
        });
    });
    if !confirmed {
        state.draft = Some(draft);
    }
}

fn meals(ui: &mut egui::Ui, state: &mut AppState) {
    ui.set_max_width(810.0);
    ui.heading("Meals from your kitchen");
    ui.label("Original recipes bundled locally. Suggestions use confirmed fridge, freezer, and pantry stock.");
    ui.horizontal(|ui| {
        ui.label("Plan for YYYY-MM-DD");
        ui.text_edit_singleline(&mut state.reference_date);
    });
    let (recipes, suggestions) = match recipe_suggestions(state) {
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
    state
        .batch_recipe_ids
        .retain(|id| recipes.iter().any(|recipe| &recipe.id == id));
    ui.label("Select recipe cards across these tabs to plan their combined grocery needs.");
    let batch: Vec<_> = recipes
        .iter()
        .filter(|recipe| state.batch_recipe_ids.contains(&recipe.id))
        .cloned()
        .collect();
    if ui
        .add_enabled(
            !batch.is_empty(),
            egui::Button::new(format!(
                "Add selected meals to shopping list ({})",
                batch.len()
            )),
        )
        .clicked()
    {
        if let Err(error) = state.dispatch(AppAction::AddMissingBatch(batch)) {
            state.message = error;
        }
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
            t(egui::Color32::from_rgb(231, 243, 239))
        } else {
            t(egui::Color32::WHITE)
        })
        .stroke(egui::Stroke::new(
            1.0_f32,
            t(egui::Color32::from_rgb(217, 226, 222)),
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
                    let mut planned = state.batch_recipe_ids.contains(&meal.id);
                    if ui.checkbox(&mut planned, "Select").changed() {
                        if planned {
                            state.batch_recipe_ids.insert(meal.id.clone());
                        } else {
                            state.batch_recipe_ids.remove(&meal.id);
                        }
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
                let missing: Vec<_> = meal
                    .ingredients
                    .iter()
                    .filter(|ingredient| ingredient.missing_quantity != "0")
                    .collect();
                let preview = missing
                    .iter()
                    .take(2)
                    .map(|ingredient| {
                        format!(
                            "{} {} {}",
                            ingredient.missing_quantity, ingredient.unit, ingredient.name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!(
                    "{}% of recipe quantities on hand; missing grocery prices unknown",
                    meal.stock_coverage_percent
                ));
                ui.label(if missing.len() > 2 {
                    format!("Get: {preview}, and {} more", missing.len() - 2)
                } else {
                    format!("Get: {preview}")
                });
            } else if !meal.soon_expiring.is_empty() {
                ui.colored_label(
                    t(egui::Color32::from_rgb(160, 96, 30)),
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
            t(egui::Color32::from_rgb(160, 96, 30)),
            format!("Use soon: {}", meal.soon_expiring.join(", ")),
        );
    }
    if meal.expiry_unknown {
        ui.label("Some selected stock has no recorded expiry date.");
    }
    ui.label(
        "Dates are planning cues, not a food safety check. Check food condition before cooking.",
    );
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
            t(egui::Color32::from_rgb(16, 112, 100)),
        );
        metric(
            ui,
            "food lines",
            money(state.snapshot.food_spending_cents),
            t(egui::Color32::from_rgb(45, 99, 135)),
        );
        metric(
            ui,
            "discarded",
            money(state.snapshot.discarded_cost_cents),
            t(egui::Color32::from_rgb(160, 96, 30)),
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
            .fill(t(egui::Color32::WHITE))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(
                1.0_f32,
                t(egui::Color32::from_rgb(215, 225, 223)),
            ))
            .show(&mut columns[0], |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.set_min_width(ui.available_width() - 16.0);
                ui.set_min_height(150.0);
                ui.label(
                    RichText::new("PURCHASE ALLOCATION")
                        .size(11.0)
                        .strong()
                        .color(t(egui::Color32::from_rgb(16, 112, 100))),
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
                    .color(t(egui::Color32::from_rgb(91, 106, 106))),
                );
            });
        egui::Frame::group(columns[1].style())
            .fill(t(egui::Color32::WHITE))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(
                1.0_f32,
                t(egui::Color32::from_rgb(215, 225, 223)),
            ))
            .show(&mut columns[1], |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.set_min_width(ui.available_width() - 16.0);
                ui.set_min_height(150.0);
                ui.label(
                    RichText::new("STOCK COST EVENTS")
                        .size(11.0)
                        .strong()
                        .color(t(egui::Color32::from_rgb(45, 99, 135))),
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
                        .color(t(egui::Color32::from_rgb(91, 106, 106))),
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
                    .color(t(egui::Color32::from_rgb(91, 106, 106))),
                );
            });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.heading("Weekly and monthly budgets");
        ui.label(
            RichText::new("Scroll for budget controls and full breakdown")
                .size(12.0)
                .color(t(egui::Color32::from_rgb(91, 106, 106))),
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
                .fill(t(egui::Color32::WHITE))
                .corner_radius(8)
                .inner_margin(egui::Margin::same(8))
                .stroke(egui::Stroke::new(1.0_f32, t(egui::Color32::from_rgb(215, 225, 223))))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.set_min_width(ui.available_width() - 16.0);
                    ui.set_min_height(170.0);
                    ui.label(
                        RichText::new(format!("{} PLAN", label.to_ascii_uppercase()))
                            .size(11.0)
                            .strong()
                            .color(t(egui::Color32::from_rgb(16, 112, 100))),
                    );
                    ui.label(RichText::new(format!("{label} budget")).text_style(egui::TextStyle::Heading).size(20.0));
                    let (budget_input, budget_text) = if matches!(period, Period::Week) {
                        (&mut state.weekly_budget, &mut state.weekly_budget_text)
                    } else {
                        (&mut state.monthly_budget, &mut state.monthly_budget_text)
                    };
                    ui.horizontal(|ui| {
                        ui.label("Budget (USD)");
                        ui.add_sized([112.0, 28.0], egui::TextEdit::singleline(budget_text).background_color(t(egui::Color32::from_rgb(241, 246, 244))));
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
                            ui.label(RichText::new(format!("{} to {} · {} receipt(s)", view.start_date, view.end_date, view.receipt_count)).size(13.0).color(t(egui::Color32::from_rgb(91, 106, 106))));
                            if let Some(budget) = view.budget_cents.filter(|budget| *budget > 0) {
                                let used = view.cash_flow_cents.max(0);
                                ui.add(egui::ProgressBar::new((used as f32 / budget as f32).clamp(0.0, 1.0)).desired_width(ui.available_width()).fill(t(egui::Color32::from_rgb(23, 125, 112))));
                                ui.label(format!("{} of {} cash flow against budget", money(view.cash_flow_cents), money(budget)));
                            } else if view.budget_cents.is_none() {
                                ui.label("Set a budget to see progress.");
                            }
                            dashboard_row(ui, "Remaining budget", view.remaining_budget_cents.map(money).unwrap_or_else(|| "not set".into()));
                            dashboard_row(ui, "Cash flow", money(view.cash_flow_cents));
                            egui::CollapsingHeader::new("Full period breakdown").id_salt(("period_breakdown", label))
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

/// Light / Dark / System segmented control. Applies at once and is saved.
fn theme_picker(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let mut chosen = None;
        for pref in crate::theme::ThemePref::ALL {
            let selected = state.theme_pref == pref;
            let text = RichText::new(pref.label());
            let button = egui::Button::new(if selected {
                text.color(egui::Color32::WHITE).strong()
            } else {
                text
            })
            .fill(if selected {
                crate::theme::accent_fill()
            } else {
                t(egui::Color32::from_rgb(232, 238, 236))
            })
            .corner_radius(6)
            .min_size(egui::vec2(76.0, 30.0));
            let hint = match pref {
                crate::theme::ThemePref::Light => "The original HungerStrike look (default)",
                crate::theme::ThemePref::Dark => "Dark panels and an evening kitchen",
                crate::theme::ThemePref::System => {
                    "Follow the operating system's light or dark setting"
                }
            };
            if ui.add(button).on_hover_text(hint).clicked() && !selected {
                chosen = Some(pref);
            }
        }
        if let Some(pref) = chosen {
            if let Err(error) = state.dispatch(AppAction::SetTheme(pref)) {
                state.message = error;
            }
        }
    });
}

fn data(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Local data");
    card(ui, |ui| {
        ui.label(RichText::new("Theme").strong().size(17.0));
        ui.label(
            RichText::new("Light is the default. System follows your operating system's light or dark setting. Saved on this computer.")
                .size(13.0)
                .color(t(MUTED)),
        );
        theme_picker(ui, state);
    });
    ui.add_space(12.0);
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
    ui.label("Receipt photos are read on this computer by the bundled OCR engine. Typing or pasting receipt text always remains available.");
}

#[cfg(test)]
mod tests {
    use super::{
        effective_selected_meal, expiry_before_planning_date, parse_budget_dollars, MealFilter,
        MealSuggestion,
    };

    #[test]
    fn default_and_explicit_meal_selection_use_visible_recipe() {
        let meal = |id: &str, available| MealSuggestion {
            id: id.into(),
            name: id.into(),
            servings: 1,
            steps: vec![],
            available,
            stock_coverage_percent: if available { 100 } else { 50 },
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
    fn kitchen_expiry_cue_uses_the_same_planning_date_boundary() {
        assert!(expiry_before_planning_date(
            Some("2026-10-03"),
            "2026-10-04"
        ));
        assert!(!expiry_before_planning_date(
            Some("2026-10-04"),
            "2026-10-04"
        ));
        assert!(!expiry_before_planning_date(None, "2026-10-04"));
        assert!(!expiry_before_planning_date(Some("unknown"), "2026-10-04"));
        assert!(!expiry_before_planning_date(Some("2026-10-03"), "bad date"));
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

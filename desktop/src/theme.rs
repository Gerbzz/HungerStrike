//! Light (default), Dark and System themes.
//!
//! The light theme is HungerStrike's original look and its colours are used
//! unchanged. The dark theme maps each of those colours to a tuned dark
//! counterpart in the same teal family. Text and status colours were chosen
//! for WCAG AA contrast on the dark surfaces they are drawn on (see the tests
//! at the bottom of this file).

use std::sync::atomic::{AtomicBool, Ordering};

use bevy_egui::egui::Color32;

/// The person's choice, stored with the other local settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemePref {
    #[default]
    Light,
    Dark,
    System,
}

impl ThemePref {
    pub const ALL: [ThemePref; 3] = [ThemePref::Light, ThemePref::Dark, ThemePref::System];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    /// Unknown or missing values fall back to the default light theme.
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("dark") => Self::Dark,
            Some("system") => Self::System,
            _ => Self::Light,
        }
    }
}

/// Decide whether to draw dark. `system_dark` is the operating system's
/// preference when the window system reports one; with System selected and
/// no report, the light theme is used.
pub fn resolve_dark(pref: ThemePref, system_dark: Option<bool>) -> bool {
    match pref {
        ThemePref::Light => false,
        ThemePref::Dark => true,
        ThemePref::System => system_dark.unwrap_or(false),
    }
}

static DARK: AtomicBool = AtomicBool::new(false);

/// Set the theme used by [`t`] for the frames that follow.
pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

// Dark tokens.
pub const DARK_BG: Color32 = rgb(22, 24, 26);
pub const DARK_PANEL: Color32 = rgb(27, 30, 33);
pub const DARK_SURFACE: Color32 = rgb(31, 34, 38);
pub const DARK_SURFACE_2: Color32 = rgb(40, 44, 49);
pub const DARK_FIELD: Color32 = rgb(19, 21, 24);
pub const DARK_BORDER: Color32 = rgb(58, 64, 70);
pub const DARK_TEXT: Color32 = rgb(236, 237, 234);
pub const DARK_MUTED: Color32 = rgb(163, 168, 164);
/// Teal for text, links and focus rings on dark surfaces.
pub const DARK_ACCENT: Color32 = rgb(94, 206, 190);
/// Teal fill behind white text (selected tab, selection).
pub const DARK_ACCENT_FILL: Color32 = rgb(20, 116, 108);
pub const DARK_ACCENT_SOFT: Color32 = rgb(25, 52, 51);
pub const DARK_ACCENT_BUTTON: Color32 = rgb(30, 86, 81);
pub const DARK_WARN: Color32 = rgb(242, 174, 96);
pub const DARK_WARN_FILL: Color32 = rgb(72, 50, 24);
pub const DARK_BLUE: Color32 = rgb(128, 186, 232);
pub const DARK_HOVER: Color32 = rgb(40, 64, 64);

/// Teal fill used behind white text in both themes.
pub fn accent_fill() -> Color32 {
    if is_dark() {
        DARK_ACCENT_FILL
    } else {
        rgb(17, 110, 107)
    }
}

/// Map one of the original light-theme colours to the active theme.
/// In the light theme every colour is returned unchanged.
pub fn t(light: Color32) -> Color32 {
    if !is_dark() {
        return light;
    }
    dark_for(light)
}

fn dark_for(light: Color32) -> Color32 {
    let [r, g, b, a] = light.to_array();
    if a < 255 {
        // The only translucent colour is the white scene caption backing.
        return Color32::from_rgba_unmultiplied(31, 34, 38, a);
    }
    match (r, g, b) {
        // Backgrounds and surfaces.
        (246, 248, 246) => DARK_BG,
        (248, 250, 249) | (235, 241, 239) => DARK_PANEL,
        (255, 255, 255) => DARK_SURFACE,
        (232, 238, 236) => DARK_SURFACE_2,
        (244, 247, 246) => DARK_FIELD,
        (216, 232, 228) => DARK_HOVER,
        // Borders.
        (217, 226, 222) | (215, 225, 223) | (218, 226, 225) => DARK_BORDER,
        // Text.
        (36, 52, 54) | (39, 58, 59) => DARK_TEXT,
        (84, 103, 105) | (82, 101, 102) | (91, 106, 106) => DARK_MUTED,
        // Teal accent family used as text.
        (17, 110, 107) | (24, 106, 101) | (16, 112, 100) | (23, 125, 112) | (17, 101, 93) => {
            DARK_ACCENT
        }
        // Teal used as a fill.
        (18, 114, 110) => DARK_ACCENT_FILL,
        (236, 246, 243) | (231, 243, 239) | (241, 246, 244) => DARK_ACCENT_SOFT,
        (196, 230, 222) => DARK_ACCENT_BUTTON,
        // Warnings, use-soon and estimate labels.
        (160, 96, 30) | (150, 86, 20) | (151, 77, 28) => DARK_WARN,
        (255, 236, 214) => DARK_WARN_FILL,
        // Informational blue.
        (45, 99, 135) => DARK_BLUE,
        _ => light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(colour: Color32) -> f64 {
        let channel = |value: u8| {
            let c = f64::from(value) / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let [r, g, b, _] = colour.to_array();
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn preference_resolution() {
        assert!(!resolve_dark(ThemePref::Light, Some(true)));
        assert!(resolve_dark(ThemePref::Dark, Some(false)));
        assert!(resolve_dark(ThemePref::System, Some(true)));
        assert!(!resolve_dark(ThemePref::System, Some(false)));
        assert!(!resolve_dark(ThemePref::System, None));
        assert_eq!(ThemePref::default(), ThemePref::Light);
    }

    #[test]
    fn stored_values_round_trip_and_unknown_is_light() {
        for pref in ThemePref::ALL {
            assert_eq!(ThemePref::parse(Some(pref.as_str())), pref);
        }
        assert_eq!(ThemePref::parse(None), ThemePref::Light);
        assert_eq!(ThemePref::parse(Some("sepia")), ThemePref::Light);
    }

    #[test]
    fn light_theme_is_unchanged() {
        set_dark(false);
        for colour in [
            rgb(36, 52, 54),
            rgb(17, 110, 107),
            Color32::WHITE,
            rgb(160, 96, 30),
        ] {
            assert_eq!(t(colour), colour);
        }
    }

    #[test]
    fn dark_text_and_status_colours_meet_wcag_aa() {
        for surface in [DARK_BG, DARK_PANEL, DARK_SURFACE, DARK_SURFACE_2] {
            for text in [DARK_TEXT, DARK_MUTED, DARK_ACCENT, DARK_WARN, DARK_BLUE] {
                assert!(
                    contrast(text, surface) >= 4.5,
                    "{text:?} on {surface:?}: {:.2}",
                    contrast(text, surface)
                );
            }
        }
        // Badges and filled buttons.
        assert!(contrast(DARK_WARN, DARK_WARN_FILL) >= 4.5);
        assert!(contrast(DARK_ACCENT, DARK_ACCENT_SOFT) >= 4.5);
        assert!(contrast(DARK_TEXT, DARK_ACCENT_BUTTON) >= 4.5);
        assert!(contrast(Color32::WHITE, DARK_ACCENT_FILL) >= 4.5);
        // Non-text UI (focus ring, selected tab edge) needs 3:1.
        assert!(contrast(DARK_ACCENT, DARK_BG) >= 3.0);
        assert!(contrast(DARK_ACCENT_FILL, DARK_BG) >= 3.0);
    }

    #[test]
    fn light_accent_and_warning_meet_wcag_aa_on_white() {
        assert!(contrast(rgb(17, 110, 107), Color32::WHITE) >= 4.5);
        assert!(contrast(rgb(160, 96, 30), Color32::WHITE) >= 4.5);
        assert!(contrast(rgb(84, 103, 105), Color32::WHITE) >= 4.5);
    }
}

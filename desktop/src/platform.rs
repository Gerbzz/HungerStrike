//! Per-operating-system locations and integrations.
//!
//! * Data: Windows keeps `%LOCALAPPDATA%\HungerStrike` (where earlier builds
//!   already store it). macOS uses `~/Library/Application Support/HungerStrike`
//!   and Linux `$XDG_DATA_HOME/HungerStrike` (normally
//!   `~/.local/share/HungerStrike`). `HUNGERSTRIKE_DATA_DIR` overrides all.
//! * Fonts: a system UI face is used when one is installed; egui's bundled
//!   fonts remain the fallback. No font bytes are redistributed.
//! * System theme on Linux: winit does not report the desktop colour scheme
//!   on X11, so it is read from the XDG desktop portal when available.

use std::path::PathBuf;

pub const APP_DIR: &str = "HungerStrike";
pub const DB_FILE: &str = "inventory.sqlite";

/// The parent directory that holds the `HungerStrike` data folder.
fn platform_data_parent() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        // Earlier builds used %LOCALAPPDATA% directly; keep that exact place.
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(dirs::data_local_dir)
    }
    #[cfg(not(windows))]
    {
        dirs::data_dir()
    }
}

/// Resolve the database path from an optional override and the platform
/// location, falling back to the working directory.
pub fn resolve_data_path(override_dir: Option<PathBuf>, platform: Option<PathBuf>) -> PathBuf {
    override_dir
        .or(platform)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
        .join(DB_FILE)
}

pub fn data_path() -> PathBuf {
    resolve_data_path(
        std::env::var_os("HUNGERSTRIKE_DATA_DIR").map(PathBuf::from),
        platform_data_parent(),
    )
}

/// Where builds before the cross-platform change put data on macOS and
/// Linux: `%LOCALAPPDATA%` is unset there, so they fell back to
/// `./HungerStrike/inventory.sqlite` relative to the working directory.
pub fn legacy_data_path() -> Option<PathBuf> {
    if cfg!(windows) || std::env::var_os("HUNGERSTRIKE_DATA_DIR").is_some() {
        return None;
    }
    Some(PathBuf::from(".").join(APP_DIR).join(DB_FILE))
}

// Shared with core regression tests: SQLite snapshot and atomic no-clobber publication.
pub use hungerstrike_core::migrate_legacy_data;

/// Candidate (regular, medium) system UI font files, most preferred first.
pub fn ui_font_candidates() -> Vec<(PathBuf, PathBuf)> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    {
        let root = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
            .join("Fonts");
        candidates.push((root.join("segoeui.ttf"), root.join("seguisb.ttf")));
    }
    #[cfg(target_os = "macos")]
    {
        let root = PathBuf::from("/System/Library/Fonts/Supplemental");
        candidates.push((root.join("Arial.ttf"), root.join("Arial Bold.ttf")));
        let root = PathBuf::from("/Library/Fonts");
        candidates.push((root.join("Arial.ttf"), root.join("Arial Bold.ttf")));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for root in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            let root = PathBuf::from(root);
            for (regular, medium) in [
                ("noto/NotoSans-Regular.ttf", "noto/NotoSans-Medium.ttf"),
                (
                    "truetype/noto/NotoSans-Regular.ttf",
                    "truetype/noto/NotoSans-Medium.ttf",
                ),
                (
                    "google-noto/NotoSans-Regular.ttf",
                    "google-noto/NotoSans-Medium.ttf",
                ),
                (
                    "truetype/dejavu/DejaVuSans.ttf",
                    "truetype/dejavu/DejaVuSans-Bold.ttf",
                ),
                ("dejavu/DejaVuSans.ttf", "dejavu/DejaVuSans-Bold.ttf"),
                (
                    "dejavu-sans-fonts/DejaVuSans.ttf",
                    "dejavu-sans-fonts/DejaVuSans-Bold.ttf",
                ),
            ] {
                candidates.push((root.join(regular), root.join(medium)));
            }
        }
    }
    candidates
}

/// Read the first installed (regular, medium) UI font pair.
pub fn load_ui_fonts() -> Option<(Vec<u8>, Vec<u8>)> {
    ui_font_candidates()
        .into_iter()
        .find_map(|(regular, medium)| {
            Some((std::fs::read(regular).ok()?, std::fs::read(medium).ok()?))
        })
}

/// The desktop colour scheme read from the XDG portal on Linux:
/// `None` until known, then `Some(true)` for dark.
#[cfg(target_os = "linux")]
mod portal {
    use std::sync::atomic::{AtomicU8, Ordering};

    const UNKNOWN: u8 = 0;
    const LIGHT: u8 = 1;
    const DARK: u8 = 2;
    static SCHEME: AtomicU8 = AtomicU8::new(UNKNOWN);

    /// Start a background thread that keeps [`system_dark`] up to date. It
    /// gives up quietly when there is no session bus or portal.
    pub fn start() {
        let _ = std::thread::Builder::new()
            .name("hungerstrike-portal-theme".into())
            .spawn(|| {
                use ashpd::desktop::settings::{ColorScheme, Settings};
                let Ok(settings) = pollster::block_on(Settings::new()) else {
                    return;
                };
                loop {
                    match pollster::block_on(settings.color_scheme()) {
                        Ok(scheme) => {
                            let value = match scheme {
                                ColorScheme::PreferDark => DARK,
                                ColorScheme::PreferLight => LIGHT,
                                ColorScheme::NoPreference => UNKNOWN,
                            };
                            SCHEME.store(value, Ordering::Relaxed);
                        }
                        Err(_) => return,
                    }
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            });
    }

    pub fn system_dark() -> Option<bool> {
        match SCHEME.load(Ordering::Relaxed) {
            DARK => Some(true),
            LIGHT => Some(false),
            _ => None,
        }
    }
}

/// Begin watching the desktop colour scheme where the window system does not
/// report it (Linux). A no-op elsewhere.
pub fn start_system_theme_watch() {
    #[cfg(target_os = "linux")]
    portal::start();
}

/// The desktop colour scheme from a platform source other than the window
/// system, when one is known.
pub fn portal_system_dark() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        portal::system_dark()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn override_wins_over_platform_location() {
        let path = resolve_data_path(Some("over".into()), Some("plat".into()));
        assert_eq!(path, Path::new("over").join(APP_DIR).join(DB_FILE));
        let path = resolve_data_path(None, Some("plat".into()));
        assert_eq!(path, Path::new("plat").join(APP_DIR).join(DB_FILE));
        let path = resolve_data_path(None, None);
        assert_eq!(path, Path::new(".").join(APP_DIR).join(DB_FILE));
    }

    #[test]
    fn platform_location_is_absolute_and_per_os() {
        let Some(parent) = platform_data_parent() else {
            return;
        };
        assert!(parent.is_absolute(), "{}", parent.display());
        #[cfg(target_os = "macos")]
        assert!(parent.ends_with("Library/Application Support"));
        #[cfg(windows)]
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            assert_eq!(parent, PathBuf::from(local));
        }
    }

    #[test]
    fn legacy_location_only_applies_off_windows() {
        if cfg!(windows) {
            assert!(legacy_data_path().is_none());
        }
    }
}

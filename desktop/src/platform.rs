use bevy::render::settings::{Backends, InstanceFlags, WgpuSettings};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    Desktop,
    Android,
}

#[cfg(not(target_os = "android"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopOs {
    Windows,
    Linux,
    Mac,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileFeatures {
    pub photo_ocr: bool,
    pub portable_backup: bool,
}

pub fn file_features_for(platform: PlatformKind) -> FileFeatures {
    match platform {
        PlatformKind::Desktop => FileFeatures {
            photo_ocr: true,
            portable_backup: true,
        },
        PlatformKind::Android => FileFeatures {
            photo_ocr: false,
            portable_backup: false,
        },
    }
}

pub fn renderer_settings_for(platform: PlatformKind) -> Option<WgpuSettings> {
    match platform {
        PlatformKind::Android => Some(WgpuSettings {
            backends: Some(Backends::VULKAN),
            instance_flags: InstanceFlags::default().difference(InstanceFlags::DEBUG),
            ..WgpuSettings::default()
        }),
        PlatformKind::Desktop => None,
    }
}

pub fn scene_picking_enabled_for(platform: PlatformKind) -> bool {
    platform == PlatformKind::Desktop
}

pub fn inventory_path_for(private_parent: Option<PathBuf>) -> Result<PathBuf, String> {
    private_parent
        .map(|parent| parent.join("HungerStrike").join("inventory.sqlite"))
        .ok_or_else(|| "no private local data directory is available".to_string())
}

pub fn current_file_features() -> FileFeatures {
    let platform = if cfg!(target_os = "android") {
        PlatformKind::Android
    } else {
        PlatformKind::Desktop
    };
    file_features_for(platform)
}

#[cfg(not(target_os = "android"))]
pub fn desktop_data_parent_for(
    os: DesktopOs,
    override_parent: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    override_parent.or_else(|| match os {
        DesktopOs::Windows => local_app_data,
        DesktopOs::Linux => xdg_data_home.or_else(|| home.map(|path| path.join(".local/share"))),
        DesktopOs::Mac => home.map(|path| path.join("Library/Application Support")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_preview_disables_unported_photo_ocr_and_file_backup() {
        let features = file_features_for(PlatformKind::Android);
        assert!(!features.photo_ocr);
        assert!(!features.portable_backup);
    }

    #[test]
    fn android_uses_vulkan_without_emulator_debug_extension() {
        let settings = renderer_settings_for(PlatformKind::Android).unwrap();
        assert_eq!(settings.backends, Some(Backends::VULKAN));
        assert_eq!(
            settings.instance_flags,
            InstanceFlags::default().difference(InstanceFlags::DEBUG)
        );
        assert!(renderer_settings_for(PlatformKind::Desktop).is_none());
    }

    #[test]
    fn android_uses_list_navigation_without_background_scene_picking() {
        assert!(!scene_picking_enabled_for(PlatformKind::Android));
        assert!(scene_picking_enabled_for(PlatformKind::Desktop));
    }

    #[test]
    fn desktop_keeps_local_photo_ocr_and_file_backup() {
        let features = file_features_for(PlatformKind::Desktop);
        assert!(features.photo_ocr);
        assert!(features.portable_backup);
    }

    #[test]
    fn private_inventory_path_never_falls_back_to_working_directory() {
        assert_eq!(
            inventory_path_for(Some(PathBuf::from("private-data"))).unwrap(),
            PathBuf::from("private-data")
                .join("HungerStrike")
                .join("inventory.sqlite")
        );
        assert!(inventory_path_for(None).is_err());
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn desktop_data_parent_uses_platform_private_directories() {
        let home = Some(PathBuf::from("home"));
        assert_eq!(
            desktop_data_parent_for(
                DesktopOs::Windows,
                None,
                Some(PathBuf::from("local")),
                None,
                home.clone()
            ),
            Some(PathBuf::from("local"))
        );
        assert_eq!(
            desktop_data_parent_for(
                DesktopOs::Linux,
                None,
                None,
                Some(PathBuf::from("xdg")),
                home.clone()
            ),
            Some(PathBuf::from("xdg"))
        );
        assert_eq!(
            desktop_data_parent_for(DesktopOs::Linux, None, None, None, home.clone()),
            Some(PathBuf::from("home").join(".local").join("share"))
        );
        assert_eq!(
            desktop_data_parent_for(DesktopOs::Mac, None, None, None, home),
            Some(
                PathBuf::from("home")
                    .join("Library")
                    .join("Application Support")
            )
        );
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn desktop_data_override_remains_available_for_isolated_windows_demo() {
        assert_eq!(
            desktop_data_parent_for(
                DesktopOs::Windows,
                Some(PathBuf::from("demo")),
                Some(PathBuf::from("local")),
                None,
                None
            ),
            Some(PathBuf::from("demo"))
        );
    }
}

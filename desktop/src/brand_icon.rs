//! Set only the app window identity; keep kitchen state and rendering unchanged.
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::Local;
use bevy::winit::WINIT_WINDOWS;

pub fn apply(_main_thread: NonSendMarker, mut applied: Local<bool>) {
    if *applied {
        return;
    }
    let Ok(image) = image::load_from_memory(include_bytes!(
        "../../assets/branding/HungerStrike-geometric.png"
    )) else {
        *applied = true;
        return;
    };
    let image = image.into_rgba8();
    let (width, height) = image.dimensions();
    let Ok(icon) = winit::window::Icon::from_rgba(image.into_raw(), width, height) else {
        *applied = true;
        return;
    };
    *applied = WINIT_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        if windows.windows.is_empty() {
            return false;
        }
        for window in windows.windows.values() {
            window.set_window_icon(Some(icon.clone()));
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::WindowExtWindows;
                window.set_taskbar_icon(Some(icon.clone()));
            }
        }
        true
    });
}

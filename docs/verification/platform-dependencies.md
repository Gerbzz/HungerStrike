# Local platform dependency inventory - 2026-10-10

## Windows preview on Neil's PC

- `dist/windows/HungerStrike.exe` is the Rust Bevy/egui desktop build. Core storage uses bundled SQLite through `rusqlite`; confirmed data goes to `%LOCALAPPDATA%\HungerStrike\inventory.sqlite` unless `HUNGERSTRIKE_DATA_DIR` selects a different parent. There is no app account, cloud service or runtime network integration.
- Manual receipt text entry needs no OCR engine. Photo OCR invokes the separately installed `C:\Program Files\Tesseract-OCR\tesseract.exe` and local `eng.traineddata`; neither is bundled into the executable. Both were present on this PC.
- The release launched and rendered on this Windows/NVIDIA Vulkan setup. Native mouse, keyboard and file-dialog use, alternate graphics drivers, displays and Windows installations still require hands-on QA.
- `dist/windows/Launch HungerStrike Demo.cmd` sets a separate `dist/windows/DemoProfile` data parent and loads synthetic fixture data. A launcher run created two synthetic receipts and eight lots there; the normal database remained absent. Do not treat fixture stock or invented prices/tax as real inventory or guidance.

## Android inventory and blockers

- Installed locally: Android Studio with JBR/OpenJDK 21; SDK platforms 33, 34 and 35; build tools 34.0.0, 35.0.1 and 36.0.0; NDK 26.3.11579264 and 27.0.12077973; ADB 35.0.2; several AVD configurations. Flutter is on `PATH` but is not used by this Rust preview.
- `rustup target list --installed` returned only `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc`. No Rust Android target is installed. `cargo-ndk` and `gradle` are not on `PATH`; there is no Gradle wrapper, Android manifest or Android app package in this repository.
- `adb devices -l` returned no attached device or running emulator. No APK was built and no camera, OCR, graphics, touch, file-picker or persistence flow was device tested. An Android app entry point and packaging/integration work are still required before those checks. No SDK terms were accepted and no tools were installed for this handoff.

## Other requested platforms

- Linux, macOS and iOS binaries were not built or tested from this Windows workspace; their Rust targets are not installed here. Apple builds remain deferred because Neil has no Mac. This document does not claim platform support beyond the verified Windows preview.

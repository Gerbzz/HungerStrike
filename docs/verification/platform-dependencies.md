# Local platform dependency inventory - 2026-10-10

## Windows preview on Neil's PC

- `dist/windows/HungerStrike.exe` is the Rust Bevy/egui desktop build. Core storage uses bundled SQLite through `rusqlite`; confirmed data goes to `%LOCALAPPDATA%\HungerStrike\inventory.sqlite` unless `HUNGERSTRIKE_DATA_DIR` selects a different parent. There is no app account, cloud service or runtime network integration.
- Manual receipt text entry needs no OCR engine. Photo OCR invokes the separately installed `C:\Program Files\Tesseract-OCR\tesseract.exe` and local `eng.traineddata`; neither is bundled into the executable. Both were present on this PC.
- The release launched and rendered on this Windows/NVIDIA Vulkan setup. Native mouse, keyboard and file-dialog use, alternate graphics drivers, displays and Windows installations still require hands-on QA.
- `dist/windows/Launch HungerStrike Demo.cmd` sets a separate `dist/windows/DemoProfile` data parent and loads synthetic fixture data. A launcher run created two synthetic receipts and eight lots there; the normal database remained absent. Do not treat fixture stock or invented prices/tax as real inventory or guidance.

## Android emulator checkpoint in isolated branch

- Installed locally: Android Studio with JBR/OpenJDK 21; SDK platforms 33, 34 and 35; build tools 34.0.0, 35.0.1 and 36.0.0; NDK 26.3.11579264 and 27.0.12077973; ADB 35.0.2; several AVD configurations. Flutter is on `PATH` but is not used by this Rust preview.
- This branch installed official `aarch64-linux-android` and `x86_64-linux-android` rust-std targets and `cargo-apk 0.10.0` from crates.io. No new SDK terms were accepted. The app has a Rust `#[bevy_main]` NativeActivity entry point and a reproducible `android/build-preview.ps1` package script. The compressed arm64 debug APK is `dist/android/HungerStrike-preview.apk`, signed with the existing local Android debug key. See [Android build and emulator record](android-build-attempt.md) for commands, checksum and limits.
- The Android manifest requests no Android permissions, disables cloud backup and cleartext traffic, and includes rules excluding app data from backup and device transfer. Android's own documentation cautions that some manufacturers may still transfer app files between devices despite `allowBackup=false`; this has not been device tested.
- The preexisting x86_64 API 36 AVD was run headless, read-only and without a snapshot. A native x86_64 build rendered with the host Vulkan GPU, accepted touch and keyboard input, confirmed a synthetic receipt and retained stock across app restart. The arm64 debug APK rendered through AVD translation; its touch/IME and full synthetic receipt confirmation also worked, with two private SQLite stock lots visible after restart. This is not a native arm64 hardware test. SwiftShader Vulkan crashed and GLES could not run the Bevy render pipeline. Screen reader, physical device graphics, performance, clipboard paste, camera/OCR, file picker and backup flows remain untested or unavailable. Android photo OCR and portable backup file actions are explicitly unavailable in this preview.

## Other requested platforms

- Linux, macOS and iOS binaries were not built or tested from this Windows workspace; their Rust targets are not installed here. Apple builds remain deferred because Neil has no Mac. This document does not claim platform support beyond the verified Windows preview.

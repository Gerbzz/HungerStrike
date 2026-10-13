# Android build and emulator checkpoint - 2026-10-10

This work is on feature/hungerstrike-android-preview in the isolated task-3/HungerStrike worktree, based on e114472. The original Windows checkout remains untouched. Its dist/windows/HungerStrike.exe SHA-256 is 3BDBE688E5DF1935285D41DA35809AA960BD7A4B71C337204CC2E3C9A4D9B3DD.

## Deliverables

- Arm64 local debug preview: dist/android/HungerStrike-preview.apk, 38,437,445 bytes, SHA-256 61F3F168F184B5CE379F00027E305A1B89C6D04780A22B10157D0DE0542C6509. ABI arm64-v8a, package com.hungerstrike.preview, minimum API 26, target API 33. This replaces the original 339,890,757-byte build-only APK locally and in the same Library item. It is still signed with the existing Android debug key and is not a store release.
- Native x86_64 emulator diagnostic: dist/android/HungerStrike-x86_64-compressed-diagnostic.apk, 29,643,330 bytes, SHA-256 E5D35EEF0B58E7B9F27B06890C54A6AF9C411A82A774F81886AA06189BAF2896. This is an optimized build signed with the same debug key, for existing x86_64 AVD testing only.
- The earlier 340 MB arm64 package is preserved under dist/android/diagnostics/HungerStrike-arm64-initial-debug.apk. An experimental optimized arm64 build is also under dist/android/HungerStrike-arm64-candidate.apk; it is excluded from the preview because it panicked during x86 emulator translation before rendering.

## Build and package

The app and core remain Rust. The Android entry point is a Rust Bevy NativeActivity; no first-party Java or Kotlin UI was introduced. Existing Android Studio JBR, SDK platform 33, build-tools 35.0.1, NDK 26.3.11579264 and the existing debug keystore were used. Official Rust aarch64-linux-android and x86_64-linux-android targets and cargo-apk 0.10.0 were installed for this branch. No new SDK terms, accounts, services, or network permissions were added.

From this worktree on Neil's Office:

    .\android\build-preview.ps1 -Target aarch64-linux-android -BuildProfile Debug
    .\android\build-preview.ps1 -Target x86_64-linux-android -BuildProfile Release -OutputName HungerStrike-x86_64-compressed-diagnostic.apk

The script strips the local native library, compresses it in a privacy-specific APK, aligns it, signs it with the existing debug key, and checks signature, alignment, compiled backup policy and declared permissions. The signed arm64 APK passed apksigner v2/v3 and zipalign checks. aapt reported no Android permissions. The manifest sets allowBackup=false and usesCleartextTraffic=false, with data extraction and full backup exclusion rules. The app has no receipt upload or runtime network path. Platform files remain in app-private SQLite at internal_data_path()/HungerStrike/inventory.sqlite. Android documents that some manufacturers can transfer app files despite allowBackup=false, so this cannot be proven solely from manifest inspection.

The Android Receipt view clearly says photo OCR is unavailable. It supports typed receipt text, review, and explicit confirmation; clipboard paste was not verified and is not promised. Android Data view clearly marks portable backup file import/export unavailable. Desktop keeps the local Tesseract and file-dialog paths; no receipt photo is sent over a network.

## Bounded emulator results

Only the preexisting Medium_Phone_API_36.0 AVD was used, headless with -read-only and -no-snapshot on port 5580. It is x86_64 with arm64 translation, not a native arm64 device. No physical device was connected or operated. The host-GPU configuration reported an NVIDIA RTX 4060 Vulkan adapter.

- The final arm64 debug APK installed and rendered on this translated AVD. Touch navigation, receipt text focus, Android soft keyboard and typed text worked. Tapping Parse text while the keyboard was visible parsed the text without activating a 3D storage zone behind the UI. A five-line synthetic $2.00 banana receipt was typed, parsed, reviewed and confirmed. After force-stop and restart, the translated run displayed two private SQLite stock lots worth $3.00 in total, including the prior $1.00 test lot. See arm64-final-full-receipt-entry.png, arm64-final-reviewed-receipt.png and arm64-final-kitchen-after-restart.png under dist/android/diagnostics.
- A native x86_64 build on the same AVD entered a five-line synthetic receipt, parsed it, reviewed the date and totals, confirmed a one-dollar banana purchase, and showed one stock lot worth $1.00 after force-stop and restart. Screenshots are in dist/android/diagnostics, including x86-full-receipt-entry.png, x86-parsed-receipt2.png, x86-purchase-confirmed.png, x86-kitchen-after-restart.png and arm64-fixed-after-parse-tap.png. This data was synthetic and stayed inside the AVD's private app directory.
- The x86_64 Vulkan build crashed under the AVD's SwiftShader software GPU. Disabling Android wgpu's debug-utils flag avoided one Vulkan debug-utils crash but did not solve a separate StagingBuffer write fault. A GLES trial avoided that fault but could not run Bevy's PBR/render pipeline because the GL backend lacks required SURFACE_VIEW_FORMATS and CUBE_TEXTURES_ARRAY support. The GLES trial was reverted. There is no verified software renderer fallback.
- An optimized arm64 experimental build panicked in jni 0.22.4 with "Expected an exception after ExceptionCheck" during x86_64 translation. The debug-profile arm64 APK above did not show that startup panic. Translation behavior does not establish native arm64 behavior.

The mobile UI now uses a full-width list, wrapped controls and phone-safe header/insets. Android disables background 3D picking because it previously intercepted receipt taps and switched to Kitchen. Desktop retains the original 3D picking and renderer defaults.

Final host gates passed: cargo fmt --all -- --check, cargo test --workspace (64 tests, 0 failures), cargo build -p hungerstrike-rust-desktop --bin hungerstrike-rust-desktop, and git diff --check. Native arm64 hardware, other GPUs, screen readers, performance, clipboard paste, photo OCR, and portable backup flows remain untested or unsupported. Linux, macOS and iOS were not built here; Apple builds still require a Mac.

## Install on an authorized arm64 device later

With an already authorized arm64 Android device connected by USB:

    adb devices -l
    adb install -r "dist\android\HungerStrike-preview.apk"

Open HungerStrike Preview from the launcher. Validate launch/rendering, touch and keyboard, explicit receipt confirmation, SQLite persistence across restart, privacy behavior and accessibility on the actual device before treating native arm64 support as proven. Do not use the experimental arm64 release candidate for this check.

# Architecture correction: first-party UI in Rust

The earlier Windows slice plan selected Tauri with TypeScript/HTML/CSS and Three.js. That produced a Rust core but did not satisfy Neil's established requirement that HungerStrike's first-party application and UI code be Rust. This ruling supersedes that stack choice.

Keep `hungerstrike-core` as the domain and local SQLite layer. Replace the Tauri shell and web frontend with a Rust-authored `hungerstrike-rust-desktop` executable using Bevy 0.18 for a perspective 3D kitchen and `bevy_egui` 0.39 for review, inventory, meals, spending, and backup controls. These versions match the plugin's published support table and the installed Rust 1.97 toolchain. The kitchen must have camera motion and 3D picking that change the inventory zone; the list must provide the same actions without needing the scene. The interface remains local. Egui/Bevy native dependencies are allowed; no first-party TypeScript, HTML, or CSS remains in the application path.

This is a Windows checkpoint, not proof of mobile support. Bevy/egui Android text input, camera/photo integration, performance, and accessibility must be built and tested on a device before claiming Android support. Existing local Tesseract may provide Windows OCR, but the release will not claim standalone OCR until engine/model packaging is resolved.

The Rust executable compiled, launched on the local GPU, and yielded an actual window capture. The web frontend and Tauri crate were removed from the active project; Git history retains the prior checkpoint. Native click-through is still unverified because the available computer-control surface does not expose native app windows. The separate synthetic preparation folder stays intact.

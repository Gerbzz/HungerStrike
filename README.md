<p align="center">
  <img src="assets/branding/HungerStrike.png" alt="Original HungerStrike dormouse and pear logo" width="128" height="128">
</p>

<h1 align="center">HungerStrike</h1>

<p align="center"><strong>A free, local-first kitchen assistant: from the receipt to the fridge to dinner.</strong></p>

<p align="center">
  <code>100% Rust</code> · <code>Windows source preview (alpha)</code> · <code>offline · no account · no paid API</code>
</p>

<p align="center">
  <a href="#what-works-today">What works</a> ·
  <a href="#how-receipt-scanning-works">Receipt scanning</a> ·
  <a href="#implemented-vs-planned">Implemented vs planned</a> ·
  <a href="#build-and-run">Build and run</a> ·
  <a href="#verification-status">Verification</a> ·
  <a href="#credits-and-licenses">Credits</a>
</p>

HungerStrike scans grocery receipts on your own computer, lets you check and correct every line, keeps track of what is in the fridge, freezer and pantry, puts the food that needs using first at the top, suggests recipes from what you have, builds a shopping list for what you are missing, and shows what you spend on food. Everything is stored locally in SQLite.

<p align="center">
  <img src="assets/screenshots/hungerstrike-selected-light-6da6e66-1440x900.png" alt="Kitchen in the default Light theme with synthetic stock, a selected lot, the expiry editor and the Use first list" width="420">
  <img src="assets/screenshots/hungerstrike-selected-dark-6da6e66-1440x900.png" alt="Kitchen in the Dark theme: dark panels and an evening kitchen scene" width="420">
</p>
<p align="center"><sub><b>Kitchen</b>: left Light (default), right Dark.</sub></p>

<p align="center">
  <img src="assets/screenshots/hungerstrike-receipt-photo-light-6da6e66-1440x900.png" alt="Receipt review in Light: draft produced by the bundled OCR from a synthetic labelled photo" width="420">
  <img src="assets/screenshots/hungerstrike-receipt-photo-dark-6da6e66-1440x900.png" alt="Receipt review in Dark" width="420">
</p>
<p align="center"><sub><b>Receipt review</b> of the synthetic photo: left Light, right Dark.</sub></p>

<p align="center">
  <img src="assets/screenshots/hungerstrike-spending-light-6da6e66-1440x900.png" alt="Spending and budgets in Light" width="420">
  <img src="assets/screenshots/hungerstrike-spending-dark-6da6e66-1440x900.png" alt="Spending and budgets in Dark" width="420">
</p>
<p align="center"><sub><b>Spending</b>: left Light, right Dark. App-rendered frames from source <code>6da6e66</code>, synthetic test data; the small red "widget ID" debug overlay was fixed afterwards. Provenance: <a href="docs/CAPTURE_2026-10-18_THEMES.md">theme captures</a>, <a href="docs/CAPTURE_2026-10-17.md">earlier captures</a> (OCR, reopen, database recovery, meals).</sub></p>

## What works today

| Area | What you can do |
| --- | --- |
| **Receipts** | Scan a PNG/JPEG receipt photo with the bundled offline OCR engine, or paste receipt text. The draft shows each line with the text the scanner read, and editable name, quantity, unit, category, price and discount. Plain-language hints say which field or total does not reconcile. **Nothing is saved until every line is ticked as checked** and the subtotal, tax and total reconcile. Duplicate receipts are recognised and never charged twice. Corrections and refunds are audited. |
| **Expiry** | Receipts do not print expiry dates, so a scanned line never gets one. You can type the package date, or ask for a rough **estimate**, which is saved as an estimate and labelled "(estimate)" everywhere until you replace it. |
| **Kitchen** | A 3D virtual kitchen (fridge, freezer, pantry) and a matching stock list. Select an item to consume, discard, move it between zones, or set, estimate or clear its expiry. Stock is sorted soonest-expiry first, and a **Use first** list links to anything dated within three days. |
| **Meals** | Ten bundled original recipes. *Cook now* uses stock you have; *Use soon* prefers recipes that use dated stock; *Plan next* ranks recipes by how few ingredients are missing. Cooking previews the exact lots before stock changes. Missing ingredients for one or several selected meals can be added to a combined local shopping list. |
| **Spending** | Cash paid, food vs household allocations, refunds, consumed and discarded food cost, with weekly and monthly budgets. |
| **Data** | Export a JSON backup, preview an import, replace or reconcile, and undo the last replacement. If the local database is ever unreadable it is renamed aside (never deleted) and a fresh one is started, with a message saying where the old file is. |
| **Theme** | Light (the original look, default), Dark, or System (follows the Windows app theme). Set it on the Data tab; it applies immediately and is saved with your local data. Dark keeps the teal accent family with colours checked for WCAG AA contrast, and switches the 3D kitchen to an evening look. |

## How receipt scanning works

1. **Scan receipt photo…** opens a file picker. The image is checked (PNG/JPEG, at most 8 MB and 40 megapixels) and decoded on a worker thread; a corrupt or truncated file is rejected before any draft exists.
2. The [ocrs](https://github.com/robertknight/ocrs) engine finds and reads text lines using two small models that are compiled into the executable. Lines are rebuilt into receipt rows (item name on the left joined with the price on the right, with slight tilt corrected) and handed to the receipt parser.
3. You review the draft. **Cancel scan** stops a running job; **Discard draft** clears it. Photo bytes and OCR text are never written to the database or backups.

OCR quality depends on the photo. The bundled engine reads the repository's labelled synthetic receipts (see `tests/images/`), but real receipts with faded thermal paper, unusual fonts or heavy creasing will need more correction, which is why the line-by-line review is mandatory. No real receipt has been tested.

## Implemented vs planned

| Feature | Status |
| --- | --- |
| Local receipt OCR (bundled, offline) | Implemented; tested in CI on synthetic images only |
| Paste/typed receipt entry | Implemented |
| Mandatory per-line review before saving | Implemented |
| Labelled, opt-in expiry estimates; editable expiry on stock | Implemented |
| Inventory by zone, consume/discard/move | Implemented |
| Use-first prioritisation | Implemented (3-day window, sorted lists) |
| Recipe suggestions and combined shopping list | Implemented (bundled recipes only) |
| Spending and budgets | Implemented |
| Backup, restore, undo; corrupt-database recovery | Implemented |
| Virtual 3D kitchen | Implemented as an illustrative kitchen; it does not reconstruct a real kitchen from photos |
| Camera capture | Planned (file import only today) |
| Android | Planned. A stale [android-preview branch](https://github.com/Gerbzz/HungerStrike/tree/android-preview) has a Bevy NativeActivity build that was run on an emulator in October; it is not merged and no Android support is claimed. The bundled OCR is pure Rust, so it does not block an Android port the way an external Tesseract did. |
| iOS, macOS, Linux | Not built or tested |
| Installer / signed release | Not available |

## Build and run

You need a stable Rust toolchain on Windows. From the repository root:

```powershell
cargo build --release -p hungerstrike-rust-desktop
.\target\release\hungerstrike-rust-desktop.exe
cargo test --workspace
```

The OCR crates are optimised even in debug builds (see `[profile.dev.package.*]` in `Cargo.toml`) so the tests run in seconds. Data lives at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`; set `HUNGERSTRIKE_DATA_DIR` to use a separate profile. See [launch notes](docs/launch.md) and the [workflow walkthrough](docs/workflow.md).

For isolated, clearly labelled synthetic captures, set `HUNGERSTRIKE_CAPTURE_DEMO` to one of `kitchen`, `selected`, `receipt`, `receipt-photo`, `plan-next`, `spending`, `about` or `launch`, plus `HUNGERSTRIKE_CAPTURE_PATH` (optionally `HUNGERSTRIKE_CAPTURE_SIZE` and `HUNGERSTRIKE_CAPTURE_DELAY`), and point `HUNGERSTRIKE_DATA_DIR` at a throwaway folder. These routes show a **SYNTHETIC TEST DATA** label.

## Verification status

See [verification.md](verification.md) for the dated record. In short, for this branch:

- **Tests:** `cargo fmt --all --check` and `cargo test --workspace` pass locally (113 tests: 78 core, 35 desktop, at `6da6e66`); hosted Windows CI passed at an earlier commit. They cover theme resolution, persistence and AA contrast, receipts, ledger, budgets, backups, recipes, database recovery, expiry estimates, bundled OCR on synthetic images, corrupt images, cancellation, timeout, per-line review gating, duplicate photo imports and app actions.
- **App-rendered frames:** the built app produced the [frames listed here](docs/CAPTURE_2026-10-17.md), including a real OCR draft from a synthetic photo, reopening a saved profile, and recovery from a corrupted database file.
- **Clippy:** `cargo clippy --workspace --all-targets -- -D warnings` passes.
- **Not verified:** native mouse/keyboard click-through, the file dialog and 3D picking on this build (computer-use access to the app window was denied); real receipts; other PCs, GPUs and DPI settings; screen readers.

## Data and privacy

No account, telemetry, cloud sync or runtime network call. Receipt photos and OCR text stay in memory until you confirm or discard; only reviewed fields and sanitized audit metadata are stored. All fixtures in this repository are **synthetic test data** with invented prices.

## Credits and licenses

- Receipt OCR: [ocrs](https://github.com/robertknight/ocrs) and [rten](https://github.com/robertknight/rten) by Robert Knight (MIT OR Apache-2.0).
- OCR models: the published ocrs text detection and recognition models (CC BY-SA 4.0), redistributed unmodified; sources and hashes in [assets/ocr/NOTICE.md](assets/ocr/NOTICE.md).
- 3D food models: [Poly Haven](https://polyhaven.com) (CC0); see [assets/models/polyhaven/PROVENANCE.md](assets/models/polyhaven/PROVENANCE.md).
- Built with [Bevy](https://bevyengine.org), [egui](https://github.com/emilk/egui) and SQLite via rusqlite.
- HungerStrike dormouse artwork is original ([provenance](assets/branding/provenance.json)). PWDS company marks appear only on the launch screen and in About.
- This README's layout loosely follows the structure of the [PhotoCraft README](https://github.com/storytold/photocraft) (tagline, jump links, feature table, status, credits). No text or assets were copied.

No license has been chosen yet for HungerStrike's own source code. Bundled third-party files keep their own licenses as listed above.

Part of the PWDS Strike suite by [ProWebDevSolutions](https://prowebdevsolutions.com). See the other [Strike repositories](https://github.com/Gerbzz?tab=repositories&q=Strike).

<p align="center">
  <img src="assets/branding/HungerStrike.png" alt="Original HungerStrike dormouse and pear logo" width="160" height="160">
</p>

<h1 align="center">HungerStrike</h1>

<p align="center">
  <a href="https://prowebdevsolutions.com"><picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/branding/PWDS-logo-orange.svg">
    <img src="assets/branding/PWDS-logo.png" alt="PWDS company logo" width="120">
  </picture></a>
</p>

<p align="center"><strong>Know what is in the kitchen, from the receipt onward.</strong></p>

HungerStrike brings reviewed purchases, local stock, meal ideas, and spending into one Windows desktop workspace. Every receipt stays a draft until you confirm what it says.

## Light interface development preview

<p align="center"><img src="assets/screenshots/hungerstrike-kitchen-light-34405b7-1600x1100.png" alt="HungerStrike light Kitchen development preview with synthetic confirmed stock" width="900"></p>

<p align="center"><img src="assets/screenshots/hungerstrike-meals-light-34405b7-1600x900.png" alt="HungerStrike light Cook now Meals development preview with synthetic stock" width="900"></p>

*These unedited app-rendered frames show synthetic data from tested [draft light-interface PR #1](https://github.com/Gerbzz/HungerStrike/pull/1), source `34405b7`. They are development previews, not captures of an executable built from this default branch. See [image hashes, capture method, and limits](docs/LIGHT_PREVIEW_PROVENANCE.md). The older dark image remains [archived with its provenance](docs/SCREENSHOT_PROVENANCE.md).*

<p align="center"><a href="docs/workflow.md">Walk through the app</a> · <a href="docs/launch.md">Build and launch</a> · <a href="verification.md">Verification and limits</a> · <a href="assets/branding">Original artwork</a></p>

## From a receipt to a useful kitchen

### Review before stock changes

Paste receipt text or open a local photo for optional offline OCR. Check the parsed lines, quantities, prices, categories, date, and totals before confirming. Duplicate detection, explicit separate-purchase acknowledgment, and correction guards help preserve the spending and inventory history.

### See what is available

Browse the fridge, freezer, and pantry in the 3D scene or the matching stock list. Keep quantities and known costs tied to confirmed purchases, while unknown expiry dates and prices stay visibly unknown. Linked refunds can reverse stock from the original lot.

### Plan meals from confirmed stock

The draft light-interface preview includes six original bundled recipes. Cook now uses sufficient confirmed stock, Use soon highlights user-entered dates as planning cues, and Plan next shows missing quantities that can be saved to a local shopping list. Cook requires an explicit action to update stock. Missing grocery prices and unknown stock costs stay unknown. These draft features are not yet in the default-branch app source.

### Keep spending and stock use distinct

The spending view separates receipt cash flow, food and household allocations, refunds, consumed food, and discarded food. Weekly and monthly budgets use those reviewed figures without counting stock use as another purchase.

## Run the Windows source preview

The commands below build the default-branch source. The light development preview above is on [draft PR #1](https://github.com/Gerbzz/HungerStrike/pull/1).

You need a local Rust toolchain and Cargo. From the repository root:

```powershell
cargo build --release -p hungerstrike-rust-desktop
.\target\release\hungerstrike-rust-desktop.exe
```

There is no installer or binary in this repository. Paste or type receipts without any OCR dependency. Photo OCR optionally uses a separately installed local English Tesseract at `C:\Program Files\Tesseract-OCR\tesseract.exe`; the engine and language model are not bundled. See [launch notes](docs/launch.md) for the input limits and local data path.

To run the repository tests:

```powershell
cargo test --workspace
```

## Status and data

This default branch is a **Windows alpha source preview**. The [2026-10-10 verification record](verification.md) reports 57 passing workspace tests for its earlier source. The separate [draft light-interface PR #1](https://github.com/Gerbzz/HungerStrike/pull/1) passed 73 offline workspace tests (55 core and 18 desktop); its light Kitchen and Meals images above were captured at the earlier tested draft source `34405b7`. Native mouse and keyboard click-through, file dialogs, accessibility, other devices, and broader receipt/photo acceptance remain open. The separate [Android preview source branch](https://github.com/Gerbzz/HungerStrike/tree/android-preview) has no APK release or physical-device validation.

Confirmed inventory lives in local SQLite at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`. The **Data** tab can export a JSON backup and preview an import. The app has no account, cloud sync, telemetry, paid API, or runtime network call in this source checkpoint. Raw receipt lines, photo bytes, and OCR transcripts are not stored in SQLite or backups; reviewed fields and sanitized audit metadata are. [Workflow details](docs/workflow.md) explain confirmation, refunds, corrections, and backup choices.

The included fixtures use **synthetic test data**. Receipt parsing requires human review; it is not a universal receipt interpreter. No first-party source license has been selected for this public checkpoint.

Explore the other [PWDS Strike repositories](https://github.com/Gerbzz?tab=repositories&q=Strike) and the [PWDS Strike Launcher](https://github.com/Gerbzz/PWDS-Strike-Launcher).

## PWDS brand artwork

The company mark in this masthead is the existing [official PWDS logo](https://github.com/Gerbzz/PWDS-Website/blob/main/public/prowebdevsolutions-logo.png), with the [official orange display asset](https://prowebdevsolutions.com/images/prowebdevsolutions-logo-orange.svg) for dark backgrounds. Any app logo above it remains the original Strike suite artwork.

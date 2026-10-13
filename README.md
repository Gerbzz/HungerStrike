# HungerStrike

**by PWDS**

**Know what is in the kitchen.**

HungerStrike connects receipt review with a local kitchen inventory. Parse receipt text, inspect each line before confirming stock changes, and use the inventory to keep track of what is on hand.

### Highlights

- Review receipt items, prices, and totals before committing.
- Track purchases and linked refunds in local inventory.
- Keep kitchen data in a local SQLite database.

## Current status

Windows source preview. The application and receipt workflow are present, but no binary or installer is included. Native pointer, keyboard, accessibility, and broader device acceptance remain open; the Android preview is a separate unverified source branch.

## Try the workflow

1. On **Receipt**, paste `core/tests/fixtures/receipts/R001-purchase.txt`, then **Parse text**. Review the date, each item, prices, units, discounts, category, subtotal, tax, and total. Inventory changes only after an explicit confirmation action. **Cancel and clear** discards temporary text. Purchase date never becomes expiry.
2. Parse `R002-refund.txt`. The milk refund can be linked to original lot `R001-L02` with one gallon of stock reversal, or left unlinked. Refunds affect spending either way; unlinked amounts appear separately in Spending.
3. Select the 3D fridge, freezer, or pantry to filter stock; the list offers the same zones and inventory actions by keyboard. Arrow keys and scene buttons orbit/zoom. Meals use confirmed stock; unknown costs and expiry dates are marked.
4. On **Spending**, inspect weekly/monthly budgets, discounts, refunds, reconciliation, and discarded food cost. Exact repeat imports do not charge again. A truly separate purchase from matching source text requires a unique reviewed ID and the deliberate **Confirm separate purchase** action, which is audited. Receipt corrections use **Correct existing receipt**. Name, date, and expiry corrections can follow stock use. Cost or quantity corrections after stock use, and corrections after linked refunds, stay blocked to protect historical allocations.
5. On **Data**, save a local JSON backup, preview an import, reconcile or replace explicitly, and use the local restore point if needed.

**Open photo · offline OCR** accepts PNG/JPEG under 8 MB and 40 megapixels through a separately installed local English Tesseract at `C:\Program Files\Tesseract-OCR\tesseract.exe`. The engine/model is not bundled. A stalled engine is stopped after 15 seconds; **Cancel and clear** stops an active job. OCR text stays a temporary draft and is cleared on confirm or cancel. Manual entry works without the engine. Backups and SQLite omit raw receipt lines, photo bytes, and OCR transcripts; reviewed fields and sanitized audit metadata remain.

The receipt and image fixtures are **SYNTHETIC TEST DATA**. Prices and tax are invented fixture values, not current store prices or tax guidance. General receipts need human review; the parser recognizes common structures but is not a universal receipt interpreter. Unknown costs require explicit reconciliation, and unknown quantities require correction before confirmation.

## Build and verify

With a local Rust toolchain:

```powershell
cargo test --workspace
cargo build --release -p hungerstrike-rust-desktop
```

[Verification](verification.md) records tested behavior and limits. The included fixtures are synthetic. Native pointer, keyboard, accessibility, and broader device acceptance remain open. No first-party source license has been selected for this public checkpoint.

The separate [Android preview source branch](https://github.com/Gerbzz/HungerStrike/tree/android-preview) contains a sanitized, source-only mobile snapshot. It has no APK release and is not yet validated on a physical arm64 device.

## Local data

The Windows app stores inventory at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`; set `HUNGERSTRIKE_DATA_DIR` to choose a separate parent directory for a test run. This source checkpoint has no account, cloud sync, telemetry, paid API, or runtime network call. See [launch notes](docs/launch.md) after building locally.

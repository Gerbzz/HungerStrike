# HungerStrike · Rust desktop checkpoint

HungerStrike is a local kitchen inventory prototype. Its first-party application, interface, 3D scene, receipt logic, and persistence are Rust. The Windows executable is `target/release/hungerstrike-rust-desktop.exe`; run it directly. It stores data at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`. Set `HUNGERSTRIKE_DATA_DIR` to use a separate parent directory for a test run. The app has no account, cloud service, sync, telemetry, paid API, or runtime network call.

## Try the workflow

1. On **Receipt**, paste `core/tests/fixtures/receipts/R001-purchase.txt`, then **Parse text**. Review the date, each item, prices, units, discounts, category, subtotal, tax, and total. **Confirm new receipt** is the only path that writes inventory. **Cancel and clear** discards temporary text. Purchase date never becomes expiry.
2. Parse `R002-refund.txt`. The milk refund can be linked to original lot `R001-L02` with one gallon of stock reversal, or left unlinked. Refunds affect spending either way; unlinked amounts appear separately in Spending.
3. Select the 3D fridge, freezer, or pantry to filter stock; the list offers the same zones and inventory actions by keyboard. Arrow keys and scene buttons orbit/zoom. Meals use confirmed stock; unknown costs and expiry dates are marked.
4. On **Spending**, inspect weekly/monthly budgets, discounts, refunds, reconciliation, and discarded food cost. Receipt corrections require **Correct existing receipt** and create an audit event; exact repeat imports do not charge again. Name, date, and expiry corrections can follow stock use. Cost or quantity corrections after stock use, and corrections after linked refunds, stay blocked to protect historical allocations.
5. On **Data**, save a local JSON backup, preview an import, reconcile or replace explicitly, and use the local restore point if needed.

**Open photo · offline OCR** accepts PNG/JPEG under 8 MB through a separately installed local English Tesseract at `C:\Program Files\Tesseract-OCR\tesseract.exe`. The engine/model is not bundled. OCR text stays a temporary draft and is cleared on confirm or cancel. Manual entry works without the engine. Backups and SQLite omit raw receipt lines, photo bytes, and OCR transcripts; reviewed fields and sanitized audit metadata remain.

The receipt and image fixtures are **SYNTHETIC TEST DATA**. Prices and tax are invented fixture values, not current store prices or tax guidance. General receipts need human review; the parser recognizes common structures but is not a universal receipt interpreter. Unknown costs require explicit reconciliation, and unknown quantities require correction before confirmation.

## Build and verify

With a local Rust toolchain:

```powershell
cargo test --workspace
cargo build --release -p hungerstrike-rust-desktop
```

[Verification](verification.md) records tested behavior and limits. The draft scope and accounting fixtures remain in the sibling `HungerStrike-preparation` folder.

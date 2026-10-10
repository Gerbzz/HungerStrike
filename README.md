# HungerStrike local Windows prototype

HungerStrike is a Rust-first, local kitchen inventory prototype. The Windows release executable is at `target/release/hungerstrike-desktop.exe`. It uses bundled UI assets and a SQLite database at `%LOCALAPPDATA%\local.hungerstrike.desktop\hungerstrike.sqlite`. No account, cloud service, sync, telemetry, or paid API is used by the app.

## Try the local workflow

1. Run the executable. Paste text from `core/tests/fixtures/receipts/R001-purchase.txt` and choose **Review receipt**. Check item quantities, cents, discounts, category, and totals, then explicitly confirm. Receipt date is not an expiry date.
2. Review `R002-refund.txt`. To remove the refunded milk from stock, select the original milk lot and enter `1` gallon for stock reversal. An unlinked refund still reduces food spending, but leaves stock unchanged and shows a refund reconciliation adjustment.
3. Click fridge, freezer, or pantry in the fixed kitchen; use the searchable inventory list for keyboard access, location changes, consumption, and discards. Meal ideas use only confirmed stock. Spending has weekly/monthly date views, a configurable week start, budgets, and audited category edits.
4. Use **Backup and restore** to save a JSON backup to a new local file, preview an import, and explicitly reconcile or replace. Replace creates a local undo point. Undo refuses to discard changes made after replacement.

For photos, **Scan a photo locally** sends PNG/JPEG bytes to an already installed English Tesseract at `C:\Program Files\Tesseract-OCR\tesseract.exe`. OCR creates only a draft. If the engine is absent or the image cannot be read, the photo preview and manual text entry remain available. No OCR binary or model is bundled.

The bundled synthetic receipts, generated test images, recipes, and all example prices/taxes are **SYNTHETIC TEST DATA**. They do not describe real purchases, current store prices, or tax guidance. The parser currently recognizes this limited fixture vocabulary; real receipts will require careful manual correction, and unresolved totals or discount lines cannot be confirmed.

## Build and test locally

From this directory, with Rust and Node already installed:

```powershell
npm ci
cargo test --workspace
npm test -- --run
npm run tauri dev
npm run tauri build
```

`tauri dev` uses a localhost Vite server. `tauri build` creates the local Windows executable without an installer or publication. Backups are plain JSON containing confirmed normalized purchase records and should be stored as private local files. Raw receipt lines, OCR transcripts, and images are excluded from portable backups.

See [verification.md](verification.md) for exact test results and platform limits. The draft scope and full synthetic accounting fixtures remain in the sibling `HungerStrike-preparation` folder.

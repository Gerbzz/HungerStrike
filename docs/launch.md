# HungerStrike Windows launch

Build from this repository with `cargo build --release -p hungerstrike-rust-desktop`, then run `target/release/hungerstrike-rust-desktop.exe`. No installer, account, cloud service or paid API is involved.

On Windows the app stores confirmed data at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`. The **Data** tab can export a JSON backup and preview a restore before applying it. Set `HUNGERSTRIKE_DATA_DIR` to a different parent directory only for an isolated test run.

Receipt text can be pasted directly. Photo OCR is optional and local: it currently looks for an existing English Tesseract installation at `C:\Program Files\Tesseract-OCR\tesseract.exe`. The engine and language data are not bundled. PNG/JPEG input is limited to 8 MB and 40 megapixels; a stalled engine is stopped after 15 seconds, and **Cancel and clear** stops an active job. If OCR is unavailable or fails, paste or type the receipt. Every parsed line, total, category and date needs human review before confirmation.

Repeated source text is held as a duplicate. If it truly represents another purchase, give it a unique receipt ID, check the date and line IDs, then use the explicit **Confirm separate purchase** acknowledgment. That choice is audited. Price or quantity corrections after stock use remain blocked to preserve historical cost events.

This is a Windows preview, not a validated cross-platform release. [Verification](../verification.md) records the latest tests, application-rendered captures, and remaining native and mobile gates.

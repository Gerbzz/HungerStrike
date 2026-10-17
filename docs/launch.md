# HungerStrike Windows launch

Build from this repository with `cargo build --release -p hungerstrike-rust-desktop`, then run `target/release/hungerstrike-rust-desktop.exe`. No installer, account, cloud service or paid API is involved.

On Windows the app stores confirmed data at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`. The **Data** tab can export a JSON backup and preview a restore before applying it. Set `HUNGERSTRIKE_DATA_DIR` to a different parent directory only for an isolated test run.

Receipt text can be pasted directly. Photo OCR is built in and local: the ocrs engine and its two models (see assets/ocr/NOTICE.md) are compiled into the executable, so nothing needs installing. PNG/JPEG input is limited to 8 MB and 40 megapixels; corrupt files are rejected before a draft exists, a job is stopped after 60 seconds, and **Cancel scan** stops an active job. If OCR fails, paste or type the receipt. Every parsed line, total, category and date needs human review before confirmation.

Repeated source text is held as a duplicate. If it truly represents another purchase, give it a unique receipt ID, check the date and line IDs, then use the explicit **Confirm separate purchase** acknowledgment. That choice is audited. Price or quantity corrections after stock use remain blocked to preserve historical cost events.

If the local database cannot be read, it is renamed next to the original as `inventory.unreadable-<timestamp>.sqlite` (never deleted) and a fresh database is started; the app says so on launch.

This is a Windows preview, not a validated cross-platform release. [Verification](../verification.md) records the latest tests, application-rendered captures, and remaining native and mobile gates.

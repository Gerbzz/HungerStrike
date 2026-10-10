# HungerStrike Windows preview verification · 2026-10-10

## Verified locally on Neil's Windows PC

| Check | Result |
| --- | --- |
| Rust tests | `cargo test --workspace`: **53 passed, 0 failed**. Core cases cover receipt layouts, duplicate and separate-purchase accounting, corrections, refunds, privacy, budgets, inventory and backups. Desktop tests cover OCR limits, timeout/cancel, state-driven navigation, receipt confirmation/cancel, backup actions and selected-zone rendering. |
| Formatting | `cargo fmt --all -- --check` passed. Optional strict Clippy was unavailable because the local toolchain lacks `cargo-clippy`; no install was attempted. |
| Release | `cargo build --release -p hungerstrike-rust-desktop` passed. The portable launch copy is `dist/windows/HungerStrike.exe` (83,011,584 bytes; SHA-256 `7254224D57A79B2FB5319D6A20E63840D521A1AB62C2FFFEEC97DF7F5017956A`), byte-identical to the build output at `target/release/hungerstrike-rust-desktop.exe`. |
| Runtime | Isolated synthetic runs created Windows windows and application-rendered frames. Bevy initialized Vulkan on the NVIDIA GeForce RTX 4060; no startup panic appeared in captured logs. |
| Rendered UI | The app saved [populated kitchen](docs/verification/desktop-kitchen-final.png), [receipt review](docs/verification/desktop-receipt-final.png), [meals](docs/verification/desktop-meals-final.png), and [spending](docs/verification/desktop-spending-final.png) frames. These were inspected visually. Receipt values are synthetic fixture values, and the receipt-only run made no confirmed write. |
| Native interaction | Native mouse and keyboard click-through, 3D picking, window resizing, file dialogs and assistive technology remain unverified because the available computer-control surface did not expose native app windows. App-state tests and rendered frames do not prove those interactions. |
| Local operation | First-party Rust source search found no runtime network calls. `Cargo.lock` has no Tauri, Three.js, Vite, or webbrowser package; the active app has no TypeScript/HTML/CSS source. |

The local Tesseract executable exists at `C:\Program Files\Tesseract-OCR\tesseract.exe`, so the clean synthetic image OCR test ran. The OCR engine/model is **not bundled**. PNG/JPEG byte size and image dimensions are bounded; a stalled local subprocess times out after 15 seconds and active OCR can be cancelled. Tests exercised timeout and cancellation against a deliberately stalled local process. Blur, tilt and crop remain future real-image checks; they were not rerun for this preview.

## Synthetic accounting checks

The R001/R002 fixture receipts total 1,741/1,769 cents and 918/956 cents (subtotal/total). Together: 2,725 cents cash, 1,711 cents food, 948 cents household goods, and 66 cents fixture tax. Reimporting R001 does not add another charge. After the linked milk refund, six eggs consumed and two ounces spinach discarded, remaining known food inventory cost is 1,416 cents; consumption and discard do not change purchase cash flow. These are invented fixture values, not current prices or tax guidance.

R003 is initially blocked. A reviewer can explicitly correct its onion line and assign the unknown savings residual. The regression case enters an 89-cent onion line and a separate -189-cent reconciliation adjustment; its confirmed cash total remains 983 cents. This records the choice rather than inferring a value from damaged text.

Corrections before stock use replace the receipt and inventory atomically and append a field-path/hash audit event. Reimports with harmless whitespace or payment-footer changes are idempotent. Name, expiry, and date corrections after stock use preserve existing cost events. Price or quantity corrections after stock use, and correction after linked refunds, remain blocked; previous totals are preserved on rejection. Refund reversal is bounded by both reviewed refund quantity and available lot quantity.

Matching source text with a different reviewed receipt identity is blocked by ordinary confirmation. A separate purchase requires a unique receipt ID and line IDs plus an explicit acknowledgment; the audit records which prior receipt had the same source. A second attempt with the same identity is rejected. Backup replacement accepts that audited relationship; backup reconciliation conservatively blocks cross-dataset source overlap for manual review.

Temporary OCR/photo bytes are never retained by the app state. Pending text/draft state clears on confirm or cancel. Confirmed SQLite payloads and portable backups omit raw receipt lines and OCR text. Unknown item costs require explicit acceptance and reconciliation; affected lot and meal costs are marked unknown.

## Limits and review decisions

- The first-party Windows interface is Rust Bevy/egui with a fixed selectable fridge/freezer/pantry and an equivalent inventory list. State tests verify zone opening and matching stock representation; native pointer behavior still needs a person-driven smoke test. A customizable kitchen and photo reconstruction have not been approved as final scope.
- Android, Linux, macOS and iOS have not been built or device-tested. Neil has no Mac; Apple builds remain deferred. Android camera/OCR, text input, graphics performance, accessibility and packaging require device work.
- General receipts still require human review; the parser cannot promise universal OCR accuracy or automatic merchant/category classification. Same-source separate purchases depend on the reviewer correctly distinguishing the transactions. Backward compatibility of backups from the superseded Tauri checkpoint has not been verified.
- The portable Windows build was rendered on one Windows 11/NVIDIA Vulkan setup. Other Windows graphics drivers, smaller screens, DPI scaling, local OCR installations, real photos, file-dialog paths and long-running offline use remain release gates. Manual receipt entry remains available without Tesseract.
- Audit logs store sanitized field paths, hashes, totals and a separate-purchase link; they do not retain full draft snapshots or payment text. Historical consumption/discard costs are never rewritten after stock use. Price or quantity corrections in that state remain blocked and need manual reconciliation outside this preview.

The separate [preparation folder](../HungerStrike-preparation/README.md) contains the concise draft requirements, synthetic receipts, expected records, and original fixture accounting check. Its former Tauri implementation plan is marked superseded by the [Rust UI ruling](docs/decisions/2026-10-10-rust-ui-ruling.md).

# HungerStrike Rust desktop checkpoint · 2026-10-10

## Verified locally on Neil's Windows PC

| Check | Result |
| --- | --- |
| Rust tests | `cargo test --workspace`: **42 passed, 0 failed** after the final parser fix. Core coverage includes fixture accounting, duplicate imports, receipt corrections, refunds, unknown costs, privacy, budgets, inventory and backups. Desktop coverage includes local OCR fallback, a clean synthetic OCR sample, and pending-state clearing. |
| Formatting | `cargo fmt --all -- --check` passed. Optional strict Clippy was unavailable because the local toolchain lacks `cargo-clippy`; no install was attempted. |
| Release | `cargo build --release -p hungerstrike-rust-desktop` passed. Executable: `target/release/hungerstrike-rust-desktop.exe` (82,869,760 bytes; SHA-256 `ABDE074860F3A06B1B3473C15498DD3FE2B07FB9825FF08F57A6864E19BFB34A`). |
| Runtime | Release process stayed alive through a six-second isolated run. Bevy created a Windows window and initialized Vulkan on the NVIDIA GeForce RTX 4060. No startup panic appeared in captured logs. |
| Rendered UI | Bevy saved actual window frames for the [empty 3D kitchen](docs/verification/desktop-initial.png) and the [synthetic receipt review draft](docs/verification/receipt-review.png). The receipt capture used a temporary **SYNTHETIC TEST DATA** draft; querying its isolated SQLite state afterward found zero confirmed receipts. |
| Native interaction | Click-through of the window, 3D picking, keyboard navigation, and file dialogs remains unverified because the available computer-control surface did not expose native app windows. Rendered screenshots and passing Rust tests do not prove those interactions. |
| Local operation | First-party Rust source search found no runtime network calls. `Cargo.lock` has no Tauri, Three.js, Vite, or webbrowser package; the active app has no TypeScript/HTML/CSS source. |

The local Tesseract executable exists at `C:\Program Files\Tesseract-OCR\tesseract.exe`, so the clean synthetic image OCR test ran. The OCR engine/model is **not bundled** with this executable. Blur, tilt, and crop remain a broader image-test matrix; those variants were not rerun in this Rust UI checkpoint.

## Synthetic accounting checks

The R001/R002 fixture receipts total 1,741/1,769 cents and 918/956 cents (subtotal/total). Together: 2,725 cents cash, 1,711 cents food, 948 cents household goods, and 66 cents fixture tax. Reimporting R001 does not add another charge. After the linked milk refund, six eggs consumed and two ounces spinach discarded, remaining known food inventory cost is 1,416 cents; consumption and discard do not change purchase cash flow. These are invented fixture values, not current prices or tax guidance.

R003 is initially blocked. A reviewer can explicitly correct its onion line and assign the unknown savings residual. The regression case enters an 89-cent onion line and a separate -189-cent reconciliation adjustment; its confirmed cash total remains 983 cents. This records the choice rather than inferring a value from damaged text.

Corrections before stock use replace the receipt and inventory atomically and append a field-path/hash audit event. Reimports with harmless whitespace or payment-footer changes are idempotent. Name, expiry, and date corrections after stock use preserve existing cost events. Price or quantity corrections after stock use, and correction after linked refunds, remain blocked; previous totals are preserved on rejection. Refund reversal is bounded by both reviewed refund quantity and available lot quantity.

Temporary OCR/photo bytes are never retained by the app state. Pending text/draft state clears on confirm or cancel. Confirmed SQLite payloads and portable backups omit raw receipt lines and OCR text. Unknown item costs require explicit acceptance and reconciliation; affected lot and meal costs are marked unknown.

## Limits and review decisions

- The first-party Windows interface is Rust Bevy/egui with a fixed clickable fridge/freezer/pantry and an equivalent inventory list. Native interaction still needs a person-driven smoke test. A customizable kitchen and photo reconstruction have not been approved as final scope.
- Android, Linux, macOS and iOS have not been built or device-tested. Neil has no Mac; Apple builds remain deferred. Android camera/OCR, text input, graphics performance, accessibility and packaging require device work.
- General receipts still require human review; the parser cannot promise universal OCR accuracy or automatic merchant/category classification. Distinct same-day transactions with identical normalized source text need a reviewed identity policy. Backward compatibility of backups from the superseded Tauri checkpoint has not been verified.
- Auto-review rejected persisting full draft snapshots in audit logs because they could retain payment/identity text. The implemented audit stores sanitized field paths, hashes, and totals. Auto-review also rejected rewriting historical consumption/discard costs during price correction after stock use because that could corrupt accounting. A policy for those historical reallocations needs explicit user approval before implementation.

The separate [preparation folder](../HungerStrike-preparation/README.md) contains the concise draft requirements, synthetic receipts, expected records, and original fixture accounting check. Its former Tauri implementation plan is marked superseded by the [Rust UI ruling](docs/decisions/2026-10-10-rust-ui-ruling.md).

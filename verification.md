# HungerStrike verification — 2026-10-10

## Measured on Neil's Windows machine

| Check | Result |
| --- | --- |
| Rust workspace tests | `cargo test --workspace` passed: 24 tests, 0 failures (receipt parsing, atomic ledger, inventory cost allocation, meals, budgets, category audit, backups, OCR adapter). |
| UI tests | `npm test -- --run` passed: 13 tests, 0 failures in jsdom (receipt review/cancel, kitchen/list search and actions, meals, budget, backup preview). |
| Frontend build | `npm run build` passed. |
| Windows release build | `npm run tauri build` passed; executable at `target/release/hungerstrike-desktop.exe` (11,793,408 bytes; SHA-256 `8F965E7A35F31AEED012EB2C224D11043D66034DE80269C46B1BC70FB53DA5A3`). |
| Windows release startup | Executable process responded and initialized an empty schema-v1 SQLite database under `%LOCALAPPDATA%\local.hungerstrike.desktop`. |
| Native interactive UI check | Not verified: the available computer-use surface listed no native app windows, even while the executable was running. UI behavior is supported by unit/DOM tests, not a completed click-through smoke. |
| Network-disconnected run | Not performed. Source search found no runtime `fetch`/HTTP calls; development config references only localhost and a schema URL. |

Fixture accounting tests confirm: R001 subtotal/total 1741/1769 cents; R002 918/956; together 2725 cash flow, 1711 food, 948 household goods, 66 tax. An exact repeated import has no second charge. An unresolved R003 remains pending. After a linked milk refund, six eggs consumed, and two ounces spinach discarded, inventory food cost is 1416 cents while purchase cash flow remains 2725. Recipe estimates are 118 and 137 cents complete, and 90 and 47 cents partial. These are invented fixture values.

## Offline OCR image matrix

Tesseract 5.4.0 with English data was already installed locally. No OCR program or model was installed or bundled for this project. Four images were rendered from the original synthetic R001 text and inspected; OCR was run on each:

| Synthetic image | `SUBTOTAL` and `TOTAL` recognized | Exact `R001` recognized |
| --- | --- | --- |
| Clean | Yes | No |
| Blurred | No | No |
| Tilted | Yes | No |
| Cropped | No | No |

This is a small smoke matrix, not an accuracy benchmark. The clean scan misread the receipt ID, so user review and correction are required. Missing engine and invalid-image tests preserve manual entry and do not write inventory.

## Platform status

| Target | Status |
| --- | --- |
| Windows | Release executable compiled and launched; native interaction and backup file-dialog click-through remain unverified. |
| Android | Not built or device-tested. Android SDK, NDK 26.3/27.0, and Android Studio JBR exist, but Android Rust targets are not installed, `JAVA_HOME`/`NDK_HOME` are unset, and `adb devices` shows no device. |
| Linux | Not built or tested. |
| macOS / iOS | Not built or tested; Apple work waits for Mac access. |

## Open design and verification work

- Expand receipt recognition beyond the synthetic fixture vocabulary and test with user-approved real receipt samples; unresolved savings lines still require corrected text before confirmation.
- Decide whether to package a verified offline OCR engine/model for other Windows machines and Android; current OCR uses Neil's existing Tesseract installation.
- Perform native Windows UI click-through, including receipt imports, persistence after reopen, WebGL fallback, keyboard list use, and native save dialog/export/reimport. A true network-isolated run also remains.
- Decide how to reconcile partially overlapping backups and how to edit a food/non-food category after stock use without changing historical costs. Current behavior rejects those cases rather than guessing.
- Decide whether a future customizable room editor or photo reconstruction belongs in scope. This build uses one fixed original three-zone kitchen.

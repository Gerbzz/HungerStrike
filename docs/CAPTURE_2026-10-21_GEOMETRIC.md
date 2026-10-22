# Native geometric identity capture · October 21, 2026 UTC

![HungerStrike Kitchen and About with the geometric pantry mark and synthetic stock](../assets/screenshots/hungerstrike-about-geometric-e97d475-1440x900.png)

This is the original, unmodified PNG produced by HungerStrike's built-in Bevy screenshot path in hosted Linux CI. It shows Kitchen with the About panel open, the geometric pantry mark, the HungerStrike name and PWDS attribution. The fixture is explicitly labelled **SYNTHETIC TEST DATA**; no personal receipt or inventory is shown.

## Source and checks

- Reviewed source: [`e97d475d7f702d471e3b13dda06e7c94203274e0`](https://github.com/Gerbzz/HungerStrike/commit/e97d475d7f702d471e3b13dda06e7c94203274e0).
- Reviewed native tree: `94888d155166de7b8aeef06d349652d43b92d4c2`.
- The capture ran from PR test merge `03b2bff00f24a81e3729a5f0d8ca4802cfc97d62`. Its tree equals the reviewed native tree and the [merged commit `6c1cf6d97d0e1c919932123ce670aaaade238186`](https://github.com/Gerbzz/HungerStrike/commit/6c1cf6d97d0e1c919932123ce670aaaade238186).
- [Linux workflow 39689376824, job 119531458209](https://github.com/Gerbzz/HungerStrike/actions/runs/39689376824/job/119531458209) passed the locked native build and About capture. The app wrote the image at 2026-10-21 22:55 UTC.
- [Windows workflow 39689376639, job 119531458118](https://github.com/Gerbzz/HungerStrike/actions/runs/39689376639/job/119531458118) passed formatting, workspace tests, the selected native executable build, original-artwork hash validation, all seven exact PE icon payload checks and three verifier controls.
- Independent source, image and capture-provenance reviews found no blockers before the branding merge.

The Linux workflow built `hungerstrike-rust-desktop` with `cargo build --locked -p hungerstrike-rust-desktop`, then used Xvfb and the app's `about` capture route. `HUNGERSTRIKE_ASSETS_DIR` pointed to that checkout's assets, and `HUNGERSTRIKE_DATA_DIR` pointed to a new isolated fixture folder. The helper checked a complete, nonblank PNG before stopping its own app process.

## File integrity

| Item | Value |
| --- | --- |
| Repository image | `assets/screenshots/hungerstrike-about-geometric-e97d475-1440x900.png` |
| Dimensions | 1440 × 900 |
| Bytes | 703,476 |
| PNG SHA-256 | `2745e6506671f93f6b451334cafdeb8dce941b98d94c5c534a44dc0354e53ce9` |
| Hosted artifact | `12388581196`, `hungerstrike-geometric-native-03b2bff00f24a81e3729a5f0d8ca4802cfc97d62` |
| Original ZIP SHA-256 | `674ddeb330cd2708efa4176f4c73b73972704150efa0b9496189d60deeebad7b` |

The PNG was copied unchanged from that artifact: no crop, repaint, compositing or resizing was applied. Hosted artifacts have a short retention period; the image and hashes above preserve the reviewed capture.

## What this proves

The selected native source builds on Linux and renders the visible Kitchen/About view with the geometric mark. Windows executable resources contain the matching icon at 16, 24, 32, 48, 64, 128 and 256 pixels.

The automated Linux capture does not establish Room interactive acceptance, mouse or keyboard click-through, file dialogs, 3D picking, Windows title-bar or taskbar appearance, other GPUs or DPI settings, or general Linux support. No release, deployment or installer is implied.

The [October 17 Windows gallery](CAPTURE_2026-10-17.md) and [October 15 light-interface gallery](CURRENT_CAPTURE_PROVENANCE.md) remain available as historical records. Their source identifiers and original image files are preserved.

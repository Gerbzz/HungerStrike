# Historical light interface captures

These October 15 source checkpoints predate the geometric branding and are retained as historical evidence. For the native tree merged in `6c1cf6d`, see [the October 21 capture record](CAPTURE_2026-10-21_GEOMETRIC.md).

These are unedited PNGs written by HungerStrike's built-in Bevy screenshot path. Each run used a separate synthetic local SQLite profile. The visible **SYNTHETIC TEST DATA** is not a personal receipt or inventory. The older light and dark captures remain in the repository with separate provenance.

## About layer correction, October 15, 2026 UTC

Source `2c122bf9d81ae4bb38d236e6f6c8cf50579b93cb` moves the About window's draw call after the Kitchen panels and floating storage caption. The earlier `34405b7` About frame remains below as evidence of the title-bar overlap. No other app source changed. A guarded, offline, locked, single-job workspace run passed **71 tests** (53 core and 18 desktop). The guarded offline release executable SHA-256 is `D4A9B120A5B3AEC477F54C59F6B2B11F055C21F034664B96B7AF169F69622042`.

| App-rendered state | PNG SHA-256 |
| --- | --- |
| [About, 1600 x 900](../assets/screenshots/hungerstrike-about-light-2c122bf-1600x900.png) | `59EEA0213425B9A145C49040F6A627DDD20CBA4743E6E953220B292A4B08B2C4` |
| [About, 1280 x 720](../assets/screenshots/hungerstrike-about-light-2c122bf-1280x720.png) | `D966FF8DD51C25A8DD07093A0ACD1BF1EC183B9787B169015FA6D62646092434` |

Both captures used a fresh isolated synthetic `HUNGERSTRIKE_DATA_DIR`, this branch's assets, the `about` capture state, the stated `HUNGERSTRIKE_CAPTURE_SIZE`, and a new PNG path. The app wrote both files through Bevy's screenshot path; the helper checked complete PNG headers and trailers before stopping its own process. Both images were visually inspected: the full About title and X close control are unobstructed, including at 1280 x 720, and the original dormouse and PWDS attribution are present. Their separate stderr logs contain no WARN, ERROR, or panic lines. Native clicking of the close control is not established by these captures. The earlier executable and symbols remain saved outside the repository.

## Previous source checkpoint, October 15, 2026 UTC

Source `34405b73a86724fc45b34a6ea1a843b6eb930ed2` built offline, locked, and single-job using the existing Cargo cache. The release executable SHA-256 is `019585400684DF120218F1C1051DCCF31291FCE273CDF0BDDE4AE5E7CD0DD5C2`. It includes the shared effective Meals selection fix at `59803ea`, for which a regression first failed on the default recipe and then passed. The full workspace run passed **71 tests** (53 core integration tests and 18 desktop tests). `cargo fmt --all --check` and `git diff --check` passed.

| App-rendered state | PNG SHA-256 |
| --- | --- |
| [Selected Kitchen, 1600 × 1100](../assets/screenshots/hungerstrike-kitchen-light-34405b7-1600x1100.png) | `89D902722F63DEDFADACCBD505B45A397903AC1B3629F99B1911A19C30C90AA3` |
| [Selected Kitchen, 1280 × 720](../assets/screenshots/hungerstrike-kitchen-light-34405b7-1280x720.png) | `8A9C6B90363AF0EC558E14AA91C65DB7B9A7E53FC86FD270AEF4F87525FD9BA3` |
| [Cook now Meals, 1600 × 900](../assets/screenshots/hungerstrike-meals-light-34405b7-1600x900.png) | `456BE75F2EDA86F676CC9AFA6BFEC99A6327DE1AEBEA02FE3871C08F9B7B90D4` |
| [Plan next Meals, 1600 × 900](../assets/screenshots/hungerstrike-plan-next-light-34405b7-1600x900.png) | `0332622656686E1CE2D885DA8E9275D6A04EE369546CF3717472D29F7DC78F56` |
| [Shopping entry saved, 1600 × 900](../assets/screenshots/hungerstrike-shopping-light-34405b7-1600x900.png) | `1C1F9233396D64B32273B1F3AA9CFCE22F508BFE76B058A6C7DFBF9981027079` |
| [Stock after Cook, 1600 × 900](../assets/screenshots/hungerstrike-cooked-light-34405b7-1600x900.png) | `353D6CEEB943B41C1984369A5FE64B95EA5F01F2A4C8E8E9496DEE2C7C5C92DD` |
| [Launch view, 1600 × 900](../assets/screenshots/hungerstrike-launch-light-34405b7-1600x900.png) | `136BF5F942F89DB7ABC8E4A8A4AA3590717F5F4469DE4BCB5F72A64D4BD87474` |
| [About view, 1600 × 900](../assets/screenshots/hungerstrike-about-light-34405b7-1600x900.png) | `D9520C803E031387B53CD1938ED2813C8D336C9CF154F53CB52D4E6308161638` |

Each capture used a fresh `HUNGERSTRIKE_DATA_DIR`, branch `assets` via `HUNGERSTRIKE_ASSETS_DIR`, its named `HUNGERSTRIKE_CAPTURE_DEMO` state, exact `HUNGERSTRIKE_CAPTURE_SIZE`, and a new PNG path. The app's Bevy screenshot path wrote each PNG; the helper checked its complete header and trailer before stopping only its own process. All eight frames were inspected, and their separate stderr logs contain no WARN, ERROR, or panic lines. The default Cook now card now visibly matches the Egg and spinach detail. The 1280 Kitchen rail fits its metrics and stock quantities. Launch and About show the approved dormouse and restrained PWDS attribution; the working header carries HungerStrike's own identity. For deterministic capture, the synthetic `launch` state holds the same launch rendering beyond its normal transient duration.

The synthetic `plan-added` state dispatched `AddMissingToShopping` for Tomato pasta. After the app exited, its isolated SQLite `app_state.payload` contained one saved entry: tomatoes, `0.25 lb`; other stock stayed at seeded amounts. The separate `cooked` state dispatched `CookRecipe` for Egg and spinach scramble. Its reopened SQLite payload showed large eggs `12 → 10`, spinach `5 → 4`, and two consumption events of 58 and 60 cents; the on-screen stock value was `$21.40 → $20.22`. These are real application dispatch and persistence paths exercised with synthetic profiles, not native mouse or keyboard clicks. The available desktop automation surface for this task did not expose native app input, so clicking recipe cards, Add missing, Cook, scrolling, file dialogs, and accessibility remain unverified. No user inventory was opened or modified.

## Preserved earlier source checkpoints

| View | Source | Release EXE SHA-256 | PNG SHA-256 |
| --- | --- | --- | --- |
| [Selected Kitchen, 1600 x 1100](../assets/screenshots/hungerstrike-kitchen-light-f9aa030-1600x1100.png) | `f9aa03012214fd618a39e4f44afeac7e188c9f70` | `B4F32B8D4770EAA0BB325EAA89579DFD43F51C2B7ED1BFF37AD9FEEAFE5938BB` | `ECD155ECD1841B0A6E899B5DBE2DA4DF8A265883E970892D0074E80C52907115` |
| [Selected Kitchen, 1280 x 720](../assets/screenshots/hungerstrike-kitchen-light-f9aa030-1280x720.png) | `f9aa03012214fd618a39e4f44afeac7e188c9f70` | `B4F32B8D4770EAA0BB325EAA89579DFD43F51C2B7ED1BFF37AD9FEEAFE5938BB` | `216FAB45C8A1D4BE65895A6AB73B529802B3FC0BA41201E25FA4F2AE1FA273AF` |
| [Cook now Meals, 1600 x 900](../assets/screenshots/hungerstrike-meals-light-f9aa030-1600x900.png) | `f9aa03012214fd618a39e4f44afeac7e188c9f70` | `B4F32B8D4770EAA0BB325EAA89579DFD43F51C2B7ED1BFF37AD9FEEAFE5938BB` | `4E4B167AD3B625E5ACC9ED17C08DB3CC8ADC20BCB285E5B5A91D13B041AB599B` |
| [Plan next Meals, 1600 x 900](../assets/screenshots/hungerstrike-plan-next-light-f9aa030-1600x900.png) | `f9aa03012214fd618a39e4f44afeac7e188c9f70` | `B4F32B8D4770EAA0BB325EAA89579DFD43F51C2B7ED1BFF37AD9FEEAFE5938BB` | `0332622656686E1CE2D885DA8E9275D6A04EE369546CF3717472D29F7DC78F56` |
| [Cook now Meals, 1600 x 900](../assets/screenshots/hungerstrike-meals-light-2d5d5ab-1600x900.png) | `2d5d5ab223067a92c3d2edfdb4a4c482ced0dc97` | `4231DEEA93243C5C0597DCAA37D66657DCCCAE8EC644AFAB17341C6BA5EA29BC` | `4E4B167AD3B625E5ACC9ED17C08DB3CC8ADC20BCB285E5B5A91D13B041AB599B` |
| [Plan next Meals, 1600 x 900](../assets/screenshots/hungerstrike-plan-next-light-2d5d5ab-1600x900.png) | `2d5d5ab223067a92c3d2edfdb4a4c482ced0dc97` | `4231DEEA93243C5C0597DCAA37D66657DCCCAE8EC644AFAB17341C6BA5EA29BC` | `0332622656686E1CE2D885DA8E9275D6A04EE369546CF3717472D29F7DC78F56` |

Both executables built with `cargo build --release --offline --locked -p hungerstrike-rust-desktop -j 1` using the existing dependency cache. The second build changed only `desktop/src/ui.rs`: metric values and labels now stack within three equal cards, and stock rows fit within the Kitchen's left rail. That source was committed unchanged as `f9aa030` immediately after the build and visual inspection. The earlier executable and debug symbols were preserved outside the repository before rebuilding.

Each run set `HUNGERSTRIKE_DATA_DIR` to a new isolated profile, `HUNGERSTRIKE_ASSETS_DIR` to this branch's `assets`, `HUNGERSTRIKE_CAPTURE_DEMO` to `selected`, `meals`, or `plan-next`, `HUNGERSTRIKE_CAPTURE_SIZE` to the stated dimensions, and `HUNGERSTRIKE_CAPTURE_PATH` to a new PNG. The capture script checked complete PNG headers and trailers, then stopped only its own process. Natural app exit is not claimed. The reviewed normal-access captures have separate state-and-size logs under `capture-brand-recipes-reviewed/`; the successful 1280 Kitchen, Cook now, and Plan next logs have no warning, error, or panic lines. The four `f9aa030` frames were visually inspected: Kitchen stock values fit at both sizes, selected-item actions are at the top of the inspector, recipe filters and details render, and the working header shows HungerStrike icon/name for branding without PWDS. The embedded approved centered transparent dormouse hashes to `1652A8D642484EF78C6E4656C44D44EA132E595FCEE3A3F92C1DD5B3D41D23C1`; the authentic PWDS About/launch attribution asset hashes to `B4C19554261C06EFCA0EE81879BC9CCCAFBEF7FDE84E92328354324E39670C18`.

Two restricted-access 1280 x 720 launches and a later restricted-access 1600 x 900 Meals launch of the second executable exited at Windows window creation with `0xC0000008`, before any PNG was written. Their isolated profiles and logs are retained outside the repository under `capture-brand-recipes-v2/` and `capture-brand-recipes-v2b/`. The exact same executable then captured 1280 Kitchen, Cook now, and Plan next successfully through reviewed normal access. The failure was specific to the restricted launch context; the evidence does not identify the exact Windows handle or permission involved. The two Meals PNGs were byte-identical across `2d5d5ab` and `f9aa030`, consistent with the sole Kitchen sizing source change. Later review found that the default Cook now detail selected Egg and spinach scramble without visibly highlighting its list card. That defect was corrected at `59803ea` and the corrected frame was recaptured from `34405b7` above.

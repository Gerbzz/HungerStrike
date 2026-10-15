# Light interface development preview provenance

The [Kitchen](../assets/screenshots/hungerstrike-kitchen-light-34405b7-1600x1100.png) and [Cook now Meals](../assets/screenshots/hungerstrike-meals-light-34405b7-1600x900.png) images in the default-branch README are unchanged copies of verified PNGs from [draft light-interface PR #1](https://github.com/Gerbzz/HungerStrike/pull/1). They show a development preview, **not the executable built from this default branch**. The older dark Kitchen PNG remains archived with [its own source record](SCREENSHOT_PROVENANCE.md).

| App-rendered view | Image size | PNG SHA-256 |
| --- | --- | --- |
| [Selected Kitchen](../assets/screenshots/hungerstrike-kitchen-light-34405b7-1600x1100.png) | 1600 × 1100 | `89D902722F63DEDFADACCBD505B45A397903AC1B3629F99B1911A19C30C90AA3` |
| [Cook now Meals](../assets/screenshots/hungerstrike-meals-light-34405b7-1600x900.png) | 1600 × 900 | `456BE75F2EDA86F676CC9AFA6BFEC99A6327DE1AEBEA02FE3871C08F9B7B90D4` |

Both images came from app source `34405b73a86724fc45b34a6ea1a843b6eb930ed2` on the draft branch. Its offline, locked, single-job release executable had SHA-256 `019585400684DF120218F1C1051DCCF31291FCE273CDF0BDDE4AE5E7CD0DD5C2`; its workspace suite passed 71 tests (53 core, 18 desktop). The app's built-in Bevy screenshot path wrote the unedited PNGs from separate synthetic `HUNGERSTRIKE_DATA_DIR` SQLite profiles, with the draft branch's assets, `HUNGERSTRIKE_CAPTURE_DEMO=selected` or `meals`, the stated capture size, and a new output path. The capture helper checked the PNG headers and trailers before stopping its own app process. Both frames were visually inspected and their separate stderr logs contained no WARN, ERROR, or panic lines.

The later draft-branch head `04277b4` passed 73 offline workspace tests and includes a recipe date-ordering fix and a Meals freshness cue. **These two images predate those later changes.** The default branch retained its older app source when this documentation was published; copying the PNGs and this record did not merge draft app code. Native mouse and keyboard use, OCR, backup restore, accessibility, broader receipts, and other devices are not established by these images. No personal inventory or receipt appears in them.

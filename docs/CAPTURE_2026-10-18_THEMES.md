# Theme captures · 2026-10-18 · source `6da6e66`

Light (the default, original look) and Dark frames saved by the HungerStrike debug executable built from commit `6da6e66`, on Neil's Windows 11 PC (RTX 4060, Vulkan). Each run used a new, empty data folder, the app's synthetic capture route, and `HUNGERSTRIKE_CAPTURE_THEME` to preview a theme without saving it. Each image is exactly the 1440 × 900 app window. All data is **SYNTHETIC TEST DATA**. These are automated captures, not an interactive test.

| Screen | Light | Dark |
| --- | --- | --- |
| Kitchen, selected lot, expiry editor, Use first | `hungerstrike-selected-light-6da6e66-1440x900.png` (796,004 B, `a7d52f41…15bb8e`) | `hungerstrike-selected-dark-6da6e66-1440x900.png` (801,650 B, `0eef832d…1b26`) |
| Receipt review (bundled OCR on the synthetic photo) | `hungerstrike-receipt-photo-light-6da6e66-1440x900.png` (186,724 B, `ac84b4a4…8a6c`) | `hungerstrike-receipt-photo-dark-6da6e66-1440x900.png` (183,512 B, `3621214c…c8b7`) |
| Spending and budgets | `hungerstrike-spending-light-6da6e66-1440x900.png` (229,936 B, `249f5c23…6179`) | `hungerstrike-spending-dark-6da6e66-1440x900.png` (225,010 B, `95dd1023…1d1f`) |
| Data, with the Theme control | `hungerstrike-data-light-6da6e66-1440x900.png` (110,789 B, `f1cde26d…012c`) | `hungerstrike-data-dark-6da6e66-1440x900.png` (110,390 B, `8ce05454…ce9`) |

The light receipt-review and spending frames are **byte-identical** to the `7222c4b` frames captured before the theme work (same SHA-256), which confirms the default light look did not change.

Known in these frames and fixed afterwards in `a9cedd3` (not re-captured, because Smart App Control blocked executables built after that commit):
- Spending shows egui's red debug overlay "First/Second use of widget ID" under the two "Full period breakdown" headers (a duplicate widget ID that existed before the theme work).
- Data still says photo OCR needs a separately installed Tesseract; the text now describes the bundled engine.

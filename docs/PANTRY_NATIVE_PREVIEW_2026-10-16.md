# Pantry recommendation native preview

The [1600 x 900 Plan next frame](../assets/screenshots/hungerstrike-plan-next-66f5dca-1600x900.png) and [1280 x 720 frame](../assets/screenshots/hungerstrike-plan-next-66f5dca-1280x720.png) were captured from the Windows desktop executable built from exact source commit [`66f5dcaf2ae585f4d3bc53d4a1af22651dbb0232`](https://github.com/Gerbzz/HungerStrike/commit/66f5dcaf2ae585f4d3bc53d4a1af22651dbb0232). This source entered `main` through [PR #2](https://github.com/Gerbzz/HungerStrike/pull/2). The images show isolated, synthetic local stock; they contain no real receipt, user inventory, or private asset.

Hosted Windows [PR run 38952706223](https://github.com/Gerbzz/HungerStrike/actions/runs/38952706223) and [push run 38952700428](https://github.com/Gerbzz/HungerStrike/actions/runs/38952700428) both passed at that commit. The PR run passed 79 workspace tests (61 core and 18 desktop), built a debug desktop executable, checked its size and SHA-256, and uploaded a one-day artifact. The 144,229,888-byte downloaded executable matched the hosted SHA-256 manifest:

`C1EF7F6CC179C4067AEAE1EF7F01A1C3D848D23D07CE5006D66524FA9114C0C9`

The app's synthetic Plan next capture route generated each PNG in a fresh, isolated local profile. Both were visually inspected: the first ranked card is highlighted, its Banana oat bowl details agree with the card, and the missing oats quantity is visible. At 1280 x 720, the lower cards and shopping action continue below the viewport. These are screenshots of a source preview, not a release or installer. Mouse and keyboard interaction, file dialogs, and behavior on other machines were not checked by this capture route.

| Frame | Bytes | SHA-256 |
| --- | ---: | --- |
| 1600 x 900 | 263,548 | `D7083EB80BFC94A77A0DDDA9DE99BE7E828DC43B71CEC1ECD4A8EFB20F889ACB` |
| 1280 x 720 | 209,807 | `0D0D769CE5731DBC3F175C9F96CF00E7ADBDD91975D372A481C929B4DB94DBA5` |

The executable and local synthetic database were not added to this repository.

# Bundled OCR models

HungerStrike reads receipt photos on the device with the
[ocrs](https://github.com/robertknight/ocrs) engine (MIT OR Apache-2.0) and the
[rten](https://github.com/robertknight/rten) runtime (MIT OR Apache-2.0), both by
Robert Knight. The two model files in this folder are the published ocrs models
and are compiled into the desktop executable.

| File | Bytes | SHA-256 | Source |
| --- | ---: | --- | --- |
| `text-detection.rten` | 2,510,284 | `f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca` | https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten |
| `text-recognition.rten` | 9,716,568 | `e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e` | https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten |

These are the download locations used by the ocrs project's own tooling
(downloaded 2026-10-17). The models are published by Robert Knight at
<https://huggingface.co/robertknight/ocrs> under the
[Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/)
license (CC BY-SA 4.0). According to the model card they are trained on
[HierText](https://github.com/google-research-datasets/hiertext) and synthetic
data. Training code: <https://github.com/robertknight/ocrs-models>.

The model files are redistributed unmodified. Any redistribution of them must
keep this attribution and the CC BY-SA 4.0 terms. They are not covered by any
license chosen for HungerStrike's own source code.

No network request is made at runtime; OCR needs no account, key or paid API.

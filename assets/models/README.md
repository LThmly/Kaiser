# Bundled OCR models

These are unmodified text-detection and text-recognition models by Robert Knight,
used by [Ocrs](https://github.com/robertknight/ocrs) and executed by its portable
Rust RTen runtime. They are embedded in Kaiser via `include_bytes!`; end users
never have to install OCR software or download weights at runtime.

Model project and licensing: https://huggingface.co/robertknight/ocrs
Training and model source: https://github.com/robertknight/ocrs-models
License: **Creative Commons Attribution-ShareAlike 4.0 International**.
https://creativecommons.org/licenses/by-sa/4.0/
A copy of the license is in `LICENSE-CC-BY-SA-4.0.txt`.
Kaiser's MIT license covers Kaiser code; it does not replace the model license.

Retrieved 2026-10-21 from the official URLs used by Ocrs 0.13.1's example:

| File | Source | SHA-256 |
| --- | --- | --- |
| `text-detection.onnx` | https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx | `a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe` |
| `text-recognition.onnx` | https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx | `86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5` |

The committed artifacts and hashes pin these exact weights. These upstream URLs
are mutable; verify checksums before replacing the files. No model training or
modification was performed for Kaiser.

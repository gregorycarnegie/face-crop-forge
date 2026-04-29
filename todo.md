# Repo improvement checklist

## State / logic

- [x] Wire up or remove ghost fields in `ProcessingSettings` (`skin_smoothing`, `red_eye_removal`, `background_blur`, `auto_color_correction`, `exposure_adjustment`) — removed; dead filter in `runtime.rs` also removed
- [x] `min_confidence` default raised from `0.0` to `0.5`

## Code structure

- [x] Break up `src/pages/csv.rs` (1244 lines) → split into `pages/csv/{mod,page,process,helpers}.rs`
- [x] Break up `src/pages/batch.rs` (911 lines) → split into `pages/batch/{mod,page,process,helpers}.rs`

## Assets / git hygiene

- [x] Move binary model files (`models/*.tflite`, `models/vision_bundle.mjs`, `models/wasm/*`) out of git — hosted as GitHub Release assets (`models-v1`), downloaded at build time in CI; `models/` excluded via `.gitignore`

## Testing

- [x] Add component-level tests for the Leptos UI layer — 23 WASM tests across 6 components in `src/components/tests.rs`

## Housekeeping

- [x] Add `CHANGELOG.md` for v0.2.0
- [x] Fix stale `CONTRIBUTING.md` — updated to match actual file structure and removed references to deleted modules
- [x] Remove broken `just perf` recipe — `src/perf_snapshot.rs` no longer exists
- [x] Add `Default` impl to `AppState`
- [x] Rename the repo/folder from `opencv-face-crop` to `face-crop-forge` on GitHub (Settings → Repository name) (only called `opencv-face-crop` on local disk - we can ignore this)

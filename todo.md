# Repo improvement checklist

## State / logic

- [x] Wire up or remove ghost fields in `ProcessingSettings` (`skin_smoothing`, `red_eye_removal`, `background_blur`, `auto_color_correction`, `exposure_adjustment`) — removed; dead filter in `runtime.rs` also removed
- [x] `min_confidence` default raised from `0.0` to `0.5`

## Code structure

- [x] Break up `src/pages/csv.rs` (1244 lines) → split into `pages/csv/{mod,page,process,helpers}.rs`
- [x] Break up `src/pages/batch.rs` (911 lines) → split into `pages/batch/{mod,page,process,helpers}.rs`

## Assets / git hygiene

- [x] `dist/` is already excluded by `.gitignore` and not tracked — no action needed
- [ ] **Manual:** Move binary model files (`models/*.tflite`, `models/vision_bundle.mjs`, `models/wasm/*`) out of git — host as GitHub Release assets or a CDN, fetch at runtime or build time

## Testing

- [ ] **Manual:** Add component-level or integration tests for the Leptos UI layer (requires wasm-bindgen-test or a browser test harness)

## Housekeeping

- [x] Add `CHANGELOG.md` for v0.2.0
- [ ] **Manual:** Rename the repo/folder from `opencv-face-crop` to `face-crop-forge` on GitHub (Settings → Repository name)

# Changelog

## [0.2.0] — current

### Added

- CSV workflow (Workflow 03): map source filenames to output names via a CSV manifest, process matched images, and export a named ZIP
- CSV column auto-detection with heuristic column name matching
- Confidence threshold slider on all three processing routes (Single, Batch, CSV)
- Docs route (`/docs`)
- Web Worker bridge with dual-backend detection: native `FaceDetector` API with MediaPipe Tasks fallback
- Browser capability detection at startup (SIMD, threads, OffscreenCanvas, ImageBitmap)
- Pipeline health diagnostics panel on the Single route
- Batch queue pagination (first page loaded immediately; remainder dequeued on process start)
- ZIP export with MIME-normalised filenames on both Batch and CSV routes
- `continue_on_error` toggle: skip failures and keep processing
- SPA 404 fallback (`dist/404.html`) for GitHub Pages routing

### Changed

- `min_confidence` default raised from 0.0 to 0.5 — previously all detections passed regardless of score
- Removed unimplemented settings fields (`auto_color_correction`, `exposure_adjustment`, `contrast_adjustment`, `sharpness`, `skin_smoothing`, `red_eye_removal`, `background_blur`) that had no UI controls and produced a no-op CSS filter
- `pages/batch` and `pages/csv` split into `page`, `process`, and `helpers` submodules for maintainability

### Fixed

- GitHub Pages base path handling for routes and assets
- MediaPipe asset paths under Trunk dev server
- Landing page preview layering
- Long filenames in output preview card overflowing the container
- Batch and CSV filter tab controls

## [0.1.0] — initial release

- Single-image face detection and crop (Workflow 02)
- Batch folder processing with gallery and output grid (Workflow 01)
- MediaPipe BlazeFace short-range model bundled locally
- Leptos CSR SPA compiled to `wasm32-unknown-unknown` via Trunk
- CI: fmt, clippy, test, release build, GitHub Pages deploy

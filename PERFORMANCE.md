# Performance Optimizations

This document summarizes real runtime performance characteristics of Face Crop Forge.

## Worker Pipeline

The app uses a Web Worker (`assets/workers/face-detection-worker.js`) managed by Rust (`src/worker_bridge.rs`) to keep detection and heavy processing off the main thread.

- Main-thread responsiveness during long batch runs
- Explicit worker lifecycle/status transitions
- Structured worker error propagation into UI logs/status panels

## Browser/WASM Pipeline

The runtime uses browser capability checks in `src/mediapipe.rs` to select execution paths.

- OffscreenCanvas/ImageBitmap-capable browsers use the worker transfer path
- Other browsers use compatible fallback execution paths
- Pipeline health is surfaced in the Single route diagnostics panel

Recommended response headers (see `deploy/nginx.conf`):

- `Cross-Origin-Embedder-Policy: require-corp`
- `Cross-Origin-Opener-Policy: same-origin`

## Export Runtime

Export work is fully real artifact generation in `src/export_runtime.rs`.

- Binary crop output creation and download
- ZIP generation for Batch/CSV exports
- MIME/extension normalization and validation for output filenames

## Measuring Runtime Performance

Run:

```bash
just perf
```

This executes Rust performance snapshot tests in `src/perf_snapshot.rs` and prints runtime timings for representative workloads.

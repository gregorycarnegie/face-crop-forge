# Performance

## Runtime Measurements

Last run: 2026-02-27 20:54:21 UTC  
Command: `cargo test perf_snapshot -- --nocapture`  
Workloads defined in `src/perf_snapshot.rs`.

| Workload | Time |
|---|---|
| Batch work-plan generation (10k images ×50) | 234ms |
| CSV parse + map + match (5k rows ×10) | 551ms |
| Export filename generation (100k) | 358ms |

Run locally:

```bash
just perf
```

## Worker Pipeline

Detection and heavy processing run in a Web Worker managed by `src/worker_bridge.rs`, keeping the main thread responsive during batch jobs. Worker lifecycle transitions and errors are surfaced in the UI status/log panels.

## Browser/WASM Execution Paths

`src/mediapipe.rs` checks browser capabilities at startup:

- OffscreenCanvas + ImageBitmap → worker transfer path
- Otherwise → compatible fallback path

Pipeline health is visible in the Single route diagnostics panel.

Recommended response headers for best WASM/MediaPipe performance (see `deploy/nginx.conf`):

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: cross-origin
```

## Export Runtime

All export work is real artifact generation in `src/export_runtime.rs`:

- Binary crop output creation and download
- ZIP generation for Batch/CSV exports
- MIME/extension normalization for output filenames (`png`, `jpeg` → `jpg`, `webp`)

## Regression Guards

Output format and naming behavior is enforced by:

- `src/perf_snapshot.rs` — `download_format_quality_regression_guardrails`
- `src/single_core.rs` — `export_filename_format_mapping_matches_legacy_behavior`
- `src/csv_core.rs` — `export_filename_preserves_output_format_extensions`

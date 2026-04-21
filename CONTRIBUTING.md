# Contributing

## Prerequisites

- Rust stable
- `wasm32-unknown-unknown` target
- Trunk
- `just` (optional but recommended)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install just
```

## Development

```bash
just dev       # trunk serve (hot reload)
just check     # cargo check --target wasm32-unknown-unknown
just fmt       # cargo fmt
just lint      # cargo clippy --target wasm32-unknown-unknown -- -D warnings
just test      # cargo test
just perf      # runtime performance snapshot
```

## Project Layout

| Path | Purpose |
|---|---|
| `src/app/` | Leptos router, pages, components |
| `src/worker_bridge.rs` | Web Worker lifecycle and message protocol |
| `src/mediapipe.rs` | MediaPipe JS bridge and capability detection |
| `src/single_core.rs` | Single-image detection/crop/export flow |
| `src/batch_core.rs` | Batch processing loop and retry policy |
| `src/csv_core.rs` | CSV parse, map, and export flow |
| `src/crop_math.rs` | Crop region geometry |
| `src/preprocessing.rs` | Exposure/contrast/sharpness adjustments |
| `src/quality_filters.rs` | Blur and confidence threshold gates |
| `src/export_runtime.rs` | Artifact generation and ZIP export |
| `assets/workers/` | Web Worker JS entry point |
| `deploy/` | Nginx config and deployment notes |

## Guidelines

- All processing runs client-side — no server round-trips or external API calls.
- Heavy work (detection, batch loops) belongs in the Web Worker, not the main thread.
- New crop/export logic goes through `src/crop_math.rs` and `src/export_runtime.rs` respectively so the regression guards in `src/perf_snapshot.rs` cover it.
- Run `just lint` before submitting — clippy warnings are treated as errors.
- Run `just test` and confirm `just perf` numbers haven't regressed significantly.

## License

By contributing you agree your changes will be licensed under AGPL-3.0.

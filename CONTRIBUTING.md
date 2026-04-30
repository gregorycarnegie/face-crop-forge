# Contributing

## Prerequisites

- Rust stable
- `wasm32-unknown-unknown` target
- Trunk
- wasm-pack
- Firefox for the local browser test recipe; CI runs the same WASM tests in Chrome
- `just` (optional but recommended)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install wasm-pack
cargo install just
```

## Development

```bash
just dev       # trunk serve (hot reload)
just check     # cargo check --target wasm32-unknown-unknown
just fmt       # cargo fmt
just lint      # cargo clippy --target wasm32-unknown-unknown -- -D warnings
just test      # cargo test
just browser-test # wasm-pack test --headless --firefox
```

## Project Layout

| Path                    | Purpose                                                             |
|-------------------------|---------------------------------------------------------------------|
| `src/main.rs`           | App entry point, top-level routing                                  |
| `src/router.rs`         | Route enum, URL helpers, popstate listener                          |
| `src/state.rs`          | Shared `AppState` and `ProcessingSettings`                          |
| `src/pages/home.rs`     | Landing page                                                        |
| `src/pages/single.rs`   | Single-image detection/crop/export flow                             |
| `src/pages/batch/`      | Batch processing loop (page, process, helpers)                      |
| `src/pages/csv/`        | CSV parse, map, and export flow (page, process, helpers)            |
| `src/pages/docs.rs`     | Documentation page                                                  |
| `src/components/`       | Reusable Leptos UI components                                       |
| `src/worker_bridge.rs`  | Browser detection bridge (native FaceDetector + MediaPipe fallback) |
| `src/mediapipe.rs`      | MediaPipe asset paths and browser capability matrix                 |
| `src/single_core.rs`    | Single-image state and runtime types                                |
| `src/batch_core.rs`     | Batch processing state                                              |
| `src/csv_core.rs`       | CSV processing state                                                |
| `src/export_runtime.rs` | Artifact generation, filename normalization, ZIP export             |
| `src/batch_export.rs`   | Batch export orchestration                                          |
| `src/runtime.rs`        | Shared runtime abstractions                                         |
| `src/base_runtime.rs`   | Base runtime types                                                  |
| `src/base_ui.rs`        | Base UI helpers                                                     |

## Guidelines

- All processing runs client-side — no server round-trips or external API calls.
- Detection runs through the browser `FaceDetector` API first, then falls back to the bundled MediaPipe Tasks assets. New detection logic belongs in `src/worker_bridge.rs`.
- New export logic goes through `src/export_runtime.rs` so the unit tests there cover it.
- Run `just lint` before submitting — clippy warnings are treated as errors.
- Run `just test` to verify unit tests pass, and `just browser-test` to run WASM browser tests locally.

## License

By contributing you agree your changes will be licensed under AGPL-3.0.

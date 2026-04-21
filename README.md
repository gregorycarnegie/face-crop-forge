# Face Crop Forge

[![CI](https://github.com/gregorycarnegie/face-crop-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/gregorycarnegie/face-crop-forge/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)
![Rust](https://img.shields.io/badge/Rust-stable-f74c00.svg)
![Leptos](https://img.shields.io/badge/Leptos-0.8.16-blue.svg)
![MediaPipe](https://img.shields.io/badge/MediaPipe-Face%20Detection-brightgreen.svg)

Client-side face detection and cropping app running as a Leptos (WASM) SPA.

## Architecture

- Runtime: Rust + Leptos CSR (`wasm32-unknown-unknown`)
- Entry point: `index.html` (Trunk)
- App router/UI: `src/app.rs`
- Worker bridge/runtime protocol: `src/worker_bridge.rs`
- Core processing modules:
  - `src/single_core.rs`
  - `src/batch_core.rs`
  - `src/csv_core.rs`
  - `src/crop_math.rs`
  - `src/preprocessing.rs`
  - `src/quality_filters.rs`
  - `src/export_runtime.rs`

Primary routes:

- `/` landing
- `/single` single processing
- `/batch` batch processing
- `/csv` CSV workflow

## Runtime Status

- Single, Batch, and CSV flows run on real image inputs end-to-end.
- Worker detection drives real status/progress updates and error reporting.
- Exports generate real output files and ZIP artifacts with validated names/extensions.
- No user-visible route depends on simulated placeholders.

## Development

Prerequisites:

- Rust stable
- `wasm32-unknown-unknown` target
- Trunk
- `just` (optional)

No Node/Bun toolchain is required.

Setup:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install just
```

Run:

```bash
just dev
# or
trunk serve
```

Build:

```bash
just build
# or
trunk build --release
```

Useful commands:

```bash
just check   # cargo check --target wasm32-unknown-unknown
just fmt     # cargo fmt
just lint    # cargo clippy --target wasm32-unknown-unknown -- -D warnings
just test    # cargo test
just perf    # runtime performance snapshot
```

## Deployment (Trunk Artifacts)

Release build output:

```bash
trunk build --release
```

Deploy contents of Trunk `dist/` to static hosting.

Recommended headers for best WASM/MediaPipe performance:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`
- `Cross-Origin-Resource-Policy: cross-origin`

Nginx example: `deploy/nginx.conf`.

## License

AGPL-3.0. See `LICENSE`.

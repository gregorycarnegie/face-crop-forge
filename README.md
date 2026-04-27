# Face Crop Forge

[![CI](https://github.com/gregorycarnegie/face-crop-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/gregorycarnegie/face-crop-forge/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)
![Rust](https://img.shields.io/badge/Rust-stable-f74c00.svg)
![Leptos](https://img.shields.io/badge/Leptos-0.8.19-blue.svg)
![MediaPipe](https://img.shields.io/badge/MediaPipe-Face%20Detection-brightgreen.svg)

Client-side face detection and cropping app running as a Leptos (WASM) SPA.

## Architecture

- Runtime: Rust + Leptos CSR (`wasm32-unknown-unknown`)
- Entry point: `index.html` (Trunk)
- App router/UI: `src/router.rs`
- Worker bridge/runtime protocol: `src/worker_bridge.rs`
- Shared state and settings: `src/state.rs`
- Runtime abstractions: `src/runtime.rs`, `src/base_runtime.rs`
- Core processing modules:
  - `src/single_core.rs`
  - `src/batch_core.rs`
  - `src/csv_core.rs`
  - `src/export_runtime.rs`
  - `src/batch_export.rs`
- Reusable UI components: `src/components/`
- Route pages: `src/pages/` (home, single, batch, csv)

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
just clean   # cargo clean
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

## Performance

### Worker Pipeline

Detection and heavy processing run in a Web Worker managed by `src/worker_bridge.rs`, keeping the main thread responsive during batch jobs. Worker lifecycle transitions and errors are surfaced in the UI status/log panels.

### Browser/WASM Execution Paths

`src/mediapipe.rs` checks browser capabilities at startup:

- OffscreenCanvas + ImageBitmap → worker transfer path
- Otherwise → compatible fallback path

Pipeline health is visible in the Single route diagnostics panel.

### Export Runtime

All export work is real artifact generation in `src/export_runtime.rs`:

- Binary crop output creation and download
- ZIP generation for Batch/CSV exports
- MIME/extension normalization for output filenames (`png`, `jpeg` → `jpg`, `webp`)

### Regression Guards

Output format and naming behavior is enforced by:

- `src/single_core.rs` — `export_filename_format_mapping_matches_legacy_behavior`
- `src/csv_core.rs` — `export_filename_preserves_output_format_extensions`

## Third-Party Notices

### MediaPipe

- **License**: Apache License 2.0
- **Copyright**: Copyright 2019 The MediaPipe Authors
- **Project URL**: <https://github.com/google/mediapipe>
- **Description**: Cross-platform, customizable ML solutions for live and streaming media

```text
Copyright 2019 The MediaPipe Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

Full Apache 2.0 License text: <https://www.apache.org/licenses/LICENSE-2.0>

## License

AGPL-3.0. See `LICENSE`.

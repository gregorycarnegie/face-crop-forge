# Performance Snapshot

Last run: 2026-02-27 20:54:21 UTC

## Method

- Command: `cargo test perf_snapshot -- --nocapture`
- Workloads are defined in `src/perf_snapshot.rs`
- Values below are latest measured runtime timings for the Rust/Leptos implementation

## Latest Measurements

- Batch work-plan generation (`10k images x50`)
  - Rust: `234ms`
- CSV parse+map+match (`5k rows x10`)
  - Rust: `551ms`
- Export filename generation (`100k`)
  - Rust: `358ms`

## Regression Guards

Format and naming guardrails are enforced by tests in:

- `src/perf_snapshot.rs` (`download_format_quality_regression_guardrails`)
- `src/single_core.rs` (`export_filename_format_mapping_matches_legacy_behavior`)
- `src/csv_core.rs` (`export_filename_preserves_output_format_extensions`)

These tests validate output extension mapping (`png`, `jpeg` -> `jpg`, `webp`) and filename-generation behavior for single and CSV exports.

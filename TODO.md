# Real Implementation TODO (Remove Simulated Paths)

Goal: replace all placeholder/simulated runtime behavior with real image processing, detection, and export flows in Rust/WASM.

## 1. Single Page: Replace Simulated Detection/Preview

- [x] Replace hardcoded face list assignment (`set_faces(vec!["face_1", ...])`) on upload/drop with real detection kickoff in `src/app.rs`.
- [x] Implement real image decode + draw-to-canvas pipeline for single preview (`inputCanvas`/`outputCanvas`) instead of status-only updates.
- [x] Replace `Detect Faces` simulated path (`"Rust simulation"`) with actual worker request + parsed detections in `src/app.rs`.
- [x] Render real face overlay boxes from detection coordinates (not synthetic IDs) in `src/app.rs` + style hooks.
- [x] Update selection state to use stable detection IDs from real results (not generated label strings).

## 2. Single Page: Webcam Real Pipeline

- [x] Replace `"camera pipeline pending"` capture status with real webcam frame capture into processing pipeline.
- [x] Connect webcam capture to same detection/crop path as file upload.
- [x] Implement actual camera switching with `MediaDevices` and stream restart logic.
- [x] Ensure webcam modal close always releases stream tracks.

## 3. Batch Page: Replace Simulated Processing Loop

- [x] Replace synthetic metadata assumptions (`1200x800`, fixed mime/file size) with real per-file metadata extraction.
- [x] Replace `simulate_detection_with_retry` execution in batch route with real worker/model detection calls.
- [x] Replace synthetic `attempt_time_ms` stats with measured elapsed times.
- [x] Keep retry policy behavior but apply it to real detection failures (timeouts/model errors/no faces).
- [x] Show real gallery thumbnails and processed outputs, not status-only transitions.

## 4. CSV Page: Replace Simulated Process/Export Flow

- [x] Replace CSV process-all/process-selected simulation loops (`record_result(true)`) with real per-image processing outcomes.
- [x] Replace fixed synthetic stats (e.g., `record_image(22, 2, true)`) with measured and observed values.
- [x] Ensure CSV mapped output names are applied to real generated outputs.
- [x] Connect CSV preview/current image panel to actual decoded image + detection results.

## 5. Worker Bridge + Detection Runtime

- [x] Expand `src/worker_bridge.rs` from lifecycle management to full request/response protocol handling.
- [x] Define typed worker message contracts for detect/crop/enhance tasks.
- [x] Replace placeholder worker status text flows with real state transitions from worker replies.
- [x] Add robust worker error propagation to UI logs/error panel.

## 6. Real Cropping and Export Artifacts

- [x] Replace export planning-only behavior with actual binary artifact generation for single outputs.
- [x] Implement real ZIP creation for batch/CSV outputs (currently planning only via `plan_zip_export`).
- [x] Replace placeholder timestamp token (`"pending"`) in export paths with real UTC timestamp generation.
- [x] Validate generated filenames/extensions from actual output blobs across png/jpeg/webp.

## 7. Settings and Enhancements: Wire to Real Processing

- [x] Apply crop settings (`outputWidth`, `outputHeight`, positioning, offsets) to real crop math at runtime.
- [x] Apply preprocessing controls (exposure/contrast/sharpness/smoothing/etc.) to real image output path.
- [x] Ensure quality filters (blur/confidence thresholds) gate real detections/crops, not mocked outputs.
- [x] Persist and restore settings management actions with real serialized settings payloads.

## 8. Routing/UX Completion for Real Runtime

- [x] Ensure upload cards show per-file progress and errors from real processing, not generic status strings.
- [x] Ensure clear/reset actions revoke object URLs, clear canvases, clear overlays, and reset worker jobs.
- [x] Add route-level busy/disabled guards during active worker jobs.

## 9. Testing for Real (Non-Simulated) Behavior

- [x] Add unit tests for worker message parsing and failure handling.
- [x] Add integration tests covering real detection result -> overlay -> crop -> export flow contracts.
- [x] Replace simulation-focused test names in `src/flow_tests.rs` with real-runtime assertions.
- [x] Add regression tests for webcam capture flow and drag/drop multi-file behavior.

## 10. Cleanup After De-Simulation

- [x] Remove simulation-specific strings/statuses from UI (`"Rust simulation"`, `"pending"`, `"pipeline pending"`).
- [x] Remove or rename simulation helpers once replaced (`simulate_detection_with_retry` in `src/base_runtime.rs`).
- [x] Remove dead code paths introduced only for interim migration scaffolding.
- [x] Update docs (`README.md`, `PERFORMANCE.md`, `PERF_COMPARISON.md`) to describe real runtime behavior only.

## Definition of Done (Real Runtime)

- [ ] No user-visible feature relies on simulated outputs.
- [ ] Single, Batch, and CSV routes process real images end-to-end.
- [ ] Webcam capture participates in the same real detection/cropping pipeline.
- [ ] Download buttons produce actual files/ZIPs with correct content and names.
- [ ] Test suite validates real runtime contracts; simulation-only tests removed.

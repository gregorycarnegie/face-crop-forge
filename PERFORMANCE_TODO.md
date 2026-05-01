# Face Crop Forge Performance Checklist

A practical checklist for improving runtime performance, responsiveness, memory use, and large-batch reliability.

## Priority 1 — Highest impact

- [ ] Benchmark current performance before changing anything.
  - [ ] Measure single-image detection time.
  - [ ] Measure 10-image, 100-image, and 500-image batch runs.
  - [ ] Record browser, device, image sizes, output format, and detection backend.
  - [x] Add a simple developer-only timing panel or console timing output.

- [x] Move batch detection, cropping, and export work into a real Web Worker.
  - [x] Keep the main thread focused on UI, file selection, previews, and progress display.
  - [x] Send files/jobs to the worker.
  - [x] Send progress events back to the UI.
  - [x] Keep detection, crop generation, and ZIP creation off the UI thread where possible.
  - [x] Rename or split `worker_bridge.rs` so the name reflects the actual architecture.

- [x] Cache the MediaPipe detector instance.
  - [x] Avoid dynamically importing the MediaPipe bundle for every image.
  - [x] Avoid resolving the vision fileset repeatedly.
  - [x] Avoid creating a new detector for every detection request.
  - [x] Create one detector per worker/session and reuse it across batch jobs.
  - [x] Add safe fallback/reinitialisation if the cached detector fails.

- [x] Detect on a downscaled image, then crop from the original.
  - [x] Decode or render a detection-sized version of the image.
  - [x] Cap detection input to a sensible max dimension, for example 1024–1600 px.
  - [x] Run face detection on the smaller image.
  - [x] Scale the detected face box back to original image coordinates.
  - [x] Crop/export from the original-resolution image to preserve quality.
  - [x] Add tests for coordinate scaling.

- [x] Change ZIP export to use stored files by default.
  - [x] Review current `Deflated` ZIP compression.
  - [x] Use `Stored` for already-compressed image outputs such as JPG, PNG, and WebP.
  - [ ] Optionally add an advanced setting: `Fast ZIP` vs `Smaller ZIP`.
  - [ ] Benchmark export time and ZIP size difference.

## Priority 2 — Responsiveness and throughput

- [x] Add controlled batch concurrency.
  - [x] Start conservatively with 1 concurrent job on mobile and 2 on desktop.
  - [x] Consider using `navigator.hardwareConcurrency` as an upper bound.
  - [x] Avoid unlimited parallel image decodes or detection calls.
  - [x] Add a queue that can pause, resume, and cancel cleanly.
  - [x] Surface current concurrency in a developer stats panel.

- [ ] Use `createImageBitmap()` where supported.
  - [ ] Prefer `createImageBitmap(file)` over object URL + `HtmlImageElement` for decode paths where practical.
  - [ ] Reuse the decoded bitmap for detection and cropping when possible.
  - [ ] Close `ImageBitmap` objects once processing is finished.
  - [ ] Keep the existing image element path as a compatibility fallback.

- [x] Use `OffscreenCanvas` for worker-side crop generation.
  - [x] Detect support for `OffscreenCanvas`.
  - [x] Move crop drawing and encoding into the worker where supported.
  - [x] Keep a normal canvas fallback for unsupported browsers.
  - [ ] Benchmark crop/export time with and without `OffscreenCanvas`.

- [x] Throttle progress and log UI updates.
  - [x] Avoid reactive state updates for every tiny internal step.
  - [ ] Update progress per image or every 100–250 ms.
  - [ ] Batch log updates during large runs.
  - [x] Keep recent logs capped, as currently done.
  - [x] Check whether Leptos rerenders become noisy during large batches.

## Priority 3 — Memory and large-batch reliability

- [x] Avoid holding all crop bytes and the final ZIP in memory at the same time.
  - [x] Stream generated entries into the ZIP where possible.
  - [x] Release each crop buffer after it has been written.
  - [x] Avoid duplicating large byte arrays unnecessarily.
  - [ ] Track approximate memory use during batch export.

- [x] Split very large exports into multiple ZIP files.
  - [x] Define a sensible max entries-per-ZIP or max estimated ZIP size.
  - [x] Export names like `face-crops-part-001.zip`, `face-crops-part-002.zip`.
  - [x] Make the split behaviour clear in the UI.
  - [x] Add tests for deterministic part naming.

- [x] Revoke and release browser resources aggressively.
  - [x] Revoke object URLs as soon as images are decoded.
  - [x] Close `ImageBitmap`s after use.
  - [x] Drop canvas references after export.
  - [x] Clear temporary batch state when a run finishes or is cancelled.

- [x] Add cancellation support for long-running batches.
  - [x] Add a cancel button to batch and CSV workflows.
  - [x] Stop queueing new jobs after cancellation.
  - [x] Let the currently running job finish or abort safely.
  - [x] Clean up temporary buffers and object URLs.
  - [x] Show a clear cancelled state in the UI.

## Priority 4 — Build and asset performance

- [ ] Compare release profiles.
  - [ ] Benchmark current `opt-level = "z"` release build.
  - [ ] Benchmark `opt-level = 3` for runtime-heavy workloads.
  - [ ] Compare WASM size, initial load time, and batch processing time.
  - [ ] Pick the profile based on measured results, not assumptions.

- [ ] Review MediaPipe asset loading.
  - [ ] Confirm MediaPipe assets are cached correctly by the browser.
  - [ ] Ensure model and WASM files are served with appropriate cache headers.
  - [ ] Consider preloading the fallback model only when needed.
  - [x] Show detection backend status clearly in the UI.

- [ ] Review recommended deployment headers.
  - [ ] Confirm static hosting can send `Cross-Origin-Opener-Policy: same-origin`.
  - [ ] Confirm static hosting can send `Cross-Origin-Embedder-Policy: require-corp`.
  - [ ] Confirm static hosting can send `Cross-Origin-Resource-Policy: cross-origin` where appropriate.
  - [ ] Document any hosting limitations.

## Priority 5 — Measurement and regression guards

- [x] Add performance regression tests or smoke checks.
  - [x] Add a small benchmark-like test for ZIP generation.
  - [ ] Add browser-level timing smoke tests for single-image detection.
  - [x] Add a large-batch simulation test for queue/progress behaviour.
  - [x] Track average processing time and export time separately.

- [x] Add developer diagnostics.
  - [x] Show selected detection backend: native FaceDetector or MediaPipe.
  - [x] Show decode time, detection time, crop time, and export time.
  - [x] Show image dimensions and downscale factor.
  - [x] Show queue length and concurrency.
  - [x] Add a copyable debug summary for bug reports.

- [x] Add browser compatibility notes.
  - [x] Document which browsers support native `FaceDetector`.
  - [x] Document when MediaPipe fallback is used.
  - [x] Document `createImageBitmap` and `OffscreenCanvas` fallback behaviour.
  - [x] Explain why performance may vary between Chrome, Edge, Firefox, Safari, desktop, and mobile.

## Suggested implementation order

- [x] Add timing instrumentation.
- [x] Change ZIP default from deflated to stored and benchmark it.
- [x] Cache the MediaPipe detector.
- [x] Add detection downscaling and coordinate remapping.
- [x] Add controlled batch concurrency.
- [x] Move batch work into a real Web Worker.
- [x] Add `OffscreenCanvas` support inside the worker.
- [x] Reduce memory duplication during ZIP export.
- [x] Add cancellation and cleanup.
- [x] Add regression checks so performance does not drift backwards.

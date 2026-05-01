use crate::base_runtime::{ImageMeta, ImageValidationConfig, validate_image_meta};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats};
use crate::batch_export::BatchProgress;
use crate::export_runtime::{
    build_zip_bytes, current_timestamp_ms, current_utc_timestamp_token, download_bytes,
    normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::runtime::{
    MAX_DETECTION_SIDE, ProcessedImageOutput, apply_detection_quality_filters, batch_file_label,
    compute_source_crop_rect, crop_face_bytes_from_source, decode_image_dimensions,
    elapsed_ms_since, is_probably_image_file, make_file_id, maybe_downscale_for_detection,
    mime_type_for_output_format, object_url_for_bytes, object_url_for_file, revoke_object_url,
    revoke_preview_urls, scale_detected_faces,
};
use crate::single_core::generate_face_filename;
use crate::state::ProcessingSettings;
use crate::worker_bridge::{CropFaceWorkerRequest, crop_face_in_worker, detect_faces_with_worker};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy)]
pub(super) struct BatchProcessCtx {
    pub(super) progress: RwSignal<BatchProgress>,
    pub(super) stats: RwSignal<BatchRuntimeStats>,
    pub(super) batch_state: RwSignal<BatchCoreState>,
    pub(super) progress_pct: RwSignal<u32>,
}

fn record_batch_failure(
    ctx: BatchProcessCtx,
    id: &str,
    elapsed_ms: u64,
    status: impl Into<String>,
    log: impl Into<String>,
) {
    let BatchProcessCtx {
        progress,
        stats,
        batch_state,
        progress_pct,
    } = ctx;
    progress.update(|p| {
        p.record_result(false);
        p.status = status.into();
    });
    let new_pct = progress.with_untracked(|p| u32::from(p.percent()));
    if new_pct != progress_pct.get_untracked() {
        progress_pct.set(new_pct);
    }
    stats.update(|s| {
        s.record_image(elapsed_ms, 0, false);
        s.push_log(log);
    });
    batch_state.update(|state| state.mark_error(id));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn load_batch_files(
    files: Vec<web_sys::File>,
    batch_state: RwSignal<BatchCoreState>,
    batch_queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    progress: RwSignal<BatchProgress>,
    progress_pct: RwSignal<u32>,
    stats: RwSignal<BatchRuntimeStats>,
) {
    revoke_preview_urls(&preview_urls.get());
    for output in outputs.get().values() {
        revoke_object_url(&output.preview_url);
    }

    let mut ids = Vec::new();
    let mut file_map = HashMap::new();
    let mut preview_map = HashMap::new();
    for (index, file) in files.into_iter().enumerate() {
        if !is_probably_image_file(&file) {
            continue;
        }
        let id = make_file_id(&file, index);
        if let Some(url) = object_url_for_file(&file) {
            preview_map.insert(id.clone(), url);
        }
        file_map.insert(id.clone(), file);
        ids.push(id);
    }

    let queue_state = BatchQueueState::from_files(&ids, 48);
    let loaded = queue_state.loaded_ids.clone();
    let total = ids.len();
    let queued = queue_state.queued_files_count();
    batch_state.update(|state| state.set_images(loaded));
    batch_queue.set(queue_state);
    files_by_id.set(file_map);
    preview_urls.set(preview_map);
    outputs.set(HashMap::new());
    stats.update(BatchRuntimeStats::reset);
    progress_pct.set(0);
    progress.update(|p| {
        p.reset();
        p.status = if queued > 0 {
            format!("Loaded first page from {total} image(s); {queued} queued for processing.")
        } else {
            format!("Loaded {total} image(s). Ready to process.")
        };
    });
}

/// How many images to process concurrently.
/// Conservative defaults: 1 on touch/mobile, 2 on desktop (capped by hardwareConcurrency).
#[cfg(target_arch = "wasm32")]
pub(super) fn batch_concurrency_limit() -> usize {
    use wasm_bindgen::JsValue;
    let window = leptos::prelude::window();
    let hardware = js_sys::Reflect::get(
        &window.navigator(),
        &JsValue::from_str("hardwareConcurrency"),
    )
    .ok()
    .and_then(|v| v.as_f64())
    .map(|n| n as usize)
    .unwrap_or(1);
    let is_touch = window.navigator().max_touch_points() > 0;
    if is_touch {
        1
    } else {
        2_usize.min(hardware.max(1))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn batch_concurrency_limit() -> usize {
    1
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_batch(
    settings: RwSignal<ProcessingSettings>,
    batch_state: RwSignal<BatchCoreState>,
    batch_queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    progress: RwSignal<BatchProgress>,
    progress_pct: RwSignal<u32>,
    stats: RwSignal<BatchRuntimeStats>,
    continue_on_error: bool,
    cancel_requested: RwSignal<bool>,
) {
    batch_queue.update(|queue| {
        while let Some(page) = queue.dequeue_next_page() {
            batch_state.update(|state| state.add_images(page));
        }
    });

    let selected_ids = batch_state.get().selected_ids();
    if selected_ids.is_empty() {
        progress.update(|p| p.status = "Select at least one image to process.".to_string());
        return;
    }

    let files = Rc::new(files_by_id.get());
    let previews = Rc::new(preview_urls.get());
    let settings_snapshot = settings.get();
    let mime_type = mime_type_for_output_format(&settings_snapshot.output_format).to_string();
    for id in &selected_ids {
        if let Some(old) = outputs.get().get(id) {
            revoke_object_url(&old.preview_url);
        }
    }
    outputs.update(|map| {
        for id in &selected_ids {
            map.remove(id);
        }
    });
    batch_state.update(|state| selected_ids.iter().for_each(|id| state.mark_processing(id)));
    cancel_requested.set(false);
    stats.update(BatchRuntimeStats::reset);
    progress_pct.set(0);
    progress.update(|p| {
        p.start(
            selected_ids.len(),
            format!("Processing {} image(s)...", selected_ids.len()),
        )
    });

    let total = selected_ids.len();
    let concurrency = batch_concurrency_limit();
    let queue: Rc<RefCell<VecDeque<String>>> =
        Rc::new(RefCell::new(selected_ids.into_iter().collect()));
    let workers_remaining: Rc<Cell<usize>> = Rc::new(Cell::new(concurrency));
    let abort: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let started_count: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let ctx = BatchProcessCtx {
        progress,
        stats,
        batch_state,
        progress_pct,
    };

    for _ in 0..concurrency {
        let queue = Rc::clone(&queue);
        let workers_remaining = Rc::clone(&workers_remaining);
        let abort = Rc::clone(&abort);
        let files = Rc::clone(&files);
        let previews = Rc::clone(&previews);
        let settings_snapshot = settings_snapshot.clone();
        let mime_type = mime_type.clone();
        let started_count = Rc::clone(&started_count);

        spawn_local(async move {
            let validation = ImageValidationConfig::default();
            loop {
                if abort.get() || cancel_requested.get_untracked() {
                    break;
                }
                let id = queue.borrow_mut().pop_front();
                let Some(id) = id else {
                    break;
                };

                let start_ms = crate::runtime::now_ms();
                let Some(file) = files.get(&id).cloned() else {
                    record_batch_failure(
                        ctx,
                        &id,
                        0,
                        "Missing source file.",
                        "Missing source file.",
                    );
                    if !continue_on_error {
                        abort.set(true);
                        break;
                    }
                    continue;
                };
                let file_name = file.name();
                let n = started_count.get() + 1;
                started_count.set(n);
                progress.update(|p| {
                    p.status = format!("Processing {n}/{total}: {file_name}");
                });

                #[cfg(target_arch = "wasm32")]
                let decode_start = crate::runtime::now_ms();
                let dimensions = match decode_image_dimensions(&file).await {
                    Ok(d) => d,
                    Err(error) => {
                        record_batch_failure(
                            ctx,
                            &id,
                            elapsed_ms_since(start_ms),
                            format!("Decode failed: {file_name}"),
                            format!("Decode failed for {file_name}: {error}"),
                        );
                        if !continue_on_error {
                            abort.set(true);
                            break;
                        }
                        continue;
                    }
                };
                #[cfg(target_arch = "wasm32")]
                let decode_ms = elapsed_ms_since(decode_start);
                if let Err(error) = validate_image_meta(
                    ImageMeta {
                        file_name: &file_name,
                        mime_type: &file.type_(),
                        file_size_bytes: file.size() as u64,
                        dimensions,
                    },
                    validation,
                ) {
                    record_batch_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Validation failed: {file_name}"),
                        format!("Validation failed for {file_name}: {error}"),
                    );
                    if !continue_on_error {
                        abort.set(true);
                        break;
                    }
                    continue;
                }

                let (detection_file, detection_scale) = match maybe_downscale_for_detection(
                    &file,
                    dimensions,
                    MAX_DETECTION_SIDE,
                )
                .await
                {
                    Ok(pair) => pair,
                    Err(_) => (file.clone(), 1.0),
                };

                #[cfg(target_arch = "wasm32")]
                let detect_start = crate::runtime::now_ms();
                let faces = match detect_faces_with_worker("browser-face-detector", detection_file)
                    .await
                {
                    Ok(raw) => {
                        let scaled =
                            scale_detected_faces(raw, 1.0 / detection_scale, 1.0 / detection_scale);
                        apply_detection_quality_filters(scaled, &settings_snapshot)
                    }
                    Err(error) => {
                        record_batch_failure(
                            ctx,
                            &id,
                            elapsed_ms_since(start_ms),
                            format!("Detection failed: {file_name}"),
                            format!("Detection failed for {file_name}: {error}"),
                        );
                        if !continue_on_error {
                            abort.set(true);
                            break;
                        }
                        continue;
                    }
                };
                #[cfg(target_arch = "wasm32")]
                let detect_ms = elapsed_ms_since(detect_start);
                let face_count = faces.len();
                let Some(face) = faces.into_iter().next() else {
                    record_batch_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("No face found: {file_name}"),
                        format!("No crop target found for {file_name}."),
                    );
                    if !continue_on_error {
                        abort.set(true);
                        break;
                    }
                    continue;
                };

                #[cfg(target_arch = "wasm32")]
                let crop_start = crate::runtime::now_ms();
                let (src_x, src_y, src_w, src_h) = compute_source_crop_rect(
                    &face,
                    f64::from(dimensions.width),
                    f64::from(dimensions.height),
                    &settings_snapshot,
                );
                let out_w = settings_snapshot.output_width.max(1);
                let out_h = settings_snapshot.output_height.max(1);
                let quality = if mime_type == "image/png" {
                    None
                } else {
                    Some(f64::from(settings_snapshot.jpeg_quality))
                };
                let crop_bytes = match crop_face_in_worker(CropFaceWorkerRequest {
                    file: &file,
                    source_x: src_x,
                    source_y: src_y,
                    source_width: src_w,
                    source_height: src_h,
                    output_width: out_w,
                    output_height: out_h,
                    mime_type: &mime_type,
                    quality,
                })
                .await
                {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        // OffscreenCanvas not supported; fall back to main-thread crop
                        let mut temporary_url = None;
                        let source_url = previews.get(&id).cloned().or_else(|| {
                            let url = object_url_for_file(&file);
                            temporary_url.clone_from(&url);
                            url
                        });
                        let Some(source_url) = source_url else {
                            record_batch_failure(
                                ctx,
                                &id,
                                elapsed_ms_since(start_ms),
                                format!("Preview URL failed: {file_name}"),
                                format!("Could not create source URL for {file_name}."),
                            );
                            if !continue_on_error {
                                abort.set(true);
                                break;
                            }
                            continue;
                        };
                        match crop_face_bytes_from_source(
                            &source_url,
                            &face,
                            &settings_snapshot,
                            &mime_type,
                        )
                        .await
                        {
                            Ok(bytes) => {
                                if let Some(url) = temporary_url.as_deref() {
                                    revoke_object_url(url);
                                }
                                bytes
                            }
                            Err(error) => {
                                if let Some(url) = temporary_url.as_deref() {
                                    revoke_object_url(url);
                                }
                                record_batch_failure(
                                    ctx,
                                    &id,
                                    elapsed_ms_since(start_ms),
                                    format!("Crop failed: {file_name}"),
                                    format!("Crop failed for {file_name}: {error}"),
                                );
                                if !continue_on_error {
                                    abort.set(true);
                                    break;
                                }
                                continue;
                            }
                        }
                    }
                };
                #[cfg(target_arch = "wasm32")]
                let crop_ms = elapsed_ms_since(crop_start);

                let preview_url = match object_url_for_bytes(&crop_bytes, &mime_type) {
                    Ok(url) => url,
                    Err(error) => {
                        record_batch_failure(
                            ctx,
                            &id,
                            elapsed_ms_since(start_ms),
                            format!("Preview failed: {file_name}"),
                            format!("Crop preview failed for {file_name}: {error}"),
                        );
                        if !continue_on_error {
                            abort.set(true);
                            break;
                        }
                        continue;
                    }
                };

                outputs.update(|map| {
                    if let Some(old) = map.insert(
                        id.clone(),
                        ProcessedImageOutput {
                            bytes: crop_bytes,
                            mime_type: mime_type.clone(),
                            preview_url,
                        },
                    ) {
                        revoke_object_url(&old.preview_url);
                    }
                });
                batch_state.update(|state| state.mark_processed(&id));
                progress.update(|p| {
                    p.record_result(true);
                    p.status = format!(
                        "Processed {}/{}: {file_name} ({face_count} face(s))",
                        p.processed, total
                    );
                });
                let new_pct = progress.with_untracked(|p| u32::from(p.percent()));
                if new_pct != progress_pct.get_untracked() {
                    progress_pct.set(new_pct);
                }
                let total_ms = elapsed_ms_since(start_ms);
                stats.update(|s| {
                    s.record_image(total_ms, face_count as u32, true);
                    s.record_backend(crate::worker_bridge::last_detection_backend_label());
                    s.record_image_size(dimensions.width, dimensions.height, detection_scale);
                    s.push_log(format!("Processed {file_name}: {face_count} face(s)."));
                });
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!(
                        "[FCF timing] {file_name}: decode={decode_ms}ms detect={detect_ms}ms crop={crop_ms}ms total={total_ms}ms backend={}",
                        crate::worker_bridge::last_detection_backend_label()
                    )
                    .into(),
                );
            }

            let remaining = workers_remaining.get().saturating_sub(1);
            workers_remaining.set(remaining);
            if remaining == 0 {
                if cancel_requested.get_untracked() {
                    progress.update(|p| {
                        let status =
                            format!("Cancelled after {}/{} images.", p.processed, p.total,);
                        p.cancel(status);
                        progress_pct.set(u32::from(p.percent()));
                    });
                } else {
                    progress.update(|p| {
                        let status = format!(
                            "Batch complete: {} processed, {} failed.",
                            p.processed.saturating_sub(p.failed),
                            p.failed
                        );
                        p.complete(status);
                        progress_pct.set(u32::from(p.percent()));
                    });
                }
            }
        });
    }
}

pub(super) fn download_batch_zip(
    settings: RwSignal<ProcessingSettings>,
    batch_state: RwSignal<BatchCoreState>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    progress: RwSignal<BatchProgress>,
    stats: RwSignal<BatchRuntimeStats>,
) {
    let mut ids = batch_state
        .get()
        .images
        .values()
        .filter(|image| image.processed)
        .map(|image| image.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    if ids.is_empty() {
        progress.update(|p| p.status = "No processed crops to export.".to_string());
        return;
    }

    let settings_snapshot = settings.get();
    let outputs_snapshot = outputs.get();
    let timestamp = current_timestamp_ms();
    let timestamp_token = current_utc_timestamp_token();
    spawn_local(async move {
        let mut entries: Vec<(String, &[u8])> = Vec::new();
        for (index, id) in ids.into_iter().enumerate() {
            let Some(output) = outputs_snapshot.get(&id) else {
                continue;
            };
            let source_name = batch_file_label(&id).to_string();
            let generated = generate_face_filename(
                &settings_snapshot.naming_template,
                &source_name,
                index,
                settings_snapshot.output_width,
                settings_snapshot.output_height,
                &settings_snapshot.output_format,
                timestamp,
            );
            let final_name = normalize_export_filename_for_mime(&generated, &output.mime_type);
            if !validate_export_filename_for_mime(&final_name, &output.mime_type) {
                stats
                    .update(|s| s.push_log(format!("ZIP skipped invalid file name: {final_name}")));
                continue;
            }
            entries.push((final_name, &output.bytes));
        }

        if entries.is_empty() {
            progress.update(|p| p.complete("No cropped outputs available for ZIP export."));
            return;
        }

        let byte_sizes: Vec<usize> = entries.iter().map(|(_, b)| b.len()).collect();
        let total_raw_bytes: usize = byte_sizes.iter().sum();
        let total_raw_mb = total_raw_bytes as f64 / 1_048_576.0;
        stats.update(|s| {
            s.push_log(format!(
                "Exporting {} file(s) (~{total_raw_mb:.1} MB uncompressed).",
                byte_sizes.len()
            ));
        });
        let part_ranges = crate::export_runtime::compute_zip_part_ranges(
            &byte_sizes,
            crate::export_runtime::MAX_ZIP_PART_BYTES,
            crate::export_runtime::MAX_ZIP_PART_ENTRIES,
        );
        let total_parts = part_ranges.len();
        let total_count = entries.len();
        #[cfg(target_arch = "wasm32")]
        let mut total_export_ms: u64 = 0;

        for (part_idx, range) in part_ranges.into_iter().enumerate() {
            let part_name =
                crate::export_runtime::zip_part_filename(&timestamp_token, part_idx, total_parts);
            #[cfg(target_arch = "wasm32")]
            let export_start = crate::runtime::now_ms();
            let zip_result = build_zip_bytes(&entries[range], settings_snapshot.zip_compress);
            #[cfg(target_arch = "wasm32")]
            {
                total_export_ms =
                    total_export_ms.saturating_add(crate::runtime::elapsed_ms_since(export_start));
            }
            match zip_result {
                Ok(zip_bytes) => {
                    if let Err(error) = download_bytes(&part_name, "application/zip", &zip_bytes) {
                        drop(zip_bytes);
                        drop(entries);
                        drop(outputs_snapshot);
                        progress.update(|p| p.complete(format!("ZIP download failed: {error}")));
                        return;
                    }
                }
                Err(error) => {
                    drop(entries);
                    drop(outputs_snapshot);
                    progress.update(|p| p.complete(format!("ZIP build failed: {error}")));
                    return;
                }
            }
        }

        drop(entries);
        drop(outputs_snapshot);

        let summary = if total_parts == 1 {
            format!("Exported face-crops-{timestamp_token}.zip with {total_count} file(s).")
        } else {
            format!("Exported {total_parts} ZIP parts with {total_count} file(s) total.")
        };
        progress.update(|p| p.complete(summary.clone()));
        #[cfg(target_arch = "wasm32")]
        stats.update(|s| {
            s.record_export(total_export_ms);
            s.push_log(format!("{summary} ({total_export_ms}ms)"));
        });
        #[cfg(not(target_arch = "wasm32"))]
        stats.update(|s| s.push_log(summary));
    });
}

pub(super) fn clear_batch(
    ctx: BatchProcessCtx,
    batch_queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
) {
    let BatchProcessCtx {
        progress,
        stats,
        batch_state,
        progress_pct,
    } = ctx;
    revoke_preview_urls(&preview_urls.get());
    for output in outputs.get().values() {
        revoke_object_url(&output.preview_url);
    }
    batch_state.set(BatchCoreState::default());
    batch_queue.set(BatchQueueState::default());
    files_by_id.set(HashMap::new());
    preview_urls.set(HashMap::new());
    outputs.set(HashMap::new());
    progress.set(BatchProgress::default());
    progress_pct.set(0);
    stats.set(BatchRuntimeStats::default());
}

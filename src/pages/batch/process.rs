use crate::base_runtime::{ImageMeta, ImageValidationConfig, validate_image_meta};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats};
use crate::batch_export::BatchProgress;
use crate::export_runtime::{
    build_zip_bytes, current_timestamp_ms, current_utc_timestamp_token, download_bytes,
    normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::runtime::{
    ProcessedImageOutput, apply_detection_quality_filters, batch_file_label,
    crop_face_bytes_from_source, decode_image_dimensions, elapsed_ms_since, is_probably_image_file,
    make_file_id, mime_type_for_output_format, object_url_for_bytes, object_url_for_file,
    revoke_object_url, revoke_preview_urls,
};
use crate::single_core::generate_face_filename;
use crate::state::ProcessingSettings;
use crate::worker_bridge::detect_faces_with_worker;
use leptos::prelude::*;
use std::collections::HashMap;
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
        progress_pct.set(u32::from(p.percent()));
    });
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

    let files = files_by_id.get();
    let previews = preview_urls.get();
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
    stats.update(BatchRuntimeStats::reset);
    progress_pct.set(0);
    progress.update(|p| {
        p.start(
            selected_ids.len(),
            format!("Processing {} image(s)...", selected_ids.len()),
        )
    });

    spawn_local(async move {
        let ctx = BatchProcessCtx {
            progress,
            stats,
            batch_state,
            progress_pct,
        };
        let total = selected_ids.len();
        let validation = ImageValidationConfig::default();
        for (index, id) in selected_ids.into_iter().enumerate() {
            let start_ms = crate::runtime::now_ms();
            let Some(file) = files.get(&id).cloned() else {
                record_batch_failure(ctx, &id, 0, "Missing source file.", "Missing source file.");
                if !continue_on_error {
                    break;
                }
                continue;
            };
            let file_name = file.name();
            progress
                .update(|p| p.status = format!("Processing {}/{}: {file_name}", index + 1, total));

            let dimensions = match decode_image_dimensions(&file).await {
                Ok(dimensions) => dimensions,
                Err(error) => {
                    record_batch_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Decode failed: {file_name}"),
                        format!("Decode failed for {file_name}: {error}"),
                    );
                    if !continue_on_error {
                        break;
                    }
                    continue;
                }
            };
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
                    break;
                }
                continue;
            }

            let faces = match detect_faces_with_worker("browser-face-detector", file.clone()).await
            {
                Ok(faces) => apply_detection_quality_filters(faces, &settings_snapshot),
                Err(error) => {
                    record_batch_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Detection failed: {file_name}"),
                        format!("Detection failed for {file_name}: {error}"),
                    );
                    if !continue_on_error {
                        break;
                    }
                    continue;
                }
            };
            let Some(face) = faces.first() else {
                record_batch_failure(
                    ctx,
                    &id,
                    elapsed_ms_since(start_ms),
                    format!("No face found: {file_name}"),
                    format!("No crop target found for {file_name}."),
                );
                if !continue_on_error {
                    break;
                }
                continue;
            };

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
                    break;
                }
                continue;
            };

            let crop_bytes = match crop_face_bytes_from_source(
                &source_url,
                face,
                &settings_snapshot,
                &mime_type,
            )
            .await
            {
                Ok(bytes) => bytes,
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
                        break;
                    }
                    continue;
                }
            };
            if let Some(url) = temporary_url.as_deref() {
                revoke_object_url(url);
            }

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
                    "Processed {}/{}: {} ({} face(s))",
                    index + 1,
                    total,
                    file_name,
                    faces.len()
                );
                progress_pct.set(u32::from(p.percent()));
            });
            stats.update(|s| {
                s.record_image(elapsed_ms_since(start_ms), faces.len() as u32, true);
                s.push_log(format!("Processed {file_name}: {} face(s).", faces.len()));
            });
        }

        progress.update(|p| {
            let status = format!(
                "Batch complete: {} processed, {} failed.",
                p.processed.saturating_sub(p.failed),
                p.failed
            );
            p.complete(status);
            progress_pct.set(u32::from(p.percent()));
        });
    });
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
    let zip_name = format!("face-crops-{}.zip", current_utc_timestamp_token());
    spawn_local(async move {
        let mut entries = Vec::new();
        for (index, id) in ids.into_iter().enumerate() {
            let Some(output) = outputs_snapshot.get(&id).cloned() else {
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
            entries.push((final_name, output.bytes));
        }

        if entries.is_empty() {
            progress.update(|p| p.complete("No cropped outputs available for ZIP export."));
            return;
        }

        match build_zip_bytes(&entries) {
            Ok(bytes) => match download_bytes(&zip_name, "application/zip", &bytes) {
                Ok(()) => {
                    let count = entries.len();
                    progress.update(|p| {
                        p.complete(format!("Exported {zip_name} with {count} file(s)."))
                    });
                    stats.update(|s| {
                        s.push_log(format!("Exported ZIP {zip_name} with {count} file(s)."))
                    });
                }
                Err(error) => {
                    progress.update(|p| p.complete(format!("ZIP download failed: {error}")))
                }
            },
            Err(error) => progress.update(|p| p.complete(format!("ZIP build failed: {error}"))),
        }
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

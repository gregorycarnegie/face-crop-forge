use crate::base_runtime::{ImageMeta, ImageValidationConfig, validate_image_meta};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats};
use crate::batch_export::BatchProgress;
use crate::csv_core::{CsvCoreState, CsvExportNameContext};
use crate::export_runtime::{
    build_zip_bytes, current_timestamp_ms, current_utc_timestamp_token, download_bytes,
    normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::runtime::{
    ProcessedImageOutput, apply_detection_quality_filters, batch_file_label,
    crop_face_bytes_from_source, decode_image_dimensions, elapsed_ms_since,
    is_probably_image_file, make_file_id, mime_type_for_output_format, object_url_for_bytes,
    object_url_for_file, revoke_object_url, revoke_preview_urls,
};
use crate::state::ProcessingSettings;
use crate::worker_bridge::detect_faces_with_worker;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use super::helpers::{csv_template_display, guess_column};

pub(super) fn load_csv_file(
    file: web_sys::File,
    csv_state: RwSignal<CsvCoreState>,
    file_path_column: RwSignal<String>,
    file_name_column: RwSignal<String>,
    mapping_confirmed: RwSignal<bool>,
    progress: RwSignal<BatchProgress>,
) {
    progress.update(|p| p.status = format!("Reading CSV: {}", file.name()));
    spawn_local(async move {
        let text = match JsFuture::from(file.text()).await {
            Ok(value) => value.as_string().unwrap_or_default(),
            Err(error) => {
                progress.update(|p| p.status = format!("CSV read failed: {error:?}"));
                return;
            }
        };
        let mut parsed = false;
        csv_state.update(|state| parsed = state.parse_csv_text(&text));
        if !parsed {
            mapping_confirmed.set(false);
            progress.update(|p| p.status = "CSV parse failed. Check headers and rows.".to_string());
            return;
        }

        let headers = csv_state.get().headers;
        let path_guess = guess_column(
            &headers,
            &[
                "file_path",
                "filepath",
                "path",
                "source",
                "image",
                "filename",
            ],
        );
        let name_guess = guess_column(
            &headers,
            &["output_id", "output_name", "member_id", "id", "name"],
        );
        file_path_column.set(path_guess.clone());
        file_name_column.set(name_guess.clone());
        let mut applied = false;
        csv_state.update(|state| applied = state.apply_mapping(&path_guess, &name_guess));
        mapping_confirmed.set(applied);
        progress.update(|p| {
            p.status = if applied {
                format!("CSV loaded and mapped: {path_guess} -> {name_guess}")
            } else {
                "CSV loaded. Confirm source and output columns.".to_string()
            };
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn load_csv_images(
    files: Vec<web_sys::File>,
    csv_state: RwSignal<CsvCoreState>,
    batch_state: RwSignal<BatchCoreState>,
    queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    source_name_by_id: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    face_count_by_id: RwSignal<HashMap<String, usize>>,
    progress: RwSignal<BatchProgress>,
    progress_pct: RwSignal<u32>,
    stats: RwSignal<BatchRuntimeStats>,
) {
    let csv = csv_state.get();
    if csv.mapping.is_none() {
        progress.update(|p| p.status = "Confirm CSV mapping before adding images.".to_string());
        return;
    }

    revoke_preview_urls(&preview_urls.get());
    for output in outputs.get().values() {
        revoke_object_url(&output.preview_url);
    }

    let mut ids = Vec::new();
    let mut file_map = HashMap::new();
    let mut preview_map = HashMap::new();
    let mut source_map = HashMap::new();
    let mut skipped = 0usize;
    for (index, file) in files.into_iter().enumerate() {
        if !is_probably_image_file(&file) {
            skipped += 1;
            continue;
        }
        let source_name = file.name();
        if csv.output_name_for_file(&source_name).is_none() {
            skipped += 1;
            continue;
        }
        let id = make_file_id(&file, index);
        if let Some(url) = object_url_for_file(&file) {
            preview_map.insert(id.clone(), url);
        }
        source_map.insert(id.clone(), source_name);
        file_map.insert(id.clone(), file);
        ids.push(id);
    }

    let queue_state = BatchQueueState::from_files(&ids, 48);
    let loaded = queue_state.loaded_ids.clone();
    batch_state.update(|state| state.set_images(loaded));
    queue.set(queue_state);
    files_by_id.set(file_map);
    preview_urls.set(preview_map);
    source_name_by_id.set(source_map);
    outputs.set(HashMap::new());
    face_count_by_id.set(HashMap::new());
    stats.update(BatchRuntimeStats::reset);
    progress_pct.set(0);
    progress.update(|p| {
        p.reset();
        p.status = format!("Matched {} image(s); skipped {skipped}.", ids.len());
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_csv_batch(
    settings: RwSignal<ProcessingSettings>,
    csv_state: RwSignal<CsvCoreState>,
    batch_state: RwSignal<BatchCoreState>,
    queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    source_name_by_id: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    face_count_by_id: RwSignal<HashMap<String, usize>>,
    progress: RwSignal<BatchProgress>,
    progress_pct: RwSignal<u32>,
    stats: RwSignal<BatchRuntimeStats>,
    continue_on_error: bool,
) {
    if csv_state.get().mapping.is_none() {
        progress.update(|p| p.status = "Confirm CSV mapping first.".to_string());
        return;
    }

    queue.update(|queue| {
        while let Some(page) = queue.dequeue_next_page() {
            batch_state.update(|state| state.add_images(page));
        }
    });
    let selected_ids = batch_state.get().selected_ids();
    if selected_ids.is_empty() {
        progress.update(|p| p.status = "No matched images to process.".to_string());
        return;
    }

    let files = files_by_id.get();
    let previews = preview_urls.get();
    let source_names = source_name_by_id.get();
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
    face_count_by_id.set(HashMap::new());
    batch_state.update(|state| selected_ids.iter().for_each(|id| state.mark_processing(id)));
    stats.update(BatchRuntimeStats::reset);
    progress_pct.set(0);
    progress.update(|p| {
        p.start(
            selected_ids.len(),
            format!("Processing {} CSV-matched image(s)...", selected_ids.len()),
        )
    });

    spawn_local(async move {
        let ctx = CsvProcessCtx {
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
                record_csv_failure(ctx, &id, 0, "Missing source file.", "Missing source file.");
                if !continue_on_error {
                    break;
                }
                continue;
            };
            let source_name = source_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| file.name());
            progress.update(|p| {
                p.status = format!("CSV processing {}/{}: {source_name}", index + 1, total)
            });

            let dimensions = match decode_image_dimensions(&file).await {
                Ok(dimensions) => dimensions,
                Err(error) => {
                    record_csv_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Decode failed: {source_name}"),
                        format!("Decode failed for {source_name}: {error}"),
                    );
                    if !continue_on_error {
                        break;
                    }
                    continue;
                }
            };
            if let Err(error) = validate_image_meta(
                ImageMeta {
                    file_name: &source_name,
                    mime_type: &file.type_(),
                    file_size_bytes: file.size() as u64,
                    dimensions,
                },
                validation,
            ) {
                record_csv_failure(
                    ctx,
                    &id,
                    elapsed_ms_since(start_ms),
                    format!("Validation failed: {source_name}"),
                    format!("Validation failed for {source_name}: {error}"),
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
                    record_csv_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Detection failed: {source_name}"),
                        format!("Detection failed for {source_name}: {error}"),
                    );
                    if !continue_on_error {
                        break;
                    }
                    continue;
                }
            };
            let Some(face) = faces.first() else {
                record_csv_failure(
                    ctx,
                    &id,
                    elapsed_ms_since(start_ms),
                    format!("No face found: {source_name}"),
                    format!("CSV found no crop target for {source_name}."),
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
                record_csv_failure(
                    ctx,
                    &id,
                    elapsed_ms_since(start_ms),
                    format!("Preview URL failed: {source_name}"),
                    format!("Could not create source URL for {source_name}."),
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
                    record_csv_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Crop failed: {source_name}"),
                        format!("CSV crop failed for {source_name}: {error}"),
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
                    record_csv_failure(
                        ctx,
                        &id,
                        elapsed_ms_since(start_ms),
                        format!("Preview failed: {source_name}"),
                        format!("CSV crop preview failed for {source_name}: {error}"),
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
            face_count_by_id.update(|map| {
                map.insert(id.clone(), faces.len());
            });
            batch_state.update(|state| state.mark_processed(&id));
            progress.update(|p| {
                p.record_result(true);
                p.status = format!(
                    "CSV processed {}/{}: {} ({} face(s))",
                    index + 1,
                    total,
                    source_name,
                    faces.len()
                );
                progress_pct.set(u32::from(p.percent()));
            });
            stats.update(|s| {
                s.record_image(elapsed_ms_since(start_ms), faces.len() as u32, true);
                s.push_log(format!(
                    "CSV processed {source_name}: {} face(s).",
                    faces.len()
                ));
            });
        }

        progress.update(|p| {
            let status = format!(
                "CSV batch complete: {} processed, {} failed.",
                p.processed.saturating_sub(p.failed),
                p.failed
            );
            p.complete(status);
            progress_pct.set(u32::from(p.percent()));
        });
    });
}

#[derive(Clone, Copy)]
struct CsvProcessCtx {
    progress: RwSignal<BatchProgress>,
    stats: RwSignal<BatchRuntimeStats>,
    batch_state: RwSignal<BatchCoreState>,
    progress_pct: RwSignal<u32>,
}

fn record_csv_failure(
    ctx: CsvProcessCtx,
    id: &str,
    elapsed_ms: u64,
    status: impl Into<String>,
    log: impl Into<String>,
) {
    let CsvProcessCtx {
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

pub(super) fn download_csv_zip(
    settings: RwSignal<ProcessingSettings>,
    csv_state: RwSignal<CsvCoreState>,
    batch_state: RwSignal<BatchCoreState>,
    source_name_by_id: RwSignal<HashMap<String, String>>,
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
        progress.update(|p| p.status = "No processed CSV crops to export.".to_string());
        return;
    }

    let csv = csv_state.get();
    let settings_snapshot = settings.get();
    let source_names = source_name_by_id.get();
    let outputs_snapshot = outputs.get();
    let timestamp_ms = current_timestamp_ms();
    let export_template = csv_template_display(&settings_snapshot.naming_template);
    let zip_name = format!("face-crops-{}.zip", current_utc_timestamp_token());
    spawn_local(async move {
        let mut entries = Vec::new();
        for (index, id) in ids.into_iter().enumerate() {
            let Some(output) = outputs_snapshot.get(&id).cloned() else {
                continue;
            };
            let source_name = source_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| batch_file_label(&id).to_string());
            let csv_output_name = csv.output_name_for_file(&source_name);
            let generated = CsvCoreState::generate_export_filename(CsvExportNameContext {
                template: &export_template,
                csv_output_name: csv_output_name.as_deref(),
                original_file_name: &source_name,
                face_index: index,
                timestamp_ms,
                output_width: settings_snapshot.output_width,
                output_height: settings_snapshot.output_height,
                output_format: &settings_snapshot.output_format,
            });
            let final_name = normalize_export_filename_for_mime(&generated, &output.mime_type);
            if !validate_export_filename_for_mime(&final_name, &output.mime_type) {
                stats.update(|s| {
                    s.push_log(format!("CSV ZIP skipped invalid file name: {final_name}"))
                });
                continue;
            }
            entries.push((final_name, output.bytes));
        }

        if entries.is_empty() {
            progress.update(|p| p.complete("No mapped cropped outputs available for ZIP export."));
            return;
        }
        match build_zip_bytes(&entries) {
            Ok(bytes) => match download_bytes(&zip_name, "application/zip", &bytes) {
                Ok(()) => {
                    let count = entries.len();
                    progress
                        .update(|p| p.complete(format!("CSV ZIP exported: {zip_name} ({count})")));
                    stats.update(|s| {
                        s.push_log(format!("Exported CSV ZIP {zip_name} with {count} file(s)."))
                    });
                }
                Err(error) => {
                    progress.update(|p| p.complete(format!("CSV ZIP download failed: {error}")))
                }
            },
            Err(error) => progress.update(|p| p.complete(format!("CSV ZIP build failed: {error}"))),
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn reset_csv_workflow(
    csv_state: RwSignal<CsvCoreState>,
    batch_state: RwSignal<BatchCoreState>,
    queue: RwSignal<BatchQueueState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    source_name_by_id: RwSignal<HashMap<String, String>>,
    outputs: RwSignal<HashMap<String, ProcessedImageOutput>>,
    face_count_by_id: RwSignal<HashMap<String, usize>>,
    progress: RwSignal<BatchProgress>,
    progress_pct: RwSignal<u32>,
    stats: RwSignal<BatchRuntimeStats>,
    file_path_column: RwSignal<String>,
    file_name_column: RwSignal<String>,
    mapping_confirmed: RwSignal<bool>,
) {
    revoke_preview_urls(&preview_urls.get());
    for output in outputs.get().values() {
        revoke_object_url(&output.preview_url);
    }
    csv_state.set(CsvCoreState::default());
    batch_state.set(BatchCoreState::default());
    queue.set(BatchQueueState::default());
    files_by_id.set(HashMap::new());
    preview_urls.set(HashMap::new());
    source_name_by_id.set(HashMap::new());
    outputs.set(HashMap::new());
    face_count_by_id.set(HashMap::new());
    progress.set(BatchProgress::default());
    progress_pct.set(0);
    stats.set(BatchRuntimeStats::default());
    file_path_column.set(String::new());
    file_name_column.set(String::new());
    mapping_confirmed.set(false);
}

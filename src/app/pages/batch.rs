use super::{
    AppState, BatchCoreState, BatchImageGalleryPanel, BatchProgress, BatchQueueState,
    BatchRuntimeStats, BatchUploadCard, ClassAttribute, CollectView, CropSettingsPanel,
    DetectedFace, DetectionRetryPolicy, ElementChild, Get, GlobalAttributes, HashMap,
    HtmlInputElement, ImageMeta, ImageValidationConfig, IntoAny, IntoView, JsFuture,
    MemoryIndicatorLevel, OnAttribute, OutputSettingsBatchPanel, PreprocessingSettingsPanel,
    PropAttribute, RwSignal, Set, Signal, ThemeToggleButton, Update,
    apply_detection_quality_filters, batch_file_label, build_memory_indicator, build_zip_bytes,
    click_element_by_id, component, current_timestamp_ms, current_utc_timestamp_token,
    decode_image_dimensions, detect_faces_with_worker, download_bytes, elapsed_ms_since,
    event_target, event_target_checked, event_target_value, export_saved_settings_json,
    file_to_bytes, import_saved_settings_json, list_saved_setting_names,
    load_named_processing_settings, navigate_to, normalize_export_filename_for_mime, now_ms,
    parse_max_retries, render_naming_template, revoke_object_url, save_named_processing_settings,
    use_context, validate_export_filename_for_mime, validate_image_meta, view,
};

#[allow(clippy::too_many_lines)]
#[component]
pub(crate) fn BatchPage() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let batch_state = RwSignal::new(BatchCoreState::default());
    let batch_queue = RwSignal::new(BatchQueueState::default());
    let batch_files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let batch_preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let batch_progress = RwSignal::new(BatchProgress::default());
    let batch_perf = RwSignal::new(BatchRuntimeStats::default());
    let batch_preview_filename = RwSignal::new(String::new());
    let settings_name_input = RwSignal::new(String::new());
    let selected_recent_setting = RwSignal::new(String::new());
    let recent_settings = RwSignal::new(list_saved_setting_names());
    let continue_on_error = RwSignal::new(true);
    let reduced_resolution = RwSignal::new(false);
    let retry_attempts = RwSignal::new("2".to_string());
    let rust_progress_percent = Signal::derive(move || batch_progress.get().percent());
    let rust_progress_status = Signal::derive(move || batch_progress.get().status);
    let rust_progress_running = Signal::derive(move || batch_progress.get().running);
    let batch_busy = Signal::derive(move || batch_progress.get().running);
    let rust_chunk_count =
        Signal::derive(move || batch_state.get().build_work_plan(128).chunks.len());
    let rust_lazy_queued_pages =
        Signal::derive(move || batch_queue.get().queued_pages_count().to_string());
    let rust_lazy_queued_files =
        Signal::derive(move || batch_queue.get().queued_files_count().to_string());
    let total_faces_detected =
        Signal::derive(move || batch_perf.get().total_faces_detected.to_string());
    let success_rate = Signal::derive(move || format!("{}%", batch_perf.get().success_rate_pct()));
    let avg_processing_time =
        Signal::derive(move || format!("{}ms", batch_perf.get().avg_processing_time_ms()));
    let images_processed = Signal::derive(move || batch_perf.get().images_processed.to_string());
    let batch_logs = Signal::derive(move || batch_perf.get().logs);
    let batch_error_logs = Signal::derive(move || {
        batch_perf
            .get()
            .logs
            .into_iter()
            .filter(|entry| {
                let lower = entry.to_lowercase();
                lower.contains("error") || lower.contains("failed")
            })
            .collect::<Vec<_>>()
    });
    let has_images = Signal::derive(move || batch_state.get().total_count() > 0);
    let has_selected_images = Signal::derive(move || batch_state.get().has_selected_images());
    let rust_estimated_preview_mib = Signal::derive(move || {
        let selected = batch_state.get().selected_count().min(128);
        let bytes = BatchCoreState::estimate_preview_memory_bytes(640, 640, selected);
        #[allow(clippy::cast_precision_loss)]
        {
            format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
        }
    });
    let rust_memory_indicator_text = Signal::derive(move || {
        let state = batch_state.get();
        let images = state.total_count();
        let processed = state
            .images
            .values()
            .filter(|image| image.processed)
            .count();
        build_memory_indicator(images, processed, 0).text
    });
    let rust_memory_indicator_level = Signal::derive(move || {
        let state = batch_state.get();
        let images = state.total_count();
        let processed = state
            .images
            .values()
            .filter(|image| image.processed)
            .count();
        match build_memory_indicator(images, processed, 0).level {
            MemoryIndicatorLevel::Hidden => "hidden",
            MemoryIndicatorLevel::Show => "normal",
            MemoryIndicatorLevel::Warning => "warning",
            MemoryIndicatorLevel::Critical => "critical",
        }
        .to_string()
    });

    view! {
        <div class="app-shell">
            <header class="app-header">
                <div class="title-row">
                    <h1><a href="/">"Face Crop Forge"</a></h1>
                    <div class="header-actions">
                        <button
                            type="button"
                            id="singleImageModeBtn"
                            class="ghost-btn"
                            title="Switch to single image mode"
                            on:click=move |_| navigate_to("/single")
                        >
                            <span>"Single Image"</span>
                        </button>
                        <button
                            type="button"
                            id="csvBatchModeBtn"
                            class="ghost-btn"
                            title="Switch to CSV batch mode"
                            on:click=move |_| navigate_to("/csv")
                        >
                            <span>"CSV Batch"</span>
                        </button>
                        <ThemeToggleButton id="darkModeBtn" />
                    </div>
                </div>

                <BatchUploadCard
                    state=batch_state
                    queue=batch_queue
                    progress=batch_progress
                    files_by_id=batch_files_by_id
                    preview_urls=batch_preview_urls
                    busy=batch_busy
                />

                <div class="batch-controls">
                    <button
                        type="button"
                        id="processAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = batch_state.get().build_work_plan(128);
                            if plan.selected_total == 0 {
                                batch_progress.update(|p| p.complete("No images selected"));
                                return;
                            }
                            let policy = DetectionRetryPolicy {
                                max_retries: parse_max_retries(Some(&retry_attempts.get())),
                                continue_on_error: continue_on_error.get(),
                                reduced_resolution: reduced_resolution.get(),
                            };
                            let validation = ImageValidationConfig::default();
                            let selected_ids = batch_state.get().selected_ids();
                            let files_by_id = batch_files_by_id.get();
                            batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            batch_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "Planned {} images in {} chunk(s) of up to {}",
                                        plan.selected_total,
                                        plan.chunks.len(),
                                        plan.chunk_size
                                    ),
                                );
                            });
                            let batch_state_for_run = batch_state;
                            let batch_progress_for_run = batch_progress;
                            let batch_perf_for_run = batch_perf;
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "Batch failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            ),
                                            format!("Missing file payload for id {id}.")
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    };

                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            record_failed_image!(
                                                batch_progress_for_run,
                                                batch_perf_for_run,
                                                batch_state_for_run,
                                                &id,
                                                0,
                                                format!(
                                                    "Batch failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                ),
                                                format!("Decode failed for {}: {error}", file.name())
                                            );
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                            continue;
                                        }
                                    };

                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    batch_progress_for_run.update(|p| {
                                        p.status = format!(
                                            "Batch processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "Batch failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("Validation failed for {file_name}: {message}")
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    }

                                    let run_start_ms = now_ms();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    let mut attempts = 0_u32;
                                    for attempt in 0..=policy.max_retries {
                                        attempts = attempt + 1;
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => {
                                                last_error = error;
                                            }
                                        }
                                    }
                                    let elapsed_ms = elapsed_ms_since(run_start_ms);

                                    if let Some(faces) = success_faces {
                                        #[allow(clippy::cast_possible_truncation)]
                                        let face_count = faces.len() as u32;
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(true);
                                            p.status = format!(
                                                "Batch processed {}/{}: {} ({} face(s))",
                                                index + 1,
                                                total,
                                                file_name,
                                                face_count
                                            );
                                        });
                                        batch_perf_for_run.update(|stats| {
                                            stats.record_image(elapsed_ms, face_count, true);
                                            stats.push_log(format!(
                                                "Processed {} in {}ms ({} attempt(s), {}x{}, {} face(s)).",
                                                file_name,
                                                elapsed_ms,
                                                attempts,
                                                dimensions.width,
                                                dimensions.height,
                                                face_count
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_processed(&id));
                                    } else {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            elapsed_ms,
                                            format!(
                                                "Batch failed {}/{}: {} ({last_error})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!(
                                                "Failed {} after {} attempt(s): {}",
                                                file_name, attempts, last_error
                                            )
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                    }
                                }
                                batch_progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("Stopped early due to failure (continue on error disabled)");
                                    } else {
                                        p.complete("Batch run completed with real worker detection");
                                    }
                                });
                            });
                        }
                    >
                        "Process All"
                    </button>
                    <button
                        type="button"
                        id="processSelectedBtn"
                        disabled=move || !has_selected_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = batch_state.get().build_work_plan(128);
                            if plan.selected_total == 0 {
                                batch_progress.update(|p| p.complete("No images selected"));
                                return;
                            }
                            let policy = DetectionRetryPolicy {
                                max_retries: parse_max_retries(Some(&retry_attempts.get())),
                                continue_on_error: continue_on_error.get(),
                                reduced_resolution: reduced_resolution.get(),
                            };
                            let validation = ImageValidationConfig::default();
                            let selected_ids = batch_state.get().selected_ids();
                            let files_by_id = batch_files_by_id.get();
                            batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            batch_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "Processing selected set in {} chunk(s) of up to {}",
                                        plan.chunks.len(),
                                        plan.chunk_size
                                    ),
                                );
                            });
                            let batch_state_for_run = batch_state;
                            let batch_progress_for_run = batch_progress;
                            let batch_perf_for_run = batch_perf;
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "Batch selected failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            ),
                                            format!("Missing file payload for id {id}.")
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    };

                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            record_failed_image!(
                                                batch_progress_for_run,
                                                batch_perf_for_run,
                                                batch_state_for_run,
                                                &id,
                                                0,
                                                format!(
                                                    "Batch selected failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                ),
                                                format!("Decode failed for {}: {error}", file.name())
                                            );
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                            continue;
                                        }
                                    };

                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    batch_progress_for_run.update(|p| {
                                        p.status = format!(
                                            "Batch selected processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "Batch selected failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("Validation failed for {file_name}: {message}")
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    }

                                    let run_start_ms = now_ms();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    let mut attempts = 0_u32;
                                    for attempt in 0..=policy.max_retries {
                                        attempts = attempt + 1;
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => {
                                                last_error = error;
                                            }
                                        }
                                    }
                                    let elapsed_ms = elapsed_ms_since(run_start_ms);

                                    if let Some(faces) = success_faces {
                                        #[allow(clippy::cast_possible_truncation)]
                                        let face_count = faces.len() as u32;
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(true);
                                            p.status = format!(
                                                "Batch selected processed {}/{}: {} ({} face(s))",
                                                index + 1,
                                                total,
                                                file_name,
                                                face_count
                                            );
                                        });
                                        batch_perf_for_run.update(|stats| {
                                            stats.record_image(elapsed_ms, face_count, true);
                                            stats.push_log(format!(
                                                "Selected run processed {file_name} in {elapsed_ms}ms ({attempts} attempt(s), {face_count} face(s))."
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_processed(&id));
                                    } else {
                                        record_failed_image!(
                                            batch_progress_for_run,
                                            batch_perf_for_run,
                                            batch_state_for_run,
                                            &id,
                                            elapsed_ms,
                                            format!(
                                                "Batch selected failed {}/{}: {} ({last_error})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!(
                                                "Selected run failed {} after {} attempt(s): {}",
                                                file_name, attempts, last_error
                                            )
                                        );
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                    }
                                }
                                batch_progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("Selected run stopped early on first failure");
                                    } else {
                                        p.complete("Selected run completed with real worker detection");
                                    }
                                });
                            });
                        }
                    >
                        "Process Selected"
                    </button>
                    <button
                        type="button"
                        id="clearAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_state.update(|s| s.set_images(Vec::new()));
                            batch_queue.set(BatchQueueState::default());
                            batch_files_by_id.set(HashMap::new());
                            for url in batch_preview_urls.get().values() {
                                revoke_object_url(url);
                            }
                            batch_preview_urls.set(HashMap::new());
                            batch_progress.update(crate::batch_export::BatchProgress::reset);
                            batch_perf.update(crate::batch_core::BatchRuntimeStats::reset);
                            batch_preview_filename.set(String::new());
                        }
                    >
                        "Clear All"
                    </button>
                    <button
                        type="button"
                        id="downloadAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            let source_ids = {
                                let batch = batch_state.get();
                                if batch.has_selected_images() {
                                    batch.selected_ids()
                                } else {
                                    batch.images.values().map(|img| img.id.clone()).collect::<Vec<_>>()
                                }
                            };
                            if source_ids.is_empty() {
                                batch_progress.update(|p| p.complete("No images available for export"));
                                return;
                            }
                            let file_map = batch_files_by_id.get();
                            let export_settings = settings.get();
                            let timestamp_ms = current_timestamp_ms();
                            let zip_name =
                                format!("face-crops-{}.zip", current_utc_timestamp_token());
                            let progress_for_download = batch_progress;
                            let stats_for_download = batch_perf;
                            let preview_for_download = batch_preview_filename;
                            leptos::task::spawn_local(async move {
                                let mut zip_entries = Vec::new();
                                for (idx, id) in source_ids.iter().enumerate() {
                                    let Some(file) = file_map.get(id).cloned() else {
                                        continue;
                                    };
                                    let bytes = match file_to_bytes(&file).await {
                                        Ok(bytes) => bytes,
                                        Err(error) => {
                                            progress_for_download.update(|p| {
                                                p.complete(format!("Batch ZIP failed: {error}"));
                                            });
                                            stats_for_download.update(|s| {
                                                s.push_log(format!("Batch ZIP read failed for {}: {error}", file.name()));
                                            });
                                            return;
                                        }
                                    };
                                    let entry_name = render_naming_template(
                                        &export_settings.naming_template,
                                        &file.name(),
                                        idx,
                                        export_settings.output_width,
                                        export_settings.output_height,
                                        timestamp_ms,
                                    );
                                    let mime_type = file.type_();
                                    let final_name =
                                        normalize_export_filename_for_mime(&entry_name, &mime_type);
                                    if !validate_export_filename_for_mime(&final_name, &mime_type) {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "Batch ZIP skipped invalid filename/mime pair: {final_name} ({mime_type})"
                                            ));
                                        });
                                        continue;
                                    }
                                    zip_entries.push((final_name, bytes));
                                }
                                if zip_entries.is_empty() {
                                    progress_for_download
                                        .update(|p| p.complete("No binary outputs available for ZIP export"));
                                    return;
                                }
                                let first_name = zip_entries[0].0.clone();
                                let zip_bytes = match build_zip_bytes(&zip_entries) {
                                    Ok(bytes) => bytes,
                                    Err(error) => {
                                        progress_for_download
                                            .update(|p| p.complete(format!("Batch ZIP build failed: {error}")));
                                        stats_for_download
                                            .update(|s| s.push_log(format!("Batch ZIP build failed: {error}")));
                                        return;
                                    }
                                };
                                if let Err(error) =
                                    download_bytes(&zip_name, "application/zip", &zip_bytes)
                                {
                                    progress_for_download.update(|p| {
                                        p.complete(format!("Batch ZIP download failed: {error}"));
                                    });
                                    stats_for_download.update(|s| {
                                        s.push_log(format!("Batch ZIP download failed: {error}"));
                                    });
                                    return;
                                }
                                preview_for_download.set(first_name.clone());
                                stats_for_download.update(|s| {
                                    s.push_log(format!(
                                        "Exported ZIP {} with {} file(s).",
                                        zip_name,
                                        zip_entries.len()
                                    ));
                                });
                                progress_for_download.update(|p| {
                                    p.complete(format!(
                                        "Batch ZIP exported: {} ({})",
                                        zip_name,
                                        zip_entries.len()
                                    ));
                                });
                            });
                        }
                    >
                        "Download All Results"
                    </button>
                </div>
            </header>

            <div class="app-body">
                <aside class="control-panel">
                    <div class="control-scroll">
                        <div class="workflow-tools">
                            <h3 class="collapsible-header">
                                "Professional Workflow Tools"
                                <span class="collapse-icon">"▼"</span>
                            </h3>
                            <div class="collapsible-content">
                                <div class="workflow-section">
                                    <h4>"Processing Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Total Faces Detected"</span>
                                            <span class="stat-value" id="totalFacesDetected">{total_faces_detected}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Success Rate"</span>
                                            <span class="stat-value" id="successRate">{success_rate}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Avg. Processing Time"</span>
                                            <span class="stat-value" id="avgProcessingTime">{avg_processing_time}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Images Processed"</span>
                                            <span class="stat-value" id="imagesProcessed">{images_processed}</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Settings Management"</h4>
                                    <div class="settings-management">
                                        <div class="recent-settings">
                                            <label for="recentSettingsDropdown">"Recent Settings"</label>
                                            <select
                                                id="recentSettingsDropdown"
                                                prop:value=move || selected_recent_setting.get()
                                                on:change=move |ev| selected_recent_setting.set(event_target_value(&ev))
                                            >
                                                <option value="">"Select a saved configuration..."</option>
                                                {move || recent_settings.get().into_iter().map(|name| {
                                                    view! { <option value=name.clone()>{name.clone()}</option> }
                                                }).collect_view()}
                                            </select>
                                            <button
                                                type="button"
                                                id="loadRecentBtn"
                                                class="small-btn"
                                                on:click=move |_| {
                                                    let selected = selected_recent_setting.get();
                                                    if selected.is_empty() {
                                                        batch_progress.update(|p| p.complete("Choose a saved configuration first"));
                                                        return;
                                                    }
                                                    if let Some(saved) = load_named_processing_settings(&selected) {
                                                        settings.set(saved);
                                                        batch_progress.update(|p| p.complete(format!("Loaded settings profile '{selected}'")));
                                                    } else {
                                                        batch_progress.update(|p| p.complete(format!("Settings profile '{selected}' not found")));
                                                    }
                                                }
                                            >
                                                "Load"
                                            </button>
                                        </div>

                                        <div class="settings-actions">
                                            <label for="settingsName" class="sr-only">"Configuration name"</label>
                                            <input
                                                type="text"
                                                id="settingsName"
                                                placeholder="Configuration name..."
                                                maxlength="50"
                                                prop:value=move || settings_name_input.get()
                                                on:input=move |ev| settings_name_input.set(event_target_value(&ev))
                                            />
                                            <button
                                                type="button"
                                                id="saveSettingsBtn"
                                                class="save-btn"
                                                on:click=move |_| {
                                                    let name = settings_name_input.get();
                                                    match save_named_processing_settings(&name, &settings.get()) {
                                                        Ok(()) => {
                                                            let names = list_saved_setting_names();
                                                            recent_settings.set(names);
                                                            selected_recent_setting.set(name.clone());
                                                            batch_progress.update(|p| p.complete(format!("Saved settings profile '{name}'")));
                                                        }
                                                        Err(error) => {
                                                            batch_progress.update(|p| p.complete(format!("Save settings failed: {error}")));
                                                        }
                                                    }
                                                }
                                            >
                                                "Save Settings"
                                            </button>
                                            <button
                                                type="button"
                                                id="exportSettingsBtn"
                                                class="export-btn"
                                                on:click=move |_| {
                                                    match export_saved_settings_json() {
                                                        Ok(json) => {
                                                            if let Err(error) = download_bytes(
                                                                "face-crop-settings.json",
                                                                "application/json",
                                                                json.as_bytes(),
                                                            ) {
                                                                batch_progress.update(|p| p.complete(format!("Export settings failed: {error}")));
                                                            } else {
                                                                batch_progress.update(|p| p.complete("Exported settings JSON"));
                                                            }
                                                        }
                                                        Err(error) => {
                                                            batch_progress.update(|p| p.complete(format!("Export settings failed: {error}")));
                                                        }
                                                    }
                                                }
                                            >
                                                "Export JSON"
                                            </button>
                                            <button
                                                type="button"
                                                id="importSettingsBtn"
                                                class="import-btn"
                                                on:click=move |_| click_element_by_id("importSettingsFile")
                                            >
                                                "Import JSON"
                                            </button>
                                            <label for="importSettingsFile" class="sr-only">"Import settings file"</label>
                                            <input
                                                type="file"
                                                id="importSettingsFile"
                                                accept=".json"
                                                class="hidden"
                                                on:change=move |ev| {
                                                    let input: HtmlInputElement = event_target(&ev);
                                                    let Some(files) = input.files() else {
                                                        return;
                                                    };
                                                    let Some(file) = files.get(0) else {
                                                        return;
                                                    };
                                                    let progress_for_import = batch_progress;
                                                    let recent_for_import = recent_settings;
                                                    leptos::task::spawn_local(async move {
                                                        let Ok(js_text) = JsFuture::from(file.text()).await else {
                                                            progress_for_import.update(|p| p.complete("Import settings failed: could not read file"));
                                                            return;
                                                        };
                                                        let Some(text) = js_text.as_string() else {
                                                            progress_for_import.update(|p| p.complete("Import settings failed: invalid file content"));
                                                            return;
                                                        };
                                                        match import_saved_settings_json(&text) {
                                                            Ok(count) => {
                                                                recent_for_import.set(list_saved_setting_names());
                                                                progress_for_import.update(|p| p.complete(format!("Imported {count} settings profile(s)")));
                                                            }
                                                            Err(error) => {
                                                                progress_for_import.update(|p| p.complete(format!("Import settings failed: {error}")));
                                                            }
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Export & Reporting"</h4>
                                    <div class="export-tools">
                                        <button type="button" id="exportLogBtn" class="export-btn">"Export Processing Log"</button>
                                        <button type="button" id="exportCsvBtn" class="export-btn">"Export CSV Report"</button>
                                        <button type="button" id="clearStatsBtn" class="reset-button">"Clear Statistics"</button>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Performance Settings"</h4>
                                    <div class="performance-settings">
                                        <div class="setting-group">
                                            <label>
                                                <input
                                                    type="checkbox"
                                                    id="continueOnError"
                                                    prop:checked=move || continue_on_error.get()
                                                    on:change=move |ev| continue_on_error.set(event_target_checked(&ev))
                                                />
                                                " Continue on Error"
                                            </label>
                                            <div class="setting-help">"Continue processing other images when one fails"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label>
                                                <input
                                                    type="checkbox"
                                                    id="reducedResolution"
                                                    prop:checked=move || reduced_resolution.get()
                                                    on:change=move |ev| reduced_resolution.set(event_target_checked(&ev))
                                                />
                                                " Fast Detection Mode"
                                            </label>
                                            <div class="setting-help">"Process at 50% resolution for faster detection"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label>
                                                <input type="checkbox" id="enableWebWorkers" checked />
                                                " Use Web Workers"
                                            </label>
                                            <div class="setting-help">"Prevent UI blocking during processing"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label for="memoryManagement">"Memory Management"</label>
                                            <select id="memoryManagement">
                                                <option value="auto">"Auto Cleanup"</option>
                                                <option value="aggressive">"Aggressive Cleanup"</option>
                                                <option value="manual">"Manual Only"</option>
                                            </select>
                                        </div>
                                        <div class="setting-group">
                                            <label for="retryAttempts">"Retry Attempts"</label>
                                            <input
                                                type="number"
                                                id="retryAttempts"
                                                min="0"
                                                max="5"
                                                prop:value=move || retry_attempts.get()
                                                on:input=move |ev| retry_attempts.set(event_target_value(&ev))
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Processing Log"</h4>
                                    <div class="processing-log" id="processingLog">
                                        {move || batch_logs.get().into_iter().map(|entry| view! { <div class="log-entry">{entry}</div> }).collect_view()}
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Rust Progress Status"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Running"</span>
                                            <span class="stat-value">{move || if rust_progress_running.get() { "Yes" } else { "No" }}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Progress"</span>
                                            <span class="stat-value">{move || format!("{}%", rust_progress_percent.get())}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Chunk Count"</span>
                                            <span class="stat-value">{move || rust_chunk_count.get().to_string()}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Preview Buffer"</span>
                                            <span class="stat-value">{rust_estimated_preview_mib}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Preview Export Name"</span>
                                            <span class="stat-value">{move || batch_preview_filename.get()}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Queued Pages"</span>
                                            <span class="stat-value">{rust_lazy_queued_pages}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Queued Files"</span>
                                            <span class="stat-value">{rust_lazy_queued_files}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Memory Indicator"</span>
                                            <span class="stat-value">{rust_memory_indicator_text}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Memory Level"</span>
                                            <span class="stat-value">{rust_memory_indicator_level}</span>
                                        </div>
                                    </div>
                                    <div class="setting-help">{rust_progress_status}</div>
                                </div>

                                <div class="workflow-section">
                                    <h4 class="collapsible-header error-log-header">
                                        "Error Details"
                                        <span class="collapse-icon">"▼"</span>
                                    </h4>
                                    <div class="collapsible-content error-log-panel">
                                        <div class="error-log" id="errorLog">
                                            {move || {
                                                let errors = batch_error_logs.get();
                                                if errors.is_empty() {
                                                    view! { <div class="log-entry">"No errors detected"</div> }
                                                        .into_any()
                                                } else {
                                                    errors
                                                        .into_iter()
                                                        .map(|entry| view! { <div class="log-entry">{entry}</div> })
                                                        .collect_view()
                                                        .into_any()
                                                }
                                            }}
                                        </div>
                                        <div class="error-actions">
                                            <button type="button" id="clearErrorsBtn" class="reset-button">"Clear Errors"</button>
                                            <button type="button" id="exportErrorsBtn" class="export-btn">"Export Error Log"</button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <CropSettingsPanel />
                        <PreprocessingSettingsPanel />
                        <OutputSettingsBatchPanel />
                    </div>
                </aside>

                <main class="workspace">
                    <div class="workspace-scroll">
                        <BatchImageGalleryPanel
                            state=batch_state
                            preview_urls=batch_preview_urls
                            busy=batch_busy
                        />

                        <div class="canvas-container hidden workspace-card" id="canvasContainer">
                            <h3 id="canvasTitle">"Current Image Preview"</h3>
                            <div class="canvas-wrapper" id="canvasWrapper">
                                <div class="canvas-panel" id="originalPanel">
                                    <h4 class="panel-title">"Original"</h4>
                                    <div class="panel-content">
                                        <canvas id="inputCanvas"></canvas>
                                        <div id="faceOverlays" class="face-overlays"></div>
                                    </div>
                                </div>
                                <div class="canvas-panel hidden" id="processedPanel">
                                    <h4 class="panel-title">"Processed"</h4>
                                    <div class="panel-content">
                                        <canvas id="outputCanvas"></canvas>
                                    </div>
                                </div>
                            </div>
                            <div class="face-controls">
                                <div id="faceCounter">
                                    <span>"Detected faces: " <span id="faceCount">"0"</span></span>
                                    <span class="separator">"•"</span>
                                    <span>"Selected: " <span id="selectedFaceCount">"0"</span></span>
                                </div>
                                <div class="face-selection-controls">
                                    <button type="button" id="selectAllFacesBtn" class="small-btn">"Select All Faces"</button>
                                    <button type="button" id="selectNoneFacesBtn" class="small-btn">"Select None"</button>
                                    <button type="button" id="detectFacesBtn" class="small-btn primary">"Detect Faces"</button>
                                </div>
                            </div>
                        </div>

                        <div class="cropped-faces workspace-card" id="croppedFaces">
                            <h3>"Cropped Faces:"</h3>
                            <div id="croppedContainer">
                                {move || {
                                    let urls = batch_preview_urls.get();
                                    let mut processed_ids = batch_state
                                        .get()
                                        .images
                                        .values()
                                        .filter(|img| img.processed)
                                        .map(|img| img.id.clone())
                                        .collect::<Vec<_>>();
                                    processed_ids.sort();
                                    processed_ids
                                        .into_iter()
                                        .map(|id| {
                                            let label = batch_file_label(&id).to_string();
                                            let url = urls.get(&id).cloned();
                                            view! {
                                                <div class="cropped-face">
                                                    {url.map(|src| {
                                                        view! {
                                                            <img src=src alt="Processed preview" />
                                                        }
                                                    })}
                                                    <div class="setting-help">{format!("Processed: {label}")}</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </div>

                        <div class="status workspace-card" id="status">"Ready to load image"</div>
                    </div>
                </main>
            </div>

        </div>
    }
}

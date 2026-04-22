use super::{
    AppShell, AppState, BatchCoreState, BatchImageGalleryPanel, BatchProgress, BatchQueueState,
    BatchRuntimeStats, ClassAttribute, CollectView, CropSettingsPanel, CsvCoreState,
    CsvExportNameContext, CsvImageUploadCard, CsvUploadCard, DetectedFace, Effect, ElementChild,
    Get, GlobalAttributes, HashMap, ImageMeta, ImageValidationConfig, IntoAny, IntoView,
    OnAttribute, OutputSettingsCsvPanel, PreprocessingSettingsPanel, ProcessedImageOutput,
    RwSignal, Set, Signal, StyleAttribute, Update, apply_detection_quality_filters,
    batch_file_label, build_zip_bytes, clear_canvas, component, crop_face_bytes_from_source,
    current_timestamp_ms, current_utc_timestamp_token, decode_image_dimensions,
    detect_faces_with_worker, download_bytes, draw_source_image_to_canvas, elapsed_ms_since,
    event_target_value, mime_type_for_output_format, normalize_export_filename_for_mime, now_ms,
    object_url_for_bytes, object_url_for_file, revoke_object_url, use_context,
    validate_export_filename_for_mime, validate_image_meta, view,
};

#[allow(clippy::too_many_lines)]
#[component]
pub(crate) fn CsvPage() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let csv_state = RwSignal::new(CsvCoreState::default());
    let csv_batch_state = RwSignal::new(BatchCoreState::default());
    let csv_queue = RwSignal::new(BatchQueueState::default());
    let csv_files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let csv_preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let csv_processed_outputs = RwSignal::new(HashMap::<String, ProcessedImageOutput>::new());
    let csv_source_name_by_id = RwSignal::new(HashMap::<String, String>::new());
    let csv_current_image_id = RwSignal::new(None::<String>);
    let csv_current_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let csv_current_dimensions = RwSignal::new((0.0_f64, 0.0_f64));
    let csv_face_count_by_id = RwSignal::new(HashMap::<String, usize>::new());
    let csv_progress = RwSignal::new(BatchProgress::default());
    let csv_perf = RwSignal::new(BatchRuntimeStats::default());
    let csv_preview_filename = RwSignal::new(String::new());
    let mapping_confirmed = RwSignal::new(false);
    let file_path_column = RwSignal::new(String::new());
    let file_name_column = RwSignal::new(String::new());
    let headers = Signal::derive(move || csv_state.get().headers);
    let preview_rows = Signal::derive(move || csv_state.get().preview_rows(5));
    let can_confirm_mapping = Signal::derive(move || {
        csv_state
            .get()
            .can_confirm_mapping(&file_path_column.get(), &file_name_column.get())
    });
    let mapping_status = Signal::derive(move || {
        if mapping_confirmed.get() && csv_state.get().mapping.is_some() {
            "Mapping ready".to_string()
        } else {
            "Awaiting mapping".to_string()
        }
    });
    let total_rows = Signal::derive(move || csv_state.get().rows.len().to_string());
    let matched_images = Signal::derive(move || {
        let loaded = csv_batch_state.get().total_count();
        let queued = csv_queue.get().queued_files_count();
        (loaded + queued).to_string()
    });
    let has_images = Signal::derive(move || {
        csv_batch_state.get().total_count() + csv_queue.get().queued_files_count() > 0
    });
    let has_selected_images = Signal::derive(move || csv_batch_state.get().has_selected_images());
    let csv_progress_percent = Signal::derive(move || csv_progress.get().percent());
    let csv_progress_status = Signal::derive(move || csv_progress.get().status);
    let csv_progress_running = Signal::derive(move || csv_progress.get().running);
    let csv_busy = Signal::derive(move || csv_progress.get().running);
    let csv_total_faces = Signal::derive(move || csv_perf.get().total_faces_detected.to_string());
    let csv_success_rate =
        Signal::derive(move || format!("{}%", csv_perf.get().success_rate_pct()));
    let csv_avg_processing_time =
        Signal::derive(move || format!("{}ms", csv_perf.get().avg_processing_time_ms()));
    let csv_images_processed = Signal::derive(move || csv_perf.get().images_processed.to_string());
    let csv_logs = Signal::derive(move || csv_perf.get().logs);
    let csv_error_logs = Signal::derive(move || {
        csv_perf
            .get()
            .logs
            .into_iter()
            .filter(|entry| {
                let lower = entry.to_lowercase();
                lower.contains("error") || lower.contains("failed")
            })
            .collect::<Vec<_>>()
    });
    let csv_current_image_url = Signal::derive(move || {
        csv_current_image_id
            .get()
            .and_then(|id| csv_preview_urls.get().get(&id).cloned())
    });
    let csv_current_faces_label = Signal::derive(move || csv_current_faces.get().len().to_string());

    Effect::new(move |_| {
        if csv_state.get().mapping.is_none() {
            mapping_confirmed.set(false);
        }
    });

    Effect::new(move |_| {
        if let Some(url) = csv_current_image_url.get() {
            let dims = csv_current_dimensions;
            leptos::task::spawn_local(async move {
                if let Ok((w, h)) = draw_source_image_to_canvas("csvInputCanvas", &url).await {
                    dims.set((f64::from(w), f64::from(h)));
                }
            });
        } else {
            csv_current_dimensions.set((0.0, 0.0));
            clear_canvas("csvInputCanvas");
        }
    });

    view! {
        <AppShell title="CSV Workflow">
            <CsvUploadCard state=csv_state progress=csv_progress busy=csv_busy />

                <div class=move || {
                    if headers.get().is_empty() {
                        "csv-mapping hidden"
                    } else {
                        "csv-mapping"
                    }
                } id="csvMapping">
                    <h3>
                        "Column Mapping"
                        " ("
                        {mapping_status}
                        ")"
                    </h3>
                    <div class="mapping-controls">
                        <div class="mapping-group">
                            <label for="filePathColumn">"File Path Column:"</label>
                            <select id="filePathColumn" on:change=move |ev| file_path_column.set(event_target_value(&ev))>
                                <option value="">"Select column..."</option>
                                {move || headers.get().into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect_view()}
                            </select>
                        </div>
                        <div class="mapping-group">
                            <label for="fileNameColumn">"Output Name Column:"</label>
                            <select id="fileNameColumn" on:change=move |ev| file_name_column.set(event_target_value(&ev))>
                                <option value="">"Select column..."</option>
                                {move || headers.get().into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect_view()}
                            </select>
                        </div>
                        <button
                            type="button"
                            id="confirmMappingBtn"
                            class="primary-btn"
                            disabled=move || !can_confirm_mapping.get()
                            on:click=move |_| {
                                let path_col = file_path_column.get();
                                let name_col = file_name_column.get();
                                let mut applied = false;
                                csv_state.update(|s| {
                                    applied = s.apply_mapping(&path_col, &name_col);
                                });
                                mapping_confirmed.set(applied);
                            }
                        >
                            "Confirm Mapping"
                        </button>
                    </div>
                    <div class="csv-preview">
                        <h4>"CSV Preview (First 5 rows):"</h4>
                        <div id="csvPreviewTable">
                            <table class="csv-preview-table">
                                <thead>
                                    <tr>
                                        {move || headers.get().into_iter().map(|header| view! { <th>{header}</th> }).collect_view()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || preview_rows.get().into_iter().map(|row| {
                                        view! {
                                            <tr>
                                                {row.into_iter().map(|cell| view! { <td>{cell}</td> }).collect_view()}
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <div id="image-upload-card-container" class=move || if mapping_confirmed.get() { "" } else { "hidden" }>
                    <CsvImageUploadCard
                        state=csv_batch_state
                        queue=csv_queue
                        csv_state=csv_state
                        files_by_id=csv_files_by_id
                        preview_urls=csv_preview_urls
                        source_name_by_id=csv_source_name_by_id
                        progress=csv_progress
                        busy=csv_busy
                    />
                </div>

                <div class="batch-controls">
                    <button
                        type="button"
                        id="processAllBtn"
                        disabled=move || !has_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = csv_batch_state.get().build_work_plan(64);
                            if plan.selected_total == 0 {
                                csv_progress.update(|p| p.complete("No mapped images selected"));
                                return;
                            }
                            let selected_ids = csv_batch_state.get().selected_ids();
                            let files_by_id = csv_files_by_id.get();
                            let validation = ImageValidationConfig::default();
                            csv_batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            csv_processed_outputs.update(|outputs| {
                                for id in &selected_ids {
                                    if let Some(old) = outputs.remove(id) {
                                        revoke_object_url(&old.preview_url);
                                    }
                                }
                            });
                            csv_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "CSV queue: {} image(s) in {} chunk(s)",
                                        plan.selected_total,
                                        plan.chunks.len()
                                    ),
                                );
                            });
                            let batch_state_for_run = csv_batch_state;
                            let progress_for_run = csv_progress;
                            let stats_for_run = csv_perf;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            let output_mime_type =
                                mime_type_for_output_format(&settings_snapshot.output_format)
                                    .to_string();
                            let preview_urls_for_run = csv_preview_urls.get();
                            let processed_outputs_for_run = csv_processed_outputs;
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        record_failed_image!(
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "CSV failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            ),
                                            format!("CSV missing file payload for id {id}.")
                                        );
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                0,
                                                format!(
                                                    "CSV failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                ),
                                                format!("CSV decode failed for {}: {error}", file.name())
                                            );
                                            continue;
                                        }
                                    };
                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    progress_for_run.update(|p| {
                                        p.status = format!(
                                            "CSV processing {}/{}: {}",
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
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "CSV failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("CSV validation failed for {file_name}: {message}")
                                        );
                                        continue;
                                    }

                                    let start_ms = now_ms();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    for _ in 0..=1 {
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
                                            Err(error) => last_error = error,
                                        }
                                    }
                                    if let Some(faces) = success_faces {
                                        let Some(face) = faces.first() else {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                elapsed_ms_since(start_ms),
                                                format!(
                                                    "CSV failed {}/{}: {} (No crop target)",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                ),
                                                format!("CSV found no crop target for {file_name}.")
                                            );
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            continue;
                                        };
                                        let mut temporary_source_url = None;
                                        let source_url = if let Some(url) =
                                            preview_urls_for_run.get(&id).cloned()
                                        {
                                            url
                                        } else if let Some(url) = object_url_for_file(&file) {
                                            temporary_source_url = Some(url.clone());
                                            url
                                        } else {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                elapsed_ms_since(start_ms),
                                                format!(
                                                    "CSV failed {}/{}: {} (No source preview)",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                ),
                                                format!("CSV source preview missing for {file_name}.")
                                            );
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            continue;
                                        };
                                        let crop_bytes = match crop_face_bytes_from_source(
                                            &source_url,
                                            face,
                                            &settings_snapshot,
                                            &output_mime_type,
                                        )
                                        .await
                                        {
                                            Ok(bytes) => bytes,
                                            Err(error) => {
                                                if let Some(url) = temporary_source_url.as_deref() {
                                                    revoke_object_url(url);
                                                }
                                                record_failed_image!(
                                                    progress_for_run,
                                                    stats_for_run,
                                                    batch_state_for_run,
                                                    &id,
                                                    elapsed_ms_since(start_ms),
                                                    format!(
                                                        "CSV failed {}/{}: {} ({error})",
                                                        index + 1,
                                                        total,
                                                        file_name
                                                    ),
                                                    format!("CSV crop failed for {file_name}: {error}")
                                                );
                                                face_count_for_run.update(|m| {
                                                    m.remove(&id);
                                                });
                                                continue;
                                            }
                                        };
                                        if let Some(url) = temporary_source_url.as_deref() {
                                            revoke_object_url(url);
                                        }
                                        let crop_preview_url = match object_url_for_bytes(
                                            &crop_bytes,
                                            &output_mime_type,
                                        ) {
                                            Ok(url) => url,
                                            Err(error) => {
                                                record_failed_image!(
                                                    progress_for_run,
                                                    stats_for_run,
                                                    batch_state_for_run,
                                                    &id,
                                                    elapsed_ms_since(start_ms),
                                                    format!(
                                                        "CSV failed {}/{}: {} ({error})",
                                                        index + 1,
                                                        total,
                                                        file_name
                                                    ),
                                                    format!(
                                                        "CSV crop preview failed for {file_name}: {error}"
                                                    )
                                                );
                                                face_count_for_run.update(|m| {
                                                    m.remove(&id);
                                                });
                                                continue;
                                            }
                                        };
                                        processed_outputs_for_run.update(|outputs| {
                                            if let Some(old) = outputs.insert(
                                                id.clone(),
                                                ProcessedImageOutput {
                                                    bytes: crop_bytes,
                                                    mime_type: output_mime_type.clone(),
                                                    preview_url: crop_preview_url,
                                                },
                                            ) {
                                                revoke_object_url(&old.preview_url);
                                            }
                                        });
                                        let elapsed_ms = elapsed_ms_since(start_ms);
                                        let face_count = faces.len();
                                        progress_for_run.update(|p| {
                                            p.record_result(true);
                                            p.status = format!(
                                                "CSV processed {}/{}: {} ({} face(s))",
                                                index + 1,
                                                total,
                                                file_name,
                                                face_count
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            #[allow(clippy::cast_possible_truncation)]
                                            stats.record_image(elapsed_ms, face_count as u32, true);
                                            let source_name = source_name_by_id
                                                .get(&id)
                                                .cloned()
                                                .unwrap_or_else(|| file_name.clone());
                                            stats.push_log(format!(
                                                "CSV processed {source_name} in {elapsed_ms}ms ({face_count} face(s))."
                                            ));
                                        });
                                        face_count_for_run.update(|m| {
                                            m.insert(id.clone(), face_count);
                                        });
                                        current_id_for_run.set(Some(id.clone()));
                                        current_faces_for_run.set(faces);
                                        batch_state_for_run.update(|s| s.mark_processed(&id));
                                    } else {
                                        record_failed_image!(
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            elapsed_ms_since(start_ms),
                                            format!(
                                                "CSV failed {}/{}: {} ({last_error})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("CSV failed {}: {}", file_name, last_error)
                                        );
                                        face_count_for_run.update(|m| {
                                            m.remove(&id);
                                        });
                                        stopped_early = false;
                                    }
                                }
                                progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("CSV run stopped early");
                                    } else {
                                        p.complete("CSV process-all completed with real detection");
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
                        disabled=move || !has_selected_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = csv_batch_state.get().build_work_plan(64);
                            if plan.selected_total == 0 {
                                csv_progress.update(|p| p.complete("No mapped images selected"));
                                return;
                            }
                            let selected_ids = csv_batch_state.get().selected_ids();
                            let files_by_id = csv_files_by_id.get();
                            let validation = ImageValidationConfig::default();
                            csv_batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            csv_processed_outputs.update(|outputs| {
                                for id in &selected_ids {
                                    if let Some(old) = outputs.remove(id) {
                                        revoke_object_url(&old.preview_url);
                                    }
                                }
                            });
                            csv_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "CSV selected queue: {} image(s) in {} chunk(s)",
                                        plan.selected_total,
                                        plan.chunks.len()
                                    ),
                                );
                            });
                            let batch_state_for_run = csv_batch_state;
                            let progress_for_run = csv_progress;
                            let stats_for_run = csv_perf;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            let output_mime_type =
                                mime_type_for_output_format(&settings_snapshot.output_format)
                                    .to_string();
                            let preview_urls_for_run = csv_preview_urls.get();
                            let processed_outputs_for_run = csv_processed_outputs;
                            leptos::task::spawn_local(async move {
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        record_failed_image!(
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "CSV selected failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            ),
                                            format!("CSV missing file payload for id {id}.")
                                        );
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                0,
                                                format!(
                                                    "CSV selected failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                ),
                                                format!("CSV decode failed for {}: {error}", file.name())
                                            );
                                            continue;
                                        }
                                    };
                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    progress_for_run.update(|p| {
                                        p.status = format!(
                                            "CSV selected processing {}/{}: {}",
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
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            0,
                                            format!(
                                                "CSV selected failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("CSV validation failed for {file_name}: {message}")
                                        );
                                        continue;
                                    }
                                    let start_ms = now_ms();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    for _ in 0..=1 {
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
                                            Err(error) => last_error = error,
                                        }
                                    }
                                    if let Some(faces) = success_faces {
                                        let Some(face) = faces.first() else {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                elapsed_ms_since(start_ms),
                                                format!(
                                                    "CSV selected failed {}/{}: {} (No crop target)",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                ),
                                                format!("CSV found no crop target for {file_name}.")
                                            );
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            continue;
                                        };
                                        let mut temporary_source_url = None;
                                        let source_url = if let Some(url) =
                                            preview_urls_for_run.get(&id).cloned()
                                        {
                                            url
                                        } else if let Some(url) = object_url_for_file(&file) {
                                            temporary_source_url = Some(url.clone());
                                            url
                                        } else {
                                            record_failed_image!(
                                                progress_for_run,
                                                stats_for_run,
                                                batch_state_for_run,
                                                &id,
                                                elapsed_ms_since(start_ms),
                                                format!(
                                                    "CSV selected failed {}/{}: {} (No source preview)",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                ),
                                                format!("CSV source preview missing for {file_name}.")
                                            );
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            continue;
                                        };
                                        let crop_bytes = match crop_face_bytes_from_source(
                                            &source_url,
                                            face,
                                            &settings_snapshot,
                                            &output_mime_type,
                                        )
                                        .await
                                        {
                                            Ok(bytes) => bytes,
                                            Err(error) => {
                                                if let Some(url) = temporary_source_url.as_deref() {
                                                    revoke_object_url(url);
                                                }
                                                record_failed_image!(
                                                    progress_for_run,
                                                    stats_for_run,
                                                    batch_state_for_run,
                                                    &id,
                                                    elapsed_ms_since(start_ms),
                                                    format!(
                                                        "CSV selected failed {}/{}: {} ({error})",
                                                        index + 1,
                                                        total,
                                                        file_name
                                                    ),
                                                    format!("CSV crop failed for {file_name}: {error}")
                                                );
                                                face_count_for_run.update(|m| {
                                                    m.remove(&id);
                                                });
                                                continue;
                                            }
                                        };
                                        if let Some(url) = temporary_source_url.as_deref() {
                                            revoke_object_url(url);
                                        }
                                        let crop_preview_url = match object_url_for_bytes(
                                            &crop_bytes,
                                            &output_mime_type,
                                        ) {
                                            Ok(url) => url,
                                            Err(error) => {
                                                record_failed_image!(
                                                    progress_for_run,
                                                    stats_for_run,
                                                    batch_state_for_run,
                                                    &id,
                                                    elapsed_ms_since(start_ms),
                                                    format!(
                                                        "CSV selected failed {}/{}: {} ({error})",
                                                        index + 1,
                                                        total,
                                                        file_name
                                                    ),
                                                    format!(
                                                        "CSV crop preview failed for {file_name}: {error}"
                                                    )
                                                );
                                                face_count_for_run.update(|m| {
                                                    m.remove(&id);
                                                });
                                                continue;
                                            }
                                        };
                                        processed_outputs_for_run.update(|outputs| {
                                            if let Some(old) = outputs.insert(
                                                id.clone(),
                                                ProcessedImageOutput {
                                                    bytes: crop_bytes,
                                                    mime_type: output_mime_type.clone(),
                                                    preview_url: crop_preview_url,
                                                },
                                            ) {
                                                revoke_object_url(&old.preview_url);
                                            }
                                        });
                                        let elapsed_ms = elapsed_ms_since(start_ms);
                                        let face_count = faces.len();
                                        progress_for_run.update(|p| {
                                            p.record_result(true);
                                            p.status = format!(
                                                "CSV selected processed {}/{}: {} ({} face(s))",
                                                index + 1,
                                                total,
                                                file_name,
                                                face_count
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            #[allow(clippy::cast_possible_truncation)]
                                            stats.record_image(elapsed_ms, face_count as u32, true);
                                            let source_name = source_name_by_id
                                                .get(&id)
                                                .cloned()
                                                .unwrap_or_else(|| file_name.clone());
                                            stats.push_log(format!(
                                                "CSV selected processed {source_name} in {elapsed_ms}ms ({face_count} face(s))."
                                            ));
                                        });
                                        face_count_for_run.update(|m| {
                                            m.insert(id.clone(), face_count);
                                        });
                                        current_id_for_run.set(Some(id.clone()));
                                        current_faces_for_run.set(faces);
                                        batch_state_for_run.update(|s| s.mark_processed(&id));
                                    } else {
                                        record_failed_image!(
                                            progress_for_run,
                                            stats_for_run,
                                            batch_state_for_run,
                                            &id,
                                            elapsed_ms_since(start_ms),
                                            format!(
                                                "CSV selected failed {}/{}: {} ({last_error})",
                                                index + 1,
                                                total,
                                                file_name
                                            ),
                                            format!("CSV selected failed {}: {}", file_name, last_error)
                                        );
                                        face_count_for_run.update(|m| {
                                            m.remove(&id);
                                        });
                                    }
                                }
                                progress_for_run
                                    .update(|p| p.complete("CSV selected processing completed with real detection"));
                            });
                        }
                    >
                        "Process Selected"
                    </button>
                    <button
                        type="button"
                        id="clearAllBtn"
                        disabled=move || csv_busy.get() || (!has_images.get() && csv_state.get().rows.is_empty())
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_batch_state.update(|s| s.set_images(Vec::new()));
                            csv_queue.set(BatchQueueState::default());
                            csv_files_by_id.set(HashMap::new());
                            for url in csv_preview_urls.get().values() {
                                revoke_object_url(url);
                            }
                            csv_preview_urls.set(HashMap::new());
                            for output in csv_processed_outputs.get().values() {
                                revoke_object_url(&output.preview_url);
                            }
                            csv_processed_outputs.set(HashMap::new());
                            csv_source_name_by_id.set(HashMap::new());
                            csv_current_image_id.set(None);
                            csv_current_faces.set(Vec::new());
                            csv_current_dimensions.set((0.0, 0.0));
                            csv_face_count_by_id.set(HashMap::new());
                            csv_state.update(|s| {
                                s.rows.clear();
                                s.headers.clear();
                                s.mapping = None;
                                s.filename_to_output.clear();
                            });
                            csv_perf.update(crate::batch_core::BatchRuntimeStats::reset);
                            csv_progress.update(crate::batch_export::BatchProgress::reset);
                            csv_preview_filename.set(String::new());
                            mapping_confirmed.set(false);
                            file_path_column.set(String::new());
                            file_name_column.set(String::new());
                        }
                    >
                        "Clear All"
                    </button>
                    <button
                        type="button"
                        id="downloadAllBtn"
                        disabled=move || !has_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let batch = csv_batch_state.get();
                            let source_ids = if batch.has_selected_images() {
                                batch
                                    .selected_ids()
                                    .into_iter()
                                    .filter(|id| {
                                        batch.images.get(id).is_some_and(|img| img.processed)
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                batch
                                    .images
                                    .values()
                                    .filter(|img| img.processed)
                                    .map(|img| img.id.clone())
                                    .collect::<Vec<_>>()
                            };
                            if source_ids.is_empty() {
                                csv_progress.update(|p| p.complete("No processed CSV crops to export"));
                                return;
                            }

                            let csv = csv_state.get();
                            let source_name_by_id = csv_source_name_by_id.get();
                            let timestamp_ms = current_timestamp_ms();
                            let export_settings = settings.get();
                            let processed_outputs = csv_processed_outputs.get();
                            let entries = source_ids
                                .iter()
                                .enumerate()
                                .map(|(idx, id)| {
                                    let source_name = source_name_by_id
                                        .get(id)
                                        .cloned()
                                        .unwrap_or_else(|| batch_file_label(id).to_string());
                                    let output_name = csv.output_name_for_file(&source_name);
                                    let generated = CsvCoreState::generate_export_filename(CsvExportNameContext {
                                        template: &export_settings.naming_template,
                                        csv_output_name: output_name.as_deref(),
                                        original_file_name: &source_name,
                                        face_index: idx,
                                        timestamp_ms,
                                        output_width: export_settings.output_width,
                                        output_height: export_settings.output_height,
                                        output_format: &export_settings.output_format,
                                    });
                                    (id.clone(), generated, source_name)
                                })
                                .collect::<Vec<_>>();

                            if let Some(first) = entries.first() {
                                csv_preview_filename.set(first.1.clone());
                            }
                            let zip_name =
                                format!("face-crops-{}.zip", current_utc_timestamp_token());
                            let progress_for_download = csv_progress;
                            let stats_for_download = csv_perf;
                            leptos::task::spawn_local(async move {
                                let mut zip_entries = Vec::new();
                                for (id, generated_name, source_name) in entries {
                                    let Some(output) = processed_outputs.get(&id).cloned() else {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "CSV ZIP skipped {source_name}: no cropped output available."
                                            ));
                                        });
                                        continue;
                                    };
                                    let final_name = normalize_export_filename_for_mime(
                                        &generated_name,
                                        &output.mime_type,
                                    );
                                    if !validate_export_filename_for_mime(&final_name, &output.mime_type) {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "CSV ZIP skipped invalid filename/mime pair: {final_name} ({})",
                                                output.mime_type
                                            ));
                                        });
                                        continue;
                                    }
                                    zip_entries.push((final_name, output.bytes));
                                }
                                if zip_entries.is_empty() {
                                    progress_for_download
                                        .update(|p| p.complete("No mapped cropped outputs available for ZIP export"));
                                    return;
                                }
                                let zip_bytes = match build_zip_bytes(&zip_entries) {
                                    Ok(bytes) => bytes,
                                    Err(error) => {
                                        progress_for_download
                                            .update(|p| p.complete(format!("CSV ZIP build failed: {error}")));
                                        stats_for_download
                                            .update(|s| s.push_log(format!("CSV ZIP build failed: {error}")));
                                        return;
                                    }
                                };
                                if let Err(error) =
                                    download_bytes(&zip_name, "application/zip", &zip_bytes)
                                {
                                    progress_for_download.update(|p| {
                                        p.complete(format!("CSV ZIP download failed: {error}"));
                                    });
                                    stats_for_download.update(|s| {
                                        s.push_log(format!("CSV ZIP download failed: {error}"));
                                    });
                                    return;
                                }
                                stats_for_download.update(|s| {
                                    s.push_log(format!(
                                        "Exported CSV ZIP {} with {} file(s).",
                                        zip_name,
                                        zip_entries.len()
                                    ));
                                });
                                progress_for_download.update(|p| {
                                    p.complete(format!(
                                        "CSV ZIP exported: {} ({})",
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

            <div class="app-body">
                <aside class="control-panel">
                    <div class="control-scroll">
                        <div class="workflow-tools">
                            <h3 class="collapsible-header">
                                "CSV Processing Status"
                                <span class="collapse-icon">"▼"</span>
                            </h3>
                            <div class="collapsible-content">
                                <div class="workflow-section">
                                    <h4>"CSV Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Total Rows"</span><span class="stat-value" id="totalRows">{total_rows}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Uploaded"</span><span class="stat-value" id="imagesUploaded">{matched_images}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Matched"</span><span class="stat-value" id="imagesMatched">{matched_images}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Processing Status"</span><span class="stat-value" id="processingStatus">{mapping_status}</span></div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Processing Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Total Faces Detected"</span><span class="stat-value" id="totalFacesDetected">{csv_total_faces}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Success Rate"</span><span class="stat-value" id="successRate">{csv_success_rate}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Avg. Processing Time"</span><span class="stat-value" id="avgProcessingTime">{csv_avg_processing_time}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Processed"</span><span class="stat-value" id="imagesProcessed">{csv_images_processed}</span></div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Rust CSV Runtime"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Running"</span><span class="stat-value">{move || if csv_progress_running.get() { "Yes" } else { "No" }}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Progress"</span><span class="stat-value">{move || format!("{}%", csv_progress_percent.get())}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Preview Export Name"</span><span class="stat-value">{move || csv_preview_filename.get()}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Queued Pages"</span><span class="stat-value">{move || csv_queue.get().queued_pages_count().to_string()}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Queued Files"</span><span class="stat-value">{move || csv_queue.get().queued_files_count().to_string()}</span></div>
                                    </div>
                                    <div class="setting-help">{csv_progress_status}</div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"File Loading Log"</h4>
                                    <div class="processing-log" id="loadingLog"><div class="log-entry">"Ready to process CSV..."</div></div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Processing Log"</h4>
                                    <div class="processing-log" id="processingLog">
                                        {move || csv_logs.get().into_iter().map(|entry| view! { <div class="log-entry">{entry}</div> }).collect_view()}
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4 class="collapsible-header error-log-header">
                                        "Error Details"
                                        <span class="collapse-icon">"▼"</span>
                                    </h4>
                                    <div class="collapsible-content error-log-panel">
                                        <div class="error-log" id="errorLog">
                                            {move || {
                                                let errors = csv_error_logs.get();
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
                        <OutputSettingsCsvPanel />
                    </div>
                </aside>

                <main class="workspace">
                    <div class="workspace-scroll">
                        <BatchImageGalleryPanel
                            state=csv_batch_state
                            preview_urls=csv_preview_urls
                            busy=csv_busy
                        />

                        <div class=move || {
                            if csv_current_image_url.get().is_some() {
                                "canvas-container workspace-card"
                            } else {
                                "canvas-container hidden workspace-card"
                            }
                        } id="canvasContainer">
                            <h3 id="canvasTitle">"Current Image Preview"</h3>
                            <div class="canvas-wrapper" id="canvasWrapper">
                                <div class="canvas-panel" id="originalPanel">
                                    <h4 class="panel-title">"Original"</h4>
                                    <div class="panel-content">
                                        <canvas id="csvInputCanvas"></canvas>
                                        <div id="csvFaceOverlays" class="face-overlays">
                                            {move || {
                                                let (source_width, source_height) = csv_current_dimensions.get();
                                                if source_width <= 0.0 || source_height <= 0.0 {
                                                    return ().into_any();
                                                }
                                                csv_current_faces
                                                    .get()
                                                    .into_iter()
                                                    .map(|face| {
                                                        let left = (face.x / source_width * 100.0).clamp(0.0, 100.0);
                                                        let top = (face.y / source_height * 100.0).clamp(0.0, 100.0);
                                                        let width = (face.width / source_width * 100.0).clamp(0.0, 100.0);
                                                        let height =
                                                            (face.height / source_height * 100.0).clamp(0.0, 100.0);
                                                        view! {
                                                            <div
                                                                class="face-box selected"
                                                                style=format!(
                                                                    "left:{left:.3}%;top:{top:.3}%;width:{width:.3}%;height:{height:.3}%;"
                                                                )
                                                            />
                                                        }
                                                    })
                                                    .collect_view()
                                                    .into_any()
                                            }}
                                        </div>
                                    </div>
                                </div>
                                <div class="canvas-panel hidden" id="processedPanel">
                                    <h4 class="panel-title">"Processed"</h4>
                                    <div class="panel-content"><canvas id="csvOutputCanvas"></canvas></div>
                                </div>
                            </div>
                            <div class="face-controls">
                                <div id="faceCounter">
                                    <span>"Detected faces: " <span id="faceCount">{csv_current_faces_label}</span></span>
                                    <span class="separator">"•"</span>
                                    <span>
                                        "Current file: "
                                        <span id="selectedFaceCount">
                                            {move || {
                                                csv_current_image_id
                                                    .get()
                                                    .and_then(|id| {
                                                        csv_source_name_by_id.get().get(&id).cloned()
                                                    })
                                                    .unwrap_or_default()
                                            }}
                                        </span>
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div class="cropped-faces workspace-card" id="croppedFaces">
                            <h3>"Cropped Faces:"</h3>
                            <div id="croppedContainer">
                                {move || {
                                    let mut processed_ids = csv_batch_state
                                        .get()
                                        .images
                                        .values()
                                        .filter(|img| img.processed)
                                        .map(|img| img.id.clone())
                                        .collect::<Vec<_>>();
                                    processed_ids.sort();
                                    let outputs = csv_processed_outputs.get();
                                    let source_names = csv_source_name_by_id.get();
                                    let face_counts = csv_face_count_by_id.get();
                                    let csv = csv_state.get();
                                    processed_ids
                                        .into_iter()
                                        .map(|id| {
                                            let src = outputs.get(&id).map(|output| output.preview_url.clone());
                                            let source_name = source_names
                                                .get(&id)
                                                .cloned()
                                                .unwrap_or_else(|| batch_file_label(&id).to_string());
                                            let output_name = csv
                                                .output_name_for_file(&source_name)
                                                .unwrap_or_else(|| source_name.clone());
                                            let faces = face_counts.get(&id).copied().unwrap_or(0);
                                            view! {
                                                <div class="cropped-face">
                                                    {src.map(|url| view! { <img src=url alt="CSV processed preview" /> })}
                                                    <div class="setting-help">{format!("Source: {source_name}")}</div>
                                                    <div class="setting-help">{format!("CSV name: {output_name}")}</div>
                                                    <div class="setting-help">{format!("Faces: {faces}")}</div>
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

        </AppShell>
    }
}

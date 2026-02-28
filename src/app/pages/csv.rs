use super::*;

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
    let csv_source_name_by_id = RwSignal::new(HashMap::<String, String>::new());
    let csv_current_image_id = RwSignal::new(None::<String>);
    let csv_current_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let csv_current_dimensions = RwSignal::new((0.0_f64, 0.0_f64));
    let csv_face_count_by_id = RwSignal::new(HashMap::<String, usize>::new());
    let csv_progress = RwSignal::new(BatchProgress::default());
    let csv_stats = RwSignal::new(BatchRuntimeStats::default());
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
    let csv_total_faces = Signal::derive(move || csv_stats.get().total_faces_detected.to_string());
    let csv_success_rate =
        Signal::derive(move || format!("{}%", csv_stats.get().success_rate_pct()));
    let csv_avg_processing_time =
        Signal::derive(move || format!("{}ms", csv_stats.get().avg_processing_time_ms()));
    let csv_images_processed = Signal::derive(move || csv_stats.get().images_processed.to_string());
    let csv_logs = Signal::derive(move || csv_stats.get().logs);
    let csv_error_logs = Signal::derive(move || {
        csv_stats
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
                    dims.set((w as f64, h as f64));
                }
            });
        } else {
            csv_current_dimensions.set((0.0, 0.0));
            clear_canvas("csvInputCanvas");
        }
    });

    view! {
        <div class="app-shell">
            <header class="app-header">
                <div class="title-row">
                    <h1><a href="/">"Face Crop Forge"</a></h1>
                    <div class="header-actions">
                        <button
                            type="button"
                            id="multipleImageModeBtn"
                            class="ghost-btn"
                            title="Switch to multiple image mode"
                            on:click=move |_| navigate_to("/batch")
                        >
                            <span>"Multiple Images"</span>
                        </button>
                        <button
                            type="button"
                            id="singleImageModeBtn"
                            class="ghost-btn"
                            title="Switch to single image mode"
                            on:click=move |_| navigate_to("/single")
                        >
                            <span>"Single Image"</span>
                        </button>
                        <ThemeToggleButton id="darkModeBtn" />
                    </div>
                </div>

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
                            let stats_for_run = csv_stats;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("CSV missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!("CSV decode failed for {}: {error}", file.name()));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
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
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "CSV validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    }

                                    let start = Instant::now();
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
                                    let elapsed_ms = start.elapsed().as_millis() as u64;
                                    match success_faces {
                                        Some(faces) => {
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
                                                stats.record_image(elapsed_ms, face_count as u32, true);
                                                let source_name = source_name_by_id
                                                    .get(&id)
                                                    .cloned()
                                                    .unwrap_or_else(|| file_name.clone());
                                                stats.push_log(format!(
                                                    "CSV processed {} in {}ms ({} face(s)).",
                                                    source_name, elapsed_ms, face_count
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.insert(id.clone(), face_count);
                                            });
                                            current_id_for_run.set(Some(id.clone()));
                                            current_faces_for_run.set(faces);
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "CSV failed {}: {}",
                                                    file_name, last_error
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            stopped_early = false;
                                        }
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
                            let stats_for_run = csv_stats;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV selected failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("CSV missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV selected failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!("CSV decode failed for {}: {error}", file.name()));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
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
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV selected failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "CSV validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    }
                                    let start = Instant::now();
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
                                    let elapsed_ms = start.elapsed().as_millis() as u64;
                                    match success_faces {
                                        Some(faces) => {
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
                                                stats.record_image(elapsed_ms, face_count as u32, true);
                                                let source_name = source_name_by_id
                                                    .get(&id)
                                                    .cloned()
                                                    .unwrap_or_else(|| file_name.clone());
                                                stats.push_log(format!(
                                                    "CSV selected processed {} in {}ms ({} face(s)).",
                                                    source_name, elapsed_ms, face_count
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.insert(id.clone(), face_count);
                                            });
                                            current_id_for_run.set(Some(id.clone()));
                                            current_faces_for_run.set(faces);
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV selected failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "CSV selected failed {}: {}",
                                                    file_name, last_error
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                        }
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
                            csv_stats.update(|s| s.reset());
                            csv_progress.update(|p| p.reset());
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
                                batch.selected_ids()
                            } else {
                                batch.images.values().map(|img| img.id.clone()).collect::<Vec<_>>()
                            };
                            if source_ids.is_empty() {
                                csv_progress.update(|p| p.complete("No mapped images to export"));
                                return;
                            }

                            let csv = csv_state.get();
                            let source_name_by_id = csv_source_name_by_id.get();
                            let timestamp_ms = current_timestamp_ms();
                            let export_settings = settings.get();
                            let file_map = csv_files_by_id.get();
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
                            let stats_for_download = csv_stats;
                            leptos::task::spawn_local(async move {
                                let mut zip_entries = Vec::new();
                                for (id, generated_name, source_name) in entries {
                                    let Some(file) = file_map.get(&id).cloned() else {
                                        continue;
                                    };
                                    let mime_type = file.type_();
                                    let bytes = match file_to_bytes(&file).await {
                                        Ok(bytes) => bytes,
                                        Err(error) => {
                                            progress_for_download
                                                .update(|p| p.complete(format!("CSV ZIP failed: {error}")));
                                            stats_for_download.update(|s| {
                                                s.push_log(format!(
                                                    "CSV ZIP read failed for {source_name}: {error}"
                                                ))
                                            });
                                            return;
                                        }
                                    };
                                    let final_name = normalize_export_filename_for_mime(
                                        &generated_name,
                                        &mime_type,
                                    );
                                    if !validate_export_filename_for_mime(&final_name, &mime_type) {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "CSV ZIP skipped invalid filename/mime pair: {} ({})",
                                                final_name, mime_type
                                            ))
                                        });
                                        continue;
                                    }
                                    zip_entries.push((final_name, bytes));
                                }
                                if zip_entries.is_empty() {
                                    progress_for_download
                                        .update(|p| p.complete("No mapped binary outputs available for ZIP export"));
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
                                        p.complete(format!("CSV ZIP download failed: {error}"))
                                    });
                                    stats_for_download.update(|s| {
                                        s.push_log(format!("CSV ZIP download failed: {error}"))
                                    });
                                    return;
                                }
                                stats_for_download.update(|s| {
                                    s.push_log(format!(
                                        "Exported CSV ZIP {} with {} file(s).",
                                        zip_name,
                                        zip_entries.len()
                                    ))
                                });
                                progress_for_download.update(|p| {
                                    p.complete(format!(
                                        "CSV ZIP exported: {} ({})",
                                        zip_name,
                                        zip_entries.len()
                                    ))
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
                                                    return view! {}.into_any();
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
                                    let previews = csv_preview_urls.get();
                                    let source_names = csv_source_name_by_id.get();
                                    let face_counts = csv_face_count_by_id.get();
                                    let csv = csv_state.get();
                                    processed_ids
                                        .into_iter()
                                        .map(|id| {
                                            let src = previews.get(&id).cloned();
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

        </div>
    }
}

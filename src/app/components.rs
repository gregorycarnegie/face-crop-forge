use super::*;

#[component]
pub(super) fn AppShell(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="app-shell">
            <header class="app-header" style="padding:12px 16px;border-bottom:1px solid var(--border-color);">
                <div class="title-row" style="margin-bottom:0;">
                    <strong>{title}</strong>
                    <div class="header-actions">
                        <nav style="display:flex;gap:12px;">
                            <a href="/" style="color:var(--text-secondary);text-decoration:none;">"Home"</a>
                            <a href="/single" style="color:var(--text-secondary);text-decoration:none;">"Single"</a>
                            <a href="/batch" style="color:var(--text-secondary);text-decoration:none;">"Batch"</a>
                            <a href="/csv" style="color:var(--text-secondary);text-decoration:none;">"CSV"</a>
                            <a href="/_panels" style="color:var(--text-secondary);text-decoration:none;">"Panels"</a>
                        </nav>
                        <ThemeToggleButton id="darkModeBtn" />
                    </div>
                </div>
            </header>
            <main style="flex:1;">{children()}</main>
        </div>
    }
}

#[component]
pub(super) fn ThemeToggleButton(id: &'static str) -> impl IntoView {
    let theme_mode =
        use_context::<RwSignal<ThemeMode>>().expect("theme context should be provided by App");
    let title = Signal::derive(move || {
        if matches!(theme_mode.get(), ThemeMode::Dark) {
            "Switch to light mode"
        } else {
            "Switch to dark mode"
        }
    });

    view! {
        <button
            type="button"
            id=id
            class="ghost-btn"
            title=title
            aria-label=title
            aria-pressed=move || matches!(theme_mode.get(), ThemeMode::Light)
            on:click=move |_| {
                theme_mode.update(|mode| *mode = toggle_theme(*mode));
            }
        >
            {move || if matches!(theme_mode.get(), ThemeMode::Dark) { "🌙" } else { "☀️" }}
        </button>
    }
}

#[component]
pub(super) fn SingleUploadCard(
    state: RwSignal<SingleCoreState>,
    worker_state: RwSignal<FaceWorkerBridgeState>,
    processing_settings: RwSignal<ProcessingSettings>,
    source_image_name: RwSignal<String>,
    source_image_file: RwSignal<Option<web_sys::File>>,
    source_image_url: RwSignal<Option<String>>,
    detected_faces: RwSignal<Vec<DetectedFace>>,
    status: RwSignal<String>,
    busy: Signal<bool>,
) -> impl IntoView {
    let upload_status = Signal::derive(move || status.get());
    view! {
        <div
            class="upload-card"
            id="uploadCard"
            on:dragover=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
            }
            on:drop=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                let Some(data) = ev.data_transfer() else {
                    return;
                };
                let Some(files) = data.files() else {
                    return;
                };
                let Some(file) = files.get(0) else {
                    return;
                };
                if let Some(existing) = source_image_url.get() {
                    revoke_object_url(&existing);
                }
                source_image_file.set(Some(file.clone()));
                source_image_url.set(object_url_for_file(&file));
                source_image_name.set(file.name());
                state.update(|s| s.set_faces(Vec::new()));
                detected_faces.set(Vec::new());
                status.set("Image loaded. Running face detection...".to_string());
                worker_state.update(|w| w.mark_request_started());
                let state_for_detect = state;
                let status_for_detect = status;
                let faces_for_detect = detected_faces;
                let worker_for_detect = worker_state;
                let settings_for_detect = processing_settings;
                let file_for_detect = file.clone();
                leptos::task::spawn_local(async move {
                    match detect_faces_with_worker("browser-face-detector", file_for_detect)
                        .await
                    {
                        Ok(faces) => {
                            let filtered =
                                apply_detection_quality_filters(faces, &settings_for_detect.get());
                            let ids = filtered.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
                            let count = ids.len();
                            faces_for_detect.set(filtered);
                            state_for_detect.update(|s| s.set_faces(ids));
                            worker_for_detect.update(|w| w.mark_request_succeeded());
                            status_for_detect.set(if count == 0 {
                                "Detection completed. No faces found.".to_string()
                            } else {
                                format!("Detection completed. Found {count} face(s).")
                            });
                        }
                        Err(error) => {
                            faces_for_detect.set(Vec::new());
                            worker_for_detect.update(|w| w.mark_request_failed(error.clone()));
                            status_for_detect.set(format!("Face detection failed: {error}"));
                        }
                    }
                });
            }
        >
            <label class="file-label" for="imageInput">
                <span class="upload-title">"Select a file"</span>
                <span class="upload-subtitle">"or drag and drop it here"</span>
                <span class="setting-help">{upload_status}</span>
                <input
                    type="file"
                    id="imageInput"
                    accept="image/*"
                    class="hidden"
                    disabled=move || busy.get()
                    on:change=move |ev| {
                        if busy.get() {
                            return;
                        }
                        let input: HtmlInputElement = event_target(&ev);
                        let Some(files) = input.files() else {
                            return;
                        };
                        let Some(file) = files.get(0) else {
                            return;
                        };
                        if let Some(existing) = source_image_url.get() {
                            revoke_object_url(&existing);
                        }
                        source_image_file.set(Some(file.clone()));
                        source_image_url.set(object_url_for_file(&file));
                        source_image_name.set(file.name());
                        state.update(|s| s.set_faces(Vec::new()));
                        detected_faces.set(Vec::new());
                        status.set("Image loaded. Running face detection...".to_string());
                        worker_state.update(|w| w.mark_request_started());
                        let state_for_detect = state;
                        let status_for_detect = status;
                        let faces_for_detect = detected_faces;
                        let worker_for_detect = worker_state;
                        let settings_for_detect = processing_settings;
                        let file_for_detect = file.clone();
                        leptos::task::spawn_local(async move {
                            match detect_faces_with_worker("browser-face-detector", file_for_detect)
                                .await
                            {
                                Ok(faces) => {
                                    let filtered = apply_detection_quality_filters(
                                        faces,
                                        &settings_for_detect.get(),
                                    );
                                    let ids = filtered.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
                                    let count = ids.len();
                                    faces_for_detect.set(filtered);
                                    state_for_detect.update(|s| s.set_faces(ids));
                                    worker_for_detect.update(|w| w.mark_request_succeeded());
                                    status_for_detect.set(if count == 0 {
                                        "Detection completed. No faces found.".to_string()
                                    } else {
                                        format!("Detection completed. Found {count} face(s).")
                                    });
                                }
                                Err(error) => {
                                    faces_for_detect.set(Vec::new());
                                    worker_for_detect.update(|w| w.mark_request_failed(error.clone()));
                                    status_for_detect.set(format!("Face detection failed: {error}"));
                                }
                            }
                        });
                    }
                />
            </label>
        </div>
    }
}

#[component]
pub(super) fn BatchUploadCard(
    state: RwSignal<BatchCoreState>,
    queue: RwSignal<BatchQueueState>,
    progress: RwSignal<BatchProgress>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    busy: Signal<bool>,
) -> impl IntoView {
    let upload_status = Signal::derive(move || progress.get().status);
    let upload_progress = Signal::derive(move || {
        let p = progress.get();
        if p.total == 0 {
            "0/0 processed".to_string()
        } else {
            format!("{}/{} processed, {} failed", p.processed, p.total, p.failed)
        }
    });
    view! {
        <div
            class="upload-card"
            on:dragover=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
            }
            on:drop=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                let Some(data) = ev.data_transfer() else {
                    return;
                };
                let Some(files) = data.files() else {
                    return;
                };
                for url in preview_urls.get().values() {
                    revoke_object_url(url);
                }
                let mut ids = Vec::new();
                let mut files_map = HashMap::new();
                let mut previews = HashMap::new();
                for idx in 0..files.length() {
                    if let Some(file) = files.get(idx) {
                        let id = format!(
                            "{}::{}::{}::{}",
                            file.name(),
                            file.size(),
                            file.last_modified(),
                            idx
                        );
                        if let Some(url) = object_url_for_file(&file) {
                            previews.insert(id.clone(), url);
                        }
                        files_map.insert(id.clone(), file);
                        ids.push(id);
                    }
                }
                let count = ids.len();
                let queue_state = BatchQueueState::from_files(ids, 20);
                let queued_pages = queue_state.queued_pages_count();
                let queued_files = queue_state.queued_files_count();
                let initial_loaded = queue_state.loaded_ids.len();
                state.update(|s| s.set_images(queue_state.loaded_ids.clone()));
                queue.set(queue_state);
                files_by_id.set(files_map);
                preview_urls.set(previews);
                progress.update(|p| {
                    p.reset();
                    p.status = if queued_pages > 0 {
                        format!(
                            "Loaded first {} image(s); queued {} image(s) across {} page(s).",
                            initial_loaded,
                            queued_files,
                            queued_pages
                        )
                    } else {
                        format!("Loaded {count} image(s). Ready to process.")
                    };
                });
            }
        >
            <label class="file-label" for="imageInput">
                <span class="upload-title">"Select files"</span>
                <span class="upload-subtitle">"or drag and drop them here"</span>
                <span class="setting-help">{upload_status}</span>
                <span class="setting-help">{upload_progress}</span>
                <input
                    type="file"
                    id="imageInput"
                    accept="image/*"
                    multiple
                    class="hidden"
                    disabled=move || busy.get()
                    on:change=move |ev| {
                        if busy.get() {
                            return;
                        }
                        let input: HtmlInputElement = event_target(&ev);
                        let Some(files) = input.files() else {
                            return;
                        };
                        for url in preview_urls.get().values() {
                            revoke_object_url(url);
                        }
                        let mut ids = Vec::new();
                        let mut files_map = HashMap::new();
                        let mut previews = HashMap::new();
                        for idx in 0..files.length() {
                            if let Some(file) = files.get(idx) {
                                let id = format!(
                                    "{}::{}::{}::{}",
                                    file.name(),
                                    file.size(),
                                    file.last_modified(),
                                    idx
                                );
                                if let Some(url) = object_url_for_file(&file) {
                                    previews.insert(id.clone(), url);
                                }
                                files_map.insert(id.clone(), file);
                                ids.push(id);
                            }
                        }
                        let count = ids.len();
                        let queue_state = BatchQueueState::from_files(ids, 20);
                        let queued_pages = queue_state.queued_pages_count();
                        let queued_files = queue_state.queued_files_count();
                        let initial_loaded = queue_state.loaded_ids.len();
                        state.update(|s| s.set_images(queue_state.loaded_ids.clone()));
                        queue.set(queue_state);
                        files_by_id.set(files_map);
                        preview_urls.set(previews);
                        progress.update(|p| {
                            p.reset();
                            p.status = if queued_pages > 0 {
                                format!(
                                    "Loaded first {} image(s); queued {} image(s) across {} page(s).",
                                    initial_loaded,
                                    queued_files,
                                    queued_pages
                                )
                            } else {
                                format!("Loaded {count} image(s). Ready to process.")
                            };
                        });
                    }
                />
            </label>
        </div>
    }
}

pub(super) fn batch_file_label(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

#[component]
pub(super) fn BatchImageGalleryPanel(
    state: RwSignal<BatchCoreState>,
    preview_urls: RwSignal<HashMap<String, String>>,
    busy: Signal<bool>,
) -> impl IntoView {
    let selected_count = Signal::derive(move || state.get().selected_count().to_string());
    let total_count = Signal::derive(move || state.get().total_count().to_string());
    let ordered_ids = Signal::derive(move || {
        let mut ids = state.get().images.keys().cloned().collect::<Vec<_>>();
        ids.sort();
        ids
    });

    view! {
        <div class=move || {
            if state.get().total_count() == 0 {
                "image-gallery hidden workspace-card"
            } else {
                "image-gallery workspace-card"
            }
        } id="imageGallery">
            <h3>"Image Gallery"</h3>
            <div class="gallery-controls">
                <button
                    type="button"
                    id="selectAllBtn"
                    disabled=move || busy.get()
                    on:click=move |_| state.update(|s| s.select_all())
                >
                    "Select All"
                </button>
                <button
                    type="button"
                    id="selectNoneBtn"
                    disabled=move || busy.get()
                    on:click=move |_| state.update(|s| s.select_none())
                >
                    "Select None"
                </button>
                <span class="selection-counter">
                    "Selected: "
                    <span id="selectedCount">{selected_count}</span>
                    " of "
                    <span id="totalCount">{total_count}</span>
                </span>
            </div>
            <div class="gallery-grid" id="galleryGrid">
                {move || {
                    let urls = preview_urls.get();
                    ordered_ids
                        .get()
                        .into_iter()
                        .map(|id| {
                            let image = state.get().images.get(&id).cloned();
                            let selected = image.as_ref().map(|img| img.selected).unwrap_or(false);
                            let status_label = image
                                .as_ref()
                                .map(|img| format!("{:?}", img.status))
                                .unwrap_or_else(|| "Unknown".to_string());
                            let border = if selected {
                                "2px solid var(--accent)"
                            } else {
                                "1px solid var(--border-color)"
                            };
                            let caption = batch_file_label(&id).to_string();
                            let url = urls.get(&id).cloned();
                            let id_for_click = id.clone();
                            view! {
                                <button
                                    type="button"
                                    class="workspace-card"
                                    style=format!("padding:10px;text-align:left;border:{border};")
                                    disabled=move || busy.get()
                                    on:click=move |_| state.update(|s| s.toggle_selection(&id_for_click))
                                >
                                    {url.map(|src| {
                                        view! {
                                            <img
                                                src=src
                                                alt="Batch preview"
                                                style="width:100%;height:120px;object-fit:cover;border-radius:8px;margin-bottom:8px;"
                                            />
                                        }
                                    })}
                                    <div style="font-size:0.8rem;color:var(--text-primary);">{caption}</div>
                                    <div style="font-size:0.75rem;color:var(--muted-text);">{status_label}</div>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </div>
    }
}

#[component]
pub(super) fn CsvUploadCard(
    state: RwSignal<CsvCoreState>,
    progress: RwSignal<BatchProgress>,
    busy: Signal<bool>,
) -> impl IntoView {
    let upload_status = Signal::derive(move || progress.get().status);
    view! {
        <div
            class="upload-card"
            on:dragover=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
            }
            on:drop=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                let Some(data) = ev.data_transfer() else {
                    return;
                };
                let Some(files) = data.files() else {
                    return;
                };
                let Some(file) = files.get(0) else {
                    return;
                };
                let state = state;
                leptos::task::spawn_local(async move {
                    let Ok(js_text) = JsFuture::from(file.text()).await else {
                        return;
                    };
                    let Some(csv_text) = js_text.as_string() else {
                        return;
                    };
                    state.update(|s| {
                        let _ = s.parse_csv_text(&csv_text);
                    });
                });
            }
        >
            <label class="file-label" for="csvInput">
                <span class="upload-title">"Select CSV file"</span>
                <span class="upload-subtitle">"or drag and drop it here"</span>
                <span class="setting-help">{upload_status}</span>
                <input
                    type="file"
                    id="csvInput"
                    accept=".csv"
                    class="hidden"
                    disabled=move || busy.get()
                    on:change=move |ev| {
                        if busy.get() {
                            return;
                        }
                        let input: HtmlInputElement = event_target(&ev);
                        let Some(files) = input.files() else {
                            return;
                        };
                        let Some(file) = files.get(0) else {
                            return;
                        };

                        let state = state;
                        leptos::task::spawn_local(async move {
                            let Ok(js_text) = JsFuture::from(file.text()).await else {
                                return;
                            };
                            let Some(csv_text) = js_text.as_string() else {
                                return;
                            };
                            state.update(|s| {
                                let _ = s.parse_csv_text(&csv_text);
                            });
                        });
                    }
                />
            </label>
        </div>
    }
}

#[component]
pub(super) fn CsvImageUploadCard(
    state: RwSignal<BatchCoreState>,
    queue: RwSignal<BatchQueueState>,
    csv_state: RwSignal<CsvCoreState>,
    files_by_id: RwSignal<HashMap<String, web_sys::File>>,
    preview_urls: RwSignal<HashMap<String, String>>,
    source_name_by_id: RwSignal<HashMap<String, String>>,
    progress: RwSignal<BatchProgress>,
    busy: Signal<bool>,
) -> impl IntoView {
    let upload_status = Signal::derive(move || progress.get().status);
    let upload_progress = Signal::derive(move || {
        let p = progress.get();
        if p.total == 0 {
            "0/0 processed".to_string()
        } else {
            format!("{}/{} processed, {} failed", p.processed, p.total, p.failed)
        }
    });
    view! {
        <div
            class="upload-card"
            on:dragover=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
            }
            on:drop=move |ev: DragEvent| {
                if busy.get() {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                let Some(data) = ev.data_transfer() else {
                    return;
                };
                let Some(files) = data.files() else {
                    return;
                };
                for url in preview_urls.get().values() {
                    revoke_object_url(url);
                }
                let csv = csv_state.get();
                let mut ids = Vec::new();
                let mut file_map = HashMap::new();
                let mut preview_map = HashMap::new();
                let mut source_name_map = HashMap::new();
                for idx in 0..files.length() {
                    if let Some(file) = files.get(idx) {
                        let source_name = file.name();
                        if csv.output_name_for_file(&source_name).is_none() {
                            continue;
                        }
                        let id = format!(
                            "{}::{}::{}::{}",
                            source_name,
                            file.size(),
                            file.last_modified(),
                            idx
                        );
                        if let Some(url) = object_url_for_file(&file) {
                            preview_map.insert(id.clone(), url);
                        }
                        source_name_map.insert(id.clone(), source_name);
                        file_map.insert(id.clone(), file);
                        ids.push(id);
                    }
                }
                let queue_state = BatchQueueState::from_files(ids, 20);
                state.update(|s| s.set_images(queue_state.loaded_ids.clone()));
                queue.set(queue_state);
                files_by_id.set(file_map);
                preview_urls.set(preview_map);
                source_name_by_id.set(source_name_map);
            }
        >
            <label class="file-label" for="imageInput">
                <span class="upload-title">"Select images"</span>
                <span class="upload-subtitle">"Only images matching CSV filenames will be kept"</span>
                <span class="setting-help">{upload_status}</span>
                <span class="setting-help">{upload_progress}</span>
                <input
                    type="file"
                    id="imageInput"
                    accept="image/*"
                    multiple
                    class="hidden"
                    disabled=move || busy.get()
                    on:change=move |ev| {
                        if busy.get() {
                            return;
                        }
                        let input: HtmlInputElement = event_target(&ev);
                        let Some(files) = input.files() else {
                            return;
                        };
                        for url in preview_urls.get().values() {
                            revoke_object_url(url);
                        }
                        let csv = csv_state.get();
                        let mut ids = Vec::new();
                        let mut file_map = HashMap::new();
                        let mut preview_map = HashMap::new();
                        let mut source_name_map = HashMap::new();
                        for idx in 0..files.length() {
                            if let Some(file) = files.get(idx) {
                                let source_name = file.name();
                                if csv.output_name_for_file(&source_name).is_none() {
                                    continue;
                                }
                                let id = format!(
                                    "{}::{}::{}::{}",
                                    source_name,
                                    file.size(),
                                    file.last_modified(),
                                    idx
                                );
                                if let Some(url) = object_url_for_file(&file) {
                                    preview_map.insert(id.clone(), url);
                                }
                                source_name_map.insert(id.clone(), source_name);
                                file_map.insert(id.clone(), file);
                                ids.push(id);
                            }
                        }
                        let queue_state = BatchQueueState::from_files(ids, 20);
                        state.update(|s| s.set_images(queue_state.loaded_ids.clone()));
                        queue.set(queue_state);
                        files_by_id.set(file_map);
                        preview_urls.set(preview_map);
                        source_name_by_id.set(source_name_map);
                    }
                />
            </label>
        </div>
    }
}

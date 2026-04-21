use super::{
    AppState, ClassAttribute, CollectView, CropSettingsPanel, DetectedFace, Effect, ElementChild,
    FaceWorkerBridgeState, Get, GlobalAttributes, IntoAny, IntoView, MediaPipeAssetPaths,
    OnAttribute, OutputSettingsBatchPanel, PreprocessingSettingsPanel, RwSignal, Set, Signal,
    SingleCoreState, SingleRuntimeState, SingleUploadCard, StyleAttribute, ThemeToggleButton,
    Update, apply_detection_quality_filters, build_export_plan, build_load_plan,
    capture_webcam_frame_to_file, clear_canvas, clear_last_detection_backend, clear_video_source,
    component, compute_display_size, crop_face_bytes_from_source, current_timestamp_ms,
    detect_browser_capabilities, detect_faces_with_worker, download_bytes,
    draw_source_image_to_canvas, evaluate_pipeline_health, last_detection_backend_label,
    list_video_input_devices, navigate_to, normalize_export_filename_for_mime, object_url_for_file,
    overlay_percent_crop_rect, revalidate_browser_fallbacks, revoke_object_url, start_face_worker,
    start_webcam_stream, stop_face_worker, stop_media_stream, use_context,
    validate_export_filename_for_mime, view,
};

#[allow(clippy::too_many_lines)]
#[component]
pub(crate) fn SinglePage() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let single_state = RwSignal::new(SingleCoreState::default());
    let worker_state = RwSignal::new(FaceWorkerBridgeState::default());
    let single_status = RwSignal::new("Ready to load image".to_string());
    let single_runtime = RwSignal::new(SingleRuntimeState::default());
    let source_image_name = RwSignal::new("image.png".to_string());
    let source_image_file = RwSignal::new(None::<web_sys::File>);
    let source_image_url = RwSignal::new(None::<String>);
    let source_image_dimensions = RwSignal::new((0.0_f64, 0.0_f64));
    let detected_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let worker_auto_started = RwSignal::new(false);
    let webcam_stream = RwSignal::new(None::<web_sys::MediaStream>);
    let webcam_device_ids = RwSignal::new(Vec::<String>::new());
    let download_preview = RwSignal::new(String::new());
    let planned_export_count = RwSignal::new(0usize);
    let rotation_label =
        Signal::derive(move || format!("{}°", single_state.get().rotation_degrees));
    let selected_label = Signal::derive(move || single_state.get().selected_count().to_string());
    let faces_label = Signal::derive(move || single_state.get().faces_count().to_string());
    let preview_size_label = Signal::derive(move || {
        let display = compute_display_size(
            1200.0,
            800.0,
            single_state.get().rotation_degrees,
            600.0,
            400.0,
        );
        format!("{:.0} × {:.0}", display.width, display.height)
    });
    let has_faces = Signal::derive(move || single_state.get().faces_count() > 0);
    let can_download = Signal::derive(move || single_state.get().selected_count() > 0);
    let has_source_image = Signal::derive(move || source_image_file.get().is_some());
    let processing_time_label =
        Signal::derive(move || format!("{}ms", single_runtime.get().processing_time_ms));
    let runtime_logs = Signal::derive(move || single_runtime.get().logs);
    let webcam_open = Signal::derive(move || single_state.get().webcam_modal_open);
    let active_camera_label =
        Signal::derive(move || single_state.get().active_camera_name().to_string());
    let worker_status_label = Signal::derive(move || worker_state.get().status_label().to_string());
    let worker_backend_label = Signal::derive(move || {
        let _ = worker_state.get();
        last_detection_backend_label().to_string()
    });
    let worker_error_label = Signal::derive(move || {
        worker_state
            .get()
            .last_error
            .unwrap_or_else(|| "None".to_string())
    });
    let single_busy = Signal::derive(move || {
        single_runtime.get().running
            || matches!(
                worker_state.get().status,
                crate::worker_bridge::FaceWorkerStatus::Starting
            )
    });
    let browser_capabilities = Signal::derive(detect_browser_capabilities);
    let mediapipe_plan = Signal::derive(move || {
        build_load_plan(
            browser_capabilities.get(),
            true,
            true,
            MediaPipeAssetPaths::default(),
        )
    });
    let mediapipe_plan_summary = Signal::derive(move || mediapipe_plan.get().summary());
    let mediapipe_wasm_root = Signal::derive(move || mediapipe_plan.get().assets.wasm_root.clone());
    let mediapipe_bundle =
        Signal::derive(move || mediapipe_plan.get().assets.vision_bundle_url.clone());
    let mediapipe_detector_model =
        Signal::derive(move || mediapipe_plan.get().assets.detector_model_url.clone());
    let mediapipe_landmarker_model =
        Signal::derive(move || mediapipe_plan.get().assets.landmarker_model_url.clone());
    let pipeline_health =
        Signal::derive(move || evaluate_pipeline_health(browser_capabilities.get()));
    let pipeline_state = Signal::derive(move || pipeline_health.get().summary().to_string());
    let pipeline_offscreen = Signal::derive(move || {
        if pipeline_health.get().offscreen_canvas {
            "Yes"
        } else {
            "No"
        }
    });
    let pipeline_image_bitmap = Signal::derive(move || {
        if pipeline_health.get().image_bitmap {
            "Yes"
        } else {
            "No"
        }
    });
    let pipeline_worker_transfer = Signal::derive(move || {
        if pipeline_health.get().worker_transfer {
            "Yes"
        } else {
            "No"
        }
    });
    let browser_fallback_matrix = Signal::derive(move || {
        revalidate_browser_fallbacks(true, true, &MediaPipeAssetPaths::default())
    });

    Effect::new(move |_| {
        let status = single_status.get();
        single_runtime.update(|r| {
            if r.status != status {
                r.set_status(status.clone());
            }
        });
    });

    Effect::new(move |_| {
        let source_url = source_image_url.get();
        if let Some(url) = source_url {
            let dimensions = source_image_dimensions;
            let status = single_status;
            leptos::task::spawn_local(async move {
                match draw_source_image_to_canvas("inputCanvas", &url).await {
                    Ok((width, height)) => {
                        dimensions.set((f64::from(width), f64::from(height)));
                    }
                    Err(error) => {
                        dimensions.set((0.0, 0.0));
                        status.set(format!("Failed to render image preview: {error}"));
                    }
                }
            });
        } else {
            source_image_dimensions.set((0.0, 0.0));
            clear_canvas("inputCanvas");
        }
    });

    Effect::new(move |_| {
        if webcam_open.get() {
            return;
        }
        if let Some(stream) = webcam_stream.get() {
            stop_media_stream(&stream);
            webcam_stream.set(None);
        }
        clear_video_source("webcamVideo");
    });

    // Automatically warm up worker state on page load so manual init is optional.
    Effect::new(move |_| {
        if !worker_auto_started.get() && worker_state.get().can_start() {
            worker_state.update(start_face_worker);
            worker_auto_started.set(true);
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
                            id="backToMultipleBtn"
                            class="ghost-btn"
                            title="Switch to multiple image mode"
                            on:click=move |_| navigate_to("/batch")
                        >
                            <span>"Multiple Images"</span>
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

                <SingleUploadCard
                    state=single_state
                    worker_state=worker_state
                    processing_settings=settings
                    source_image_name=source_image_name
                    source_image_file=source_image_file
                    source_image_url=source_image_url
                    detected_faces=detected_faces
                    status=single_status
                    busy=single_busy
                />

                <div class="webcam-toggle-container">
                    <button
                        type="button"
                        id="toggleWebcamBtn"
                        class="ghost-btn"
                        title="Use webcam"
                        disabled=move || single_busy.get()
                        on:click=move |_| {
                            if single_busy.get() {
                                return;
                            }
                            single_state.update(crate::single_core::SingleCoreState::open_webcam_modal);
                            single_status.set("Webcam modal opened. Initializing camera...".to_string());
                            let status_for_camera = single_status;
                            let stream_for_camera = webcam_stream;
                            let devices_for_camera = webcam_device_ids;
                            let state_for_camera = single_state;
                            leptos::task::spawn_local(async move {
                                let devices = match list_video_input_devices().await {
                                    Ok(devices) => devices,
                                    Err(error) => {
                                        status_for_camera.set(format!("Failed to list cameras: {error}"));
                                        return;
                                    }
                                };
                                if devices.is_empty() {
                                    status_for_camera.set("No camera devices were found.".to_string());
                                    return;
                                }
                                let device_ids = devices.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
                                let labels = devices.into_iter().map(|(_, label)| label).collect::<Vec<_>>();
                                let mut active_index = state_for_camera.get().active_camera_index;
                                if active_index >= device_ids.len() {
                                    active_index = 0;
                                    state_for_camera.update(|s| s.active_camera_index = 0);
                                }
                                state_for_camera.update(|s| s.cameras = labels);
                                devices_for_camera.set(device_ids.clone());
                                if let Some(existing) = stream_for_camera.get() {
                                    stop_media_stream(&existing);
                                    stream_for_camera.set(None);
                                }
                                let preferred = device_ids.get(active_index).cloned();
                                match start_webcam_stream("webcamVideo", preferred.as_deref()).await {
                                    Ok(stream) => {
                                        stream_for_camera.set(Some(stream));
                                        status_for_camera.set("Webcam initialized. Capture a photo to process.".to_string());
                                    }
                                    Err(error) => {
                                        status_for_camera.set(format!("Failed to initialize webcam: {error}"));
                                    }
                                }
                            });
                        }
                    >
                        <span>"📷 Use Webcam"</span>
                    </button>
                </div>
            </header>

            <div class="app-body">
                <aside class="control-panel">
                    <div class="control-scroll">
                        <div class="workflow-tools">
                            <h3 class="collapsible-header">
                                "Processing Status"
                                <span class="collapse-icon">"▼"</span>
                            </h3>
                            <div class="collapsible-content">
                                <div class="workflow-section">
                                    <h4>"Current Image Stats"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Faces Detected"</span>
                                            <span class="stat-value" id="facesDetected">{faces_label}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Processing Time"</span>
                                            <span class="stat-value" id="processingTime">{processing_time_label}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Image Size"</span>
                                            <span class="stat-value" id="imageSize">{preview_size_label}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Status"</span>
                                            <span class="stat-value" id="processingStatus">{single_status}</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Detection Runtime"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Status"</span>
                                            <span class="stat-value">{worker_status_label}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Error"</span>
                                            <span class="stat-value">{worker_error_label}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Backend"</span>
                                            <span class="stat-value">{worker_backend_label}</span>
                                        </div>
                                    </div>
                                    <div class="setting-help">
                                        "Detection initializes automatically when this page loads."
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"MediaPipe Load Strategy"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Strategy"</span>
                                            <span class="stat-value">{mediapipe_plan_summary}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"WASM Root"</span>
                                            <span class="stat-value">{mediapipe_wasm_root}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Vision Bundle"</span>
                                            <span class="stat-value">{mediapipe_bundle}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Detector Model"</span>
                                            <span class="stat-value">{mediapipe_detector_model}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Landmarker Model"</span>
                                            <span class="stat-value">{mediapipe_landmarker_model}</span>
                                        </div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Offscreen/ImageBitmap Pipeline"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Pipeline State"</span>
                                            <span class="stat-value">{pipeline_state}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"OffscreenCanvas"</span>
                                            <span class="stat-value">{pipeline_offscreen}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"ImageBitmap"</span>
                                            <span class="stat-value">{pipeline_image_bitmap}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Worker Transfer Path"</span>
                                            <span class="stat-value">{pipeline_worker_transfer}</span>
                                        </div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Browser SIMD/Thread Fallback Validation"</h4>
                                    <div class="stats-grid">
                                        {move || browser_fallback_matrix.get().into_iter().map(|entry| {
                                            let execution_path = if entry.worker_pipeline {
                                                "worker pipeline"
                                            } else {
                                                "main-thread fallback"
                                            };
                                            let summary = format!(
                                                "{} | {} | {}",
                                                entry.wasm_variant.label(),
                                                entry.pipeline_state.label(),
                                                execution_path
                                            );
                                            view! {
                                                <div class="stat-item">
                                                    <span class="stat-label">{entry.browser}</span>
                                                    <span class="stat-value">{summary}</span>
                                                    <span class="setting-help">{entry.fallback_reason}</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Processing Log"</h4>
                                    <div class="processing-log" id="processingLog">
                                        {move || runtime_logs.get().into_iter().map(|entry| view! { <div class="log-entry">{entry}</div> }).collect_view()}
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
                        <div class="side-by-side-container">
                            <div class=move || {
                                if has_source_image.get() {
                                    "canvas-container workspace-card"
                                } else {
                                    "canvas-container hidden workspace-card"
                                }
                            } id="canvasContainer">
                                <h3 id="canvasTitle">"Image Preview"</h3>
                                <div class="canvas-wrapper" id="canvasWrapper">
                                    <div class="canvas-panel" id="originalPanel">
                                        <h4 class="panel-title">"Original"</h4>
                                        <div class="panel-content">
                                            <div class="canvas-stage">
                                                <canvas id="inputCanvas"></canvas>
                                                <div id="faceOverlays" class="face-overlays">
                                                    {move || {
                                                        let (source_width, source_height) = source_image_dimensions.get();
                                                        if source_width <= 0.0 || source_height <= 0.0 {
                                                            return ().into_any();
                                                        }
                                                        let selected_ids = single_state.get().selected_face_ids;
                                                        detected_faces
                                                            .get()
                                                            .into_iter()
                                                            .map(|face| {
                                                                let is_selected = selected_ids.contains(&face.id);
                                                                let (left, top, width, height) = overlay_percent_crop_rect(
                                                                    &face,
                                                                    source_width,
                                                                    source_height,
                                                                    &settings.get(),
                                                                );
                                                                let class_name = if is_selected {
                                                                    "face-box selected"
                                                                } else {
                                                                    "face-box unselected"
                                                                };
                                                                let face_id = face.id.clone();
                                                                let title = format!(
                                                                    "{} ({:.0}% confidence)",
                                                                    face.id,
                                                                    face.confidence * 100.0
                                                                );
                                                                view! {
                                                                    <button
                                                                        type="button"
                                                                        class=class_name
                                                                        style=format!(
                                                                            "left:{left:.3}%;top:{top:.3}%;width:{width:.3}%;height:{height:.3}%;"
                                                                        )
                                                                        title=title
                                                                        on:click=move |_| {
                                                                            single_state.update(|s| s.toggle_face_selection(&face_id));
                                                                            single_status.set(format!("Toggled selection for {face_id}."));
                                                                        }
                                                                    />
                                                                }
                                                            })
                                                            .collect_view()
                                                            .into_any()
                                                    }}
                                                </div>
                                            </div>
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
                                        <span>
                                            "Detected faces: "
                                            <span id="faceCount">{faces_label}</span>
                                        </span>
                                        <span class="separator">"•"</span>
                                        <span>
                                            "Selected: "
                                            <span id="selectedFaceCount">{selected_label}</span>
                                        </span>
                                        <span class="separator">"•"</span>
                                        <span>"Rotation: " {rotation_label}</span>
                                    </div>
                                    <div class="face-selection-controls">
                                        <div class="button-group rotation-controls">
                                            <button
                                                type="button"
                                                id="rotateCounterClockwiseBtn"
                                                class="small-btn rotation-btn"
                                                disabled=move || !has_faces.get()
                                                title="Rotate 90° counter-clockwise"
                                                on:click=move |_| {
                                                    single_state.update(|s| {
                                                        s.rotate_by(-90);
                                                        s.clear_faces_after_rotation();
                                                    });
                                                    detected_faces.set(Vec::new());
                                                    download_preview.set(String::new());
                                                    single_status.set("Rotated image. Face detections cleared; click Detect Faces.".to_string());
                                                }
                                            >
                                                <span class="btn-icon">"↶"</span>
                                                " 90°"
                                            </button>
                                            <button
                                                type="button"
                                                id="rotateClockwiseBtn"
                                                class="small-btn rotation-btn"
                                                disabled=move || !has_faces.get()
                                                title="Rotate 90° clockwise"
                                                on:click=move |_| {
                                                    single_state.update(|s| {
                                                        s.rotate_by(90);
                                                        s.clear_faces_after_rotation();
                                                    });
                                                    detected_faces.set(Vec::new());
                                                    download_preview.set(String::new());
                                                    single_status.set("Rotated image. Face detections cleared; click Detect Faces.".to_string());
                                                }
                                            >
                                                "90° "
                                                <span class="btn-icon">"↷"</span>
                                            </button>
                                        </div>
                                        <div class="button-group selection-controls">
                                            <button
                                                type="button"
                                                id="selectAllFacesBtn"
                                                class="small-btn selection-btn"
                                                on:click=move |_| {
                                                    single_state.update(crate::single_core::SingleCoreState::select_all_faces);
                                                    single_status.set("Selected all detected faces.".to_string());
                                                }
                                            >
                                                "Select All"
                                            </button>
                                            <button
                                                type="button"
                                                id="selectNoneFacesBtn"
                                                class="small-btn selection-btn"
                                                on:click=move |_| {
                                                    single_state.update(crate::single_core::SingleCoreState::select_none_faces);
                                                    single_status.set("Cleared selected faces.".to_string());
                                                }
                                            >
                                                "Select None"
                                            </button>
                                        </div>
                                        <div class="button-group selection-controls">
                                            <div class="setting-help">"Per-face toggles"</div>
                                            <div class="gallery-controls">
                                                {move || {
                                                    let state = single_state.get();
                                                    state
                                                        .all_face_ids
                                                        .iter()
                                                        .map(|face_id| {
                                                            let is_selected = state.selected_face_ids.contains(face_id);
                                                            let label = if is_selected {
                                                                format!("{face_id} ✓")
                                                            } else {
                                                                format!("{face_id} ○")
                                                            };
                                                            let face_id_for_click = face_id.clone();
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class="small-btn selection-btn"
                                                                    on:click=move |_| {
                                                                        single_state.update(|s| s.toggle_face_selection(&face_id_for_click));
                                                                        single_status.set(format!("Toggled selection for {face_id_for_click}."));
                                                                    }
                                                                >
                                                                    {label}
                                                                </button>
                                                            }
                                                        })
                                                        .collect_view()
                                                }}
                                            </div>
                                        </div>
                                        <div class="button-group action-controls">
                                            <button
                                                type="button"
                                                id="detectFacesBtn"
                                                class="small-btn primary"
                                                disabled=move || !has_source_image.get() || single_busy.get()
                                                on:click=move |_| {
                                                    if single_busy.get() {
                                                        return;
                                                    }
                                                    single_runtime.update(|r| r.start("Detecting faces..."));
                                                    single_state.update(|s| {
                                                        s.set_faces(Vec::new());
                                                        s.reset_rotation_after_reencode();
                                                    });
                                                    detected_faces.set(Vec::new());
                                                    let state_for_detect = single_state;
                                                    let runtime_for_detect = single_runtime;
                                                    let status_for_detect = single_status;
                                                    let faces_for_detect = detected_faces;
                                                    let worker_for_detect = worker_state;
                                                    if let Some(file) = source_image_file.get() {
                                                        let settings_for_detect = settings.get();
                                                        worker_for_detect.update(crate::worker_bridge::FaceWorkerBridgeState::mark_request_started);
                                                        leptos::task::spawn_local(async move {
                                                            runtime_for_detect.update(|r| r.start("Detecting faces..."));
                                                            match detect_faces_with_worker(
                                                                "browser-face-detector",
                                                                file,
                                                            )
                                                            .await
                                                            {
                                                                Ok(faces) => {
                                                                    let filtered = apply_detection_quality_filters(
                                                                        faces,
                                                                        &settings_for_detect,
                                                                    );
                                                                    let ids = filtered.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
                                                                    let count = ids.len();
                                                                    faces_for_detect.set(filtered);
                                                                    state_for_detect.update(|s| s.set_faces(ids));
                                                                    worker_for_detect.update(crate::worker_bridge::FaceWorkerBridgeState::mark_request_succeeded);
                                                                    runtime_for_detect.update(|r| {
                                                                        r.complete(24, format!("Detected {count} face(s)."));
                                                                    });
                                                                    status_for_detect.set(if count == 0 {
                                                                        "Detection completed. No faces found.".to_string()
                                                                    } else {
                                                                        format!("Detection completed. Found {count} face(s).")
                                                                    });
                                                                }
                                                                Err(error) => {
                                                                    faces_for_detect.set(Vec::new());
                                                                    worker_for_detect.update(|w| w.mark_request_failed(error.clone()));
                                                                    runtime_for_detect.update(|r| {
                                                                        r.complete(0, format!("Detection failed: {error}"));
                                                                    });
                                                                    status_for_detect.set(format!("Face detection failed: {error}"));
                                                                }
                                                            }
                                                        });
                                                    } else {
                                                        single_status.set("Load an image before running detection.".to_string());
                                                    }
                                                }
                                            >
                                                "Detect Faces"
                                            </button>
                                            <button
                                                type="button"
                                                id="clearImageBtn"
                                                class="small-btn clear-btn"
                                                disabled=move || !has_source_image.get() || single_busy.get()
                                                on:click=move |_| {
                                                    if single_busy.get() {
                                                        return;
                                                    }
                                                    single_state.update(|s| s.set_faces(Vec::new()));
                                                    detected_faces.set(Vec::new());
                                                    if let Some(existing) = source_image_url.get() {
                                                        revoke_object_url(&existing);
                                                    }
                                                    source_image_file.set(None);
                                                    source_image_url.set(None);
                                                    source_image_dimensions.set((0.0, 0.0));
                                                    clear_canvas("inputCanvas");
                                                    clear_canvas("outputCanvas");
                                                    clear_canvas("webcamCanvas");
                                                    source_image_name.set("image.png".to_string());
                                                    download_preview.set(String::new());
                                                    planned_export_count.set(0);
                                                    clear_last_detection_backend();
                                                    if let Some(stream) = webcam_stream.get() {
                                                        stop_media_stream(&stream);
                                                        webcam_stream.set(None);
                                                    }
                                                    clear_video_source("webcamVideo");
                                                    single_state.update(crate::single_core::SingleCoreState::close_webcam_modal);
                                                    single_runtime.update(crate::single_core::SingleRuntimeState::reset);
                                                    worker_state.update(stop_face_worker);
                                                    single_status.set("Cleared current image state.".to_string());
                                                }
                                            >
                                                "Clear Image"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div class="cropped-faces workspace-card" id="croppedFaces">
                                <div class="cropped-faces-header">
                                    <h3>"Cropped Faces:"</h3>
                                    <button
                                        type="button"
                                        id="downloadResultsBtn"
                                        class="small-btn primary"
                                        disabled=move || !can_download.get() || single_busy.get()
                                        on:click=move |_| {
                                            if single_busy.get() {
                                                return;
                                            }
                                            let source = source_image_name.get();
                                            let timestamp_ms = current_timestamp_ms();
                                            let export_settings = settings.get();
                                            let output_format = export_settings.output_format.clone();
                                            let mime_type = match output_format.as_str() {
                                                "jpeg" | "jpg" => "image/jpeg",
                                                "webp" => "image/webp",
                                                _ => "image/png",
                                            };
                                            let plan = build_export_plan(
                                                &single_state.get().selected_face_ids,
                                                &export_settings.naming_template,
                                                &source,
                                                export_settings.output_width,
                                                export_settings.output_height,
                                                &output_format,
                                                timestamp_ms,
                                            );
                                            planned_export_count.set(plan.filenames.len());
                                            if let Some(first) = plan.filenames.first() {
                                                download_preview.set(first.clone());
                                            }
                                            let source_url = source_image_url.get();
                                            let faces = detected_faces.get();
                                            let selected_ids = single_state
                                                .get()
                                                .selected_face_ids
                                                .iter()
                                                .cloned()
                                                .collect::<Vec<_>>();
                                            let status_for_export = single_status;
                                            let runtime_for_export = single_runtime;
                                            leptos::task::spawn_local(async move {
                                                let Some(source_url) = source_url else {
                                                    status_for_export
                                                        .set("No source image loaded for export.".to_string());
                                                    return;
                                                };
                                                let mut selected = selected_ids;
                                                selected.sort();
                                                let mut exported = 0usize;
                                                for (idx, face_id) in selected.into_iter().enumerate() {
                                                    let Some(face) = faces.iter().find(|f| f.id == face_id) else {
                                                        continue;
                                                    };
                                                    let file_name = plan
                                                        .filenames
                                                        .get(idx)
                                                        .cloned()
                                                        .unwrap_or_else(|| format!("face_{}.{}", idx + 1, output_format));
                                                    match crop_face_bytes_from_source(
                                                        &source_url,
                                                        face,
                                                        &export_settings,
                                                        mime_type,
                                                    )
                                                    .await
                                                    {
                                                        Ok(bytes) => {
                                                            let final_name = normalize_export_filename_for_mime(
                                                                &file_name,
                                                                mime_type,
                                                            );
                                                            if !validate_export_filename_for_mime(
                                                                &final_name,
                                                                mime_type,
                                                            ) {
                                                                status_for_export.set(format!(
                                                                    "Skipped invalid export name for {mime_type} blob: {final_name}"
                                                                ));
                                                                return;
                                                            }
                                                            if let Err(error) = download_bytes(
                                                                &final_name,
                                                                mime_type,
                                                                &bytes,
                                                            ) {
                                                                status_for_export.set(format!(
                                                                    "Failed to download {final_name}: {error}"
                                                                ));
                                                                return;
                                                            }
                                                            exported += 1;
                                                        }
                                                        Err(error) => {
                                                            status_for_export.set(format!(
                                                                "Failed to crop {file_name}: {error}"
                                                            ));
                                                            return;
                                                        }
                                                    }
                                                }
                                                runtime_for_export.update(|r| {
                                                    r.complete(0, format!("Exported {exported} cropped face file(s)."));
                                                });
                                                status_for_export
                                                    .set(format!("Exported {exported} cropped face file(s)."));
                                            });
                                        }
                                    >
                                        "Download Results"
                                    </button>
                                </div>
                                <div id="croppedContainer"></div>
                                <div class="setting-help">{move || format!("Planned exports: {}", planned_export_count.get())}</div>
                                <div class="setting-help">{move || format!("Preview export: {}", download_preview.get())}</div>
                            </div>
                        </div>

                        <div class="status workspace-card" id="status">{single_status}</div>
                    </div>
                </main>
            </div>

            <div id="webcamModal" class=move || if webcam_open.get() { "modal" } else { "modal hidden" }>
                <div class="modal-content webcam-modal-content">
                    <div class="modal-header">
                        <h2>"Webcam Capture"</h2>
                        <button
                            type="button"
                            id="closeWebcamBtn"
                            class="close-btn"
                            title="Close"
                            on:click=move |_| {
                                if let Some(stream) = webcam_stream.get() {
                                    stop_media_stream(&stream);
                                    webcam_stream.set(None);
                                }
                                clear_video_source("webcamVideo");
                                single_state.update(crate::single_core::SingleCoreState::close_webcam_modal);
                                single_status.set("Webcam modal closed.".to_string());
                            }
                        >
                            "×"
                        </button>
                    </div>
                    <div class="modal-body">
                        <div class="webcam-container">
                            <video id="webcamVideo" autoplay playsinline></video>
                            <canvas id="webcamCanvas" class="webcam-hidden-canvas"></canvas>
                            <div id="webcamFaceOverlays" class="face-overlays"></div>
                        </div>
                        <div class="webcam-controls">
                            <button
                                type="button"
                                id="captureBtn"
                                class="primary-btn"
                                disabled=move || !webcam_open.get() || single_busy.get()
                                on:click=move |_| {
                                    if single_busy.get() {
                                        return;
                                    }
                                    single_status.set("Capturing webcam frame...".to_string());
                                    let status_for_capture = single_status;
                                    let stream_for_capture = webcam_stream;
                                    let state_for_capture = single_state;
                                    let source_name_for_capture = source_image_name;
                                    let source_file_for_capture = source_image_file;
                                    let source_url_for_capture = source_image_url;
                                    let source_dims_for_capture = source_image_dimensions;
                                    let detected_faces_for_capture = detected_faces;
                                    let core_state_for_capture = single_state;
                                    let runtime_for_capture = single_runtime;
                                    let worker_for_capture = worker_state;
                                    let settings_for_capture = settings.get();
                                    leptos::task::spawn_local(async move {
                                        match capture_webcam_frame_to_file("webcamVideo", "webcamCanvas").await {
                                            Ok(file) => {
                                                if let Some(existing) = source_url_for_capture.get() {
                                                    revoke_object_url(&existing);
                                                }
                                                source_name_for_capture.set(file.name());
                                                source_file_for_capture.set(Some(file.clone()));
                                                source_url_for_capture.set(object_url_for_file(&file));
                                                source_dims_for_capture.set((0.0, 0.0));
                                                core_state_for_capture.update(|s| {
                                                    s.set_faces(Vec::new());
                                                    s.webcam_modal_open = false;
                                                });
                                                detected_faces_for_capture.set(Vec::new());
                                                runtime_for_capture.update(|r| r.start("Detecting faces..."));
                                                worker_for_capture.update(crate::worker_bridge::FaceWorkerBridgeState::mark_request_started);

                                                if let Some(stream) = stream_for_capture.get() {
                                                    stop_media_stream(&stream);
                                                    stream_for_capture.set(None);
                                                }
                                                clear_video_source("webcamVideo");
                                                state_for_capture.update(crate::single_core::SingleCoreState::close_webcam_modal);

                                                match detect_faces_with_worker(
                                                    "browser-face-detector",
                                                    file,
                                                )
                                                .await
                                                {
                                                    Ok(faces) => {
                                                        let filtered = apply_detection_quality_filters(
                                                            faces,
                                                            &settings_for_capture,
                                                        );
                                                        let ids = filtered.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
                                                        let count = ids.len();
                                                        detected_faces_for_capture.set(filtered);
                                                        core_state_for_capture.update(|s| s.set_faces(ids));
                                                        worker_for_capture.update(crate::worker_bridge::FaceWorkerBridgeState::mark_request_succeeded);
                                                        runtime_for_capture.update(|r| {
                                                            r.complete(24, format!("Detected {count} face(s)."));
                                                        });
                                                        status_for_capture.set(if count == 0 {
                                                            "Captured image. Detection completed with no faces.".to_string()
                                                        } else {
                                                            format!("Captured image processed. Found {count} face(s).")
                                                        });
                                                    }
                                                    Err(error) => {
                                                        worker_for_capture.update(|w| w.mark_request_failed(error.clone()));
                                                        runtime_for_capture.update(|r| {
                                                            r.complete(0, format!("Detection failed: {error}"));
                                                        });
                                                        status_for_capture.set(format!(
                                                            "Captured image but detection failed: {error}"
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(error) => {
                                                status_for_capture
                                                    .set(format!("Failed to capture webcam frame: {error}"));
                                            }
                                        }
                                    });
                                }
                            >
                                <span>"📸 Capture Photo"</span>
                            </button>
                            <button
                                type="button"
                                id="switchCameraBtn"
                                class="ghost-btn"
                                disabled=move || !webcam_open.get() || single_busy.get()
                                title="Switch camera"
                                on:click=move |_| {
                                    if single_busy.get() {
                                        return;
                                    }
                                    single_state.update(crate::single_core::SingleCoreState::switch_camera);
                                    let active_name = single_state.get().active_camera_name().to_string();
                                    single_status.set(format!("Switching to {active_name}..."));
                                    let state_for_switch = single_state;
                                    let stream_for_switch = webcam_stream;
                                    let devices_for_switch = webcam_device_ids;
                                    let status_for_switch = single_status;
                                    leptos::task::spawn_local(async move {
                                        if let Some(existing) = stream_for_switch.get() {
                                            stop_media_stream(&existing);
                                            stream_for_switch.set(None);
                                        }
                                        let device_ids = devices_for_switch.get();
                                        if device_ids.is_empty() {
                                            status_for_switch.set("No camera devices available to switch.".to_string());
                                            return;
                                        }
                                        let mut active_index = state_for_switch.get().active_camera_index;
                                        if active_index >= device_ids.len() {
                                            active_index = 0;
                                            state_for_switch.update(|s| s.active_camera_index = 0);
                                        }
                                        let preferred = device_ids.get(active_index).cloned();
                                        match start_webcam_stream("webcamVideo", preferred.as_deref()).await {
                                            Ok(stream) => {
                                                stream_for_switch.set(Some(stream));
                                                status_for_switch.set(format!(
                                                    "Switched to {}.",
                                                    state_for_switch.get().active_camera_name()
                                                ));
                                            }
                                            Err(error) => {
                                                status_for_switch
                                                    .set(format!("Failed to switch camera: {error}"));
                                            }
                                        }
                                    });
                                }
                            >
                                <span>"🔄 Switch Camera"</span>
                            </button>
                            <div class="setting-help">{move || format!("Active camera: {}", active_camera_label.get())}</div>
                        </div>
                    </div>
                </div>
            </div>

        </div>
    }
}

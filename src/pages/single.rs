use crate::components::log_card::LogCard;
use crate::components::panel::Panel;
use crate::components::pill::Badge;
use crate::components::topbar::Topbar;
use crate::export_runtime::{
    current_timestamp_ms, download_bytes, normalize_export_filename_for_mime,
    validate_export_filename_for_mime,
};
use crate::router::Route;
use crate::runtime::{
    ProcessedImageOutput, apply_detection_quality_filters, capture_webcam_frame_to_file,
    clear_video_source, crop_face_bytes_from_source, decode_image_dimensions,
    list_video_input_devices, mime_type_for_output_format, object_url_for_bytes,
    object_url_for_file, overlay_percent_crop_rect, revoke_object_url, start_webcam_stream,
    stop_media_stream,
};
use crate::single_core::{SingleCoreState, SingleRuntimeState, build_export_plan};
use crate::state::{AppState, ProcessingSettings};
use crate::worker_bridge::{
    DetectedFace, FaceWorkerBridgeState, detect_faces_with_worker, last_detection_backend_label,
    start_face_worker,
};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};

#[component]
pub fn Single(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let single_state = RwSignal::new(SingleCoreState::default());
    let worker_state = RwSignal::new(FaceWorkerBridgeState::default());
    let runtime = RwSignal::new(SingleRuntimeState::default());
    let source_name = RwSignal::new("No image loaded".to_string());
    let source_file = RwSignal::new(None::<web_sys::File>);
    let source_url = RwSignal::new(None::<String>);
    let source_dimensions = RwSignal::new((0.0_f64, 0.0_f64));
    let raw_detected_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let detected_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let output_preview = RwSignal::new(None::<ProcessedImageOutput>);
    let output_preview_name = RwSignal::new(String::new());
    let webcam_stream = RwSignal::new(None::<web_sys::MediaStream>);
    let webcam_device_ids = RwSignal::new(Vec::<String>::new());

    Effect::new(move |_| {
        if worker_state.get().can_start() {
            worker_state.update(start_face_worker);
        }
    });

    let has_source = Signal::derive(move || source_file.get().is_some());
    let can_export = Signal::derive(move || single_state.get().selected_count() > 0);
    let busy = Signal::derive(move || runtime.get().running);
    let faces_label = Signal::derive(move || single_state.get().faces_count().to_string());
    let selected_label = Signal::derive(move || single_state.get().selected_count().to_string());
    let status_label = Signal::derive(move || runtime.get().status);
    let backend_label = Signal::derive(move || {
        let _ = worker_state.get();
        last_detection_backend_label().to_string()
    });
    let output_format_label = Signal::derive(move || settings.get().output_format.to_uppercase());

    let load_file = move |file: web_sys::File| {
        load_single_file(
            file,
            settings,
            single_state,
            worker_state,
            runtime,
            source_name,
            source_file,
            source_url,
            source_dimensions,
            raw_detected_faces,
            detected_faces,
            output_preview,
            output_preview_name,
        );
    };

    Effect::new(move |_| {
        let raw_faces = raw_detected_faces.get();
        if raw_faces.is_empty() {
            return;
        }
        refresh_single_filtered_faces(settings.get(), single_state, raw_faces, detected_faces);
    });

    view! {
        <Topbar route set_route />

        <div class="page-head">
            <div>
                <div class="crumb"><span>"Workflow 02"</span><span class="dot"></span><span class="now">"Single Image"</span></div>
                <h1>"Tune one shot, "<span class="grad">"just right"</span>"."</h1>
                <p class="lede">"Load an image, detect faces locally, tune the crop frame, and export the selected face crops."</p>
            </div>
            <div class="meta-pills">
                <span class="pill"><span class="d"></span>{backend_label}</span>
                <span class=move || { if busy.get() { "pill run" } else if has_source.get() { "pill ok" } else { "pill" } }>
                    <span class="d"></span>{status_label}
                </span>
            </div>
        </div>

        <div class="layout">
            <aside class="sidebar">
                <div class="sb-scroll">
                    <Panel title="Input / Output" num="01" initially_open=true accent="cyan">
                        <div
                            class="upload-card single"
                            on:dragover=move |ev: DragEvent| {
                                if !busy.get() {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                }
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
                                load_file(file);
                            }
                        >
                            <label class="upload-label" for="singleImageInput">
                                <div class="icon">
                                    <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="3" y="3" width="16" height="16" rx="2"/><circle cx="9" cy="9" r="2"/><path d="M3 16l5-5 4 4 3-3 4 4"/></svg>
                                </div>
                                <h4>{move || if has_source.get() { source_name.get() } else { "Drop image here".to_string() }}</h4>
                                <p>"Drag an image here, paste from your file picker, or capture from webcam."</p>
                                <div class="actions"><span class="chip">"Browse file"</span></div>
                            </label>
                            <input
                                id="singleImageInput"
                                class="hidden-input"
                                type="file"
                                accept="image/*"
                                disabled=move || busy.get()
                                on:change=move |ev| {
                                    let input: HtmlInputElement = event_target(&ev);
                                    let Some(files) = input.files() else {
                                        return;
                                    };
                                    let Some(file) = files.get(0) else {
                                        return;
                                    };
                                    load_file(file);
                                }
                            />
                        </div>
                        <div class="webcam-row">
                            <div class="l"><div class="ic"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"></path><circle cx="12" cy="13" r="4"></circle></svg></div>"Webcam input"</div>
                            <button
                                class="open"
                                disabled=move || busy.get()
                                on:click=move |_| {
                                    single_state.update(SingleCoreState::open_webcam_modal);
                                    runtime.update(|r| r.set_status("Opening webcam..."));
                                    let runtime_for_camera = runtime;
                                    let streams = webcam_stream;
                                    let devices_signal = webcam_device_ids;
                                    let state_for_camera = single_state;
                                    spawn_local(async move {
                                        match list_video_input_devices().await {
                                            Ok(devices) => {
                                                let ids = devices.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
                                                let labels = devices.into_iter().map(|(_, label)| label).collect::<Vec<_>>();
                                                devices_signal.set(ids.clone());
                                                state_for_camera.update(|s| s.cameras = labels);
                                                match start_webcam_stream("webcamVideo", ids.first().map(String::as_str)).await {
                                                    Ok(stream) => {
                                                        streams.set(Some(stream));
                                                        runtime_for_camera.update(|r| r.set_status("Webcam ready. Capture a frame."));
                                                    }
                                                    Err(error) => runtime_for_camera.update(|r| r.set_status(format!("Webcam failed: {error}"))),
                                                }
                                            }
                                            Err(error) => runtime_for_camera.update(|r| r.set_status(format!("Camera list failed: {error}"))),
                                        }
                                    });
                                }
                            >"Open camera"</button>
                        </div>
                    </Panel>

                    <Panel title="Crop framing" num="02" initially_open=true accent="cyan">
                        <div class="field">
                            <label>"Aspect ratio"</label>
                            <div class="seg cols-4">
                                <button class=move || single_aspect_class(&settings.get(), 512, 512) on:click=move |_| settings.update(|s| { s.output_width = 512; s.output_height = 512; })>"1:1"</button>
                                <button class=move || single_aspect_class(&settings.get(), 640, 800) on:click=move |_| settings.update(|s| { s.output_width = 640; s.output_height = 800; })>"4:5"</button>
                                <button class=move || single_aspect_class(&settings.get(), 600, 800) on:click=move |_| settings.update(|s| { s.output_width = 600; s.output_height = 800; })>"3:4"</button>
                                <button class=move || single_aspect_class(&settings.get(), 768, 512) on:click=move |_| settings.update(|s| { s.output_width = 768; s.output_height = 512; })>"3:2"</button>
                            </div>
                        </div>
                        <div class="field">
                            <label>"Face size - " {move || settings.get().face_height_pct.to_string()} "%"</label>
                            <div class="slider-row">
                                <input
                                    type="range"
                                    class="slider"
                                    min="10"
                                    max="95"
                                    prop:value=move || settings.get().face_height_pct.to_string()
                                    on:input=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<u8>() {
                                            settings.update(|s| {
                                                s.face_height_pct = value.clamp(10, 95);
                                            });
                                        }
                                    }
                                />
                                <span class="num">{move || format!("{}%", settings.get().face_height_pct)}</span>
                            </div>
                        </div>
                        <div class="field">
                            <label>"Confidence threshold"</label>
                            <input
                                type="range"
                                class="slider"
                                min="0"
                                max="99"
                                prop:value=move || (settings.get().min_confidence * 100.0).round().to_string()
                                on:input=move |ev| {
                                    if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                        settings.update(|s| s.min_confidence = (value / 100.0).clamp(0.0, 0.99));
                                    }
                                }
                            />
                        </div>
                    </Panel>

                    <Panel title="Output format" num="03" initially_open=true accent="cyan">
                        <div class="field">
                            <label>"Format"</label>
                            <div class="seg cols-3">
                                <button class=move || format_button_class(&settings.get().output_format, "jpeg", "cyan") on:click=move |_| settings.update(|s| s.output_format = "jpeg".to_string())>"JPG"</button>
                                <button class=move || format_button_class(&settings.get().output_format, "png", "cyan") on:click=move |_| settings.update(|s| s.output_format = "png".to_string())>"PNG"</button>
                                <button class=move || format_button_class(&settings.get().output_format, "webp", "cyan") on:click=move |_| settings.update(|s| s.output_format = "webp".to_string())>"WEBP"</button>
                            </div>
                        </div>
                        <div class="field row2">
                            <div>
                                <label>"Width"</label>
                                <input
                                    class="input"
                                    prop:value=move || settings.get().output_width.to_string()
                                    on:change=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<u32>() {
                                            settings.update(|s| s.output_width = value.max(1));
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label>"Height"</label>
                                <input
                                    class="input"
                                    prop:value=move || settings.get().output_height.to_string()
                                    on:change=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<u32>() {
                                            settings.update(|s| s.output_height = value.max(1));
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </Panel>

                    <Panel title="Diagnostics" num="04" initially_open=true accent="cyan">
                        <div class="field row2">
                            <div><label>"Faces"</label><div class="diag-value cyan">{faces_label}</div></div>
                            <div><label>"Selected"</label><div class="diag-value">{selected_label}</div></div>
                        </div>
                    </Panel>
                </div>
            </aside>

            <div class="workspace">
                <div class="canvas-card">
                    <div class="canvas-head">
                        <div class="left"><b>{source_name}</b><span>{move || dimensions_label(source_dimensions.get())}</span></div>
                        <div class="ctlrow">
                            <span class="ctl">"format "<b>{output_format_label}</b></span>
                            <span class="ctl">"faces "<b>{faces_label}</b></span>
                        </div>
                    </div>
                    <div class="canvas-body">
                        <div class="stage">
                            {move || {
                                if let Some(src) = source_url.get() {
                                    view! { <img class="stage-image" src=src alt="Source image" /> }.into_any()
                                } else {
                                    view! {
                                        <svg viewBox="0 0 100 100">
                                            <defs>
                                                <linearGradient id="single-empty" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#121a24"/><stop offset="1" stop-color="#0a0f18"/></linearGradient>
                                            </defs>
                                            <rect width="100" height="100" fill="url(#single-empty)"/>
                                            <circle cx="48" cy="42" r="16" fill="#2a365a"/>
                                            <ellipse cx="48" cy="86" rx="26" ry="20" fill="#2a365a"/>
                                        </svg>
                                    }.into_any()
                                }
                            }}
                            {move || {
                                let (source_width, source_height) = source_dimensions.get();
                                if source_width <= 0.0 || source_height <= 0.0 {
                                    return Vec::new();
                                }
                                let state = single_state.get();
                                let selected = state.selected_face_ids;
                                let current_settings = settings.get();
                                detected_faces
                                    .get()
                                    .into_iter()
                                    .map(|face| {
                                        let selected_face = selected.contains(&face.id);
                                        let class_name = if selected_face { "frm alt" } else { "frm unsel" };
                                        let (left, top, width, height) = overlay_percent_crop_rect(
                                            &face,
                                            source_width,
                                            source_height,
                                            &current_settings,
                                        );
                                        let style = format!(
                                            "left:{left:.2}%;top:{top:.2}%;width:{width:.2}%;height:{height:.2}%"
                                        );
                                        let id = face.id.clone();
                                        let confidence = format!("{:.2}", face.confidence);
                                        view! {
                                            <button
                                                type="button"
                                                class=class_name
                                                style=style
                                                data-c=confidence
                                                on:click=move |_| single_state.update(|s| s.toggle_face_selection(&id))
                                            ></button>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </div>

                        <div class="canvas-pane">
                            <h4>"Detected faces"<span>{faces_label}</span></h4>
                            <div class="face-chips">
                                {move || {
                                    let selected = single_state.get().selected_face_ids;
                                    detected_faces
                                        .get()
                                        .into_iter()
                                        .map(|face| {
                                            let id = face.id.clone();
                                            let selected_face = selected.contains(&id);
                                            view! {
                                                <button
                                                    type="button"
                                                    class=if selected_face { "face-chip" } else { "face-chip off" }
                                                    on:click=move |_| single_state.update(|s| s.toggle_face_selection(&id))
                                                >
                                                    <span class="check">{if selected_face { "✓" } else { "" }}</span>
                                                    {format!("{} - {:.2}", face.id, face.confidence)}
                                                </button>
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                                {move || if detected_faces.get().is_empty() {
                                    Some(view! { <span class="empty-note">"No faces detected yet."</span> })
                                } else {
                                    None
                                }}
                            </div>

                            <h4 style="margin-top:auto">"Output preview"</h4>
                            <div class="out-grid-col">
                                <div class="out-card-single">
                                    <div class="thumb">
                                        {move || {
                                            if let Some(output) = output_preview.get() {
                                                view! { <img src=output.preview_url alt="Cropped output preview" /> }.into_any()
                                            } else {
                                                view! { <div class="empty-thumb">"No export"</div> }.into_any()
                                            }
                                        }}
                                    </div>
                                    <div class="meta">
                                        <div class="nm">{move || if output_preview_name.get().is_empty() { "No cropped file generated".to_string() } else { output_preview_name.get() }}</div>
                                        <div class="sub">{move || output_meta_label(output_preview.get())}</div>
                                        <Badge variant="ok">{move || if output_preview.get().is_some() { "ready" } else { "waiting" }}</Badge>
                                    </div>
                                    <button
                                        class="dl"
                                        disabled=move || !can_export.get() || busy.get()
                                        on:click=move |_| export_selected_faces(
                                            settings,
                                            single_state,
                                            runtime,
                                            source_name,
                                            source_url,
                                            detected_faces,
                                            output_preview,
                                            output_preview_name,
                                        )
                                    >"Save"</button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <LogCard title="Pipeline" accent="cyan" meta_left="runtime" meta_right="">
                    {move || runtime.get().logs.into_iter().rev().map(|entry| {
                        let variant = if entry.to_lowercase().contains("fail") || entry.to_lowercase().contains("error") { "err" } else { "" };
                        view! { <div class=format!("line {variant}")><span class="t">"runtime"</span><span class="m">{entry}</span></div> }
                    }).collect::<Vec<_>>()}
                </LogCard>

                <div style="height:120px"></div>
            </div>
        </div>

        <div class=move || { if single_state.get().webcam_modal_open { "modal" } else { "modal hidden" } }>
            <div class="modal-content webcam-modal-content">
                <div class="modal-header">
                    <h2>"Webcam Capture"</h2>
                    <button
                        type="button"
                        class="close-btn"
                        on:click=move |_| {
                            if let Some(stream) = webcam_stream.get() {
                                stop_media_stream(&stream);
                                webcam_stream.set(None);
                            }
                            clear_video_source("webcamVideo");
                            single_state.update(SingleCoreState::close_webcam_modal);
                            runtime.update(|r| r.set_status("Webcam closed."));
                        }
                    >"x"</button>
                </div>
                <div class="modal-body">
                    <div class="webcam-container">
                        <video id="webcamVideo" autoplay playsinline></video>
                        <canvas id="webcamCanvas" class="webcam-hidden-canvas"></canvas>
                    </div>
                    <div class="webcam-controls">
                        <button
                            type="button"
                            class="btn btn-cyan"
                            disabled=move || busy.get()
                            on:click=move |_| {
                                runtime.update(|r| r.set_status("Capturing webcam frame..."));
                                let stream_signal = webcam_stream;
                                spawn_local(async move {
                                    match capture_webcam_frame_to_file("webcamVideo", "webcamCanvas").await {
                                        Ok(file) => {
                                            if let Some(stream) = stream_signal.get() {
                                                stop_media_stream(&stream);
                                                stream_signal.set(None);
                                            }
                                            clear_video_source("webcamVideo");
                                            single_state.update(SingleCoreState::close_webcam_modal);
                                            load_single_file(
                                                file,
                                                settings,
                                                single_state,
                                                worker_state,
                                                runtime,
                                                source_name,
                                                source_file,
                                                source_url,
                                                source_dimensions,
                                                raw_detected_faces,
                                                detected_faces,
                                                output_preview,
                                                output_preview_name,
                                            );
                                        }
                                        Err(error) => runtime.update(|r| r.set_status(format!("Capture failed: {error}"))),
                                    }
                                });
                            }
                        >"Capture"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn load_single_file(
    file: web_sys::File,
    settings: RwSignal<ProcessingSettings>,
    single_state: RwSignal<SingleCoreState>,
    worker_state: RwSignal<FaceWorkerBridgeState>,
    runtime: RwSignal<SingleRuntimeState>,
    source_name: RwSignal<String>,
    source_file: RwSignal<Option<web_sys::File>>,
    source_url: RwSignal<Option<String>>,
    source_dimensions: RwSignal<(f64, f64)>,
    raw_detected_faces: RwSignal<Vec<DetectedFace>>,
    detected_faces: RwSignal<Vec<DetectedFace>>,
    output_preview: RwSignal<Option<ProcessedImageOutput>>,
    output_preview_name: RwSignal<String>,
) {
    if let Some(existing) = source_url.get() {
        revoke_object_url(&existing);
    }
    if let Some(existing) = output_preview.get() {
        revoke_object_url(&existing.preview_url);
    }

    let Some(url) = object_url_for_file(&file) else {
        runtime.update(|r| r.set_status("Could not create image preview URL."));
        return;
    };

    source_name.set(file.name());
    source_file.set(Some(file.clone()));
    source_url.set(Some(url));
    source_dimensions.set((0.0, 0.0));
    raw_detected_faces.set(Vec::new());
    detected_faces.set(Vec::new());
    output_preview.set(None);
    output_preview_name.set(String::new());
    single_state.update(|state| state.set_faces(Vec::new()));
    worker_state.update(FaceWorkerBridgeState::mark_request_started);
    runtime.update(|state| state.start("Image loaded. Running face detection..."));

    spawn_local(async move {
        let start_ms = crate::runtime::now_ms();
        #[cfg(target_arch = "wasm32")]
        let decode_start = crate::runtime::now_ms();
        let maybe_dims = match decode_image_dimensions(&file).await {
            Ok(dimensions) => {
                source_dimensions.set((f64::from(dimensions.width), f64::from(dimensions.height)));
                Some(dimensions)
            }
            Err(error) => {
                runtime.update(|state| {
                    state.set_status(format!("Image dimension read failed: {error}"))
                });
                None
            }
        };
        #[cfg(target_arch = "wasm32")]
        let decode_ms = crate::runtime::elapsed_ms_since(decode_start);

        let (detection_file, detection_scale) = if let Some(dims) = maybe_dims {
            match crate::runtime::maybe_downscale_for_detection(
                &file,
                dims,
                crate::runtime::MAX_DETECTION_SIDE,
            )
            .await
            {
                Ok(pair) => pair,
                Err(_) => (file, 1.0),
            }
        } else {
            (file, 1.0)
        };

        #[cfg(target_arch = "wasm32")]
        let detect_start = crate::runtime::now_ms();
        match detect_faces_with_worker("browser-face-detector", detection_file).await {
            Ok(raw) => {
                #[cfg(target_arch = "wasm32")]
                let detect_ms = crate::runtime::elapsed_ms_since(detect_start);
                let faces = crate::runtime::scale_detected_faces(
                    raw,
                    1.0 / detection_scale,
                    1.0 / detection_scale,
                );
                raw_detected_faces.set(faces.clone());
                let filtered = apply_detection_quality_filters(faces, &settings.get());
                let ids = filtered
                    .iter()
                    .map(|face| face.id.clone())
                    .collect::<Vec<_>>();
                let count = ids.len();
                detected_faces.set(filtered);
                single_state.update(|state| state.set_faces(ids));
                worker_state.update(FaceWorkerBridgeState::mark_request_succeeded);
                let total_ms = crate::runtime::elapsed_ms_since(start_ms);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!(
                        "[FCF timing] single: decode={decode_ms}ms detect={detect_ms}ms total={total_ms}ms backend={}",
                        crate::worker_bridge::last_detection_backend_label()
                    )
                    .into(),
                );
                runtime.update(|state| {
                    state.complete(
                        total_ms,
                        if count == 0 {
                            "Detection completed. No faces found.".to_string()
                        } else {
                            format!("Detection completed. Found {count} face(s).")
                        },
                    );
                });
            }
            Err(error) => {
                detected_faces.set(Vec::new());
                worker_state.update(|state| state.mark_request_failed(error.clone()));
                runtime.update(|state| {
                    state.complete(
                        crate::runtime::elapsed_ms_since(start_ms),
                        format!("Face detection failed: {error}"),
                    );
                });
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn export_selected_faces(
    settings: RwSignal<ProcessingSettings>,
    single_state: RwSignal<SingleCoreState>,
    runtime: RwSignal<SingleRuntimeState>,
    source_name: RwSignal<String>,
    source_url: RwSignal<Option<String>>,
    detected_faces: RwSignal<Vec<DetectedFace>>,
    output_preview: RwSignal<Option<ProcessedImageOutput>>,
    output_preview_name: RwSignal<String>,
) {
    let Some(source_url_value) = source_url.get() else {
        runtime.update(|state| state.set_status("No source image is loaded."));
        return;
    };

    let state = single_state.get();
    if state.selected_face_ids.is_empty() {
        runtime.update(|state| state.set_status("Select at least one face to export."));
        return;
    }

    let export_settings = settings.get();
    let mime_type = mime_type_for_output_format(&export_settings.output_format).to_string();
    let timestamp_ms = current_timestamp_ms();
    let plan = build_export_plan(
        &state.selected_face_ids,
        &export_settings.naming_template,
        &source_name.get(),
        export_settings.output_width,
        export_settings.output_height,
        &export_settings.output_format,
        timestamp_ms,
    );
    let selected_ids = {
        let mut ids = state.selected_face_ids.iter().cloned().collect::<Vec<_>>();
        ids.sort();
        ids
    };
    let faces = detected_faces.get();

    runtime.update(|state| state.start("Exporting selected face crop(s)..."));
    spawn_local(async move {
        let mut exported = 0usize;
        for (index, face_id) in selected_ids.iter().enumerate() {
            let Some(face) = faces.iter().find(|face| &face.id == face_id) else {
                continue;
            };
            let file_name =
                plan.filenames.get(index).cloned().unwrap_or_else(|| {
                    format!("face_{}.{}", index + 1, export_settings.output_format)
                });
            #[cfg(target_arch = "wasm32")]
            let crop_start = crate::runtime::now_ms();
            match crop_face_bytes_from_source(&source_url_value, face, &export_settings, &mime_type)
                .await
            {
                Ok(bytes) => {
                    #[cfg(target_arch = "wasm32")]
                    let crop_ms = crate::runtime::elapsed_ms_since(crop_start);
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!("[FCF timing] single export: {file_name} crop={crop_ms}ms")
                            .into(),
                    );
                    let final_name = normalize_export_filename_for_mime(&file_name, &mime_type);
                    if !validate_export_filename_for_mime(&final_name, &mime_type) {
                        runtime.update(|state| {
                            state.complete(0, format!("Invalid export name: {final_name}"))
                        });
                        return;
                    }
                    if let Some(existing) = output_preview.get() {
                        revoke_object_url(&existing.preview_url);
                    }
                    match object_url_for_bytes(&bytes, &mime_type) {
                        Ok(preview_url) => {
                            output_preview_name.set(final_name.clone());
                            output_preview.set(Some(ProcessedImageOutput {
                                bytes: bytes.clone(),
                                mime_type: mime_type.clone(),
                                preview_url,
                            }));
                        }
                        Err(error) => {
                            runtime.update(|state| {
                                state.set_status(format!("Preview failed: {error}"))
                            });
                        }
                    }
                    if let Err(error) = download_bytes(&final_name, &mime_type, &bytes) {
                        runtime
                            .update(|state| state.complete(0, format!("Download failed: {error}")));
                        return;
                    }
                    exported += 1;
                }
                Err(error) => {
                    runtime.update(|state| state.complete(0, format!("Crop failed: {error}")));
                    return;
                }
            }
        }
        runtime.update(|state| {
            state.complete(0, format!("Exported {exported} cropped face file(s)."))
        });
    });
}

fn dimensions_label(dimensions: (f64, f64)) -> String {
    if dimensions.0 <= 0.0 || dimensions.1 <= 0.0 {
        "Awaiting image dimensions".to_string()
    } else {
        format!("{:.0} x {:.0}", dimensions.0, dimensions.1)
    }
}

fn refresh_single_filtered_faces(
    settings: ProcessingSettings,
    single_state: RwSignal<SingleCoreState>,
    raw_faces: Vec<DetectedFace>,
    detected_faces: RwSignal<Vec<DetectedFace>>,
) {
    let filtered = apply_detection_quality_filters(raw_faces, &settings);
    let ids = filtered
        .iter()
        .map(|face| face.id.clone())
        .collect::<Vec<_>>();
    detected_faces.set(filtered);
    single_state.update(|state| {
        let previous_selection = state.selected_face_ids.clone();
        state.all_face_ids.clone_from(&ids);
        state.selected_face_ids = ids
            .iter()
            .filter(|id| previous_selection.contains(*id))
            .cloned()
            .collect();
        if state.selected_face_ids.is_empty() {
            state.selected_face_ids = ids.into_iter().collect();
        }
    });
}

fn format_button_class(current: &str, expected: &str, accent: &str) -> String {
    if current.eq_ignore_ascii_case(expected)
        || (expected == "jpeg" && current.eq_ignore_ascii_case("jpg"))
    {
        format!("on {accent}")
    } else {
        String::new()
    }
}

fn single_aspect_class(settings: &ProcessingSettings, width: u32, height: u32) -> &'static str {
    if settings.output_width == width && settings.output_height == height {
        "on cyan"
    } else {
        ""
    }
}

fn output_meta_label(output: Option<ProcessedImageOutput>) -> String {
    output.map_or_else(
        || "Generate a crop to preview the export.".to_string(),
        |output| format!("{} bytes - {}", output.bytes.len(), output.mime_type),
    )
}

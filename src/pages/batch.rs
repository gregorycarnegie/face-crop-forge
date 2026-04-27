use crate::base_runtime::{ImageMeta, ImageValidationConfig, validate_image_meta};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats, ImageStatus};
use crate::batch_export::BatchProgress;
use crate::components::log_card::LogCard;
use crate::components::panel::Panel;
use crate::components::progress_bar::ProgressBar;
use crate::components::topbar::Topbar;
use crate::export_runtime::{
    build_zip_bytes, current_timestamp_ms, current_utc_timestamp_token, download_bytes,
    normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::router::Route;
use crate::runtime::{
    ProcessedImageOutput, apply_detection_quality_filters, batch_file_label,
    crop_face_bytes_from_source, decode_image_dimensions, elapsed_ms_since,
    files_from_data_transfer, is_probably_image_file, make_file_id, mime_type_for_output_format,
    object_url_for_bytes, object_url_for_file, revoke_object_url, revoke_preview_urls,
};
use crate::single_core::generate_face_filename;
use crate::state::{AppState, ProcessingSettings};
use crate::worker_bridge::detect_faces_with_worker;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};

#[component]
pub fn Batch(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let batch_state = RwSignal::new(BatchCoreState::default());
    let batch_queue = RwSignal::new(BatchQueueState::default());
    let files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let outputs = RwSignal::new(HashMap::<String, ProcessedImageOutput>::new());
    let progress = RwSignal::new(BatchProgress::default());
    let progress_pct = RwSignal::new(0_u32);
    let stats = RwSignal::new(BatchRuntimeStats::default());
    let continue_on_error = RwSignal::new(true);
    let padding_pct = RwSignal::new(15_i32);

    let busy = Signal::derive(move || progress.get().running);
    let has_images = Signal::derive(move || batch_state.get().total_count() > 0);
    let selected_count = Signal::derive(move || batch_state.get().selected_count());
    let processed_count = Signal::derive(move || {
        batch_state
            .get()
            .images
            .values()
            .filter(|image| image.processed)
            .count()
    });
    let failed_count = Signal::derive(move || progress.get().failed);
    let faces_detected = Signal::derive(move || stats.get().total_faces_detected);
    let img_per_second = Signal::derive(move || {
        let avg = stats.get().avg_processing_time_ms();
        if avg == 0 {
            "0.0".to_string()
        } else {
            format!("{:.1}", 1000.0 / avg as f64)
        }
    });

    let load_files = move |files: Vec<web_sys::File>| {
        load_batch_files(
            files,
            batch_state,
            batch_queue,
            files_by_id,
            preview_urls,
            outputs,
            progress,
            progress_pct,
            stats,
        );
    };

    view! {
        <Topbar route set_route />

        <div class="page-head">
            <div>
                <div class="crumb"><span>"Workflow 01"</span><span class="dot"></span><span class="now" style="color:var(--peach)">"Batch Processing"</span></div>
                <h1>"Process by the "<span class="grad" style="background:linear-gradient(110deg,var(--peach),var(--rose));-webkit-background-clip:text;background-clip:text;color:transparent">"folder"</span>"."</h1>
                <p class="lede">"Drop a folder or image set. Detection, cropping, previews, and ZIP export all run locally in the browser."</p>
            </div>
            <div class="meta-pills">
                <span class="pill"><span class="d"></span>"Runtime - browser"</span>
                <span class=move || { if has_images.get() { "pill ok" } else { "pill" } }><span class="d"></span>{move || format!("{} queued", batch_state.get().total_count())}</span>
                <span class=move || { if busy.get() { "pill run" } else { "pill" } }><span class="d"></span>{move || progress.get().status}</span>
            </div>
        </div>

        <div class="toolbar">
            <div class="group">
                <button
                    class="btn btn-peach"
                    disabled=move || !has_images.get() || busy.get()
                    on:click=move |_| process_batch(
                        settings,
                        batch_state,
                        batch_queue,
                        files_by_id,
                        preview_urls,
                        outputs,
                        progress,
                        progress_pct,
                        stats,
                        continue_on_error.get(),
                    )
                >"Start processing"</button>
            </div>
            <span class="sep"></span>
            <button
                class="btn btn-ghost"
                disabled=move || outputs.get().is_empty() || busy.get()
                on:click=move |_| download_batch_zip(settings, batch_state, outputs, progress, stats)
            >"Download ZIP"</button>
            <span class="sep"></span>
            <button
                class="btn btn-danger"
                style="margin-left:auto"
                disabled=move || busy.get()
                on:click=move |_| clear_batch(batch_state, batch_queue, files_by_id, preview_urls, outputs, progress, progress_pct, stats)
            >"Clear queue"</button>
            <div class="status-mini">
                <span class="live"></span>
                <span><b>{img_per_second}</b>" img/s"</span>
            </div>
        </div>

        <div class="runbar">
            <div class="top">
                <div class="nm"><span class="live"></span><b>"batch run"</b><span>{move || format!(" - processed {} of {}", progress.get().processed, progress.get().total)}</span></div>
                <div class="pct"><b>{move || format!("{}%", progress_pct.get())}</b></div>
            </div>
            <ProgressBar progress=progress_pct />
            <div class="meta">
                <span>{move || progress.get().status}</span>
                <span>"Selected "<b>{move || selected_count.get().to_string()}</b></span>
            </div>
        </div>

        <div class="layout">
            <aside class="sidebar">
                <div class="sb-scroll">
                    <Panel title="Input / Output" num="01" initially_open=true accent="peach">
                        <div
                            class="upload-card batch"
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
                                let progress_for_drop = progress;
                                spawn_local(async move {
                                    match files_from_data_transfer(data).await {
                                        Ok(files) => load_files(files),
                                        Err(error) => progress_for_drop.update(|p| p.status = format!("Drop failed: {error}")),
                                    }
                                });
                            }
                        >
                            <label class="upload-label" for="batchImageInput">
                                <div class="icon">
                                    <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="3" y="3" width="16" height="16" rx="2"/><path d="M3 8h16M8 3v16M14 3v16"/></svg>
                                </div>
                                <h4>"Drop folder here"</h4>
                                <p>"Subdirectories are traversed when the browser exposes folder entries."</p>
                                <div class="actions"><span class="chip">"Browse files"</span><span class="chip">"Drop folder"</span></div>
                                <div class="breakdown">
                                    <div><b>{move || batch_state.get().total_count().to_string()}</b>"images"</div>
                                    <div><b style="color:var(--peach)">{move || processed_count.get().to_string()}</b>"done"</div>
                                    <div><b style="color:var(--rose)">{move || failed_count.get().to_string()}</b>"failed"</div>
                                </div>
                            </label>
                            <input
                                id="batchImageInput"
                                class="hidden-input"
                                type="file"
                                accept="image/*"
                                multiple
                                disabled=move || busy.get()
                                on:change=move |ev| {
                                    let input: HtmlInputElement = event_target(&ev);
                                    let Some(file_list) = input.files() else {
                                        return;
                                    };
                                    let mut files = Vec::new();
                                    for index in 0..file_list.length() {
                                        if let Some(file) = file_list.get(index) {
                                            files.push(file);
                                        }
                                    }
                                    load_files(files);
                                }
                            />
                        </div>
                    </Panel>

                    <Panel title="Crop framing" num="02" initially_open=true accent="peach">
                        <div class="field">
                            <label>"Aspect ratio"</label>
                            <div class="seg cols-4">
                                <button class=move || { if settings.get().output_width == settings.get().output_height { "on" } else { "" } } on:click=move |_| settings.update(|s| { s.output_width = 512; s.output_height = 512; })>"1:1"</button>
                                <button on:click=move |_| settings.update(|s| { s.output_width = 640; s.output_height = 800; })>"4:5"</button>
                                <button on:click=move |_| settings.update(|s| { s.output_width = 600; s.output_height = 800; })>"3:4"</button>
                                <button on:click=move |_| settings.update(|s| { s.output_width = 768; s.output_height = 512; })>"3:2"</button>
                            </div>
                        </div>
                        <div class="field">
                            <label>"Padding - " {move || padding_pct.get().to_string()} "%"</label>
                            <div class="slider-row">
                                <input
                                    type="range"
                                    class="slider"
                                    min="0"
                                    max="45"
                                    prop:value=move || padding_pct.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<i32>() {
                                            padding_pct.set(value);
                                            settings.update(|s| {
                                                s.face_height_pct = (100 - value.saturating_mul(2)).clamp(10, 95) as u8;
                                            });
                                        }
                                    }
                                />
                                <span class="num">{move || format!("{}%", padding_pct.get())}</span>
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

                    <Panel title="Output format" num="03" initially_open=false accent="peach">
                        <div class="field">
                            <label>"Format"</label>
                            <div class="seg cols-3">
                                <button class=move || batch_format_class(&settings.get().output_format, "jpeg") on:click=move |_| settings.update(|s| s.output_format = "jpeg".to_string())>"JPG"</button>
                                <button class=move || batch_format_class(&settings.get().output_format, "png") on:click=move |_| settings.update(|s| s.output_format = "png".to_string())>"PNG"</button>
                                <button class=move || batch_format_class(&settings.get().output_format, "webp") on:click=move |_| settings.update(|s| s.output_format = "webp".to_string())>"WEBP"</button>
                            </div>
                        </div>
                        <div class="field row2">
                            <div>
                                <label>"Width"</label>
                                <input
                                    class="input"
                                    prop:value=move || settings.get().output_width.to_string()
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                            settings.update(|s| s.output_width = v.max(1));
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
                                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                            settings.update(|s| s.output_height = v.max(1));
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </Panel>

                    <Panel title="Performance" num="04" initially_open=false accent="peach">
                        <label class="toggle-row">
                            <span><span>"Continue on error"</span><span class="desc">"Skip failures and keep processing"</span></span>
                            <input type="checkbox" prop:checked=move || continue_on_error.get() on:change=move |ev| continue_on_error.set(event_target_checked(&ev)) />
                        </label>
                    </Panel>
                </div>
            </aside>

            <div class="workspace">
                <div class="strip strip-4">
                    <div class="s peach"><div class="k">"queued"</div><div class="v"><em>{move || batch_state.get().total_count().to_string()}</em></div></div>
                    <div class="s peach"><div class="k">"processed"</div><div class="v"><em>{move || processed_count.get().to_string()}</em><span style="font-size:14px;color:var(--ink-3)">" / " {move || batch_state.get().total_count().to_string()}</span></div><div class="sub">{move || format!("{}% complete", progress_pct.get())}</div></div>
                    <div class="s peach"><div class="k">"detected"</div><div class="v"><em>{move || faces_detected.get().to_string()}</em></div><div class="sub">"faces found"</div></div>
                    <div class="s rose"><div class="k">"failed"</div><div class="v"><em>{move || failed_count.get().to_string()}</em></div><div class="sub">"skipped"</div></div>
                </div>

                <div class="gallery-card">
                    <div class="gallery-head">
                        <div style="display:flex;align-items:center;gap:14px;flex-wrap:wrap">
                            <h3>"Input queue - "<b>{move || batch_state.get().total_count().to_string()}</b>" files"</h3>
                            <div class="filter-tabs">
                                <button class="on">"All "<span class="n">{move || batch_state.get().total_count().to_string()}</span></button>
                                <button>"Done "<span class="n">{move || processed_count.get().to_string()}</span></button>
                                <button>"Failed "<span class="n">{move || failed_count.get().to_string()}</span></button>
                            </div>
                        </div>
                    </div>
                    <div class="gallery">
                        {move || {
                            let state = batch_state.get();
                            let previews = preview_urls.get();
                            let mut ids = state.images.keys().cloned().collect::<Vec<_>>();
                            ids.sort();
                            if ids.is_empty() {
                                return vec![view! { <div class="empty-gallery">"Drop images to populate the queue."</div> }.into_any()];
                            }
                            ids.into_iter().map(|id| {
                                let image = state.images.get(&id).cloned();
                                let selected = image.as_ref().is_some_and(|img| img.selected);
                                let status = image.as_ref().map(|img| img.status.clone()).unwrap_or(ImageStatus::Loaded);
                                let (badge_class, badge_label) = status_badge(&status);
                                let class_name = gallery_cell_class(selected, &status);
                                let preview = previews.get(&id).cloned();
                                let label = batch_file_label(&id).to_string();
                                let id_for_click = id.clone();
                                view! {
                                    <button
                                        type="button"
                                        class=class_name
                                        disabled=move || busy.get()
                                        on:click=move |_| batch_state.update(|state| state.toggle_selection(&id_for_click))
                                    >
                                        {preview.map(|src| view! { <img src=src alt="Input preview" /> })}
                                        <span class="check">{if selected { "✓" } else { "" }}</span>
                                        <span class="name">{label}</span>
                                        <span class=badge_class>{badge_label}</span>
                                    </button>
                                }.into_any()
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>

                <div class="out-card">
                    <div class="out-head">
                        <h3>"Cropped "<b>"Outputs"</b></h3>
                        <div class="meta">{move || format!("{} images generated", outputs.get().len())}</div>
                    </div>
                    <div class="out-grid">
                        {move || {
                            let outputs_map = outputs.get();
                            if outputs_map.is_empty() {
                                return vec![view! { <div class="empty-output">"Processed crops will appear here."</div> }.into_any()];
                            }
                            let mut ids = outputs_map.keys().cloned().collect::<Vec<_>>();
                            ids.sort();
                            ids.into_iter().filter_map(|id| {
                                outputs_map.get(&id).map(|output| {
                                    let name = batch_file_label(&id).to_string();
                                    view! {
                                        <div class="out">
                                            <div class="img"><img src=output.preview_url.clone() alt="Cropped output" /></div>
                                            <div class="nm">{name}</div>
                                        </div>
                                    }.into_any()
                                })
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>

                <LogCard title="Processing" accent="peach" meta_left="batch" meta_right="">
                    {move || stats.get().logs.into_iter().rev().take(30).map(|entry| {
                        let lower = entry.to_lowercase();
                        let variant = if lower.contains("failed") || lower.contains("error") { "err" } else if lower.contains("processed") || lower.contains("exported") { "ok" } else { "" };
                        view! { <div class=format!("line {variant}")><span class="t">"batch"</span><span class="m">{entry}</span></div> }
                    }).collect::<Vec<_>>()}
                </LogCard>

                <div style="height:120px"></div>
            </div>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn load_batch_files(
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
fn process_batch(
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
        let total = selected_ids.len();
        let validation = ImageValidationConfig::default();
        for (index, id) in selected_ids.into_iter().enumerate() {
            let start_ms = crate::runtime::now_ms();
            let Some(file) = files.get(&id).cloned() else {
                record_batch_failure(
                    progress,
                    stats,
                    batch_state,
                    progress_pct,
                    &id,
                    0,
                    "Missing source file.",
                    "Missing source file.",
                );
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
                        progress,
                        stats,
                        batch_state,
                        progress_pct,
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
                    progress,
                    stats,
                    batch_state,
                    progress_pct,
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
                        progress,
                        stats,
                        batch_state,
                        progress_pct,
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
                    progress,
                    stats,
                    batch_state,
                    progress_pct,
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
                    progress,
                    stats,
                    batch_state,
                    progress_pct,
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
                        progress,
                        stats,
                        batch_state,
                        progress_pct,
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
                        progress,
                        stats,
                        batch_state,
                        progress_pct,
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

fn record_batch_failure(
    progress: RwSignal<BatchProgress>,
    stats: RwSignal<BatchRuntimeStats>,
    batch_state: RwSignal<BatchCoreState>,
    progress_pct: RwSignal<u32>,
    id: &str,
    elapsed_ms: u64,
    status: impl Into<String>,
    log: impl Into<String>,
) {
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

fn download_batch_zip(
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

fn clear_batch(
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
    batch_state.set(BatchCoreState::default());
    batch_queue.set(BatchQueueState::default());
    files_by_id.set(HashMap::new());
    preview_urls.set(HashMap::new());
    outputs.set(HashMap::new());
    progress.set(BatchProgress::default());
    progress_pct.set(0);
    stats.set(BatchRuntimeStats::default());
}

fn status_badge(status: &ImageStatus) -> (&'static str, &'static str) {
    match status {
        ImageStatus::Loaded => ("badge queued", "queued"),
        ImageStatus::Processing => ("badge run", "running"),
        ImageStatus::Processed => ("badge ok", "done"),
        ImageStatus::Error => ("badge fail", "failed"),
    }
}

fn gallery_cell_class(selected: bool, status: &ImageStatus) -> String {
    let mut class_name = "gcell".to_string();
    if selected {
        class_name.push_str(" sel");
    }
    match status {
        ImageStatus::Processing => class_name.push_str(" processing"),
        ImageStatus::Error => class_name.push_str(" failed"),
        ImageStatus::Loaded | ImageStatus::Processed => {}
    }
    class_name
}

fn batch_format_class(current: &str, expected: &str) -> String {
    if current.eq_ignore_ascii_case(expected)
        || (expected == "jpeg" && current.eq_ignore_ascii_case("jpg"))
    {
        "on".to_string()
    } else {
        String::new()
    }
}

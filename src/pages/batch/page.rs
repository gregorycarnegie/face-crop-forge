use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats, ImageStatus};
use crate::batch_export::BatchProgress;
use crate::components::log_card::LogCard;
use crate::components::panel::Panel;
use crate::components::progress_bar::ProgressBar;
use crate::components::topbar::Topbar;
use crate::router::Route;
use crate::runtime::{ProcessedImageOutput, batch_file_label, files_from_data_transfer};
use crate::state::AppState;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};

use super::helpers::{
    BatchGalleryFilter, batch_aspect_class, batch_empty_filter_label, batch_filter_matches,
    batch_format_class, filter_tab_class, gallery_cell_class, status_badge,
};
use super::process::{
    BatchProcessCtx, batch_concurrency_limit, clear_batch, download_batch_zip, load_batch_files,
    process_batch,
};

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
    let gallery_filter = RwSignal::new(BatchGalleryFilter::All);

    let cancel_requested = RwSignal::new(false);
    let busy = Signal::derive(move || progress.get().running);
    let cancelled = Signal::derive(move || progress.get().cancelled);
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
                <span class=move || { if busy.get() { "pill run" } else if cancelled.get() { "pill cancel" } else { "pill" } }><span class="d"></span>{move || progress.get().status}</span>
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
                        cancel_requested,
                    )
                >"Start processing"</button>
                <button
                    class="btn btn-danger"
                    style=move || if busy.get() { "" } else { "display:none" }
                    on:click=move |_| cancel_requested.set(true)
                >"Cancel"</button>
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
                on:click=move |_| clear_batch(BatchProcessCtx { progress, stats, batch_state, progress_pct }, batch_queue, files_by_id, preview_urls, outputs)
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
                                <button class=move || batch_aspect_class(&settings.get(), 512, 512) on:click=move |_| settings.update(|s| { s.output_width = 512; s.output_height = 512; })>"1:1"</button>
                                <button class=move || batch_aspect_class(&settings.get(), 640, 800) on:click=move |_| settings.update(|s| { s.output_width = 640; s.output_height = 800; })>"4:5"</button>
                                <button class=move || batch_aspect_class(&settings.get(), 600, 800) on:click=move |_| settings.update(|s| { s.output_width = 600; s.output_height = 800; })>"3:4"</button>
                                <button class=move || batch_aspect_class(&settings.get(), 768, 512) on:click=move |_| settings.update(|s| { s.output_width = 768; s.output_height = 512; })>"3:2"</button>
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
                        <label class="toggle-row">
                            <span><span>"Compress ZIP"</span><span class="desc">"Smaller file, slower export (Deflate)"</span></span>
                            <input type="checkbox" prop:checked=move || settings.get().zip_compress on:change=move |ev| settings.update(|s| s.zip_compress = event_target_checked(&ev)) />
                        </label>
                        <div class="compat-note">
                            <p class="compat-head">"Detection backend"</p>
                            <p>"Uses native "<code>"FaceDetector"</code>" API on Chrome and Edge (Chromium ≥ 123). Falls back to MediaPipe on Firefox and Safari."</p>
                            <p class="compat-head">"Worker crop"</p>
                            <p>"Off-thread crop via "<code>"OffscreenCanvas"</code>" runs on Chrome, Edge, and Firefox. Safari uses the main-thread canvas fallback."</p>
                            <p class="compat-head">"Large exports"</p>
                            <p>"Exports over 500 crops or 200 MB are split into numbered ZIP parts automatically."</p>
                            <p class="compat-head">"Deployment headers"</p>
                            <p><code>"Cross-Origin-Opener-Policy: same-origin"</code>" is recommended on all hosts. "<code>"Cross-Origin-Embedder-Policy: require-corp"</code>" requires every cross-origin asset (including MediaPipe model files) to carry a "<code>"Cross-Origin-Resource-Policy"</code>" header — only enable it if you self-host the models."</p>
                        </div>
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

                <div class="diag-strip">
                    <div class="d">
                        <div class="k">"backend"</div>
                        <div class="v">{move || {
                            let b = stats.get().detected_backend;
                            if b.is_empty() { "—".to_string() } else { b }
                        }}</div>
                    </div>
                    <div class="d">
                        <div class="k">"avg / img"</div>
                        <div class="v">{move || {
                            let ms = stats.get().avg_processing_time_ms();
                            if ms == 0 { "—".to_string() } else { format!("{ms}ms") }
                        }}</div>
                    </div>
                    <div class="d">
                        <div class="k">"last export"</div>
                        <div class="v">{move || stats.get().last_export_time_ms
                            .map_or("—".to_string(), |ms| format!("{ms}ms"))
                        }</div>
                    </div>
                    <div class="d">
                        <div class="k">"concurrency"</div>
                        <div class="v">{move || format!("{} worker(s)", batch_concurrency_limit())}</div>
                    </div>
                    <div class="d">
                        <div class="k">"last image"</div>
                        <div class="v">{move || stats.get().last_image_dimensions
                            .map_or("—".to_string(), |(w, h)| format!("{w}×{h}"))
                        }</div>
                    </div>
                    <div class="d">
                        <div class="k">"detect scale"</div>
                        <div class="v">{move || {
                            let s = stats.get();
                            if s.last_image_dimensions.is_none() {
                                "—".to_string()
                            } else if (s.last_downscale_factor - 1.0).abs() < 0.005 {
                                "1× (full)".to_string()
                            } else {
                                format!("{:.2}×", s.last_downscale_factor)
                            }
                        }}</div>
                    </div>
                </div>
                <div class="diag-copy-row">
                    <button
                        class="btn-copy-debug"
                        title="Copy diagnostics to clipboard"
                        on:click=move |_| {
                            let s = stats.get_untracked();
                            let conc = batch_concurrency_limit();
                            let backend = if s.detected_backend.is_empty() { "—".to_string() } else { s.detected_backend.clone() };
                            let avg_ms = s.avg_processing_time_ms();
                            let export = s.last_export_time_ms.map_or("—".to_string(), |ms| format!("{ms}ms"));
                            let dims = s.last_image_dimensions.map_or("—".to_string(), |(w, h)| format!("{w}×{h}"));
                            let ds = if s.last_image_dimensions.is_none() {
                                "—".to_string()
                            } else if (s.last_downscale_factor - 1.0).abs() < 0.005 {
                                "1× (full res)".to_string()
                            } else {
                                format!("{:.2}×", s.last_downscale_factor)
                            };
                            #[allow(unused_variables)]
                            let summary = format!(
                                "face-crop-forge diagnostics\nbackend:      {backend}\navg / img:    {avg_ms}ms\nlast export:  {export}\nconcurrency:  {conc} worker(s)\nlast image:   {dims}\ndetect scale: {ds}\nprocessed:    {proc}\nfaces:        {faces}\nfailed:       {failed}",
                                proc = s.images_processed,
                                faces = s.total_faces_detected,
                                failed = s.images_processed.saturating_sub(s.successful_processing),
                            );
                            spawn_local(async move {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use wasm_bindgen_futures::JsFuture;
                                    let window = leptos::prelude::window();
                                    let clipboard = window.navigator().clipboard();
                                    let _ = JsFuture::from(clipboard.write_text(&summary)).await;
                                }
                            });
                        }
                    >"copy debug"</button>
                </div>

                <div class="gallery-card">
                    <div class="gallery-head">
                        <div style="display:flex;align-items:center;gap:14px;flex-wrap:wrap">
                            <h3>"Input queue - "<b>{move || batch_state.get().total_count().to_string()}</b>" files"</h3>
                            <div class="filter-tabs">
                                <button class=move || filter_tab_class(gallery_filter.get(), BatchGalleryFilter::All) on:click=move |_| gallery_filter.set(BatchGalleryFilter::All)>"All "<span class="n">{move || batch_state.get().total_count().to_string()}</span></button>
                                <button class=move || filter_tab_class(gallery_filter.get(), BatchGalleryFilter::Done) on:click=move |_| gallery_filter.set(BatchGalleryFilter::Done)>"Done "<span class="n">{move || processed_count.get().to_string()}</span></button>
                                <button class=move || filter_tab_class(gallery_filter.get(), BatchGalleryFilter::Failed) on:click=move |_| gallery_filter.set(BatchGalleryFilter::Failed)>"Failed "<span class="n">{move || failed_count.get().to_string()}</span></button>
                            </div>
                        </div>
                    </div>
                    <div class="gallery">
                        {move || {
                            let state = batch_state.get();
                            let previews = preview_urls.get();
                            let active_filter = gallery_filter.get();
                            let mut ids = state.images.keys().cloned().collect::<Vec<_>>();
                            ids.sort();
                            let cells = ids.into_iter().filter_map(|id| {
                                let image = state.images.get(&id).cloned();
                                let selected = image.as_ref().is_some_and(|img| img.selected);
                                let status = image.as_ref().map(|img| img.status.clone()).unwrap_or(ImageStatus::Loaded);
                                if !batch_filter_matches(active_filter, &status) {
                                    return None;
                                }
                                let (badge_class, badge_label) = status_badge(&status);
                                let class_name = gallery_cell_class(selected, &status);
                                let preview = previews.get(&id).cloned();
                                let label = batch_file_label(&id).to_string();
                                let id_for_click = id.clone();
                                Some(view! {
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
                                }.into_any())
                            }).collect::<Vec<_>>();
                            if cells.is_empty() {
                                vec![view! { <div class="empty-gallery">{batch_empty_filter_label(active_filter)}</div> }.into_any()]
                            } else {
                                cells
                            }
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

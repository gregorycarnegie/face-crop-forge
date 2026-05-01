use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats, ImageStatus};
use crate::batch_export::BatchProgress;
use crate::components::log_card::LogCard;
use crate::components::panel::Panel;
use crate::components::progress_bar::ProgressBar;
use crate::components::topbar::Topbar;
use crate::csv_core::CsvCoreState;
use crate::router::Route;
use crate::runtime::{ProcessedImageOutput, batch_file_label, files_from_data_transfer};
use crate::state::AppState;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, HtmlInputElement};

use super::helpers::{
    CsvMatchFilter, csv_aspect_class, csv_empty_filter_label, csv_filter_matches,
    csv_filter_tab_class, csv_format_class, csv_status_badge, csv_template_display,
};
use super::process::{
    download_csv_zip, load_csv_file, load_csv_images, process_csv_batch, reset_csv_workflow,
};

#[component]
pub fn Csv(route: Route, set_route: WriteSignal<Route>) -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let csv_state = RwSignal::new(CsvCoreState::default());
    let batch_state = RwSignal::new(BatchCoreState::default());
    let queue = RwSignal::new(BatchQueueState::default());
    let files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let source_name_by_id = RwSignal::new(HashMap::<String, String>::new());
    let outputs = RwSignal::new(HashMap::<String, ProcessedImageOutput>::new());
    let face_count_by_id = RwSignal::new(HashMap::<String, usize>::new());
    let progress = RwSignal::new(BatchProgress::default());
    let progress_pct = RwSignal::new(0_u32);
    let stats = RwSignal::new(BatchRuntimeStats::default());
    let file_path_column = RwSignal::new(String::new());
    let file_name_column = RwSignal::new(String::new());
    let mapping_confirmed = RwSignal::new(false);
    let match_filter = RwSignal::new(CsvMatchFilter::All);
    let continue_on_error = RwSignal::new(true);

    let busy = Signal::derive(move || progress.get().running);
    let rows = Signal::derive(move || csv_state.get().rows.len());
    let mapped_rows = Signal::derive(move || csv_state.get().filename_to_output.len());
    let matched_images = Signal::derive(move || batch_state.get().total_count());
    let processed = Signal::derive(move || {
        batch_state
            .get()
            .images
            .values()
            .filter(|image| image.processed)
            .count()
    });
    let failed_count = Signal::derive(move || progress.get().failed);
    let missing_rows = Signal::derive(move || rows.get().saturating_sub(mapped_rows.get()));
    let mapping_status = Signal::derive(move || {
        if mapping_confirmed.get() {
            format!("Mapping ready - {} filename(s)", mapped_rows.get())
        } else if rows.get() > 0 {
            "Choose columns and confirm mapping".to_string()
        } else {
            "Upload CSV".to_string()
        }
    });

    let load_images = move |files: Vec<web_sys::File>| {
        load_csv_images(
            files,
            csv_state,
            batch_state,
            queue,
            files_by_id,
            preview_urls,
            source_name_by_id,
            outputs,
            face_count_by_id,
            progress,
            progress_pct,
            stats,
        );
    };

    view! {
        <Topbar route set_route />

        <div class="page-head">
            <div>
                <div class="crumb"><span>"Workflow 03"</span><span class="dot"></span><span>"CSV-driven"</span><span class="dot"></span><span class="now" style="color:var(--lime)">{mapping_status}</span></div>
                <h1>"Driven by your "<span class="grad" style="background:linear-gradient(110deg,var(--lime),var(--cyan));-webkit-background-clip:text;background-clip:text;color:transparent">"CSV"</span>"."</h1>
                <p class="lede">"Map source filenames to output IDs, process matching images, and export a ZIP named from your data."</p>
            </div>
            <div class="meta-pills">
                <span class=move || { if mapping_confirmed.get() { "pill ok" } else { "pill" } }><span class="d"></span>{mapping_status}</span>
                <span class="pill"><span class="d"></span>{move || format!("{} rows", rows.get())}</span>
                <span class="pill"><span class="d"></span>{move || format!("{} matched", matched_images.get())}</span>
            </div>
        </div>

        <div class="stepper">
            <div class=move || { if rows.get() > 0 { "step done" } else { "step now" } }>
                <div class="num-big">"01"</div>
                <div class="n">"Step 01"</div>
                <div class="t">"Upload CSV"</div>
                <div class="s"><span class="dot"></span>{move || format!("{} rows", rows.get())}</div>
            </div>
            <div class=move || { if mapping_confirmed.get() { "step done" } else if rows.get() > 0 { "step now" } else { "step locked" } }>
                <div class="num-big">"02"</div>
                <div class="n">"Step 02"</div>
                <div class="t">"Map columns"</div>
                <div class="s"><span class="dot"></span>{mapping_status}</div>
            </div>
            <div class=move || { if matched_images.get() > 0 { "step done" } else if mapping_confirmed.get() { "step now" } else { "step locked" } }>
                <div class="num-big">"03"</div>
                <div class="n">"Step 03"</div>
                <div class="t">"Upload images"</div>
                <div class="s"><span class="dot"></span>{move || format!("{} matched", matched_images.get())}</div>
            </div>
            <div class=move || { if busy.get() { "step now" } else if processed.get() > 0 { "step done" } else { "step locked" } }>
                <div class="num-big">"04"</div>
                <div class="n">"Step 04"</div>
                <div class="t">"Process & export"</div>
                <div class="s"><span class="dot"></span>{move || format!("{} processed", processed.get())}</div>
            </div>
        </div>

        <div class="toolbar">
            <div class="group">
                <button
                    class="btn btn-lime"
                    disabled=move || matched_images.get() == 0 || busy.get()
                    on:click=move |_| process_csv_batch(
                        settings,
                        csv_state,
                        batch_state,
                        queue,
                        files_by_id,
                        preview_urls,
                        source_name_by_id,
                        outputs,
                        face_count_by_id,
                        progress,
                        progress_pct,
                        stats,
                        continue_on_error.get(),
                    )
                >{move || format!("Process all ({})", matched_images.get())}</button>
            </div>
            <span class="sep"></span>
            <button
                class="btn btn-peach"
                disabled=move || outputs.get().is_empty() || busy.get()
                on:click=move |_| download_csv_zip(settings, csv_state, batch_state, source_name_by_id, outputs, progress, stats)
            >"Download ZIP"</button>
            <span class="sep"></span>
            <button
                class="btn btn-danger"
                style="margin-left:auto"
                disabled=move || busy.get()
                on:click=move |_| reset_csv_workflow(
                    csv_state,
                    batch_state,
                    queue,
                    files_by_id,
                    preview_urls,
                    source_name_by_id,
                    outputs,
                    face_count_by_id,
                    progress,
                    progress_pct,
                    stats,
                    file_path_column,
                    file_name_column,
                    mapping_confirmed,
                )
            >"Reset workflow"</button>
        </div>

        <div class="layout">
            <aside class="sidebar">
                <div class="sb-scroll">
                    <Panel title="Match summary" num="01" initially_open=true accent="lime">
                        <div class="mini-grid">
                            <div><div class="k">"rows"</div><div class="v">{move || rows.get().to_string()}</div></div>
                            <div><div class="k">"mapped"</div><div class="v lime">{move || mapped_rows.get().to_string()}</div></div>
                            <div><div class="k">"unmapped"</div><div class="v rose">{move || missing_rows.get().to_string()}</div></div>
                            <div><div class="k">"processed"</div><div class="v cyan">{move || processed.get().to_string()}</div></div>
                        </div>
                    </Panel>

                    <Panel title="Crop framing" num="02" initially_open=true accent="lime">
                        <div class="field">
                            <label>"Aspect ratio"</label>
                            <div class="seg cols-4">
                                <button class=move || csv_aspect_class(&settings.get(), 512, 512) on:click=move |_| settings.update(|s| { s.output_width = 512; s.output_height = 512; })>"1:1"</button>
                                <button class=move || csv_aspect_class(&settings.get(), 640, 800) on:click=move |_| settings.update(|s| { s.output_width = 640; s.output_height = 800; })>"4:5"</button>
                                <button class=move || csv_aspect_class(&settings.get(), 600, 800) on:click=move |_| settings.update(|s| { s.output_width = 600; s.output_height = 800; })>"3:4"</button>
                                <button class=move || csv_aspect_class(&settings.get(), 768, 512) on:click=move |_| settings.update(|s| { s.output_width = 768; s.output_height = 512; })>"3:2"</button>
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

                    <Panel title="Output naming" num="03" initially_open=true accent="lime">
                        <div class="field">
                            <label>"Format"</label>
                            <div class="seg cols-3">
                                <button class=move || csv_format_class(&settings.get().output_format, "jpeg") on:click=move |_| settings.update(|s| s.output_format = "jpeg".to_string())>"JPG"</button>
                                <button class=move || csv_format_class(&settings.get().output_format, "png") on:click=move |_| settings.update(|s| s.output_format = "png".to_string())>"PNG"</button>
                                <button class=move || csv_format_class(&settings.get().output_format, "webp") on:click=move |_| settings.update(|s| s.output_format = "webp".to_string())>"WEBP"</button>
                            </div>
                        </div>
                        <div class="field">
                            <label>"Filename pattern"</label>
                            <input class="input" prop:value=move || csv_template_display(&settings.get().naming_template) on:change=move |ev| settings.update(|s| s.naming_template = event_target_value(&ev)) />
                            <div class="hint">"Use {csv_name}, {original}, {index}, {width}, {height}, or {timestamp}."</div>
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

                    <Panel title="Performance" num="04" initially_open=false accent="lime">
                        <label class="toggle-row">
                            <span><span>"Continue on error"</span><span class="desc">"Skip failures and keep processing"</span></span>
                            <input type="checkbox" prop:checked=move || continue_on_error.get() on:change=move |ev| continue_on_error.set(event_target_checked(&ev)) />
                        </label>
                        <label class="toggle-row">
                            <span><span>"Compress ZIP"</span><span class="desc">"Smaller file, slower export (Deflate)"</span></span>
                            <input type="checkbox" prop:checked=move || settings.get().zip_compress on:change=move |ev| settings.update(|s| s.zip_compress = event_target_checked(&ev)) />
                        </label>
                    </Panel>
                </div>
            </aside>

            <div class="workspace">
                <div class="upload-row">
                    <div
                        class="upload-card csv-up"
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
                            load_csv_file(file, csv_state, file_path_column, file_name_column, mapping_confirmed, progress);
                        }
                    >
                        <label class="upload-label" for="csvInput">
                            <div class="icon">
                                <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="3" y="3" width="16" height="16" rx="2"/><path d="M3 8h16M8 3v16M14 3v16"/></svg>
                            </div>
                            <h4>"CSV manifest"</h4>
                            <p>"Headers and rows that map source filenames to export names."</p>
                            <div class="actions"><span class="chip">"Choose CSV"</span></div>
                            <div class="stat-row">
                                <div class="s"><div class="v">{move || rows.get().to_string()}</div><div class="k">"rows"</div></div>
                                <div class="s"><div class="v">{move || csv_state.get().headers.len().to_string()}</div><div class="k">"columns"</div></div>
                                <div class="s"><div class="v">{move || if mapping_confirmed.get() { "2" } else { "0" }}</div><div class="k">"bound"</div></div>
                            </div>
                        </label>
                        <input
                            id="csvInput"
                            class="hidden-input"
                            type="file"
                            accept=".csv,text/csv"
                            disabled=move || busy.get()
                            on:change=move |ev| {
                                let input: HtmlInputElement = event_target(&ev);
                                let Some(files) = input.files() else {
                                    return;
                                };
                                let Some(file) = files.get(0) else {
                                    return;
                                };
                                load_csv_file(file, csv_state, file_path_column, file_name_column, mapping_confirmed, progress);
                            }
                        />
                    </div>

                    <div
                        class=move || { if mapping_confirmed.get() { "upload-card img-up" } else { "upload-card img-up locked" } }
                        on:dragover=move |ev: DragEvent| {
                            if mapping_confirmed.get() && !busy.get() {
                                ev.prevent_default();
                                ev.stop_propagation();
                            }
                        }
                        on:drop=move |ev: DragEvent| {
                            if !mapping_confirmed.get() || busy.get() {
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
                                    Ok(files) => load_images(files),
                                    Err(error) => progress_for_drop.update(|p| p.status = format!("Image drop failed: {error}")),
                                }
                            });
                        }
                    >
                        <label class="upload-label" for="csvImageInput">
                            <div class="icon">
                                <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="3" y="3" width="16" height="16" rx="2"/><circle cx="9" cy="9" r="2"/><path d="M3 16l5-5 4 4 3-3 4 4"/></svg>
                            </div>
                            <h4>"Image folder"</h4>
                            <p>"Only files matching the mapped CSV source column are queued."</p>
                            <div class="actions"><span class="chip">"Browse files"</span><span class="chip">"Drop folder"</span></div>
                            <div class="stat-row">
                                <div class="s"><div class="v" style="color:var(--cyan)">{move || matched_images.get().to_string()}</div><div class="k">"matched"</div></div>
                                <div class="s"><div class="v" style="color:var(--rose)">{move || mapped_rows.get().saturating_sub(matched_images.get()).to_string()}</div><div class="k">"missing"</div></div>
                                <div class="s"><div class="v">{move || outputs.get().len().to_string()}</div><div class="k">"output"</div></div>
                            </div>
                        </label>
                        <input
                            id="csvImageInput"
                            class="hidden-input"
                            type="file"
                            accept="image/*"
                            multiple
                            disabled=move || !mapping_confirmed.get() || busy.get()
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
                                load_images(files);
                            }
                        />
                    </div>
                </div>

                <div class="map-card">
                    <div class="map-head">
                        <h3><b>"Column"</b>" mapping"</h3>
                        <span class="status"><span class="d"></span>{mapping_status}</span>
                    </div>
                    <div class="map-body">
                        <div class="map-grid">
                            <div class="field">
                                <label>"File path column"</label>
                                <select class="select-csv bound" prop:value=move || file_path_column.get() on:change=move |ev| {
                                    file_path_column.set(event_target_value(&ev));
                                    mapping_confirmed.set(false);
                                }>
                                    {move || csv_state.get().headers.into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect::<Vec<_>>()}
                                </select>
                            </div>
                            <div class="field">
                                <label>"Output name column"</label>
                                <select class="select-csv bound" prop:value=move || file_name_column.get() on:change=move |ev| {
                                    file_name_column.set(event_target_value(&ev));
                                    mapping_confirmed.set(false);
                                }>
                                    {move || csv_state.get().headers.into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect::<Vec<_>>()}
                                </select>
                            </div>
                            <button
                                class="btn btn-lime"
                                disabled=move || rows.get() == 0 || busy.get()
                                on:click=move |_| {
                                    let path = file_path_column.get();
                                    let name = file_name_column.get();
                                    let mut applied = false;
                                    csv_state.update(|state| applied = state.apply_mapping(&path, &name));
                                    mapping_confirmed.set(applied);
                                    progress.update(|p| {
                                        p.status = if applied {
                                            format!("Mapping confirmed: {path} -> source, {name} -> output")
                                        } else {
                                            "Mapping failed. Choose two different CSV columns.".to_string()
                                        };
                                    });
                                }
                            >"Confirm mapping"</button>
                        </div>

                        <div class="csv-preview">
                            <div class="ph"><span>"CSV preview - first rows"</span><span>{move || format!("{} total", rows.get())}</span></div>
                            <table class="csv-table">
                                <thead>
                                    <tr>
                                        {move || csv_state.get().headers.into_iter().take(5).map(|header| {
                                            let class_name = if header == file_path_column.get() {
                                                "bound-path"
                                            } else if header == file_name_column.get() {
                                                "bound-name"
                                            } else {
                                                ""
                                            };
                                            view! { <th class=class_name>{header}</th> }
                                        }).collect::<Vec<_>>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || {
                                        let state = csv_state.get();
                                        let headers = state.headers.into_iter().take(5).collect::<Vec<_>>();
                                        state.rows.into_iter().take(5).map(|row| {
                                            let path_col = file_path_column.get();
                                            let name_col = file_name_column.get();
                                            view! {
                                                <tr>
                                                    {headers.iter().map(|header| {
                                                        let class_name = if header == &path_col {
                                                            "bound-path"
                                                        } else if header == &name_col {
                                                            "bound-name"
                                                        } else {
                                                            ""
                                                        };
                                                        let value = row.0.get(header).cloned().unwrap_or_default();
                                                        view! { <td class=class_name>{value}</td> }
                                                    }).collect::<Vec<_>>()}
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <div class="runbar runbar-csv">
                    <div class="top">
                        <div class="nm"><b>"CSV batch"</b><span>{move || format!(" - processed {} of {}", progress.get().processed, progress.get().total)}</span></div>
                        <div class="pct"><b>{move || format!("{}%", progress_pct.get())}</b></div>
                    </div>
                    <ProgressBar progress=progress_pct />
                    <div class="meta"><span>{move || progress.get().status}</span></div>
                </div>

                <div class="strip">
                    <div class="s lime"><div class="k">"CSV rows"</div><div class="v"><em>{move || rows.get().to_string()}</em></div><div class="sub">{move || format!("{} columns", csv_state.get().headers.len())}</div></div>
                    <div class="s cyan"><div class="k">"Matched"</div><div class="v"><em>{move || matched_images.get().to_string()}</em></div><div class="sub">"queued images"</div></div>
                    <div class="s rose"><div class="k">"Missing"</div><div class="v"><em>{move || mapped_rows.get().saturating_sub(matched_images.get()).to_string()}</em></div><div class="sub">"no upload match"</div></div>
                    <div class="s peach"><div class="k">"Processed"</div><div class="v"><em>{move || processed.get().to_string()}</em></div><div class="sub">{move || format!("{}% complete", progress_pct.get())}</div></div>
                    <div class="s lime"><div class="k">"Faces"</div><div class="v"><em>{move || stats.get().total_faces_detected.to_string()}</em></div><div class="sub">"detected total"</div></div>
                </div>

                <div class="match-card">
                    <div class="match-head">
                        <h3>"Row "<b>"matches"</b>" - "{move || batch_state.get().total_count().to_string()}" entries"</h3>
                        <div class="filter-tabs cyan">
                            <button class=move || csv_filter_tab_class(match_filter.get(), CsvMatchFilter::All) on:click=move |_| match_filter.set(CsvMatchFilter::All)>"All "<span class="n">{move || batch_state.get().total_count().to_string()}</span></button>
                            <button class=move || csv_filter_tab_class(match_filter.get(), CsvMatchFilter::Done) on:click=move |_| match_filter.set(CsvMatchFilter::Done)>"Done "<span class="n">{move || processed.get().to_string()}</span></button>
                            <button class=move || csv_filter_tab_class(match_filter.get(), CsvMatchFilter::Failed) on:click=move |_| match_filter.set(CsvMatchFilter::Failed)>"Failed "<span class="n">{move || failed_count.get().to_string()}</span></button>
                        </div>
                    </div>
                    <table class="match-table">
                        <thead>
                            <tr><th></th><th>"source file"</th><th>"output filename"</th><th>"faces"</th><th>"state"</th></tr>
                        </thead>
                        <tbody>
                            {move || {
                                let state = batch_state.get();
                                let previews = preview_urls.get();
                                let source_names = source_name_by_id.get();
                                let csv = csv_state.get();
                                let face_counts = face_count_by_id.get();
                                let active_filter = match_filter.get();
                                let mut ids = state.images.keys().cloned().collect::<Vec<_>>();
                                ids.sort();
                                let rows = ids.into_iter().filter_map(|id| {
                                    let source_name = source_names.get(&id).cloned().unwrap_or_else(|| batch_file_label(&id).to_string());
                                    let output_name = csv.output_name_for_file(&source_name).unwrap_or_else(|| source_name.clone());
                                    let preview = previews.get(&id).cloned();
                                    let image = state.images.get(&id).cloned();
                                    let status = image.as_ref().map(|image| image.status.clone()).unwrap_or(ImageStatus::Loaded);
                                    if !csv_filter_matches(active_filter, &status) {
                                        return None;
                                    }
                                    let (badge_class, badge_label) = csv_status_badge(&status);
                                    let faces = face_counts.get(&id).copied().unwrap_or(0);
                                    Some(view! {
                                        <tr>
                                            <td><span class="thumb">{preview.map(|src| view! { <img src=src alt="CSV match preview" /> })}</span></td>
                                            <td class="filename">{source_name}</td>
                                            <td class="out">{output_name}</td>
                                            <td class="filename">{faces}</td>
                                            <td><span class=badge_class>{badge_label}</span></td>
                                        </tr>
                                    }.into_any())
                                }).collect::<Vec<_>>();
                                if rows.is_empty() {
                                    vec![view! { <tr><td colspan="5" class="filename">{csv_empty_filter_label(active_filter)}</td></tr> }.into_any()]
                                } else {
                                    rows
                                }
                            }}
                        </tbody>
                    </table>
                </div>

                <div class="out-card">
                    <div class="out-head">
                        <h3>"Cropped "<b>"Outputs"</b></h3>
                        <div class="meta">{move || format!("{} generated", outputs.get().len())}</div>
                    </div>
                    <div class="out-grid">
                        {move || {
                            let outputs_map = outputs.get();
                            let names = source_name_by_id.get();
                            let csv = csv_state.get();
                            if outputs_map.is_empty() {
                                return vec![view! { <div class="empty-output">"CSV crops will appear here."</div> }.into_any()];
                            }
                            let mut ids = outputs_map.keys().cloned().collect::<Vec<_>>();
                            ids.sort();
                            ids.into_iter().filter_map(|id| {
                                outputs_map.get(&id).map(|output| {
                                    let source = names.get(&id).cloned().unwrap_or_else(|| batch_file_label(&id).to_string());
                                    let out_name = csv.output_name_for_file(&source).unwrap_or(source);
                                    view! {
                                        <div class="out">
                                            <div class="img"><img src=output.preview_url.clone() alt="CSV cropped output" /></div>
                                            <div class="nm">{out_name}</div>
                                        </div>
                                    }.into_any()
                                })
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>

                <LogCard title="CSV runtime" accent="lime" meta_left="csv" meta_right="">
                    {move || stats.get().logs.into_iter().rev().take(30).map(|entry| {
                        let lower = entry.to_lowercase();
                        let variant = if lower.contains("failed") || lower.contains("error") { "err" } else if lower.contains("processed") || lower.contains("exported") { "ok" } else { "" };
                        view! { <div class=format!("line {variant}")><span class="t">"csv"</span><span class="m">{entry}</span></div> }
                    }).collect::<Vec<_>>()}
                </LogCard>

                <div style="height:120px"></div>
            </div>
        </div>
    }
}

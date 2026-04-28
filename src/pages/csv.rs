use crate::base_runtime::{ImageMeta, ImageValidationConfig, validate_image_meta};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats, ImageStatus};
use crate::batch_export::BatchProgress;
use crate::components::log_card::LogCard;
use crate::components::panel::Panel;
use crate::components::progress_bar::ProgressBar;
use crate::components::topbar::Topbar;
use crate::csv_core::{CsvCoreState, CsvExportNameContext};
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
use crate::state::{AppState, ProcessingSettings};
use crate::worker_bridge::detect_faces_with_worker;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{DragEvent, HtmlInputElement};

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

                    <Panel title="Output naming" num="02" initially_open=true accent="lime">
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

fn load_csv_file(
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
fn load_csv_images(
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
fn process_csv_batch(
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
        let ctx = BatchProcessCtx {
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
struct BatchProcessCtx {
    progress: RwSignal<BatchProgress>,
    stats: RwSignal<BatchRuntimeStats>,
    batch_state: RwSignal<BatchCoreState>,
    progress_pct: RwSignal<u32>,
}

fn record_csv_failure(
    ctx: BatchProcessCtx,
    id: &str,
    elapsed_ms: u64,
    status: impl Into<String>,
    log: impl Into<String>,
) {
    let BatchProcessCtx {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CsvMatchFilter {
    All,
    Done,
    Failed,
}

fn csv_filter_tab_class(current: CsvMatchFilter, expected: CsvMatchFilter) -> &'static str {
    if current == expected { "on" } else { "" }
}

fn csv_filter_matches(filter: CsvMatchFilter, status: &ImageStatus) -> bool {
    match filter {
        CsvMatchFilter::All => true,
        CsvMatchFilter::Done => matches!(status, ImageStatus::Processed),
        CsvMatchFilter::Failed => matches!(status, ImageStatus::Error),
    }
}

fn csv_empty_filter_label(filter: CsvMatchFilter) -> &'static str {
    match filter {
        CsvMatchFilter::All => "No CSV matches yet.",
        CsvMatchFilter::Done => "No processed CSV matches yet.",
        CsvMatchFilter::Failed => "No failed CSV matches.",
    }
}

fn download_csv_zip(
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
fn reset_csv_workflow(
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

fn guess_column(headers: &[String], candidates: &[&str]) -> String {
    candidates
        .iter()
        .find_map(|candidate| {
            headers
                .iter()
                .find(|header| header.eq_ignore_ascii_case(candidate))
                .cloned()
        })
        .or_else(|| headers.first().cloned())
        .unwrap_or_default()
}

fn csv_status_badge(status: &ImageStatus) -> (&'static str, &'static str) {
    match status {
        ImageStatus::Loaded => ("badge queued", "queued"),
        ImageStatus::Processing => ("badge run", "running"),
        ImageStatus::Processed => ("badge ok", "done"),
        ImageStatus::Error => ("badge fail", "failed"),
    }
}

fn csv_format_class(current: &str, expected: &str) -> String {
    if current.eq_ignore_ascii_case(expected)
        || (expected == "jpeg" && current.eq_ignore_ascii_case("jpg"))
    {
        "on lime".to_string()
    } else {
        String::new()
    }
}

fn csv_template_display(template: &str) -> String {
    if template.trim().is_empty() || template == "face_{original}_{index}" {
        "{csv_name}".to_string()
    } else {
        template.to_string()
    }
}

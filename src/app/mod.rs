mod components;
mod helpers;
mod pages;

use crate::base_runtime::{
    DetectionRetryPolicy, Dimensions, ImageMeta, ImageValidationConfig, MemoryIndicatorLevel,
    build_memory_indicator, parse_max_retries, validate_image_meta,
};
use crate::base_ui::{ThemeMode, toggle_theme};
use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats};
use crate::batch_export::BatchProgress;
use crate::csv_core::{CsvCoreState, CsvExportNameContext};
use crate::export_runtime::{
    build_zip_bytes, current_timestamp_ms, current_utc_timestamp_token, download_bytes,
    file_to_bytes, normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::mediapipe::{
    MediaPipeAssetPaths, build_load_plan, detect_browser_capabilities, evaluate_pipeline_health,
    revalidate_browser_fallbacks,
};
use crate::panels::{
    CropSettingsPanel, OutputSettingsBatchPanel, OutputSettingsCsvPanel, PreprocessingSettingsPanel,
};
use crate::single_core::{
    SingleCoreState, SingleRuntimeState, build_export_plan, compute_display_size,
};
use crate::state::{AppState, ProcessingSettings};
use crate::worker_bridge::{
    DetectedFace, FaceWorkerBridgeState, clear_last_detection_backend, detect_faces_with_worker,
    last_detection_backend_label, start_face_worker, stop_face_worker,
};
use leptos::prelude::*;
use std::collections::HashMap;
use std::time::Instant;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DragEvent, HtmlInputElement};

use helpers::*;
use pages::{BatchPage, CsvPage, LandingPage, PanelsGalleryPage, SinglePage};

#[component]
pub fn App() -> impl IntoView {
    provide_context(AppState::new());
    let theme_mode = RwSignal::new(load_theme_mode());
    provide_context(theme_mode);
    Effect::new(move |_| {
        let mode = theme_mode.get();
        apply_theme_mode(mode);
        persist_theme_mode(mode);
    });

    let path = current_path();

    let page = match route_for_path(path.as_str()) {
        RouteTarget::Single => view! { <SinglePage /> }.into_any(),
        RouteTarget::Batch => view! { <BatchPage /> }.into_any(),
        RouteTarget::Csv => view! { <CsvPage /> }.into_any(),
        RouteTarget::Panels => view! { <PanelsGalleryPage /> }.into_any(),
        RouteTarget::Landing => view! { <LandingPage /> }.into_any(),
    };

    view! {
        <div
            on:dragover=move |ev: DragEvent| {
                ev.prevent_default();
            }
            on:drop=move |ev: DragEvent| {
                ev.prevent_default();
            }
        >
            {page}
        </div>
    }
    .into_any()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteTarget {
    Landing,
    Single,
    Batch,
    Csv,
    Panels,
}

fn route_for_path(path: &str) -> RouteTarget {
    match path {
        "/single" => RouteTarget::Single,
        "/single-processing.html" => RouteTarget::Single,
        "/batch" => RouteTarget::Batch,
        "/batch-processing.html" => RouteTarget::Batch,
        "/csv" => RouteTarget::Csv,
        "/csv-processing.html" => RouteTarget::Csv,
        "/_panels" => RouteTarget::Panels,
        _ => RouteTarget::Landing,
    }
}

fn current_path() -> String {
    window()
        .location()
        .pathname()
        .unwrap_or_else(|_| "/".to_string())
}

#[cfg(target_arch = "wasm32")]
const SAVED_SETTINGS_KEY: &str = "fcf.saved_settings.v1";

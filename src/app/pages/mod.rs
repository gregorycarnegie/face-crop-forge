use super::components::{
    AppShell, BatchImageGalleryPanel, BatchUploadCard, CsvImageUploadCard, CsvUploadCard,
    SingleUploadCard, batch_file_label,
};
use super::helpers::{
    apply_detection_quality_filters, capture_webcam_frame_to_file, clear_canvas,
    clear_video_source, click_element_by_id, crop_face_bytes_from_source, decode_image_dimensions,
    draw_source_image_to_canvas, elapsed_ms_since, export_saved_settings_json,
    import_saved_settings_json, list_saved_setting_names, list_video_input_devices,
    load_named_processing_settings, mime_type_for_output_format, now_ms, object_url_for_bytes,
    object_url_for_file, overlay_percent_crop_rect, render_naming_template, revoke_object_url,
    save_named_processing_settings, start_webcam_stream, stop_media_stream,
};
use super::{
    AppState, BatchCoreState, BatchProgress, BatchQueueState, BatchRuntimeStats, ClassAttribute,
    CollectView, CropSettingsPanel, CsvCoreState, CsvExportNameContext, CustomAttribute,
    DetectedFace, DetectionRetryPolicy, Effect, ElementChild, FaceWorkerBridgeState, Get,
    GlobalAttributes, HashMap, HtmlInputElement, ImageMeta, ImageValidationConfig, IntoAny,
    IntoView, JsFuture, MediaPipeAssetPaths, MemoryIndicatorLevel, OnAttribute,
    OutputSettingsBatchPanel, OutputSettingsCsvPanel, PreprocessingSettingsPanel, PropAttribute,
    RwSignal, Set, Signal, SingleCoreState, SingleRuntimeState, StyleAttribute, Update,
    build_export_plan, build_load_plan, build_memory_indicator, build_zip_bytes,
    clear_last_detection_backend, component, compute_display_size, current_timestamp_ms,
    current_utc_timestamp_token, detect_browser_capabilities, detect_faces_with_worker,
    download_bytes, evaluate_pipeline_health, event_target, event_target_checked,
    event_target_value, last_detection_backend_label, normalize_export_filename_for_mime,
    parse_max_retries, revalidate_browser_fallbacks, start_face_worker, stop_face_worker,
    use_context, validate_export_filename_for_mime, validate_image_meta, view,
};

macro_rules! record_failed_image {
    (
        $progress:expr,
        $stats:expr,
        $state:expr,
        $id:expr,
        $elapsed_ms:expr,
        $status:expr,
        $log:expr
    ) => {{
        $progress.update(|p| {
            p.record_result(false);
            p.status = $status;
        });
        $stats.update(|stats| {
            stats.record_image($elapsed_ms, 0, false);
            stats.push_log($log);
        });
        $state.update(|s| s.mark_error($id));
    }};
}

#[derive(Clone)]
struct ProcessedImageOutput {
    bytes: Vec<u8>,
    mime_type: String,
    preview_url: String,
}

mod batch;
mod csv;
mod misc;
mod single;

pub(super) use batch::BatchPage;
pub(super) use csv::CsvPage;
pub(super) use misc::{LandingPage, PanelsGalleryPage};
pub(super) use single::SinglePage;

use super::components::{ThemeToggleButton, BatchUploadCard, batch_file_label, BatchImageGalleryPanel, CsvUploadCard, CsvImageUploadCard, AppShell, SingleUploadCard};
use super::helpers::{list_saved_setting_names, navigate_to, decode_image_dimensions, now_ms, apply_detection_quality_filters, elapsed_ms_since, revoke_object_url, render_naming_template, load_named_processing_settings, save_named_processing_settings, export_saved_settings_json, click_element_by_id, import_saved_settings_json, draw_source_image_to_canvas, clear_canvas, stop_media_stream, clear_video_source, list_video_input_devices, start_webcam_stream, overlay_percent_crop_rect, crop_face_bytes_from_source, capture_webcam_frame_to_file, object_url_for_file};
use super::{component, view, IntoView, use_context, AppState, RwSignal, BatchCoreState, BatchQueueState, HashMap, BatchProgress, BatchRuntimeStats, Signal, Get, build_memory_indicator, MemoryIndicatorLevel, ClassAttribute, ElementChild, OnAttribute, GlobalAttributes, Update, DetectionRetryPolicy, parse_max_retries, ImageValidationConfig, ImageMeta, validate_image_meta, DetectedFace, detect_faces_with_worker, Set, current_timestamp_ms, current_utc_timestamp_token, file_to_bytes, normalize_export_filename_for_mime, validate_export_filename_for_mime, build_zip_bytes, download_bytes, PropAttribute, CollectView, event_target_value, HtmlInputElement, event_target, JsFuture, event_target_checked, IntoAny, CropSettingsPanel, PreprocessingSettingsPanel, OutputSettingsBatchPanel, CsvCoreState, Effect, CsvExportNameContext, OutputSettingsCsvPanel, StyleAttribute, CustomAttribute, SingleCoreState, FaceWorkerBridgeState, SingleRuntimeState, compute_display_size, last_detection_backend_label, detect_browser_capabilities, build_load_plan, MediaPipeAssetPaths, evaluate_pipeline_health, revalidate_browser_fallbacks, start_face_worker, clear_last_detection_backend, stop_face_worker, build_export_plan};

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

mod batch;
mod csv;
mod misc;
mod single;

pub(super) use batch::BatchPage;
pub(super) use csv::CsvPage;
pub(super) use misc::{LandingPage, PanelsGalleryPage};
pub(super) use single::SinglePage;

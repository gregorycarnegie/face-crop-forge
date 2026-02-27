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

#[cfg(target_arch = "wasm32")]
fn load_saved_settings_map() -> HashMap<String, ProcessingSettings> {
    let Some(storage) = window().local_storage().ok().flatten() else {
        return HashMap::new();
    };
    let Some(raw) = storage.get_item(SAVED_SETTINGS_KEY).ok().flatten() else {
        return HashMap::new();
    };
    serde_json::from_str::<HashMap<String, ProcessingSettings>>(&raw).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_saved_settings_map() -> HashMap<String, ProcessingSettings> {
    HashMap::new()
}

#[cfg(target_arch = "wasm32")]
fn persist_saved_settings_map(map: &HashMap<String, ProcessingSettings>) -> Result<(), String> {
    let serialized =
        serde_json::to_string_pretty(map).map_err(|err| format!("Serialize failed: {err}"))?;
    let storage = window()
        .local_storage()
        .map_err(|err| format!("{err:?}"))?
        .ok_or_else(|| "LocalStorage unavailable".to_string())?;
    storage
        .set_item(SAVED_SETTINGS_KEY, &serialized)
        .map_err(|err| format!("{err:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_saved_settings_map(_map: &HashMap<String, ProcessingSettings>) -> Result<(), String> {
    Err("Settings persistence is only available on wasm32".to_string())
}

fn list_saved_setting_names() -> Vec<String> {
    let mut names = load_saved_settings_map()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn save_named_processing_settings(name: &str, settings: &ProcessingSettings) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Settings name cannot be empty".to_string());
    }
    let mut map = load_saved_settings_map();
    map.insert(trimmed.to_string(), settings.clone());
    persist_saved_settings_map(&map)
}

fn load_named_processing_settings(name: &str) -> Option<ProcessingSettings> {
    let mut map = load_saved_settings_map();
    map.remove(name)
}

fn export_saved_settings_json() -> Result<String, String> {
    serde_json::to_string_pretty(&load_saved_settings_map())
        .map_err(|err| format!("Serialize failed: {err}"))
}

fn import_saved_settings_json(json: &str) -> Result<usize, String> {
    let parsed = serde_json::from_str::<HashMap<String, ProcessingSettings>>(json)
        .map_err(|err| format!("Invalid settings JSON: {err}"))?;
    let count = parsed.len();
    persist_saved_settings_map(&parsed)?;
    Ok(count)
}

#[cfg(target_arch = "wasm32")]
fn click_element_by_id(id: &str) {
    use web_sys::wasm_bindgen::JsCast;
    if let Some(document) = window().document() {
        if let Some(element) = document.get_element_by_id(id) {
            if let Ok(html) = element.dyn_into::<web_sys::HtmlElement>() {
                html.click();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn click_element_by_id(_id: &str) {}

#[cfg(target_arch = "wasm32")]
fn object_url_for_file(file: &web_sys::File) -> Option<String> {
    web_sys::Url::create_object_url_with_blob(file).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn object_url_for_file(_file: &web_sys::File) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn revoke_object_url(url: &str) {
    let _ = web_sys::Url::revoke_object_url(url);
}

#[cfg(not(target_arch = "wasm32"))]
fn revoke_object_url(_url: &str) {}

#[cfg(target_arch = "wasm32")]
async fn draw_source_image_to_canvas(
    canvas_id: &str,
    source_url: &str,
) -> Result<(u32, u32), String> {
    use web_sys::wasm_bindgen::JsCast;

    let document = window()
        .document()
        .ok_or_else(|| "Document is unavailable".to_string())?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| format!("Canvas #{canvas_id} not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| format!("Element #{canvas_id} is not a canvas"))?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(source_url);
    JsFuture::from(image.decode())
        .await
        .map_err(|err| format!("Image decode failed: {err:?}"))?;

    let width = image.natural_width();
    let height = image.natural_height();
    if width == 0 || height == 0 {
        return Err("Decoded image dimensions are empty".to_string());
    }

    canvas.set_width(width);
    canvas.set_height(height);
    let context = canvas
        .get_context("2d")
        .map_err(|err| format!("{err:?}"))?
        .ok_or_else(|| "2d canvas context not available".to_string())?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast to CanvasRenderingContext2d".to_string())?;
    context.clear_rect(0.0, 0.0, width as f64, height as f64);
    context
        .draw_image_with_html_image_element(&image, 0.0, 0.0)
        .map_err(|err| format!("Canvas draw failed: {err:?}"))?;

    Ok((width, height))
}

#[cfg(not(target_arch = "wasm32"))]
async fn draw_source_image_to_canvas(
    _canvas_id: &str,
    _source_url: &str,
) -> Result<(u32, u32), String> {
    Err("Canvas drawing is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
fn clear_canvas(canvas_id: &str) {
    use web_sys::wasm_bindgen::JsCast;

    let Some(document) = window().document() else {
        return;
    };
    let Some(element) = document.get_element_by_id(canvas_id) else {
        return;
    };
    let Ok(canvas) = element.dyn_into::<web_sys::HtmlCanvasElement>() else {
        return;
    };
    let Ok(Some(context)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(context) = context.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
        return;
    };
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    canvas.set_width(0);
    canvas.set_height(0);
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_canvas(_canvas_id: &str) {}

#[cfg(target_arch = "wasm32")]
async fn decode_image_dimensions(file: &web_sys::File) -> Result<Dimensions, String> {
    let object_url =
        web_sys::Url::create_object_url_with_blob(file).map_err(|err| format!("{err:?}"))?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(&object_url);
    let decode_result = JsFuture::from(image.decode()).await;
    let _ = web_sys::Url::revoke_object_url(&object_url);
    decode_result.map_err(|err| format!("Image decode failed: {err:?}"))?;
    Ok(Dimensions {
        width: image.natural_width(),
        height: image.natural_height(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn decode_image_dimensions(_file: &web_sys::File) -> Result<Dimensions, String> {
    Err("Image decode is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn crop_face_bytes_from_source(
    source_url: &str,
    face: &DetectedFace,
    settings: &ProcessingSettings,
    mime_type: &str,
) -> Result<Vec<u8>, String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use web_sys::js_sys::{Function, Promise};
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;
    use web_sys::wasm_bindgen::closure::Closure;

    let document = window()
        .document()
        .ok_or_else(|| "Document is unavailable".to_string())?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(source_url);
    JsFuture::from(image.decode())
        .await
        .map_err(|err| format!("Image decode failed: {err:?}"))?;

    let canvas = document
        .create_element("canvas")
        .map_err(|err| format!("{err:?}"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Failed to build crop canvas".to_string())?;
    let crop_w = settings.output_width.max(1);
    let crop_h = settings.output_height.max(1);
    canvas.set_width(crop_w);
    canvas.set_height(crop_h);

    let context = canvas
        .get_context("2d")
        .map_err(|err| format!("{err:?}"))?
        .ok_or_else(|| "2d canvas context not available".to_string())?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast canvas context".to_string())?;
    let mut source_x = face.x - ((settings.horizontal_offset_pct as f64 / 100.0) * face.width);
    let mut source_y = face.y - ((settings.vertical_offset_pct as f64 / 100.0) * face.height);
    if source_x < 0.0 {
        source_x = 0.0;
    }
    if source_y < 0.0 {
        source_y = 0.0;
    }
    let filter = format!(
        "brightness({:.0}%) contrast({:.0}%) blur({:.2}px)",
        (100.0 + settings.exposure_adjustment as f64 * 50.0).clamp(50.0, 200.0),
        (settings.contrast_adjustment as f64 * 100.0).clamp(50.0, 200.0),
        settings.background_blur as f64 + settings.skin_smoothing as f64 * 0.2
    );
    context.set_filter(&filter);
    context
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &image,
            source_x,
            source_y,
            face.width.max(1.0),
            face.height.max(1.0),
            0.0,
            0.0,
            crop_w as f64,
            crop_h as f64,
        )
        .map_err(|err| format!("Crop draw failed: {err:?}"))?;

    let mime = mime_type.to_string();
    let canvas_clone = canvas.clone();
    let blob_promise = Promise::new(&mut move |resolve: Function, reject: Function| {
        let resolve_fn = resolve.clone();
        let reject_fn = reject.clone();
        let slot: Rc<RefCell<Option<Closure<dyn FnMut(Option<web_sys::Blob>)>>>> =
            Rc::new(RefCell::new(None));
        let slot_for_cb: Rc<RefCell<Option<Closure<dyn FnMut(Option<web_sys::Blob>)>>>> =
            Rc::clone(&slot);
        let callback = Closure::new(move |blob: Option<web_sys::Blob>| {
            if let Some(blob) = blob {
                let _ = resolve_fn.call1(&JsValue::NULL, &blob);
            } else {
                let _ = reject_fn.call1(
                    &JsValue::NULL,
                    &JsValue::from_str("Crop to_blob produced no data"),
                );
            }
            slot_for_cb.borrow_mut().take();
        });
        match canvas_clone.to_blob_with_type(callback.as_ref().unchecked_ref(), &mime) {
            Ok(()) => {
                *slot.borrow_mut() = Some(callback);
            }
            Err(err) => {
                let _ = reject.call1(
                    &JsValue::NULL,
                    &JsValue::from(format!("Crop to_blob failed: {err:?}")),
                );
                slot.borrow_mut().take();
            }
        }
    });

    let blob_js = JsFuture::from(blob_promise)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let blob = blob_js
        .dyn_into::<web_sys::Blob>()
        .map_err(|_| "Failed to cast crop blob".to_string())?;
    let buffer = JsFuture::from(blob.array_buffer())
        .await
        .map_err(|err| format!("Failed to read crop bytes: {err:?}"))?;
    let bytes = web_sys::js_sys::Uint8Array::new(&buffer).to_vec();
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
async fn crop_face_bytes_from_source(
    _source_url: &str,
    _face: &DetectedFace,
    _settings: &ProcessingSettings,
    _mime_type: &str,
) -> Result<Vec<u8>, String> {
    Err("Face crop export is only available on wasm32".to_string())
}

fn apply_detection_quality_filters(
    mut faces: Vec<DetectedFace>,
    settings: &ProcessingSettings,
) -> Vec<DetectedFace> {
    faces.retain(|face| face.confidence as f32 >= settings.min_confidence);
    faces
}

fn overlay_percent_rect(
    face: &DetectedFace,
    source_width: f64,
    source_height: f64,
) -> (f64, f64, f64, f64) {
    let mut x = face.x;
    let mut y = face.y;
    let mut width = face.width;
    let mut height = face.height;

    // Some detectors return normalized [0..1] boxes; map those to source pixels first.
    if width <= 1.0 && height <= 1.0 && x <= 1.0 && y <= 1.0 {
        x *= source_width;
        y *= source_height;
        width *= source_width;
        height *= source_height;
    }

    let left = (x / source_width * 100.0).clamp(0.0, 100.0);
    let top = (y / source_height * 100.0).clamp(0.0, 100.0);
    let w = (width / source_width * 100.0).clamp(0.0, 100.0);
    let h = (height / source_height * 100.0).clamp(0.0, 100.0);
    (left, top, w, h)
}

fn render_naming_template(
    template: &str,
    original_file_name: &str,
    index_zero_based: usize,
    output_width: u32,
    output_height: u32,
    timestamp_ms: u64,
) -> String {
    let original = original_file_name
        .rsplit_once('.')
        .map(|(base, _)| base)
        .unwrap_or(original_file_name);
    template
        .replace("{original}", original)
        .replace("{index}", &(index_zero_based + 1).to_string())
        .replace("{timestamp}", &timestamp_ms.to_string())
        .replace("{width}", &output_width.to_string())
        .replace("{height}", &output_height.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
    use web_sys::js_sys::Array;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;

    let media_devices = window()
        .navigator()
        .media_devices()
        .map_err(|err| format!("MediaDevices unavailable: {err:?}"))?;
    let devices_js = JsFuture::from(
        media_devices
            .enumerate_devices()
            .map_err(|err| format!("Failed to enumerate devices: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("Failed to enumerate devices: {err:?}"))?;
    let devices = Array::from(&devices_js);
    let mut video_inputs = Vec::new();
    for idx in 0..devices.length() {
        let raw = devices.get(idx);
        let kind = web_sys::js_sys::Reflect::get(&raw, &JsValue::from_str("kind"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        if kind != "videoinput" {
            continue;
        }
        let Ok(device) = raw.dyn_into::<web_sys::MediaDeviceInfo>() else {
            continue;
        };
        let label = if device.label().is_empty() {
            format!("Camera {}", video_inputs.len() + 1)
        } else {
            device.label()
        };
        video_inputs.push((device.device_id(), label));
    }
    Ok(video_inputs)
}

#[cfg(not(target_arch = "wasm32"))]
async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
    Err("Webcam listing is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
fn stop_media_stream(stream: &web_sys::MediaStream) {
    use web_sys::js_sys::Array;
    use web_sys::wasm_bindgen::JsCast;

    let tracks = Array::from(&stream.get_tracks());
    for idx in 0..tracks.length() {
        let raw = tracks.get(idx);
        if let Ok(track) = raw.dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn stop_media_stream(_stream: &web_sys::MediaStream) {}

#[cfg(target_arch = "wasm32")]
fn clear_video_source(video_id: &str) {
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;

    let Some(document) = window().document() else {
        return;
    };
    let Some(element) = document.get_element_by_id(video_id) else {
        return;
    };
    let Ok(video) = element.dyn_into::<web_sys::HtmlVideoElement>() else {
        return;
    };
    let _ = web_sys::js_sys::Reflect::set(
        video.as_ref(),
        &JsValue::from_str("srcObject"),
        &JsValue::NULL,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_video_source(_video_id: &str) {}

#[cfg(target_arch = "wasm32")]
async fn start_webcam_stream(
    video_id: &str,
    preferred_device_id: Option<&str>,
) -> Result<web_sys::MediaStream, String> {
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;

    let media_devices = window()
        .navigator()
        .media_devices()
        .map_err(|err| format!("MediaDevices unavailable: {err:?}"))?;

    let constraints_obj = web_sys::js_sys::Object::new();
    let _ = web_sys::js_sys::Reflect::set(
        &constraints_obj,
        &JsValue::from_str("audio"),
        &JsValue::FALSE,
    );

    let video_constraints = web_sys::js_sys::Object::new();
    let _ = web_sys::js_sys::Reflect::set(
        &video_constraints,
        &JsValue::from_str("width"),
        &JsValue::from_f64(1920.0),
    );
    let _ = web_sys::js_sys::Reflect::set(
        &video_constraints,
        &JsValue::from_str("height"),
        &JsValue::from_f64(1080.0),
    );
    if let Some(device_id) = preferred_device_id {
        let exact = web_sys::js_sys::Object::new();
        let _ = web_sys::js_sys::Reflect::set(
            &exact,
            &JsValue::from_str("exact"),
            &JsValue::from_str(device_id),
        );
        let _ = web_sys::js_sys::Reflect::set(
            &video_constraints,
            &JsValue::from_str("deviceId"),
            &exact,
        );
    } else {
        let _ = web_sys::js_sys::Reflect::set(
            &video_constraints,
            &JsValue::from_str("facingMode"),
            &JsValue::from_str("user"),
        );
    }
    let _ = web_sys::js_sys::Reflect::set(
        &constraints_obj,
        &JsValue::from_str("video"),
        &video_constraints,
    );

    let constraints = constraints_obj.unchecked_into::<web_sys::MediaStreamConstraints>();
    let media_stream_js = JsFuture::from(
        media_devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|err| format!("getUserMedia failed to start: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("getUserMedia was rejected: {err:?}"))?;
    let stream = media_stream_js
        .dyn_into::<web_sys::MediaStream>()
        .map_err(|_| "Failed to cast getUserMedia result to MediaStream".to_string())?;

    let document = window()
        .document()
        .ok_or_else(|| "Document is unavailable".to_string())?;
    let video = document
        .get_element_by_id(video_id)
        .ok_or_else(|| format!("Video element #{video_id} not found"))?
        .dyn_into::<web_sys::HtmlVideoElement>()
        .map_err(|_| format!("Element #{video_id} is not a video"))?;
    video.set_muted(true);
    let _ = web_sys::js_sys::Reflect::set(
        video.as_ref(),
        &JsValue::from_str("srcObject"),
        stream.as_ref(),
    );
    let _ = video.play();
    Ok(stream)
}

#[cfg(not(target_arch = "wasm32"))]
async fn start_webcam_stream(
    _video_id: &str,
    _preferred_device_id: Option<&str>,
) -> Result<web_sys::MediaStream, String> {
    Err("Webcam streaming is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn capture_webcam_frame_to_file(
    video_id: &str,
    canvas_id: &str,
) -> Result<web_sys::File, String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use web_sys::js_sys::{Array, Function, Promise};
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;
    use web_sys::wasm_bindgen::closure::Closure;

    let document = window()
        .document()
        .ok_or_else(|| "Document is unavailable".to_string())?;
    let video = document
        .get_element_by_id(video_id)
        .ok_or_else(|| format!("Video element #{video_id} not found"))?
        .dyn_into::<web_sys::HtmlVideoElement>()
        .map_err(|_| format!("Element #{video_id} is not a video"))?;
    let width = video.video_width();
    let height = video.video_height();
    if width == 0 || height == 0 {
        return Err("Webcam stream is not ready yet".to_string());
    }

    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| format!("Canvas #{canvas_id} not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| format!("Element #{canvas_id} is not a canvas"))?;
    canvas.set_width(width);
    canvas.set_height(height);
    let context = canvas
        .get_context("2d")
        .map_err(|err| format!("{err:?}"))?
        .ok_or_else(|| "2d canvas context not available".to_string())?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast to CanvasRenderingContext2d".to_string())?;
    context
        .draw_image_with_html_video_element(&video, 0.0, 0.0)
        .map_err(|err| format!("Failed to draw webcam frame: {err:?}"))?;

    let canvas_clone = canvas.clone();
    let blob_promise = Promise::new(&mut move |resolve: Function, reject: Function| {
        let resolve_fn = resolve.clone();
        let reject_fn = reject.clone();
        let callback_slot: Rc<RefCell<Option<Closure<dyn FnMut(Option<web_sys::Blob>)>>>> =
            Rc::new(RefCell::new(None));
        let callback_slot_for_cb: Rc<RefCell<Option<Closure<dyn FnMut(Option<web_sys::Blob>)>>>> =
            Rc::clone(&callback_slot);
        let callback = Closure::new(move |blob: Option<web_sys::Blob>| {
            if let Some(blob) = blob {
                let _ = resolve_fn.call1(&JsValue::NULL, &blob);
            } else {
                let _ = reject_fn.call1(
                    &JsValue::NULL,
                    &JsValue::from_str("Canvas to_blob produced no data"),
                );
            }
            callback_slot_for_cb.borrow_mut().take();
        });
        match canvas_clone.to_blob(callback.as_ref().unchecked_ref()) {
            Ok(()) => {
                *callback_slot.borrow_mut() = Some(callback);
            }
            Err(err) => {
                let _ = reject.call1(
                    &JsValue::NULL,
                    &JsValue::from(format!("Canvas to_blob failed: {err:?}")),
                );
                callback_slot.borrow_mut().take();
            }
        }
    });

    let blob_js = JsFuture::from(blob_promise)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let blob = blob_js
        .dyn_into::<web_sys::Blob>()
        .map_err(|_| "Failed to cast captured blob".to_string())?;
    let parts = Array::new();
    parts.push(&blob);
    let file_options = web_sys::FilePropertyBag::new();
    file_options.set_type("image/png");
    web_sys::File::new_with_blob_sequence_and_options(&parts, "webcam-capture.png", &file_options)
        .map_err(|err| format!("Failed to build capture file: {err:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
async fn capture_webcam_frame_to_file(
    _video_id: &str,
    _canvas_id: &str,
) -> Result<web_sys::File, String> {
    Err("Webcam capture is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
const THEME_STORAGE_KEY: &str = "fcf.theme";

#[cfg(target_arch = "wasm32")]
fn load_theme_mode() -> ThemeMode {
    let value = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(THEME_STORAGE_KEY).ok().flatten());
    match value.as_deref() {
        Some("light") => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_theme_mode() -> ThemeMode {
    ThemeMode::Dark
}

#[cfg(target_arch = "wasm32")]
fn persist_theme_mode(mode: ThemeMode) {
    if let Ok(Some(storage)) = window().local_storage() {
        let value = match mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        };
        let _ = storage.set_item(THEME_STORAGE_KEY, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_theme_mode(_mode: ThemeMode) {}

#[cfg(target_arch = "wasm32")]
fn apply_theme_mode(mode: ThemeMode) {
    if let Some(root) = window().document().and_then(|doc| doc.document_element()) {
        match mode {
            ThemeMode::Light => {
                let _ = root.set_attribute("data-theme", "light");
            }
            ThemeMode::Dark => {
                let _ = root.remove_attribute("data-theme");
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_theme_mode(_mode: ThemeMode) {}

#[cfg(test)]
mod route_tests {
    use super::{RouteTarget, route_for_path};

    #[test]
    fn primary_routes_resolve_to_expected_targets() {
        assert_eq!(route_for_path("/"), RouteTarget::Landing);
        assert_eq!(route_for_path("/single"), RouteTarget::Single);
        assert_eq!(route_for_path("/batch"), RouteTarget::Batch);
        assert_eq!(route_for_path("/csv"), RouteTarget::Csv);
    }

    #[test]
    fn legacy_html_paths_resolve_to_leptos_targets() {
        assert_eq!(
            route_for_path("/single-processing.html"),
            RouteTarget::Single
        );
        assert_eq!(route_for_path("/batch-processing.html"), RouteTarget::Batch);
        assert_eq!(route_for_path("/csv-processing.html"), RouteTarget::Csv);
    }
}

#[component]
fn AppShell(title: &'static str, children: Children) -> impl IntoView {
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
fn ThemeToggleButton(id: &'static str) -> impl IntoView {
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
fn SingleUploadCard(
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
fn BatchUploadCard(
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

fn batch_file_label(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

#[component]
fn BatchImageGalleryPanel(
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
fn CsvUploadCard(
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
fn CsvImageUploadCard(
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

#[component]
fn CsvPage() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let csv_state = RwSignal::new(CsvCoreState::default());
    let csv_batch_state = RwSignal::new(BatchCoreState::default());
    let csv_queue = RwSignal::new(BatchQueueState::default());
    let csv_files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let csv_preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let csv_source_name_by_id = RwSignal::new(HashMap::<String, String>::new());
    let csv_current_image_id = RwSignal::new(None::<String>);
    let csv_current_faces = RwSignal::new(Vec::<DetectedFace>::new());
    let csv_current_dimensions = RwSignal::new((0.0_f64, 0.0_f64));
    let csv_face_count_by_id = RwSignal::new(HashMap::<String, usize>::new());
    let csv_progress = RwSignal::new(BatchProgress::default());
    let csv_stats = RwSignal::new(BatchRuntimeStats::default());
    let csv_preview_filename = RwSignal::new(String::new());
    let mapping_confirmed = RwSignal::new(false);
    let file_path_column = RwSignal::new(String::new());
    let file_name_column = RwSignal::new(String::new());
    let headers = Signal::derive(move || csv_state.get().headers);
    let preview_rows = Signal::derive(move || csv_state.get().preview_rows(5));
    let can_confirm_mapping = Signal::derive(move || {
        csv_state
            .get()
            .can_confirm_mapping(&file_path_column.get(), &file_name_column.get())
    });
    let mapping_status = Signal::derive(move || {
        if mapping_confirmed.get() && csv_state.get().mapping.is_some() {
            "Mapping ready".to_string()
        } else {
            "Awaiting mapping".to_string()
        }
    });
    let total_rows = Signal::derive(move || csv_state.get().rows.len().to_string());
    let matched_images = Signal::derive(move || {
        let loaded = csv_batch_state.get().total_count();
        let queued = csv_queue.get().queued_files_count();
        (loaded + queued).to_string()
    });
    let has_images = Signal::derive(move || {
        csv_batch_state.get().total_count() + csv_queue.get().queued_files_count() > 0
    });
    let has_selected_images = Signal::derive(move || csv_batch_state.get().has_selected_images());
    let csv_progress_percent = Signal::derive(move || csv_progress.get().percent());
    let csv_progress_status = Signal::derive(move || csv_progress.get().status);
    let csv_progress_running = Signal::derive(move || csv_progress.get().running);
    let csv_busy = Signal::derive(move || csv_progress.get().running);
    let csv_total_faces = Signal::derive(move || csv_stats.get().total_faces_detected.to_string());
    let csv_success_rate =
        Signal::derive(move || format!("{}%", csv_stats.get().success_rate_pct()));
    let csv_avg_processing_time =
        Signal::derive(move || format!("{}ms", csv_stats.get().avg_processing_time_ms()));
    let csv_images_processed = Signal::derive(move || csv_stats.get().images_processed.to_string());
    let csv_logs = Signal::derive(move || csv_stats.get().logs);
    let csv_error_logs = Signal::derive(move || {
        csv_stats
            .get()
            .logs
            .into_iter()
            .filter(|entry| {
                let lower = entry.to_lowercase();
                lower.contains("error") || lower.contains("failed")
            })
            .collect::<Vec<_>>()
    });
    let csv_current_image_url = Signal::derive(move || {
        csv_current_image_id
            .get()
            .and_then(|id| csv_preview_urls.get().get(&id).cloned())
    });
    let csv_current_faces_label = Signal::derive(move || csv_current_faces.get().len().to_string());

    Effect::new(move |_| {
        if csv_state.get().mapping.is_none() {
            mapping_confirmed.set(false);
        }
    });

    Effect::new(move |_| {
        if let Some(url) = csv_current_image_url.get() {
            let dims = csv_current_dimensions;
            leptos::task::spawn_local(async move {
                if let Ok((w, h)) = draw_source_image_to_canvas("csvInputCanvas", &url).await {
                    dims.set((w as f64, h as f64));
                }
            });
        } else {
            csv_current_dimensions.set((0.0, 0.0));
            clear_canvas("csvInputCanvas");
        }
    });

    view! {
        <div class="app-shell">
            <header class="app-header">
                <div class="title-row">
                    <h1><a href="/">"Face Crop Forge"</a></h1>
                    <div class="header-actions">
                        <button type="button" id="multipleImageModeBtn" class="ghost-btn" title="Switch to multiple image mode">
                            <span>"Multiple Images"</span>
                        </button>
                        <button type="button" id="singleImageModeBtn" class="ghost-btn" title="Switch to single image mode">
                            <span>"Single Image"</span>
                        </button>
                        <ThemeToggleButton id="darkModeBtn" />
                    </div>
                </div>

                <CsvUploadCard state=csv_state progress=csv_progress busy=csv_busy />

                <div class=move || {
                    if headers.get().is_empty() {
                        "csv-mapping hidden"
                    } else {
                        "csv-mapping"
                    }
                } id="csvMapping">
                    <h3>
                        "Column Mapping"
                        " ("
                        {mapping_status}
                        ")"
                    </h3>
                    <div class="mapping-controls">
                        <div class="mapping-group">
                            <label for="filePathColumn">"File Path Column:"</label>
                            <select id="filePathColumn" on:change=move |ev| file_path_column.set(event_target_value(&ev))>
                                <option value="">"Select column..."</option>
                                {move || headers.get().into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect_view()}
                            </select>
                        </div>
                        <div class="mapping-group">
                            <label for="fileNameColumn">"Output Name Column:"</label>
                            <select id="fileNameColumn" on:change=move |ev| file_name_column.set(event_target_value(&ev))>
                                <option value="">"Select column..."</option>
                                {move || headers.get().into_iter().map(|header| view! { <option value=header.clone()>{header.clone()}</option> }).collect_view()}
                            </select>
                        </div>
                        <button
                            type="button"
                            id="confirmMappingBtn"
                            class="primary-btn"
                            disabled=move || !can_confirm_mapping.get()
                            on:click=move |_| {
                                let path_col = file_path_column.get();
                                let name_col = file_name_column.get();
                                let mut applied = false;
                                csv_state.update(|s| {
                                    applied = s.apply_mapping(&path_col, &name_col);
                                });
                                mapping_confirmed.set(applied);
                            }
                        >
                            "Confirm Mapping"
                        </button>
                    </div>
                    <div class="csv-preview">
                        <h4>"CSV Preview (First 5 rows):"</h4>
                        <div id="csvPreviewTable">
                            <table class="csv-preview-table">
                                <thead>
                                    <tr>
                                        {move || headers.get().into_iter().map(|header| view! { <th>{header}</th> }).collect_view()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || preview_rows.get().into_iter().map(|row| {
                                        view! {
                                            <tr>
                                                {row.into_iter().map(|cell| view! { <td>{cell}</td> }).collect_view()}
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <div id="image-upload-card-container" class=move || if mapping_confirmed.get() { "" } else { "hidden" }>
                    <CsvImageUploadCard
                        state=csv_batch_state
                        queue=csv_queue
                        csv_state=csv_state
                        files_by_id=csv_files_by_id
                        preview_urls=csv_preview_urls
                        source_name_by_id=csv_source_name_by_id
                        progress=csv_progress
                        busy=csv_busy
                    />
                </div>

                <div class="batch-controls">
                    <button
                        type="button"
                        id="processAllBtn"
                        disabled=move || !has_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = csv_batch_state.get().build_work_plan(64);
                            if plan.selected_total == 0 {
                                csv_progress.update(|p| p.complete("No mapped images selected"));
                                return;
                            }
                            let selected_ids = csv_batch_state.get().selected_ids();
                            let files_by_id = csv_files_by_id.get();
                            let validation = ImageValidationConfig::default();
                            csv_batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            csv_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "CSV queue: {} image(s) in {} chunk(s)",
                                        plan.selected_total,
                                        plan.chunks.len()
                                    ),
                                );
                            });
                            let batch_state_for_run = csv_batch_state;
                            let progress_for_run = csv_progress;
                            let stats_for_run = csv_stats;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("CSV missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!("CSV decode failed for {}: {error}", file.name()));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            continue;
                                        }
                                    };
                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    progress_for_run.update(|p| {
                                        p.status = format!(
                                            "CSV processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "CSV validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    }

                                    let start = Instant::now();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    for _ in 0..=1 {
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => last_error = error,
                                        }
                                    }
                                    let elapsed_ms = start.elapsed().as_millis() as u64;
                                    match success_faces {
                                        Some(faces) => {
                                            let face_count = faces.len();
                                            progress_for_run.update(|p| {
                                                p.record_result(true);
                                                p.status = format!(
                                                    "CSV processed {}/{}: {} ({} face(s))",
                                                    index + 1,
                                                    total,
                                                    file_name,
                                                    face_count
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, face_count as u32, true);
                                                let source_name = source_name_by_id
                                                    .get(&id)
                                                    .cloned()
                                                    .unwrap_or_else(|| file_name.clone());
                                                stats.push_log(format!(
                                                    "CSV processed {} in {}ms ({} face(s)).",
                                                    source_name, elapsed_ms, face_count
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.insert(id.clone(), face_count);
                                            });
                                            current_id_for_run.set(Some(id.clone()));
                                            current_faces_for_run.set(faces);
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "CSV failed {}: {}",
                                                    file_name, last_error
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            stopped_early = false;
                                        }
                                    }
                                }
                                progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("CSV run stopped early");
                                    } else {
                                        p.complete("CSV process-all completed with real detection");
                                    }
                                });
                            });
                        }
                    >
                        "Process All"
                    </button>
                    <button
                        type="button"
                        id="processSelectedBtn"
                        disabled=move || !has_selected_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = csv_batch_state.get().build_work_plan(64);
                            if plan.selected_total == 0 {
                                csv_progress.update(|p| p.complete("No mapped images selected"));
                                return;
                            }
                            let selected_ids = csv_batch_state.get().selected_ids();
                            let files_by_id = csv_files_by_id.get();
                            let validation = ImageValidationConfig::default();
                            csv_batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            csv_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "CSV selected queue: {} image(s) in {} chunk(s)",
                                        plan.selected_total,
                                        plan.chunks.len()
                                    ),
                                );
                            });
                            let batch_state_for_run = csv_batch_state;
                            let progress_for_run = csv_progress;
                            let stats_for_run = csv_stats;
                            let current_id_for_run = csv_current_image_id;
                            let current_faces_for_run = csv_current_faces;
                            let face_count_for_run = csv_face_count_by_id;
                            let source_name_by_id = csv_source_name_by_id.get();
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV selected failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("CSV missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    };
                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV selected failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!("CSV decode failed for {}: {error}", file.name()));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            continue;
                                        }
                                    };
                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    progress_for_run.update(|p| {
                                        p.status = format!(
                                            "CSV selected processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "CSV selected failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "CSV validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        continue;
                                    }
                                    let start = Instant::now();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    for _ in 0..=1 {
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => last_error = error,
                                        }
                                    }
                                    let elapsed_ms = start.elapsed().as_millis() as u64;
                                    match success_faces {
                                        Some(faces) => {
                                            let face_count = faces.len();
                                            progress_for_run.update(|p| {
                                                p.record_result(true);
                                                p.status = format!(
                                                    "CSV selected processed {}/{}: {} ({} face(s))",
                                                    index + 1,
                                                    total,
                                                    file_name,
                                                    face_count
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, face_count as u32, true);
                                                let source_name = source_name_by_id
                                                    .get(&id)
                                                    .cloned()
                                                    .unwrap_or_else(|| file_name.clone());
                                                stats.push_log(format!(
                                                    "CSV selected processed {} in {}ms ({} face(s)).",
                                                    source_name, elapsed_ms, face_count
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.insert(id.clone(), face_count);
                                            });
                                            current_id_for_run.set(Some(id.clone()));
                                            current_faces_for_run.set(faces);
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "CSV selected failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "CSV selected failed {}: {}",
                                                    file_name, last_error
                                                ));
                                            });
                                            face_count_for_run.update(|m| {
                                                m.remove(&id);
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                        }
                                    }
                                }
                                progress_for_run
                                    .update(|p| p.complete("CSV selected processing completed with real detection"));
                            });
                        }
                    >
                        "Process Selected"
                    </button>
                    <button
                        type="button"
                        id="clearAllBtn"
                        disabled=move || csv_busy.get() || (!has_images.get() && csv_state.get().rows.is_empty())
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_batch_state.update(|s| s.set_images(Vec::new()));
                            csv_queue.set(BatchQueueState::default());
                            csv_files_by_id.set(HashMap::new());
                            for url in csv_preview_urls.get().values() {
                                revoke_object_url(url);
                            }
                            csv_preview_urls.set(HashMap::new());
                            csv_source_name_by_id.set(HashMap::new());
                            csv_current_image_id.set(None);
                            csv_current_faces.set(Vec::new());
                            csv_current_dimensions.set((0.0, 0.0));
                            csv_face_count_by_id.set(HashMap::new());
                            csv_state.update(|s| {
                                s.rows.clear();
                                s.headers.clear();
                                s.mapping = None;
                                s.filename_to_output.clear();
                            });
                            csv_stats.update(|s| s.reset());
                            csv_progress.update(|p| p.reset());
                            csv_preview_filename.set(String::new());
                            mapping_confirmed.set(false);
                            file_path_column.set(String::new());
                            file_name_column.set(String::new());
                        }
                    >
                        "Clear All"
                    </button>
                    <button
                        type="button"
                        id="downloadAllBtn"
                        disabled=move || !has_images.get() || csv_busy.get()
                        on:click=move |_| {
                            if csv_busy.get() {
                                return;
                            }
                            csv_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    csv_batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let batch = csv_batch_state.get();
                            let source_ids = if batch.has_selected_images() {
                                batch.selected_ids()
                            } else {
                                batch.images.values().map(|img| img.id.clone()).collect::<Vec<_>>()
                            };
                            if source_ids.is_empty() {
                                csv_progress.update(|p| p.complete("No mapped images to export"));
                                return;
                            }

                            let csv = csv_state.get();
                            let source_name_by_id = csv_source_name_by_id.get();
                            let timestamp_ms = current_timestamp_ms();
                            let export_settings = settings.get();
                            let file_map = csv_files_by_id.get();
                            let entries = source_ids
                                .iter()
                                .enumerate()
                                .map(|(idx, id)| {
                                    let source_name = source_name_by_id
                                        .get(id)
                                        .cloned()
                                        .unwrap_or_else(|| batch_file_label(id).to_string());
                                    let output_name = csv.output_name_for_file(&source_name);
                                    let generated = CsvCoreState::generate_export_filename(CsvExportNameContext {
                                        template: &export_settings.naming_template,
                                        csv_output_name: output_name.as_deref(),
                                        original_file_name: &source_name,
                                        face_index: idx,
                                        timestamp_ms,
                                        output_width: export_settings.output_width,
                                        output_height: export_settings.output_height,
                                        output_format: &export_settings.output_format,
                                    });
                                    (id.clone(), generated, source_name)
                                })
                                .collect::<Vec<_>>();

                            if let Some(first) = entries.first() {
                                csv_preview_filename.set(first.1.clone());
                            }
                            let zip_name =
                                format!("face-crops-{}.zip", current_utc_timestamp_token());
                            let progress_for_download = csv_progress;
                            let stats_for_download = csv_stats;
                            leptos::task::spawn_local(async move {
                                let mut zip_entries = Vec::new();
                                for (id, generated_name, source_name) in entries {
                                    let Some(file) = file_map.get(&id).cloned() else {
                                        continue;
                                    };
                                    let mime_type = file.type_();
                                    let bytes = match file_to_bytes(&file).await {
                                        Ok(bytes) => bytes,
                                        Err(error) => {
                                            progress_for_download
                                                .update(|p| p.complete(format!("CSV ZIP failed: {error}")));
                                            stats_for_download.update(|s| {
                                                s.push_log(format!(
                                                    "CSV ZIP read failed for {source_name}: {error}"
                                                ))
                                            });
                                            return;
                                        }
                                    };
                                    let final_name = normalize_export_filename_for_mime(
                                        &generated_name,
                                        &mime_type,
                                    );
                                    if !validate_export_filename_for_mime(&final_name, &mime_type) {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "CSV ZIP skipped invalid filename/mime pair: {} ({})",
                                                final_name, mime_type
                                            ))
                                        });
                                        continue;
                                    }
                                    zip_entries.push((final_name, bytes));
                                }
                                if zip_entries.is_empty() {
                                    progress_for_download
                                        .update(|p| p.complete("No mapped binary outputs available for ZIP export"));
                                    return;
                                }
                                let zip_bytes = match build_zip_bytes(&zip_entries) {
                                    Ok(bytes) => bytes,
                                    Err(error) => {
                                        progress_for_download
                                            .update(|p| p.complete(format!("CSV ZIP build failed: {error}")));
                                        stats_for_download
                                            .update(|s| s.push_log(format!("CSV ZIP build failed: {error}")));
                                        return;
                                    }
                                };
                                if let Err(error) =
                                    download_bytes(&zip_name, "application/zip", &zip_bytes)
                                {
                                    progress_for_download.update(|p| {
                                        p.complete(format!("CSV ZIP download failed: {error}"))
                                    });
                                    stats_for_download.update(|s| {
                                        s.push_log(format!("CSV ZIP download failed: {error}"))
                                    });
                                    return;
                                }
                                stats_for_download.update(|s| {
                                    s.push_log(format!(
                                        "Exported CSV ZIP {} with {} file(s).",
                                        zip_name,
                                        zip_entries.len()
                                    ))
                                });
                                progress_for_download.update(|p| {
                                    p.complete(format!(
                                        "CSV ZIP exported: {} ({})",
                                        zip_name,
                                        zip_entries.len()
                                    ))
                                });
                            });
                        }
                    >
                        "Download All Results"
                    </button>
                </div>
            </header>

            <div class="app-body">
                <aside class="control-panel">
                    <div class="control-scroll">
                        <div class="workflow-tools">
                            <h3 class="collapsible-header">
                                "CSV Processing Status"
                                <span class="collapse-icon">"▼"</span>
                            </h3>
                            <div class="collapsible-content">
                                <div class="workflow-section">
                                    <h4>"CSV Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Total Rows"</span><span class="stat-value" id="totalRows">{total_rows}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Uploaded"</span><span class="stat-value" id="imagesUploaded">{matched_images}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Matched"</span><span class="stat-value" id="imagesMatched">{matched_images}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Processing Status"</span><span class="stat-value" id="processingStatus">{mapping_status}</span></div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Processing Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Total Faces Detected"</span><span class="stat-value" id="totalFacesDetected">{csv_total_faces}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Success Rate"</span><span class="stat-value" id="successRate">{csv_success_rate}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Avg. Processing Time"</span><span class="stat-value" id="avgProcessingTime">{csv_avg_processing_time}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Images Processed"</span><span class="stat-value" id="imagesProcessed">{csv_images_processed}</span></div>
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Rust CSV Runtime"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item"><span class="stat-label">"Running"</span><span class="stat-value">{move || if csv_progress_running.get() { "Yes" } else { "No" }}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Progress"</span><span class="stat-value">{move || format!("{}%", csv_progress_percent.get())}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Preview Export Name"</span><span class="stat-value">{move || csv_preview_filename.get()}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Queued Pages"</span><span class="stat-value">{move || csv_queue.get().queued_pages_count().to_string()}</span></div>
                                        <div class="stat-item"><span class="stat-label">"Queued Files"</span><span class="stat-value">{move || csv_queue.get().queued_files_count().to_string()}</span></div>
                                    </div>
                                    <div class="setting-help">{csv_progress_status}</div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"File Loading Log"</h4>
                                    <div class="processing-log" id="loadingLog"><div class="log-entry">"Ready to process CSV..."</div></div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Processing Log"</h4>
                                    <div class="processing-log" id="processingLog">
                                        {move || csv_logs.get().into_iter().map(|entry| view! { <div class="log-entry">{entry}</div> }).collect_view()}
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4 class="collapsible-header error-log-header">
                                        "Error Details"
                                        <span class="collapse-icon">"▼"</span>
                                    </h4>
                                    <div class="collapsible-content error-log-panel">
                                        <div class="error-log" id="errorLog">
                                            {move || {
                                                let errors = csv_error_logs.get();
                                                if errors.is_empty() {
                                                    view! { <div class="log-entry">"No errors detected"</div> }
                                                        .into_any()
                                                } else {
                                                    errors
                                                        .into_iter()
                                                        .map(|entry| view! { <div class="log-entry">{entry}</div> })
                                                        .collect_view()
                                                        .into_any()
                                                }
                                            }}
                                        </div>
                                        <div class="error-actions">
                                            <button type="button" id="clearErrorsBtn" class="reset-button">"Clear Errors"</button>
                                            <button type="button" id="exportErrorsBtn" class="export-btn">"Export Error Log"</button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <CropSettingsPanel />
                        <PreprocessingSettingsPanel />
                        <OutputSettingsCsvPanel />
                    </div>
                </aside>

                <main class="workspace">
                    <div class="workspace-scroll">
                        <BatchImageGalleryPanel
                            state=csv_batch_state
                            preview_urls=csv_preview_urls
                            busy=csv_busy
                        />

                        <div class=move || {
                            if csv_current_image_url.get().is_some() {
                                "canvas-container workspace-card"
                            } else {
                                "canvas-container hidden workspace-card"
                            }
                        } id="canvasContainer">
                            <h3 id="canvasTitle">"Current Image Preview"</h3>
                            <div class="canvas-wrapper" id="canvasWrapper">
                                <div class="canvas-panel" id="originalPanel">
                                    <h4 class="panel-title">"Original"</h4>
                                    <div class="panel-content">
                                        <canvas id="csvInputCanvas"></canvas>
                                        <div id="csvFaceOverlays" class="face-overlays">
                                            {move || {
                                                let (source_width, source_height) = csv_current_dimensions.get();
                                                if source_width <= 0.0 || source_height <= 0.0 {
                                                    return view! {}.into_any();
                                                }
                                                csv_current_faces
                                                    .get()
                                                    .into_iter()
                                                    .map(|face| {
                                                        let left = (face.x / source_width * 100.0).clamp(0.0, 100.0);
                                                        let top = (face.y / source_height * 100.0).clamp(0.0, 100.0);
                                                        let width = (face.width / source_width * 100.0).clamp(0.0, 100.0);
                                                        let height =
                                                            (face.height / source_height * 100.0).clamp(0.0, 100.0);
                                                        view! {
                                                            <div
                                                                class="face-box selected"
                                                                style=format!(
                                                                    "left:{left:.3}%;top:{top:.3}%;width:{width:.3}%;height:{height:.3}%;"
                                                                )
                                                            />
                                                        }
                                                    })
                                                    .collect_view()
                                                    .into_any()
                                            }}
                                        </div>
                                    </div>
                                </div>
                                <div class="canvas-panel hidden" id="processedPanel">
                                    <h4 class="panel-title">"Processed"</h4>
                                    <div class="panel-content"><canvas id="csvOutputCanvas"></canvas></div>
                                </div>
                            </div>
                            <div class="face-controls">
                                <div id="faceCounter">
                                    <span>"Detected faces: " <span id="faceCount">{csv_current_faces_label}</span></span>
                                    <span class="separator">"•"</span>
                                    <span>
                                        "Current file: "
                                        <span id="selectedFaceCount">
                                            {move || {
                                                csv_current_image_id
                                                    .get()
                                                    .and_then(|id| {
                                                        csv_source_name_by_id.get().get(&id).cloned()
                                                    })
                                                    .unwrap_or_default()
                                            }}
                                        </span>
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div class="cropped-faces workspace-card" id="croppedFaces">
                            <h3>"Cropped Faces:"</h3>
                            <div id="croppedContainer">
                                {move || {
                                    let mut processed_ids = csv_batch_state
                                        .get()
                                        .images
                                        .values()
                                        .filter(|img| img.processed)
                                        .map(|img| img.id.clone())
                                        .collect::<Vec<_>>();
                                    processed_ids.sort();
                                    let previews = csv_preview_urls.get();
                                    let source_names = csv_source_name_by_id.get();
                                    let face_counts = csv_face_count_by_id.get();
                                    let csv = csv_state.get();
                                    processed_ids
                                        .into_iter()
                                        .map(|id| {
                                            let src = previews.get(&id).cloned();
                                            let source_name = source_names
                                                .get(&id)
                                                .cloned()
                                                .unwrap_or_else(|| batch_file_label(&id).to_string());
                                            let output_name = csv
                                                .output_name_for_file(&source_name)
                                                .unwrap_or_else(|| source_name.clone());
                                            let faces = face_counts.get(&id).copied().unwrap_or(0);
                                            view! {
                                                <div class="cropped-face">
                                                    {src.map(|url| view! { <img src=url alt="CSV processed preview" /> })}
                                                    <div class="setting-help">{format!("Source: {source_name}")}</div>
                                                    <div class="setting-help">{format!("CSV name: {output_name}")}</div>
                                                    <div class="setting-help">{format!("Faces: {faces}")}</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </div>

                        <div class="status workspace-card" id="status">"Ready to load image"</div>
                    </div>
                </main>
            </div>

        </div>
    }
}

#[component]
fn BatchPage() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    let batch_state = RwSignal::new(BatchCoreState::default());
    let batch_queue = RwSignal::new(BatchQueueState::default());
    let batch_files_by_id = RwSignal::new(HashMap::<String, web_sys::File>::new());
    let batch_preview_urls = RwSignal::new(HashMap::<String, String>::new());
    let batch_progress = RwSignal::new(BatchProgress::default());
    let batch_stats = RwSignal::new(BatchRuntimeStats::default());
    let batch_preview_filename = RwSignal::new(String::new());
    let settings_name_input = RwSignal::new(String::new());
    let selected_recent_setting = RwSignal::new(String::new());
    let recent_settings = RwSignal::new(list_saved_setting_names());
    let continue_on_error = RwSignal::new(true);
    let reduced_resolution = RwSignal::new(false);
    let retry_attempts = RwSignal::new("2".to_string());
    let rust_progress_percent = Signal::derive(move || batch_progress.get().percent());
    let rust_progress_status = Signal::derive(move || batch_progress.get().status);
    let rust_progress_running = Signal::derive(move || batch_progress.get().running);
    let batch_busy = Signal::derive(move || batch_progress.get().running);
    let rust_chunk_count =
        Signal::derive(move || batch_state.get().build_work_plan(128).chunks.len());
    let rust_lazy_queued_pages =
        Signal::derive(move || batch_queue.get().queued_pages_count().to_string());
    let rust_lazy_queued_files =
        Signal::derive(move || batch_queue.get().queued_files_count().to_string());
    let total_faces_detected =
        Signal::derive(move || batch_stats.get().total_faces_detected.to_string());
    let success_rate = Signal::derive(move || format!("{}%", batch_stats.get().success_rate_pct()));
    let avg_processing_time =
        Signal::derive(move || format!("{}ms", batch_stats.get().avg_processing_time_ms()));
    let images_processed = Signal::derive(move || batch_stats.get().images_processed.to_string());
    let batch_logs = Signal::derive(move || batch_stats.get().logs);
    let batch_error_logs = Signal::derive(move || {
        batch_stats
            .get()
            .logs
            .into_iter()
            .filter(|entry| {
                let lower = entry.to_lowercase();
                lower.contains("error") || lower.contains("failed")
            })
            .collect::<Vec<_>>()
    });
    let has_images = Signal::derive(move || batch_state.get().total_count() > 0);
    let has_selected_images = Signal::derive(move || batch_state.get().has_selected_images());
    let rust_estimated_preview_mib = Signal::derive(move || {
        let selected = batch_state.get().selected_count().min(128);
        let bytes = BatchCoreState::estimate_preview_memory_bytes(640, 640, selected);
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    });
    let rust_memory_indicator_text = Signal::derive(move || {
        let state = batch_state.get();
        let images = state.total_count();
        let processed = state
            .images
            .values()
            .filter(|image| image.processed)
            .count();
        build_memory_indicator(images, processed, 0).text
    });
    let rust_memory_indicator_level = Signal::derive(move || {
        let state = batch_state.get();
        let images = state.total_count();
        let processed = state
            .images
            .values()
            .filter(|image| image.processed)
            .count();
        match build_memory_indicator(images, processed, 0).level {
            MemoryIndicatorLevel::Hidden => "hidden",
            MemoryIndicatorLevel::Show => "normal",
            MemoryIndicatorLevel::Warning => "warning",
            MemoryIndicatorLevel::Critical => "critical",
        }
        .to_string()
    });

    view! {
        <div class="app-shell">
            <header class="app-header">
                <div class="title-row">
                    <h1><a href="/">"Face Crop Forge"</a></h1>
                    <div class="header-actions">
                        <button type="button" id="singleImageModeBtn" class="ghost-btn" title="Switch to single image mode">
                            <span>"Single Image"</span>
                        </button>
                        <button type="button" id="csvBatchModeBtn" class="ghost-btn" title="Switch to CSV batch mode">
                            <span>"CSV Batch"</span>
                        </button>
                        <ThemeToggleButton id="darkModeBtn" />
                    </div>
                </div>

                <BatchUploadCard
                    state=batch_state
                    queue=batch_queue
                    progress=batch_progress
                    files_by_id=batch_files_by_id
                    preview_urls=batch_preview_urls
                    busy=batch_busy
                />

                <div class="batch-controls">
                    <button
                        type="button"
                        id="processAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = batch_state.get().build_work_plan(128);
                            if plan.selected_total == 0 {
                                batch_progress.update(|p| p.complete("No images selected"));
                                return;
                            }
                            let policy = DetectionRetryPolicy {
                                max_retries: parse_max_retries(Some(&retry_attempts.get())),
                                continue_on_error: continue_on_error.get(),
                                reduced_resolution: reduced_resolution.get(),
                            };
                            let validation = ImageValidationConfig::default();
                            let selected_ids = batch_state.get().selected_ids();
                            let files_by_id = batch_files_by_id.get();
                            batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            batch_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "Planned {} images in {} chunk(s) of up to {}",
                                        plan.selected_total,
                                        plan.chunks.len(),
                                        plan.chunk_size
                                    ),
                                )
                            });
                            let batch_state_for_run = batch_state;
                            let batch_progress_for_run = batch_progress;
                            let batch_stats_for_run = batch_stats;
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "Batch failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        batch_stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("Missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    };

                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "Batch failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!(
                                                    "Decode failed for {}: {error}",
                                                    file.name()
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                            continue;
                                        }
                                    };

                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    batch_progress_for_run.update(|p| {
                                        p.status = format!(
                                            "Batch processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "Batch failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        batch_stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "Validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    }

                                    let run_start = Instant::now();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    let mut attempts = 0_u32;
                                    for attempt in 0..=policy.max_retries {
                                        attempts = attempt + 1;
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => {
                                                last_error = error;
                                            }
                                        }
                                    }
                                    let elapsed_ms = run_start.elapsed().as_millis() as u64;

                                    match success_faces {
                                        Some(faces) => {
                                            let face_count = faces.len() as u32;
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(true);
                                                p.status = format!(
                                                    "Batch processed {}/{}: {} ({} face(s))",
                                                    index + 1,
                                                    total,
                                                    file_name,
                                                    face_count
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, face_count, true);
                                                stats.push_log(format!(
                                                    "Processed {} in {}ms ({} attempt(s), {}x{}, {} face(s)).",
                                                    file_name,
                                                    elapsed_ms,
                                                    attempts,
                                                    dimensions.width,
                                                    dimensions.height,
                                                    face_count
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "Batch failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "Failed {} after {} attempt(s): {}",
                                                    file_name, attempts, last_error
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                batch_progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("Stopped early due to failure (continue on error disabled)");
                                    } else {
                                        p.complete("Batch run completed with real worker detection");
                                    }
                                });
                            });
                        }
                    >
                        "Process All"
                    </button>
                    <button
                        type="button"
                        id="processSelectedBtn"
                        disabled=move || !has_selected_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_queue.update(|q| {
                                while let Some(page) = q.dequeue_next_page() {
                                    batch_state.update(|s| s.add_images(page));
                                }
                            });
                            let plan = batch_state.get().build_work_plan(128);
                            if plan.selected_total == 0 {
                                batch_progress.update(|p| p.complete("No images selected"));
                                return;
                            }
                            let policy = DetectionRetryPolicy {
                                max_retries: parse_max_retries(Some(&retry_attempts.get())),
                                continue_on_error: continue_on_error.get(),
                                reduced_resolution: reduced_resolution.get(),
                            };
                            let validation = ImageValidationConfig::default();
                            let selected_ids = batch_state.get().selected_ids();
                            let files_by_id = batch_files_by_id.get();
                            batch_state.update(|s| selected_ids.iter().for_each(|id| s.mark_processing(id)));
                            batch_progress.update(|p| {
                                p.start(
                                    plan.selected_total,
                                    format!(
                                        "Processing selected set in {} chunk(s) of up to {}",
                                        plan.chunks.len(),
                                        plan.chunk_size
                                    ),
                                )
                            });
                            let batch_state_for_run = batch_state;
                            let batch_progress_for_run = batch_progress;
                            let batch_stats_for_run = batch_stats;
                            let settings_snapshot = settings.get();
                            leptos::task::spawn_local(async move {
                                let mut stopped_early = false;
                                let total = selected_ids.len();
                                for (index, id) in selected_ids.into_iter().enumerate() {
                                    let Some(file) = files_by_id.get(&id).cloned() else {
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "Batch selected failed {}/{}: missing file payload for {}",
                                                index + 1,
                                                total,
                                                batch_file_label(&id)
                                            );
                                        });
                                        batch_stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!("Missing file payload for id {id}."));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    };

                                    let dimensions = match decode_image_dimensions(&file).await {
                                        Ok(dimensions) => dimensions,
                                        Err(error) => {
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "Batch selected failed {}/{}: {} ({error})",
                                                    index + 1,
                                                    total,
                                                    file.name()
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(0, 0, false);
                                                stats.push_log(format!(
                                                    "Decode failed for {}: {error}",
                                                    file.name()
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                            continue;
                                        }
                                    };

                                    let mime_type = file.type_();
                                    let file_name = file.name();
                                    batch_progress_for_run.update(|p| {
                                        p.status = format!(
                                            "Batch selected processing {}/{}: {}",
                                            index + 1,
                                            total,
                                            file_name
                                        );
                                    });
                                    let meta = ImageMeta {
                                        file_name: &file_name,
                                        mime_type: &mime_type,
                                        file_size_bytes: file.size() as u64,
                                        dimensions,
                                    };
                                    if let Err(message) = validate_image_meta(meta, validation) {
                                        batch_progress_for_run.update(|p| {
                                            p.record_result(false);
                                            p.status = format!(
                                                "Batch selected failed {}/{}: {} ({message})",
                                                index + 1,
                                                total,
                                                file_name
                                            );
                                        });
                                        batch_stats_for_run.update(|stats| {
                                            stats.record_image(0, 0, false);
                                            stats.push_log(format!(
                                                "Validation failed for {file_name}: {message}"
                                            ));
                                        });
                                        batch_state_for_run.update(|s| s.mark_error(&id));
                                        if !policy.continue_on_error {
                                            stopped_early = true;
                                            break;
                                        }
                                        continue;
                                    }

                                    let run_start = Instant::now();
                                    let mut success_faces: Option<Vec<DetectedFace>> = None;
                                    let mut last_error = String::new();
                                    let mut attempts = 0_u32;
                                    for attempt in 0..=policy.max_retries {
                                        attempts = attempt + 1;
                                        match detect_faces_with_worker(
                                            "browser-face-detector",
                                            file.clone(),
                                        )
                                        .await
                                        {
                                            Ok(faces) => {
                                                let filtered = apply_detection_quality_filters(
                                                    faces,
                                                    &settings_snapshot,
                                                );
                                                if filtered.is_empty() {
                                                    last_error = "No faces detected".to_string();
                                                } else {
                                                    success_faces = Some(filtered);
                                                    break;
                                                }
                                            }
                                            Err(error) => {
                                                last_error = error;
                                            }
                                        }
                                    }
                                    let elapsed_ms = run_start.elapsed().as_millis() as u64;

                                    match success_faces {
                                        Some(faces) => {
                                            let face_count = faces.len() as u32;
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(true);
                                                p.status = format!(
                                                    "Batch selected processed {}/{}: {} ({} face(s))",
                                                    index + 1,
                                                    total,
                                                    file_name,
                                                    face_count
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, face_count, true);
                                                stats.push_log(format!(
                                                    "Selected run processed {} in {}ms ({} attempt(s), {} face(s)).",
                                                    file_name, elapsed_ms, attempts, face_count
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_processed(&id));
                                        }
                                        None => {
                                            batch_progress_for_run.update(|p| {
                                                p.record_result(false);
                                                p.status = format!(
                                                    "Batch selected failed {}/{}: {} ({last_error})",
                                                    index + 1,
                                                    total,
                                                    file_name
                                                );
                                            });
                                            batch_stats_for_run.update(|stats| {
                                                stats.record_image(elapsed_ms, 0, false);
                                                stats.push_log(format!(
                                                    "Selected run failed {} after {} attempt(s): {}",
                                                    file_name, attempts, last_error
                                                ));
                                            });
                                            batch_state_for_run.update(|s| s.mark_error(&id));
                                            if !policy.continue_on_error {
                                                stopped_early = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                batch_progress_for_run.update(|p| {
                                    if stopped_early {
                                        p.complete("Selected run stopped early on first failure");
                                    } else {
                                        p.complete("Selected run completed with real worker detection");
                                    }
                                });
                            });
                        }
                    >
                        "Process Selected"
                    </button>
                    <button
                        type="button"
                        id="clearAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            batch_state.update(|s| s.set_images(Vec::new()));
                            batch_queue.set(BatchQueueState::default());
                            batch_files_by_id.set(HashMap::new());
                            for url in batch_preview_urls.get().values() {
                                revoke_object_url(url);
                            }
                            batch_preview_urls.set(HashMap::new());
                            batch_progress.update(|p| p.reset());
                            batch_stats.update(|s| s.reset());
                            batch_preview_filename.set(String::new());
                        }
                    >
                        "Clear All"
                    </button>
                    <button
                        type="button"
                        id="downloadAllBtn"
                        disabled=move || !has_images.get() || batch_busy.get()
                        on:click=move |_| {
                            if batch_busy.get() {
                                return;
                            }
                            let source_ids = {
                                let batch = batch_state.get();
                                if batch.has_selected_images() {
                                    batch.selected_ids()
                                } else {
                                    batch.images.values().map(|img| img.id.clone()).collect::<Vec<_>>()
                                }
                            };
                            if source_ids.is_empty() {
                                batch_progress.update(|p| p.complete("No images available for export"));
                                return;
                            }
                            let file_map = batch_files_by_id.get();
                            let export_settings = settings.get();
                            let timestamp_ms = current_timestamp_ms();
                            let zip_name =
                                format!("face-crops-{}.zip", current_utc_timestamp_token());
                            let progress_for_download = batch_progress;
                            let stats_for_download = batch_stats;
                            let preview_for_download = batch_preview_filename;
                            leptos::task::spawn_local(async move {
                                let mut zip_entries = Vec::new();
                                for (idx, id) in source_ids.iter().enumerate() {
                                    let Some(file) = file_map.get(id).cloned() else {
                                        continue;
                                    };
                                    let bytes = match file_to_bytes(&file).await {
                                        Ok(bytes) => bytes,
                                        Err(error) => {
                                            progress_for_download.update(|p| {
                                                p.complete(format!("Batch ZIP failed: {error}"));
                                            });
                                            stats_for_download.update(|s| {
                                                s.push_log(format!("Batch ZIP read failed for {}: {error}", file.name()))
                                            });
                                            return;
                                        }
                                    };
                                    let entry_name = render_naming_template(
                                        &export_settings.naming_template,
                                        &file.name(),
                                        idx,
                                        export_settings.output_width,
                                        export_settings.output_height,
                                        timestamp_ms,
                                    );
                                    let mime_type = file.type_();
                                    let final_name =
                                        normalize_export_filename_for_mime(&entry_name, &mime_type);
                                    if !validate_export_filename_for_mime(&final_name, &mime_type) {
                                        stats_for_download.update(|s| {
                                            s.push_log(format!(
                                                "Batch ZIP skipped invalid filename/mime pair: {} ({})",
                                                final_name, mime_type
                                            ))
                                        });
                                        continue;
                                    }
                                    zip_entries.push((final_name, bytes));
                                }
                                if zip_entries.is_empty() {
                                    progress_for_download
                                        .update(|p| p.complete("No binary outputs available for ZIP export"));
                                    return;
                                }
                                let first_name = zip_entries[0].0.clone();
                                let zip_bytes = match build_zip_bytes(&zip_entries) {
                                    Ok(bytes) => bytes,
                                    Err(error) => {
                                        progress_for_download
                                            .update(|p| p.complete(format!("Batch ZIP build failed: {error}")));
                                        stats_for_download
                                            .update(|s| s.push_log(format!("Batch ZIP build failed: {error}")));
                                        return;
                                    }
                                };
                                if let Err(error) =
                                    download_bytes(&zip_name, "application/zip", &zip_bytes)
                                {
                                    progress_for_download.update(|p| {
                                        p.complete(format!("Batch ZIP download failed: {error}"))
                                    });
                                    stats_for_download.update(|s| {
                                        s.push_log(format!("Batch ZIP download failed: {error}"))
                                    });
                                    return;
                                }
                                preview_for_download.set(first_name.clone());
                                stats_for_download.update(|s| {
                                    s.push_log(format!(
                                        "Exported ZIP {} with {} file(s).",
                                        zip_name,
                                        zip_entries.len()
                                    ))
                                });
                                progress_for_download.update(|p| {
                                    p.complete(format!(
                                        "Batch ZIP exported: {} ({})",
                                        zip_name,
                                        zip_entries.len()
                                    ))
                                });
                            });
                        }
                    >
                        "Download All Results"
                    </button>
                </div>
            </header>

            <div class="app-body">
                <aside class="control-panel">
                    <div class="control-scroll">
                        <div class="workflow-tools">
                            <h3 class="collapsible-header">
                                "Professional Workflow Tools"
                                <span class="collapse-icon">"▼"</span>
                            </h3>
                            <div class="collapsible-content">
                                <div class="workflow-section">
                                    <h4>"Processing Statistics"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Total Faces Detected"</span>
                                            <span class="stat-value" id="totalFacesDetected">{total_faces_detected}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Success Rate"</span>
                                            <span class="stat-value" id="successRate">{success_rate}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Avg. Processing Time"</span>
                                            <span class="stat-value" id="avgProcessingTime">{avg_processing_time}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Images Processed"</span>
                                            <span class="stat-value" id="imagesProcessed">{images_processed}</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Settings Management"</h4>
                                    <div class="settings-management">
                                        <div class="recent-settings">
                                            <label for="recentSettingsDropdown">"Recent Settings"</label>
                                            <select
                                                id="recentSettingsDropdown"
                                                prop:value=move || selected_recent_setting.get()
                                                on:change=move |ev| selected_recent_setting.set(event_target_value(&ev))
                                            >
                                                <option value="">"Select a saved configuration..."</option>
                                                {move || recent_settings.get().into_iter().map(|name| {
                                                    view! { <option value=name.clone()>{name.clone()}</option> }
                                                }).collect_view()}
                                            </select>
                                            <button
                                                type="button"
                                                id="loadRecentBtn"
                                                class="small-btn"
                                                on:click=move |_| {
                                                    let selected = selected_recent_setting.get();
                                                    if selected.is_empty() {
                                                        batch_progress.update(|p| p.complete("Choose a saved configuration first"));
                                                        return;
                                                    }
                                                    if let Some(saved) = load_named_processing_settings(&selected) {
                                                        settings.set(saved);
                                                        batch_progress.update(|p| p.complete(format!("Loaded settings profile '{selected}'")));
                                                    } else {
                                                        batch_progress.update(|p| p.complete(format!("Settings profile '{selected}' not found")));
                                                    }
                                                }
                                            >
                                                "Load"
                                            </button>
                                        </div>

                                        <div class="settings-actions">
                                            <label for="settingsName" class="sr-only">"Configuration name"</label>
                                            <input
                                                type="text"
                                                id="settingsName"
                                                placeholder="Configuration name..."
                                                maxlength="50"
                                                prop:value=move || settings_name_input.get()
                                                on:input=move |ev| settings_name_input.set(event_target_value(&ev))
                                            />
                                            <button
                                                type="button"
                                                id="saveSettingsBtn"
                                                class="save-btn"
                                                on:click=move |_| {
                                                    let name = settings_name_input.get();
                                                    match save_named_processing_settings(&name, &settings.get()) {
                                                        Ok(()) => {
                                                            let names = list_saved_setting_names();
                                                            recent_settings.set(names);
                                                            selected_recent_setting.set(name.clone());
                                                            batch_progress.update(|p| p.complete(format!("Saved settings profile '{name}'")));
                                                        }
                                                        Err(error) => {
                                                            batch_progress.update(|p| p.complete(format!("Save settings failed: {error}")));
                                                        }
                                                    }
                                                }
                                            >
                                                "Save Settings"
                                            </button>
                                            <button
                                                type="button"
                                                id="exportSettingsBtn"
                                                class="export-btn"
                                                on:click=move |_| {
                                                    match export_saved_settings_json() {
                                                        Ok(json) => {
                                                            if let Err(error) = download_bytes(
                                                                "face-crop-settings.json",
                                                                "application/json",
                                                                json.as_bytes(),
                                                            ) {
                                                                batch_progress.update(|p| p.complete(format!("Export settings failed: {error}")));
                                                            } else {
                                                                batch_progress.update(|p| p.complete("Exported settings JSON"));
                                                            }
                                                        }
                                                        Err(error) => {
                                                            batch_progress.update(|p| p.complete(format!("Export settings failed: {error}")));
                                                        }
                                                    }
                                                }
                                            >
                                                "Export JSON"
                                            </button>
                                            <button
                                                type="button"
                                                id="importSettingsBtn"
                                                class="import-btn"
                                                on:click=move |_| click_element_by_id("importSettingsFile")
                                            >
                                                "Import JSON"
                                            </button>
                                            <label for="importSettingsFile" class="sr-only">"Import settings file"</label>
                                            <input
                                                type="file"
                                                id="importSettingsFile"
                                                accept=".json"
                                                class="hidden"
                                                on:change=move |ev| {
                                                    let input: HtmlInputElement = event_target(&ev);
                                                    let Some(files) = input.files() else {
                                                        return;
                                                    };
                                                    let Some(file) = files.get(0) else {
                                                        return;
                                                    };
                                                    let progress_for_import = batch_progress;
                                                    let recent_for_import = recent_settings;
                                                    leptos::task::spawn_local(async move {
                                                        let Ok(js_text) = JsFuture::from(file.text()).await else {
                                                            progress_for_import.update(|p| p.complete("Import settings failed: could not read file"));
                                                            return;
                                                        };
                                                        let Some(text) = js_text.as_string() else {
                                                            progress_for_import.update(|p| p.complete("Import settings failed: invalid file content"));
                                                            return;
                                                        };
                                                        match import_saved_settings_json(&text) {
                                                            Ok(count) => {
                                                                recent_for_import.set(list_saved_setting_names());
                                                                progress_for_import.update(|p| p.complete(format!("Imported {count} settings profile(s)")));
                                                            }
                                                            Err(error) => {
                                                                progress_for_import.update(|p| p.complete(format!("Import settings failed: {error}")));
                                                            }
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Export & Reporting"</h4>
                                    <div class="export-tools">
                                        <button type="button" id="exportLogBtn" class="export-btn">"Export Processing Log"</button>
                                        <button type="button" id="exportCsvBtn" class="export-btn">"Export CSV Report"</button>
                                        <button type="button" id="clearStatsBtn" class="reset-button">"Clear Statistics"</button>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Performance Settings"</h4>
                                    <div class="performance-settings">
                                        <div class="setting-group">
                                            <label>
                                                <input
                                                    type="checkbox"
                                                    id="continueOnError"
                                                    prop:checked=move || continue_on_error.get()
                                                    on:change=move |ev| continue_on_error.set(event_target_checked(&ev))
                                                />
                                                " Continue on Error"
                                            </label>
                                            <div class="setting-help">"Continue processing other images when one fails"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label>
                                                <input
                                                    type="checkbox"
                                                    id="reducedResolution"
                                                    prop:checked=move || reduced_resolution.get()
                                                    on:change=move |ev| reduced_resolution.set(event_target_checked(&ev))
                                                />
                                                " Fast Detection Mode"
                                            </label>
                                            <div class="setting-help">"Process at 50% resolution for faster detection"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label>
                                                <input type="checkbox" id="enableWebWorkers" checked />
                                                " Use Web Workers"
                                            </label>
                                            <div class="setting-help">"Prevent UI blocking during processing"</div>
                                        </div>
                                        <div class="setting-group">
                                            <label for="memoryManagement">"Memory Management"</label>
                                            <select id="memoryManagement">
                                                <option value="auto">"Auto Cleanup"</option>
                                                <option value="aggressive">"Aggressive Cleanup"</option>
                                                <option value="manual">"Manual Only"</option>
                                            </select>
                                        </div>
                                        <div class="setting-group">
                                            <label for="retryAttempts">"Retry Attempts"</label>
                                            <input
                                                type="number"
                                                id="retryAttempts"
                                                min="0"
                                                max="5"
                                                prop:value=move || retry_attempts.get()
                                                on:input=move |ev| retry_attempts.set(event_target_value(&ev))
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div class="workflow-section">
                                    <h4>"Processing Log"</h4>
                                    <div class="processing-log" id="processingLog">
                                        {move || batch_logs.get().into_iter().map(|entry| view! { <div class="log-entry">{entry}</div> }).collect_view()}
                                    </div>
                                </div>
                                <div class="workflow-section">
                                    <h4>"Rust Progress Status"</h4>
                                    <div class="stats-grid">
                                        <div class="stat-item">
                                            <span class="stat-label">"Running"</span>
                                            <span class="stat-value">{move || if rust_progress_running.get() { "Yes" } else { "No" }}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Progress"</span>
                                            <span class="stat-value">{move || format!("{}%", rust_progress_percent.get())}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Chunk Count"</span>
                                            <span class="stat-value">{move || rust_chunk_count.get().to_string()}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Preview Buffer"</span>
                                            <span class="stat-value">{rust_estimated_preview_mib}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Preview Export Name"</span>
                                            <span class="stat-value">{move || batch_preview_filename.get()}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Queued Pages"</span>
                                            <span class="stat-value">{rust_lazy_queued_pages}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Queued Files"</span>
                                            <span class="stat-value">{rust_lazy_queued_files}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Memory Indicator"</span>
                                            <span class="stat-value">{rust_memory_indicator_text}</span>
                                        </div>
                                        <div class="stat-item">
                                            <span class="stat-label">"Memory Level"</span>
                                            <span class="stat-value">{rust_memory_indicator_level}</span>
                                        </div>
                                    </div>
                                    <div class="setting-help">{rust_progress_status}</div>
                                </div>

                                <div class="workflow-section">
                                    <h4 class="collapsible-header error-log-header">
                                        "Error Details"
                                        <span class="collapse-icon">"▼"</span>
                                    </h4>
                                    <div class="collapsible-content error-log-panel">
                                        <div class="error-log" id="errorLog">
                                            {move || {
                                                let errors = batch_error_logs.get();
                                                if errors.is_empty() {
                                                    view! { <div class="log-entry">"No errors detected"</div> }
                                                        .into_any()
                                                } else {
                                                    errors
                                                        .into_iter()
                                                        .map(|entry| view! { <div class="log-entry">{entry}</div> })
                                                        .collect_view()
                                                        .into_any()
                                                }
                                            }}
                                        </div>
                                        <div class="error-actions">
                                            <button type="button" id="clearErrorsBtn" class="reset-button">"Clear Errors"</button>
                                            <button type="button" id="exportErrorsBtn" class="export-btn">"Export Error Log"</button>
                                        </div>
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
                        <BatchImageGalleryPanel
                            state=batch_state
                            preview_urls=batch_preview_urls
                            busy=batch_busy
                        />

                        <div class="canvas-container hidden workspace-card" id="canvasContainer">
                            <h3 id="canvasTitle">"Current Image Preview"</h3>
                            <div class="canvas-wrapper" id="canvasWrapper">
                                <div class="canvas-panel" id="originalPanel">
                                    <h4 class="panel-title">"Original"</h4>
                                    <div class="panel-content">
                                        <canvas id="inputCanvas"></canvas>
                                        <div id="faceOverlays" class="face-overlays"></div>
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
                                    <span>"Detected faces: " <span id="faceCount">"0"</span></span>
                                    <span class="separator">"•"</span>
                                    <span>"Selected: " <span id="selectedFaceCount">"0"</span></span>
                                </div>
                                <div class="face-selection-controls">
                                    <button type="button" id="selectAllFacesBtn" class="small-btn">"Select All Faces"</button>
                                    <button type="button" id="selectNoneFacesBtn" class="small-btn">"Select None"</button>
                                    <button type="button" id="detectFacesBtn" class="small-btn primary">"Detect Faces"</button>
                                </div>
                            </div>
                        </div>

                        <div class="cropped-faces workspace-card" id="croppedFaces">
                            <h3>"Cropped Faces:"</h3>
                            <div id="croppedContainer">
                                {move || {
                                    let urls = batch_preview_urls.get();
                                    let mut processed_ids = batch_state
                                        .get()
                                        .images
                                        .values()
                                        .filter(|img| img.processed)
                                        .map(|img| img.id.clone())
                                        .collect::<Vec<_>>();
                                    processed_ids.sort();
                                    processed_ids
                                        .into_iter()
                                        .map(|id| {
                                            let label = batch_file_label(&id).to_string();
                                            let url = urls.get(&id).cloned();
                                            view! {
                                                <div class="cropped-face">
                                                    {url.map(|src| {
                                                        view! {
                                                            <img src=src alt="Processed preview" />
                                                        }
                                                    })}
                                                    <div class="setting-help">{format!("Processed: {label}")}</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </div>

                        <div class="status workspace-card" id="status">"Ready to load image"</div>
                    </div>
                </main>
            </div>

        </div>
    }
}

#[component]
fn SinglePage() -> impl IntoView {
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
    let browser_capabilities = Signal::derive(move || detect_browser_capabilities());
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
        revalidate_browser_fallbacks(true, true, MediaPipeAssetPaths::default())
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
                        dimensions.set((width as f64, height as f64));
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
                        <button type="button" id="backToMultipleBtn" class="ghost-btn" title="Switch to multiple image mode">
                            <span>"Multiple Images"</span>
                        </button>
                        <button type="button" id="csvBatchModeBtn" class="ghost-btn" title="Switch to CSV batch mode">
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
                            single_state.update(|s| s.open_webcam_modal());
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
                                                            return view! {}.into_any();
                                                        }
                                                        let selected_ids = single_state.get().selected_face_ids;
                                                        detected_faces
                                                            .get()
                                                            .into_iter()
                                                            .map(|face| {
                                                                let is_selected = selected_ids.contains(&face.id);
                                                                let (left, top, width, height) = overlay_percent_rect(
                                                                    &face,
                                                                    source_width,
                                                                    source_height,
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
                                                    single_state.update(|s| s.select_all_faces());
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
                                                    single_state.update(|s| s.select_none_faces());
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
                                                        .cloned()
                                                        .map(|face_id| {
                                                            let is_selected = state.selected_face_ids.contains(&face_id);
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
                                                        worker_for_detect.update(|w| w.mark_request_started());
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
                                                                    worker_for_detect.update(|w| w.mark_request_succeeded());
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
                                                    single_state.update(|s| s.close_webcam_modal());
                                                    single_runtime.update(|r| r.reset());
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
                                single_state.update(|s| s.close_webcam_modal());
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
                                                worker_for_capture.update(|w| w.mark_request_started());

                                                if let Some(stream) = stream_for_capture.get() {
                                                    stop_media_stream(&stream);
                                                    stream_for_capture.set(None);
                                                }
                                                clear_video_source("webcamVideo");
                                                state_for_capture.update(|s| s.close_webcam_modal());

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
                                                        worker_for_capture.update(|w| w.mark_request_succeeded());
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
                                    single_state.update(|s| s.switch_camera());
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

#[component]
fn PanelsGalleryPage() -> impl IntoView {
    view! {
        <AppShell title="Leptos Panels Gallery">
            <div class="app-shell" style="padding-top: 24px;">
                <div class="app-body">
                    <aside class="control-panel">
                        <div class="control-scroll">
                            <CropSettingsPanel />
                            <PreprocessingSettingsPanel />
                            <OutputSettingsBatchPanel />
                            <OutputSettingsCsvPanel />
                        </div>
                    </aside>
                </div>
            </div>
        </AppShell>
    }
}

#[component]
fn LandingPage() -> impl IntoView {
    view! {
        <AppShell title="Face Crop Forge">
            <div class="home-container">
                <div class="background-blobs">
                    <div class="blob blob-1"></div>
                    <div class="blob blob-2"></div>
                </div>

                <header class="hero-section">
                    <div class="hero-badge">"v1.7.0 • 100% Client-Side Privacy"</div>
                    <h1 class="hero-title">
                        "Face Crop "
                        <span class="gradient-text">"Forge"</span>
                    </h1>
                    <p class="hero-subtitle">
                        "The ultimate tool for batch face detection and cropping. Process thousands of images securely in your browser without uploading a single file."
                    </p>
                    <div class="hero-stats">
                        <div class="stat">
                            <span class="stat-number">"0s"</span>
                            <span class="stat-label">"Upload Time"</span>
                        </div>
                        <div class="stat-divider"></div>
                        <div class="stat">
                            <span class="stat-number">"100%"</span>
                            <span class="stat-label">"Private"</span>
                        </div>
                        <div class="stat-divider"></div>
                        <div class="stat">
                            <span class="stat-number">"Free"</span>
                            <span class="stat-label">"Open Source"</span>
                        </div>
                    </div>
                </header>

                <main class="modes-grid">
                    <ModeCard
                        href="/batch"
                        title="Batch Processing"
                        description="The powerhouse mode. Drag and drop folders of images and process them all at once."
                        action="Launch Batch Mode"
                        featured=true
                        icon_path="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                        feature_1="Bulk processing"
                        feature_2="ZIP export"
                        feature_3="Smart centering"
                    />
                    <ModeCard
                        href="/single"
                        title="Single Image"
                        description="Perfect for fine-tuning. Adjust crops, rotate, and inspect detections one by one."
                        action="Launch Single Mode"
                        featured=false
                        icon_path="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
                        feature_1="Real-time preview"
                        feature_2="Manual adjustments"
                        feature_3="Webcam support"
                    />
                    <ModeCard
                        href="/csv"
                        title="CSV Workflow"
                        description="Enterprise workflow. Map CSV data to filenames for automated organization."
                        action="Launch CSV Mode"
                        featured=false
                        icon_path="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                        feature_1="Data mapping"
                        feature_2="Custom naming"
                        feature_3="Bulk organization"
                    />
                </main>

                <footer class="footer">
                    <div class="footer-content">
                        <p>
                            "Built with "
                            <a href="https://ai.google.dev/edge/mediapipe/solutions/guide" target="_blank" class="footer-link">
                                "MediaPipe Vision"
                            </a>
                        </p>
                        <div class="footer-links">
                            <a href="https://github.com/gregorycarnegie/face-crop-forge" target="_blank" rel="noopener">
                                "GitHub"
                            </a>
                        </div>
                    </div>
                </footer>
            </div>
        </AppShell>
    }
}

#[component]
fn ModeCard(
    href: &'static str,
    title: &'static str,
    description: &'static str,
    action: &'static str,
    featured: bool,
    icon_path: &'static str,
    feature_1: &'static str,
    feature_2: &'static str,
    feature_3: &'static str,
) -> impl IntoView {
    let class_name = if featured {
        "mode-card featured"
    } else {
        "mode-card"
    };
    let click_title = title;

    view! {
        <a
            href=href
            class=class_name
            on:click=move |_| {
                leptos::logging::log!("User selected: {} mode", click_title);
            }
        >
            <div class="card-glow"></div>
            <div class="mode-icon-wrapper">
                <svg class="mode-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=icon_path></path>
                </svg>
            </div>
            <h2 class="mode-title">{title}</h2>
            <p class="mode-description">{description}</p>
            <ul class="mode-features">
                <li>{feature_1}</li>
                <li>{feature_2}</li>
                <li>{feature_3}</li>
            </ul>
            <span class="mode-action">
                {action}
                <span class="arrow">"->"</span>
            </span>
        </a>
    }
}

use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn load_saved_settings_map() -> HashMap<String, ProcessingSettings> {
    let Some(storage) = window().local_storage().ok().flatten() else {
        return HashMap::new();
    };
    let Some(raw) = storage.get_item(SAVED_SETTINGS_KEY).ok().flatten() else {
        return HashMap::new();
    };
    serde_json::from_str::<HashMap<String, ProcessingSettings>>(&raw).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load_saved_settings_map() -> HashMap<String, ProcessingSettings> {
    HashMap::new()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn persist_saved_settings_map(
    map: &HashMap<String, ProcessingSettings>,
) -> Result<(), String> {
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
pub(super) fn persist_saved_settings_map(
    _map: &HashMap<String, ProcessingSettings>,
) -> Result<(), String> {
    Err("Settings persistence is only available on wasm32".to_string())
}

pub(super) fn list_saved_setting_names() -> Vec<String> {
    let mut names = load_saved_settings_map()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    names.sort();
    names
}

pub(super) fn save_named_processing_settings(
    name: &str,
    settings: &ProcessingSettings,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Settings name cannot be empty".to_string());
    }
    let mut map = load_saved_settings_map();
    map.insert(trimmed.to_string(), settings.clone());
    persist_saved_settings_map(&map)
}

pub(super) fn load_named_processing_settings(name: &str) -> Option<ProcessingSettings> {
    let mut map = load_saved_settings_map();
    map.remove(name)
}

pub(super) fn export_saved_settings_json() -> Result<String, String> {
    serde_json::to_string_pretty(&load_saved_settings_map())
        .map_err(|err| format!("Serialize failed: {err}"))
}

pub(super) fn import_saved_settings_json(json: &str) -> Result<usize, String> {
    let parsed = serde_json::from_str::<HashMap<String, ProcessingSettings>>(json)
        .map_err(|err| format!("Invalid settings JSON: {err}"))?;
    let count = parsed.len();
    persist_saved_settings_map(&parsed)?;
    Ok(count)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn click_element_by_id(id: &str) {
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
pub(super) fn click_element_by_id(_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub(super) fn object_url_for_file(file: &web_sys::File) -> Option<String> {
    web_sys::Url::create_object_url_with_blob(file).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn object_url_for_file(_file: &web_sys::File) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub(super) fn revoke_object_url(url: &str) {
    let _ = web_sys::Url::revoke_object_url(url);
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn revoke_object_url(_url: &str) {}

#[cfg(target_arch = "wasm32")]
pub(super) async fn draw_source_image_to_canvas(
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
pub(super) async fn draw_source_image_to_canvas(
    _canvas_id: &str,
    _source_url: &str,
) -> Result<(u32, u32), String> {
    Err("Canvas drawing is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn clear_canvas(canvas_id: &str) {
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
pub(super) fn clear_canvas(_canvas_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub(super) async fn decode_image_dimensions(file: &web_sys::File) -> Result<Dimensions, String> {
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
pub(super) async fn decode_image_dimensions(_file: &web_sys::File) -> Result<Dimensions, String> {
    Err("Image decode is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn crop_face_bytes_from_source(
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
pub(super) async fn crop_face_bytes_from_source(
    _source_url: &str,
    _face: &DetectedFace,
    _settings: &ProcessingSettings,
    _mime_type: &str,
) -> Result<Vec<u8>, String> {
    Err("Face crop export is only available on wasm32".to_string())
}

pub(super) fn apply_detection_quality_filters(
    mut faces: Vec<DetectedFace>,
    settings: &ProcessingSettings,
) -> Vec<DetectedFace> {
    faces.retain(|face| face.confidence as f32 >= settings.min_confidence);
    faces
}

pub(super) fn overlay_percent_rect(
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

pub(super) fn render_naming_template(
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
pub(super) async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
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
pub(super) async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
    Err("Webcam listing is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn stop_media_stream(stream: &web_sys::MediaStream) {
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
pub(super) fn stop_media_stream(_stream: &web_sys::MediaStream) {}

#[cfg(target_arch = "wasm32")]
pub(super) fn clear_video_source(video_id: &str) {
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
pub(super) fn clear_video_source(_video_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub(super) async fn start_webcam_stream(
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
pub(super) async fn start_webcam_stream(
    _video_id: &str,
    _preferred_device_id: Option<&str>,
) -> Result<web_sys::MediaStream, String> {
    Err("Webcam streaming is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn capture_webcam_frame_to_file(
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
pub(super) async fn capture_webcam_frame_to_file(
    _video_id: &str,
    _canvas_id: &str,
) -> Result<web_sys::File, String> {
    Err("Webcam capture is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
const THEME_STORAGE_KEY: &str = "fcf.theme";

#[cfg(target_arch = "wasm32")]
pub(super) fn load_theme_mode() -> ThemeMode {
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
pub(super) fn load_theme_mode() -> ThemeMode {
    ThemeMode::Dark
}

#[cfg(target_arch = "wasm32")]
pub(super) fn persist_theme_mode(mode: ThemeMode) {
    if let Ok(Some(storage)) = window().local_storage() {
        let value = match mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        };
        let _ = storage.set_item(THEME_STORAGE_KEY, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn persist_theme_mode(_mode: ThemeMode) {}

#[cfg(target_arch = "wasm32")]
pub(super) fn apply_theme_mode(mode: ThemeMode) {
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
pub(super) fn apply_theme_mode(_mode: ThemeMode) {}

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

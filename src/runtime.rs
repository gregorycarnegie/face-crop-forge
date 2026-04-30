use crate::base_runtime::Dimensions;
use crate::state::ProcessingSettings;
use crate::worker_bridge::DetectedFace;
use std::collections::HashMap;

pub const MAX_DETECTION_SIDE: u32 = 1600;

/// Scale detected face coordinates from detection-image space back to original-image space.
/// Call this after detecting on a downscaled image with `scale = detection_size / original_size`.
/// Pass `1.0 / scale` as both `scale_x` and `scale_y`.
pub fn scale_detected_faces(mut faces: Vec<DetectedFace>, scale_x: f64, scale_y: f64) -> Vec<DetectedFace> {
    if (scale_x - 1.0).abs() < 1e-9 && (scale_y - 1.0).abs() < 1e-9 {
        return faces;
    }
    for face in &mut faces {
        face.x *= scale_x;
        face.y *= scale_y;
        face.width *= scale_x;
        face.height *= scale_y;
    }
    faces
}

#[derive(Clone)]
pub struct ProcessedImageOutput {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub preview_url: String,
}

#[cfg(target_arch = "wasm32")]
type BlobClosureSlot = std::rc::Rc<
    std::cell::RefCell<Option<wasm_bindgen::closure::Closure<dyn FnMut(Option<web_sys::Blob>)>>>,
>;

pub fn batch_file_label(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

pub fn make_file_id(file: &web_sys::File, index: usize) -> String {
    format!(
        "{}::{}::{}::{}",
        file.name(),
        file.size(),
        file.last_modified(),
        index
    )
}

pub fn is_probably_image_file(file: &web_sys::File) -> bool {
    let mime = file.type_().to_lowercase();
    if mime.starts_with("image/") {
        return true;
    }

    let name = file.name().to_lowercase();
    [
        ".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp", ".avif", ".tif", ".tiff",
    ]
    .iter()
    .any(|ext| name.ends_with(ext))
}

pub fn mime_type_for_output_format(output_format: &str) -> &'static str {
    match output_format {
        "jpeg" | "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

#[cfg(target_arch = "wasm32")]
pub fn object_url_for_file(file: &web_sys::File) -> Option<String> {
    web_sys::Url::create_object_url_with_blob(file).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn object_url_for_file(_file: &web_sys::File) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn object_url_for_bytes(bytes: &[u8], mime_type: &str) -> Result<String, String> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let blob_options = web_sys::BlobPropertyBag::new();
    blob_options.set_type(mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options)
        .map_err(|err| format!("Failed to build preview blob: {err:?}"))?;
    web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|err| format!("Failed to create crop preview URL: {err:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn object_url_for_bytes(_bytes: &[u8], _mime_type: &str) -> Result<String, String> {
    Err("Blob preview URLs are only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn revoke_object_url(url: &str) {
    let _ = web_sys::Url::revoke_object_url(url);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn revoke_object_url(_url: &str) {}

pub fn revoke_preview_urls(urls: &HashMap<String, String>) {
    for url in urls.values() {
        if url.starts_with("blob:") {
            revoke_object_url(url);
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> u64 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        js_sys::Date::now() as u64
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

pub fn elapsed_ms_since(start_ms: u64) -> u64 {
    now_ms().saturating_sub(start_ms)
}

#[cfg(target_arch = "wasm32")]
pub async fn files_from_data_transfer(
    data: web_sys::DataTransfer,
) -> Result<Vec<web_sys::File>, String> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let extractor = js_sys::Function::new_with_args(
        "dt",
        r"
        const fromList = (list) => {
          const out = [];
          for (let i = 0; i < list.length; i++) {
            const file = list[i];
            if (file) out.push(file);
          }
          return out;
        };

        const walkHandle = async (handle) => {
          if (!handle) return [];
          if (handle.kind === 'file') {
            try {
              const file = await handle.getFile();
              return file ? [file] : [];
            } catch {
              return [];
            }
          }
          if (handle.kind !== 'directory') return [];
          const out = [];
          try {
            for await (const entry of handle.values()) {
              const nested = await walkHandle(entry);
              out.push(...nested);
            }
          } catch {
          }
          return out;
        };

        const walkEntry = (entry) => new Promise((resolve) => {
          if (!entry) return resolve([]);
          if (entry.isFile) {
            entry.file((file) => resolve(file ? [file] : []), () => resolve([]));
            return;
          }
          if (!entry.isDirectory) return resolve([]);
          const reader = entry.createReader();
          const out = [];
          const pump = () => {
            reader.readEntries(async (entries) => {
              if (!entries || entries.length === 0) return resolve(out);
              for (const child of entries) {
                const nested = await walkEntry(child);
                out.push(...nested);
              }
              pump();
            }, () => resolve(out));
          };
          pump();
        });

        return (async () => {
          const items = dt?.items ? Array.from(dt.items) : [];

          const supportsHandles = items.some((item) => item && typeof item.getAsFileSystemHandle === 'function');
          if (supportsHandles) {
            const handles = [];
            for (const item of items) {
              if (!item || typeof item.getAsFileSystemHandle !== 'function') continue;
              try {
                const handle = await item.getAsFileSystemHandle();
                if (handle) handles.push(handle);
              } catch {
              }
            }
            if (handles.length > 0) {
              const nested = await Promise.all(handles.map(walkHandle));
              const files = nested.flat();
              if (files.length > 0) return files;
            }
          }

          const entries = items
            .map((item) => item && typeof item.webkitGetAsEntry === 'function'
              ? item.webkitGetAsEntry()
              : null)
            .filter(Boolean);
          if (entries.length > 0) {
            const nested = await Promise.all(entries.map(walkEntry));
            const files = nested.flat();
            if (files.length > 0) return files;
          }

          if (dt?.files?.length) return fromList(dt.files);

          return items
            .map((item) => item && typeof item.getAsFile === 'function' ? item.getAsFile() : null)
            .filter(Boolean);
        })();
        ",
    );

    let promise_value = extractor
        .call1(&JsValue::NULL, data.as_ref())
        .map_err(|err| format!("DataTransfer extraction failed: {err:?}"))?;
    let js_files = JsFuture::from(js_sys::Promise::from(promise_value))
        .await
        .map_err(|err| format!("DataTransfer extraction rejected: {err:?}"))?;
    let arr = js_sys::Array::from(&js_files);
    let mut files = Vec::with_capacity(arr.length() as usize);
    for idx in 0..arr.length() {
        if let Ok(file) = arr.get(idx).dyn_into::<web_sys::File>() {
            files.push(file);
        }
    }
    Ok(files)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn files_from_data_transfer(
    _data: web_sys::DataTransfer,
) -> Result<Vec<web_sys::File>, String> {
    Ok(Vec::new())
}

/// Returns `(file_for_detection, scale)` where `scale < 1.0` means the returned file is a
/// downscaled JPEG. Multiply detected face coords by `1.0 / scale` to recover original-image
/// coordinates. Returns the original file unchanged when the image already fits within `max_side`.
#[cfg(target_arch = "wasm32")]
pub async fn maybe_downscale_for_detection(
    file: &web_sys::File,
    dims: Dimensions,
    max_side: u32,
) -> Result<(web_sys::File, f64), String> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let longest = dims.width.max(dims.height);
    if longest <= max_side {
        return Ok((file.clone(), 1.0));
    }

    let scale = f64::from(max_side) / f64::from(longest);
    let target_w = ((f64::from(dims.width) * scale).round() as u32).max(1);
    let target_h = ((f64::from(dims.height) * scale).round() as u32).max(1);

    let object_url = web_sys::Url::create_object_url_with_blob(file)
        .map_err(|err| format!("Downscale: create_object_url failed: {err:?}"))?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(&object_url);
    let decode_result = JsFuture::from(image.decode()).await;
    let _ = web_sys::Url::revoke_object_url(&object_url);
    decode_result.map_err(|err| format!("Downscale: image decode failed: {err:?}"))?;

    let document = leptos::prelude::window()
        .document()
        .ok_or_else(|| "Document unavailable".to_string())?;
    let canvas = document
        .create_element("canvas")
        .map_err(|err| format!("{err:?}"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Failed to create detection canvas".to_string())?;
    canvas.set_width(target_w);
    canvas.set_height(target_h);
    let context = canvas
        .get_context("2d")
        .map_err(|err| format!("{err:?}"))?
        .ok_or_else(|| "2d context unavailable".to_string())?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast detection canvas context".to_string())?;
    context
        .draw_image_with_html_image_element_and_dw_and_dh(
            &image,
            0.0,
            0.0,
            f64::from(target_w),
            f64::from(target_h),
        )
        .map_err(|err| format!("Downscale: draw failed: {err:?}"))?;

    let canvas_clone = canvas.clone();
    let blob_promise = js_sys::Promise::new(&mut move |resolve, reject| {
        let resolve_fn = resolve.clone();
        let reject_fn = reject.clone();
        let slot: BlobClosureSlot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let slot_for_cb: BlobClosureSlot = std::rc::Rc::clone(&slot);
        let callback = Closure::new(move |blob: Option<web_sys::Blob>| {
            if let Some(blob) = blob {
                let _ = resolve_fn.call1(&JsValue::NULL, &blob);
            } else {
                let _ = reject_fn.call1(
                    &JsValue::NULL,
                    &JsValue::from_str("Detection canvas to_blob produced no data"),
                );
            }
            slot_for_cb.borrow_mut().take();
        });
        match canvas_clone.to_blob_with_type(callback.as_ref().unchecked_ref(), "image/jpeg") {
            Ok(()) => {
                *slot.borrow_mut() = Some(callback);
            }
            Err(err) => {
                let _ = reject.call1(
                    &JsValue::NULL,
                    &JsValue::from(format!("Detection canvas to_blob failed: {err:?}")),
                );
                slot.borrow_mut().take();
            }
        }
    });

    let blob_js = JsFuture::from(blob_promise)
        .await
        .map_err(|err| format!("Downscale: blob promise failed: {err:?}"))?;
    let blob = blob_js
        .dyn_into::<web_sys::Blob>()
        .map_err(|_| "Failed to cast downscaled detection blob".to_string())?;

    let parts = js_sys::Array::new();
    parts.push(&blob);
    let file_options = web_sys::FilePropertyBag::new();
    file_options.set_type("image/jpeg");
    let detection_file = web_sys::File::new_with_blob_sequence_and_options(
        &parts,
        "detect.jpg",
        &file_options,
    )
    .map_err(|err| format!("Failed to create detection file: {err:?}"))?;

    Ok((detection_file, scale))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn maybe_downscale_for_detection(
    file: &web_sys::File,
    _dims: Dimensions,
    _max_side: u32,
) -> Result<(web_sys::File, f64), String> {
    Ok((file.clone(), 1.0))
}

#[cfg(target_arch = "wasm32")]
pub async fn decode_image_dimensions(file: &web_sys::File) -> Result<Dimensions, String> {
    use wasm_bindgen_futures::JsFuture;

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
pub async fn decode_image_dimensions(_file: &web_sys::File) -> Result<Dimensions, String> {
    Err("Image decode is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn crop_face_bytes_from_source(
    source_url: &str,
    face: &DetectedFace,
    settings: &ProcessingSettings,
    mime_type: &str,
) -> Result<Vec<u8>, String> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let document = leptos::prelude::window()
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

    let source_rect = compute_source_crop_rect(
        face,
        f64::from(image.natural_width()),
        f64::from(image.natural_height()),
        settings,
    );
    context
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &image,
            source_rect.0,
            source_rect.1,
            source_rect.2,
            source_rect.3,
            0.0,
            0.0,
            f64::from(crop_w),
            f64::from(crop_h),
        )
        .map_err(|err| format!("Crop draw failed: {err:?}"))?;

    let mime = mime_type.to_string();
    let canvas_clone = canvas.clone();
    let blob_promise = js_sys::Promise::new(&mut move |resolve, reject| {
        let resolve_fn = resolve.clone();
        let reject_fn = reject.clone();
        let slot: BlobClosureSlot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let slot_for_cb: BlobClosureSlot = std::rc::Rc::clone(&slot);
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
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crop_face_bytes_from_source(
    _source_url: &str,
    _face: &DetectedFace,
    _settings: &ProcessingSettings,
    _mime_type: &str,
) -> Result<Vec<u8>, String> {
    Err("Face crop export is only available on wasm32".to_string())
}

pub fn apply_detection_quality_filters(
    mut faces: Vec<DetectedFace>,
    settings: &ProcessingSettings,
) -> Vec<DetectedFace> {
    faces.retain(|face| face.confidence >= f64::from(settings.min_confidence));
    faces
}

pub fn overlay_percent_crop_rect(
    face: &DetectedFace,
    source_width: f64,
    source_height: f64,
    settings: &ProcessingSettings,
) -> (f64, f64, f64, f64) {
    let (x, y, width, height) =
        compute_source_crop_rect(face, source_width, source_height, settings);
    let left = (x / source_width * 100.0).clamp(0.0, 100.0);
    let top = (y / source_height * 100.0).clamp(0.0, 100.0);
    let w = (width / source_width * 100.0).clamp(0.0, 100.0);
    let h = (height / source_height * 100.0).clamp(0.0, 100.0);
    (left, top, w, h)
}

fn normalize_face_box(
    face: &DetectedFace,
    source_width: f64,
    source_height: f64,
) -> (f64, f64, f64, f64) {
    let mut x = face.x;
    let mut y = face.y;
    let mut width = face.width;
    let mut height = face.height;
    if width <= 1.0 && height <= 1.0 && x <= 1.0 && y <= 1.0 {
        x *= source_width;
        y *= source_height;
        width *= source_width;
        height *= source_height;
    }
    (x, y, width.max(1.0), height.max(1.0))
}

fn compute_source_crop_rect(
    face: &DetectedFace,
    source_width: f64,
    source_height: f64,
    settings: &ProcessingSettings,
) -> (f64, f64, f64, f64) {
    let (face_x, face_y, face_w, face_h) = normalize_face_box(face, source_width, source_height);
    let target_ratio =
        f64::from(settings.output_width.max(1)) / f64::from(settings.output_height.max(1));
    let face_height_ratio = (f64::from(settings.face_height_pct) / 100.0).clamp(0.10, 1.0);

    let mut crop_h = (face_h / face_height_ratio).max(face_h);
    let mut crop_w = (crop_h * target_ratio).max(face_w);
    crop_h = crop_w / target_ratio;
    if crop_h < face_h {
        crop_h = face_h;
        crop_w = crop_h * target_ratio;
    }

    if crop_w > source_width || crop_h > source_height {
        let scale = (source_width / crop_w)
            .min(source_height / crop_h)
            .max(0.0001);
        crop_w *= scale;
        crop_h *= scale;
    }

    let center_x =
        face_x + face_w / 2.0 + (f64::from(settings.horizontal_offset_pct) / 100.0) * face_w;
    let center_y =
        face_y + face_h / 2.0 + (f64::from(settings.vertical_offset_pct) / 100.0) * face_h;
    let crop_x = (center_x - crop_w / 2.0).clamp(0.0, (source_width - crop_w).max(0.0));
    let crop_y = (center_y - crop_h / 2.0).clamp(0.0, (source_height - crop_h).max(0.0));

    (crop_x, crop_y, crop_w.max(1.0), crop_h.max(1.0))
}

#[cfg(target_arch = "wasm32")]
pub async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let media_devices = leptos::prelude::window()
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
    let devices = js_sys::Array::from(&devices_js);
    let mut video_inputs = Vec::new();
    for idx in 0..devices.length() {
        let raw = devices.get(idx);
        let kind = js_sys::Reflect::get(&raw, &wasm_bindgen::JsValue::from_str("kind"))
            .ok()
            .and_then(|value| value.as_string())
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
pub async fn list_video_input_devices() -> Result<Vec<(String, String)>, String> {
    Err("Webcam listing is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn stop_media_stream(stream: &web_sys::MediaStream) {
    use wasm_bindgen::JsCast;

    let tracks = js_sys::Array::from(&stream.get_tracks());
    for idx in 0..tracks.length() {
        if let Ok(track) = tracks.get(idx).dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stop_media_stream(_stream: &web_sys::MediaStream) {}

#[cfg(target_arch = "wasm32")]
pub fn clear_video_source(video_id: &str) {
    use wasm_bindgen::{JsCast, JsValue};

    let Some(document) = leptos::prelude::window().document() else {
        return;
    };
    let Some(element) = document.get_element_by_id(video_id) else {
        return;
    };
    let Ok(video) = element.dyn_into::<web_sys::HtmlVideoElement>() else {
        return;
    };
    let _ = js_sys::Reflect::set(
        video.as_ref(),
        &JsValue::from_str("srcObject"),
        &JsValue::NULL,
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_video_source(_video_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub async fn start_webcam_stream(
    video_id: &str,
    preferred_device_id: Option<&str>,
) -> Result<web_sys::MediaStream, String> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let media_devices = leptos::prelude::window()
        .navigator()
        .media_devices()
        .map_err(|err| format!("MediaDevices unavailable: {err:?}"))?;

    let constraints_obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &constraints_obj,
        &JsValue::from_str("audio"),
        &JsValue::FALSE,
    );

    let video_constraints = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &video_constraints,
        &JsValue::from_str("width"),
        &JsValue::from_f64(1920.0),
    );
    let _ = js_sys::Reflect::set(
        &video_constraints,
        &JsValue::from_str("height"),
        &JsValue::from_f64(1080.0),
    );
    if let Some(device_id) = preferred_device_id {
        let exact = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &exact,
            &JsValue::from_str("exact"),
            &JsValue::from_str(device_id),
        );
        let _ = js_sys::Reflect::set(&video_constraints, &JsValue::from_str("deviceId"), &exact);
    } else {
        let _ = js_sys::Reflect::set(
            &video_constraints,
            &JsValue::from_str("facingMode"),
            &JsValue::from_str("user"),
        );
    }
    let _ = js_sys::Reflect::set(
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

    let document = leptos::prelude::window()
        .document()
        .ok_or_else(|| "Document is unavailable".to_string())?;
    let video = document
        .get_element_by_id(video_id)
        .ok_or_else(|| format!("Video element #{video_id} not found"))?
        .dyn_into::<web_sys::HtmlVideoElement>()
        .map_err(|_| format!("Element #{video_id} is not a video"))?;
    video.set_muted(true);
    let _ = js_sys::Reflect::set(
        video.as_ref(),
        &JsValue::from_str("srcObject"),
        stream.as_ref(),
    );
    let _ = video.play();
    Ok(stream)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn start_webcam_stream(
    _video_id: &str,
    _preferred_device_id: Option<&str>,
) -> Result<web_sys::MediaStream, String> {
    Err("Webcam streaming is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn capture_webcam_frame_to_file(
    video_id: &str,
    canvas_id: &str,
) -> Result<web_sys::File, String> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let document = leptos::prelude::window()
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
    let blob_promise = js_sys::Promise::new(&mut move |resolve, reject| {
        let resolve_fn = resolve.clone();
        let reject_fn = reject.clone();
        let callback_slot: BlobClosureSlot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let callback_slot_for_cb: BlobClosureSlot = std::rc::Rc::clone(&callback_slot);
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
    let parts = js_sys::Array::new();
    parts.push(&blob);
    let file_options = web_sys::FilePropertyBag::new();
    file_options.set_type("image/png");
    web_sys::File::new_with_blob_sequence_and_options(&parts, "webcam-capture.png", &file_options)
        .map_err(|err| format!("Failed to build capture file: {err:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn capture_webcam_frame_to_file(
    _video_id: &str,
    _canvas_id: &str,
) -> Result<web_sys::File, String> {
    Err("Webcam capture is only available on wasm32".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ProcessingSettings;
    use crate::worker_bridge::DetectedFace;

    fn face(x: f64, y: f64, w: f64, h: f64) -> DetectedFace {
        DetectedFace {
            id: "face_1".to_string(),
            x,
            y,
            width: w,
            height: h,
            confidence: 1.0,
        }
    }

    // ── scale_detected_faces ──────────────────────────────────────────────────

    #[test]
    fn scale_detected_faces_multiplies_all_coords_uniformly() {
        let faces = vec![face(100.0, 80.0, 200.0, 250.0)];
        let scaled = scale_detected_faces(faces, 2.5, 2.5);
        assert!((scaled[0].x - 250.0).abs() < 1e-9);
        assert!((scaled[0].y - 200.0).abs() < 1e-9);
        assert!((scaled[0].width - 500.0).abs() < 1e-9);
        assert!((scaled[0].height - 625.0).abs() < 1e-9);
    }

    #[test]
    fn scale_detected_faces_is_noop_at_unity() {
        let faces = vec![face(100.0, 80.0, 200.0, 250.0)];
        let scaled = scale_detected_faces(faces.clone(), 1.0, 1.0);
        assert_eq!(scaled[0].x, faces[0].x);
        assert_eq!(scaled[0].width, faces[0].width);
    }

    #[test]
    fn scale_detected_faces_round_trips_with_inverse() {
        let original = vec![face(320.0, 240.0, 160.0, 200.0)];
        let downscale = 0.4;
        let detection_space = scale_detected_faces(original.clone(), downscale, downscale);
        let recovered = scale_detected_faces(detection_space, 1.0 / downscale, 1.0 / downscale);
        assert!((recovered[0].x - original[0].x).abs() < 1e-6, "x should round-trip");
        assert!((recovered[0].y - original[0].y).abs() < 1e-6, "y should round-trip");
        assert!((recovered[0].width - original[0].width).abs() < 1e-6, "width should round-trip");
        assert!((recovered[0].height - original[0].height).abs() < 1e-6, "height should round-trip");
    }

    #[test]
    fn scale_detected_faces_handles_empty_vec() {
        let scaled = scale_detected_faces(vec![], 2.0, 2.0);
        assert!(scaled.is_empty());
    }

    // ── batch_file_label ──────────────────────────────────────────────────────

    #[test]
    fn batch_file_label_extracts_filename_from_composite_id() {
        assert_eq!(batch_file_label("photo.jpg::1024::0::0"), "photo.jpg");
        assert_eq!(batch_file_label("a::b::c"), "a");
        assert_eq!(batch_file_label("plain_name"), "plain_name");
    }

    // ── mime_type_for_output_format ───────────────────────────────────────────

    #[test]
    fn mime_type_for_output_format_covers_all_variants() {
        assert_eq!(mime_type_for_output_format("jpeg"), "image/jpeg");
        assert_eq!(mime_type_for_output_format("jpg"), "image/jpeg");
        assert_eq!(mime_type_for_output_format("webp"), "image/webp");
        assert_eq!(mime_type_for_output_format("png"), "image/png");
        assert_eq!(mime_type_for_output_format("unknown"), "image/png");
    }

    // ── apply_detection_quality_filters ──────────────────────────────────────

    #[test]
    fn quality_filter_removes_faces_below_threshold() {
        let mut low = face(0.0, 0.0, 100.0, 100.0);
        low.confidence = 0.3;
        let mut high = face(50.0, 50.0, 80.0, 80.0);
        high.confidence = 0.9;
        let settings = ProcessingSettings {
            min_confidence: 0.5,
            ..ProcessingSettings::default()
        };
        let filtered = apply_detection_quality_filters(vec![low, high], &settings);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].confidence, 0.9);
    }

    #[test]
    fn quality_filter_keeps_face_exactly_at_threshold() {
        let mut at = face(0.0, 0.0, 100.0, 100.0);
        at.confidence = 0.5;
        let settings = ProcessingSettings {
            min_confidence: 0.5,
            ..ProcessingSettings::default()
        };
        let filtered = apply_detection_quality_filters(vec![at], &settings);
        assert_eq!(filtered.len(), 1);
    }

    // ── normalize_face_box ────────────────────────────────────────────────────

    #[test]
    fn normalize_face_box_scales_normalized_coords_to_pixels() {
        let f = face(0.5, 0.25, 0.2, 0.3);
        let (x, y, w, h) = normalize_face_box(&f, 1000.0, 800.0);
        assert!((x - 500.0).abs() < 0.01);
        assert!((y - 200.0).abs() < 0.01);
        assert!((w - 200.0).abs() < 0.01);
        assert!((h - 240.0).abs() < 0.01);
    }

    #[test]
    fn normalize_face_box_leaves_pixel_coords_unchanged() {
        let f = face(100.0, 200.0, 150.0, 180.0);
        let (x, y, w, h) = normalize_face_box(&f, 1000.0, 800.0);
        assert!((x - 100.0).abs() < 0.01);
        assert!((y - 200.0).abs() < 0.01);
        assert!((w - 150.0).abs() < 0.01);
        assert!((h - 180.0).abs() < 0.01);
    }

    // ── compute_source_crop_rect ──────────────────────────────────────────────

    #[test]
    fn crop_rect_is_centered_on_face_for_square_output() {
        let f = face(450.0, 350.0, 100.0, 100.0);
        let settings = ProcessingSettings {
            output_width: 256,
            output_height: 256,
            face_height_pct: 100,
            vertical_offset_pct: 0,
            horizontal_offset_pct: 0,
            ..ProcessingSettings::default()
        };
        let (x, y, w, h) = compute_source_crop_rect(&f, 1000.0, 800.0, &settings);
        assert!((w - h).abs() < 0.01, "crop must be square for 1:1 output");
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        assert!((cx - 500.0).abs() < 1.0, "crop must be centered on face x");
        assert!((cy - 400.0).abs() < 1.0, "crop must be centered on face y");
    }

    #[test]
    fn crop_rect_respects_output_aspect_ratio() {
        let f = face(400.0, 300.0, 100.0, 100.0);
        let settings = ProcessingSettings {
            output_width: 400,
            output_height: 200,
            face_height_pct: 100,
            ..ProcessingSettings::default()
        };
        let (_, _, w, h) = compute_source_crop_rect(&f, 1000.0, 800.0, &settings);
        assert!(
            (w / h - 2.0).abs() < 0.01,
            "crop ratio must be 2:1, got {}",
            w / h
        );
    }

    #[test]
    fn crop_rect_clamps_to_image_bounds() {
        let f = face(0.0, 0.0, 50.0, 50.0);
        let settings = ProcessingSettings {
            output_width: 256,
            output_height: 256,
            ..ProcessingSettings::default()
        };
        let (x, y, w, h) = compute_source_crop_rect(&f, 500.0, 500.0, &settings);
        assert!(x >= 0.0, "crop x must not go negative");
        assert!(y >= 0.0, "crop y must not go negative");
        assert!(x + w <= 500.001, "crop must not exceed image width");
        assert!(y + h <= 500.001, "crop must not exceed image height");
    }

    #[test]
    fn crop_rect_scales_down_when_face_exceeds_image() {
        let f = face(0.0, 0.0, 800.0, 800.0);
        let settings = ProcessingSettings {
            output_width: 256,
            output_height: 256,
            face_height_pct: 100,
            ..ProcessingSettings::default()
        };
        let (x, y, w, h) = compute_source_crop_rect(&f, 600.0, 600.0, &settings);
        assert!(x >= 0.0);
        assert!(y >= 0.0);
        assert!(w <= 600.001, "crop width must not exceed image width");
        assert!(h <= 600.001, "crop height must not exceed image height");
    }

    // ── overlay_percent_crop_rect ─────────────────────────────────────────────

    #[test]
    fn overlay_percent_crop_rect_values_are_within_0_to_100() {
        let f = face(200.0, 150.0, 100.0, 120.0);
        let settings = ProcessingSettings::default();
        let (left, top, w, h) = overlay_percent_crop_rect(&f, 1000.0, 800.0, &settings);
        for v in [left, top, w, h] {
            assert!(v >= 0.0 && v <= 100.0, "percent value {v} out of range");
        }
    }

    #[test]
    fn overlay_percent_crop_rect_center_matches_face_center_at_100pct_height() {
        let f = face(450.0, 350.0, 100.0, 100.0);
        let settings = ProcessingSettings {
            output_width: 256,
            output_height: 256,
            face_height_pct: 100,
            ..ProcessingSettings::default()
        };
        let (left, top, w, h) = overlay_percent_crop_rect(&f, 1000.0, 800.0, &settings);
        let cx = left + w / 2.0;
        let cy = top + h / 2.0;
        assert!(
            (cx - 50.0).abs() < 1.0,
            "overlay center x should be 50%, got {cx}"
        );
        assert!(
            (cy - 50.0).abs() < 1.0,
            "overlay center y should be 50%, got {cy}"
        );
    }

    // ── revoke_preview_urls ───────────────────────────────────────────────────

    #[test]
    fn revoke_preview_urls_does_not_panic_on_mixed_url_types() {
        let mut urls = std::collections::HashMap::new();
        urls.insert("a".to_string(), "blob:http://example.com/abc".to_string());
        urls.insert("b".to_string(), "data:image/png;base64,abc".to_string());
        urls.insert("c".to_string(), String::new());
        revoke_preview_urls(&urls);
    }
}

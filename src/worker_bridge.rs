#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetectionBackend {
    BrowserFaceDetector,
    MediaPipe,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static LAST_DETECTION_BACKEND: std::cell::RefCell<Option<DetectionBackend>> =
        const { std::cell::RefCell::new(None) };
}

pub fn last_detection_backend_label() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        LAST_DETECTION_BACKEND.with(|slot| match *slot.borrow() {
            Some(DetectionBackend::BrowserFaceDetector) => "Native FaceDetector",
            Some(DetectionBackend::MediaPipe) => "MediaPipe Tasks",
            None => "None",
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "Unavailable"
    }
}

pub fn clear_last_detection_backend() {
    #[cfg(target_arch = "wasm32")]
    LAST_DETECTION_BACKEND.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

#[cfg(target_arch = "wasm32")]
fn set_last_detection_backend(backend: DetectionBackend) {
    LAST_DETECTION_BACKEND.with(|slot| {
        *slot.borrow_mut() = Some(backend);
    });
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaceWorkerStatus {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Unsupported,
    Starting,
    Ready,
    Error,
    Stopped,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetectedFace {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub confidence: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceWorkerBridgeState {
    pub status: FaceWorkerStatus,
    pub last_error: Option<String>,
}

impl Default for FaceWorkerBridgeState {
    fn default() -> Self {
        Self {
            status: default_status(),
            last_error: None,
        }
    }
}

impl FaceWorkerBridgeState {
    pub fn status_label(&self) -> &'static str {
        match self.status {
            FaceWorkerStatus::Unsupported => "Unsupported",
            FaceWorkerStatus::Starting => "Starting",
            FaceWorkerStatus::Ready => "Ready",
            FaceWorkerStatus::Error => "Error",
            FaceWorkerStatus::Stopped => "Stopped",
        }
    }

    pub fn can_start(&self) -> bool {
        matches!(self.status, FaceWorkerStatus::Stopped)
    }

    pub fn can_stop(&self) -> bool {
        matches!(
            self.status,
            FaceWorkerStatus::Starting | FaceWorkerStatus::Ready | FaceWorkerStatus::Error
        )
    }

    pub fn mark_request_started(&mut self) {
        self.status = FaceWorkerStatus::Starting;
        self.last_error = None;
    }

    pub fn mark_request_succeeded(&mut self) {
        self.status = FaceWorkerStatus::Ready;
        self.last_error = None;
    }

    pub fn mark_request_failed(&mut self, error: impl Into<String>) {
        self.status = FaceWorkerStatus::Error;
        self.last_error = Some(error.into());
    }
}

pub fn start_face_worker(state: &mut FaceWorkerBridgeState) {
    state.last_error = None;
    if !state.can_start() {
        return;
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Runtime no longer depends on a JS worker script; detection runs through browser APIs.
        state.status = FaceWorkerStatus::Ready;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        state.status = FaceWorkerStatus::Unsupported;
        state.last_error = Some("Face detection runtime is only available on wasm32".to_string());
    }
}

pub fn stop_face_worker(state: &mut FaceWorkerBridgeState) {
    if !state.can_stop() {
        return;
    }

    state.status = FaceWorkerStatus::Stopped;
    state.last_error = None;
}

const fn default_status() -> FaceWorkerStatus {
    #[cfg(target_arch = "wasm32")]
    {
        FaceWorkerStatus::Stopped
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        FaceWorkerStatus::Unsupported
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn detect_faces_with_worker(
    _runtime: &str,
    file: web_sys::File,
) -> Result<Vec<DetectedFace>, String> {
    clear_last_detection_backend();
    match detect_faces_with_browser_api(file.clone()).await {
        Ok(faces) => {
            set_last_detection_backend(DetectionBackend::BrowserFaceDetector);
            Ok(faces)
        }
        Err(browser_error) => match detect_faces_with_mediapipe(file).await {
            Ok(faces) => {
                set_last_detection_backend(DetectionBackend::MediaPipe);
                Ok(faces)
            }
            Err(mediapipe_error) => Err(format!(
                "Browser FaceDetector failed ({browser_error}); MediaPipe fallback failed ({mediapipe_error})"
            )),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn detect_faces_with_worker(
    _runtime: &str,
    _file: web_sys::File,
) -> Result<Vec<DetectedFace>, String> {
    Err("Face detection is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn detect_faces_with_browser_api(file: web_sys::File) -> Result<Vec<DetectedFace>, String> {
    use web_sys::js_sys::{Array, Function, Object, Promise, Reflect};
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;

    let face_detector_ctor = Reflect::get(
        &web_sys::js_sys::global(),
        &JsValue::from_str("FaceDetector"),
    )
    .map_err(|err| format!("FaceDetector lookup failed: {err:?}"))?;
    if face_detector_ctor.is_null() || face_detector_ctor.is_undefined() {
        return Err("FaceDetector API unavailable in this browser".to_string());
    }
    let ctor_fn = face_detector_ctor
        .dyn_into::<Function>()
        .map_err(|_| "FaceDetector constructor is not callable".to_string())?;

    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("fastMode"),
        &JsValue::from_bool(true),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("maxDetectedFaces"),
        &JsValue::from_f64(32.0),
    );
    let ctor_args = Array::new();
    ctor_args.push(&options);
    let detector = Reflect::construct(&ctor_fn, &ctor_args).map_err(|err| format!("{err:?}"))?;

    let object_url = web_sys::Url::create_object_url_with_blob(&file)
        .map_err(|err| format!("Failed to create object URL: {err:?}"))?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(&object_url);
    let decode_result = wasm_bindgen_futures::JsFuture::from(image.decode()).await;
    let _ = web_sys::Url::revoke_object_url(&object_url);
    decode_result.map_err(|err| format!("Image decode failed: {err:?}"))?;

    let detect_fn = Reflect::get(&detector, &JsValue::from_str("detect"))
        .map_err(|err| format!("FaceDetector.detect lookup failed: {err:?}"))?
        .dyn_into::<Function>()
        .map_err(|_| "FaceDetector.detect is not callable".to_string())?;
    let detect_result = detect_fn
        .call1(&detector, &image)
        .map_err(|err| format!("FaceDetector.detect call failed: {err:?}"))?;
    let detections_js = wasm_bindgen_futures::JsFuture::from(Promise::from(detect_result))
        .await
        .map_err(|err| format!("FaceDetector.detect rejected: {err:?}"))?;
    let detections = Array::from(&detections_js);

    let mut faces = Vec::with_capacity(detections.length() as usize);
    for idx in 0..detections.length() {
        let detection = detections.get(idx);
        let bbox =
            Reflect::get(&detection, &JsValue::from_str("boundingBox")).unwrap_or(JsValue::NULL);
        let x = Reflect::get(&bbox, &JsValue::from_str("x"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let y = Reflect::get(&bbox, &JsValue::from_str("y"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let width = Reflect::get(&bbox, &JsValue::from_str("width"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let height = Reflect::get(&bbox, &JsValue::from_str("height"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let confidence = Reflect::get(&detection, &JsValue::from_str("confidence"))
            .ok()
            .and_then(|v| {
                if let Some(n) = v.as_f64() {
                    Some(n)
                } else {
                    let arr = Array::from(&v);
                    arr.get(0).as_f64()
                }
            })
            .unwrap_or(1.0);
        faces.push(DetectedFace {
            id: format!("face_{}", idx + 1),
            x,
            y,
            width,
            height,
            confidence,
        });
    }
    Ok(faces)
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_lines)]
async fn detect_faces_with_mediapipe(file: web_sys::File) -> Result<Vec<DetectedFace>, String> {
    use web_sys::js_sys::{Array, Function, Object, Reflect};
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;

    let assets = crate::mediapipe::MediaPipeAssetPaths::default();
    let module = import_js_module(&assets.vision_bundle_url).await?;

    let fileset_resolver = Reflect::get(&module, &JsValue::from_str("FilesetResolver"))
        .map_err(|err| format!("MediaPipe FilesetResolver lookup failed: {err:?}"))?;
    let fileset_resolver = fileset_resolver
        .dyn_into::<Object>()
        .map_err(|_| "MediaPipe FilesetResolver export is not an object".to_string())?;
    let for_vision_tasks = Reflect::get(
        fileset_resolver.as_ref(),
        &JsValue::from_str("forVisionTasks"),
    )
    .map_err(|err| format!("MediaPipe forVisionTasks lookup failed: {err:?}"))?
    .dyn_into::<Function>()
    .map_err(|_| "MediaPipe forVisionTasks is not callable".to_string())?;
    let vision = resolve_js_value(
        for_vision_tasks
            .call1(
                fileset_resolver.as_ref(),
                &JsValue::from_str(&assets.wasm_root),
            )
            .map_err(|err| format!("MediaPipe forVisionTasks call failed: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("MediaPipe FilesetResolver init failed: {err}"))?;

    let face_detector = Reflect::get(&module, &JsValue::from_str("FaceDetector"))
        .map_err(|err| format!("MediaPipe FaceDetector export lookup failed: {err:?}"))?;
    let face_detector = face_detector
        .dyn_into::<Object>()
        .map_err(|_| "MediaPipe FaceDetector export is not an object".to_string())?;
    let create_from_options = Reflect::get(
        face_detector.as_ref(),
        &JsValue::from_str("createFromOptions"),
    )
    .map_err(|err| format!("MediaPipe createFromOptions lookup failed: {err:?}"))?
    .dyn_into::<Function>()
    .map_err(|_| "MediaPipe createFromOptions is not callable".to_string())?;

    let base_options = Object::new();
    let _ = Reflect::set(
        &base_options,
        &JsValue::from_str("modelAssetPath"),
        &JsValue::from_str(&assets.detector_model_url),
    );
    let _ = Reflect::set(
        &base_options,
        &JsValue::from_str("delegate"),
        &JsValue::from_str("GPU"),
    );

    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("baseOptions"),
        base_options.as_ref(),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("runningMode"),
        &JsValue::from_str("IMAGE"),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("minDetectionConfidence"),
        &JsValue::from_f64(0.25),
    );

    let detector = resolve_js_value(
        create_from_options
            .call2(face_detector.as_ref(), &vision, options.as_ref())
            .map_err(|err| format!("MediaPipe createFromOptions call failed: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("MediaPipe detector creation failed: {err}"))?;

    let object_url = web_sys::Url::create_object_url_with_blob(&file)
        .map_err(|err| format!("Failed to create object URL: {err:?}"))?;
    let image = web_sys::HtmlImageElement::new().map_err(|err| format!("{err:?}"))?;
    image.set_src(&object_url);
    let decode_result = wasm_bindgen_futures::JsFuture::from(image.decode()).await;
    let _ = web_sys::Url::revoke_object_url(&object_url);
    decode_result.map_err(|err| format!("Image decode failed: {err:?}"))?;

    let detect_fn = Reflect::get(&detector, &JsValue::from_str("detect"))
        .map_err(|err| format!("MediaPipe detect lookup failed: {err:?}"))?
        .dyn_into::<Function>()
        .map_err(|_| "MediaPipe detect is not callable".to_string())?;
    let detection_result = resolve_js_value(
        detect_fn
            .call1(&detector, &image)
            .map_err(|err| format!("MediaPipe detect call failed: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("MediaPipe detect failed: {err}"))?;

    let detections = Reflect::get(&detection_result, &JsValue::from_str("detections"))
        .unwrap_or(JsValue::UNDEFINED);
    let detections = Array::from(&detections);
    let mut faces = Vec::with_capacity(detections.length() as usize);
    for idx in 0..detections.length() {
        let detection = detections.get(idx);
        let bbox =
            Reflect::get(&detection, &JsValue::from_str("boundingBox")).unwrap_or(JsValue::NULL);

        let x = first_number_property(&bbox, &["originX", "x"]).unwrap_or(0.0);
        let y = first_number_property(&bbox, &["originY", "y"]).unwrap_or(0.0);
        let width = first_number_property(&bbox, &["width"]).unwrap_or(0.0);
        let height = first_number_property(&bbox, &["height"]).unwrap_or(0.0);

        let confidence = if let Some(score) = first_number_property(&detection, &["score"]) {
            score
        } else {
            let categories =
                Reflect::get(&detection, &JsValue::from_str("categories")).unwrap_or(JsValue::NULL);
            let categories = Array::from(&categories);
            let first_category = categories.get(0);
            first_number_property(&first_category, &["score"]).unwrap_or(1.0)
        };

        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        faces.push(DetectedFace {
            id: format!("face_{}", idx + 1),
            x,
            y,
            width,
            height,
            confidence,
        });
    }

    Ok(faces)
}

#[cfg(target_arch = "wasm32")]
fn first_number_property(value: &web_sys::wasm_bindgen::JsValue, keys: &[&str]) -> Option<f64> {
    use web_sys::js_sys::Reflect;
    use web_sys::wasm_bindgen::JsValue;

    keys.iter().find_map(|key| {
        Reflect::get(value, &JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_f64())
    })
}

#[cfg(target_arch = "wasm32")]
async fn resolve_js_value(
    value: web_sys::wasm_bindgen::JsValue,
) -> Result<web_sys::wasm_bindgen::JsValue, String> {
    use web_sys::js_sys::Promise;

    wasm_bindgen_futures::JsFuture::from(Promise::resolve(&value))
        .await
        .map_err(|err| format!("{err:?}"))
}

#[cfg(target_arch = "wasm32")]
async fn import_js_module(url: &str) -> Result<web_sys::wasm_bindgen::JsValue, String> {
    use web_sys::js_sys::Function;
    use web_sys::wasm_bindgen::JsValue;

    let import_fn = Function::new_with_args("url", "return import(url);");
    let promise = import_fn
        .call1(&JsValue::NULL, &JsValue::from_str(url))
        .map_err(|err| format!("Dynamic import call failed for {url}: {err:?}"))?;
    resolve_js_value(promise)
        .await
        .map_err(|err| format!("Dynamic import failed for {url}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_stopped_or_unsupported() {
        let state = FaceWorkerBridgeState::default();
        #[cfg(target_arch = "wasm32")]
        assert_eq!(state.status, FaceWorkerStatus::Stopped);
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(state.status, FaceWorkerStatus::Unsupported);
    }

    #[test]
    fn stop_requires_active_state() {
        let mut state = FaceWorkerBridgeState {
            status: FaceWorkerStatus::Stopped,
            ..FaceWorkerBridgeState::default()
        };
        stop_face_worker(&mut state);
        assert_eq!(state.status, FaceWorkerStatus::Stopped);

        state.status = FaceWorkerStatus::Ready;
        stop_face_worker(&mut state);
        assert_eq!(state.status, FaceWorkerStatus::Stopped);
    }

    #[test]
    fn mark_request_failed_moves_worker_state_to_error() {
        let mut state = FaceWorkerBridgeState::default();
        state.mark_request_started();
        assert_eq!(state.status, FaceWorkerStatus::Starting);
        state.mark_request_failed("network timeout");
        assert_eq!(state.status, FaceWorkerStatus::Error);
        assert_eq!(state.last_error.as_deref(), Some("network timeout"));
    }
}

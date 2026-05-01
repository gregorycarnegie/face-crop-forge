// The inline JS run inside a dedicated Web Worker for face detection.
// Uses createImageBitmap (worker-safe) instead of HtmlImageElement, caches the
// FaceDetector instance across calls, and returns compact face objects.
#[cfg(target_arch = "wasm32")]
const DETECTION_WORKER_SCRIPT: &str = r#"'use strict';
var _detector = null;
self.onmessage = async function(e) {
    var d = e.data;
    var id = d.id;
    var type = d.type || 'detect';
    if (type === 'detect') {
        try {
            if (!_detector) {
                if (typeof FaceDetector === 'undefined') {
                    self.postMessage({ id: id, error: 'FaceDetector API not available in this worker' });
                    return;
                }
                _detector = new FaceDetector({ fastMode: true, maxDetectedFaces: 32 });
            }
            var bitmap = await createImageBitmap(d.file);
            var faces = await _detector.detect(bitmap);
            bitmap.close();
            var out = [];
            for (var i = 0; i < faces.length; i++) {
                var f = faces[i];
                var bb = f.boundingBox;
                var conf = f.confidence;
                if (Array.isArray(conf)) { conf = conf[0]; }
                if (typeof conf !== 'number') { conf = 1.0; }
                out.push({ x: bb.x, y: bb.y, w: bb.width, h: bb.height, c: conf });
            }
            self.postMessage({ id: id, faces: out });
        } catch(err) {
            _detector = null;
            self.postMessage({ id: id, error: String(err) });
        }
    } else if (type === 'crop') {
        try {
            if (typeof OffscreenCanvas === 'undefined') {
                self.postMessage({ id: id, error: 'OffscreenCanvas not supported' });
                return;
            }
            var bitmap;
            try {
                bitmap = await createImageBitmap(d.file, d.sx, d.sy, d.sw, d.sh, {
                    resizeWidth: d.outW, resizeHeight: d.outH, resizeQuality: 'high'
                });
            } catch(_) {
                bitmap = await createImageBitmap(d.file, d.sx, d.sy, d.sw, d.sh);
            }
            var canvas = new OffscreenCanvas(d.outW, d.outH);
            var ctx = canvas.getContext('2d');
            ctx.drawImage(bitmap, 0, 0, d.outW, d.outH);
            bitmap.close();
            var opts = { type: d.mime };
            if (d.quality != null) { opts.quality = d.quality; }
            var blob = await canvas.convertToBlob(opts);
            var buffer = await blob.arrayBuffer();
            self.postMessage({ id: id, buffer: buffer }, [buffer]);
        } catch(err) {
            self.postMessage({ id: id, error: String(err) });
        }
    }
};
"#;

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
    static DETECTION_WORKER: std::cell::RefCell<Option<web_sys::Worker>> =
        const { std::cell::RefCell::new(None) };
    static PENDING_DETECTIONS: std::cell::RefCell<
        std::collections::HashMap<u32, (js_sys::Function, js_sys::Function)>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
    static NEXT_JOB_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static CACHED_MP_DETECTOR: std::cell::RefCell<Option<wasm_bindgen::JsValue>> =
        const { std::cell::RefCell::new(None) };
}

pub fn last_detection_backend_label() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        LAST_DETECTION_BACKEND.with(|slot| match *slot.borrow() {
            Some(DetectionBackend::BrowserFaceDetector) => "Native FaceDetector (worker)",
            Some(DetectionBackend::MediaPipe) => "MediaPipe Tasks",
            None => "None",
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "Unavailable"
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clear_last_detection_backend() {
    LAST_DETECTION_BACKEND.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn clear_detector_caches() {
    DETECTION_WORKER.with(|slot| {
        if let Some(worker) = slot.borrow_mut().take() {
            worker.terminate();
        }
    });
    PENDING_DETECTIONS.with(|pending| pending.borrow_mut().clear());
    CACHED_MP_DETECTOR.with(|slot| *slot.borrow_mut() = None);
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
    #[cfg(any(target_arch = "wasm32", test))]
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
    pub fn can_start(&self) -> bool {
        #[cfg(any(target_arch = "wasm32", test))]
        {
            matches!(self.status, FaceWorkerStatus::Stopped)
        }
        #[cfg(not(any(target_arch = "wasm32", test)))]
        {
            false
        }
    }

    #[cfg(test)]
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
        state.status = FaceWorkerStatus::Ready;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        state.status = FaceWorkerStatus::Unsupported;
        state.last_error = Some("Face detection runtime is only available on wasm32".to_string());
    }
}

#[cfg(test)]
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
    match detect_with_detection_worker(file.clone()).await {
        Ok(faces) => {
            set_last_detection_backend(DetectionBackend::BrowserFaceDetector);
            Ok(faces)
        }
        Err(worker_error) => match detect_faces_with_mediapipe(file).await {
            Ok(faces) => {
                set_last_detection_backend(DetectionBackend::MediaPipe);
                Ok(faces)
            }
            Err(mediapipe_error) => Err(format!(
                "Worker FaceDetector failed ({worker_error}); MediaPipe fallback failed ({mediapipe_error})"
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

/// Creates a Web Worker from the inline detection script and wires up persistent
/// message and error handlers.  The worker is cached; subsequent calls return the
/// same instance until it crashes.
#[cfg(target_arch = "wasm32")]
fn get_or_create_detection_worker() -> Result<web_sys::Worker, String> {
    use js_sys::Reflect;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen::closure::Closure;

    DETECTION_WORKER.with(|slot| {
        if let Some(worker) = slot.borrow().as_ref().cloned() {
            return Ok(worker);
        }

        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(DETECTION_WORKER_SCRIPT));
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/javascript");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)
            .map_err(|e| format!("Detection worker blob: {e:?}"))?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|e| format!("Detection worker URL: {e:?}"))?;
        let worker = web_sys::Worker::new(&url).map_err(|e| {
            let _ = web_sys::Url::revoke_object_url(&url);
            format!("Detection worker creation: {e:?}")
        })?;
        let _ = web_sys::Url::revoke_object_url(&url);

        // Route incoming worker messages to the waiting Promise resolve for that job id.
        let onmessage: Closure<dyn FnMut(JsValue)> =
            Closure::wrap(Box::new(move |event: JsValue| {
                let data =
                    Reflect::get(&event, &JsValue::from_str("data")).unwrap_or(JsValue::UNDEFINED);
                let id = Reflect::get(&data, &JsValue::from_str("id"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|n| n as u32);
                if let Some(job_id) = id {
                    PENDING_DETECTIONS.with(|pending| {
                        if let Some((resolve, _reject)) = pending.borrow_mut().remove(&job_id) {
                            let _ = resolve.call1(&JsValue::UNDEFINED, &data);
                        }
                    });
                }
            }));
        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // On an unrecoverable worker crash, clear the cached instance and reject
        // all in-flight jobs so callers don't hang indefinitely.
        let onerror: Closure<dyn FnMut(JsValue)> =
            Closure::wrap(Box::new(move |_event: JsValue| {
                DETECTION_WORKER.with(|s| *s.borrow_mut() = None);
                PENDING_DETECTIONS.with(|pending| {
                    for (_id, (_resolve, reject)) in pending.borrow_mut().drain() {
                        let _ = reject.call1(
                            &JsValue::UNDEFINED,
                            &JsValue::from_str("Detection worker crashed"),
                        );
                    }
                });
            }));
        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        *slot.borrow_mut() = Some(worker.clone());
        Ok(worker)
    })
}

#[cfg(target_arch = "wasm32")]
fn next_detection_job_id() -> u32 {
    NEXT_JOB_ID.with(|c| {
        let id = c.get();
        c.set(id.wrapping_add(1));
        id
    })
}

/// Sends one file to the detection worker and awaits the response via a Promise
/// whose resolve/reject functions are stored in `PENDING_DETECTIONS`.
#[cfg(target_arch = "wasm32")]
async fn detect_with_detection_worker(file: web_sys::File) -> Result<Vec<DetectedFace>, String> {
    use js_sys::Reflect;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsValue;

    let worker = get_or_create_detection_worker()?;
    let job_id = next_detection_job_id();

    // Promise::new runs the executor synchronously, so resolve/reject are set before
    // the constructor returns.
    let slot: Rc<RefCell<Option<(js_sys::Function, js_sys::Function)>>> =
        Rc::new(RefCell::new(None));
    let slot2 = Rc::clone(&slot);
    let promise = js_sys::Promise::new(&mut move |resolve, reject| {
        *slot2.borrow_mut() = Some((resolve, reject));
    });
    let (resolve, reject) = slot
        .borrow_mut()
        .take()
        .expect("Promise executor is synchronous");

    PENDING_DETECTIONS.with(|pending| pending.borrow_mut().insert(job_id, (resolve, reject)));

    let msg = js_sys::Object::new();
    let _ = Reflect::set(
        &msg,
        &JsValue::from_str("id"),
        &JsValue::from_f64(job_id as f64),
    );
    let _ = Reflect::set(&msg, &JsValue::from_str("file"), file.as_ref());
    if let Err(e) = worker.post_message(msg.as_ref()) {
        PENDING_DETECTIONS.with(|pending| {
            pending.borrow_mut().remove(&job_id);
        });
        return Err(format!("Detection worker post_message failed: {e:?}"));
    }

    let data = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Detection worker rejected: {e:?}"))?;

    parse_worker_faces(&data)
}

#[cfg(target_arch = "wasm32")]
fn parse_worker_faces(data: &wasm_bindgen::JsValue) -> Result<Vec<DetectedFace>, String> {
    use js_sys::{Array, Reflect};
    use wasm_bindgen::JsValue;

    if let Some(error) = Reflect::get(data, &JsValue::from_str("error"))
        .ok()
        .filter(|v| !v.is_null() && !v.is_undefined())
        .and_then(|v| v.as_string())
    {
        return Err(error);
    }

    let faces_val = Reflect::get(data, &JsValue::from_str("faces")).unwrap_or(JsValue::UNDEFINED);
    let faces_arr = Array::from(&faces_val);
    let mut faces = Vec::with_capacity(faces_arr.length() as usize);
    for i in 0..faces_arr.length() {
        let face = faces_arr.get(i);
        let x = Reflect::get(&face, &JsValue::from_str("x"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let y = Reflect::get(&face, &JsValue::from_str("y"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let w = Reflect::get(&face, &JsValue::from_str("w"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let h = Reflect::get(&face, &JsValue::from_str("h"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let c = Reflect::get(&face, &JsValue::from_str("c"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        if w > 0.0 && h > 0.0 {
            faces.push(DetectedFace {
                id: format!("face_{}", i + 1),
                x,
                y,
                width: w,
                height: h,
                confidence: c,
            });
        }
    }
    Ok(faces)
}

/// Sends one crop job to the detection worker, which uses `OffscreenCanvas` to
/// do the pixel work off the main thread.  Returns `Err` if `OffscreenCanvas`
/// is not available in this browser; the caller falls back to the main-thread
/// canvas path in that case.
#[cfg(target_arch = "wasm32")]
pub async fn crop_face_in_worker(
    file: &web_sys::File,
    sx: f64,
    sy: f64,
    sw: f64,
    sh: f64,
    out_w: u32,
    out_h: u32,
    mime: &str,
    quality: Option<f64>,
) -> Result<Vec<u8>, String> {
    use js_sys::Reflect;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsValue;

    let worker = get_or_create_detection_worker()?;
    let job_id = next_detection_job_id();

    let slot: Rc<RefCell<Option<(js_sys::Function, js_sys::Function)>>> =
        Rc::new(RefCell::new(None));
    let slot2 = Rc::clone(&slot);
    let promise = js_sys::Promise::new(&mut move |resolve, reject| {
        *slot2.borrow_mut() = Some((resolve, reject));
    });
    let (resolve, reject) = slot
        .borrow_mut()
        .take()
        .expect("Promise executor is synchronous");

    PENDING_DETECTIONS.with(|pending| pending.borrow_mut().insert(job_id, (resolve, reject)));

    let msg = js_sys::Object::new();
    let _ = Reflect::set(&msg, &JsValue::from_str("type"), &JsValue::from_str("crop"));
    let _ = Reflect::set(
        &msg,
        &JsValue::from_str("id"),
        &JsValue::from_f64(job_id as f64),
    );
    let _ = Reflect::set(&msg, &JsValue::from_str("file"), file.as_ref());
    let _ = Reflect::set(&msg, &JsValue::from_str("sx"), &JsValue::from_f64(sx));
    let _ = Reflect::set(&msg, &JsValue::from_str("sy"), &JsValue::from_f64(sy));
    let _ = Reflect::set(&msg, &JsValue::from_str("sw"), &JsValue::from_f64(sw));
    let _ = Reflect::set(&msg, &JsValue::from_str("sh"), &JsValue::from_f64(sh));
    let _ = Reflect::set(
        &msg,
        &JsValue::from_str("outW"),
        &JsValue::from_f64(f64::from(out_w)),
    );
    let _ = Reflect::set(
        &msg,
        &JsValue::from_str("outH"),
        &JsValue::from_f64(f64::from(out_h)),
    );
    let _ = Reflect::set(&msg, &JsValue::from_str("mime"), &JsValue::from_str(mime));
    let quality_val = quality.map(JsValue::from_f64).unwrap_or(JsValue::NULL);
    let _ = Reflect::set(&msg, &JsValue::from_str("quality"), &quality_val);

    if let Err(e) = worker.post_message(msg.as_ref()) {
        PENDING_DETECTIONS.with(|pending| {
            pending.borrow_mut().remove(&job_id);
        });
        return Err(format!("Crop worker post_message failed: {e:?}"));
    }

    let data = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Crop worker rejected: {e:?}"))?;

    if let Some(error) = Reflect::get(&data, &JsValue::from_str("error"))
        .ok()
        .filter(|v| !v.is_null() && !v.is_undefined())
        .and_then(|v| v.as_string())
    {
        return Err(error);
    }

    let buffer_val = Reflect::get(&data, &JsValue::from_str("buffer"))
        .map_err(|e| format!("Crop response missing buffer: {e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buffer_val).to_vec())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crop_face_in_worker(
    _file: &web_sys::File,
    _sx: f64,
    _sy: f64,
    _sw: f64,
    _sh: f64,
    _out_w: u32,
    _out_h: u32,
    _mime: &str,
    _quality: Option<f64>,
) -> Result<Vec<u8>, String> {
    Err("Worker crop is only available on wasm32".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn get_or_create_mp_detector() -> Result<wasm_bindgen::JsValue, String> {
    use js_sys::{Function, Object, Reflect};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    if let Some(cached) = CACHED_MP_DETECTOR.with(|s| s.borrow().clone()) {
        return Ok(cached);
    }

    let assets = crate::mediapipe::MediaPipeAssetPaths::default();
    let module = import_js_module(&assets.vision_bundle_url).await?;

    let fileset_resolver = Reflect::get(&module, &JsValue::from_str("FilesetResolver"))
        .map_err(|err| format!("MediaPipe FilesetResolver lookup failed: {err:?}"))?
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

    let face_detector_cls = Reflect::get(&module, &JsValue::from_str("FaceDetector"))
        .map_err(|err| format!("MediaPipe FaceDetector export lookup failed: {err:?}"))?
        .dyn_into::<Object>()
        .map_err(|_| "MediaPipe FaceDetector export is not an object".to_string())?;
    let create_from_options = Reflect::get(
        face_detector_cls.as_ref(),
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
            .call2(face_detector_cls.as_ref(), &vision, options.as_ref())
            .map_err(|err| format!("MediaPipe createFromOptions call failed: {err:?}"))?,
    )
    .await
    .map_err(|err| format!("MediaPipe detector creation failed: {err}"))?;

    CACHED_MP_DETECTOR.with(|s| *s.borrow_mut() = Some(detector.clone()));
    Ok(detector)
}

#[cfg(target_arch = "wasm32")]
async fn detect_faces_with_mediapipe(file: web_sys::File) -> Result<Vec<DetectedFace>, String> {
    use js_sys::{Array, Function, Reflect};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let detector = get_or_create_mp_detector().await?;

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
    let detection_result = resolve_js_value(detect_fn.call1(&detector, &image).map_err(|err| {
        CACHED_MP_DETECTOR.with(|s| *s.borrow_mut() = None);
        format!("MediaPipe detect call failed: {err:?}")
    })?)
    .await
    .map_err(|err| {
        CACHED_MP_DETECTOR.with(|s| *s.borrow_mut() = None);
        format!("MediaPipe detect failed: {err}")
    })?;

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
fn first_number_property(value: &wasm_bindgen::JsValue, keys: &[&str]) -> Option<f64> {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    keys.iter().find_map(|key| {
        Reflect::get(value, &JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_f64())
    })
}

#[cfg(target_arch = "wasm32")]
async fn resolve_js_value(value: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, String> {
    use js_sys::Promise;

    wasm_bindgen_futures::JsFuture::from(Promise::resolve(&value))
        .await
        .map_err(|err| format!("{err:?}"))
}

#[cfg(target_arch = "wasm32")]
async fn import_js_module(url: &str) -> Result<wasm_bindgen::JsValue, String> {
    use js_sys::Function;
    use wasm_bindgen::JsValue;

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

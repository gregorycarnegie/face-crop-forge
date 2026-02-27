#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelegatePreference {
    Gpu,
    Cpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmVariant {
    SimdThreaded,
    SimdSingleThread,
    NoSimd,
}

impl WasmVariant {
    pub fn label(self) -> &'static str {
        match self {
            WasmVariant::SimdThreaded => "SIMD+threads",
            WasmVariant::SimdSingleThread => "SIMD",
            WasmVariant::NoSimd => "no-SIMD",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserCapabilities {
    pub simd: bool,
    pub threads: bool,
    pub offscreen_canvas: bool,
    pub image_bitmap: bool,
}

impl BrowserCapabilities {
    pub const fn baseline() -> Self {
        Self {
            simd: true,
            threads: true,
            offscreen_canvas: true,
            image_bitmap: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineState {
    Ready,
    Partial,
    Unavailable,
}

impl PipelineState {
    pub fn label(self) -> &'static str {
        match self {
            PipelineState::Ready => "Ready",
            PipelineState::Partial => "Partial Fallback",
            PipelineState::Unavailable => "Unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineHealth {
    pub offscreen_canvas: bool,
    pub image_bitmap: bool,
    pub worker_transfer: bool,
    pub state: PipelineState,
}

impl PipelineHealth {
    pub fn summary(&self) -> &'static str {
        self.state.label()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaPipeAssetPaths {
    pub wasm_root: String,
    pub vision_bundle_url: String,
    pub detector_model_url: String,
    pub landmarker_model_url: String,
}

impl Default for MediaPipeAssetPaths {
    fn default() -> Self {
        Self {
            wasm_root: "/models/wasm".to_string(),
            vision_bundle_url: "/models/vision_bundle.mjs".to_string(),
            detector_model_url: "/models/blaze_face_short_range.tflite".to_string(),
            landmarker_model_url: "/models/face_landmarker.task".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaPipeLoadPlan {
    pub assets: MediaPipeAssetPaths,
    pub wasm_variant: WasmVariant,
    pub delegate: DelegatePreference,
    pub enable_landmarker: bool,
    pub use_worker_pipeline: bool,
}

impl MediaPipeLoadPlan {
    pub fn summary(&self) -> String {
        let wasm = self.wasm_variant.label();
        let delegate = match self.delegate {
            DelegatePreference::Gpu => "GPU",
            DelegatePreference::Cpu => "CPU",
        };
        let landmarker = if self.enable_landmarker {
            "landmarker+detector"
        } else {
            "detector-only"
        };
        let pipeline = if self.use_worker_pipeline {
            "worker pipeline"
        } else {
            "main-thread fallback"
        };
        format!("{wasm}, {delegate}, {landmarker}, {pipeline}")
    }
}

pub fn choose_wasm_variant(capabilities: BrowserCapabilities) -> WasmVariant {
    if capabilities.simd && capabilities.threads {
        WasmVariant::SimdThreaded
    } else if capabilities.simd {
        WasmVariant::SimdSingleThread
    } else {
        WasmVariant::NoSimd
    }
}

pub fn build_load_plan(
    capabilities: BrowserCapabilities,
    prefer_gpu: bool,
    prefer_landmarker: bool,
    assets: MediaPipeAssetPaths,
) -> MediaPipeLoadPlan {
    let wasm_variant = choose_wasm_variant(capabilities);
    let delegate = if prefer_gpu {
        DelegatePreference::Gpu
    } else {
        DelegatePreference::Cpu
    };

    MediaPipeLoadPlan {
        assets,
        wasm_variant,
        delegate,
        enable_landmarker: prefer_landmarker,
        use_worker_pipeline: capabilities.offscreen_canvas && capabilities.image_bitmap,
    }
}

pub fn validate_asset_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains("..")
}

pub fn evaluate_pipeline_health(capabilities: BrowserCapabilities) -> PipelineHealth {
    let worker_transfer = capabilities.offscreen_canvas && capabilities.image_bitmap;
    let state = if worker_transfer {
        PipelineState::Ready
    } else if capabilities.image_bitmap || capabilities.offscreen_canvas {
        PipelineState::Partial
    } else {
        PipelineState::Unavailable
    };

    PipelineHealth {
        offscreen_canvas: capabilities.offscreen_canvas,
        image_bitmap: capabilities.image_bitmap,
        worker_transfer,
        state,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserValidationProfile {
    pub browser: &'static str,
    pub capabilities: BrowserCapabilities,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFallbackValidation {
    pub browser: &'static str,
    pub wasm_variant: WasmVariant,
    pub pipeline_state: PipelineState,
    pub worker_pipeline: bool,
    pub fallback_reason: &'static str,
}

pub fn browser_validation_profiles() -> [BrowserValidationProfile; 6] {
    [
        BrowserValidationProfile {
            browser: "Chrome 122+",
            capabilities: BrowserCapabilities {
                simd: true,
                threads: true,
                offscreen_canvas: true,
                image_bitmap: true,
            },
        },
        BrowserValidationProfile {
            browser: "Edge 122+",
            capabilities: BrowserCapabilities {
                simd: true,
                threads: true,
                offscreen_canvas: true,
                image_bitmap: true,
            },
        },
        BrowserValidationProfile {
            browser: "Firefox 123+",
            capabilities: BrowserCapabilities {
                simd: true,
                threads: true,
                offscreen_canvas: true,
                image_bitmap: true,
            },
        },
        BrowserValidationProfile {
            browser: "Safari 17+ (macOS)",
            capabilities: BrowserCapabilities {
                simd: true,
                threads: false,
                offscreen_canvas: true,
                image_bitmap: true,
            },
        },
        BrowserValidationProfile {
            browser: "Safari 17+ (iOS)",
            capabilities: BrowserCapabilities {
                simd: true,
                threads: false,
                offscreen_canvas: false,
                image_bitmap: true,
            },
        },
        BrowserValidationProfile {
            browser: "Legacy WebView",
            capabilities: BrowserCapabilities {
                simd: false,
                threads: false,
                offscreen_canvas: false,
                image_bitmap: false,
            },
        },
    ]
}

pub fn revalidate_browser_fallbacks(
    prefer_gpu: bool,
    prefer_landmarker: bool,
    assets: MediaPipeAssetPaths,
) -> Vec<BrowserFallbackValidation> {
    browser_validation_profiles()
        .into_iter()
        .map(|profile| {
            let plan = build_load_plan(
                profile.capabilities,
                prefer_gpu,
                prefer_landmarker,
                assets.clone(),
            );
            let health = evaluate_pipeline_health(profile.capabilities);
            let fallback_reason = match (plan.wasm_variant, health.state) {
                (WasmVariant::SimdThreaded, PipelineState::Ready) => "No fallback required",
                (WasmVariant::SimdSingleThread, PipelineState::Ready) => {
                    "Threads unavailable; using SIMD single-thread"
                }
                (WasmVariant::SimdThreaded, PipelineState::Partial)
                | (WasmVariant::SimdSingleThread, PipelineState::Partial) => {
                    "Offscreen/transfer partial; running mixed pipeline"
                }
                (_, PipelineState::Unavailable) => "Worker pipeline unavailable; main-thread path",
                (WasmVariant::NoSimd, _) => "SIMD unavailable; using scalar WASM fallback",
            };

            BrowserFallbackValidation {
                browser: profile.browser,
                wasm_variant: plan.wasm_variant,
                pipeline_state: health.state,
                worker_pipeline: plan.use_worker_pipeline,
                fallback_reason,
            }
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub fn detect_browser_capabilities() -> BrowserCapabilities {
    use web_sys::js_sys::Reflect;
    use web_sys::wasm_bindgen::JsValue;

    let global = web_sys::js_sys::global();
    let has = |name: &str| Reflect::has(&global, &JsValue::from_str(name)).unwrap_or(false);

    let offscreen_canvas = has("OffscreenCanvas");
    let image_bitmap = has("ImageBitmap") || has("createImageBitmap");
    // Robust SIMD/thread probing is browser/runtime-specific; keep optimistic defaults and
    // fall back to no-SIMD assets as needed at runtime.
    let simd = has("WebAssembly");
    let threads = has("SharedArrayBuffer");

    BrowserCapabilities {
        simd,
        threads,
        offscreen_canvas,
        image_bitmap,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn detect_browser_capabilities() -> BrowserCapabilities {
    BrowserCapabilities::baseline()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_variant_selection_prefers_simd_threads() {
        assert_eq!(
            choose_wasm_variant(BrowserCapabilities {
                simd: true,
                threads: true,
                offscreen_canvas: true,
                image_bitmap: true,
            }),
            WasmVariant::SimdThreaded
        );
        assert_eq!(
            choose_wasm_variant(BrowserCapabilities {
                simd: true,
                threads: false,
                offscreen_canvas: true,
                image_bitmap: true,
            }),
            WasmVariant::SimdSingleThread
        );
        assert_eq!(
            choose_wasm_variant(BrowserCapabilities {
                simd: false,
                threads: false,
                offscreen_canvas: true,
                image_bitmap: true,
            }),
            WasmVariant::NoSimd
        );
    }

    #[test]
    fn load_plan_respects_worker_pipeline_requirements() {
        let plan = build_load_plan(
            BrowserCapabilities {
                simd: true,
                threads: true,
                offscreen_canvas: false,
                image_bitmap: true,
            },
            true,
            true,
            MediaPipeAssetPaths::default(),
        );
        assert!(!plan.use_worker_pipeline);
        assert_eq!(plan.delegate, DelegatePreference::Gpu);
        assert!(plan.enable_landmarker);
    }

    #[test]
    fn asset_path_validation_blocks_parent_traversal() {
        assert!(validate_asset_path("/models/face_landmarker.task"));
        assert!(!validate_asset_path("models/face_landmarker.task"));
        assert!(!validate_asset_path("/models/../../secret"));
    }

    #[test]
    fn pipeline_health_states_are_computed_correctly() {
        let ready = evaluate_pipeline_health(BrowserCapabilities {
            simd: true,
            threads: true,
            offscreen_canvas: true,
            image_bitmap: true,
        });
        assert_eq!(ready.state, PipelineState::Ready);
        assert!(ready.worker_transfer);

        let partial = evaluate_pipeline_health(BrowserCapabilities {
            simd: true,
            threads: true,
            offscreen_canvas: false,
            image_bitmap: true,
        });
        assert_eq!(partial.state, PipelineState::Partial);
        assert!(!partial.worker_transfer);

        let unavailable = evaluate_pipeline_health(BrowserCapabilities {
            simd: true,
            threads: true,
            offscreen_canvas: false,
            image_bitmap: false,
        });
        assert_eq!(unavailable.state, PipelineState::Unavailable);
        assert_eq!(unavailable.summary(), "Unavailable");
    }

    #[test]
    fn browser_fallback_matrix_has_expected_profiles() {
        let matrix = revalidate_browser_fallbacks(true, true, MediaPipeAssetPaths::default());
        assert_eq!(matrix.len(), 6);
        assert!(matrix.iter().any(|entry| entry.browser.contains("Chrome")));
        assert!(matrix.iter().any(|entry| entry.browser.contains("Safari")));
    }

    #[test]
    fn safari_profiles_degrade_to_no_threads() {
        let matrix = revalidate_browser_fallbacks(true, true, MediaPipeAssetPaths::default());
        let mac = matrix
            .iter()
            .find(|entry| entry.browser == "Safari 17+ (macOS)")
            .expect("missing macOS Safari profile");
        assert_eq!(mac.wasm_variant, WasmVariant::SimdSingleThread);
        assert_eq!(mac.pipeline_state, PipelineState::Ready);
    }

    #[test]
    fn legacy_profile_uses_full_fallback_path() {
        let matrix = revalidate_browser_fallbacks(true, true, MediaPipeAssetPaths::default());
        let legacy = matrix
            .iter()
            .find(|entry| entry.browser == "Legacy WebView")
            .expect("missing legacy profile");
        assert_eq!(legacy.wasm_variant, WasmVariant::NoSimd);
        assert_eq!(legacy.pipeline_state, PipelineState::Unavailable);
        assert!(!legacy.worker_pipeline);
    }
}

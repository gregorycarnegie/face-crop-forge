use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn CropSettingsPanel() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    view! {
        <div class="crop-settings">
            <h3 class="collapsible-header">
                "Smart Cropping Settings"
                <span class="collapse-icon">"▼"</span>
            </h3>
            <div class="collapsible-content">
                <div class="preset-controls">
                    <div class="setting-group full-width">
                        <label for="sizePreset">"Size Presets"</label>
                        <select id="sizePreset">
                            <option value="custom">"Custom"</option>
                            <option value="linkedin">"LinkedIn Profile (400×400)"</option>
                            <option value="passport">"Passport Photo (413×531)"</option>
                            <option value="instagram">"Instagram Square (1080×1080)"</option>
                            <option value="idcard">"ID Card (332×498)"</option>
                            <option value="avatar">"Avatar (512×512)"</option>
                            <option value="headshot">"Professional Headshot (600×800)"</option>
                        </select>
                    </div>
                </div>

                <div class="settings-grid">
                    <div class="setting-group size-controls">
                        <div class="size-inputs">
                            <div class="input-group">
                                <label for="outputWidth">"Output Width (px)"</label>
                                <div class="input-with-lock">
                                    <input
                                        type="number"
                                        id="outputWidth"
                                        min="64"
                                        max="2048"
                                        step="32"
                                        prop:value=move || settings.get().output_width.to_string()
                                        on:input=move |ev| {
                                            if let Ok(value) = event_target_value(&ev).parse::<u32>() {
                                                settings.update(|s| s.output_width = value.clamp(64, 2048));
                                            }
                                        }
                                    />
                                    <button type="button" id="aspectRatioLock" class="lock-button" title="Lock aspect ratio">
                                        "🔓"
                                    </button>
                                </div>
                            </div>
                            <div class="input-group">
                                <label for="outputHeight">"Output Height (px)"</label>
                                <input
                                    type="number"
                                    id="outputHeight"
                                    min="64"
                                    max="2048"
                                    step="32"
                                    prop:value=move || settings.get().output_height.to_string()
                                    on:input=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<u32>() {
                                            settings.update(|s| s.output_height = value.clamp(64, 2048));
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </div>
                    <div class="setting-group">
                        <label for="faceHeightPct">"Face Height %"</label>
                        <input
                            type="number"
                            id="faceHeightPct"
                            min="10"
                            max="100"
                            step="5"
                            prop:value=move || settings.get().face_height_pct.to_string()
                            on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<u8>() {
                                    settings.update(|s| s.face_height_pct = value.clamp(10, 100));
                                }
                            }
                        />
                    </div>
                    <div class="setting-group">
                        <label for="positioningMode">"Positioning Mode"</label>
                        <select id="positioningMode">
                            <option value="center">"Center Face"</option>
                            <option value="rule-of-thirds">"Rule of Thirds"</option>
                            <option value="custom">"Custom Position"</option>
                        </select>
                    </div>
                </div>

                <div class="advanced-positioning" id="advancedPositioning">
                    <div class="setting-group">
                        <label for="verticalOffset">"Vertical Offset"</label>
                        <div class="offset-slider-container">
                            <input
                                type="range"
                                id="verticalOffset"
                                min="-50"
                                max="50"
                                prop:value=move || settings.get().vertical_offset_pct.to_string()
                                on:input=move |ev| {
                                    if let Ok(value) = event_target_value(&ev).parse::<i32>() {
                                        settings.update(|s| s.vertical_offset_pct = value.clamp(-50, 50));
                                    }
                                }
                            />
                            <span class="offset-value" id="verticalOffsetValue">"0%"</span>
                        </div>
                        <div class="offset-help">"Negative: up, Positive: down"</div>
                    </div>
                    <div class="setting-group">
                        <label for="horizontalOffset">"Horizontal Offset"</label>
                        <div class="offset-slider-container">
                            <input
                                type="range"
                                id="horizontalOffset"
                                min="-50"
                                max="50"
                                prop:value=move || settings.get().horizontal_offset_pct.to_string()
                                on:input=move |ev| {
                                    if let Ok(value) = event_target_value(&ev).parse::<i32>() {
                                        settings.update(|s| s.horizontal_offset_pct = value.clamp(-50, 50));
                                    }
                                }
                            />
                            <span class="offset-value" id="horizontalOffsetValue">"0%"</span>
                        </div>
                        <div class="offset-help">"Negative: left, Positive: right"</div>
                    </div>
                </div>

                <div class="preset-actions">
                    <button type="button" id="resetSettingsBtn" class="reset-button" on:click=move |_| settings.set(crate::state::ProcessingSettings::default())>"Reset to Defaults"</button>
                </div>

                <div class="setting-preview">
                    <span>
                        "Preview: "
                        <span id="previewText">"256×256px, face at 70% height, PNG format"</span>
                    </span>
                    <span class="aspect-ratio" id="aspectRatioText">"1:1 ratio"</span>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn PreprocessingSettingsPanel() -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    view! {
        <div class="preprocessing-settings">
            <h3 class="collapsible-header">
                "Image Preprocessing"
                <span class="collapse-icon">"▼"</span>
            </h3>
            <div class="collapsible-content">
                <div class="preprocessing-controls">
                    <div class="setting-group">
                        <label>
                            <input type="checkbox" id="autoColorCorrection" prop:checked=move || settings.get().auto_color_correction on:change=move |ev| settings.update(|s| s.auto_color_correction = event_target_checked(&ev)) />
                            " Auto Color Correction"
                        </label>
                        <div class="setting-help">"Automatic brightness/contrast normalization"</div>
                    </div>

                    <div class="setting-group">
                        <label for="exposureAdjustment">"Exposure"</label>
                        <div class="slider-container">
                            <input type="range" id="exposureAdjustment" min="-2" max="2" step="0.1" prop:value=move || settings.get().exposure_adjustment.to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.exposure_adjustment = value.clamp(-2.0, 2.0));
                                }
                            } />
                            <span class="slider-value" id="exposureValue">"0"</span>
                        </div>
                        <div class="setting-help">"Adjust image brightness (-2 to +2)"</div>
                    </div>

                    <div class="setting-group">
                        <label for="contrastAdjustment">"Contrast"</label>
                        <div class="slider-container">
                            <input type="range" id="contrastAdjustment" min="0.5" max="2" step="0.1" prop:value=move || settings.get().contrast_adjustment.to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.contrast_adjustment = value.clamp(0.5, 2.0));
                                }
                            } />
                            <span class="slider-value" id="contrastValue">"1.0"</span>
                        </div>
                        <div class="setting-help">"Adjust image contrast (0.5 to 2.0)"</div>
                    </div>

                    <div class="setting-group">
                        <label for="sharpnessControl">"Sharpness"</label>
                        <div class="slider-container">
                            <input type="range" id="sharpnessControl" min="0" max="2" step="0.1" prop:value=move || settings.get().sharpness.to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.sharpness = value.clamp(0.0, 2.0));
                                }
                            } />
                            <span class="slider-value" id="sharpnessValue">"0"</span>
                        </div>
                        <div class="setting-help">"Apply unsharp mask filter (0 to 2)"</div>
                    </div>

                    <div class="setting-group">
                        <label for="skinSmoothing">"Skin Smoothing"</label>
                        <div class="slider-container">
                            <input type="range" id="skinSmoothing" min="0" max="5" step="0.5" prop:value=move || settings.get().skin_smoothing.to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.skin_smoothing = value.clamp(0.0, 5.0));
                                }
                            } />
                            <span class="slider-value" id="skinSmoothingValue">"0"</span>
                        </div>
                        <div class="setting-help">"Smooth skin tones (0 to 5)"</div>
                    </div>

                    <div class="setting-group">
                        <label>
                            <input type="checkbox" id="redEyeRemoval" prop:checked=move || settings.get().red_eye_removal on:change=move |ev| settings.update(|s| s.red_eye_removal = event_target_checked(&ev)) />
                            " Red-eye Removal"
                        </label>
                        <div class="setting-help">"Detect and correct red-eye effect"</div>
                    </div>

                    <div class="setting-group">
                        <label for="backgroundBlur">"Background Blur"</label>
                        <div class="slider-container">
                            <input type="range" id="backgroundBlur" min="0" max="10" step="0.5" prop:value=move || settings.get().background_blur.to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.background_blur = value.clamp(0.0, 10.0));
                                }
                            } />
                            <span class="slider-value" id="backgroundBlurValue">"0px"</span>
                        </div>
                        <div class="setting-help">"Blur background around faces (0-10px)"</div>
                    </div>
                    <div class="setting-group">
                        <label for="minConfidence">"Min Detection Confidence"</label>
                        <div class="slider-container">
                            <input
                                type="range"
                                id="minConfidence"
                                min="0"
                                max="1"
                                step="0.05"
                                prop:value=move || settings.get().min_confidence.to_string()
                                on:input=move |ev| {
                                    if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                        settings.update(|s| s.min_confidence = value.clamp(0.0, 1.0));
                                    }
                                }
                            />
                            <span class="slider-value">{move || format!("{:.2}", settings.get().min_confidence)}</span>
                        </div>
                        <div class="setting-help">"Discard detections below this confidence"</div>
                    </div>
                </div>

                <div class="preprocessing-actions">
                    <button type="button" id="previewEnhancementsBtn" class="preview-button">"Preview Enhancements"</button>
                    <button type="button" id="resetEnhancementsBtn" class="reset-button">"Reset All"</button>
                </div>

                <div class="enhancement-preview">
                    <span>"Preview: "</span>
                    <span id="enhancementSummary">"No enhancements applied"</span>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn OutputSettingsBatchPanel() -> impl IntoView {
    view! {
        <OutputSettingsPanel naming_template="face_{original}_{index}" template_help="Available variables: {original}, {index}, {timestamp}, {width}, {height}" />
    }
}

#[component]
pub fn OutputSettingsCsvPanel() -> impl IntoView {
    view! {
        <OutputSettingsPanel naming_template="{csv_name}" template_help="Available variables: {csv_name}, {original}, {index}, {timestamp}, {width}, {height}" />
    }
}

#[component]
fn OutputSettingsPanel(
    naming_template: &'static str,
    template_help: &'static str,
) -> impl IntoView {
    let settings = use_context::<AppState>()
        .expect("app state should be provided")
        .settings;
    view! {
        <div class="output-settings">
            <h3 class="collapsible-header">
                "Output Settings"
                <span class="collapse-icon">"▼"</span>
            </h3>
            <div class="collapsible-content">
                <div class="settings-grid">
                    <div class="setting-group">
                        <label for="outputFormat">"Output Format"</label>
                        <select id="outputFormat" prop:value=move || settings.get().output_format.clone() on:change=move |ev| {
                            let value = event_target_value(&ev);
                            settings.update(|s| s.output_format = value);
                        }>
                            <option value="png">"PNG (Lossless)"</option>
                            <option value="jpeg">"JPEG (Compressed)"</option>
                            <option value="webp">"WebP (Modern)"</option>
                        </select>
                    </div>
                    <div class="setting-group hidden" id="jpegQualityGroup">
                        <label for="jpegQuality">"JPEG Quality"</label>
                        <div class="quality-slider-container">
                            <input type="range" id="jpegQuality" min="1" max="100" prop:value=move || (settings.get().jpeg_quality * 100.0).round().to_string() on:input=move |ev| {
                                if let Ok(value) = event_target_value(&ev).parse::<f32>() {
                                    settings.update(|s| s.jpeg_quality = (value / 100.0).clamp(0.01, 1.0));
                                }
                            } />
                            <span class="quality-value" id="qualityValue">"85%"</span>
                        </div>
                    </div>
                    <div class="setting-group full-width">
                        <label for="namingTemplate">"Filename Template"</label>
                        <input type="text" id="namingTemplate" prop:value=move || {
                            let configured = settings.get().naming_template;
                            if configured.is_empty() { naming_template.to_string() } else { configured }
                        } placeholder=naming_template on:input=move |ev| {
                            settings.update(|s| s.naming_template = event_target_value(&ev));
                        } />
                        <div class="template-help">{template_help}</div>
                    </div>
                </div>
                <div class="download-options">
                    <div class="download-option">
                        <input type="checkbox" id="zipDownload" checked />
                        <label for="zipDownload">"Download as ZIP archive"</label>
                    </div>
                    <div class="download-option">
                        <input type="checkbox" id="individualDownload" />
                        <label for="individualDownload">"Show individual download buttons"</label>
                    </div>
                </div>
            </div>
        </div>
    }
}

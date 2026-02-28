use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessingSettings {
    pub output_width: u32,
    pub output_height: u32,
    pub face_height_pct: u8,
    pub vertical_offset_pct: i32,
    pub horizontal_offset_pct: i32,
    pub output_format: String,
    pub jpeg_quality: f32,
    pub naming_template: String,
    pub auto_color_correction: bool,
    pub exposure_adjustment: f32,
    pub contrast_adjustment: f32,
    pub sharpness: f32,
    pub skin_smoothing: f32,
    pub red_eye_removal: bool,
    pub background_blur: f32,
    pub min_confidence: f32,
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            output_width: 256,
            output_height: 256,
            face_height_pct: 70,
            vertical_offset_pct: 0,
            horizontal_offset_pct: 0,
            output_format: "png".to_string(),
            jpeg_quality: 0.85,
            naming_template: "face_{original}_{index}".to_string(),
            auto_color_correction: true,
            exposure_adjustment: 0.0,
            contrast_adjustment: 1.0,
            sharpness: 0.0,
            skin_smoothing: 0.0,
            red_eye_removal: false,
            background_blur: 0.0,
            min_confidence: 0.0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct AppState {
    pub settings: RwSignal<ProcessingSettings>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            settings: RwSignal::new(ProcessingSettings::default()),
        }
    }
}

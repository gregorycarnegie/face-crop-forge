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
    pub min_confidence: f32,
    pub zip_compress: bool,
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
            min_confidence: 0.5,
            zip_compress: false,
        }
    }
}

#[derive(Copy, Clone)]
pub struct AppState {
    pub settings: RwSignal<ProcessingSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: RwSignal::new(ProcessingSettings::default()),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}

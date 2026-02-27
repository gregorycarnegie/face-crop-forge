use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct BoundingBox {
    pub origin_x: f32,
    pub origin_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PixelBoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FaceCategory {
    pub score: f32,
    pub index: Option<u32>,
    pub category_name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FaceDetection {
    pub bounding_box: BoundingBox,
    pub score: Option<f32>,
    pub categories: Vec<FaceCategory>,
}

#[derive(Clone, Debug, Default)]
pub struct Landmark3D {
    pub x: f32,
    pub y: f32,
    pub z: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct FaceQuality {
    pub score: f32,
    pub level: String,
}

#[derive(Clone, Debug)]
pub enum FaceId {
    Number(i64),
    Text(String),
}

impl Default for FaceId {
    fn default() -> Self {
        Self::Number(0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct FaceBox {
    pub x_min: f32,
    pub y_min: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FaceData {
    pub id: FaceId,
    pub bbox: PixelBoundingBox,
    pub box_region: Option<FaceBox>,
    pub selected: bool,
    pub confidence: Option<f32>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub index: Option<u32>,
    pub quality: Option<FaceQuality>,
    pub score: Option<f32>,
    pub landmarks: Vec<Landmark3D>,
}

#[derive(Clone, Debug, Default)]
pub struct CropResult {
    pub bbox: Option<PixelBoundingBox>,
    pub face: Option<FaceData>,
    pub file_name: Option<String>,
    pub data_url: Option<String>,
    pub face_index: Option<u32>,
    pub face_id: Option<String>,
    pub source_image: Option<String>,
    pub filename: Option<String>,
    pub format: Option<String>,
    pub quality: Option<f32>,
    pub blob_url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum ProcessingStatus {
    Loaded,
    Processing,
    Processed,
    Error,
    Custom(String),
}

impl Default for ProcessingStatus {
    fn default() -> Self {
        Self::Loaded
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProcessorImageData {
    pub id: String,
    pub file_name: String,
    pub faces: Vec<FaceData>,
    pub results: Vec<CropResult>,
    pub selected: bool,
    pub processed: bool,
    pub status: Option<ProcessingStatus>,
    pub csv_output_name: Option<String>,
    pub page: Option<u32>,
    pub processed_at: Option<u64>,
    pub memory_cleaned_up: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub total_faces_detected: u64,
    pub images_processed: u64,
    pub successful_processing: u64,
    pub processing_times: Vec<f64>,
    pub start_time: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum PositioningMode {
    Center,
    RuleOfThirds,
    Custom,
    Other(String),
}

impl Default for PositioningMode {
    fn default() -> Self {
        Self::Center
    }
}

#[derive(Clone, Debug, Default)]
pub struct CropSettings {
    pub output_width: u32,
    pub output_height: u32,
    pub face_height_pct: u8,
    pub positioning_mode: PositioningMode,
    pub vertical_offset: i32,
    pub horizontal_offset: i32,
    pub output_format: String,
    pub jpeg_quality: f32,
    pub naming_template: String,
    pub format: Option<String>,
    pub quality: Option<f32>,
}

#[derive(Clone, Debug)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub enum ErrorSeverity {
    Error,
    Critical,
    Warning,
    Info,
}

#[derive(Clone, Debug, Default)]
pub struct ProcessingLogEntry {
    pub timestamp: String,
    pub message: String,
    pub kind: Option<LogLevel>,
}

#[derive(Clone, Debug, Default)]
pub struct ErrorLogEntry {
    pub timestamp: String,
    pub title: String,
    pub details: String,
    pub severity: Option<ErrorSeverity>,
}

#[derive(Clone, Debug, Default)]
pub struct ProcessingState {
    pub images: HashMap<String, ProcessorImageData>,
    pub settings: CropSettings,
    pub selected_images: Vec<String>,
}

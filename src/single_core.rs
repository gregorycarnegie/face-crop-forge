use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct SingleCoreState {
    pub all_face_ids: Vec<String>,
    pub selected_face_ids: HashSet<String>,
    pub rotation_degrees: i16,
    pub webcam_modal_open: bool,
    pub cameras: Vec<String>,
    pub active_camera_index: usize,
}

impl Default for SingleCoreState {
    fn default() -> Self {
        Self {
            all_face_ids: Vec::new(),
            selected_face_ids: HashSet::new(),
            rotation_degrees: 0,
            webcam_modal_open: false,
            cameras: vec!["Front Camera".to_string(), "Rear Camera".to_string()],
            active_camera_index: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleRuntimeState {
    pub running: bool,
    pub processing_time_ms: u64,
    pub status: String,
    pub logs: Vec<String>,
}

impl Default for SingleRuntimeState {
    fn default() -> Self {
        Self {
            running: false,
            processing_time_ms: 0,
            status: "Ready to load image".to_string(),
            logs: vec!["Ready to process image...".to_string()],
        }
    }
}

impl SingleRuntimeState {
    pub fn start(&mut self, status: impl Into<String>) {
        self.running = true;
        self.processing_time_ms = 0;
        self.status = status.into();
        self.push_log(self.status.clone());
    }

    pub fn complete(&mut self, processing_time_ms: u64, status: impl Into<String>) {
        self.running = false;
        self.processing_time_ms = processing_time_ms;
        self.status = status.into();
        self.push_log(self.status.clone());
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.push_log(self.status.clone());
    }

    pub fn push_log(&mut self, message: impl Into<String>) {
        self.logs.push(message.into());
        if self.logs.len() > 20 {
            let overflow = self.logs.len() - 20;
            self.logs.drain(0..overflow);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplaySize {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

impl SingleCoreState {
    pub fn new(face_ids: Vec<String>) -> Self {
        let selected_face_ids = face_ids.iter().cloned().collect();
        Self {
            all_face_ids: face_ids,
            selected_face_ids,
            rotation_degrees: 0,
            webcam_modal_open: false,
            cameras: vec!["Front Camera".to_string(), "Rear Camera".to_string()],
            active_camera_index: 0,
        }
    }

    pub fn set_faces(&mut self, face_ids: Vec<String>) {
        self.all_face_ids = face_ids.clone();
        self.selected_face_ids = face_ids.into_iter().collect();
    }

    pub fn toggle_face_selection(&mut self, face_id: &str) {
        if !self.selected_face_ids.remove(face_id) {
            self.selected_face_ids.insert(face_id.to_string());
        }
    }

    pub fn select_all_faces(&mut self) {
        self.selected_face_ids = self.all_face_ids.iter().cloned().collect();
    }

    pub fn select_none_faces(&mut self) {
        self.selected_face_ids.clear();
    }

    pub fn selected_count(&self) -> usize {
        self.selected_face_ids.len()
    }

    pub fn faces_count(&self) -> usize {
        self.all_face_ids.len()
    }

    /// Rotates by +/-90 style increments, normalized to [0, 359].
    /// Mirrors TS logic: `(angle + degrees + 360) % 360`.
    pub fn rotate_by(&mut self, delta_degrees: i16) {
        let raw = self.rotation_degrees as i32 + delta_degrees as i32 + 3600;
        self.rotation_degrees = (raw % 360) as i16;
    }

    /// Single-processor clears face selection after rotation before redetection.
    pub fn clear_faces_after_rotation(&mut self) {
        self.all_face_ids.clear();
        self.selected_face_ids.clear();
    }

    pub fn reset_rotation_after_reencode(&mut self) {
        self.rotation_degrees = 0;
    }

    pub fn open_webcam_modal(&mut self) {
        self.webcam_modal_open = true;
    }

    pub fn close_webcam_modal(&mut self) {
        self.webcam_modal_open = false;
    }

    pub fn active_camera_name(&self) -> &str {
        self.cameras
            .get(self.active_camera_index)
            .map(String::as_str)
            .unwrap_or("Unknown Camera")
    }

    pub fn switch_camera(&mut self) {
        if self.cameras.is_empty() {
            return;
        }
        self.active_camera_index = (self.active_camera_index + 1) % self.cameras.len();
    }
}

/// Port of single-page preview sizing behavior (`maxWidth=600`, `maxHeight=400`).
pub fn compute_display_size(
    image_width: f32,
    image_height: f32,
    rotation_degrees: i16,
    max_width: f32,
    max_height: f32,
) -> DisplaySize {
    let is_quarter_turn = rotation_degrees.rem_euclid(180) != 0;
    let (rotated_w, rotated_h) = if is_quarter_turn {
        (image_height, image_width)
    } else {
        (image_width, image_height)
    };

    let scale = (max_width / rotated_w).min(max_height / rotated_h).min(1.0);
    DisplaySize {
        width: rotated_w * scale,
        height: rotated_h * scale,
        scale,
    }
}

pub fn generate_face_filename(
    template: &str,
    original_name: &str,
    face_index_zero_based: usize,
    output_width: u32,
    output_height: u32,
    output_format: &str,
    timestamp_ms: u64,
) -> String {
    let base_name = strip_extension(original_name);
    let extension = if output_format.eq_ignore_ascii_case("jpeg") {
        "jpg"
    } else {
        output_format
    };

    template
        .replace("{original}", base_name)
        .replace("{index}", &(face_index_zero_based + 1).to_string())
        .replace("{timestamp}", &timestamp_ms.to_string())
        .replace("{width}", &output_width.to_string())
        .replace("{height}", &output_height.to_string())
        + "."
        + extension
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SingleExportPlan {
    pub filenames: Vec<String>,
}

pub fn build_export_plan(
    selected_face_ids: &HashSet<String>,
    template: &str,
    original_name: &str,
    output_width: u32,
    output_height: u32,
    output_format: &str,
    timestamp_ms: u64,
) -> SingleExportPlan {
    let mut ordered_ids = selected_face_ids.iter().cloned().collect::<Vec<_>>();
    ordered_ids.sort();

    let filenames = ordered_ids
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            generate_face_filename(
                template,
                original_name,
                idx,
                output_width,
                output_height,
                output_format,
                timestamp_ms,
            )
        })
        .collect::<Vec<_>>();

    SingleExportPlan { filenames }
}

fn strip_extension(filename: &str) -> &str {
    filename.rfind('.').map_or(filename, |idx| &filename[..idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_toggle_and_bulk_selection_work() {
        let mut state = SingleCoreState::new(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(state.selected_count(), 3);

        state.toggle_face_selection("b");
        assert_eq!(state.selected_count(), 2);
        assert!(!state.selected_face_ids.contains("b"));

        state.select_none_faces();
        assert_eq!(state.selected_count(), 0);

        state.select_all_faces();
        assert_eq!(state.selected_count(), 3);
    }

    #[test]
    fn rotation_normalizes_to_0_359() {
        let mut state = SingleCoreState::default();
        state.rotate_by(90);
        assert_eq!(state.rotation_degrees, 90);
        state.rotate_by(-90);
        assert_eq!(state.rotation_degrees, 0);
        state.rotate_by(-90);
        assert_eq!(state.rotation_degrees, 270);
    }

    #[test]
    fn display_size_swaps_dimensions_at_90_deg() {
        let no_rotation = compute_display_size(1200.0, 800.0, 0, 600.0, 400.0);
        let quarter_turn = compute_display_size(1200.0, 800.0, 90, 600.0, 400.0);
        assert!((no_rotation.width - 600.0).abs() < 0.01);
        assert!((no_rotation.height - 400.0).abs() < 0.01);
        assert!((quarter_turn.width - 266.66666).abs() < 0.1);
        assert!((quarter_turn.height - 400.0).abs() < 0.01);
    }

    #[test]
    fn face_filename_template_matches_single_processor_behavior() {
        let filename = generate_face_filename(
            "face_{original}_{index}_{width}x{height}_{timestamp}",
            "person.png",
            1,
            256,
            256,
            "jpeg",
            1234567890,
        );
        assert_eq!(filename, "face_person_2_256x256_1234567890.jpg");
    }

    #[test]
    fn export_filename_format_mapping_matches_legacy_behavior() {
        let png = generate_face_filename(
            "face_{original}_{index}",
            "portrait.jpeg",
            0,
            512,
            512,
            "png",
            1,
        );
        let jpg = generate_face_filename(
            "face_{original}_{index}",
            "portrait.jpeg",
            0,
            512,
            512,
            "jpeg",
            1,
        );
        let webp = generate_face_filename(
            "face_{original}_{index}",
            "portrait.jpeg",
            0,
            512,
            512,
            "webp",
            1,
        );

        assert_eq!(png, "face_portrait_1.png");
        assert_eq!(jpg, "face_portrait_1.jpg");
        assert_eq!(webp, "face_portrait_1.webp");
    }

    #[test]
    fn webcam_modal_and_camera_switching_work() {
        let mut state = SingleCoreState::default();
        assert!(!state.webcam_modal_open);
        assert_eq!(state.active_camera_name(), "Front Camera");

        state.open_webcam_modal();
        assert!(state.webcam_modal_open);

        state.switch_camera();
        assert_eq!(state.active_camera_name(), "Rear Camera");

        state.switch_camera();
        assert_eq!(state.active_camera_name(), "Front Camera");

        state.close_webcam_modal();
        assert!(!state.webcam_modal_open);
    }

    #[test]
    fn export_plan_is_stable_and_uses_template() {
        let selected = ["face_2".to_string(), "face_1".to_string()]
            .into_iter()
            .collect::<HashSet<_>>();
        let plan = build_export_plan(
            &selected,
            "face_{original}_{index}_{width}x{height}",
            "person.png",
            512,
            512,
            "png",
            0,
        );
        assert_eq!(plan.filenames.len(), 2);
        assert_eq!(plan.filenames[0], "face_person_1_512x512.png");
        assert_eq!(plan.filenames[1], "face_person_2_512x512.png");
    }

    #[test]
    fn runtime_tracks_status_time_and_bounded_logs() {
        let mut runtime = SingleRuntimeState::default();
        runtime.start("Detecting faces...");
        runtime.complete(42, "Detected 3 face(s)");
        assert!(!runtime.running);
        assert_eq!(runtime.processing_time_ms, 42);
        assert_eq!(runtime.status, "Detected 3 face(s)");

        for i in 0..30 {
            runtime.push_log(format!("log_{i}"));
        }
        assert_eq!(runtime.logs.len(), 20);
        assert_eq!(runtime.logs.first().map(String::as_str), Some("log_10"));
    }
}

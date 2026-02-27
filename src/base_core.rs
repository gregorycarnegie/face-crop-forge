use crate::crop_math::{CropRegion, FaceBox, PositioningMode};

#[derive(Clone, Debug, PartialEq)]
pub struct CoreSettings {
    pub naming_template: String,
    pub output_format: String,
    pub positioning_mode: PositioningMode,
    pub vertical_offset_pct: f32,
    pub horizontal_offset_pct: f32,
}

impl Default for CoreSettings {
    fn default() -> Self {
        Self {
            naming_template: "face_{original}_{index}".to_string(),
            output_format: "png".to_string(),
            positioning_mode: PositioningMode::Center,
            vertical_offset_pct: 0.0,
            horizontal_offset_pct: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessingStatistics {
    pub total_faces_detected: u64,
    pub images_processed: u64,
    pub successful_processing: u64,
    pub processing_times_ms: Vec<u64>,
    pub processing_start_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatsSummary {
    pub success_rate_pct: u32,
    pub avg_processing_time_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct BaseCore {
    pub settings: CoreSettings,
    pub stats: ProcessingStatistics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizePreset {
    LinkedIn,
    Passport,
    Instagram,
    IdCard,
    Avatar,
    Headshot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatVisibility {
    ShowJpegQuality,
    HideJpegQuality,
}

impl BaseCore {
    pub fn new(settings: CoreSettings) -> Self {
        Self {
            settings,
            stats: ProcessingStatistics::default(),
        }
    }

    pub fn generate_batch_filename(
        &self,
        original_name: &str,
        face_index: usize,
        width: u32,
        height: u32,
        timestamp: &str,
    ) -> String {
        let base_name = strip_extension(original_name);
        let ext = normalize_extension(&self.settings.output_format);
        self.settings
            .naming_template
            .replace("{original}", base_name)
            .replace("{index}", &face_index.to_string())
            .replace("{timestamp}", timestamp)
            .replace("{width}", &width.to_string())
            .replace("{height}", &height.to_string())
            + "."
            + ext
    }

    pub fn record_processing_start(&mut self, now_ms: u64) {
        self.stats.processing_start_ms = Some(now_ms);
    }

    pub fn record_processing_end(&mut self, now_ms: u64, success: bool, faces_detected: u64) {
        if let Some(start) = self.stats.processing_start_ms {
            let elapsed = now_ms.saturating_sub(start);
            self.stats.processing_times_ms.push(elapsed);

            // Keep last 50 timings (matches TS behavior).
            if self.stats.processing_times_ms.len() > 50 {
                let overflow = self.stats.processing_times_ms.len() - 50;
                self.stats.processing_times_ms.drain(0..overflow);
            }
        }

        self.stats.images_processed += 1;
        self.stats.total_faces_detected += faces_detected;
        if success {
            self.stats.successful_processing += 1;
        }
    }

    pub fn summary(&self) -> StatsSummary {
        let success_rate_pct = if self.stats.images_processed > 0 {
            ((self.stats.successful_processing as f64 / self.stats.images_processed as f64) * 100.0)
                .round() as u32
        } else {
            0
        };

        let avg_processing_time_ms = if self.stats.processing_times_ms.is_empty() {
            0
        } else {
            let sum: u64 = self.stats.processing_times_ms.iter().sum();
            (sum as f64 / self.stats.processing_times_ms.len() as f64).round() as u64
        };

        StatsSummary {
            success_rate_pct,
            avg_processing_time_ms,
        }
    }

    pub fn selected_count(selected_flags: &[bool]) -> usize {
        selected_flags.iter().filter(|selected| **selected).count()
    }

    /// Port of TS `calculateSmartCropPosition` geometry.
    pub fn calculate_smart_crop_position(
        &self,
        face: FaceBox,
        crop_width: f32,
        crop_height: f32,
        image_width: f32,
        image_height: f32,
    ) -> CropRegion {
        let face_center_x = face.x_min + (face.width / 2.0);
        let face_center_y = face.y_min + (face.height / 2.0);
        let v_offset = self.settings.vertical_offset_pct / 100.0;
        let h_offset = self.settings.horizontal_offset_pct / 100.0;

        let (target_x, target_y) = match self.settings.positioning_mode {
            PositioningMode::RuleOfThirds => {
                let eyes_y = face.y_min + face.height * 0.35;
                (
                    face_center_x + (h_offset * crop_width / 2.0),
                    eyes_y - (crop_height / 3.0),
                )
            }
            PositioningMode::Custom => (
                face_center_x + (h_offset * crop_width / 2.0),
                face_center_y + (v_offset * crop_height / 2.0),
            ),
            PositioningMode::Center => (face_center_x, face_center_y),
        };

        let crop_x = clamp(target_x - crop_width / 2.0, 0.0, image_width - crop_width);
        let crop_y = clamp(
            target_y - crop_height / 2.0,
            0.0,
            image_height - crop_height,
        );

        CropRegion {
            x: crop_x,
            y: crop_y,
            width: crop_width,
            height: crop_height,
        }
    }

    pub fn preview_summary_text(
        width: u32,
        height: u32,
        face_height_pct: u8,
        output_format: &str,
    ) -> String {
        format!(
            "{}×{}px, face at {}% height, {} format",
            width,
            height,
            face_height_pct,
            output_format.to_uppercase()
        )
    }

    pub fn aspect_ratio_text(width: u32, height: u32) -> String {
        let ratio = if height == 0 {
            0.0
        } else {
            width as f32 / height as f32
        };
        format!("{ratio:.2}:1 ratio")
    }

    pub fn show_advanced_positioning(positioning_mode: &PositioningMode) -> bool {
        matches!(positioning_mode, PositioningMode::Custom)
    }

    pub fn offset_text(value_pct: i32) -> String {
        format!("{value_pct}%")
    }

    pub fn toggle_aspect_ratio_lock(
        currently_locked: bool,
        width: u32,
        height: u32,
    ) -> (bool, Option<f32>) {
        let next_locked = !currently_locked;
        if next_locked && height != 0 {
            (true, Some(width as f32 / height as f32))
        } else {
            (next_locked, None)
        }
    }

    pub fn maintain_aspect_ratio(width: u32, height: u32, locked_ratio: f32) -> u32 {
        if height == 0 {
            return 0;
        }
        let current_ratio = width as f32 / height as f32;
        if (current_ratio - locked_ratio).abs() > 0.01 {
            ((width as f32) / locked_ratio).round() as u32
        } else {
            height
        }
    }

    pub fn size_preset_dimensions(preset: SizePreset) -> OutputDimensions {
        match preset {
            SizePreset::LinkedIn => OutputDimensions {
                width: 400,
                height: 400,
            },
            SizePreset::Passport => OutputDimensions {
                width: 413,
                height: 531,
            },
            SizePreset::Instagram => OutputDimensions {
                width: 1080,
                height: 1080,
            },
            SizePreset::IdCard => OutputDimensions {
                width: 332,
                height: 498,
            },
            SizePreset::Avatar => OutputDimensions {
                width: 512,
                height: 512,
            },
            SizePreset::Headshot => OutputDimensions {
                width: 600,
                height: 800,
            },
        }
    }

    pub fn format_visibility(output_format: &str) -> FormatVisibility {
        if output_format.eq_ignore_ascii_case("jpeg") {
            FormatVisibility::ShowJpegQuality
        } else {
            FormatVisibility::HideJpegQuality
        }
    }
}

fn strip_extension(filename: &str) -> &str {
    filename.rfind('.').map_or(filename, |idx| &filename[..idx])
}

fn normalize_extension(output_format: &str) -> &str {
    if output_format.eq_ignore_ascii_case("jpeg") {
        "jpg"
    } else {
        output_format
    }
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_template_variables_expand() {
        let settings = CoreSettings {
            naming_template: "face_{original}_{index}_{width}x{height}_{timestamp}".to_string(),
            output_format: "jpeg".to_string(),
            ..CoreSettings::default()
        };
        let core = BaseCore::new(settings);
        let filename =
            core.generate_batch_filename("person.png", 2, 256, 512, "2026-02-27T18-00-00");
        assert_eq!(filename, "face_person_2_256x512_2026-02-27T18-00-00.jpg");
    }

    #[test]
    fn statistics_tracks_success_rate_and_average_time() {
        let mut core = BaseCore::default();
        core.record_processing_start(1_000);
        core.record_processing_end(1_100, true, 2);
        core.record_processing_start(2_000);
        core.record_processing_end(2_300, false, 1);

        let summary = core.summary();
        assert_eq!(summary.success_rate_pct, 50);
        assert_eq!(summary.avg_processing_time_ms, 200);
        assert_eq!(core.stats.total_faces_detected, 3);
        assert_eq!(core.stats.images_processed, 2);
    }

    #[test]
    fn selected_count_matches_true_flags() {
        let selected = vec![true, false, true, true, false];
        assert_eq!(BaseCore::selected_count(&selected), 3);
    }

    #[test]
    fn smart_crop_custom_mode_offsets_target() {
        let settings = CoreSettings {
            positioning_mode: PositioningMode::Custom,
            horizontal_offset_pct: 20.0,
            vertical_offset_pct: 10.0,
            ..CoreSettings::default()
        };
        let core = BaseCore::new(settings);
        let face = FaceBox {
            x_min: 100.0,
            y_min: 100.0,
            width: 100.0,
            height: 120.0,
        };

        let custom = core.calculate_smart_crop_position(face, 200.0, 200.0, 1000.0, 1000.0);
        let center =
            BaseCore::default().calculate_smart_crop_position(face, 200.0, 200.0, 1000.0, 1000.0);
        assert!(custom.x > center.x);
        assert!(custom.y > center.y);
    }

    #[test]
    fn preview_and_ratio_text_match_ui_expectations() {
        assert_eq!(
            BaseCore::preview_summary_text(512, 512, 70, "png"),
            "512×512px, face at 70% height, PNG format"
        );
        assert_eq!(BaseCore::aspect_ratio_text(600, 800), "0.75:1 ratio");
    }

    #[test]
    fn aspect_ratio_lock_toggle_and_maintain_work() {
        let (locked, ratio) = BaseCore::toggle_aspect_ratio_lock(false, 600, 800);
        assert!(locked);
        assert_eq!(ratio, Some(0.75));
        assert_eq!(BaseCore::maintain_aspect_ratio(750, 800, 0.75), 1000);
    }

    #[test]
    fn size_preset_and_format_visibility_match_ts_values() {
        assert_eq!(
            BaseCore::size_preset_dimensions(SizePreset::Passport),
            OutputDimensions {
                width: 413,
                height: 531
            }
        );
        assert_eq!(
            BaseCore::format_visibility("jpeg"),
            FormatVisibility::ShowJpegQuality
        );
        assert_eq!(
            BaseCore::format_visibility("png"),
            FormatVisibility::HideJpegQuality
        );
        assert!(BaseCore::show_advanced_positioning(
            &PositioningMode::Custom
        ));
        assert!(!BaseCore::show_advanced_positioning(
            &PositioningMode::Center
        ));
        assert_eq!(BaseCore::offset_text(-15), "-15%");
    }
}

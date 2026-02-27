#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PositioningMode {
    Center,
    RuleOfThirds,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropSettings {
    pub output_width: f32,
    pub output_height: f32,
    pub face_height_pct: f32,
    pub positioning_mode: PositioningMode,
    pub vertical_offset_pct: f32,
    pub horizontal_offset_pct: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceBox {
    pub x_min: f32,
    pub y_min: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Port of the TypeScript crop region math used by `BaseFaceCropper.cropFace`.
pub fn compute_crop_region(
    image_width: f32,
    image_height: f32,
    face_box: FaceBox,
    settings: CropSettings,
) -> CropRegion {
    let face_height = settings.output_height * (settings.face_height_pct / 100.0);
    let scale = face_height / face_box.height;

    let crop_width = settings.output_width / scale;
    let crop_height = settings.output_height / scale;

    let mut center_x = face_box.x_min + (face_box.width / 2.0);
    let mut center_y = face_box.y_min + (face_box.height / 2.0);

    match settings.positioning_mode {
        PositioningMode::Center => {}
        PositioningMode::RuleOfThirds => {
            center_y = face_box.y_min + (face_box.height * 0.33);
        }
        PositioningMode::Custom => {
            center_x += (settings.horizontal_offset_pct / 100.0) * crop_width;
            center_y += (settings.vertical_offset_pct / 100.0) * crop_height;
        }
    }

    let crop_x = clamp(center_x - (crop_width / 2.0), 0.0, image_width - crop_width);
    let crop_y = clamp(
        center_y - (crop_height / 2.0),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_settings(mode: PositioningMode) -> CropSettings {
        CropSettings {
            output_width: 256.0,
            output_height: 256.0,
            face_height_pct: 70.0,
            positioning_mode: mode,
            vertical_offset_pct: 0.0,
            horizontal_offset_pct: 0.0,
        }
    }

    fn sample_face() -> FaceBox {
        FaceBox {
            x_min: 100.0,
            y_min: 80.0,
            width: 120.0,
            height: 140.0,
        }
    }

    #[test]
    fn center_mode_computes_expected_size() {
        let region = compute_crop_region(
            1000.0,
            1000.0,
            sample_face(),
            default_settings(PositioningMode::Center),
        );
        assert!((region.width - 200.0).abs() < 0.001);
        assert!((region.height - 200.0).abs() < 0.001);
    }

    #[test]
    fn rule_of_thirds_moves_crop_up() {
        let center = compute_crop_region(
            1000.0,
            1000.0,
            sample_face(),
            default_settings(PositioningMode::Center),
        );
        let thirds = compute_crop_region(
            1000.0,
            1000.0,
            sample_face(),
            default_settings(PositioningMode::RuleOfThirds),
        );
        assert!(thirds.y < center.y);
    }

    #[test]
    fn custom_offsets_shift_crop() {
        let mut settings = default_settings(PositioningMode::Custom);
        settings.horizontal_offset_pct = 20.0;
        settings.vertical_offset_pct = 10.0;

        let shifted = compute_crop_region(1000.0, 1000.0, sample_face(), settings);
        let center = compute_crop_region(
            1000.0,
            1000.0,
            sample_face(),
            default_settings(PositioningMode::Center),
        );

        assert!(shifted.x > center.x);
        assert!(shifted.y > center.y);
    }

    #[test]
    fn crop_region_clamps_to_image_bounds() {
        let face = FaceBox {
            x_min: 5.0,
            y_min: 5.0,
            width: 120.0,
            height: 140.0,
        };
        let mut settings = default_settings(PositioningMode::Custom);
        settings.horizontal_offset_pct = -200.0;
        settings.vertical_offset_pct = -200.0;

        let region = compute_crop_region(300.0, 300.0, face, settings);
        assert!(region.x >= 0.0);
        assert!(region.y >= 0.0);
    }
}

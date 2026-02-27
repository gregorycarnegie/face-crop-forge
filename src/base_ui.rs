#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizePresetName {
    LinkedIn,
    Passport,
    Instagram,
    IdCard,
    Avatar,
    Headshot,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

pub fn extract_file_name(file_path: &str) -> String {
    file_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(file_path)
        .to_string()
}

pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn processing_status_short(message: &str) -> String {
    message
        .split('.')
        .next()
        .unwrap_or(message)
        .trim()
        .to_string()
}

pub fn generate_image_id(timestamp_ms: u64, sequence: u32) -> String {
    format!("img_{timestamp_ms}_{:09x}", sequence)
}

pub fn find_matching_preset(width: u32, height: u32) -> SizePresetName {
    match (width, height) {
        (400, 400) => SizePresetName::LinkedIn,
        (413, 531) => SizePresetName::Passport,
        (1080, 1080) => SizePresetName::Instagram,
        (332, 498) => SizePresetName::IdCard,
        (512, 512) => SizePresetName::Avatar,
        (600, 800) => SizePresetName::Headshot,
        _ => SizePresetName::Custom,
    }
}

pub fn aspect_ratio_display(width: u32, height: u32) -> String {
    if width == 0 || height == 0 {
        return "0.00:1 ratio".to_string();
    }

    let ratio = width as f32 / height as f32;
    let ratio_text = if (ratio - 1.0).abs() < 0.01 {
        "1:1 (Square)".to_string()
    } else if (ratio - (4.0 / 3.0)).abs() < 0.01 {
        "4:3 (Standard)".to_string()
    } else if (ratio - (16.0 / 9.0)).abs() < 0.01 {
        "16:9 (Widescreen)".to_string()
    } else if (ratio - (3.0 / 4.0)).abs() < 0.01 {
        "3:4 (Portrait)".to_string()
    } else {
        format!("{ratio:.2}:1")
    };

    format!("{ratio_text} ratio")
}

pub fn preview_text(width: u32, height: u32, face_height_pct: u8, output_format: &str) -> String {
    format!(
        "{}×{}px, face at {}% height, {} format",
        width,
        height,
        face_height_pct,
        output_format.to_uppercase()
    )
}

pub fn toggle_theme(mode: ThemeMode) -> ThemeMode {
    match mode {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    }
}

pub fn theme_storage_value(mode: ThemeMode) -> Option<&'static str> {
    match mode {
        ThemeMode::Dark => Some("dark"),
        ThemeMode::Light => None,
    }
}

pub fn theme_from_storage(value: Option<&str>) -> ThemeMode {
    if matches!(value, Some("dark")) {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_extraction_matches_windows_and_unix_paths() {
        assert_eq!(extract_file_name("images/a.jpg"), "a.jpg");
        assert_eq!(extract_file_name(r"C:\tmp\b.png"), "b.png");
    }

    #[test]
    fn html_escape_covers_common_entities() {
        assert_eq!(
            escape_html(r#"<tag attr="x&y">ok</tag>"#),
            "&lt;tag attr=&quot;x&amp;y&quot;&gt;ok&lt;/tag&gt;"
        );
    }

    #[test]
    fn status_short_uses_text_before_first_period() {
        assert_eq!(
            processing_status_short("Loaded first page. Ready to process."),
            "Loaded first page"
        );
    }

    #[test]
    fn preset_and_aspect_helpers_match_typescript_behavior() {
        assert_eq!(find_matching_preset(400, 400), SizePresetName::LinkedIn);
        assert_eq!(find_matching_preset(123, 456), SizePresetName::Custom);
        assert_eq!(aspect_ratio_display(1080, 1080), "1:1 (Square) ratio");
        assert_eq!(aspect_ratio_display(1600, 900), "16:9 (Widescreen) ratio");
        assert_eq!(aspect_ratio_display(500, 333), "1.50:1 ratio");
    }

    #[test]
    fn preview_theme_and_image_id_are_stable() {
        assert_eq!(
            preview_text(256, 256, 70, "png"),
            "256×256px, face at 70% height, PNG format"
        );
        assert_eq!(generate_image_id(1000, 15), "img_1000_00000000f");
        assert_eq!(toggle_theme(ThemeMode::Light), ThemeMode::Dark);
        assert_eq!(theme_storage_value(ThemeMode::Light), None);
        assert_eq!(theme_storage_value(ThemeMode::Dark), Some("dark"));
        assert_eq!(theme_from_storage(Some("dark")), ThemeMode::Dark);
    }
}

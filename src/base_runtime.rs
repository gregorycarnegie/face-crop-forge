#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryIndicatorLevel {
    Hidden,
    Show,
    Warning,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryIndicator {
    pub images: usize,
    pub processed: usize,
    pub errors: usize,
    pub score: usize,
    pub level: MemoryIndicatorLevel,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryManagementMode {
    Aggressive,
    Auto,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCleanupEntry {
    pub id: String,
    pub processed: bool,
    pub processed_at_ms: Option<u64>,
    pub memory_cleaned_up: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageValidationConfig {
    pub max_file_size_bytes: u64,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
}

impl Default for ImageValidationConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 50 * 1024 * 1024,
            min_width: 50,
            min_height: 50,
            max_width: 8192,
            max_height: 8192,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageMeta<'a> {
    pub file_name: &'a str,
    pub mime_type: &'a str,
    pub file_size_bytes: u64,
    pub dimensions: Dimensions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetectionRetryPolicy {
    pub max_retries: u32,
    pub continue_on_error: bool,
    pub reduced_resolution: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectionAttempt {
    pub attempt: u32,
    pub delay_ms: u64,
    pub dimensions: Dimensions,
    pub success: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectionOutcome {
    pub image_id: String,
    pub attempts: Vec<DetectionAttempt>,
    pub success: bool,
    pub faces_detected: u32,
    pub final_dimensions: Dimensions,
    pub error_message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn build_memory_indicator(images: usize, processed: usize, errors: usize) -> MemoryIndicator {
    let score = images.saturating_mul(10) + processed.saturating_mul(5);
    let level = if score > 200 {
        MemoryIndicatorLevel::Critical
    } else if score > 100 {
        MemoryIndicatorLevel::Warning
    } else if images > 0 {
        MemoryIndicatorLevel::Show
    } else {
        MemoryIndicatorLevel::Hidden
    };

    MemoryIndicator {
        images,
        processed,
        errors,
        score,
        level,
        text: format!("Memory: {images} images, {processed} processed"),
    }
}

pub fn cleanup_candidate_ids(
    mode: MemoryManagementMode,
    entries: &[MemoryCleanupEntry],
    now_ms: u64,
) -> Vec<String> {
    let five_minutes_ms = 5 * 60 * 1000;
    entries
        .iter()
        .filter(|entry| entry.processed && !entry.memory_cleaned_up)
        .filter(|entry| match mode {
            MemoryManagementMode::Manual => false,
            MemoryManagementMode::Aggressive => true,
            MemoryManagementMode::Auto => entry
                .processed_at_ms
                .map(|processed_at| processed_at < now_ms.saturating_sub(five_minutes_ms))
                .unwrap_or(false),
        })
        .map(|entry| entry.id.clone())
        .collect()
}

pub fn retry_delay_ms(attempt: u32) -> u64 {
    // TS behavior: delay(1000 * attempt) for retry attempt N.
    (attempt as u64).saturating_mul(1000)
}

pub fn parse_max_retries(raw_value: Option<&str>) -> u32 {
    raw_value
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

pub fn reduced_resolution_dimensions(source: Dimensions, enabled: bool) -> Dimensions {
    if !enabled {
        return source;
    }

    Dimensions {
        width: ((source.width as f32) * 0.5).floor().max(1.0) as u32,
        height: ((source.height as f32) * 0.5).floor().max(1.0) as u32,
    }
}

pub fn validate_image_meta(
    meta: ImageMeta<'_>,
    config: ImageValidationConfig,
) -> Result<(), String> {
    if !meta.mime_type.starts_with("image/") {
        return Err("Invalid file type. Please select an image file.".to_string());
    }
    if meta.file_size_bytes > config.max_file_size_bytes {
        return Err("File too large. Maximum size is 50MB.".to_string());
    }
    if meta.dimensions.width < config.min_width || meta.dimensions.height < config.min_height {
        return Err("Image too small. Minimum size is 50x50 pixels.".to_string());
    }
    if meta.dimensions.width > config.max_width || meta.dimensions.height > config.max_height {
        return Err("Image too large. Maximum size is 8192x8192 pixels.".to_string());
    }
    Ok(())
}

pub fn model_detection_retry_outcome(
    image_id: &str,
    source_dimensions: Dimensions,
    policy: DetectionRetryPolicy,
) -> DetectionOutcome {
    let processed_dimensions =
        reduced_resolution_dimensions(source_dimensions, policy.reduced_resolution);
    let transient_failure = image_id
        .as_bytes()
        .iter()
        .fold(0u32, |acc, byte| acc.wrapping_add(*byte as u32))
        % 5
        == 0;
    let hard_failure = image_id.ends_with(".corrupt");

    let mut attempts = Vec::new();
    let mut success = false;
    let mut error_message = None;
    let max_attempt = policy.max_retries;

    for attempt in 0..=max_attempt {
        let this_success = if hard_failure {
            false
        } else if transient_failure {
            attempt > 0
        } else {
            true
        };

        attempts.push(DetectionAttempt {
            attempt,
            delay_ms: retry_delay_ms(attempt),
            dimensions: processed_dimensions,
            success: this_success,
        });

        if this_success {
            success = true;
            break;
        }
    }

    if !success {
        error_message = Some(format!(
            "Face detection failed after {} attempts",
            policy.max_retries + 1
        ));
    }

    DetectionOutcome {
        image_id: image_id.to_string(),
        attempts,
        success,
        faces_detected: if success { 2 } else { 0 },
        final_dimensions: processed_dimensions,
        error_message,
    }
}

pub fn scale_faces_after_reduced_resolution(faces: &[FaceRect], scale: f32) -> Vec<FaceRect> {
    faces
        .iter()
        .map(|face| FaceRect {
            x: face.x * scale,
            y: face.y * scale,
            width: face.width * scale,
            height: face.height * scale,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_indicator_thresholds_match_typescript_logic() {
        let hidden = build_memory_indicator(0, 0, 0);
        assert_eq!(hidden.level, MemoryIndicatorLevel::Hidden);

        let show = build_memory_indicator(3, 0, 0);
        assert_eq!(show.level, MemoryIndicatorLevel::Show);

        let warning = build_memory_indicator(9, 3, 0); // 105
        assert_eq!(warning.level, MemoryIndicatorLevel::Warning);

        let critical = build_memory_indicator(15, 11, 0); // 205
        assert_eq!(critical.level, MemoryIndicatorLevel::Critical);
    }

    #[test]
    fn cleanup_candidates_respect_modes_and_timestamps() {
        let now = 1_000_000;
        let entries = vec![
            MemoryCleanupEntry {
                id: "fresh".to_string(),
                processed: true,
                processed_at_ms: Some(now - 60_000),
                memory_cleaned_up: false,
            },
            MemoryCleanupEntry {
                id: "stale".to_string(),
                processed: true,
                processed_at_ms: Some(now - 600_000),
                memory_cleaned_up: false,
            },
            MemoryCleanupEntry {
                id: "unprocessed".to_string(),
                processed: false,
                processed_at_ms: Some(now - 600_000),
                memory_cleaned_up: false,
            },
            MemoryCleanupEntry {
                id: "already_cleaned".to_string(),
                processed: true,
                processed_at_ms: Some(now - 600_000),
                memory_cleaned_up: true,
            },
        ];

        let manual = cleanup_candidate_ids(MemoryManagementMode::Manual, &entries, now);
        assert!(manual.is_empty());

        let auto = cleanup_candidate_ids(MemoryManagementMode::Auto, &entries, now);
        assert_eq!(auto, vec!["stale".to_string()]);

        let aggressive = cleanup_candidate_ids(MemoryManagementMode::Aggressive, &entries, now);
        assert_eq!(aggressive, vec!["fresh".to_string(), "stale".to_string()]);
    }

    #[test]
    fn retry_and_reduced_resolution_helpers_match_typescript_rules() {
        assert_eq!(parse_max_retries(Some("2")), 2);
        assert_eq!(parse_max_retries(Some("bad")), 0);
        assert_eq!(retry_delay_ms(0), 0);
        assert_eq!(retry_delay_ms(3), 3000);

        let original = Dimensions {
            width: 1920,
            height: 1080,
        };
        assert_eq!(reduced_resolution_dimensions(original, false), original);
        assert_eq!(
            reduced_resolution_dimensions(original, true),
            Dimensions {
                width: 960,
                height: 540
            }
        );
    }

    #[test]
    fn image_validation_matches_typescript_limits() {
        let config = ImageValidationConfig::default();
        let valid = validate_image_meta(
            ImageMeta {
                file_name: "ok.jpg",
                mime_type: "image/jpeg",
                file_size_bytes: 1024,
                dimensions: Dimensions {
                    width: 512,
                    height: 512,
                },
            },
            config,
        );
        assert!(valid.is_ok());

        let invalid_type = validate_image_meta(
            ImageMeta {
                file_name: "bad.txt",
                mime_type: "text/plain",
                file_size_bytes: 100,
                dimensions: Dimensions {
                    width: 512,
                    height: 512,
                },
            },
            config,
        );
        assert!(invalid_type.is_err());

        let too_small = validate_image_meta(
            ImageMeta {
                file_name: "tiny.png",
                mime_type: "image/png",
                file_size_bytes: 100,
                dimensions: Dimensions {
                    width: 10,
                    height: 40,
                },
            },
            config,
        );
        assert!(too_small.is_err());
    }

    #[test]
    fn retry_outcome_succeeds_after_retry_for_transient_ids() {
        let policy = DetectionRetryPolicy {
            max_retries: 2,
            continue_on_error: true,
            reduced_resolution: true,
        };
        let source = Dimensions {
            width: 1200,
            height: 800,
        };
        let transient_id = (0..50)
            .map(|i| format!("img_{i}.jpg"))
            .find(|id| {
                let probe = model_detection_retry_outcome(id, source, policy);
                probe.success && probe.attempts.len() > 1 && !probe.attempts[0].success
            })
            .expect("expected at least one deterministic transient id");
        let out = model_detection_retry_outcome(&transient_id, source, policy);
        assert!(out.success);
        assert!(out.attempts.len() >= 2);
        assert!(!out.attempts[0].success);
        assert_eq!(
            out.final_dimensions,
            Dimensions {
                width: 600,
                height: 400
            }
        );
    }

    #[test]
    fn retry_outcome_fails_for_corrupt_suffix() {
        let policy = DetectionRetryPolicy {
            max_retries: 1,
            continue_on_error: false,
            reduced_resolution: false,
        };
        let out = model_detection_retry_outcome(
            "img.corrupt",
            Dimensions {
                width: 1200,
                height: 800,
            },
            policy,
        );
        assert!(!out.success);
        assert_eq!(out.attempts.len(), 2);
        assert!(out.error_message.is_some());
    }

    #[test]
    fn reduced_resolution_face_scaling_matches_typescript_mapping() {
        let reduced = vec![FaceRect {
            x: 20.0,
            y: 30.0,
            width: 100.0,
            height: 120.0,
        }];
        let scaled = scale_faces_after_reduced_resolution(&reduced, 2.0);
        assert_eq!(
            scaled[0],
            FaceRect {
                x: 40.0,
                y: 60.0,
                width: 200.0,
                height: 240.0
            }
        );
    }
}

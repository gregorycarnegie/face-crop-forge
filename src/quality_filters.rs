#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityLevel {
    Unknown,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceQuality {
    pub score: f32,
    pub level: QualityLevel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityFilterSettings {
    pub min_confidence: f32,
    pub min_quality_score: f32,
    pub min_quality_level: QualityLevel,
}

impl Default for QualityFilterSettings {
    fn default() -> Self {
        Self {
            min_confidence: 0.0,
            min_quality_score: 0.0,
            min_quality_level: QualityLevel::Unknown,
        }
    }
}

/// Port of TypeScript blur quality thresholds from `calculateFaceQuality`.
pub fn classify_blur_quality(laplacian_variance: f32) -> FaceQuality {
    if laplacian_variance > 1000.0 {
        FaceQuality {
            score: laplacian_variance,
            level: QualityLevel::High,
        }
    } else if laplacian_variance > 300.0 {
        FaceQuality {
            score: laplacian_variance,
            level: QualityLevel::Medium,
        }
    } else {
        FaceQuality {
            score: laplacian_variance.max(0.0),
            level: QualityLevel::Low,
        }
    }
}

pub fn passes_confidence_filter(confidence: f32, min_confidence: f32) -> bool {
    confidence >= min_confidence
}

pub fn passes_quality_filter(
    quality: Option<FaceQuality>,
    settings: QualityFilterSettings,
) -> bool {
    match quality {
        Some(q) => q.score >= settings.min_quality_score && q.level >= settings.min_quality_level,
        None => {
            settings.min_quality_level == QualityLevel::Unknown && settings.min_quality_score <= 0.0
        }
    }
}

pub fn is_face_accepted(
    confidence: f32,
    quality: Option<FaceQuality>,
    settings: QualityFilterSettings,
) -> bool {
    passes_confidence_filter(confidence, settings.min_confidence)
        && passes_quality_filter(quality, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_thresholds_match_typescript_logic() {
        assert_eq!(classify_blur_quality(1200.0).level, QualityLevel::High);
        assert_eq!(classify_blur_quality(350.0).level, QualityLevel::Medium);
        assert_eq!(classify_blur_quality(300.0).level, QualityLevel::Low);
        assert_eq!(classify_blur_quality(50.0).level, QualityLevel::Low);
    }

    #[test]
    fn confidence_filter_works() {
        assert!(passes_confidence_filter(0.9, 0.8));
        assert!(!passes_confidence_filter(0.79, 0.8));
    }

    #[test]
    fn quality_filter_level_and_score_are_enforced() {
        let settings = QualityFilterSettings {
            min_confidence: 0.0,
            min_quality_score: 300.0,
            min_quality_level: QualityLevel::Medium,
        };
        let medium = classify_blur_quality(350.0);
        let low = classify_blur_quality(250.0);

        assert!(passes_quality_filter(Some(medium), settings));
        assert!(!passes_quality_filter(Some(low), settings));
    }

    #[test]
    fn combined_acceptance_checks_all_filters() {
        let settings = QualityFilterSettings {
            min_confidence: 0.85,
            min_quality_score: 300.0,
            min_quality_level: QualityLevel::Medium,
        };
        let quality = classify_blur_quality(1200.0);

        assert!(is_face_accepted(0.9, Some(quality), settings));
        assert!(!is_face_accepted(0.8, Some(quality), settings));
        assert!(!is_face_accepted(
            0.9,
            Some(classify_blur_quality(100.0)),
            settings
        ));
    }
}

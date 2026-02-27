#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

fn clamp_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0).round() as u8
}

/// Exposure in stops: -2.0..+2.0 (TS slider range).
pub fn apply_exposure(px: Rgba, exposure_stops: f32) -> Rgba {
    let factor = 2.0_f32.powf(exposure_stops);
    Rgba {
        r: clamp_u8(px.r as f32 * factor),
        g: clamp_u8(px.g as f32 * factor),
        b: clamp_u8(px.b as f32 * factor),
        a: px.a,
    }
}

/// Contrast scalar: 0.5..2.0 (TS slider range), midpoint around 128.
pub fn apply_contrast(px: Rgba, contrast: f32) -> Rgba {
    let apply = |c: u8| clamp_u8(((c as f32 - 128.0) * contrast) + 128.0);
    Rgba {
        r: apply(px.r),
        g: apply(px.g),
        b: apply(px.b),
        a: px.a,
    }
}

/// Lightweight sharpness approximation used for deterministic core tests.
/// Positive values (>0) increase local contrast around midpoint.
pub fn apply_sharpness_scalar(px: Rgba, sharpness: f32) -> Rgba {
    let factor = 1.0 + (sharpness * 0.1);
    apply_contrast(px, factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_increases_and_decreases_brightness() {
        let px = Rgba {
            r: 100,
            g: 120,
            b: 140,
            a: 255,
        };
        let brighter = apply_exposure(px, 1.0);
        let darker = apply_exposure(px, -1.0);
        assert!(brighter.r > px.r && brighter.g > px.g && brighter.b > px.b);
        assert!(darker.r < px.r && darker.g < px.g && darker.b < px.b);
    }

    #[test]
    fn contrast_preserves_midpoint_and_stretches_extremes() {
        let mid = Rgba {
            r: 128,
            g: 128,
            b: 128,
            a: 255,
        };
        let out_mid = apply_contrast(mid, 2.0);
        assert_eq!(out_mid.r, 128);

        let px = Rgba {
            r: 100,
            g: 150,
            b: 200,
            a: 255,
        };
        let out = apply_contrast(px, 1.5);
        assert!(out.r < px.r);
        assert!(out.b > px.b);
    }

    #[test]
    fn sharpness_scalar_uses_contrast_style_boost() {
        let px = Rgba {
            r: 90,
            g: 128,
            b: 210,
            a: 255,
        };
        let out = apply_sharpness_scalar(px, 2.0);
        assert!(out.r < px.r);
        assert_eq!(out.g, px.g);
        assert!(out.b > px.b);
    }
}

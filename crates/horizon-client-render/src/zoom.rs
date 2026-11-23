//! Horizon's built-in zoom (`Pvp::zoom`): the pure rules the game applies
//! while the zoom key is held. A native mod's zoom (`native_zoom`) still
//! takes precedence; this one is the fallback when no mod zooms.

/// The zoom factors the level setting and the wheel can reach.
pub const LEVELS: (u8, u8) = (2, 10);
/// The factor the wheel may take it to while the key is held.
pub const MAX_WHEEL: u8 = 30;

/// The field-of-view divisor for a level: level 4 is a 4x zoom.
pub fn factor(level: u8) -> f32 {
    f32::from(level.clamp(1, MAX_WHEEL))
}

/// One wheel notch while zooming: up (positive) zooms in, down out. The
/// result stays between 1 (no zoom) and `MAX_WHEEL`.
pub fn wheel(level: u8, notches: i32) -> u8 {
    let next = i32::from(level) + notches;
    next.clamp(1, i32::from(MAX_WHEEL)) as u8
}

/// Mouse turning while zoomed: divided by the factor so the view does not
/// whip across the screen; undivided when the setting is off.
pub fn turn_scale(factor: f32, sensitivity: bool) -> f64 {
    if sensitivity && factor > 1.0 {
        1.0 / f64::from(factor)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_is_its_factor() {
        assert_eq!(factor(4), 4.0);
        assert_eq!(factor(0), 1.0);
        assert_eq!(factor(200), f32::from(MAX_WHEEL));
    }
    #[test]
    fn the_wheel_zooms_in_and_out_within_bounds() {
        assert_eq!(wheel(4, 1), 5);
        assert_eq!(wheel(4, -2), 2);
        assert_eq!(wheel(1, -5), 1);
        assert_eq!(wheel(MAX_WHEEL, 9), MAX_WHEEL);
    }
    #[test]
    fn turning_slows_with_the_zoom_only_when_asked() {
        assert_eq!(turn_scale(4.0, true), 0.25);
        assert_eq!(turn_scale(4.0, false), 1.0);
        assert_eq!(turn_scale(1.0, true), 1.0);
    }
}

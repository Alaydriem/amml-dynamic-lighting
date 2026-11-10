use crate::remesh_plan::HeldLight;

/// The extra block light a held light gives at a block position.
pub struct LightFalloff;

impl LightFalloff {
    /// The block light (0 to 15) a light of `level` at `source` (in steps of 1/`HeldLight::STEPS` block) gives at
    /// the block `pos`, measured to the block centre. `round` fades by straight-line distance; otherwise by block
    /// steps (taxicab distance), as a placed torch does. Both are rounded to whole blocks.
    pub fn boost(level: u8, round: bool, source: [i64; 3], pos: [i32; 3]) -> u8 {
        if level == 0 {
            return 0;
        }
        let steps = HeldLight::STEPS;
        let centre = |i: usize| i64::from(pos[i]) * steps + steps / 2;
        let d = [0, 1, 2].map(|i| centre(i).saturating_sub(source[i]));
        let reach = i64::from(level);
        // fast reject: anything `level` blocks or more away on one axis gets nothing in either mode
        if d.iter().any(|c| c.unsigned_abs() >= reach.unsigned_abs() * steps.unsigned_abs()) {
            return 0;
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let distance = if round {
            (((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f64).sqrt() / steps as f64).round() as i64
        } else {
            ((d[0].abs() + d[1].abs() + d[2].abs()) as f64 / steps as f64).round() as i64
        };
        u8::try_from((reach - distance).clamp(0, 15)).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::LightFalloff;
    use crate::remesh_plan::HeldLight;

    fn at(block: [i32; 3]) -> [i64; 3] {
        HeldLight::centred(block, 1).steps()
    }

    #[test]
    fn full_level_at_the_source() {
        assert_eq!(LightFalloff::boost(14, true, at([0, 64, 0]), [0, 64, 0]), 14);
        assert_eq!(LightFalloff::boost(14, false, at([0, 64, 0]), [0, 64, 0]), 14);
    }

    #[test]
    fn taxicab_matches_a_placed_torch() {
        assert_eq!(LightFalloff::boost(14, false, at([0, 0, 0]), [1, 1, 0]), 12);
        assert_eq!(LightFalloff::boost(14, false, at([0, 0, 0]), [13, 0, 0]), 1);
        assert_eq!(LightFalloff::boost(14, false, at([0, 0, 0]), [14, 0, 0]), 0);
    }

    #[test]
    fn round_uses_straight_line_distance() {
        // sqrt(2) rounds to 1
        assert_eq!(LightFalloff::boost(14, true, at([0, 0, 0]), [1, 1, 0]), 13);
        assert_eq!(LightFalloff::boost(14, true, at([0, 0, 0]), [5, 5, 5]), 5);
    }

    #[test]
    fn a_quarter_step_moves_the_edge() {
        // centred in block 0: block 3 is 3 blocks away; at x = 1.25 it is 2.25 (rounds to 2) and block -3 is 3.75
        let centred = HeldLight::at([0.5, 0.5, 0.5], 10).steps();
        let forward = HeldLight::at([1.25, 0.5, 0.5], 10).steps();
        assert_eq!(LightFalloff::boost(10, true, centred, [3, 0, 0]), 7);
        assert_eq!(LightFalloff::boost(10, true, forward, [3, 0, 0]), 8);
        assert_eq!(LightFalloff::boost(10, true, forward, [-3, 0, 0]), 6);
    }

    #[test]
    fn zero_level_and_far_blocks_get_nothing() {
        assert_eq!(LightFalloff::boost(0, true, at([0, 0, 0]), [0, 0, 0]), 0);
        assert_eq!(LightFalloff::boost(15, true, at([0, 0, 0]), [0, 40, 0]), 0);
        assert_eq!(LightFalloff::boost(15, true, at([i32::MAX, 0, 0]), [i32::MIN, 0, 0]), 0);
    }
}

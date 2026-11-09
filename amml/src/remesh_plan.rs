/// One held light: the block it is in, where in that block (quarter blocks, 0 to 3 on each axis), and its level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldLight {
    pub source: [i32; 3],
    pub fine: [u8; 3],
    pub level: u8,
}

impl HeldLight {
    /// Steps per block of the light position. The light moves (and the terrain near it is rebuilt) each time the
    /// player crosses a quarter block, not only a whole block, so the light follows more smoothly.
    pub const STEPS: i64 = 4;

    /// A light at the centre of a block.
    pub fn centred(source: [i32; 3], level: u8) -> Self {
        Self { source, fine: [2, 2, 2], level }
    }

    /// A light at a world position.
    pub fn at(position: [f32; 3], level: u8) -> Self {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let steps = position.map(|c| (f64::from(c) * Self::STEPS as f64).floor() as i64);
        Self::from_steps(steps, level)
    }

    /// A light from a position in steps (quarter blocks).
    pub fn from_steps(steps: [i64; 3], level: u8) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let source = steps.map(|s| s.div_euclid(Self::STEPS).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let fine = steps.map(|s| s.rem_euclid(Self::STEPS) as u8);
        Self { source, fine, level }
    }

    /// The position in steps (quarter blocks).
    pub fn steps(&self) -> [i64; 3] {
        [0, 1, 2].map(|i| i64::from(self.source[i]) * Self::STEPS + i64::from(self.fine[i]))
    }
}

/// The subchunks whose meshes must be rebuilt when the held light changes from `old` to `new`: every subchunk that
/// either light reaches. Coordinates are subchunk indices (block >> 4).
pub struct RemeshPlan;

impl RemeshPlan {
    pub fn subchunks(old: Option<HeldLight>, new: Option<HeldLight>) -> Vec<[i32; 3]> {
        let mut out: Vec<[i32; 3]> = Vec::new();
        for light in [old, new].into_iter().flatten() {
            if light.level == 0 {
                continue;
            }
            // as `boxes`
            let reach = i32::from(light.level);
            let lo = light.source.map(|c| c.saturating_sub(reach).div_euclid(16));
            let hi = light.source.map(|c| c.saturating_add(reach).div_euclid(16));
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        if !out.contains(&[x, y, z]) {
                            out.push([x, y, z]);
                        }
                    }
                }
            }
        }
        out
    }

    /// The block boxes (inclusive min, max) the old and the new light reach: `source +/- level`. (A light anywhere in
    /// its block gives nothing at `level` blocks or more on one axis; one block more covers the fine offset.)
    pub fn boxes(old: Option<HeldLight>, new: Option<HeldLight>) -> Vec<([i32; 3], [i32; 3])> {
        let mut out: Vec<([i32; 3], [i32; 3])> = Vec::new();
        for light in [old, new].into_iter().flatten() {
            if light.level == 0 {
                continue;
            }
            let reach = i32::from(light.level);
            let found = (light.source.map(|c| c.saturating_sub(reach)), light.source.map(|c| c.saturating_add(reach)));
            if !out.contains(&found) {
                out.push(found);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{HeldLight, RemeshPlan};

    #[test]
    fn nothing_without_a_light() {
        assert!(RemeshPlan::subchunks(None, None).is_empty());
        assert!(RemeshPlan::subchunks(Some(HeldLight::centred([0, 0, 0], 0)), None).is_empty());
    }

    #[test]
    fn a_full_light_reaches_at_most_three_per_axis() {
        let light = HeldLight::centred([8, 72, 8], 15);
        // 8 +/- 14 spans subchunks -1..=1 on each axis
        assert_eq!(RemeshPlan::subchunks(None, Some(light)).len(), 27);
    }

    #[test]
    fn a_small_light_in_the_middle_stays_in_one_subchunk() {
        let light = HeldLight::centred([8, 8, 8], 3);
        assert_eq!(RemeshPlan::subchunks(Some(light), None), vec![[0, 0, 0]]);
    }

    #[test]
    fn old_and_new_are_joined_without_repeats() {
        let old = HeldLight::centred([8, 8, 8], 3);
        let new = HeldLight::centred([9, 8, 8], 3);
        assert_eq!(RemeshPlan::subchunks(Some(old), Some(new)), vec![[0, 0, 0]]);
    }

    #[test]
    fn boxes_cover_the_reach_of_each_light_once() {
        let old = HeldLight::centred([0, 64, 0], 15);
        let new = HeldLight::centred([1, 64, 0], 3);
        assert_eq!(
            RemeshPlan::boxes(Some(old), Some(new)),
            vec![([-15, 49, -15], [15, 79, 15]), ([-2, 61, -3], [4, 67, 3])]
        );
        assert_eq!(RemeshPlan::boxes(Some(new), Some(new)).len(), 1);
        assert!(RemeshPlan::boxes(Some(HeldLight::centred([0, 0, 0], 0)), None).is_empty());
    }

    #[test]
    fn negative_coordinates_floor() {
        let light = HeldLight::centred([-8, 8, 8], 1);
        assert_eq!(RemeshPlan::subchunks(None, Some(light)), vec![[-1, 0, 0]]);
    }
}

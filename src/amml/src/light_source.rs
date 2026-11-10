use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering};

use crate::light_falloff::LightFalloff;
use crate::remesh_plan::HeldLight;
use crate::vv_emitter::{HandEmitter, VvEmitter};

/// What the Vibrant Visuals static light pass does for one subchunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VvPlan {
    /// Run it as the game does.
    Normal,
    /// Run it with the held block added as an emitter (the game gives it its own colour and falloff).
    Emit(HandEmitter),
    /// Skip it, so the boosted vertex light stays (no hand block, or the emitter entries are missing).
    Skip,
}

/// The held light as the mesh worker threads see it. The game thread publishes it; the light getter detour reads
/// it on any thread. A sequence counter keeps a reader from mixing an old and a new position (a seqlock): the
/// writer makes it odd while it writes, and a reader retries when it saw an odd or changed value.
pub struct LightSource {
    seq: AtomicU32,
    level: AtomicU8,
    round: AtomicBool,
    x: AtomicI32,
    y: AtomicI32,
    z: AtomicI32,
    /// The held item's `Block const*` (0 when it places no block).
    block: AtomicUsize,
}

static SOURCE: LightSource = LightSource::new();

impl LightSource {
    const fn new() -> Self {
        Self {
            seq: AtomicU32::new(0),
            level: AtomicU8::new(0),
            round: AtomicBool::new(true),
            x: AtomicI32::new(0),
            y: AtomicI32::new(0),
            z: AtomicI32::new(0),
            block: AtomicUsize::new(0),
        }
    }

    pub fn get() -> &'static Self {
        &SOURCE
    }

    /// Game thread only (one writer).
    pub fn publish(&self, light: Option<HeldLight>, block: usize, round: bool) {
        let light = light.unwrap_or(HeldLight::centred([0, 0, 0], 0));
        // the position in steps; world coordinates (at most 30 million) times 4 fit an i32
        #[allow(clippy::cast_possible_truncation)]
        let steps = light.steps().map(|s| s.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
        self.seq.fetch_add(1, Ordering::AcqRel);
        self.x.store(steps[0], Ordering::Relaxed);
        self.y.store(steps[1], Ordering::Relaxed);
        self.z.store(steps[2], Ordering::Relaxed);
        self.round.store(round, Ordering::Relaxed);
        self.level.store(light.level, Ordering::Relaxed);
        self.block.store(block, Ordering::Relaxed);
        self.seq.fetch_add(1, Ordering::AcqRel);
    }

    /// A consistent copy of the light; `None` when there is no light.
    pub fn read(&self) -> Option<(HeldLight, bool)> {
        self.read_with_block().map(|(light, round, _)| (light, round))
    }

    /// As `read`, with the held block pointer.
    pub fn read_with_block(&self) -> Option<(HeldLight, bool, usize)> {
        for _ in 0..8 {
            let before = self.seq.load(Ordering::Acquire);
            if before % 2 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let level = self.level.load(Ordering::Relaxed);
            let steps =
                [self.x.load(Ordering::Relaxed), self.y.load(Ordering::Relaxed), self.z.load(Ordering::Relaxed)]
                    .map(i64::from);
            let round = self.round.load(Ordering::Relaxed);
            let block = self.block.load(Ordering::Relaxed);
            if self.seq.load(Ordering::Acquire) == before {
                return (level > 0).then_some((HeldLight::from_steps(steps, level), round, block));
            }
        }
        None
    }

    /// The new block-light byte for a getter result at `pos`: `block`, or the held light's boost when that is
    /// higher. Called from the getter detour on any thread.
    pub fn apply(&self, pos: [i32; 3], block: u8) -> u8 {
        // fast exit while no light is held: one load, no seqlock read
        if self.level.load(Ordering::Relaxed) == 0 {
            return block;
        }
        let Some((light, round)) = self.read() else { return block };
        LightFalloff::boost(light.level, round, light.steps(), pos).max(block)
    }

    /// The new light pair (`[sky, block]`) for an entity light lookup at `pos`: the block byte raised as by `apply`
    /// when the call is on the draw thread, otherwise `pair` unchanged. The sky byte never changes.
    pub fn raise_pair(&self, pair: [u8; 2], pos: [i32; 3], on_draw_thread: bool) -> [u8; 2] {
        if !on_draw_thread {
            return pair;
        }
        [pair[0], self.apply(pos, pair[1])]
    }

    /// True when the held light reaches a block of the subchunk at `subchunk` (subchunk coordinates). The Vibrant
    /// Visuals static light pass is skipped for these subchunks. Called on the mesh threads.
    pub fn reaches_subchunk(&self, subchunk: [i32; 3]) -> bool {
        let Some((light, _)) = self.read() else { return false };
        let level = i32::from(light.level);
        (0..3).all(|axis| {
            let low = (light.source[axis] - level).div_euclid(16);
            let high = (light.source[axis] + level).div_euclid(16);
            (low..=high).contains(&subchunk[axis])
        })
    }

    /// What the Vibrant Visuals static light pass does for the subchunk at `subchunk` (`variant` 0 is the mesher's
    /// pass). With a hand block and the emitter entries, every subchunk within one of the light's subchunk gets the
    /// hand emitter; otherwise the subchunks the light reaches are skipped. Called on the mesh threads.
    pub fn vv_plan(&self, subchunk: [i32; 3], variant: u64) -> VvPlan {
        let Some((light, _, block)) = self.read_with_block() else { return VvPlan::Normal };
        if block != 0 && variant & 0xFFFF == 0 && VvEmitter::ready() {
            let hand = light.source.map(|value| value.div_euclid(16));
            if (0..3).all(|axis| (subchunk[axis] - hand[axis]).abs() <= 1) {
                return VvPlan::Emit(HandEmitter { block, position: light.source, emission: light.level });
            }
            return VvPlan::Normal;
        }
        if self.reaches_subchunk(subchunk) { VvPlan::Skip } else { VvPlan::Normal }
    }
}

#[cfg(test)]
mod tests {
    use super::LightSource;
    use crate::remesh_plan::HeldLight;

    #[test]
    fn raise_pair_off_draw_thread_is_unchanged() {
        let source = LightSource::new();
        source.publish(Some(HeldLight::centred([0, 0, 0], 14)), 0, false);
        assert_eq!(source.raise_pair([15, 2], [0, 0, 0], false), [15, 2]);
    }

    #[test]
    fn raise_pair_raises_block_in_reach_only() {
        let source = LightSource::new();
        assert_eq!(source.raise_pair([0, 2], [0, 0, 0], true), [0, 2]); // no light held
        source.publish(Some(HeldLight::centred([0, 0, 0], 14)), 0, false);
        assert_eq!(source.raise_pair([0, 2], [0, 0, 0], true), [0, 14]);
        assert_eq!(source.raise_pair([0, 2], [0, 0, 3], true), [0, 11]);
        assert_eq!(source.raise_pair([0, 2], [0, 0, 20], true), [0, 2]); // out of reach
    }

    #[test]
    fn raise_pair_keeps_brighter_value() {
        let source = LightSource::new();
        source.publish(Some(HeldLight::centred([0, 0, 0], 14)), 0, false);
        assert_eq!(source.raise_pair([0, 15], [0, 0, 3], true), [0, 15]);
    }

    #[test]
    fn raise_pair_never_changes_sky() {
        let source = LightSource::new();
        source.publish(Some(HeldLight::centred([0, 0, 0], 14)), 0, false);
        assert_eq!(source.raise_pair([7, 0], [0, 0, 0], true)[0], 7);
    }

    #[test]
    fn publish_then_apply() {
        let source = LightSource::new();
        assert_eq!(source.apply([0, 0, 0], 2), 2);
        source.publish(Some(HeldLight::centred([0, 0, 0], 14)), 0, false);
        assert_eq!(source.apply([0, 0, 0], 2), 14);
        assert_eq!(source.apply([0, 0, 3], 2), 11);
        assert_eq!(source.apply([0, 0, 3], 13), 13);
        source.publish(None, 0, false);
        assert_eq!(source.apply([0, 0, 0], 2), 2);
    }

    #[test]
    fn reaches_the_subchunks_in_light_range_only() {
        let source = LightSource::new();
        assert!(!source.reaches_subchunk([0, 0, 0]));
        source.publish(Some(HeldLight::centred([2, 64, 15], 14)), 0, false);
        assert!(source.reaches_subchunk([0, 4, 0]));
        assert!(source.reaches_subchunk([-1, 4, 1]));
        assert!(source.reaches_subchunk([0, 3, 0]));
        assert!(!source.reaches_subchunk([2, 4, 0]));
        assert!(!source.reaches_subchunk([0, 5, 0]));
        assert!(!source.reaches_subchunk([0, 4, 2]));
    }
}

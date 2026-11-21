use amml_api::{LightFalloff, LightSource, Vec3};

use crate::held_light_table::HeldLightTable;
use crate::lighting_settings::LightingConfig;
use crate::structs::HeldLight;

/// Picks the one light the player gives off from what the two hands hold.
pub struct LightPicker;

impl LightPicker {
    /// The light at `position`: the brighter hand gives it (the main hand on a tie). An item's level is the light
    /// of the block it places, or the exception level of an item that places no block (by name). With a level
    /// override (1 to 15) a light item lights at that level; other items never give light. `None` while the light
    /// is switched off (`enabled`, or the toggle key), without a position, or when no hand gives light.
    pub fn pick(
        main: Option<&HeldLight>,
        off: Option<&HeldLight>,
        settings: &LightingConfig,
        toggled_on: bool,
        position: Option<Vec3>,
    ) -> Option<LightSource> {
        if !settings.enabled || !toggled_on {
            return None;
        }
        let position = position?;
        let (main_level, off_level) = (Self::level(main), Self::level(off));
        let (held, item_level) = if off_level > main_level { (off, off_level) } else { (main, main_level) };
        if item_level == 0 {
            return None;
        }
        let level = if settings.level_override > 0 { settings.level_override.min(15) } else { item_level };
        let falloff = if settings.round { LightFalloff::Round } else { LightFalloff::Taxicab };
        Some(LightSource { position, level, falloff, block: held.and_then(|held| held.block) })
    }

    /// The light level of one hand (0 to 15).
    pub fn level(held: Option<&HeldLight>) -> u8 {
        let Some(held) = held else { return 0 };
        match held.emission {
            Some(emission) if emission > 0 => emission.min(15),
            _ => held.name.as_deref().map_or(0, HeldLightTable::exception),
        }
    }
}

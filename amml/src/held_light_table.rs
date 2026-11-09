/// Light levels for held items that place no block, so the game has no block light value for them. Every other item
/// takes its level from the block it places (`Block.mDirectData.mLightEmission`).
pub struct HeldLightTable;

impl HeldLightTable {
    const EXCEPTIONS: &'static [(&'static str, u8)] = &[("minecraft:lava_bucket", 15), ("minecraft:blaze_rod", 10)];

    /// The level for an item that places no block, by its full name; 0 for any other item.
    pub fn exception(name: &str) -> u8 {
        Self::EXCEPTIONS.iter().find(|(known, _)| *known == name).map_or(0, |(_, level)| *level)
    }
}

#[cfg(test)]
mod tests {
    use super::HeldLightTable;

    #[test]
    fn the_two_exceptions() {
        assert_eq!(HeldLightTable::exception("minecraft:lava_bucket"), 15);
        assert_eq!(HeldLightTable::exception("minecraft:blaze_rod"), 10);
    }

    #[test]
    fn other_items_give_no_exception_light() {
        assert_eq!(HeldLightTable::exception("minecraft:torch"), 0);
        assert_eq!(HeldLightTable::exception("minecraft:stone"), 0);
        assert_eq!(HeldLightTable::exception(""), 0);
    }
}

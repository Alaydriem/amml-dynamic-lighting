use alaydriem_dynamic_lighting::held_light_table::HeldLightTable;

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

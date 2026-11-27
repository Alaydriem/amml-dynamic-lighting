use alaydriem_dynamic_lighting::lighting_settings::LightingConfig;
use alaydriem_dynamic_lighting::services::LightPicker;
use alaydriem_dynamic_lighting::structs::HeldLight;
use amml_api::{LightFalloff, Vec3};

const AT: Vec3 = Vec3 { x: 1.5, y: 65.62, z: -3.25 };

fn block_item(emission: u8) -> HeldLight {
    HeldLight { emission: Some(emission), block: None, name: None }
}

fn named(name: &str) -> HeldLight {
    HeldLight { emission: None, block: None, name: Some(name.to_string()) }
}

fn defaults() -> LightingConfig {
    LightingConfig::default()
}

#[test]
fn a_torch_in_the_main_hand_lights_at_its_level() {
    let light = LightPicker::pick(Some(&block_item(14)), None, &defaults(), true, Some(AT)).unwrap();
    assert_eq!((light.position, light.level, light.falloff), (AT, 14, LightFalloff::Round));
}

#[test]
fn the_off_hand_lights_alone() {
    let light = LightPicker::pick(None, Some(&block_item(14)), &defaults(), true, Some(AT)).unwrap();
    assert_eq!(light.level, 14);
}

#[test]
fn the_brighter_hand_wins_and_the_main_hand_wins_a_tie() {
    assert_eq!(
        LightPicker::pick(Some(&block_item(14)), Some(&block_item(15)), &defaults(), true, Some(AT)).unwrap().level,
        15
    );
    assert_eq!(
        LightPicker::pick(Some(&block_item(15)), Some(&block_item(7)), &defaults(), true, Some(AT)).unwrap().level,
        15
    );
}

#[test]
fn items_without_a_block_use_the_exceptions_by_name() {
    assert_eq!(
        LightPicker::pick(Some(&named("minecraft:lava_bucket")), None, &defaults(), true, Some(AT)).unwrap().level,
        15
    );
    assert_eq!(
        LightPicker::pick(Some(&named("minecraft:blaze_rod")), None, &defaults(), true, Some(AT)).unwrap().level,
        10
    );
    assert!(LightPicker::pick(Some(&named("minecraft:stick")), None, &defaults(), true, Some(AT)).is_none());
    assert!(LightPicker::pick(Some(&block_item(0)), None, &defaults(), true, Some(AT)).is_none());
}

#[test]
fn the_override_changes_light_items_only() {
    let settings = LightingConfig { level_override: 5, ..defaults() };
    assert_eq!(LightPicker::pick(Some(&block_item(14)), None, &settings, true, Some(AT)).unwrap().level, 5);
    assert!(LightPicker::pick(Some(&named("minecraft:stick")), None, &settings, true, Some(AT)).is_none());
}

#[test]
fn round_off_gives_the_taxicab_falloff() {
    let settings = LightingConfig { round: false, ..defaults() };
    assert_eq!(
        LightPicker::pick(Some(&block_item(14)), None, &settings, true, Some(AT)).unwrap().falloff,
        LightFalloff::Taxicab
    );
}

#[test]
fn no_light_while_off_or_without_a_position() {
    let off = LightingConfig { enabled: false, ..defaults() };
    assert!(LightPicker::pick(Some(&block_item(14)), None, &off, true, Some(AT)).is_none());
    assert!(LightPicker::pick(Some(&block_item(14)), None, &defaults(), false, Some(AT)).is_none());
    assert!(LightPicker::pick(Some(&block_item(14)), None, &defaults(), true, None).is_none());
}

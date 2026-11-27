use amml_api::{LightFalloff, Vec3};

use crate::support::light_rig::{EYES, GLOWSTONE, LAVA, LightRig, TORCH};

const DEFAULTS: &str = "lighting {\n  enabled = true\n}\n";

#[test]
fn a_held_torch_sets_a_light_with_its_block_at_the_eyes() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.tick();
    let light = rig.game.light_source().unwrap();
    assert_eq!((light.position, light.level, light.falloff), (EYES, 14, LightFalloff::Round));
    assert!(light.block.is_some());
    assert!(rig.logged("info light source set level=14 falloff=round block=yes"));
    assert!(rig.logged("info item names: on"));
}

#[test]
fn the_light_is_sent_only_when_it_changes() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.tick();
    rig.tick();
    assert_eq!(rig.game.light_source_calls(), 1);
    rig.move_to(Vec3 { x: 2.0, ..EYES });
    rig.tick();
    assert_eq!(rig.game.light_source_calls(), 2);
}

#[test]
fn the_brighter_off_hand_gives_the_light() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), Some(GLOWSTONE));
    rig.tick();
    assert_eq!(rig.game.light_source().unwrap().level, 15);
}

#[test]
fn a_lava_bucket_lights_by_its_name_without_a_block() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(LAVA), None);
    rig.tick();
    let light = rig.game.light_source().unwrap();
    assert_eq!((light.level, light.block), (15, None));
}

#[test]
fn putting_the_torch_away_clears_the_light() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.tick();
    rig.hold(None, None);
    rig.tick();
    assert_eq!(rig.game.light_source(), None);
    assert!(rig.logged("info light source cleared"));
}

#[test]
fn the_toggle_key_switches_the_light_off_and_on() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.tick();
    rig.press_toggle();
    assert!(rig.logged("info toggled off"));
    assert_eq!(rig.game.light_source(), None);
    rig.press_toggle();
    assert!(rig.game.light_source().is_some());
}

#[test]
fn the_settings_set_the_falloff_and_the_override() {
    let rig = LightRig::new("lighting {\n  round = false\n  level_override = 5\n}\n");
    rig.hold(Some(TORCH), None);
    rig.tick();
    let light = rig.game.light_source().unwrap();
    assert_eq!((light.level, light.falloff), (5, LightFalloff::Taxicab));
}

#[test]
fn after_a_world_leave_the_light_is_sent_again() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.tick();
    rig.world_left();
    rig.game.set_epoch(2);
    rig.tick();
    assert_eq!(rig.game.light_source_calls(), 2);
    assert!(rig.game.light_source().unwrap().block.is_some());
}

#[test]
fn a_refused_light_is_tried_again_and_logged_once() {
    let rig = LightRig::new(DEFAULTS);
    rig.hold(Some(TORCH), None);
    rig.game.refuse_lighting(true);
    rig.tick();
    rig.tick();
    assert_eq!(rig.game.logs().iter().filter(|line| line.starts_with("warn light source refused")).count(), 1);
    rig.game.refuse_lighting(false);
    rig.tick();
    assert!(rig.game.light_source().is_some());
}

#[test]
fn enable_reports_the_entity_light_and_the_vibrant_colour() {
    let rig = LightRig::new(DEFAULTS);
    assert!(rig.logged("info entity light: on"));
    assert!(rig.logged("info vibrant colour: on"));
    assert!(rig.logged("info enabled"));
}

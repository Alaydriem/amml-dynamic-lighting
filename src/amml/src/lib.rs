//! @alaydriem/dynamic-lighting: when the local player holds a light-emitting item, the terrain, the entities and
//! the block entities near the player are drawn lit, and in Vibrant Visuals the held block is a real emitter with
//! its own colour. Visual only: the world's light values do not change.
//!
//! The mod picks one light per client tick (the brighter hand, the item exceptions, the level override and the
//! toggle) and gives it to the loader with `Lighting::set_light_source`; the loader does the drawing and the mesh
//! rebuilds.

#![deny(unsafe_code)]

pub mod dynamic_lighting_mod;
pub mod held_light_table;
pub mod lighting_settings;
pub mod services;
pub mod structs;

pub use dynamic_lighting_mod::DynamicLightingMod;

amml_api::amml_mod!(DynamicLightingMod);

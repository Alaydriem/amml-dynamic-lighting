//! @alaydriem/dynamic-lighting: when the local player holds a light-emitting item, the terrain
//! meshes near the player are built with extra block light. Visual only: the world's light values do not change.
//!
//! How: a detour on the tessellator cache light getter raises the block-light nibble by distance to the player,
//! and the subchunks in reach are re-meshed through `RenderChunkCoordinator::_setDirty` when the light moves a
//! quarter block or changes level. In Vibrant Visuals the static light pass that would overwrite that vertex light
//! is skipped for the subchunks in reach.

#![deny(unsafe_code)]

pub mod detours;
pub mod dynamic_lighting_mod;
pub mod held_light_table;
pub mod light_falloff;
pub mod light_source;
pub mod lighting_settings;
pub mod player_inventory;
pub mod remesh_plan;
pub mod remesher;
pub mod runtime;
pub mod vv_emitter;

pub use dynamic_lighting_mod::DynamicLightingMod;

amml_sdk::amml_mod!(DynamicLightingMod);

use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;

use amml_game::{EntryId, EntryNeed, GameEntries};
use amml_sdk::{AmmlMod, Api, Logger, ModDirs, SdkError};

use crate::detours::{client_update, get_light_color, light_grid_fill, propagate_for_mesh, render_level};
use crate::runtime::DynamicLightingRuntime;
use crate::vv_emitter::VvEmitter;

/// The AMML entry point: load makes the runtime, enable resolves the entries and installs the hooks, disable
/// removes them.
pub struct DynamicLightingMod;

impl DynamicLightingMod {
    pub const MODULE: &'static str = "@amml/dynamic-lighting";

    /// The Vibrant Visuals hand emitter (optional): the light grid fill (hooked), the game's `_tls_index` and the
    /// emitter vector offset in the game's TLS block. Without them the mod skips the static light pass near the light.
    const GRID_FILL: &'static str = "LightPropagation.fillSubChunkGrid";
    const TLS_INDEX: &'static str = "core.tls_index";
    const TLS_EMITTERS: &'static str = "LightPropagation.tlsEmitterVector";

    const ENTRIES: [&'static str; 21] = [
        "ClientInstance.update",
        "ClientInstance.getLocalPlayer",
        "ClientInstance.getRegion",
        "Actor.stateVectorComponent",
        "Actor.actorRotationComponent",
        "StateVectorComponent.mPos",
        "ActorRotationComponent.mRot",
        "Player.playerInventory",
        "Item.mFullName",
        "Actor.getItemInHandSlot",
        "ItemStackBase.mBlock",
        "Block.mDirectData.mLightEmission",
        "BlockTessellatorCache.getLightColor",
        "LevelRenderer.renderLevel",
        "RenderChunkCoordinator._setDirty",
        "RenderChunkCoordinator.vtable",
        "LevelRenderer.mRenderChunkCoordinators",
        "LightPropagationCoordinator.propagateForMesh",
        Self::GRID_FILL,
        Self::TLS_INDEX,
        Self::TLS_EMITTERS,
    ];

    fn hooks() -> [(&'static str, *const c_void, &'static AtomicPtr<c_void>); 4] {
        [
            (
                "BlockTessellatorCache.getLightColor",
                get_light_color::detour as *const c_void,
                get_light_color::GUARD.original(),
            ),
            ("LevelRenderer.renderLevel", render_level::detour as *const c_void, render_level::GUARD.original()),
            ("ClientInstance.update", client_update::detour as *const c_void, client_update::GUARD.original()),
            (
                "LightPropagationCoordinator.propagateForMesh",
                propagate_for_mesh::detour as *const c_void,
                propagate_for_mesh::GUARD.original(),
            ),
        ]
    }

    fn install(api: &Api, runtime: &DynamicLightingRuntime) -> Result<(), SdkError> {
        // Item.mFullName is refused on builds with the new HashedString layout (Preview 1.26.70); without it no item
        // name is read, so only the two coded exceptions (no block) lose their light
        let need = |id: &str| match id {
            "Item.mFullName" => EntryNeed::Optional { feature: "item names" },
            Self::GRID_FILL | Self::TLS_INDEX | Self::TLS_EMITTERS => EntryNeed::Optional { feature: "vibrant colour" },
            _ => EntryNeed::Required,
        };
        let wanted: Vec<(EntryId, EntryNeed)> = Self::ENTRIES.iter().map(|id| (EntryId::new(id), need(id))).collect();
        let entries = GameEntries::resolve(api, &wanted).map_err(|error| SdkError::Mod(error.to_string()))?;
        let mut targets = Vec::new();
        for (entry, detour, original) in Self::hooks() {
            let target = entries.get(EntryId::new(entry)).ok_or_else(|| SdkError::MissingEntry(entry.to_string()))?;
            targets.push((target, detour, original));
        }
        let emitter = match (
            entries.get(EntryId::new(Self::GRID_FILL)),
            entries.get(EntryId::new(Self::TLS_INDEX)),
            entries.get(EntryId::new(Self::TLS_EMITTERS)),
        ) {
            (Some(grid_fill), Some(tls_index), Some(emitters)) => {
                targets.push((grid_fill, light_grid_fill::detour as *const c_void, light_grid_fill::GUARD.original()));
                usize::try_from(tls_index).ok().zip(usize::try_from(emitters).ok())
            }
            _ => None,
        };
        runtime.set_entries(entries);
        for (target, detour, original) in targets {
            // SAFETY: each target is the derived start of the game function its detour was written for (see
            // detours.rs for the signatures); each original slot belongs to one hook
            #[allow(unsafe_code)]
            let hooked = unsafe { api.hook(target, detour, 0, original) };
            match hooked {
                Ok(hook) => runtime.keep_hook(hook),
                Err(error) => {
                    Self::remove(api, runtime);
                    return Err(error);
                }
            }
        }
        match emitter {
            Some((tls_index, emitters)) => {
                VvEmitter::enable(tls_index, emitters);
                runtime.logger().info("vibrant visuals: the held block is added as a light (game colour)");
            }
            None => {
                runtime.logger().info("vibrant visuals: emitter entries missing; static light skipped near the light")
            }
        }
        Ok(())
    }

    fn remove(api: &Api, runtime: &DynamicLightingRuntime) {
        for hook in runtime.take_hooks() {
            if let Err(error) = api.unhook(hook) {
                runtime.logger().warn(&format!("unhook failed: {error}"));
            }
        }
    }
}

impl AmmlMod for DynamicLightingMod {
    const NAMESPACE: &'static str = "amml";
    const NAME: &'static str = "dynamic-lighting";

    fn load(api: &Api) -> Result<(), SdkError> {
        let dirs = ModDirs::new(api, Self::MODULE)?;
        DynamicLightingRuntime::install(Logger::new(api, Self::MODULE), dirs.config_dir);
        Ok(())
    }

    fn enable(api: &Api) -> Result<(), SdkError> {
        let runtime = DynamicLightingRuntime::get().ok_or_else(|| SdkError::Mod("load did not run".to_string()))?;
        Self::install(api, runtime)?;
        runtime.enable_link();
        runtime.logger().info("enabled");
        Ok(())
    }

    fn disable(api: &Api) {
        let Some(runtime) = DynamicLightingRuntime::get() else { return };
        Self::remove(api, runtime);
        runtime.detach();
    }
}

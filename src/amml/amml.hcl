namespace    = "alaydriem"
name         = "dynamic-lighting"
entry        = "alaydriem_dynamic_lighting.dll"
api_version  = 2
amml_version = "26.63.2"
entries = [
  # required: the per-frame tick, the player, the held item
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
  # required: the terrain light (recipe in ../amml/recipes/block_tessellator_cache_light.hcl)
  "BlockTessellatorCache.getLightColor",
  # required: the mesh rebuild near the player
  "LevelRenderer.renderLevel",
  # recipe in ../amml/recipes/render_chunk_coordinator_set_dirty.hcl
  "RenderChunkCoordinator._setDirty",
  "RenderChunkCoordinator.vtable",
  "LevelRenderer.mRenderChunkCoordinators",
  # optional: entity and block-entity light (recipe in ../amml/recipes/block_source.hcl)
  "BlockSource.getLightColor",
  # Vibrant Visuals at Block Light Quality "off": skip the static light rewrite near the light
  "LightPropagationCoordinator.propagateForMesh",
  # optional: Vibrant Visuals hand emitter (the held block is lit with its own game colour)
  "LightPropagation.fillSubChunkGrid",
  "core.tls_index",
  "LightPropagation.tlsEmitterVector",
]

listing {
  title       = "Dynamic Lighting"
  summary     = "A held torch or lantern lights up the world around you."
  description = <<-EOT
    When you hold a light-giving item, the terrain around you is drawn lit, and the light follows you. Works in the normal renderer and in Vibrant Visuals. Visual only: no real light level changes.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-dynamic-lighting"
  homepage    = "https://github.com/alaydriem/amml-dynamic-lighting#readme"
  tags        = ["lighting", "visual", "client"]
  icon        = "icon.png"
}

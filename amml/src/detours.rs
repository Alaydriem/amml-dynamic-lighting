// The game functions the mod hooks. Each body hands the raw arguments to the runtime and calls the original.

use amml_sdk::amml_detour;

use crate::light_source::{LightSource, VvPlan};
use crate::runtime::DynamicLightingRuntime;
use crate::vv_emitter::VvEmitter;

// bool ClientInstance::update(bool, bool): once per frame on the game thread. The third argument (26.60+) is carried
// as a raw u64 so the register reaches the original bit for bit
amml_detour!(pub mod client_update(this: usize, init_finished: u8, extra: u64) -> u8 |original| {
    let result = original(this, init_finished, extra);
    if let Some(runtime) = DynamicLightingRuntime::get() {
        runtime.tick(this);
    }
    result
});

// void LevelRenderer::renderLevel(ScreenContext&, FrameRenderObject const&): the rebuild requests are sent here,
// before the original, so the dirty boxes are read by this frame's update
amml_detour!(pub mod render_level(this: usize, screen: usize, frame: usize) -> () |original| {
    if let Some(runtime) = DynamicLightingRuntime::get() {
        runtime.render(this);
    }
    original(this, screen, frame);
});

// BrightnessPair* BlockTessellatorCache::getLightColor(BrightnessPair* out, BlockPos const& pos, Block const* block):
// runs on the mesh worker threads. After the original, the block byte (out[1]) is raised to the held light's
// value; the sky byte and the cache stay as they are. No logging and no locks here
amml_detour!(pub mod get_light_color(cache: usize, out: usize, pos: usize, block: usize) -> usize |original| {
    let result = original(cache, out, pos, block);
    if out != 0 && pos != 0 {
        let out = out as *mut u8;
        let pos = pos as *const [i32; 3];
        // SAFETY: the game passes a live BlockPos and a 2-byte result buffer, both valid for the call
        #[allow(unsafe_code)]
        unsafe {
            let current = *out.add(1);
            let raised = LightSource::get().apply(*pos, current);
            if raised != current {
                *out.add(1) = raised;
            }
        }
    }
    result
});

// bool LightPropagationCoordinator::propagateForMesh(BlockSource&, SubChunkPos const&, u16 variant, u16 flags,
// provider, onResult, onEmpty): Vibrant Visuals only; the mesher calls it after tessellation, on the mesh threads.
// It spreads the static block light and writes it over the vertex block light, so the getLightColor boost is lost.
// For subchunks the held light reaches, the call is skipped; the mesher does not read the result. The two u16
// arguments are carried as raw u64 so the registers and stack slots reach the original bit for bit
amml_detour!(pub mod propagate_for_mesh(
    this: usize, source: usize, pos: usize, variant: u64, flags: u64, provider: usize, on_result: usize, on_empty: usize
) -> usize |original| {
    if pos != 0 {
        // SAFETY: the game passes a live SubChunkPos (three i32) for the call
        #[allow(unsafe_code)]
        let subchunk = unsafe { *(pos as *const [i32; 3]) };
        match LightSource::get().vv_plan(subchunk, variant) {
            VvPlan::Skip => return 0,
            VvPlan::Emit(emitter) => {
                return VvEmitter::armed(emitter, || original(this, source, pos, variant, flags, provider, on_result, on_empty));
            }
            VvPlan::Normal => {}
        }
    }
    original(this, source, pos, variant, flags, provider, on_result, on_empty)
});

// bool <light grid fill>(BlockSource*, SubChunkPos const*, u16 mode, grid[27]*, u32* mask): the light spread core calls
// it after it empties its thread-local emitter vector and before it collects the light blocks. After the original,
// the armed hand emitter (set by the propagateForMesh detour above, same thread) is added to that vector. The u16 is
// carried as a raw u64
amml_detour!(pub mod light_grid_fill(source: usize, pos: usize, mode: u64, grid: usize, mask: usize) -> u64 |original| {
    let result = original(source, pos, mode, grid, mask);
    if pos != 0 {
        // SAFETY: the core passes a live SubChunkPos (three i32) for the call
        #[allow(unsafe_code)]
        let subchunk = unsafe { *(pos as *const [i32; 3]) };
        VvEmitter::after_grid_fill(subchunk);
    }
    result
});

use std::ffi::c_void;

use amml_game::{BlockPos, EntryId, GameEntries, RawObject};

// void RenderChunkCoordinator::_setDirty(BlockPos const& min, BlockPos const& max, bool immediate,
// bool changesVisibility, bool canInterlock)
type SetDirtyFn = unsafe extern "C" fn(*mut c_void, *const BlockPos, *const BlockPos, bool, bool, bool);

/// Marks terrain meshes dirty through the game's own `RenderChunkCoordinator::_setDirty`, the call the coordinator's
/// block-change listener makes. The per-frame LevelRenderer update then rebuilds the marked subchunks; with
/// `immediate` set, the ones within 24 blocks of the camera are built at once.
/// See docs/findings/terrain-dirty-path/report.md.
pub struct Remesher;

impl Remesher {
    /// The node fields of the coordinator map `unordered_map<DimensionType, shared_ptr<RenderChunkCoordinator>>`
    /// (entry `LevelRenderer.mRenderChunkCoordinators`, the list head pointer). A node is used only when its object
    /// has the vtable of entry `RenderChunkCoordinator.vtable`.
    const NODE_DIMENSION: u64 = 0x10;
    const NODE_COORDINATOR: u64 = 0x18;
    const MAX_NODES: usize = 16;

    /// The coordinators in the LevelRenderer map that pass the vtable guard, with their dimension ids, and how many
    /// nodes failed the guard.
    ///
    /// # Safety
    /// `level_renderer` must be the live LevelRenderer; game thread.
    #[allow(unsafe_code)]
    pub unsafe fn coordinators(level_renderer: *mut c_void, entries: &GameEntries) -> (Vec<(i32, *mut c_void)>, usize) {
        let mut found = Vec::new();
        let mut refused = 0;
        let vtable = entries.get(EntryId::new("RenderChunkCoordinator.vtable"));
        let map = entries.get(EntryId::new("LevelRenderer.mRenderChunkCoordinators"));
        let (false, Some(vtable), Some(map)) = (level_renderer.is_null(), vtable, map) else {
            return (found, refused);
        };
        let Ok(expected) = usize::try_from(vtable) else { return (found, refused) };
        // SAFETY: the list head (a node pointer) of the coordinator map
        let Some(head) = (unsafe { RawObject::pointer(level_renderer, map) }) else {
            return (found, refused);
        };
        let mut node = head;
        for _ in 0..Self::MAX_NODES {
            // SAFETY: a live list node: next at +0
            let Some(next) = (unsafe { RawObject::pointer(node, 0) }) else { break };
            if next == head {
                break;
            }
            node = next;
            // SAFETY: a live map node: the key at +10h and the shared_ptr object pointer at +18h
            let dimension = unsafe { RawObject::read::<i32>(node, Self::NODE_DIMENSION) };
            // SAFETY: as above
            let coordinator = unsafe { RawObject::pointer(node, Self::NODE_COORDINATOR) };
            let (Some(dimension), Some(coordinator)) = (dimension, coordinator) else { continue };
            // SAFETY: a live object whose first qword is its vtable
            let vtable = unsafe { RawObject::read::<usize>(coordinator, 0) };
            if vtable == Some(expected) {
                found.push((dimension, coordinator));
            } else {
                refused += 1;
            }
        }
        (found, refused)
    }

    /// Marks every block box (inclusive min, max) dirty in each coordinator; returns the number of calls made. A
    /// coordinator drops marks for subchunks it has no render chunk for, so the coordinators of other dimensions are
    /// not harmed.
    ///
    /// # Safety
    /// `level_renderer` must be the live LevelRenderer; game thread (the thread of the update that reads the marks).
    #[allow(unsafe_code)]
    pub unsafe fn mark(
        level_renderer: *mut c_void,
        entries: &GameEntries,
        boxes: &[([i32; 3], [i32; 3])],
    ) -> Result<usize, String> {
        let set_dirty = entries
            .get(EntryId::new("RenderChunkCoordinator._setDirty"))
            .ok_or("RenderChunkCoordinator._setDirty missing")?;
        // SAFETY: the entry is the start of RenderChunkCoordinator::_setDirty (signature above)
        let set_dirty: SetDirtyFn =
            unsafe { std::mem::transmute(usize::try_from(set_dirty).map_err(|e| e.to_string())?) };
        // SAFETY: as the caller guarantees
        let (coordinators, refused) = unsafe { Self::coordinators(level_renderer, entries) };
        if coordinators.is_empty() {
            return Err(format!("no coordinator passed the vtable guard ({refused} refused)"));
        }
        let mut calls = 0;
        for (_, coordinator) in &coordinators {
            for (min, max) in boxes {
                let min = BlockPos { x: min[0], y: min[1], z: min[2] };
                let max = BlockPos { x: max[0], y: max[1], z: max[2] };
                // SAFETY: a guarded coordinator and two positions that live across the call; immediate build, no
                // visibility change, no interlock
                unsafe { set_dirty(*coordinator, &raw const min, &raw const max, true, false, false) };
                calls += 1;
            }
        }
        Ok(calls)
    }
}

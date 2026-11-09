use std::cell::Cell;
use std::sync::OnceLock;

/// The held block as a real emitter in the Vibrant Visuals static light spread, so the game lights it with its own
/// colour and falloff. See docs/findings/vv-hand-emitter/report.md.
///
/// The light spread core empties a thread-local emitter vector (`TLS[game _tls_index] + offset`), fills the
/// 48x48x48 grid of 3x3x3 subchunks (the grid fill function), then adds one 16-byte entry per light block and spreads
/// them. The grid fill detour adds the hand entry right after the original, while `propagateForMesh` is on the stack
/// for a subchunk near the light (the thread-local `ARMED` below).
pub struct VvEmitter;

/// One hand emitter: the held block, its world block position and its level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandEmitter {
    pub block: usize,
    pub position: [i32; 3],
    pub emission: u8,
}

/// The two addresses the push needs, from the entries.
#[derive(Debug, Clone, Copy)]
struct TlsLayout {
    /// Absolute address of the game's `_tls_index` (u32).
    tls_index: usize,
    /// Offset of the emitter vector (begin, end, capacity) in the game's TLS block.
    emitters: usize,
}

static LAYOUT: OnceLock<TlsLayout> = OnceLock::new();

thread_local! {
    static ARMED: Cell<Option<HandEmitter>> = const { Cell::new(None) };
}

impl VvEmitter {
    pub const ENTRY_SIZE: usize = 16;
    const WINDOW: i32 = 48;

    /// Turns the hand emitter on: called once at enable when the grid fill hook and the two TLS entries resolved.
    pub fn enable(tls_index: usize, emitters: usize) {
        let _ = LAYOUT.set(TlsLayout { tls_index, emitters });
    }

    pub fn ready() -> bool {
        LAYOUT.get().is_some()
    }

    /// Runs `inner` with `emitter` armed on this thread (the `propagateForMesh` detour), then disarms.
    pub fn armed<R>(emitter: HandEmitter, inner: impl FnOnce() -> R) -> R {
        ARMED.with(|slot| slot.set(Some(emitter)));
        let result = inner();
        ARMED.with(|slot| slot.set(None));
        result
    }

    /// The window-local position of `position` in the grid of the subchunk at `subchunk`; `None` outside 0..47.
    pub fn local(position: [i32; 3], subchunk: [i32; 3]) -> Option<[u8; 3]> {
        let mut out = [0u8; 3];
        for axis in 0..3 {
            let origin = subchunk[axis].checked_sub(1)?.checked_mul(16)?;
            let local = position[axis].checked_sub(origin)?;
            if !(0..Self::WINDOW).contains(&local) {
                return None;
            }
            out[axis] = u8::try_from(local).ok()?;
        }
        Some(out)
    }

    /// The 16 bytes of one emitter entry: the Block pointer at 0, x/y/z at 8/9/10, the emission at 11.
    pub fn entry(block: usize, local: [u8; 3], emission: u8) -> [u8; Self::ENTRY_SIZE] {
        let mut out = [0u8; Self::ENTRY_SIZE];
        out[..8].copy_from_slice(&(block as u64).to_le_bytes());
        out[8..11].copy_from_slice(&local);
        out[11] = emission.min(15);
        out
    }

    /// The grid fill detour, after the original, for the subchunk at `subchunk`: adds the armed hand entry to this
    /// thread's emitter vector. Returns true when an entry was added.
    pub fn after_grid_fill(subchunk: [i32; 3]) -> bool {
        let Some(emitter) = ARMED.with(Cell::get) else { return false };
        let Some(layout) = LAYOUT.get() else { return false };
        let Some(local) = Self::local(emitter.position, subchunk) else { return false };
        if emitter.block == 0 {
            return false;
        }
        let entry = Self::entry(emitter.block, local, emitter.emission);
        // SAFETY: the grid fill runs inside the light spread core on this thread, which holds its emitter vector in
        // the game's TLS block at the entry offset
        #[allow(unsafe_code)]
        unsafe {
            let Some(vector) = Self::vector(*layout) else { return false };
            Self::push(vector, &entry)
        }
    }

    /// This thread's emitter vector in the game's TLS block.
    ///
    /// # Safety
    /// The layout must hold the game's `_tls_index` address and the vector offset; the game's TLS block of this
    /// thread must be set up (true inside the light spread core).
    #[allow(unsafe_code)]
    unsafe fn vector(layout: TlsLayout) -> Option<*mut [usize; 3]> {
        let tls_array: *const usize;
        // SAFETY: gs:[0x58] is the TEB ThreadLocalStoragePointer on x64 Windows
        unsafe {
            std::arch::asm!("mov {}, gs:[0x58]", out(reg) tls_array, options(nostack, readonly, preserves_flags))
        };
        if tls_array.is_null() || layout.tls_index == 0 {
            return None;
        }
        // SAFETY: the game's _tls_index global (u32)
        let index = usize::try_from(unsafe { (layout.tls_index as *const u32).read() }).ok()?;
        // SAFETY: the TLS array holds one block pointer per module index
        let block = unsafe { tls_array.add(index).read() };
        if block == 0 {
            return None;
        }
        Some(block.checked_add(layout.emitters)? as *mut [usize; 3])
    }

    /// Appends `entry` to the vector (begin, end, capacity) of 16-byte entries. When the vector is full it grows it
    /// as the MSVC STL does (1.5 times, through the game heap, big-block layout) and frees the old buffer. Returns
    /// false, and changes nothing, when the vector looks wrong or the heap is not there.
    ///
    /// # Safety
    /// `vector` must be a live `std::vector` of 16-byte trivially copyable entries that the game owns and frees with
    /// its heap; no other thread uses it during the call.
    #[allow(unsafe_code)]
    pub unsafe fn push(vector: *mut [usize; 3], entry: &[u8; Self::ENTRY_SIZE]) -> bool {
        let size = Self::ENTRY_SIZE;
        // SAFETY: the caller guarantees a live vector
        let [begin, end, cap] = unsafe { vector.read() };
        let sane = if begin == 0 {
            end == 0 && cap == 0
        } else {
            begin <= end && end <= cap && (end - begin) % size == 0 && (cap - begin) % size == 0
        };
        if !sane {
            return false;
        }
        if cap - end >= size {
            // SAFETY: the spare capacity holds at least one entry
            unsafe { std::ptr::copy_nonoverlapping(entry.as_ptr(), end as *mut u8, size) };
            // SAFETY: as above
            unsafe { (*vector)[1] = end + size };
            return true;
        }
        let Some(heap) = amml_game::GameHeap::msvc() else { return false };
        let count = (end - begin) / size;
        let old_cap = (cap - begin) / size;
        let new_cap = (old_cap + old_cap / 2).max(count + 1);
        let Some(bytes) = new_cap.checked_mul(size) else { return false };
        // SAFETY: the loader's game heap, used the way the STL allocator uses it
        let new = unsafe { heap.allocate(bytes) };
        if new.is_null() {
            return false;
        }
        // SAFETY: the new block holds `new_cap > count` entries; the old block holds `count`; they do not overlap
        unsafe {
            if count > 0 {
                std::ptr::copy_nonoverlapping(begin as *const u8, new, count * size);
            }
            std::ptr::copy_nonoverlapping(entry.as_ptr(), new.add(count * size), size);
        }
        if begin != 0 {
            // SAFETY: the old buffer came from the game heap with `cap - begin` bytes; its entries were copied out
            unsafe { heap.deallocate(begin as *mut u8, cap - begin) };
        }
        let new = new as usize;
        // SAFETY: as the caller guarantees
        unsafe { vector.write([new, new + (count + 1) * size, new + bytes]) };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{HandEmitter, VvEmitter};

    #[test]
    fn local_position_in_the_window() {
        // subchunk (0, 4, 0): the window starts at (-16, 48, -16)
        assert_eq!(VvEmitter::local([2, 64, 15], [0, 4, 0]), Some([18, 16, 31]));
        assert_eq!(VvEmitter::local([-16, 48, -16], [0, 4, 0]), Some([0, 0, 0]));
        assert_eq!(VvEmitter::local([31, 95, 31], [0, 4, 0]), Some([47, 47, 47]));
        assert_eq!(VvEmitter::local([32, 64, 0], [0, 4, 0]), None);
        assert_eq!(VvEmitter::local([-17, 64, 0], [0, 4, 0]), None);
    }

    #[test]
    fn entry_layout() {
        let entry = VvEmitter::entry(0x1122_3344_5566_7788, [1, 2, 3], 20);
        assert_eq!(&entry[..8], &0x1122_3344_5566_7788u64.to_le_bytes());
        assert_eq!(&entry[8..12], &[1, 2, 3, 15]);
        assert!(entry[12..].iter().all(|b| *b == 0));
    }

    #[test]
    #[allow(unsafe_code)]
    fn push_uses_spare_capacity_and_refuses_a_bad_vector() {
        let size = VvEmitter::ENTRY_SIZE;
        let mut buffer = vec![0xAAu8; size * 2];
        let begin = buffer.as_mut_ptr() as usize;
        let entry = VvEmitter::entry(8, [1, 2, 3], 14);
        let mut vector = [begin, begin + size, begin + 2 * size];
        // SAFETY: a test vector over a live buffer with room for one more entry
        assert!(unsafe { VvEmitter::push(&raw mut vector, &entry) });
        assert_eq!(vector, [begin, begin + 2 * size, begin + 2 * size]);
        assert_eq!(&buffer[size..], &entry[..]);
        let mut bad = [begin, begin + 1, begin + 2 * size];
        // SAFETY: refused before any write
        assert!(!unsafe { VvEmitter::push(&raw mut bad, &entry) });
    }

    #[test]
    fn nothing_is_added_when_not_armed() {
        assert!(!VvEmitter::after_grid_fill([0, 4, 0]));
        let emitter = HandEmitter { block: 0, position: [0, 64, 0], emission: 14 };
        assert!(!VvEmitter::armed(emitter, || VvEmitter::after_grid_fill([0, 4, 0])));
    }
}

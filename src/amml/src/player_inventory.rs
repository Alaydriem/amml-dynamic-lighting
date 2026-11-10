use std::ffi::c_void;

use amml_game::{EntryId, GameEntries, ItemStackBaseRef, ItemStackLayout, RawObject};

use crate::held_light_table::HeldLightTable;

// ItemStack const& Actor::getItemInHandSlot(HandSlot slot) const: rcx the Actor, dl the HandSlot
type GetItemInHandSlotFn = unsafe extern "C" fn(*mut c_void, u8) -> *const c_void;

/// One held item: its full name (when the build has the name entry) and its light level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldItem {
    pub name: Option<String>,
    /// The light the item's block gives (0..15), or the exception level of an item that places no block.
    pub level: u8,
    /// The `Block const*` the item places; 0 for an item that places no block.
    pub block: usize,
}

/// The hand stacks of the local player. The main hand is read the way stratum reads it: `Player.playerInventory` is
/// the offset of the `unique_ptr<PlayerInventory>`; PlayerInventory holds mSelected at 10h and the Inventory at B8h,
/// and the FillingContainer holds its `vector<ItemStack>` at 198h (98h bytes a stack). The three constants are
/// guarded by the kept bytes of the `Player.playerInventory` recipe. The offhand comes from
/// `Actor.getItemInHandSlot`. The light level comes from the block the stack places: `ItemStackBase.mBlock`, then
/// `Block.mDirectData.mLightEmission`.
pub struct PlayerInventory;

impl PlayerInventory {
    const SELECTED: u64 = 0x10;
    const INVENTORY: u64 = 0xB8;
    const ITEMS: u64 = 0x198;
    const OFFHAND: u8 = 1;

    /// The selected hotbar item; `None` for an empty hand or a missing entry or pointer.
    ///
    /// # Safety
    /// `player` must be null or the live local player; game thread.
    #[allow(unsafe_code)]
    pub unsafe fn main_hand(player: *mut c_void, entries: &GameEntries) -> Option<HeldItem> {
        if player.is_null() {
            return None;
        }
        let inventory_at = entries.get(EntryId::new("Player.playerInventory"))?;
        // SAFETY: a live player holds its unique_ptr<PlayerInventory> at the entry offset
        let supplies = unsafe { RawObject::pointer(player, inventory_at) }?;
        // SAFETY: a live PlayerInventory holds mSelected at SELECTED
        let selected = unsafe { RawObject::read::<i32>(supplies, Self::SELECTED) }?;
        // SAFETY: a live PlayerInventory holds the unique_ptr<Inventory> at INVENTORY
        let items = unsafe { RawObject::pointer(supplies, Self::INVENTORY) }?;
        // SAFETY: the live container holds mItems (first, last) at ITEMS
        let [first, last] = unsafe { RawObject::read::<[usize; 2]>(items, Self::ITEMS) }?;
        let count = last.checked_sub(first)? / size_of::<ItemStackLayout>();
        let slot = usize::try_from(selected).ok()?;
        if first == 0 || slot >= count {
            return None;
        }
        let stack = first.checked_add(slot.checked_mul(size_of::<ItemStackLayout>())?)?;
        // SAFETY: a stack inside the live item vector
        unsafe { Self::held(stack as *mut c_void, entries) }
    }

    /// The offhand item; `None` for an empty offhand or a missing entry. Uses
    /// `ItemStack const& Actor::getItemInHandSlot(HandSlot)` (HandSlot 1 = offhand), which returns the hand
    /// container's own slot.
    ///
    /// # Safety
    /// `player` must be null or the live local player; game thread.
    #[allow(unsafe_code)]
    pub unsafe fn offhand(player: *mut c_void, entries: &GameEntries) -> Option<HeldItem> {
        if player.is_null() {
            return None;
        }
        let address = entries.get(EntryId::new("Actor.getItemInHandSlot"))?;
        let address = usize::try_from(address).ok().filter(|address| *address != 0)?;
        // SAFETY: the entry is the absolute address of `ItemStack const& Actor::getItemInHandSlot(HandSlot) const`
        let get: GetItemInHandSlotFn = unsafe { std::mem::transmute::<usize, GetItemInHandSlotFn>(address) };
        // SAFETY: a live player on the game thread; the result is the container's slot, valid for this frame
        let stack = unsafe { get(player, Self::OFFHAND) };
        if stack.is_null() {
            return None;
        }
        // SAFETY: as above
        unsafe { Self::held(stack.cast_mut(), entries) }
    }

    /// # Safety
    /// `stack` must be a live ItemStack.
    #[allow(unsafe_code)]
    unsafe fn held(stack: *mut c_void, entries: &GameEntries) -> Option<HeldItem> {
        // SAFETY: the caller guarantees a live stack
        let item = unsafe { ItemStackBaseRef::from_raw(stack, entries) }?;
        if item.is_empty() {
            return None;
        }
        let name = item.type_name();
        // SAFETY: as above
        let (block, block_light) = unsafe { Self::block_light(stack, entries) };
        let level = if block_light > 0 { block_light } else { name.as_deref().map_or(0, HeldLightTable::exception) };
        Some(HeldItem { name, level, block })
    }

    /// The block the stack places and its light emission; (0, 0) for an item that places no block or a missing
    /// entry.
    ///
    /// # Safety
    /// `stack` must be a live ItemStack.
    #[allow(unsafe_code)]
    unsafe fn block_light(stack: *mut c_void, entries: &GameEntries) -> (usize, u8) {
        let (Some(block_at), Some(emission_at)) = (
            entries.get(EntryId::new("ItemStackBase.mBlock")),
            entries.get(EntryId::new("Block.mDirectData.mLightEmission")),
        ) else {
            return (0, 0);
        };
        // SAFETY: a live stack holds its `Block const*` at the entry offset (null for an item without a block)
        let Some(block) = (unsafe { RawObject::pointer(stack, block_at) }) else { return (0, 0) };
        // SAFETY: a live Block holds its light emission byte at the entry offset
        let emission = unsafe { RawObject::read::<u8>(block, emission_at) };
        (block as usize, emission.unwrap_or(0).min(15))
    }
}

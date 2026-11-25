use serde::{Deserialize, Serialize};

/// 64-bit generational handle (`#[repr(C)]` identical to `FgeEntity` in `fge.h`).
///
/// Valid live handles always have `generation >= 1`, so both `EntityId::INVALID`
/// (`{ u32::MAX, 0 }`) and C zero-initialized `{ 0, 0 }` are guaranteed invalid.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

const _: () = assert!(core::mem::size_of::<EntityId>() == 8);

impl EntityId {
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    #[inline]
    pub const fn is_null(self) -> bool {
        self.generation == 0 || self.index == u32::MAX
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct SparseSlot {
    pub dense_index: u32,
    pub generation: u32,
    pub alive: bool,
}

/// Sparse-to-dense generational allocator supporting $O(1)$ validation and swap-remove compaction.
#[derive(Clone, Debug, Default)]
pub struct GenerationalArena {
    pub(crate) slots: Vec<SparseSlot>,
    pub(crate) free_indices: Vec<u32>,
    pub(crate) dense_to_sparse: Vec<u32>,
}

/// Outcome of removing an entity from `GenerationalArena` so parallel SoA arrays can `swap_remove`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SwapRemoveRecord {
    /// Dense SoA index to `swap_remove`.
    pub removed_dense: usize,
}

impl GenerationalArena {
    pub fn with_capacity(_capacity: usize) -> Self {
        todo!("GenerationalArena::with_capacity")
    }

    pub fn len(&self) -> usize {
        self.dense_to_sparse.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense_to_sparse.is_empty()
    }

    /// Allocates a new `EntityId` appended at the end of the dense array (`dense_idx == old_len`).
    pub fn allocate(&mut self) -> EntityId {
        todo!("GenerationalArena::allocate")
    }

    /// Resolves `id` to its current dense SoA index in $O(1)$, returning `None` if stale, dead, or out-of-bounds.
    pub fn resolve(&self, _id: EntityId) -> Option<usize> {
        todo!("GenerationalArena::resolve")
    }

    /// Returns the live `EntityId` currently residing at `dense_index`.
    pub fn entity_at_dense(&self, _dense_index: usize) -> Option<EntityId> {
        todo!("GenerationalArena::entity_at_dense")
    }

    /// Deallocates `id` in $O(1)`, updating sparse redirection for the swapped tail element.
    /// Returns `Some(SwapRemoveRecord)` if `id` was alive, or `None` if invalid/already despawned.
    pub fn deallocate(&mut self, _id: EntityId) -> Option<SwapRemoveRecord> {
        todo!("GenerationalArena::deallocate")
    }

    /// Resets dense occupancy while incrementing generations of previously live slots so old handles stay invalid!
    pub fn clear_invalidating_handles(&mut self) {
        todo!("GenerationalArena::clear_invalidating_handles")
    }
}

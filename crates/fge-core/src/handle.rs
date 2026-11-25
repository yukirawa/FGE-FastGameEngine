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
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free_indices: Vec::new(),
            dense_to_sparse: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dense_to_sparse.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense_to_sparse.is_empty()
    }

    /// Allocates a new `EntityId` appended at the end of the dense array (`dense_idx == old_len`).
    pub fn allocate(&mut self) -> EntityId {
        let dense_index = u32::try_from(self.dense_to_sparse.len())
            .expect("GenerationalArena dense capacity exceeded u32::MAX");

        if let Some(sparse_index) = self.free_indices.pop() {
            let slot = &mut self.slots[sparse_index as usize];
            debug_assert!(!slot.alive);
            if slot.generation == 0 {
                slot.generation = 1;
            }
            slot.dense_index = dense_index;
            slot.alive = true;
            let generation = slot.generation;
            self.dense_to_sparse.push(sparse_index);
            EntityId {
                index: sparse_index,
                generation,
            }
        } else {
            assert!(
                self.slots.len() < u32::MAX as usize,
                "GenerationalArena sparse slot capacity exceeded"
            );
            let sparse_index = self.slots.len() as u32;
            let generation = 1;
            self.slots.push(SparseSlot {
                dense_index,
                generation,
                alive: true,
            });
            self.dense_to_sparse.push(sparse_index);
            EntityId {
                index: sparse_index,
                generation,
            }
        }
    }

    /// Resolves `id` to its current dense SoA index in $O(1)$, returning `None` if stale, dead, or out-of-bounds.
    #[inline]
    pub fn resolve(&self, id: EntityId) -> Option<usize> {
        if id.is_null() {
            return None;
        }
        let slot = self.slots.get(id.index as usize)?;
        if slot.alive && slot.generation == id.generation {
            Some(slot.dense_index as usize)
        } else {
            None
        }
    }

    /// Returns the live `EntityId` currently residing at `dense_index`.
    #[inline]
    pub fn entity_at_dense(&self, dense_index: usize) -> Option<EntityId> {
        let &sparse_index = self.dense_to_sparse.get(dense_index)?;
        let slot = self.slots.get(sparse_index as usize)?;
        if slot.alive {
            Some(EntityId {
                index: sparse_index,
                generation: slot.generation,
            })
        } else {
            None
        }
    }

    /// Deallocates `id` in $O(1)`, updating sparse redirection for the swapped tail element.
    /// Returns `Some(SwapRemoveRecord)` if `id` was alive, or `None` if invalid/already despawned.
    pub fn deallocate(&mut self, id: EntityId) -> Option<SwapRemoveRecord> {
        let removed_dense = self.resolve(id)?;
        let slot = &mut self.slots[id.index as usize];
        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free_indices.push(id.index);

        self.dense_to_sparse.swap_remove(removed_dense);
        if removed_dense < self.dense_to_sparse.len() {
            let moved_sparse = self.dense_to_sparse[removed_dense];
            self.slots[moved_sparse as usize].dense_index = removed_dense as u32;
        }

        Some(SwapRemoveRecord { removed_dense })
    }

    /// Resets dense occupancy while incrementing generations of previously live slots so old handles stay invalid!
    pub fn clear_invalidating_handles(&mut self) {
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.alive {
                slot.alive = false;
                slot.generation = slot.generation.wrapping_add(1).max(1);
                self.free_indices.push(idx as u32);
            }
        }
        self.dense_to_sparse.clear();
    }
}

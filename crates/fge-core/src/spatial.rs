use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::{camera::Aabb2D, error::CoreError, handle::EntityId, world::EntityWorld};

/// Narrowphase circle-circle collision event (`#[repr(C)]` identical to `FgeCollisionPair` in `fge.h`).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollisionPair {
    pub a: EntityId,
    pub b: EntityId,
}

const _: () = assert!(core::mem::size_of::<CollisionPair>() == 16);

/// Zero-allocation Compressed-Sparse-Row (CSR) Spatial Hash Grid for $O(N)$ broadphase & narrowphase queries.
#[derive(Clone, Debug)]
pub struct SpatialHashGrid {
    pub(crate) cell_size: f32,
    pub(crate) inv_cell_size: f32,
    pub(crate) bucket_mask: usize,
    pub(crate) cell_counts: Vec<u32>,
    pub(crate) cell_offsets: Vec<u32>,
    pub(crate) cell_cursors: Vec<u32>,
    pub(crate) entries: Vec<u32>,
    pub(crate) seen_stamp: Vec<u32>,
    pub(crate) current_stamp: u32,
}

impl SpatialHashGrid {
    /// Creates a flat CSR spatial hash grid with `cell_size > 0.0` and power-of-two `buckets` (e.g. `4096` or `16384`).
    pub fn new(_cell_size: f32, _buckets: usize) -> Result<Self, CoreError> {
        todo!("SpatialHashGrid::new")
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    pub fn bucket_count(&self) -> usize {
        self.cell_counts.len()
    }

    /// Rebuilds the CSR grid from `world` (inserting every entity with `ENTITY_FLAG_COLLIDABLE` into overlapping cells).
    pub fn rebuild(&mut self, _world: &EntityWorld) {
        todo!("SpatialHashGrid::rebuild")
    }

    /// Appends unique dense indices of collidable entities whose circle overlaps `aabb` and matches `layer_mask` (`(mask & layer_mask) != 0`).
    pub fn query_aabb_dense(
        &mut self,
        _world: &EntityWorld,
        _aabb: Aabb2D,
        _layer_mask: u32,
        _out_dense: &mut Vec<u32>,
    ) {
        todo!("SpatialHashGrid::query_aabb_dense")
    }

    /// Appends unique `EntityId`s whose circle overlaps `(center, radius)` and matches `layer_mask`.
    pub fn query_circle(
        &mut self,
        _world: &EntityWorld,
        _center: Vec2,
        _radius: f32,
        _layer_mask: u32,
        _out_entities: &mut Vec<EntityId>,
    ) -> usize {
        todo!("SpatialHashGrid::query_circle")
    }

    /// Rebuilds the grid and detects all unique colliding pairs `(a, b)` where `(layer[a] & mask_a) != 0`
    /// and `(layer[b] & mask_b) != 0`, avoiding duplicate `(b, a)` pairs when `mask_a` and `mask_b` overlap.
    pub fn detect_collisions(
        &mut self,
        _world: &EntityWorld,
        _mask_a: u32,
        _mask_b: u32,
        _out_pairs: &mut Vec<CollisionPair>,
    ) -> usize {
        todo!("SpatialHashGrid::detect_collisions")
    }
}

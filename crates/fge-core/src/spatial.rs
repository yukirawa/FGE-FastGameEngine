use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::{
    camera::Aabb2D,
    error::CoreError,
    handle::EntityId,
    world::{EntityWorld, ENTITY_FLAG_COLLIDABLE},
};

/// Narrowphase circle-circle collision event (`#[repr(C)]` identical to `FgeCollisionPair` in `fge.h`).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollisionPair {
    pub a: EntityId,
    pub b: EntityId,
}

const _: () = assert!(core::mem::size_of::<CollisionPair>() == 16);

#[inline(always)]
fn cell_coord(value: f32, inv_cell_size: f32) -> i32 {
    (value * inv_cell_size).floor() as i32
}

#[inline(always)]
fn hash_cell(cx: i32, cy: i32, bucket_mask: usize) -> usize {
    (((cx as u32).wrapping_mul(0x9E37_79B1) ^ (cy as u32).wrapping_mul(0x85EB_CA77)) as usize)
        & bucket_mask
}

#[inline(always)]
fn span_exceeds_buckets(
    min_cx: i32,
    max_cx: i32,
    min_cy: i32,
    max_cy: i32,
    buckets: usize,
) -> bool {
    let width = (max_cx as i64) - (min_cx as i64) + 1;
    let height = (max_cy as i64) - (min_cy as i64) + 1;
    width.saturating_mul(height) >= buckets as i64
}

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
    pub fn new(cell_size: f32, buckets: usize) -> Result<Self, CoreError> {
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err(CoreError::InvalidCellSize { cell_size });
        }
        if buckets == 0 || !buckets.is_power_of_two() {
            return Err(CoreError::InvalidBucketCount { buckets });
        }

        Ok(Self {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            bucket_mask: buckets - 1,
            cell_counts: vec![0; buckets],
            cell_offsets: vec![0; buckets + 1],
            cell_cursors: vec![0; buckets],
            entries: Vec::new(),
            seen_stamp: Vec::new(),
            current_stamp: 0,
        })
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    pub fn bucket_count(&self) -> usize {
        self.cell_counts.len()
    }

    #[inline]
    fn next_stamp(&mut self) -> u32 {
        self.current_stamp = self.current_stamp.wrapping_add(1);
        if self.current_stamp == 0 {
            self.seen_stamp.fill(0);
            self.current_stamp = 1;
        }
        self.current_stamp
    }

    /// Rebuilds the CSR grid from `world` (inserting every entity with `ENTITY_FLAG_COLLIDABLE` into overlapping cells).
    pub fn rebuild(&mut self, world: &EntityWorld) {
        let count = world.len();
        let buckets = self.cell_counts.len();
        let inv_cell_size = self.inv_cell_size;
        let bucket_mask = self.bucket_mask;

        if self.seen_stamp.len() < count {
            self.seen_stamp.resize(count, 0);
        }

        self.cell_counts.fill(0);

        // Pass 1: count entries per hash bucket.
        for i in 0..count {
            if (world.flags[i] & ENTITY_FLAG_COLLIDABLE) == 0 {
                continue;
            }
            let r = world.collider_radius[i];
            let px = world.pos_x[i];
            let py = world.pos_y[i];
            if !r.is_finite() || r <= 0.0 || !px.is_finite() || !py.is_finite() {
                continue;
            }
            let min_cx = cell_coord(px - r, inv_cell_size);
            let max_cx = cell_coord(px + r, inv_cell_size);
            let min_cy = cell_coord(py - r, inv_cell_size);
            let max_cy = cell_coord(py + r, inv_cell_size);
            for cy in min_cy..=max_cy {
                for cx in min_cx..=max_cx {
                    let bucket = hash_cell(cx, cy, bucket_mask);
                    self.cell_counts[bucket] += 1;
                }
            }
        }

        // Prefix sum into CSR offsets.
        let mut total: u32 = 0;
        for b in 0..buckets {
            self.cell_offsets[b] = total;
            total += self.cell_counts[b];
        }
        self.cell_offsets[buckets] = total;

        let total_usize = total as usize;
        self.entries.resize(total_usize, 0);
        self.cell_cursors
            .copy_from_slice(&self.cell_offsets[..buckets]);

        if total_usize == 0 {
            return;
        }

        // Pass 2: scatter dense entity indices into CSR entries.
        for i in 0..count {
            if (world.flags[i] & ENTITY_FLAG_COLLIDABLE) == 0 {
                continue;
            }
            let r = world.collider_radius[i];
            let px = world.pos_x[i];
            let py = world.pos_y[i];
            if !r.is_finite() || r <= 0.0 || !px.is_finite() || !py.is_finite() {
                continue;
            }
            let min_cx = cell_coord(px - r, inv_cell_size);
            let max_cx = cell_coord(px + r, inv_cell_size);
            let min_cy = cell_coord(py - r, inv_cell_size);
            let max_cy = cell_coord(py + r, inv_cell_size);
            let dense_idx = i as u32;
            for cy in min_cy..=max_cy {
                for cx in min_cx..=max_cx {
                    let bucket = hash_cell(cx, cy, bucket_mask);
                    let dst = self.cell_cursors[bucket] as usize;
                    self.entries[dst] = dense_idx;
                    self.cell_cursors[bucket] += 1;
                }
            }
        }
    }

    /// Appends unique dense indices of collidable entities whose circle overlaps `aabb` and matches `layer_mask` (`(mask & layer_mask) != 0`).
    pub fn query_aabb_dense(
        &mut self,
        world: &EntityWorld,
        aabb: Aabb2D,
        layer_mask: u32,
        out_dense: &mut Vec<u32>,
    ) {
        out_dense.clear();

        let world_len = world.len();
        if layer_mask == 0
            || world_len == 0
            || self.entries.is_empty()
            || !aabb.min.is_finite()
            || !aabb.max.is_finite()
            || aabb.min.x > aabb.max.x
            || aabb.min.y > aabb.max.y
        {
            return;
        }

        if self.seen_stamp.len() < world_len {
            self.seen_stamp.resize(world_len, 0);
        }

        let stamp = self.next_stamp();
        let buckets = self.cell_counts.len();
        let inv_cell_size = self.inv_cell_size;
        let bucket_mask = self.bucket_mask;

        let min_cx = cell_coord(aabb.min.x, inv_cell_size);
        let max_cx = cell_coord(aabb.max.x, inv_cell_size);
        let min_cy = cell_coord(aabb.min.y, inv_cell_size);
        let max_cy = cell_coord(aabb.max.y, inv_cell_size);

        let offsets = &self.cell_offsets;
        let entries = &self.entries;
        let seen = &mut self.seen_stamp;

        let mut check_candidate = |entry: u32| {
            let idx = entry as usize;
            if idx >= world_len || seen[idx] == stamp {
                return;
            }
            seen[idx] = stamp;

            if (world.flags[idx] & ENTITY_FLAG_COLLIDABLE) == 0
                || (world.layer_mask[idx] & layer_mask) == 0
            {
                return;
            }
            let r = world.collider_radius[idx];
            let px = world.pos_x[idx];
            let py = world.pos_y[idx];
            if !r.is_finite() || r <= 0.0 || !px.is_finite() || !py.is_finite() {
                return;
            }
            let pos = Vec2::new(px, py);
            let closest = pos.clamp(aabb.min, aabb.max);
            if pos.distance_squared(closest) <= r * r {
                out_dense.push(entry);
            }
        };

        if span_exceeds_buckets(min_cx, max_cx, min_cy, max_cy, buckets) {
            for &entry in entries {
                check_candidate(entry);
            }
        } else {
            for cy in min_cy..=max_cy {
                for cx in min_cx..=max_cx {
                    let bucket = hash_cell(cx, cy, bucket_mask);
                    let start = offsets[bucket] as usize;
                    let end = offsets[bucket + 1] as usize;
                    for &entry in &entries[start..end] {
                        check_candidate(entry);
                    }
                }
            }
        }
    }

    /// Appends unique `EntityId`s whose circle overlaps `(center, radius)` and matches `layer_mask`.
    pub fn query_circle(
        &mut self,
        world: &EntityWorld,
        center: Vec2,
        radius: f32,
        layer_mask: u32,
        out_entities: &mut Vec<EntityId>,
    ) -> usize {
        out_entities.clear();

        let world_len = world.len();
        if layer_mask == 0
            || world_len == 0
            || self.entries.is_empty()
            || !center.is_finite()
            || !radius.is_finite()
            || radius < 0.0
        {
            return 0;
        }

        if self.seen_stamp.len() < world_len {
            self.seen_stamp.resize(world_len, 0);
        }

        let stamp = self.next_stamp();
        let buckets = self.cell_counts.len();
        let inv_cell_size = self.inv_cell_size;
        let bucket_mask = self.bucket_mask;

        let min_cx = cell_coord(center.x - radius, inv_cell_size);
        let max_cx = cell_coord(center.x + radius, inv_cell_size);
        let min_cy = cell_coord(center.y - radius, inv_cell_size);
        let max_cy = cell_coord(center.y + radius, inv_cell_size);

        let offsets = &self.cell_offsets;
        let entries = &self.entries;
        let seen = &mut self.seen_stamp;

        let mut check_candidate = |entry: u32| {
            let idx = entry as usize;
            if idx >= world_len || seen[idx] == stamp {
                return;
            }
            seen[idx] = stamp;

            if (world.flags[idx] & ENTITY_FLAG_COLLIDABLE) == 0
                || (world.layer_mask[idx] & layer_mask) == 0
            {
                return;
            }
            let ri = world.collider_radius[idx];
            let px = world.pos_x[idx];
            let py = world.pos_y[idx];
            if !ri.is_finite() || ri <= 0.0 || !px.is_finite() || !py.is_finite() {
                return;
            }
            let dx = px - center.x;
            let dy = py - center.y;
            let sum_r = radius + ri;
            if dx * dx + dy * dy <= sum_r * sum_r {
                out_entities.push(world.entities[idx]);
            }
        };

        if span_exceeds_buckets(min_cx, max_cx, min_cy, max_cy, buckets) {
            for &entry in entries {
                check_candidate(entry);
            }
        } else {
            for cy in min_cy..=max_cy {
                for cx in min_cx..=max_cx {
                    let bucket = hash_cell(cx, cy, bucket_mask);
                    let start = offsets[bucket] as usize;
                    let end = offsets[bucket + 1] as usize;
                    for &entry in &entries[start..end] {
                        check_candidate(entry);
                    }
                }
            }
        }

        out_entities.len()
    }

    /// Rebuilds the grid and detects all unique colliding pairs `(a, b)` where `(layer[a] & mask_a) != 0`
    /// and `(layer[b] & mask_b) != 0`, avoiding duplicate `(b, a)` pairs when `mask_a` and `mask_b` overlap.
    pub fn detect_collisions(
        &mut self,
        world: &EntityWorld,
        mask_a: u32,
        mask_b: u32,
        out_pairs: &mut Vec<CollisionPair>,
    ) -> usize {
        out_pairs.clear();
        self.rebuild(world);

        let world_len = world.len();
        if mask_a == 0 || mask_b == 0 || world_len < 2 || self.entries.is_empty() {
            return 0;
        }

        let buckets = self.cell_counts.len();
        let inv_cell_size = self.inv_cell_size;
        let bucket_mask = self.bucket_mask;

        for a in 0..world_len {
            if (world.flags[a] & ENTITY_FLAG_COLLIDABLE) == 0 {
                continue;
            }
            let layer_a = world.layer_mask[a];
            if (layer_a & mask_a) == 0 {
                continue;
            }
            let ra = world.collider_radius[a];
            let ax = world.pos_x[a];
            let ay = world.pos_y[a];
            if !ra.is_finite() || ra <= 0.0 || !ax.is_finite() || !ay.is_finite() {
                continue;
            }

            let stamp = self.next_stamp();
            let a_matches_b = (layer_a & mask_b) != 0;
            let entity_a = world.entities[a];

            let offsets = &self.cell_offsets;
            let entries = &self.entries;
            let seen = &mut self.seen_stamp;
            seen[a] = stamp;

            let min_cx = cell_coord(ax - ra, inv_cell_size);
            let max_cx = cell_coord(ax + ra, inv_cell_size);
            let min_cy = cell_coord(ay - ra, inv_cell_size);
            let max_cy = cell_coord(ay + ra, inv_cell_size);

            let mut check_candidate = |b_u32: u32| {
                let b = b_u32 as usize;
                if seen[b] == stamp {
                    return;
                }
                seen[b] = stamp;

                let layer_b = world.layer_mask[b];
                if (layer_b & mask_b) == 0 {
                    return;
                }
                if a_matches_b && (layer_b & mask_a) != 0 && b < a {
                    return;
                }

                let rb = world.collider_radius[b];
                let dx = world.pos_x[b] - ax;
                let dy = world.pos_y[b] - ay;
                let sum_r = ra + rb;
                if dx * dx + dy * dy <= sum_r * sum_r {
                    out_pairs.push(CollisionPair {
                        a: entity_a,
                        b: world.entities[b],
                    });
                }
            };

            if span_exceeds_buckets(min_cx, max_cx, min_cy, max_cy, buckets) {
                for &b_u32 in entries {
                    check_candidate(b_u32);
                }
            } else {
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        let bucket = hash_cell(cx, cy, bucket_mask);
                        let start = offsets[bucket] as usize;
                        let end = offsets[bucket + 1] as usize;
                        for &b_u32 in &entries[start..end] {
                            check_candidate(b_u32);
                        }
                    }
                }
            }
        }

        out_pairs.len()
    }
}

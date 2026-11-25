use crate::{
    error::CoreError,
    handle::{EntityId, GenerationalArena},
    sim::FULL_UV_RECT,
};

pub const ENTITY_FLAG_VISIBLE: u32 = 1 << 0;
pub const ENTITY_FLAG_COLLIDABLE: u32 = 1 << 1;
pub const ENTITY_FLAG_KINEMATIC: u32 = 1 << 2;
pub const ENTITY_FLAG_Y_SORT: u32 = 1 << 3;

pub const DEFAULT_ENTITY_FLAGS: u32 = ENTITY_FLAG_VISIBLE | ENTITY_FLAG_KINEMATIC;

/// Packed Structure-of-Arrays (SoA) gameplay entity store guarded by `GenerationalArena`.
#[derive(Clone, Debug, Default)]
pub struct EntityWorld {
    pub(crate) arena: GenerationalArena,
    pub entities: Vec<EntityId>,
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    pub size_w: Vec<f32>,
    pub size_h: Vec<f32>,
    pub rotation: Vec<f32>,
    pub ang_vel: Vec<f32>,
    pub collider_radius: Vec<f32>,
    pub layer_mask: Vec<u32>,
    pub tag: Vec<u32>,
    pub hp: Vec<f32>,
    pub flags: Vec<u32>,
    pub z_order: Vec<f32>,
    pub uv_rect: Vec<[u16; 4]>,
    pub color_rgba: Vec<[u8; 4]>,
}

impl EntityWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: GenerationalArena::with_capacity(capacity),
            entities: Vec::with_capacity(capacity),
            pos_x: Vec::with_capacity(capacity),
            pos_y: Vec::with_capacity(capacity),
            vel_x: Vec::with_capacity(capacity),
            vel_y: Vec::with_capacity(capacity),
            size_w: Vec::with_capacity(capacity),
            size_h: Vec::with_capacity(capacity),
            rotation: Vec::with_capacity(capacity),
            ang_vel: Vec::with_capacity(capacity),
            collider_radius: Vec::with_capacity(capacity),
            layer_mask: Vec::with_capacity(capacity),
            tag: Vec::with_capacity(capacity),
            hp: Vec::with_capacity(capacity),
            flags: Vec::with_capacity(capacity),
            z_order: Vec::with_capacity(capacity),
            uv_rect: Vec::with_capacity(capacity),
            color_rgba: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    #[inline]
    pub fn is_alive(&self, id: EntityId) -> bool {
        self.arena.resolve(id).is_some()
    }

    #[inline]
    pub fn dense_index_of(&self, id: EntityId) -> Option<usize> {
        self.arena.resolve(id)
    }

    /// Spawns a default entity at `(0, 0)` with size `(16, 16)`, white color, `FULL_UV_RECT`, and `DEFAULT_ENTITY_FLAGS`.
    pub fn spawn(&mut self) -> EntityId {
        self.spawn_sprite([0.0, 0.0], [16.0, 16.0], [255, 255, 255, 255])
    }

    /// Spawns a visible kinematic sprite entity at `pos` with `size` and `color`.
    pub fn spawn_sprite(&mut self, pos: [f32; 2], size: [f32; 2], color: [u8; 4]) -> EntityId {
        let id = self.arena.allocate();
        self.entities.push(id);
        self.pos_x.push(pos[0]);
        self.pos_y.push(pos[1]);
        self.vel_x.push(0.0);
        self.vel_y.push(0.0);
        self.size_w.push(size[0]);
        self.size_h.push(size[1]);
        self.rotation.push(0.0);
        self.ang_vel.push(0.0);
        self.collider_radius.push(0.0);
        self.layer_mask.push(0);
        self.tag.push(0);
        self.hp.push(0.0);
        self.flags.push(DEFAULT_ENTITY_FLAGS);
        self.z_order.push(0.0);
        self.uv_rect.push(FULL_UV_RECT);
        self.color_rgba.push(color);
        debug_assert!(self.has_consistent_lengths());
        id
    }

    /// Despawns `id` in $O(1)$ via `swap_remove` across all SoA columns. Returns `false` if `id` is stale/invalid.
    pub fn despawn(&mut self, id: EntityId) -> bool {
        let Some(rec) = self.arena.deallocate(id) else {
            return false;
        };
        let idx = rec.removed_dense;
        self.entities.swap_remove(idx);
        self.pos_x.swap_remove(idx);
        self.pos_y.swap_remove(idx);
        self.vel_x.swap_remove(idx);
        self.vel_y.swap_remove(idx);
        self.size_w.swap_remove(idx);
        self.size_h.swap_remove(idx);
        self.rotation.swap_remove(idx);
        self.ang_vel.swap_remove(idx);
        self.collider_radius.swap_remove(idx);
        self.layer_mask.swap_remove(idx);
        self.tag.swap_remove(idx);
        self.hp.swap_remove(idx);
        self.flags.swap_remove(idx);
        self.z_order.swap_remove(idx);
        self.uv_rect.swap_remove(idx);
        self.color_rgba.swap_remove(idx);
        debug_assert!(self.has_consistent_lengths());
        true
    }

    /// Clears all entities while invalidating previously issued handles.
    pub fn clear(&mut self) {
        self.arena.clear_invalidating_handles();
        self.entities.clear();
        self.pos_x.clear();
        self.pos_y.clear();
        self.vel_x.clear();
        self.vel_y.clear();
        self.size_w.clear();
        self.size_h.clear();
        self.rotation.clear();
        self.ang_vel.clear();
        self.collider_radius.clear();
        self.layer_mask.clear();
        self.tag.clear();
        self.hp.clear();
        self.flags.clear();
        self.z_order.clear();
        self.uv_rect.clear();
        self.color_rgba.clear();
        debug_assert!(self.has_consistent_lengths());
    }

    #[inline]
    pub fn set_position(&mut self, id: EntityId, pos: [f32; 2]) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.pos_x[idx] = pos[0];
        self.pos_y[idx] = pos[1];
        true
    }

    #[inline]
    pub fn get_position(&self, id: EntityId) -> Option<[f32; 2]> {
        let idx = self.arena.resolve(id)?;
        Some([self.pos_x[idx], self.pos_y[idx]])
    }

    #[inline]
    pub fn set_velocity(&mut self, id: EntityId, vel: [f32; 2]) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.vel_x[idx] = vel[0];
        self.vel_y[idx] = vel[1];
        true
    }

    #[inline]
    pub fn get_velocity(&self, id: EntityId) -> Option<[f32; 2]> {
        let idx = self.arena.resolve(id)?;
        Some([self.vel_x[idx], self.vel_y[idx]])
    }

    #[inline]
    pub fn set_size(&mut self, id: EntityId, size: [f32; 2]) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.size_w[idx] = size[0];
        self.size_h[idx] = size[1];
        true
    }

    #[inline]
    pub fn get_size(&self, id: EntityId) -> Option<[f32; 2]> {
        let idx = self.arena.resolve(id)?;
        Some([self.size_w[idx], self.size_h[idx]])
    }

    #[inline]
    pub fn set_rotation(&mut self, id: EntityId, radians: f32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.rotation[idx] = radians;
        true
    }

    #[inline]
    pub fn get_rotation(&self, id: EntityId) -> Option<f32> {
        let idx = self.arena.resolve(id)?;
        Some(self.rotation[idx])
    }

    #[inline]
    pub fn set_angular_velocity(&mut self, id: EntityId, ang_vel: f32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.ang_vel[idx] = ang_vel;
        true
    }

    #[inline]
    pub fn set_color(&mut self, id: EntityId, color: [u8; 4]) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.color_rgba[idx] = color;
        true
    }

    #[inline]
    pub fn get_color(&self, id: EntityId) -> Option<[u8; 4]> {
        let idx = self.arena.resolve(id)?;
        Some(self.color_rgba[idx])
    }

    #[inline]
    pub fn set_uv_rect(&mut self, id: EntityId, uv: [u16; 4]) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.uv_rect[idx] = uv;
        true
    }

    #[inline]
    pub fn set_z_order(&mut self, id: EntityId, z: f32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.z_order[idx] = z;
        true
    }

    #[inline]
    pub fn set_flags(&mut self, id: EntityId, flags: u32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.flags[idx] = flags;
        true
    }

    #[inline]
    pub fn get_flags(&self, id: EntityId) -> Option<u32> {
        let idx = self.arena.resolve(id)?;
        Some(self.flags[idx])
    }

    /// Sets circle collider `radius` (`>= 0`) and `layer_mask`, automatically enabling `ENTITY_FLAG_COLLIDABLE` when `radius > 0.0 && layer_mask != 0`.
    #[inline]
    pub fn set_collider(&mut self, id: EntityId, radius: f32, layer_mask: u32) -> bool {
        if !radius.is_finite() || radius < 0.0 {
            return false;
        }
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.collider_radius[idx] = radius;
        self.layer_mask[idx] = layer_mask;
        if radius > 0.0 && layer_mask != 0 {
            self.flags[idx] |= ENTITY_FLAG_COLLIDABLE;
        } else {
            self.flags[idx] &= !ENTITY_FLAG_COLLIDABLE;
        }
        true
    }

    #[inline]
    pub fn set_tag(&mut self, id: EntityId, tag: u32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.tag[idx] = tag;
        true
    }

    #[inline]
    pub fn get_tag(&self, id: EntityId) -> Option<u32> {
        let idx = self.arena.resolve(id)?;
        Some(self.tag[idx])
    }

    #[inline]
    pub fn set_hp(&mut self, id: EntityId, hp: f32) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.hp[idx] = hp;
        true
    }

    #[inline]
    pub fn get_hp(&self, id: EntityId) -> Option<f32> {
        let idx = self.arena.resolve(id)?;
        Some(self.hp[idx])
    }

    /// Integrates `pos += vel * dt` and `rotation += ang_vel * dt` for all entities with `ENTITY_FLAG_KINEMATIC`.
    pub fn step_kinematics(&mut self, dt: f32) -> Result<(), CoreError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(CoreError::InvalidDeltaTime { dt });
        }
        debug_assert!(self.has_consistent_lengths());
        if dt == 0.0 || self.entities.is_empty() {
            return Ok(());
        }

        let len = self.entities.len();
        let flags = &self.flags[..len];
        integrate_kinematic_column(&mut self.pos_x[..len], &self.vel_x[..len], flags, dt);
        integrate_kinematic_column(&mut self.pos_y[..len], &self.vel_y[..len], flags, dt);
        integrate_kinematic_column(&mut self.rotation[..len], &self.ang_vel[..len], flags, dt);

        Ok(())
    }

    /// Verifies that `arena.len()` and every SoA column share the exact same length and valid handles.
    pub fn debug_assert_invariants(&self) -> bool {
        if !self.has_consistent_lengths() {
            return false;
        }
        for (dense_idx, &id) in self.entities.iter().enumerate() {
            if self.arena.resolve(id) != Some(dense_idx) {
                return false;
            }
            if self.arena.entity_at_dense(dense_idx) != Some(id) {
                return false;
            }
        }
        true
    }

    #[inline]
    fn has_consistent_lengths(&self) -> bool {
        let len = self.entities.len();
        self.arena.len() == len
            && self.pos_x.len() == len
            && self.pos_y.len() == len
            && self.vel_x.len() == len
            && self.vel_y.len() == len
            && self.size_w.len() == len
            && self.size_h.len() == len
            && self.rotation.len() == len
            && self.ang_vel.len() == len
            && self.collider_radius.len() == len
            && self.layer_mask.len() == len
            && self.tag.len() == len
            && self.hp.len() == len
            && self.flags.len() == len
            && self.z_order.len() == len
            && self.uv_rect.len() == len
            && self.color_rgba.len() == len
    }
}

/// Branchless, auto-vectorizable kinematic integration across contiguous SoA slices.
#[inline]
fn integrate_kinematic_column(values: &mut [f32], rates: &[f32], flags: &[u32], dt: f32) {
    for ((val, &rate), &f) in values.iter_mut().zip(rates.iter()).zip(flags.iter()) {
        let m = if (f & ENTITY_FLAG_KINEMATIC) != 0 {
            dt
        } else {
            0.0
        };
        *val += rate * m;
    }
}

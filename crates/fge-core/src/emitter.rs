use std::f32::consts::TAU;

use crate::{
    handle::{EntityId, GenerationalArena},
    world::EntityWorld,
};

/// 64-bit generational emitter handle (`FgeEmitter` in `fge.h`).
pub type EmitterId = EntityId;

/// Safety cap on bursts fired by a single emitter in one `step_emitters` call to prevent runaway allocation.
const MAX_BURSTS_PER_STEP: u32 = 1024;

/// Configuration for a bulk radial/spiral bullet or particle emitter offloaded to Rust.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EmitterSpec {
    pub pos: [f32; 2],
    pub rate_per_sec: f32,
    pub count_per_burst: u32,
    pub speed: f32,
    pub bullet_ttl: f32,
    pub collider_radius: f32,
    pub layer_mask: u32,
    pub size: [f32; 2],
    pub color_rgba: [u8; 4],
    pub base_angle_rad: f32,
    pub angular_vel_rad: f32,
    pub active: bool,
}

/// Packed Structure-of-Arrays (SoA) emitter store guarded by `GenerationalArena`.
#[derive(Clone, Debug, Default)]
pub struct EmitterWorld {
    pub(crate) arena: GenerationalArena,
    pub emitters: Vec<EmitterId>,
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub rate: Vec<f32>,
    pub accumulator: Vec<f32>,
    pub burst_count: Vec<u32>,
    pub speed: Vec<f32>,
    pub bullet_ttl: Vec<f32>,
    pub collider_radius: Vec<f32>,
    pub layer_mask: Vec<u32>,
    pub size: Vec<[f32; 2]>,
    pub color_rgba: Vec<[u8; 4]>,
    pub base_angle: Vec<f32>,
    pub ang_vel: Vec<f32>,
    pub active: Vec<bool>,
}

impl EmitterWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: GenerationalArena::with_capacity(capacity),
            emitters: Vec::with_capacity(capacity),
            pos_x: Vec::with_capacity(capacity),
            pos_y: Vec::with_capacity(capacity),
            rate: Vec::with_capacity(capacity),
            accumulator: Vec::with_capacity(capacity),
            burst_count: Vec::with_capacity(capacity),
            speed: Vec::with_capacity(capacity),
            bullet_ttl: Vec::with_capacity(capacity),
            collider_radius: Vec::with_capacity(capacity),
            layer_mask: Vec::with_capacity(capacity),
            size: Vec::with_capacity(capacity),
            color_rgba: Vec::with_capacity(capacity),
            base_angle: Vec::with_capacity(capacity),
            ang_vel: Vec::with_capacity(capacity),
            active: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.emitters.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.emitters.is_empty()
    }

    #[inline]
    pub fn is_alive(&self, id: EmitterId) -> bool {
        self.arena.resolve(id).is_some()
    }

    #[inline]
    pub fn dense_index_of(&self, id: EmitterId) -> Option<usize> {
        self.arena.resolve(id)
    }

    /// Creates a new radial/spiral emitter and returns its generational [`EmitterId`].
    pub fn create_radial(&mut self, spec: EmitterSpec) -> EmitterId {
        let px = if spec.pos[0].is_finite() {
            spec.pos[0]
        } else {
            0.0
        };
        let py = if spec.pos[1].is_finite() {
            spec.pos[1]
        } else {
            0.0
        };
        let rate = if spec.rate_per_sec.is_finite() && spec.rate_per_sec > 0.0 {
            spec.rate_per_sec
        } else {
            0.0
        };
        let speed = if spec.speed.is_finite() {
            spec.speed
        } else {
            0.0
        };
        let bullet_ttl = if spec.bullet_ttl.is_finite() && spec.bullet_ttl > 0.0 {
            spec.bullet_ttl
        } else {
            0.0
        };
        let collider_radius = if spec.collider_radius.is_finite() && spec.collider_radius > 0.0 {
            spec.collider_radius
        } else {
            0.0
        };
        let default_dim = (collider_radius * 2.0).max(4.0);
        let size = [
            if spec.size[0].is_finite() && spec.size[0] > 0.0 {
                spec.size[0]
            } else {
                default_dim
            },
            if spec.size[1].is_finite() && spec.size[1] > 0.0 {
                spec.size[1]
            } else {
                default_dim
            },
        ];
        let base_angle = if spec.base_angle_rad.is_finite() {
            spec.base_angle_rad
        } else {
            0.0
        };
        let ang_vel = if spec.angular_vel_rad.is_finite() {
            spec.angular_vel_rad
        } else {
            0.0
        };

        let id = self.arena.allocate();
        self.emitters.push(id);
        self.pos_x.push(px);
        self.pos_y.push(py);
        self.rate.push(rate);
        self.accumulator.push(0.0);
        self.burst_count.push(spec.count_per_burst);
        self.speed.push(speed);
        self.bullet_ttl.push(bullet_ttl);
        self.collider_radius.push(collider_radius);
        self.layer_mask.push(spec.layer_mask);
        self.size.push(size);
        self.color_rgba.push(spec.color_rgba);
        self.base_angle.push(base_angle);
        self.ang_vel.push(ang_vel);
        self.active.push(spec.active);
        debug_assert!(self.has_consistent_lengths());
        id
    }

    #[inline]
    pub fn set_position(&mut self, id: EmitterId, x: f32, y: f32) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.pos_x[idx] = x;
        self.pos_y[idx] = y;
        true
    }

    #[inline]
    pub fn set_spin(&mut self, id: EmitterId, base_angle_rad: f32, angular_vel_rad: f32) -> bool {
        if !base_angle_rad.is_finite() || !angular_vel_rad.is_finite() {
            return false;
        }
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.base_angle[idx] = base_angle_rad;
        self.ang_vel[idx] = angular_vel_rad;
        true
    }

    #[inline]
    pub fn set_active(&mut self, id: EmitterId, active: bool) -> bool {
        let Some(idx) = self.arena.resolve(id) else {
            return false;
        };
        self.active[idx] = active;
        true
    }

    /// Destroys `id` in $O(1)$ via `swap_remove` across all SoA columns. Returns `false` if `id` is stale/invalid.
    pub fn destroy(&mut self, id: EmitterId) -> bool {
        let Some(rec) = self.arena.deallocate(id) else {
            return false;
        };
        let idx = rec.removed_dense;
        self.emitters.swap_remove(idx);
        self.pos_x.swap_remove(idx);
        self.pos_y.swap_remove(idx);
        self.rate.swap_remove(idx);
        self.accumulator.swap_remove(idx);
        self.burst_count.swap_remove(idx);
        self.speed.swap_remove(idx);
        self.bullet_ttl.swap_remove(idx);
        self.collider_radius.swap_remove(idx);
        self.layer_mask.swap_remove(idx);
        self.size.swap_remove(idx);
        self.color_rgba.swap_remove(idx);
        self.base_angle.swap_remove(idx);
        self.ang_vel.swap_remove(idx);
        self.active.swap_remove(idx);
        debug_assert!(self.has_consistent_lengths());
        true
    }

    /// Clears all emitters while invalidating previously issued handles.
    pub fn clear(&mut self) {
        self.arena.clear_invalidating_handles();
        self.emitters.clear();
        self.pos_x.clear();
        self.pos_y.clear();
        self.rate.clear();
        self.accumulator.clear();
        self.burst_count.clear();
        self.speed.clear();
        self.bullet_ttl.clear();
        self.collider_radius.clear();
        self.layer_mask.clear();
        self.size.clear();
        self.color_rgba.clear();
        self.base_angle.clear();
        self.ang_vel.clear();
        self.active.clear();
        debug_assert!(self.has_consistent_lengths());
    }

    /// Advances active emitter spin angles and firing accumulators, batch-spawning radial/spiral
    /// bullets directly into `world` entirely in native Rust.
    pub fn step_emitters(&mut self, dt: f32, world: &mut EntityWorld) {
        if !dt.is_finite() || dt <= 0.0 || self.emitters.is_empty() {
            return;
        }
        debug_assert!(self.has_consistent_lengths());

        let len = self.emitters.len();
        for i in 0..len {
            if !self.active[i] {
                continue;
            }

            let start_angle = self.base_angle[i];
            let ang_vel = self.ang_vel[i];
            let rate = self.rate[i];
            let count = self.burst_count[i];

            if rate > 0.0 && count > 0 {
                let acc = self.accumulator[i] + dt * rate;
                let raw_bursts = acc.floor();
                if raw_bursts >= 1.0 {
                    let bursts = (raw_bursts as u32).min(MAX_BURSTS_PER_STEP);
                    self.accumulator[i] = (acc - raw_bursts).max(0.0);

                    let px = self.pos_x[i];
                    let py = self.pos_y[i];
                    let speed = self.speed[i];
                    let bullet_ttl = self.bullet_ttl[i];
                    let collider_radius = self.collider_radius[i];
                    let layer_mask = self.layer_mask[i];
                    let bullet_size = self.size[i];
                    let color = self.color_rgba[i];

                    let angle_step = TAU / (count as f32);
                    let burst_dt = 1.0 / rate;

                    for b in 0..bursts {
                        let burst_angle = start_angle + ang_vel * ((b as f32) * burst_dt);
                        for k in 0..count {
                            let angle = burst_angle + (k as f32) * angle_step;
                            let (sin_a, cos_a) = angle.sin_cos();
                            let eid = world.spawn_sprite([px, py], bullet_size, color);
                            world.set_velocity(eid, [cos_a * speed, sin_a * speed]);
                            world.set_rotation(eid, angle);
                            if collider_radius > 0.0 && layer_mask != 0 {
                                world.set_collider(eid, collider_radius, layer_mask);
                            }
                            if bullet_ttl > 0.0 {
                                world.set_ttl(eid, bullet_ttl);
                            }
                        }
                    }
                } else {
                    self.accumulator[i] = acc;
                }
            }

            self.base_angle[i] = (start_angle + ang_vel * dt).rem_euclid(TAU);
        }
    }

    /// Verifies that `arena.len()` and every SoA column share the exact same length and valid handles.
    pub fn debug_assert_invariants(&self) -> bool {
        if !self.has_consistent_lengths() {
            return false;
        }
        for (dense_idx, &id) in self.emitters.iter().enumerate() {
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
        let len = self.emitters.len();
        self.arena.len() == len
            && self.pos_x.len() == len
            && self.pos_y.len() == len
            && self.rate.len() == len
            && self.accumulator.len() == len
            && self.burst_count.len() == len
            && self.speed.len() == len
            && self.bullet_ttl.len() == len
            && self.collider_radius.len() == len
            && self.layer_mask.len() == len
            && self.size.len() == len
            && self.color_rgba.len() == len
            && self.base_angle.len() == len
            && self.ang_vel.len() == len
            && self.active.len() == len
    }
}

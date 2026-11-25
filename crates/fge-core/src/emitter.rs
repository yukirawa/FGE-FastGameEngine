use crate::{
    handle::{EntityId, GenerationalArena},
    world::EntityWorld,
};

/// 64-bit generational emitter handle (`FgeEmitter` in `fge.h`).
pub type EmitterId = EntityId;

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

    pub fn create_radial(&mut self, _spec: EmitterSpec) -> EmitterId {
        todo!("implemented by impl-core5")
    }

    pub fn set_position(&mut self, _id: EmitterId, _x: f32, _y: f32) -> bool {
        todo!("implemented by impl-core5")
    }

    pub fn set_spin(&mut self, _id: EmitterId, _base_angle_rad: f32, _angular_vel_rad: f32) -> bool {
        todo!("implemented by impl-core5")
    }

    pub fn set_active(&mut self, _id: EmitterId, _active: bool) -> bool {
        todo!("implemented by impl-core5")
    }

    pub fn destroy(&mut self, _id: EmitterId) -> bool {
        todo!("implemented by impl-core5")
    }

    pub fn step_emitters(&mut self, _dt: f32, _world: &mut EntityWorld) {
        todo!("implemented by impl-core5")
    }
}

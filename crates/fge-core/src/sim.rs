use crate::error::CoreError;

/// Default full-texture normalized UV rectangle (`0.0..=1.0` encoded as `u16`).
pub const FULL_UV_RECT: [u16; 4] = [0, 0, u16::MAX, u16::MAX];

/// Configuration parameters for spawning deterministic bulk 2D sprites/quads.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SimConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub min_size: f32,
    pub max_size: f32,
    pub max_speed: f32,
    pub max_angular_speed: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 1280.0,
            world_height: 720.0,
            min_size: 4.0,
            max_size: 16.0,
            max_speed: 240.0,
            max_angular_speed: core::f32::consts::PI,
        }
    }
}

impl SimConfig {
    /// Validates world bounds, sprite size range, and speed parameters.
    pub fn validate(&self) -> Result<(), CoreError> {
        todo!("SimConfig::validate")
    }
}

/// Trait for DOD/SoA bulk simulations executed natively in Rust.
pub trait BulkSimulation {
    /// Number of active simulated objects.
    fn len(&self) -> usize;

    /// Returns `true` when `len() == 0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Advances all objects by `dt` seconds within `[0.0, bounds_w] x [0.0, bounds_h]`,
    /// reflecting velocity components on boundary collisions.
    fn step(&mut self, dt: f32, bounds_w: f32, bounds_h: f32) -> Result<(), CoreError>;
}

/// Pure Structure-of-Arrays (SoA) container for 2D sprites/rectangles.
/// All parallel vectors MUST maintain identical length at all times.
#[derive(Clone, Debug, Default)]
pub struct SpriteSoA {
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    pub size_w: Vec<f32>,
    pub size_h: Vec<f32>,
    pub rotation: Vec<f32>,
    pub ang_vel: Vec<f32>,
    pub uv_rect: Vec<[u16; 4]>,
    pub color_rgba: Vec<[u8; 4]>,
}

impl SpriteSoA {
    /// Allocates empty parallel arrays with pre-reserved `capacity`.
    pub fn with_capacity(_capacity: usize) -> Self {
        todo!("SpriteSoA::with_capacity")
    }

    /// Deterministically populates `count` sprites using `seed` and `config`.
    pub fn spawn_random_batch(
        _count: usize,
        _seed: u64,
        _config: &SimConfig,
    ) -> Result<Self, CoreError> {
        todo!("SpriteSoA::spawn_random_batch")
    }

    /// Appends a single sprite across all parallel SoA columns.
    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        _pos: [f32; 2],
        _vel: [f32; 2],
        _size: [f32; 2],
        _rotation: f32,
        _ang_vel: f32,
        _uv_rect: [u16; 4],
        _color_rgba: [u8; 4],
    ) {
        todo!("SpriteSoA::push")
    }

    /// Clears all columns without releasing allocated capacity.
    pub fn clear(&mut self) {
        todo!("SpriteSoA::clear")
    }

    /// Verifies that every parallel column has the exact same length.
    pub fn debug_assert_soa_invariants(&self) -> bool {
        todo!("SpriteSoA::debug_assert_soa_invariants")
    }
}

impl BulkSimulation for SpriteSoA {
    fn len(&self) -> usize {
        todo!("SpriteSoA::len")
    }

    fn step(&mut self, _dt: f32, _bounds_w: f32, _bounds_h: f32) -> Result<(), CoreError> {
        todo!("SpriteSoA::step")
    }
}

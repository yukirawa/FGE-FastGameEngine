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
        if !self.world_width.is_finite()
            || !self.world_height.is_finite()
            || self.world_width <= 0.0
            || self.world_height <= 0.0
        {
            return Err(CoreError::InvalidDimensions {
                width: self.world_width,
                height: self.world_height,
            });
        }

        if !self.min_size.is_finite()
            || !self.max_size.is_finite()
            || self.min_size <= 0.0
            || self.max_size < self.min_size
        {
            return Err(CoreError::InvalidSizeRange {
                min: self.min_size,
                max: self.max_size,
            });
        }

        if !self.max_speed.is_finite() || self.max_speed < 0.0 {
            return Err(CoreError::InvalidSpeed {
                value: self.max_speed,
            });
        }

        if !self.max_angular_speed.is_finite() || self.max_angular_speed < 0.0 {
            return Err(CoreError::InvalidSpeed {
                value: self.max_angular_speed,
            });
        }

        Ok(())
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
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pos_x: Vec::with_capacity(capacity),
            pos_y: Vec::with_capacity(capacity),
            vel_x: Vec::with_capacity(capacity),
            vel_y: Vec::with_capacity(capacity),
            size_w: Vec::with_capacity(capacity),
            size_h: Vec::with_capacity(capacity),
            rotation: Vec::with_capacity(capacity),
            ang_vel: Vec::with_capacity(capacity),
            uv_rect: Vec::with_capacity(capacity),
            color_rgba: Vec::with_capacity(capacity),
        }
    }

    /// Deterministically populates `count` sprites using `seed` and `config`.
    pub fn spawn_random_batch(
        count: usize,
        seed: u64,
        config: &SimConfig,
    ) -> Result<Self, CoreError> {
        config.validate()?;

        let mut rng = fastrand::Rng::with_seed(seed);
        let mut soa = Self::with_capacity(count);
        let size_span = config.max_size - config.min_size;

        for _ in 0..count {
            let px = (rng.f32() * config.world_width).clamp(0.0, config.world_width);
            let py = (rng.f32() * config.world_height).clamp(0.0, config.world_height);
            let vx = (rng.f32() * 2.0 - 1.0) * config.max_speed;
            let vy = (rng.f32() * 2.0 - 1.0) * config.max_speed;
            let sw = (config.min_size + rng.f32() * size_span)
                .clamp(config.min_size, config.max_size);
            let sh = (config.min_size + rng.f32() * size_span)
                .clamp(config.min_size, config.max_size);
            let rot = rng.f32() * core::f32::consts::TAU;
            let av = (rng.f32() * 2.0 - 1.0) * config.max_angular_speed;

            let hue = rng.f32() * 6.0;
            let sat = 0.75 + rng.f32() * 0.25;
            let val = 0.85 + rng.f32() * 0.15;
            let color = hsv_to_rgba8(hue, sat, val);

            soa.push([px, py], [vx, vy], [sw, sh], rot, av, FULL_UV_RECT, color);
        }

        debug_assert!(soa.debug_assert_soa_invariants());
        Ok(soa)
    }

    /// Appends a single sprite across all parallel SoA columns.
    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        pos: [f32; 2],
        vel: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        ang_vel: f32,
        uv_rect: [u16; 4],
        color_rgba: [u8; 4],
    ) {
        self.pos_x.push(pos[0]);
        self.pos_y.push(pos[1]);
        self.vel_x.push(vel[0]);
        self.vel_y.push(vel[1]);
        self.size_w.push(size[0]);
        self.size_h.push(size[1]);
        self.rotation.push(rotation);
        self.ang_vel.push(ang_vel);
        self.uv_rect.push(uv_rect);
        self.color_rgba.push(color_rgba);
    }

    /// Clears all columns without releasing allocated capacity.
    pub fn clear(&mut self) {
        self.pos_x.clear();
        self.pos_y.clear();
        self.vel_x.clear();
        self.vel_y.clear();
        self.size_w.clear();
        self.size_h.clear();
        self.rotation.clear();
        self.ang_vel.clear();
        self.uv_rect.clear();
        self.color_rgba.clear();
    }

    /// Verifies that every parallel column has the exact same length.
    pub fn debug_assert_soa_invariants(&self) -> bool {
        let len = self.pos_x.len();
        self.pos_y.len() == len
            && self.vel_x.len() == len
            && self.vel_y.len() == len
            && self.size_w.len() == len
            && self.size_h.len() == len
            && self.rotation.len() == len
            && self.ang_vel.len() == len
            && self.uv_rect.len() == len
            && self.color_rgba.len() == len
    }
}

impl BulkSimulation for SpriteSoA {
    fn len(&self) -> usize {
        self.pos_x.len()
    }

    fn step(&mut self, dt: f32, bounds_w: f32, bounds_h: f32) -> Result<(), CoreError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(CoreError::InvalidDeltaTime { dt });
        }
        if !bounds_w.is_finite() || !bounds_h.is_finite() || bounds_w <= 0.0 || bounds_h <= 0.0 {
            return Err(CoreError::InvalidDimensions {
                width: bounds_w,
                height: bounds_h,
            });
        }
        debug_assert!(self.debug_assert_soa_invariants());
        if dt == 0.0 || self.pos_x.is_empty() {
            return Ok(());
        }

        integrate_axis_bounce(&mut self.pos_x, &mut self.vel_x, dt, bounds_w);
        integrate_axis_bounce(&mut self.pos_y, &mut self.vel_y, dt, bounds_h);
        integrate_rotation(&mut self.rotation, &self.ang_vel, dt);

        Ok(())
    }
}

/// Branchless, auto-vectorizable 1D integration with boundary clamping and velocity reflection.
#[inline]
fn integrate_axis_bounce(positions: &mut [f32], velocities: &mut [f32], dt: f32, max_bound: f32) {
    for (p, v) in positions.iter_mut().zip(velocities.iter_mut()) {
        let vel = *v;
        let next_pos = *p + vel * dt;
        let hit_low = next_pos < 0.0;
        let hit_high = next_pos > max_bound;

        let abs_vel = vel.abs();
        let vel_after_low = if hit_low { abs_vel } else { vel };
        let new_vel = if hit_high { -abs_vel } else { vel_after_low };

        let pos_after_low = if hit_low { 0.0 } else { next_pos };
        let new_pos = if hit_high { max_bound } else { pos_after_low };

        *p = new_pos;
        *v = new_vel;
    }
}

/// Contiguous angular integration over independent slices for SIMD vectorization.
#[inline]
fn integrate_rotation(rotations: &mut [f32], angular_velocities: &[f32], dt: f32) {
    for (rot, &ang_vel) in rotations.iter_mut().zip(angular_velocities.iter()) {
        *rot += ang_vel * dt;
    }
}

/// Converts HSV (`hue_0_to_6` in `[0.0, 6.0)`, `sat` and `val` in `[0.0, 1.0]`) into opaque RGBA8.
#[inline]
fn hsv_to_rgba8(hue_0_to_6: f32, sat: f32, val: f32) -> [u8; 4] {
    let sector = (hue_0_to_6 as u32) % 6;
    let frac = hue_0_to_6 - hue_0_to_6.floor();
    let p = val * (1.0 - sat);
    let q = val * (1.0 - sat * frac);
    let t = val * (1.0 - sat * (1.0 - frac));

    let (r, g, b) = match sector {
        0 => (val, t, p),
        1 => (q, val, p),
        2 => (p, val, t),
        3 => (p, q, val),
        4 => (t, p, val),
        _ => (val, p, q),
    };

    [
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
        255,
    ]
}

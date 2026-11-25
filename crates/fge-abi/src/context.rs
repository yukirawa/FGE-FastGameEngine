use fastrand::Rng;
use fge_core::{
    camera::Camera2D,
    error::CoreError,
    handle::EntityId,
    input_core::CoreInputState,
    spatial::{CollisionPair, SpatialHashGrid},
    world::EntityWorld,
};

/// Complete host gameplay state operated on by C callbacks (`fge_init`, `fge_update`, `fge_shutdown`)
/// as well as headless tests and the Phase 4 AI Bridge.
pub struct EngineContext {
    pub world: EntityWorld,
    pub camera: Camera2D,
    pub spatial: SpatialHashGrid,
    pub input: CoreInputState,
    pub rng: Rng,
    pub dt: f32,
    pub frame_count: u64,
    pub quit_requested: bool,
    pub last_collisions: Vec<CollisionPair>,
    pub last_query_entities: Vec<EntityId>,
}

impl EngineContext {
    /// Creates a new `EngineContext` with viewport `(width, height)`, deterministic `seed`,
    /// and default spatial grid (`cell_size = 64.0`, `buckets = 4096`).
    ///
    /// // TODO(user-decision): SpatialHashGrid default `cell_size = 64.0` and `buckets = 4096`
    /// // are tuned for general 2D STG/RPG scenes; can be exposed in `game.toml` if per-game tuning is desired.
    pub fn new(_width: f32, _height: f32, _seed: u64) -> Result<Self, CoreError> {
        todo!("EngineContext::new")
    }

    /// Advances frame counters, runs `f` while `self` is bound as the active C-ABI thread-local context,
    /// and then steps kinematic physics (`world.step_kinematics(dt)`).
    pub fn step_with_callback<F, R>(&mut self, _dt: f32, _callback: F) -> Result<R, CoreError>
    where
        F: FnOnce() -> R,
    {
        todo!("EngineContext::step_with_callback")
    }
}

/// Executes `f` with `ctx` bound as the active thread-local `EngineContext` via an RAII drop guard,
/// guaranteeing cleanup even if `f` unwinds.
pub fn with_active_context<F, R>(_ctx: &mut EngineContext, _f: F) -> R
where
    F: FnOnce() -> R,
{
    todo!("with_active_context")
}

/// Borrows the currently active thread-local `EngineContext` mutably inside an `extern "C"` host call.
/// Returns `None` if called outside `with_active_context`.
pub(crate) fn try_with_context_mut<F, R>(_f: F) -> Option<R>
where
    F: FnOnce(&mut EngineContext) -> R,
{
    todo!("try_with_context_mut")
}

use std::cell::Cell;
use std::ptr::NonNull;

use fastrand::Rng;
use fge_core::{
    camera::Camera2D,
    error::CoreError,
    handle::EntityId,
    input_core::CoreInputState,
    spatial::{CollisionPair, SpatialHashGrid},
    world::EntityWorld,
};

const DEFAULT_SPATIAL_CELL_SIZE: f32 = 64.0;
const DEFAULT_SPATIAL_BUCKETS: usize = 4096;

thread_local! {
    /// Thread-local pointer to the currently active `EngineContext` for synchronous C-ABI calls.
    static ACTIVE_CTX: Cell<Option<NonNull<EngineContext>>> = const { Cell::new(None) };
    /// Re-entrancy borrow guard ensuring at most one `&mut EngineContext` exists at a time.
    static CTX_BORROWED: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard restoring the previous thread-local `EngineContext` binding on scope exit or unwind.
struct ContextGuard {
    prev_ctx: Option<NonNull<EngineContext>>,
    prev_borrowed: bool,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        CTX_BORROWED.with(|borrowed| borrowed.set(self.prev_borrowed));
        ACTIVE_CTX.with(|cell| cell.set(self.prev_ctx));
    }
}

/// RAII guard releasing the thread-local re-entrancy borrow flag on scope exit or unwind.
struct BorrowGuard;

impl Drop for BorrowGuard {
    fn drop(&mut self) {
        CTX_BORROWED.with(|borrowed| borrowed.set(false));
    }
}

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
    pub fn new(width: f32, height: f32, seed: u64) -> Result<Self, CoreError> {
        let camera = Camera2D::from_top_left_viewport(width, height)?;
        let spatial = SpatialHashGrid::new(DEFAULT_SPATIAL_CELL_SIZE, DEFAULT_SPATIAL_BUCKETS)?;
        Ok(Self {
            world: EntityWorld::new(),
            camera,
            spatial,
            input: CoreInputState::new(),
            rng: Rng::with_seed(seed),
            dt: 1.0 / 60.0,
            frame_count: 0,
            quit_requested: false,
            last_collisions: Vec::new(),
            last_query_entities: Vec::new(),
        })
    }

    /// Advances frame counters, runs `callback` while `self` is bound as the active C-ABI
    /// thread-local context, steps kinematic physics (`world.step_kinematics(dt)`), and clears
    /// per-frame input edge flags.
    pub fn step_with_callback<F, R>(&mut self, dt: f32, callback: F) -> Result<R, CoreError>
    where
        F: FnOnce() -> R,
    {
        if !dt.is_finite() || dt < 0.0 {
            return Err(CoreError::InvalidDeltaTime { dt });
        }

        self.dt = dt;
        let out = with_active_context(self, callback);
        self.world.step_kinematics(dt)?;
        self.frame_count = self.frame_count.saturating_add(1);
        self.input.begin_frame();
        Ok(out)
    }
}

/// Executes `f` with `ctx` bound as the active thread-local `EngineContext` via an RAII drop guard,
/// guaranteeing cleanup even if `f` unwinds.
pub fn with_active_context<F, R>(ctx: &mut EngineContext, f: F) -> R
where
    F: FnOnce() -> R,
{
    let ptr = NonNull::from(ctx);
    let prev_ctx = ACTIVE_CTX.with(|cell| cell.replace(Some(ptr)));
    let prev_borrowed = CTX_BORROWED.with(|borrowed| borrowed.replace(false));
    let _guard = ContextGuard {
        prev_ctx,
        prev_borrowed,
    };
    f()
}

/// Borrows the currently active thread-local `EngineContext` mutably inside an `extern "C"` host call.
/// Returns `None` if called outside `with_active_context` or re-entrantly while already borrowed.
pub(crate) fn try_with_context_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut EngineContext) -> R,
{
    let mut ptr = ACTIVE_CTX.with(|cell| cell.get())?;
    let acquired = CTX_BORROWED.with(|borrowed| {
        if borrowed.get() {
            false
        } else {
            borrowed.set(true);
            true
        }
    });
    if !acquired {
        return None;
    }

    let _borrow_guard = BorrowGuard;

    // SAFETY:
    // 1. Validity & Lifetime: `ptr` was created via `NonNull::from(ctx)` from a live,
    //    properly aligned, initialized `&mut EngineContext` in `with_active_context` on
    //    this exact thread. `ContextGuard::drop` restores `ACTIVE_CTX` before
    //    `with_active_context` returns (including on stack unwind), so `ptr` cannot dangle
    //    while `try_with_context_mut` executes.
    // 2. Exclusivity / No Aliasing: `with_active_context` owns an exclusive `&mut EngineContext`
    //    and never accesses `ctx` again until `ContextGuard` drops. Because `ACTIVE_CTX` and
    //    `CTX_BORROWED` are thread-local and `CTX_BORROWED` transitions `false -> true` above
    //    (and is reset only after `ctx_ref` ends via `BorrowGuard::drop`), no aliasing
    //    `&EngineContext` or `&mut EngineContext` reference can exist concurrently.
    let ctx_ref = unsafe { ptr.as_mut() };
    Some(f(ctx_ref))
}

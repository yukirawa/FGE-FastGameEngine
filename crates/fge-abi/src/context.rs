use std::cell::Cell;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use fastrand::Rng;
use fge_audio::engine::AudioEngine;
use fge_core::{
    camera::Camera2D,
    emitter::EmitterWorld,
    error::CoreError,
    handle::{EntityId, GenerationalArena},
    input_core::CoreInputState,
    spatial::{CollisionPair, SpatialHashGrid},
    ui::UiCommandQueue,
    world::EntityWorld,
};

use crate::types::{FgeAiEvent, FgeTexture};

const DEFAULT_SPATIAL_CELL_SIZE: f32 = 64.0;
const DEFAULT_SPATIAL_BUCKETS: usize = 4096;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

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
    pub emitters: EmitterWorld,
    pub ui: UiCommandQueue,
    pub audio: AudioEngine,
    pub project_dir: PathBuf,
    pub texture_arena: GenerationalArena,
    pub texture_uvs: Vec<Option<[u16; 4]>>,
    pub pending_textures: Vec<(FgeTexture, PathBuf)>,
    pub entity_textures: Vec<(EntityId, FgeTexture)>,
    pub camera: Camera2D,
    pub spatial: SpatialHashGrid,
    pub input: CoreInputState,
    pub rng: Rng,
    pub dt: f32,
    pub frame_count: u64,
    pub quit_requested: bool,
    pub last_collisions: Vec<CollisionPair>,
    pub last_query_entities: Vec<EntityId>,
    pub params: Vec<(u32, f32)>,
    pub pending_ai_events: Vec<FgeAiEvent>,
    pub active_ai_events: Vec<FgeAiEvent>,
    pub metrics: Vec<(u32, f32)>,
}

impl EngineContext {
    /// Creates a new `EngineContext` with viewport `(width, height)`, deterministic `seed`,
    /// and default spatial grid (`cell_size = 64.0`, `buckets = 4096`).
    pub fn new(width: f32, height: f32, seed: u64) -> Result<Self, CoreError> {
        Self::with_spatial_config(
            width,
            height,
            seed,
            DEFAULT_SPATIAL_CELL_SIZE,
            DEFAULT_SPATIAL_BUCKETS,
        )
    }

    /// Creates a new `EngineContext` with viewport `(width, height)`, deterministic `seed`,
    /// and explicit spatial hash grid configuration (`cell_size`, `buckets`).
    pub fn with_spatial_config(
        width: f32,
        height: f32,
        seed: u64,
        cell_size: f32,
        buckets: usize,
    ) -> Result<Self, CoreError> {
        let camera = Camera2D::from_top_left_viewport(width, height)?;
        let spatial = SpatialHashGrid::new(cell_size, buckets)?;
        Ok(Self {
            world: EntityWorld::new(),
            emitters: EmitterWorld::new(),
            ui: UiCommandQueue::new(),
            audio: AudioEngine::new_headless(),
            project_dir: PathBuf::from("."),
            texture_arena: GenerationalArena::default(),
            texture_uvs: Vec::new(),
            pending_textures: Vec::new(),
            entity_textures: Vec::new(),
            camera,
            spatial,
            input: CoreInputState::new(),
            rng: Rng::with_seed(seed),
            dt: 1.0 / 60.0,
            frame_count: 0,
            quit_requested: false,
            last_collisions: Vec::new(),
            last_query_entities: Vec::new(),
            params: Vec::new(),
            pending_ai_events: Vec::new(),
            active_ai_events: Vec::new(),
            metrics: Vec::new(),
        })
    }

    /// Sets the project root directory used to resolve relative asset paths (WAVs, PNGs).
    pub fn set_project_dir(&mut self, dir: impl Into<PathBuf>) {
        self.project_dir = dir.into();
    }

    /// Resolves `relative_path` against `self.project_dir` (returning it directly if already absolute).
    pub fn resolve_asset_path(&self, relative_path: &str) -> PathBuf {
        let p = Path::new(relative_path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.project_dir.join(p)
        }
    }

    /// Upgrades the headless audio engine to a live `cpal` output stream (with automatic
    /// headless fallback if no audio hardware is present) and applies `master_volume`.
    pub fn enable_live_audio(&mut self, master_volume: f32) {
        self.audio = AudioEngine::new_live();
        self.audio.set_master_volume(master_volume);
    }

    /// Validates a PNG file on disk (existence + 8-byte PNG header) and queues it for GPU atlas upload,
    /// returning a deterministic generational [`FgeTexture`] handle even in headless mode.
    pub fn queue_texture_load_png(&mut self, relative_path: &str) -> FgeTexture {
        let trimmed = relative_path.trim();
        if trimmed.is_empty() {
            return EntityId::INVALID;
        }
        let full_path = self.resolve_asset_path(trimmed);
        let Ok(mut file) = std::fs::File::open(&full_path) else {
            return EntityId::INVALID;
        };
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() || header != PNG_SIGNATURE {
            return EntityId::INVALID;
        }

        let tex_id = self.texture_arena.allocate();
        self.texture_uvs.push(None);
        self.pending_textures.push((tex_id, full_path));
        tex_id
    }

    /// Binds `texture` to `entity`. If the texture UV rect has already been resolved by the GPU
    /// runner, applies it immediately; otherwise queues the binding for the next runner sync.
    pub fn bind_entity_texture(&mut self, entity: EntityId, texture: FgeTexture) -> bool {
        if !self.world.is_alive(entity) {
            return false;
        }
        let Some(dense_idx) = self.texture_arena.resolve(texture) else {
            return false;
        };
        if let Some(Some(uv)) = self.texture_uvs.get(dense_idx).copied() {
            return self.world.set_uv_rect(entity, uv);
        }
        self.entity_textures.push((entity, texture));
        true
    }

    /// Records the resolved atlas UV rectangle for an ABI-side `texture` handle.
    pub fn record_resolved_texture_uv(&mut self, texture: FgeTexture, uv: [u16; 4]) {
        if let Some(dense_idx) = self.texture_arena.resolve(texture) {
            if let Some(slot) = self.texture_uvs.get_mut(dense_idx) {
                *slot = Some(uv);
            }
        }
    }

    /// Drains all queued `(FgeTexture, PathBuf)` loads for the GPU runner to upload into `DynamicAtlas`.
    pub fn take_pending_textures(&mut self) -> Vec<(FgeTexture, PathBuf)> {
        std::mem::take(&mut self.pending_textures)
    }

    /// Applies any queued `(EntityId, FgeTexture)` bindings whose textures now have resolved UV rects.
    pub fn apply_pending_entity_textures(&mut self) {
        if self.entity_textures.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.entity_textures);
        for (entity, texture) in pending {
            if !self.world.is_alive(entity) {
                continue;
            }
            if let Some(dense_idx) = self.texture_arena.resolve(texture) {
                if let Some(Some(uv)) = self.texture_uvs.get(dense_idx).copied() {
                    let _ = self.world.set_uv_rect(entity, uv);
                } else {
                    self.entity_textures.push((entity, texture));
                }
            }
        }
    }

    /// Inserts or updates a numeric parameter by `id`.
    pub fn set_param(&mut self, id: u32, value: f32) {
        if let Some((_, existing)) = self.params.iter_mut().find(|(k, _)| *k == id) {
            *existing = value;
        } else {
            self.params.push((id, value));
        }
    }

    /// Returns the numeric parameter for `id`, or `default_val` if unset.
    pub fn get_param(&self, id: u32, default_val: f32) -> f32 {
        self.params
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, v)| *v)
            .unwrap_or(default_val)
    }

    /// Appends an incoming AI event into the pending queue for the next poll.
    pub fn push_ai_event(&mut self, event: FgeAiEvent) {
        self.pending_ai_events.push(event);
    }

    /// Moves all pending AI events into the active per-frame slice and returns the event count.
    pub fn poll_ai_events(&mut self) -> u32 {
        self.active_ai_events.clear();
        self.active_ai_events.append(&mut self.pending_ai_events);
        self.active_ai_events.len().min(u32::MAX as usize) as u32
    }

    /// Records or updates a numeric telemetry metric by `id`.
    pub fn record_metric(&mut self, id: u32, value: f32) {
        if let Some((_, existing)) = self.metrics.iter_mut().find(|(k, _)| *k == id) {
            *existing = value;
        } else {
            self.metrics.push((id, value));
        }
    }

    /// Advances frame counters, clears the per-frame UI command queue, runs `callback` while
    /// `self` is bound as the active C-ABI thread-local context, steps bulk emitters and
    /// kinematic physics (`world.step_kinematics(dt)`), and clears per-frame input edge flags.
    pub fn step_with_callback<F, R>(&mut self, dt: f32, callback: F) -> Result<R, CoreError>
    where
        F: FnOnce() -> R,
    {
        if !dt.is_finite() || dt < 0.0 {
            return Err(CoreError::InvalidDeltaTime { dt });
        }

        self.dt = dt;
        self.ui.clear();
        let out = with_active_context(self, callback);
        self.emitters.step_emitters(dt, &mut self.world);
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

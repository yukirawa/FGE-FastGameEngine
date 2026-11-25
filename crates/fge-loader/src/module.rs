use std::path::{Path, PathBuf};

use crate::error::LoaderError;

/// Dynamically loaded native C11 game shared library (`.so`) and resolved entrypoints.
#[allow(dead_code)]
pub struct LoadedGameModule {
    pub(crate) library: Option<libloading::Library>,
    pub(crate) so_path: PathBuf,
    pub(crate) generation: u64,
    pub(crate) cleanup_on_drop: bool,
    pub(crate) init_fn: unsafe extern "C" fn(),
    pub(crate) update_fn: unsafe extern "C" fn(f32),
    pub(crate) shutdown_fn: Option<unsafe extern "C" fn()>,
    pub(crate) on_reload_fn: Option<unsafe extern "C" fn()>,
}

impl LoadedGameModule {
    /// Ensures C-ABI symbols are linked, loads `so_path` via `dlopen`, and resolves game entrypoints.
    #[allow(unused_variables)]
    pub fn load(so_path: PathBuf, generation: u64, cleanup_on_drop: bool) -> Result<Self, LoaderError> {
        todo!()
    }

    /// Returns the monotonic generation counter of this loaded module.
    pub fn generation(&self) -> u64 {
        todo!()
    }

    /// Returns the filesystem path to the loaded `.so` artifact.
    pub fn so_path(&self) -> &Path {
        todo!()
    }

    /// Returns `true` if the game module exported an optional `fge_on_reload` hook.
    pub fn has_on_reload(&self) -> bool {
        todo!()
    }

    /// Invokes `fge_init()` with `ctx` bound as the active thread-local C-ABI context.
    #[allow(unused_variables)]
    pub fn call_init(&self, ctx: &mut fge_abi::context::EngineContext) {
        todo!()
    }

    /// Invokes optional `fge_on_reload()` with `ctx` bound as the active thread-local C-ABI context.
    #[allow(unused_variables)]
    pub fn call_on_reload(&self, ctx: &mut fge_abi::context::EngineContext) {
        todo!()
    }

    /// Executes one frame by invoking `fge_update(dt)` inside `ctx.step_with_callback`.
    #[allow(unused_variables)]
    pub fn step_frame(
        &self,
        ctx: &mut fge_abi::context::EngineContext,
        dt: f32,
    ) -> Result<(), LoaderError> {
        todo!()
    }

    /// Invokes optional `fge_shutdown()` with `ctx` bound as the active thread-local C-ABI context.
    #[allow(unused_variables)]
    pub fn call_shutdown(&self, ctx: &mut fge_abi::context::EngineContext) {
        todo!()
    }
}

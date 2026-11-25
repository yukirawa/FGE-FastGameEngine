use std::path::PathBuf;
use std::time::SystemTime;

use crate::compiler::CBuildPlan;
use crate::error::LoaderError;
use crate::module::LoadedGameModule;

/// Single-threaded mtime/size-polling C11 hot-reloader that preserves the last working module on error.
#[allow(dead_code)]
pub struct HotReloader {
    plan: CBuildPlan,
    module: LoadedGameModule,
    snapshot: Vec<(PathBuf, Option<SystemTime>, u64)>,
    last_error: Option<String>,
}

impl HotReloader {
    /// Compiles generation `1`, loads the `.so`, calls `fge_init`, and snapshots watched sources.
    #[allow(unused_variables)]
    pub fn initial_build_and_load(
        plan: CBuildPlan,
        ctx: &mut fge_abi::context::EngineContext,
    ) -> Result<Self, LoaderError> {
        todo!()
    }

    /// Returns the currently active module generation (`>= 1`).
    pub fn generation(&self) -> u64 {
        todo!()
    }

    /// Returns the most recent compile/load error message if the last reload attempt failed.
    pub fn last_compile_error(&self) -> Option<&str> {
        todo!()
    }

    /// Polls watched source files for changes and hot-reloads on modification.
    #[allow(unused_variables)]
    pub fn poll_and_reload(
        &mut self,
        ctx: &mut fge_abi::context::EngineContext,
    ) -> Result<bool, LoaderError> {
        todo!()
    }

    /// Steps the active module for one frame with delta time `dt`.
    #[allow(unused_variables)]
    pub fn step_frame(
        &self,
        ctx: &mut fge_abi::context::EngineContext,
        dt: f32,
    ) -> Result<(), LoaderError> {
        todo!()
    }

    /// Shuts down the currently active module by calling `fge_shutdown` if present.
    #[allow(unused_variables)]
    pub fn shutdown(&self, ctx: &mut fge_abi::context::EngineContext) {
        todo!()
    }
}

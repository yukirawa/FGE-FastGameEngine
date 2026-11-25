use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::compiler::{compile_shared_library, CBuildPlan};
use crate::error::LoaderError;
use crate::module::LoadedGameModule;

const MAX_SCAN_DEPTH: usize = 32;

/// Single-threaded mtime/size-polling C11 hot-reloader that preserves the last working module on error.
pub struct HotReloader {
    plan: CBuildPlan,
    module: LoadedGameModule,
    snapshot: Vec<(PathBuf, Option<SystemTime>, u64)>,
    last_error: Option<String>,
}

impl HotReloader {
    /// Compiles generation `1`, loads the `.so`, calls `fge_init`, and snapshots watched sources.
    pub fn initial_build_and_load(
        plan: CBuildPlan,
        ctx: &mut fge_abi::context::EngineContext,
    ) -> Result<Self, LoaderError> {
        let so_path = compile_shared_library(&plan, 1)?;
        let module = LoadedGameModule::load(so_path, 1, true)?;
        module.call_init(ctx);
        let snapshot = snapshot_watched_files(&plan)?;

        Ok(Self {
            plan,
            module,
            snapshot,
            last_error: None,
        })
    }

    /// Returns the currently active module generation (`>= 1`).
    pub fn generation(&self) -> u64 {
        self.module.generation()
    }

    /// Returns the most recent compile/load error message if the last reload attempt failed.
    pub fn last_compile_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Polls watched source files for changes and hot-reloads on modification.
    pub fn poll_and_reload(
        &mut self,
        ctx: &mut fge_abi::context::EngineContext,
    ) -> Result<bool, LoaderError> {
        let new_snapshot = snapshot_watched_files(&self.plan)?;
        if new_snapshot == self.snapshot {
            return Ok(false);
        }

        // Update snapshot before attempting compile so a broken save is not recompiled every frame.
        self.snapshot = new_snapshot;

        let next_gen = self.module.generation().saturating_add(1);
        let reload_result = compile_shared_library(&self.plan, next_gen)
            .and_then(|so_path| LoadedGameModule::load(so_path, next_gen, true));

        match reload_result {
            Ok(new_module) => {
                self.last_error = None;
                // Replacing `self.module` drops the previous `.so` handle and cleans up its file.
                self.module = new_module;
                if self.module.has_on_reload() {
                    self.module.call_on_reload(ctx);
                }
                // TODO(user-decision): When a reloaded C module does not export `fge_on_reload`,
                // decide whether hot-reload should keep the world untouched (current default)
                // or optionally re-run `fge_init`.
                Ok(true)
            }
            Err(err) => {
                log::warn!(
                    "hot-reload failed for generation {} (keeping generation {}): {}",
                    next_gen,
                    self.module.generation(),
                    err
                );
                self.last_error = Some(err.to_string());
                Err(err)
            }
        }
    }

    /// Steps the active module for one frame with delta time `dt`.
    pub fn step_frame(
        &self,
        ctx: &mut fge_abi::context::EngineContext,
        dt: f32,
    ) -> Result<(), LoaderError> {
        self.module.step_frame(ctx, dt)
    }

    /// Shuts down the currently active module by calling `fge_shutdown` if present.
    pub fn shutdown(&self, ctx: &mut fge_abi::context::EngineContext) {
        self.module.call_shutdown(ctx);
    }
}

fn snapshot_watched_files(
    plan: &CBuildPlan,
) -> Result<Vec<(PathBuf, Option<SystemTime>, u64)>, LoaderError> {
    let mut entries: Vec<(PathBuf, Option<SystemTime>, u64)> = Vec::new();

    let manifest_path = plan.project_dir.join("game.toml");
    if let Ok(meta) = std::fs::metadata(&manifest_path) {
        if meta.is_file() {
            entries.push((manifest_path, meta.modified().ok(), meta.len()));
        }
    }

    collect_c_sources_recursive(&plan.project_dir, &plan.out_dir, 0, &mut entries)?;

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    Ok(entries)
}

fn collect_c_sources_recursive(
    dir: &Path,
    out_dir: &Path,
    depth: usize,
    entries: &mut Vec<(PathBuf, Option<SystemTime>, u64)>,
) -> Result<(), LoaderError> {
    if depth >= MAX_SCAN_DEPTH {
        return Ok(());
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(LoaderError::Io(err)),
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(LoaderError::Io(err)),
        };

        let path = entry.path();
        if path == out_dir {
            continue;
        }
        let file_name = entry.file_name();
        if file_name == ".git" || file_name == "target" {
            continue;
        }

        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(LoaderError::Io(err)),
        };

        if meta.is_dir() {
            collect_c_sources_recursive(&path, out_dir, depth + 1, entries)?;
        } else if meta.is_file() && is_watched_source_file(&path) {
            entries.push((path, meta.modified().ok(), meta.len()));
        }
    }

    Ok(())
}

fn is_watched_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "h")
    )
}

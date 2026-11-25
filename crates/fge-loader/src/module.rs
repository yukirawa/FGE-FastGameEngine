use std::path::{Path, PathBuf};

use crate::error::LoaderError;

/// Dynamically loaded native C11 game shared library (`.so`) and resolved entrypoints.
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
    pub fn load(
        so_path: PathBuf,
        generation: u64,
        cleanup_on_drop: bool,
    ) -> Result<Self, LoaderError> {
        let _ = fge_abi::ensure_abi_symbols_linked();

        // Keep `library` scoped inside the closure so on any symbol-resolution error
        // `dlclose` runs strictly before `remove_so_file` unlinks `so_path`.
        let loaded = (|| {
            // SAFETY:
            // 1. `libloading::Library::new` invokes platform dynamic loading (`dlopen`), which may
            //    execute shared-object initialization routines. Callers only pass game `.so` artifacts
            //    compiled against `include/fge.h` for the host target.
            // 2. `fge_abi::ensure_abi_symbols_linked()` was invoked immediately above so all 42
            //    `fge_*` C-ABI host symbols are retained in the host binary's dynamic symbol table
            //    (`-Wl,--export-dynamic`) for symbol resolution by the loaded shared library.
            let library = unsafe { libloading::Library::new(&so_path) }?;

            // SAFETY:
            // 1. `fge_init` is declared in `include/fge.h` as `void fge_init(void)`, matching
            //    `unsafe extern "C" fn()`.
            // 2. The copied function pointer is only invoked while `self.library` holds `Some(library)`
            //    (it is taken and dropped exclusively in `Drop`), ensuring the mapped code remains
            //    valid for the entire lifetime of `LoadedGameModule`.
            let init_fn: unsafe extern "C" fn() = unsafe {
                *library
                    .get::<unsafe extern "C" fn()>(b"fge_init\0")
                    .map_err(|_| LoaderError::MissingSymbol { symbol: "fge_init" })?
            };

            // SAFETY:
            // 1. `fge_update` is declared in `include/fge.h` as `void fge_update(float dt)`, matching
            //    `unsafe extern "C" fn(f32)`.
            // 2. The copied function pointer is only invoked while `self.library` holds `Some(library)`
            //    (dropped exclusively in `Drop`), keeping the mapped executable code valid.
            let update_fn: unsafe extern "C" fn(f32) = unsafe {
                *library
                    .get::<unsafe extern "C" fn(f32)>(b"fge_update\0")
                    .map_err(|_| LoaderError::MissingSymbol {
                        symbol: "fge_update",
                    })?
            };

            // SAFETY:
            // 1. Optional `fge_shutdown` is declared in `include/fge.h` as `void fge_shutdown(void)`,
            //    matching `unsafe extern "C" fn()`.
            // 2. If present, the copied function pointer is only invoked while `self.library` holds
            //    `Some(library)` (dropped exclusively in `Drop`), keeping the code mapping valid.
            let shutdown_fn: Option<unsafe extern "C" fn()> =
                unsafe { library.get::<unsafe extern "C" fn()>(b"fge_shutdown\0") }
                    .ok()
                    .map(|sym| *sym);

            // SAFETY:
            // 1. Optional `fge_on_reload` follows the `void fge_on_reload(void)` C-ABI convention,
            //    matching `unsafe extern "C" fn()`.
            // 2. If present, the copied function pointer is only invoked while `self.library` holds
            //    `Some(library)` (dropped exclusively in `Drop`), keeping the code mapping valid.
            let on_reload_fn: Option<unsafe extern "C" fn()> =
                unsafe { library.get::<unsafe extern "C" fn()>(b"fge_on_reload\0") }
                    .ok()
                    .map(|sym| *sym);

            Ok((library, init_fn, update_fn, shutdown_fn, on_reload_fn))
        })();

        match loaded {
            Ok((library, init_fn, update_fn, shutdown_fn, on_reload_fn)) => Ok(Self {
                library: Some(library),
                so_path,
                generation,
                cleanup_on_drop,
                init_fn,
                update_fn,
                shutdown_fn,
                on_reload_fn,
            }),
            Err(err) => {
                if cleanup_on_drop {
                    remove_so_file(&so_path);
                }
                Err(err)
            }
        }
    }

    /// Returns the monotonic generation counter of this loaded module.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the filesystem path to the loaded `.so` artifact.
    pub fn so_path(&self) -> &Path {
        &self.so_path
    }

    /// Returns `true` if the game module exported an optional `fge_on_reload` hook.
    pub fn has_on_reload(&self) -> bool {
        self.on_reload_fn.is_some()
    }

    /// Invokes `fge_init()` with `ctx` bound as the active thread-local C-ABI context.
    pub fn call_init(&self, ctx: &mut fge_abi::context::EngineContext) {
        fge_abi::context::with_active_context(ctx, || {
            // SAFETY:
            // 1. `self.init_fn` was resolved from `self.library` with signature `void fge_init(void)`
            //    matching `include/fge.h`, and `self.library` remains `Some(...)` for the entire
            //    lifetime of `&self`, so the function pointer targets valid mapped code.
            // 2. `with_active_context` binds `ctx` on the current thread via an RAII drop guard
            //    before entering C code, ensuring all host `fge_*` calls access `ctx` safely.
            unsafe { (self.init_fn)() };
        });
    }

    /// Invokes optional `fge_on_reload()` with `ctx` bound as the active thread-local C-ABI context.
    pub fn call_on_reload(&self, ctx: &mut fge_abi::context::EngineContext) {
        if let Some(on_reload_fn) = self.on_reload_fn {
            fge_abi::context::with_active_context(ctx, || {
                // SAFETY:
                // 1. `on_reload_fn` was resolved from `self.library` as `void fge_on_reload(void)`
                //    and `self.library` remains `Some(...)` for the lifetime of `&self`, so the
                //    function pointer targets valid mapped executable code.
                // 2. `with_active_context` binds `ctx` on the current thread via an RAII drop guard
                //    for the duration of the callback, ensuring safe host `fge_*` calls.
                unsafe { on_reload_fn() };
            });
        }
    }

    /// Executes one frame by invoking `fge_update(dt)` inside `ctx.step_with_callback`.
    pub fn step_frame(
        &self,
        ctx: &mut fge_abi::context::EngineContext,
        dt: f32,
    ) -> Result<(), LoaderError> {
        ctx.step_with_callback(dt, || {
            // SAFETY:
            // 1. `self.update_fn` was resolved from `self.library` with signature
            //    `void fge_update(float dt)` matching `include/fge.h`, and `self.library` remains
            //    `Some(...)` for the lifetime of `&self`, keeping the code mapping valid.
            // 2. `EngineContext::step_with_callback` validates `dt` and binds `ctx` via
            //    `with_active_context` on the current thread before invoking this closure, so
            //    C-ABI host calls execute against a valid, exclusively bound `EngineContext`.
            unsafe { (self.update_fn)(dt) };
        })?;
        Ok(())
    }

    /// Invokes optional `fge_shutdown()` with `ctx` bound as the active thread-local C-ABI context.
    pub fn call_shutdown(&self, ctx: &mut fge_abi::context::EngineContext) {
        if let Some(shutdown_fn) = self.shutdown_fn {
            fge_abi::context::with_active_context(ctx, || {
                // SAFETY:
                // 1. `shutdown_fn` was resolved from `self.library` with signature
                //    `void fge_shutdown(void)` matching `include/fge.h`, and `self.library` remains
                //    `Some(...)` for the lifetime of `&self`, keeping the code mapping valid.
                // 2. `with_active_context` binds `ctx` on the current thread via an RAII drop guard
                //    for the duration of the callback, ensuring safe host `fge_*` calls.
                unsafe { shutdown_fn() };
            });
        }
    }
}

impl Drop for LoadedGameModule {
    fn drop(&mut self) {
        // Unload (`dlclose`) the shared library BEFORE removing the `.so` file on disk.
        drop(self.library.take());

        if self.cleanup_on_drop {
            remove_so_file(&self.so_path);
        }
    }
}

fn remove_so_file(path: &Path) {
    if let Err(err) = std::fs::remove_file(path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            log::warn!(
                "failed to remove temporary game module '{}': {err}",
                path.display()
            );
        }
    }
}

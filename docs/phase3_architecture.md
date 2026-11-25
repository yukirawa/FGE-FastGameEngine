# FGE Phase 3 (Step 3) Architecture Specification: C11 Loader, `dlopen`, & Live Hot-Reload

## 1. Scope & Design Principles
Phase 3 (`PROJECT.md` Step 3 / Phase A) connects the Rust engine core and C-ABI (`fge-abi`) to real multi-file user C11 game projects (`game.toml` + `src/*.c`, `src/*.h`) via `crates/fge-loader`:

1. **Multi-File C11 Native Compiler Driver (`fge_loader::compiler`)**:
   - Detects an available C11 compiler via `CompilerKind::detect()` (`FGE_CC` env override first, then probing `tcc`, `clang`, `gcc`).
   - **Standalone Single-Binary Header Staging**: Embeds `include/fge.h` at compile time (`EMBEDDED_FGE_H = include_str!("../../../include/fge.h")`) and stages it to `<out_dir>/include/fge.h` via `stage_embedded_header(out_dir)` (only writing if missing or content differs), ensuring `fge` works as a standalone single binary without requiring the engine repository on disk.
   - Resolves `entry` and `sources` from `CBuildPlan` (`resolve_source_files`), deduplicating paths while preserving order and failing fast with `LoaderError::SourceNotFound` if any unit is missing.
   - Compiles all C11 translation units into a versioned shared library (`libfge_game_gen<generation>_<pid>.so`) using `-shared -fPIC -std=c11` (`-O2` when `optimize == true`, else `-O0`), `-I <staged_include_dir>`, `-I <project_dir>/src`, `-I <project_dir>`, and `-lm`.

2. **Dynamic `.so` Loader & C-ABI Symbol Retention (`fge_loader::module` & `fge_abi`)**:
   - Host binaries (`fge` and test harnesses) link `fge-abi` with `-Wl,--export-dynamic` (`.cargo/config.toml` on Linux x86_64) and invoke `fge_abi::ensure_abi_symbols_linked()` at the start of `LoadedGameModule::load`, passing all 42 `extern "C"` `fge_*` function addresses through `std::hint::black_box` so the linker never dead-strips C-ABI entrypoints.
   - `LoadedGameModule` loads the `.so` via `libloading::Library::new`, resolves required `fge_init` and `fge_update` symbols (returning `LoaderError::MissingSymbol` if absent) and optional `fge_shutdown` and `fge_on_reload` symbols.
   - Lifecycle calls (`call_init`, `call_on_reload`, `call_shutdown`) bind `EngineContext` via `fge_abi::context::with_active_context`, while `step_frame` delegates to `EngineContext::step_with_callback(dt, ...)`.
   - `Drop` explicitly drops `Option<libloading::Library>` (`dlclose`) **before** unlinking the temporary `.so` file when `cleanup_on_drop` is enabled.

3. **Polling `HotReloader` (`fge_loader::hot_reload`)**:
   - Pure single-threaded frame-loop polling (zero background watcher threads, zero `Arc<Mutex<T>>`).
   - Tracks `game.toml` and `.c` / `.h` files under `project_dir` and `project_dir/src` by `(PathBuf, Option<SystemTime>, u64)` (`mtime` and byte length).
   - When source files change, updates the snapshot immediately (preventing repeated recompilation of broken edits on every frame) and compiles + loads generation `N + 1`.
   - **Fault-Tolerant Retention**: If compilation or `dlopen` fails, `HotReloader` records the diagnostic in `last_compile_error()`, logs a warning, retains the previous working `LoadedGameModule` untouched so the game keeps running, and returns `Err(LoaderError)`.
   - On reload success, replaces the module (unloading the old `.so`) and invokes `fge_on_reload` if exported while preserving live `EngineContext` state (`EntityWorld`, `Camera2D`, frame counters).

## 2. Updated Crate Dependency Graph
```text
fge (bin + lib)
 ├── fge-loader (lib)
 │    ├── fge-abi (lib, C-ABI symbols + ensure_abi_symbols_linked)
 │    │    └── fge-core (lib)
 │    └── fge-core (lib)
 ├── fge-abi (lib)
 │    └── fge-core (lib)
 ├── fge-render (lib)
 │    └── fge-core (lib)
 └── fge-core (lib)
```

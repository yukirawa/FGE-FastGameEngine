# FGE Phase 4 (Step 4) Architecture Specification: AI Bridge & Headless Test Runner

## 1. Scope & Design Principles
Phase 4 (`PROJECT.md` Section 3.2 & Section 4) establishes a unified observation and intervention architecture for both **AI-driven autonomous development loops** (`fge test`) and **live runtime AI control** (`fge run --bridge`) inside `crates/fge-bridge`:

1. **Zero-Pointer C-ABI AI Mailbox (`include/fge.h` & `crates/fge-abi`)**:
   - Extends `include/fge.h` with a fixed-layout value struct `FgeAiEvent` (`20` bytes, `#[repr(C)]`, zero raw pointers) and 5 exported `extern "C"` functions (47 total `fge_*` symbols retained via `ensure_abi_symbols_linked()`):
     - `fge_param_set(param_id, value)` / `fge_param_get(param_id, default_val)`: runtime parameter tuning mailbox shared between external AI clients and C game code.
     - `fge_poll_ai_events()` / `fge_get_ai_event(index)`: double-buffered per-frame AI event queue (`pending_ai_events` -> `active_ai_events`) drained safely inside `fge_update`.
     - `fge_log_metric(metric_id, value)`: lightweight C-to-Bridge telemetry channel captured in snapshots and headless reports.

2. **Unified JSON-RPC 2.0 Protocol (`fge_bridge::protocol`)**:
   - Line-delimited JSON-RPC 2.0 (`NDJSON-RPC`) handled synchronously by `handle_rpc_line(line, ctx, state) -> String`.
   - **Observation**: `fge.ping`, `fge.world.snapshot`, `fge.world.query_nearby`, `fge.entity.get`, `fge.param.get`.
   - **Intervention**: `fge.entity.spawn`, `fge.entity.despawn`, `fge.entity.patch` (including per-entity NPC `dialogue` stored in `BridgeState`), `fge.input.inject`, `fge.event.emit`, `fge.param.set`, `fge.camera.set`.
   - Standard JSON-RPC error codes (`-32700` parse error, `-32601` method not found, `-32602` invalid params, `-32001` stale/dead `EntityId`).

3. **Single-Threaded Non-Blocking TCP `BridgeServer` (`fge_bridge::server`)**:
   - Bound via `std::net::TcpListener` in non-blocking mode (`set_nonblocking(true)`), maintaining non-blocking `TcpStream` clients with per-connection line buffers.
   - **Zero background threads and zero `Arc<Mutex<T>>`** (`MODEL.md` compliance): `BridgeServer::poll_frame(&mut ctx)` runs once per frame on the main loop right before `reloader.step_frame(&mut ctx, dt)`, accepting connections, draining complete `\n`-terminated requests, mutating `EngineContext`/`BridgeState` directly, writing `\n`-terminated responses, and pruning dead entity dialogues.

4. **GPU-Free Headless Test Runner (`fge_bridge::headless` & `fge_bridge::snapshot`)**:
   - `run_headless_test(&HeadlessConfig) -> HeadlessTestReport` skips `winit` and `wgpu` completely, compiling and loading the C11 project via `fge-loader` and stepping `EngineContext` deterministically with `seed` and fixed `dt`.
   - Captures structured JSON diagnostics (`HeadlessTestReport`): status (`"ok"`, `"compile_error"`, `"load_error"`, `"config_error"`, `"runtime_error"`), `CompileDiagnostic` (`compiler`, `stderr`, `exit_code`), `SimTimingReport` (`total_ms`, `avg_step_us`, `max_step_us`), collision totals/samples, and deterministic `WorldSnapshot`.

5. **CLI Integration (`crates/fge`)**:
   - `fge test <project_dir> --frames 300 --seed 42 --dt 0.0166667 --dump-json` executes `run_test_cmd` and emits structured JSON or concise summary output.
   - `fge run <project_dir> --bridge 127.0.0.1:7777` enables `BridgeServer` in both `--offscreen` and windowed `winit` modes.

## 2. Updated Crate Dependency Graph
```text
fge (bin + lib)
 ├── fge-bridge (lib)
 │    ├── fge-loader (lib)
 │    │    ├── fge-abi (lib)
 │    │    │    └── fge-core (lib)
 │    │    └── fge-core (lib)
 │    ├── fge-abi (lib)
 │    │    └── fge-core (lib)
 │    └── fge-core (lib)
 ├── fge-loader (lib)
 ├── fge-abi (lib)
 ├── fge-render (lib)
 │    └── fge-core (lib)
 └── fge-core (lib)
```

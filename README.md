# FGE — Fast Game Engine

A **code-first, ultra-fast, lightweight 2D game engine** written in Rust with a **zero-pointer C11 gameplay ABI**, **single-draw-call `wgpu` instanced rendering**, **native C hot-reloading**, and a built-in **AI Bridge** (GPU-free deterministic headless test runner + live JSON-RPC 2.0 TCP intervention server).

Built for bullet-hell STGs, RPGs, and 2D top-down open worlds that demand tens to hundreds of thousands of active entities at native speed.

---

## Architecture

FGE is structured as a 6-crate Cargo workspace with strict Data-Oriented Design (DOD / SoA), generational handles, zero `Arc<Mutex<T>>` lock contention, and native release tuning (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `-C target-cpu=native` in `.cargo/config.toml`):

```text
crates/
├── fge-core    # Pure CPU simulation kernel: SoA EntityWorld, Generational EntityId,
│               # Camera2D, O(N) SpatialHashGrid, BulkSimulation, CoreInputState, FrameStatsTracker
├── fge-abi     # Thread-local EngineContext + 47 zero-pointer #[no_mangle] extern "C" symbols
│               # matching include/fge.h (handles, physics, collisions, input, AI mailbox/metrics)
├── fge-loader  # Native C11 multi-file compiler driver (tcc / clang / gcc, FGE_CC override),
│               # embedded fge.h staging, dlopen LoadedGameModule, mtime-based HotReloader
├── fge-bridge  # GPU-free deterministic run_headless_test() + structured HeadlessTestReport,
│               # WorldSnapshot capture, JSON-RPC 2.0 protocol, single-threaded non-blocking TcpListener
├── fge-render  # Single-draw-call instanced 2D wgpu renderer, Y-sort + Camera2D frustum culling,
│               # windowed (winit) & offscreen render targets
└── fge         # CLI binary (`fge bench`, `fge run`, `fge test`), game.toml manifest loader, app loop

include/fge.h        # Public C11 API header (strict -std=c11 -Wall -Wextra -Werror -pedantic, 0 raw pointers)
examples/demo_game/  # Multi-file C11 STG demo (game.toml + src/{main,player,enemy}.c)
```

### Core Guarantees
- **Zero raw pointers across C-ABI**: Game C code interacts solely through value-type structs and generational handles (`FgeEntity { index, generation }`). Stale or invalid handles safely return `false` / zero values without panicking.
- **Single-threaded simulation & live bridge**: `BridgeServer::poll_frame(&mut ctx)` drains non-blocking TCP NDJSON-RPC requests once per frame on the main loop — zero background threads and zero `Arc<Mutex<T>>`.
- **1-draw-call instanced rendering**: `EntityWorld` SoA arrays pack directly into a GPU instance buffer with optional `Camera2D` AABB frustum culling and `FGE_FLAG_Y_SORT` ordering.

---

## CLI Usage

Build and run with Cargo (or invoke the compiled `fge` binary directly):

```bash
# 1. Single-draw-call rendering & SoA physics benchmark (100,000 sprites)
cargo run --release -p fge -- bench --count 100000 --frames 300 --offscreen --json
fge bench --count 100000 --cull --vsync

# 2. Run a C11 game project (auto-compiles game.toml sources & hot-reloads on edit)
fge run ./examples/demo_game
fge run ./examples/demo_game --offscreen --frames 60 --no-hot-reload

# 3. GPU-free deterministic headless test runner (structured JSON report for AI loops)
fge test ./examples/demo_game --frames 600 --seed 42 --dt 0.0166667 --dump-json

# 4. Launch a game with the live JSON-RPC 2.0 AI Bridge enabled
fge run ./examples/demo_game --bridge 127.0.0.1:7777
```

> **Compiler selection**: `fge-loader` automatically detects `tcc`, `clang`, or `gcc` (or honors the `FGE_CC` environment variable) and embeds `include/fge.h` automatically so game projects require no manual include paths.

---

## Quickstart (`game.toml` + `fge.h`)

A game project is a plain directory containing `game.toml` and one or more C11 source files:

```toml
# my_game/game.toml
[game]
title = "My FGE Game"
width = 1280
height = 720
target_fps = 60
vsync = false

[build]
entry = "src/main.c"
sources = ["src/enemy.c"]
```

```c
/* my_game/src/main.c */
#include "fge.h"

static FgeEntity g_player;

void fge_init(void) {
    fge_camera_set(640.0f, 360.0f, 1.0f);
    g_player = fge_spawn_sprite(640.0f, 360.0f, 24.0f, 24.0f, fge_rgba(80, 220, 255, 255));
    fge_set_collider(g_player, 12.0f, 1u);
    fge_set_hp(g_player, 100.0f);
}

void fge_update(float dt) {
    (void)dt;
    float speed = fge_param_get(1u, 280.0f); /* Live-tunable via AI Bridge */
    float vx = (fge_key_down(FGE_KEY_RIGHT) ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_LEFT)  ? 1.0f : 0.0f);
    float vy = (fge_key_down(FGE_KEY_DOWN)  ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_UP)    ? 1.0f : 0.0f);
    fge_set_velocity(g_player, vx * speed, vy * speed);

    /* Drain incoming live AI events */
    uint32_t ev_count = fge_poll_ai_events();
    for (uint32_t i = 0; i < ev_count; ++i) {
        FgeAiEvent ev = fge_get_ai_event(i);
        if (ev.kind == 1u) {
            fge_spawn_sprite(ev.arg0, ev.arg1, 16.0f, 16.0f, fge_rgba(255, 220, 40, 255));
        }
    }

    fge_log_metric(1u, (float)fge_entity_count());
}

void fge_shutdown(void) {
    if (fge_entity_is_alive(g_player)) {
        fge_entity_despawn(g_player);
    }
}
```

Optional hot-reload hook: export `void fge_on_reload(void)` in your C module to run custom re-initialization after a live code reload.

---

## Live AI Bridge (JSON-RPC 2.0 over TCP)

When started with `fge run <dir> --bridge 127.0.0.1:7777`, FGE accepts newline-delimited JSON-RPC 2.0 (`NDJSON-RPC`) requests over TCP:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"fge.ping","params":{}}\n' | nc 127.0.0.1 7777
```

### Supported RPC Methods

| Category | Method | Params | Description |
| --- | --- | --- | --- |
| Health | `fge.ping` | `{}` | Returns `ok`, current `frame`, and live `entities` count. |
| Observe | `fge.world.snapshot` | `{ limit?: usize }` | Captures `WorldSnapshot` (frame, camera, entities, collisions, params, metrics). |
| Observe | `fge.world.query_nearby` | `{ x, y, radius, layer_mask? }` | Lists `EntitySnapshot`s within circle `(x, y, radius)`. |
| Observe | `fge.entity.get` | `{ index, generation }` | Fetches a single live `EntitySnapshot` (or `-32001` if dead/stale). |
| Observe | `fge.param.get` | `{ id, default? }` | Reads a numeric runtime tuning parameter. |
| Intervene | `fge.entity.spawn` | `{ x, y, w?, h?, color?, vel?, hp?, tag?, collider_radius?, layer_mask?, dialogue? }` | Spawns a sprite entity and returns its generational `EntityId`. |
| Intervene | `fge.entity.despawn` | `{ index, generation }` | Despawns an entity and clears its dialogue metadata. |
| Intervene | `fge.entity.patch` | `{ index, generation, pos?, vel?, size?, hp?, tag?, color?, flags?, dialogue? }` | Mutates live entity fields and/or attaches NPC dialogue text. |
| Intervene | `fge.input.inject` | `{ key, pressed }` | Injects a `FgeKey` press/release into `CoreInputState`. |
| Intervene | `fge.event.emit` | `{ kind, target?, arg0?, arg1? }` | Enqueues an `FgeAiEvent` for C code to read via `fge_poll_ai_events()`. |
| Intervene | `fge.param.set` | `{ id, value }` | Sets a numeric parameter readable by C code via `fge_param_get()`. |
| Intervene | `fge.camera.set` | `{ center_x, center_y, zoom }` | Updates the active `Camera2D` center and zoom. |

**Error Codes**: `-32700` (parse error), `-32601` (method not found), `-32602` (invalid params), `-32001` (entity dead or stale handle).

### Headless Test Report (`fge test --dump-json`)
`run_headless_test` emits a `HeadlessTestReport` JSON object containing:
- `status`: `"ok"`, `"compile_error"`, `"load_error"`, `"config_error"`, or `"runtime_error"`
- `compile_error`: `{ compiler, stderr, exit_code }` when C11 compilation fails (allowing AI agents to self-heal syntax/type errors immediately)
- `sim_timing`: `{ total_ms, avg_step_us, max_step_us }`
- `collision_events_total` & `collision_samples` (up to 64 sampled `CollisionPair`s)
- `world_snapshot`: final deterministic `WorldSnapshot` (captured prior to `fge_shutdown`)

---

## Development & Verification

```bash
# Check & test each workspace crate
for crate in fge-core fge-abi fge-loader fge-bridge fge-render fge; do
  cargo check -p "$crate"
  cargo test -p "$crate" --lib
done

# Strict C11 public header validation
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic include/fge.h
```

---

## Open Design Decisions (`TODO(user-decision)`)

The following design trade-offs are tagged directly in the codebase for user review:

1. **Spatial Hash Grid defaults** — [`crates/fge-abi/src/context.rs:71`](crates/fge-abi/src/context.rs#L71):
   `SpatialHashGrid` defaults to `cell_size = 64.0` and `buckets = 4096` for general 2D STG/RPG scenes; consider exposing these in `game.toml` for per-game tuning.
2. **C11 compiler backend priority** — [`crates/fge-loader/src/compiler.rs:30`](crates/fge-loader/src/compiler.rs#L30):
   Evaluate optional in-process `libtcc` static linking versus external `tcc` / `clang` / `gcc` driver priority (`FGE_CC` override is already supported).
3. **Hot-reload behavior when `fge_on_reload` is absent** — [`crates/fge-loader/src/hot_reload.rs:72`](crates/fge-loader/src/hot_reload.rs#L72):
   Currently keeps the live `EntityWorld` untouched on reload if the module does not export `fge_on_reload`; decide whether to offer an optional flag to re-run `fge_init`.
4. **Y-sort tie-breaking precision for huge open worlds** — [`crates/fge-render/src/world_batch.rs:122`](crates/fge-render/src/world_batch.rs#L122):
   Currently uses `z_order + pos_y * 1e-4` (cleanly separates integer `z_order` layers up to 10,000px); switch to a `(i32 layer, f32 y)` composite key if open-world coordinates exceed 100k px.
5. **Optional Unix Domain Socket transport for AI Bridge** — [`crates/fge-bridge/src/server.rs:24`](crates/fge-bridge/src/server.rs#L24):
   Currently uses non-blocking TCP loopback (`127.0.0.1:<port>`); evaluate adding an optional Unix Domain Socket listener on Linux/macOS for lower-latency local IPC.

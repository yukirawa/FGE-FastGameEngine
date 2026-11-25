# FGE — Fast Game Engine

A **code-first, ultra-fast, lightweight 2D game engine** written in Rust with a **zero-pointer C11 gameplay ABI**, **single-pass `wgpu` instanced rendering & 2048×2048 dynamic PNG/CJK glyph atlas**, **lock-free audio & chiptune synth/BGM**, **Rust-side bulk bullet emitters**, **native C hot-reloading**, and a built-in **AI Bridge** (GPU-free deterministic headless test runner + live JSON-RPC 2.0 TCP intervention server).

Built for bullet-hell STGs, 2D Action RPGs, and top-down open worlds that demand tens to hundreds of thousands of active entities at native speed.

---

## Architecture

FGE is structured as a 7-crate Cargo workspace with strict Data-Oriented Design (DOD / SoA), generational handles, zero `Arc<Mutex<T>>` lock contention, and native release tuning (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `-C target-cpu=native` in `.cargo/config.toml`):

```text
crates/
├── fge-core    # Pure CPU simulation kernel: SoA EntityWorld (with native TTL auto-expiry),
│               # Generational EntityId, EmitterWorld (Rust-side bulk bullet/particle offload),
│               # zero-alloc UiCommandQueue, Camera2D, O(N) SpatialHashGrid, BulkSimulation,
│               # CoreInputState, FrameStatsTracker
├── fge-audio   # Lock-free audio engine (sync_channel -> cpal 0.15.3, ZERO Arc<Mutex<T>>):
│               # 8/16/32-bit WAV loader (FgeSound), procedural chiptune SFX synth,
│               # multi-channel RPG BGM sequencer, automatic headless silent fallback
├── fge-abi     # Thread-local EngineContext + 68 zero-pointer #[no_mangle] extern "C" symbols
│               # matching include/fge.h (entities, physics, collisions, input, AI bridge,
│               # audio, TTL/emitters, PNG textures, immediate-mode Japanese/UTF-8 Text & UI)
├── fge-loader  # Native C11 multi-file compiler driver (gcc / clang / tcc priority, FGE_CC & [build] compiler),
│               # embedded fge.h staging, dlopen LoadedGameModule, opt-in fge_on_reload HotReloader
├── fge-bridge  # GPU-free deterministic run_headless_test() + structured HeadlessTestReport,
│               # WorldSnapshot capture, JSON-RPC 2.0 protocol, single-threaded non-blocking TcpListener
├── fge-render  # Instanced 2D wgpu renderer with Y-sort + Camera2D frustum culling,
│               # 2048x2048 DynamicAtlas (PNG loader + ab_glyph Japanese/CJK & 8x8 ASCII glyph cache),
│               # unified world + screen-space UI pass, windowed (winit) & offscreen targets
└── fge         # CLI binary (`fge bench`, `fge run`, `fge test`), game.toml manifest loader, app loop

include/fge.h               # Public C11 API header (strict -std=c11 -Wall -Wextra -Werror -pedantic, 0 raw pointers)
examples/demo_game/         # Multi-file C11 STG starter demo (game.toml + src/{main,player,enemy}.c)
examples/neon_survivors/    # Neon bullet-heaven survival action showcase (6 C11 modules + auto-play AI)
examples/crystal_chronicle/ # Playable Japanese 2D Action RPG (6 C11 modules, dialog UI, BGM, boss emitters)
```

### Core Guarantees
- **Zero raw pointers across C-ABI**: Game C code interacts solely through value-type structs (`FgeTextBuf` via `fge_str("...")`) and generational handles (`FgeEntity`, `FgeSound`, `FgeEmitter`, `FgeTexture`). Stale or invalid handles safely return `false` / zero values without panicking.
- **Zero `Arc<Mutex<T>>` lock contention**: Simulation and live `BridgeServer::poll_frame(&mut ctx)` run single-threaded on the main loop, while `fge-audio` feeds the `cpal` output thread over a bounded lock-free `sync_channel`.
- **High-throughput instanced rendering & bulk offload**: `EntityWorld` SoA arrays pack directly into GPU instance buffers with optional `Camera2D` AABB frustum culling, `FGE_FLAG_Y_SORT` ordering (`fast` or `composite`), and `EmitterWorld` native Rust bullet spawning + TTL expiry.

---

## Step 5 Highlights: Audio, Bulk Emitters & Japanese Text/UI

- **Lock-Free Audio (`fge-audio`)**:
  - **WAV Playback**: Load 8-bit PCM, 16-bit PCM, or 32-bit float WAV files via `fge_sound_load_wav(fge_str("assets/hit.wav"))` + `fge_sound_play(sound, vol)` with generational `FgeSound` validation.
  - **Procedural Chiptune SFX & Custom Tones**: 8 built-in presets (`FGE_SFX_SLASH`, `FGE_SFX_HIT`, `FGE_SFX_EXPLOSION`, `FGE_SFX_PICKUP`, `FGE_SFX_BLIP`, `FGE_SFX_CONFIRM`, `FGE_SFX_HEAL`, `FGE_SFX_LEVEL_UP`) plus `fge_audio_play_tone` (`Square`, `Triangle`, `Saw`, `Noise`).
  - **Looping RPG BGM Sequencer**: Switch seamlessly between `FGE_BGM_VILLAGE`, `FGE_BGM_FIELD`, `FGE_BGM_BATTLE`, and `FGE_BGM_BOSS` via `fge_bgm_play_preset` / `fge_bgm_stop`. Automatically falls back to silent headless mode during `fge test` or on hosts without audio hardware.
- **Native TTL & Rust-Side Bulk Emitters (`FgeEmitter`)**:
  - `fge_set_ttl(entity, seconds)` automatically despawns short-lived slashes, particles, and projectiles inside Rust `step_kinematics`.
  - `fge_emitter_create_radial(...)`, `fge_emitter_set_position`, `fge_emitter_set_spin`, and `fge_emitter_set_active` offload spiral/radial danmaku patterns directly to Rust's `EmitterWorld`, avoiding per-bullet C-ABI crossings.
- **2048×2048 Dynamic Atlas & Zero-Pointer UTF-8/Japanese UI**:
  - `fge_texture_load_png(fge_str("assets/hero.png"))` + `fge_set_texture(entity, tex)` dynamically shelf-pack PNG sprites into the GPU atlas with dirty-rect uploads.
  - `ab_glyph` rasterizes UTF-8 / Japanese CJK fonts (auto-probing system CJK fonts or loading `[ui].font` from `game.toml`, with a built-in 8×8 ASCII bitmap fallback).
  - Immediate-mode UI calls (`fge_ui_panel`, `fge_ui_bar`, `fge_ui_text`, `fge_ui_number`, `fge_ui_dialog_box`, `fge_world_text`) pass strings by value in `FgeTextBuf` (`fge_str("勇者よ！北の遺跡へ！")`) into a zero-allocation reusable `UiCommandQueue`.

---

## Example Games

| Project | Genre | Highlights | Quick Run |
| --- | --- | --- | --- |
| [`examples/demo_game`](examples/demo_game) | Arcade STG | Multi-file C11 starter (`main`, `player`, `enemy`), spatial collisions, AI Bridge metrics | `fge run ./examples/demo_game` |
| [`examples/neon_survivors`](examples/neon_survivors) | Bullet-Heaven Action | Wave director, auto-targeting weapons, XP gems, screen shake, hot-reload, autopilot AI | `fge run ./examples/neon_survivors` |
| [`examples/crystal_chronicle`](examples/crystal_chronicle) | Japanese 2D Action RPG | Y-sorted 2200×2000 world (Village / Whispering Woods / Northern Ruins), Hero sword slash & dash, 村長 & 森の妖精 NPCs with typewriter `fge_ui_dialog_box`, Slimes/Goblins/Obsidian Golem boss using `FgeEmitter` spirals, dynamic BGM, `fge_on_reload`, smart auto-adventure AI | `fge run ./examples/crystal_chronicle` |

---

## CLI Usage

Build and run with Cargo (or invoke the compiled `fge` binary directly):

```bash
# 1. Single-draw-call rendering & SoA physics benchmark (100,000 sprites)
cargo run --release -p fge -- bench --count 100000 --frames 300 --offscreen --json
fge bench --count 100000 --cull --vsync

# 2. Run a C11 game project (auto-compiles game.toml sources & hot-reloads on edit)
fge run ./examples/crystal_chronicle
fge run ./examples/neon_survivors
fge run ./examples/demo_game --offscreen --frames 60 --no-hot-reload

# 3. GPU-free deterministic headless test runner (structured JSON report for AI loops)
fge test ./examples/crystal_chronicle --frames 900 --seed 42 --dump-json
fge test ./examples/neon_survivors --frames 600 --seed 42 --dump-json
fge test ./examples/demo_game --frames 600 --seed 42 --dt 0.0166667 --dump-json

# 4. Launch a game with the live JSON-RPC 2.0 AI Bridge enabled (with optional API key auth)
fge run ./examples/crystal_chronicle --bridge 127.0.0.1:7777
fge run ./examples/demo_game --bridge 127.0.0.1:7777 --bridge-api-key secret-token
```

> **Compiler selection**: `fge-loader` prefers `gcc`, then `clang`, then `tcc` (or honors `[build] compiler` in `game.toml` and the `FGE_CC` environment variable) and embeds `include/fge.h` automatically so game projects require no manual include paths.

---

## Quickstart (`game.toml` + `fge.h`)

A game project is a plain directory containing `game.toml` and one or more C11 source files:

```toml
# my_rpg/game.toml
[game]
title = "My FGE RPG"
width = 1280
height = 720
target_fps = 60
vsync = false

[build]
entry = "src/main.c"
sources = ["src/world.c", "src/enemies.c"]
compiler = "gcc"          # Optional: "gcc" | "clang" | "tcc" (default probe: gcc -> clang -> tcc)

[spatial]
cell_size = 64.0          # SpatialHashGrid cell size in world units (> 0.0)
buckets = 4096            # SpatialHashGrid bucket count (must be a positive power of two)

[render]
y_sort_mode = "composite" # "fast" (z_order + y*1e-4) or "composite" ((layer, y, z) for huge worlds)

[audio]
master_volume = 0.8       # Master gain >= 0.0 (default: 0.8)
muted = false             # Disable audio output if true (default: false)

[ui]
# font = "assets/NotoSansCJK-Regular.ttc" # Optional custom TTF/OTF/TTC font (auto-probes system CJK if omitted)
```

```c
/* my_rpg/src/main.c */
#include "fge.h"

static FgeEntity  g_hero;
static FgeEmitter g_boss_ring;

void fge_init(void) {
    fge_camera_set(640.0f, 360.0f, 1.0f);
    fge_bgm_play_preset(FGE_BGM_VILLAGE, 0.45f);

    g_hero = fge_spawn_sprite(640.0f, 360.0f, 24.0f, 24.0f, fge_rgba(80, 220, 255, 255));
    fge_set_collider(g_hero, 12.0f, 1u);
    fge_set_hp(g_hero, 100.0f);

    /* Rust-side radial bullet emitter (zero per-bullet FFI overhead) */
    g_boss_ring = fge_emitter_create_radial(
        640.0f, 180.0f, 2.0f, 8u, 180.0f, 2.5f, 6.0f, 2u, fge_rgba(220, 80, 255, 255)
    );
    fge_emitter_set_spin(g_boss_ring, 0.0f, 1.2f);
}

void fge_update(float dt) {
    (void)dt;
    float speed = fge_param_get(1u, 260.0f); /* Live-tunable via AI Bridge */
    float vx = (fge_key_down(FGE_KEY_RIGHT) ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_LEFT)  ? 1.0f : 0.0f);
    float vy = (fge_key_down(FGE_KEY_DOWN)  ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_UP)    ? 1.0f : 0.0f);
    fge_set_velocity(g_hero, vx * speed, vy * speed);

    if (fge_key_pressed(FGE_KEY_SPACE)) {
        fge_audio_play_sfx(FGE_SFX_SLASH, 0.8f, 1.0f);
    }

    /* Zero-pointer immediate-mode Japanese RPG UI */
    fge_ui_panel(16.0f, 16.0f, 320.0f, 84.0f, fge_rgba(14, 18, 34, 215), fge_rgba(220, 195, 110, 255), 2.0f);
    fge_ui_text(30.0f, 26.0f, 18.0f, fge_rgba(255, 232, 128, 255), fge_str("勇者 LV.1"));
    fge_ui_bar(30.0f, 58.0f, 220.0f, 14.0f, fge_get_hp(g_hero) / 100.0f,
               fge_rgba(68, 224, 118, 255), fge_rgba(28, 20, 32, 235));

    fge_log_metric(1u, (float)fge_entity_count());
}

void fge_shutdown(void) {
    fge_bgm_stop();
    fge_emitter_destroy(g_boss_ring);
    if (fge_entity_is_alive(g_hero)) {
        fge_entity_despawn(g_hero);
    }
}
```

**Hot-reload opt-in (`fge_on_reload`)**: `HotReloader` only hot-reloads C modules that export `void fge_on_reload(void)`. Modules that omit `fge_on_reload` skip hot-reloading (`poll_and_reload` returns `Ok(false)`) so live simulation state is preserved cleanly.

---

## Live AI Bridge (JSON-RPC 2.0 over TCP)

When started with `fge run <dir> --bridge 127.0.0.1:7777`, FGE accepts newline-delimited JSON-RPC 2.0 (`NDJSON-RPC`) requests over TCP:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"fge.ping","params":{}}\n' | nc 127.0.0.1 7777
```

### Bridge Authentication (`--bridge-api-key` / `FGE_BRIDGE_API_KEY`)

Pass `--bridge-api-key <SECRET>` (or set `FGE_BRIDGE_API_KEY=<SECRET>`) to require authentication on the TCP bridge:
- Clients can authenticate their TCP session once via `fge.auth` (`{"api_key": "<SECRET>"}`), after which subsequent requests on that connection succeed without re-sending the key.
- Alternatively, clients can pass `"api_key": "<SECRET>"` inside `params` on individual requests.
- Unauthenticated requests fail with JSON-RPC error `-32003` (`Unauthorized`).

### Supported RPC Methods

| Category | Method | Params | Description |
| --- | --- | --- | --- |
| Auth | `fge.auth` | `{ api_key }` | Authenticates the current TCP connection for subsequent RPC calls. |
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

**Error Codes**: `-32700` (parse error), `-32601` (method not found), `-32602` (invalid params), `-32001` (entity dead or stale handle), `-32003` (unauthorized / missing or invalid API key).

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
# Check & test all 7 workspace crates individually
for crate in fge-core fge-audio fge-abi fge-loader fge-bridge fge-render fge; do
  cargo check -p "$crate"
  cargo test -p "$crate" --lib
done

# Strict C11 validation of public header and all 3 example games
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic include/fge.h
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include examples/demo_game/src/*.c
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include -I examples/neon_survivors/src examples/neon_survivors/src/*.c
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include -I examples/crystal_chronicle/src examples/crystal_chronicle/src/*.c
```

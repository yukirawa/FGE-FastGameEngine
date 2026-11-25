# FGE Phase 5 (Step 5) Architecture Specification: Audio, Bulk Emitters, Dynamic Atlas & Zero-Pointer Text UI

## 1. Scope & Design Principles
Phase 5 (`PROJECT.md` Section 5, Step 5) expands FGE from a core simulation & AI-tested C11 engine into a full-featured 2D Action RPG & Danmaku production runtime across 4 pillars, while strictly adhering to `MODEL.md` guardrails (**zero `Arc<Mutex<T>>`**, **zero raw pointers across C-ABI**, **SoA + Generational Handle validation**, and **clean `// SAFETY:` invariants**):

1. **Lock-Free Audio Subsystem (`crates/fge-audio`)**:
   - **Single-Producer / Single-Consumer Command Channel (Zero `Arc<Mutex<T>>`)**: `AudioEngine` sends `AudioCommand` values through a bounded `std::sync::mpsc::sync_channel::<AudioCommand>(512)` to a `MixerState` owned exclusively by the `cpal` 0.15.3 audio output callback thread.
   - **WAV Loader (`fge_audio::wav`)**: Pure-Rust RIFF/WAVE parser supporting 8-bit unsigned PCM, 16-bit signed PCM (`format_tag = 1`), and 32-bit IEEE float (`format_tag = 3`), mono or stereo, resampled via linear interpolation to the target mixer sample rate as interleaved stereo `[f32; 2]` frames. Validated by `GenerationalArena` (`SoundId = EntityId`, C-ABI `FgeSound`).
   - **Procedural Chiptune Synthesizer & Looping RPG BGM Sequencer (`fge_audio::synth`)**:
     - `SynthVoice` with phase-accumulated `Square`, `Triangle`, `Saw`, and LFSR `Noise` waveforms, linear/exponential frequency sweeps, fast attack + exponential decay envelopes, and anti-click fade-outs.
     - 8 tuned `SfxPreset` effects (`Slash`, `Hit`, `Explosion`, `Pickup`, `Blip`, `Confirm`, `Heal`, `LevelUp`).
     - Multi-channel `BgmSequencer` (melody + bass + light percussion) for `Village`, `Field`, `Battle`, and `Boss`.
   - **Automatic Headless Silent Fallback**: `AudioEngine::new_live()` probes `cpal::default_host().default_output_device()`; when no audio hardware/ALSA device is present (or in `fge test` / `--offscreen`), it transparently falls back to `AudioEngine::new_headless()`, preserving `SoundId` generational validation and exposing `render_test_frames(&mut self, frames: usize)` for deterministic unit tests.

2. **Performance & Practical Gameplay Polish (`crates/fge-core` & `crates/fge-render`)**:
   - **Native Entity TTL (`EntityWorld::ttl`)**: Adds a packed `ttl: Vec<f32>` SoA column (`0.0` = infinite lifetime; `> 0.0` = seconds remaining). `step_kinematics(dt)` decrements active TTLs in a reverse-index loop (`(0..len).rev()`) and auto-despawns expired entities, keeping `swap_remove` O(1) and safe against skipped indices.
   - **Rust-Side Bulk Bullet/Particle Offload (`EmitterWorld` / `FgeEmitter`)**: `GenerationalArena`-backed SoA emitter table (`pos_x/y`, `rate`, `accumulator`, `burst_count`, `speed`, `bullet_ttl`, `collider_radius`, `layer_mask`, `size`, `color_rgba`, `base_angle`, `ang_vel`, `active`). `step_emitters(dt, &mut EntityWorld)` advances spin angles and accumulators and batch-spawns radial/spiral bullets with velocity, colliders, and TTL directly in native Rust—eliminating per-bullet C-to-Rust FFI overhead.
   - **Zero-Allocation Immediate-Mode UI Queue (`UiCommandQueue`)**: Retains a single reusable `text_arena: String` and `cmds: Vec<UiDrawCmd>` storing `Range<u32>` byte spans (`Panel`, `Bar`, `ScreenText`, `WorldText`, `DialogBox`). Clearing `.clear()` each frame preserves buffer capacity for zero per-frame heap allocations after warm-up. Includes `truncate_utf8_chars(s, max_chars)` for panic-free UTF-8 character-boundary slicing (e.g., Japanese RPG typewriter dialogue).
   - **Dynamic 2048x2048 GPU Shelf Atlas (`DynamicAtlas`)**:
     - `2048 x 2048` RGBA8 CPU backing mirror + shelf packer + dirty-rect GPU upload (`queue.write_texture`).
     - Reserves the top-left `8x8` block as opaque white (`SOLID_WHITE_UV`) while `shader.wgsl` preserves `FULL_UV_RECT` (`[0, 0, u16::MAX, u16::MAX]`) compatibility for untextured sprites.
     - `png` 0.18.1 decoder (`FgeTexture` generational handle via `fge_texture_load_png` / `fge_set_texture`).
     - `ab_glyph` 0.2.32 UTF-8/Japanese CJK font rasterizer (auto-probing system CJK/Latin fonts or loading `[ui].font` from `game.toml`) paired with a built-in `8x8` ASCII bitmap fallback (`0x20..=0x7E`), cached by `(char, quantized_px_u16)`.
   - **Unified Render Pass (`GpuRenderer::render_world_and_ui`)**: Renders world entities + `WorldText` under the active `Camera2D` and screen-space `Panel`/`Bar`/`DialogBox`/`ScreenText` under a top-left screen projection, while keeping `render_soa` and `render_world` intact at 1 draw call.

3. **Zero-Pointer C-ABI Value Strings & Handles (`include/fge.h` & `crates/fge-abi`)**:
   - **By-Value Fixed UTF-8 Buffer**: `FgeTextBuf { uint8_t bytes[120]; uint32_t len; }` (`124` bytes, `#[repr(C)]`) crosses `extern "C"` strictly **by value**, constructed on the C stack via `static inline FgeTextBuf fge_str(const char *utf8_literal)`.
   - **Generational Handle Value Types**: `FgeSound`, `FgeEmitter`, and `FgeTexture` (`{ uint32_t index; uint32_t generation; }`, 8 bytes each, `{0, 0}` / `{0xFFFFFFFF, 0}` guaranteed invalid).
   - **Zero raw pointers** appear in any `extern "C"` function signature in `include/fge.h`.

## 2. Updated Crate Dependency Graph
```text
fge (bin + lib)
 ├── fge-audio (lib)
 │    ├── fge-core (lib)
 │    └── cpal (0.15.3)
 ├── fge-bridge (lib)
 │    ├── fge-loader (lib)
 │    │    ├── fge-abi (lib)
 │    │    └── fge-core (lib)
 │    ├── fge-abi (lib)
 │    └── fge-core (lib)
 ├── fge-loader (lib)
 ├── fge-abi (lib)
 │    ├── fge-audio (lib)
 │    └── fge-core (lib)
 ├── fge-render (lib)
 │    ├── fge-core (lib)
 │    ├── ab_glyph (0.2.32)
 │    └── png (0.18.1)
 └── fge-core (lib)
```

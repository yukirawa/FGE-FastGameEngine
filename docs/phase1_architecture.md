# FGE Phase 1 (Step 1) Architecture Specification

## 1. Scope & Goals
Phase 1 (`PROJECT.md` Step 1) establishes the FGE foundation:
1. **Native-Optimized Cargo Workspace**:
   - `.cargo/config.toml` enabling `-C target-cpu=native` (AVX2/FMA auto-vectorization on x86_64).
   - `[profile.release]` configured with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.
2. **Modular Crate Boundaries**:
   - `fge-core`: Pure Data-Oriented Design (DOD) / Structure-of-Arrays (SoA) simulation (`SpriteSoA`), `Camera2D` orthographic projection & frustum culling, and `FrameStatsTracker` / `BenchSummary`. Zero windowing or GPU dependencies.
   - `fge-render`: `wgpu` 30.0.1 1-draw-call instanced 2D renderer (`SpriteInstance`, `SpriteBatchPipeline`, `GpuRenderer`), supporting both windowed (`wgpu::Surface<'static>`) and offscreen (`wgpu::Texture`) targets without depending on `winit`.
   - `fge`: CLI (`clap`), `game.toml` manifest loader (`toml` + `serde`), `winit` 0.30.13 input abstraction (`InputState`), and benchmark/application runner (`fge bench`, `fge run`).

## 2. Dependency Graph
```text
fge (bin + lib)
 ├── fge-render (lib)
 │    └── fge-core (lib)
 └── fge-core (lib)
```

## 3. Data-Oriented Memory Layout

### 3.1 `SpriteSoA` (`fge-core::sim`)
All per-object attributes are stored in parallel contiguous `Vec` buffers of identical length `N` (invariants checked via `debug_assert_soa_invariants`):
- `pos_x: Vec<f32>`, `pos_y: Vec<f32>`
- `vel_x: Vec<f32>`, `vel_y: Vec<f32>`
- `size_w: Vec<f32>`, `size_h: Vec<f32>`
- `rotation: Vec<f32>`, `ang_vel: Vec<f32>`
- `uv_rect: Vec<[u16; 4]>` (`[0, 0, 65535, 65535]` for full quad)
- `color_rgba: Vec<[u8; 4]>`

Physics integration (`BulkSimulation::step`) iterates over independent contiguous `&mut [f32]` / `&[f32]` slices (`pos_x`/`vel_x`, `pos_y`/`vel_y`, `rotation`/`ang_vel`) so LLVM emits wide SIMD (AVX2/FMA) vector loops without pointer chasing.

### 3.2 `SpriteInstance` (`fge-render::instance`) — Exactly 32 Bytes
Two instances fit cleanly into a single 64-byte CPU/GPU cache line with zero padding:
| Offset | Field | Rust Type | `wgpu::VertexFormat` | WGSL `@location` |
|---:|---|---|---|---|
| 0 | `pos` | `[f32; 2]` | `Float32x2` | `@location(0) vec2<f32>` |
| 8 | `size` | `[f32; 2]` | `Float32x2` | `@location(1) vec2<f32>` |
| 16 | `uv_rect` | `[u16; 4]` | `Unorm16x4` | `@location(2) vec4<f32>` |
| 24 | `rotation` | `f32` | `Float32` | `@location(3) f32` |
| 28 | `color` | `[u8; 4]` | `Unorm8x4` | `@location(4) vec4<f32>` |

Total stride: **32 bytes** (`100,000` instances = `3.2 MB` per frame).

### 3.3 Single-Draw-Call Procedural Quad Instancing (`fge-render::pipeline`)
- Topology: `wgpu::PrimitiveTopology::TriangleStrip`.
- Vertex Buffer: Only 1 buffer at slot `0` with `VertexStepMode::Instance` (`SpriteInstance`).
- Quad corners are generated procedurally in `@vertex` from `@builtin(vertex_index)` (`0..4`):
  - `idx 0 -> (-0.5, -0.5)`, `idx 1 -> (0.5, -0.5)`, `idx 2 -> (-0.5, 0.5)`, `idx 3 -> (0.5, 0.5)`.
- A 1x1 white `Rgba8UnormSrgb` texture + nearest/linear sampler is bound at `@group(1)`, allowing solid colored rectangles and textured sprites to share the **exact same 1 draw call** (`pass.draw(0..4, 0..instance_count)`).

### 3.4 `Camera2D` Coordinate System (`fge-core::camera`)
- World coordinates use **2D top-left origin (`+X` right, `+Y` down)**.
- `visible_bounds()`: `half = (viewport_size * 0.5) / zoom`, `min = center - half`, `max = center + half`.
- `projection_matrix()`: `glam::Mat4::orthographic_rh(min.x, max.x, max.y, min.y, -1.0, 1.0)`, mapping `(min.x, min.y)` to WebGPU NDC `(-1.0, +1.0)` and `(max.x, max.y)` to `(+1.0, -1.0)` with `z` in `[0.0, 1.0]`.

## 4. `wgpu` 30.0.1 & `winit` 0.30.13 API Notes
- `wgpu::Instance::new(desc)` takes an **owned** `wgpu::InstanceDescriptor` (`InstanceDescriptor::new_without_display_handle_from_env()` for offscreen, `InstanceDescriptor::new_with_display_handle_from_env(Box::new(event_loop.owned_display_handle()))` for windowed).
- `adapter.request_adapter(&options).await` returns `Result<Adapter, RequestAdapterError>`.
- `adapter.request_device(&DeviceDescriptor::default()).await` takes 1 argument and returns `Result<(Device, Queue), RequestDeviceError>`.
- `surface.get_current_texture()` returns `wgpu::CurrentSurfaceTexture` (`Success(tex)`, `Suboptimal(tex)`, `Timeout`, `Occluded`, `Outdated`, `Lost`, `Validation`).
- Present via `queue.present(surface_texture)` (NOT `surface_texture.present()`).
- Offscreen synchronization uses `device.poll(wgpu::wgt::PollType::Wait { submission_index: None, timeout: None })`.

## 5. Build & Verification Guardrails (`MODEL.md`)
- **NEVER** run workspace-wide `cargo build`.
- Always pass `--offline` in sandboxed commands:
  - Compile check: `cargo check --offline -p <crate>`
  - Library unit tests: `cargo test --offline -p <crate> --lib`
- Forbidden patterns: unjustified `.clone()`, nested `Arc<Mutex<T>>`, undocumented `unsafe` without `// SAFETY:`.

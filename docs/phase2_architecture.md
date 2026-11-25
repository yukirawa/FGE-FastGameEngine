# FGE Phase 2 (Step 2) Architecture Specification: Engine Core & C-ABI (`fge.h`)

## 1. Scope & Design Principles
Phase 2 (`PROJECT.md` Step 2) builds the gameplay core and C-ABI boundary:
1. **Generational Handle (`EntityId` / `FgeEntity`)**:
   - 64-bit value struct `{ index: u32, generation: u32 }`.
   - Live generations start at `1` (`generation >= 1`), guaranteeing that zero-initialized `{ 0, 0 }` in C and `EntityId::INVALID` (`{ u32::MAX, 0 }`) are always rejected in $O(1)$.
2. **Packed SoA `EntityWorld` (`fge-core::world`)**:
   - Sparse `GenerationalArena` maps validated `EntityId` to dense SoA array index `usize`.
   - `despawn` performs $O(1)$ swap-remove across all SoA columns (`pos_x`, `pos_y`, `vel_x`, `vel_y`, `size_w`, `size_h`, `rotation`, `ang_vel`, `collider_radius`, `layer_mask`, `tag`, `hp`, `flags`, `z_order`, `uv_rect`, `color_rgba`), keeping memory 100% contiguous for SIMD kinematics and GPU instancing.
3. **Zero-Allocation CSR `SpatialHashGrid` (`fge-core::spatial`)**:
   - Flat Compressed-Sparse-Row (CSR) hash table (`cell_counts`, `cell_offsets`, `entries`, `query_stamps`) with power-of-two bucket count.
   - Rebuilt each frame in two linear passes with zero per-cell `Vec` allocations.
   - Supports deduplicated AABB/circle queries and narrowphase circle-circle collision pair detection (`CollisionPair`) filtered by `layer_mask`.
4. **Platform-Independent `CoreInputState` (`fge-core::input_core` + `fge::keymap`)**:
   - 128-bit key bitsets (`down`, `pressed`, `released`) indexed by `FgeKey` (`#[repr(u32)]`) plus mouse buttons, cursor position, and wheel delta.
   - Bridges both `winit` window events and headless/AI-injected inputs.
5. **Zero-Raw-Pointer C-ABI (`include/fge.h` & `crates/fge-abi`)**:
   - Strict compliance with `MODEL.md`: **no raw pointers cross the C-ABI boundary**.
   - All C parameters and return values are plain value types (`FgeEntity`, `FgeVec2`, `FgeColor`, `FgeUvRect`, `FgeCamera`, `FgeCollisionPair`, `uint32_t`, `float`, `bool`).
   - Bulk queries (`fge_detect_collisions`, `fge_query_circle`) store results inside Rust's `EngineContext` scratch buffers and return the result count `uint32_t`, with bounds-checked indexed getters (`fge_get_collision(i)`, `fge_get_query_entity(i)`).
   - Scoped RAII thread-local activation (`ScopedContextGuard`) binds `&mut EngineContext` only for the synchronous duration of C lifecycle callbacks (`fge_init`, `fge_update`) with zero `Arc<Mutex<T>>`.

## 2. Updated Crate Dependency Graph
```text
fge (bin + lib)
 ├── fge-abi (lib, C-ABI symbols)
 │    └── fge-core (lib)
 ├── fge-render (lib)
 │    └── fge-core (lib)
 └── fge-core (lib)
```

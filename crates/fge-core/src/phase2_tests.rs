use std::collections::HashSet;

use glam::Vec2;

use crate::camera::Aabb2D;
use crate::error::CoreError;
use crate::handle::{EntityId, GenerationalArena, SwapRemoveRecord};
use crate::input_core::{CoreInputState, FgeKey, FgeMouseButton};
use crate::sim::FULL_UV_RECT;
use crate::spatial::{CollisionPair, SpatialHashGrid};
use crate::world::{
    DEFAULT_ENTITY_FLAGS, ENTITY_FLAG_COLLIDABLE, ENTITY_FLAG_KINEMATIC, ENTITY_FLAG_VISIBLE,
    ENTITY_FLAG_Y_SORT, EntityWorld,
};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn canonical_pair(pair: CollisionPair) -> (EntityId, EntityId) {
    if pair.a <= pair.b {
        (pair.a, pair.b)
    } else {
        (pair.b, pair.a)
    }
}

// ============================================================================
// EntityId & GenerationalArena Tests
// ============================================================================

#[test]
fn entity_id_layout_and_null_sentinels() {
    assert_eq!(core::mem::size_of::<EntityId>(), 8);
    assert_eq!(EntityId::INVALID.index, u32::MAX);
    assert_eq!(EntityId::INVALID.generation, 0);
    assert!(EntityId::INVALID.is_null());

    let c_zero = EntityId {
        index: 0,
        generation: 0,
    };
    assert!(c_zero.is_null());
    assert!(EntityId::default().is_null());

    let valid = EntityId {
        index: 0,
        generation: 1,
    };
    assert!(!valid.is_null());

    let max_idx_gen1 = EntityId {
        index: u32::MAX,
        generation: 1,
    };
    assert!(max_idx_gen1.is_null());
}

#[test]
fn arena_rejects_zero_and_invalid_handles() {
    let mut arena = GenerationalArena::with_capacity(8);
    let c_zero = EntityId {
        index: 0,
        generation: 0,
    };

    assert_eq!(arena.resolve(c_zero), None);
    assert_eq!(arena.resolve(EntityId::INVALID), None);
    assert_eq!(arena.deallocate(c_zero), None);
    assert_eq!(arena.deallocate(EntityId::INVALID), None);

    // Even after allocating slot 0, a {0, 0} handle must still be rejected.
    let e0 = arena.allocate();
    assert_eq!(e0.index, 0);
    assert!(e0.generation >= 1);
    assert_eq!(arena.resolve(c_zero), None);
    assert_eq!(arena.resolve(EntityId::INVALID), None);
    assert_eq!(arena.deallocate(c_zero), None);
    assert_eq!(arena.deallocate(EntityId::INVALID), None);
    assert_eq!(arena.resolve(e0), Some(0));
}

#[test]
fn arena_allocate_deallocate_generation_reuse_and_stale_rejection() {
    let mut arena = GenerationalArena::new_for_test();
    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);

    let e0 = arena.allocate();
    let e1 = arena.allocate();
    assert!(e0.generation >= 1);
    assert!(e1.generation >= 1);
    assert_eq!(arena.len(), 2);
    assert_eq!(arena.resolve(e0), Some(0));
    assert_eq!(arena.resolve(e1), Some(1));
    assert_eq!(arena.entity_at_dense(0), Some(e0));
    assert_eq!(arena.entity_at_dense(1), Some(e1));
    assert_eq!(arena.entity_at_dense(2), None);

    // Deallocate e0: tail (e1) swaps into dense index 0.
    let rec = arena.deallocate(e0);
    assert_eq!(rec, Some(SwapRemoveRecord { removed_dense: 0 }));
    assert_eq!(arena.len(), 1);
    assert_eq!(arena.resolve(e0), None);
    assert_eq!(arena.deallocate(e0), None);
    assert_eq!(arena.resolve(e1), Some(0));
    assert_eq!(arena.entity_at_dense(0), Some(e1));

    // Reallocate reuses e0's sparse index with an incremented generation.
    let e0_reused = arena.allocate();
    assert_eq!(e0_reused.index, e0.index);
    assert!(e0_reused.generation > e0.generation);
    assert_eq!(arena.resolve(e0), None);
    assert_eq!(arena.resolve(e0_reused), Some(1));
    assert_eq!(arena.entity_at_dense(1), Some(e0_reused));
}

#[test]
fn arena_swap_remove_stress_across_1000_plus_cycles() {
    let mut arena = GenerationalArena::with_capacity(64);
    let mut rng = fastrand::Rng::with_seed(0x5EED_F6E2);
    let mut live: Vec<EntityId> = Vec::new();
    let mut retired: Vec<EntityId> = Vec::new();

    for cycle in 0..1_500 {
        let should_alloc = live.is_empty() || (live.len() < 128 && rng.u32(0..100) < 60);
        if should_alloc {
            let id = arena.allocate();
            assert!(!id.is_null());
            assert!(id.generation >= 1);
            live.push(id);
        } else {
            let pick = rng.usize(0..live.len());
            let target = live.swap_remove(pick);
            let expected_dense = arena.resolve(target).expect("live entity must resolve");
            let rec = arena
                .deallocate(target)
                .expect("live entity must deallocate");
            assert_eq!(rec.removed_dense, expected_dense);
            retired.push(target);
        }

        assert_eq!(arena.len(), live.len());

        // Periodically verify every live entity resolves to a unique dense index
        // and maps back via entity_at_dense, while retired handles stay invalid.
        if cycle % 50 == 0 || cycle == 1_499 {
            let mut seen_dense = HashSet::with_capacity(live.len());
            for &id in &live {
                let dense = arena
                    .resolve(id)
                    .expect("live entity lost resolution after swap-remove");
                assert!(dense < arena.len());
                assert!(seen_dense.insert(dense), "duplicate dense slot {dense}");
                assert_eq!(arena.entity_at_dense(dense), Some(id));
            }
            for &old_id in retired.iter().rev().take(64) {
                assert_eq!(arena.resolve(old_id), None);
            }
        }
    }
}

#[test]
fn arena_clear_invalidating_handles() {
    let mut arena = GenerationalArena::with_capacity(16);
    let handles: Vec<EntityId> = (0..10).map(|_| arena.allocate()).collect();
    assert_eq!(arena.len(), 10);

    arena.clear_invalidating_handles();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
    assert_eq!(arena.entity_at_dense(0), None);

    for &old in &handles {
        assert_eq!(arena.resolve(old), None);
        assert_eq!(arena.deallocate(old), None);
    }

    let new_handles: Vec<EntityId> = (0..10).map(|_| arena.allocate()).collect();
    assert_eq!(arena.len(), 10);
    for &new_id in &new_handles {
        assert!(new_id.generation >= 2);
        assert!(arena.resolve(new_id).is_some());
    }
    for &old in &handles {
        assert_eq!(arena.resolve(old), None);
    }
}

trait ArenaTestExt {
    fn new_for_test() -> Self;
}

impl ArenaTestExt for GenerationalArena {
    fn new_for_test() -> Self {
        Self::default()
    }
}

// ============================================================================
// EntityWorld Tests
// ============================================================================

#[test]
fn world_spawn_spawn_sprite_despawn_compaction_and_invariants() {
    let mut world = EntityWorld::with_capacity(8);
    assert!(world.is_empty());
    assert_eq!(world.len(), 0);
    assert!(world.debug_assert_invariants());

    let e0 = world.spawn();
    assert_eq!(world.len(), 1);
    assert!(world.is_alive(e0));
    assert_eq!(world.dense_index_of(e0), Some(0));
    assert_eq!(world.get_position(e0), Some([0.0, 0.0]));
    assert_eq!(world.get_size(e0), Some([16.0, 16.0]));
    assert_eq!(world.get_color(e0), Some([255, 255, 255, 255]));
    assert_eq!(world.get_flags(e0), Some(DEFAULT_ENTITY_FLAGS));
    assert_eq!(world.uv_rect[0], FULL_UV_RECT);
    assert!(world.debug_assert_invariants());

    let e1 = world.spawn_sprite([100.0, 200.0], [32.0, 48.0], [10, 20, 30, 255]);
    let e2 = world.spawn_sprite([300.0, 400.0], [64.0, 64.0], [40, 50, 60, 200]);
    assert_eq!(world.len(), 3);
    assert_eq!(world.dense_index_of(e1), Some(1));
    assert_eq!(world.dense_index_of(e2), Some(2));
    assert!(world.debug_assert_invariants());

    // Despawn middle entity e1: e2 must swap-remove into dense index 1.
    assert!(world.despawn(e1));
    assert!(!world.despawn(e1));
    assert!(!world.is_alive(e1));
    assert_eq!(world.dense_index_of(e1), None);
    assert_eq!(world.len(), 2);
    assert_eq!(world.dense_index_of(e0), Some(0));
    assert_eq!(world.dense_index_of(e2), Some(1));
    assert_eq!(world.get_position(e2), Some([300.0, 400.0]));
    assert_eq!(world.get_size(e2), Some([64.0, 64.0]));
    assert_eq!(world.get_color(e2), Some([40, 50, 60, 200]));
    assert_eq!(world.entities[1], e2);
    assert!(world.debug_assert_invariants());

    world.clear();
    assert!(world.is_empty());
    assert!(!world.is_alive(e0));
    assert!(!world.is_alive(e2));
    assert!(world.debug_assert_invariants());
}

#[test]
fn world_getters_setters_and_stale_handle_rejection() {
    let mut world = EntityWorld::new();
    let e = world.spawn_sprite([10.0, 20.0], [30.0, 40.0], [1, 2, 3, 4]);

    assert!(world.set_position(e, [11.5, 22.5]));
    assert_eq!(world.get_position(e), Some([11.5, 22.5]));

    assert!(world.set_velocity(e, [-5.0, 12.0]));
    assert_eq!(world.get_velocity(e), Some([-5.0, 12.0]));

    assert!(world.set_size(e, [64.0, 32.0]));
    assert_eq!(world.get_size(e), Some([64.0, 32.0]));

    assert!(world.set_rotation(e, 1.25));
    assert_eq!(world.get_rotation(e), Some(1.25));

    assert!(world.set_angular_velocity(e, 0.5));
    let idx = world.dense_index_of(e).unwrap();
    assert_eq!(world.ang_vel[idx], 0.5);

    assert!(world.set_color(e, [200, 150, 100, 250]));
    assert_eq!(world.get_color(e), Some([200, 150, 100, 250]));

    assert!(world.set_uv_rect(e, [100, 200, 300, 400]));
    assert_eq!(world.uv_rect[idx], [100, 200, 300, 400]);

    assert!(world.set_z_order(e, -3.5));
    assert_eq!(world.z_order[idx], -3.5);

    assert!(world.set_tag(e, 42));
    assert_eq!(world.get_tag(e), Some(42));

    assert!(world.set_hp(e, 99.5));
    assert_eq!(world.get_hp(e), Some(99.5));

    let custom_flags = ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT;
    assert!(world.set_flags(e, custom_flags));
    assert_eq!(world.get_flags(e), Some(custom_flags));

    // set_collider should set radius/mask and auto-enable ENTITY_FLAG_COLLIDABLE.
    assert!(world.set_collider(e, 18.0, 0b0010));
    assert_eq!(world.collider_radius[idx], 18.0);
    assert_eq!(world.layer_mask[idx], 0b0010);
    assert_ne!(world.get_flags(e).unwrap() & ENTITY_FLAG_COLLIDABLE, 0);

    // Despawn and verify stale handle + sentinels are rejected on every getter and setter.
    assert!(world.despawn(e));
    let c_zero = EntityId {
        index: 0,
        generation: 0,
    };

    for bad in [e, c_zero, EntityId::INVALID] {
        assert!(!world.is_alive(bad));
        assert_eq!(world.dense_index_of(bad), None);
        assert!(!world.despawn(bad));

        assert!(!world.set_position(bad, [1.0, 2.0]));
        assert_eq!(world.get_position(bad), None);

        assert!(!world.set_velocity(bad, [1.0, 2.0]));
        assert_eq!(world.get_velocity(bad), None);

        assert!(!world.set_size(bad, [1.0, 2.0]));
        assert_eq!(world.get_size(bad), None);

        assert!(!world.set_rotation(bad, 0.5));
        assert_eq!(world.get_rotation(bad), None);

        assert!(!world.set_angular_velocity(bad, 0.5));

        assert!(!world.set_color(bad, [0, 0, 0, 0]));
        assert_eq!(world.get_color(bad), None);

        assert!(!world.set_uv_rect(bad, [0, 0, 1, 1]));
        assert!(!world.set_z_order(bad, 1.0));

        assert!(!world.set_collider(bad, 10.0, 1));

        assert!(!world.set_tag(bad, 7));
        assert_eq!(world.get_tag(bad), None);

        assert!(!world.set_hp(bad, 50.0));
        assert_eq!(world.get_hp(bad), None);

        assert!(!world.set_flags(bad, 0));
        assert_eq!(world.get_flags(bad), None);
    }
}

#[test]
fn world_step_kinematics_respects_kinematic_flag_and_rejects_bad_dt() {
    let mut world = EntityWorld::new();
    let kinematic = world.spawn_sprite([10.0, 20.0], [16.0, 16.0], [255, 255, 255, 255]);
    let static_ent = world.spawn_sprite([50.0, 60.0], [16.0, 16.0], [255, 255, 255, 255]);

    assert!(world.set_velocity(kinematic, [100.0, -40.0]));
    assert!(world.set_angular_velocity(kinematic, 2.0));
    assert!(world.set_rotation(kinematic, 0.5));

    // Verify kinematic flag is set by default on kinematic, and clear it on static_ent.
    assert_ne!(
        world.get_flags(kinematic).unwrap() & ENTITY_FLAG_KINEMATIC,
        0
    );
    assert!(world.set_velocity(static_ent, [500.0, 500.0]));
    assert!(world.set_angular_velocity(static_ent, 10.0));
    assert!(world.set_rotation(static_ent, 1.0));
    assert!(world.set_flags(static_ent, ENTITY_FLAG_VISIBLE));

    // Reject negative, NaN, and infinite dt.
    for bad_dt in [-0.001, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let res = world.step_kinematics(bad_dt);
        assert!(
            matches!(res, Err(CoreError::InvalidDeltaTime { .. })),
            "expected InvalidDeltaTime for dt={bad_dt}, got {res:?}"
        );
    }

    // dt == 0.0 is valid and leaves everything unchanged.
    assert!(world.step_kinematics(0.0).is_ok());
    assert_eq!(world.get_position(kinematic), Some([10.0, 20.0]));

    // Advance by 0.5s.
    assert!(world.step_kinematics(0.5).is_ok());
    let k_pos = world.get_position(kinematic).unwrap();
    assert!(approx_eq(k_pos[0], 60.0, 1e-5));
    assert!(approx_eq(k_pos[1], 0.0, 1e-5));
    assert!(approx_eq(world.get_rotation(kinematic).unwrap(), 1.5, 1e-5));

    // Non-kinematic entity must remain completely stationary.
    assert_eq!(world.get_position(static_ent), Some([50.0, 60.0]));
    assert!(approx_eq(world.get_rotation(static_ent).unwrap(), 1.0, 1e-5));
}

// ============================================================================
// SpatialHashGrid Tests
// ============================================================================

#[test]
fn spatial_grid_validation_errors() {
    for bad_cell in [0.0, -1.0, -32.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let res = SpatialHashGrid::new(bad_cell, 1024);
        assert!(
            matches!(res, Err(CoreError::InvalidCellSize { .. })),
            "expected InvalidCellSize for {bad_cell}, got {res:?}"
        );
    }

    for bad_buckets in [0, 3, 5, 6, 12, 100, 1000] {
        let res = SpatialHashGrid::new(32.0, bad_buckets);
        assert!(
            matches!(res, Err(CoreError::InvalidBucketCount { .. })),
            "expected InvalidBucketCount for {bad_buckets}, got {res:?}"
        );
    }

    let grid = SpatialHashGrid::new(32.0, 2048).expect("valid spatial grid");
    assert_eq!(grid.cell_size(), 32.0);
    assert_eq!(grid.bucket_count(), 2048);
}

#[test]
fn spatial_grid_query_circle_and_aabb_unique_hits_and_layer_masks() {
    let mut world = EntityWorld::new();
    // Large collider spanning many 16px cells.
    let big = world.spawn_sprite([100.0, 100.0], [10.0, 10.0], [255, 255, 255, 255]);
    world.set_collider(big, 48.0, 0b0001);

    // Small collider inside query region on layer 2.
    let layer2 = world.spawn_sprite([140.0, 100.0], [10.0, 10.0], [255, 255, 255, 255]);
    world.set_collider(layer2, 8.0, 0b0010);

    // Non-collidable entity inside region (no collider set).
    let non_collidable = world.spawn_sprite([105.0, 105.0], [10.0, 10.0], [255, 255, 255, 255]);
    let _ = non_collidable;

    // Far collidable entity outside region.
    let far = world.spawn_sprite([900.0, 900.0], [10.0, 10.0], [255, 255, 255, 255]);
    world.set_collider(far, 10.0, 0b0011);

    let mut grid = SpatialHashGrid::new(16.0, 1024).unwrap();
    grid.rebuild(&world);

    // Large radius query covering multiple cells must deduplicate `big` and `layer2`.
    let mut hits = Vec::new();
    let count = grid.query_circle(&world, Vec2::new(110.0, 100.0), 80.0, 0b0011, &mut hits);
    assert_eq!(count, 2);
    assert_eq!(hits.len(), 2);
    let hit_set: HashSet<EntityId> = hits.iter().copied().collect();
    assert_eq!(hit_set.len(), 2);
    assert!(hit_set.contains(&big));
    assert!(hit_set.contains(&layer2));
    assert!(!hit_set.contains(&far));

    // Layer mask filtering for query_circle.
    hits.clear();
    let count_l2 = grid.query_circle(&world, Vec2::new(110.0, 100.0), 80.0, 0b0010, &mut hits);
    assert_eq!(count_l2, 1);
    assert_eq!(hits, vec![layer2]);

    // query_aabb_dense spanning many cells returns unique dense indices and respects layer_mask.
    let mut dense_hits = Vec::new();
    grid.query_aabb_dense(
        &world,
        Aabb2D::new(Vec2::new(40.0, 40.0), Vec2::new(180.0, 180.0)),
        0b0011,
        &mut dense_hits,
    );
    assert_eq!(dense_hits.len(), 2);
    let dense_set: HashSet<u32> = dense_hits.iter().copied().collect();
    assert!(dense_set.contains(&(world.dense_index_of(big).unwrap() as u32)));
    assert!(dense_set.contains(&(world.dense_index_of(layer2).unwrap() as u32)));

    dense_hits.clear();
    grid.query_aabb_dense(
        &world,
        Aabb2D::new(Vec2::new(40.0, 40.0), Vec2::new(180.0, 180.0)),
        0b0001,
        &mut dense_hits,
    );
    assert_eq!(dense_hits, vec![world.dense_index_of(big).unwrap() as u32]);
}

#[test]
fn spatial_grid_detect_collisions_deduplicates_pairs_and_respects_masks() {
    let mut world = EntityWorld::new();
    // Large overlapping pair spanning multiple cells.
    let a = world.spawn_sprite([100.0, 100.0], [20.0, 20.0], [255, 255, 255, 255]);
    let b = world.spawn_sprite([120.0, 100.0], [20.0, 20.0], [255, 255, 255, 255]);
    let c = world.spawn_sprite([100.0, 120.0], [20.0, 20.0], [255, 255, 255, 255]);
    // Separated entity on same layer.
    let d = world.spawn_sprite([400.0, 400.0], [20.0, 20.0], [255, 255, 255, 255]);

    world.set_collider(a, 24.0, 0b0001);
    world.set_collider(b, 24.0, 0b0001);
    world.set_collider(c, 24.0, 0b0010);
    world.set_collider(d, 24.0, 0b0001);

    let mut grid = SpatialHashGrid::new(16.0, 1024).unwrap();
    let mut pairs = Vec::new();

    // Same mask: (a, b) only, no duplicate (b, a) and no self-collisions.
    let n_same = grid.detect_collisions(&world, 0b0001, 0b0001, &mut pairs);
    assert_eq!(n_same, 1);
    assert_eq!(pairs.len(), 1);
    assert_eq!(canonical_pair(pairs[0]), canonical_pair(CollisionPair { a, b }));

    // Disjoint masks (layer 1 vs layer 2): (a, c) and (b, c), not (a, b).
    pairs.clear();
    let n_cross = grid.detect_collisions(&world, 0b0001, 0b0010, &mut pairs);
    assert_eq!(n_cross, 2);
    assert_eq!(pairs.len(), 2);
    let pair_set: HashSet<(EntityId, EntityId)> = pairs.iter().copied().map(canonical_pair).collect();
    assert_eq!(pair_set.len(), 2);
    assert!(pair_set.contains(&canonical_pair(CollisionPair { a, b: c })));
    assert!(pair_set.contains(&canonical_pair(CollisionPair { a: b, b: c })));

    // Overlapping all-layers mask: (a, b), (a, c), (b, c) uniquely.
    pairs.clear();
    let n_all = grid.detect_collisions(&world, 0b0011, 0b0011, &mut pairs);
    assert_eq!(n_all, 3);
    assert_eq!(pairs.len(), 3);
    for p in &pairs {
        assert_ne!(p.a, p.b, "self-collision detected");
    }
    let all_set: HashSet<(EntityId, EntityId)> = pairs.iter().copied().map(canonical_pair).collect();
    assert_eq!(all_set.len(), 3);
}

#[test]
fn spatial_grid_detect_collisions_scales_to_5000_plus_entities() {
    const PAIR_COUNT: usize = 2_600; // 5,200 entities total
    let mut world = EntityWorld::with_capacity(PAIR_COUNT * 2);
    let mut expected = HashSet::with_capacity(PAIR_COUNT);

    for k in 0..PAIR_COUNT {
        let x = (k % 100) as f32 * 64.0;
        let y = (k / 100) as f32 * 64.0;

        let ea = world.spawn_sprite([x, y], [12.0, 12.0], [255, 0, 0, 255]);
        let eb = world.spawn_sprite([x + 8.0, y], [12.0, 12.0], [0, 255, 0, 255]);
        world.set_collider(ea, 6.0, 1);
        world.set_collider(eb, 6.0, 1);
        expected.insert(canonical_pair(CollisionPair { a: ea, b: eb }));
    }

    assert_eq!(world.len(), 5_200);

    let mut grid = SpatialHashGrid::new(32.0, 16_384).unwrap();
    let mut pairs = Vec::with_capacity(PAIR_COUNT);
    let count = grid.detect_collisions(&world, 1, 1, &mut pairs);

    assert_eq!(count, PAIR_COUNT);
    assert_eq!(pairs.len(), PAIR_COUNT);

    let actual: HashSet<(EntityId, EntityId)> = pairs.iter().copied().map(canonical_pair).collect();
    assert_eq!(actual, expected);
}

// ============================================================================
// CoreInputState, FgeKey & FgeMouseButton Tests
// ============================================================================

#[test]
fn fge_key_and_mouse_button_from_u32() {
    for code in 0..(FgeKey::COUNT as u32) {
        let key = FgeKey::from_u32(code).expect("valid key code should map to FgeKey");
        assert_eq!(key as u32, code);
    }
    assert_eq!(FgeKey::from_u32(0), Some(FgeKey::Unknown));
    assert_eq!(FgeKey::from_u32(31), Some(FgeKey::Tab));
    assert_eq!(FgeKey::from_u32(32), None);
    assert_eq!(FgeKey::from_u32(999), None);
    assert_eq!(FgeKey::from_u32(u32::MAX), None);

    assert_eq!(FgeMouseButton::from_u32(0), Some(FgeMouseButton::Left));
    assert_eq!(FgeMouseButton::from_u32(1), Some(FgeMouseButton::Right));
    assert_eq!(FgeMouseButton::from_u32(2), Some(FgeMouseButton::Middle));
    assert_eq!(FgeMouseButton::from_u32(3), None);
    assert_eq!(FgeMouseButton::from_u32(u32::MAX), None);
}

#[test]
fn core_input_state_key_edges_repeat_mouse_and_wheel_transitions() {
    let mut input = CoreInputState::new();
    assert_eq!(input.down_bits, 0);
    assert_eq!(input.pressed_bits, 0);
    assert_eq!(input.released_bits, 0);
    assert_eq!(input.mouse_down_bits, 0);
    assert_eq!(input.mouse_pos, [0.0, 0.0]);
    assert_eq!(input.mouse_wheel, [0.0, 0.0]);

    // Frame 1: press W and Space.
    input.set_key_state(FgeKey::W, true);
    input.set_key_state(FgeKey::Space, true);
    assert!(input.is_key_down(FgeKey::W));
    assert!(input.is_key_pressed(FgeKey::W));
    assert!(!input.is_key_released(FgeKey::W));
    assert!(input.is_key_down(FgeKey::Space));
    assert!(input.is_key_pressed(FgeKey::Space));

    // Repeat key-down on already-held key in same frame or next frame must not re-trigger pressed.
    input.begin_frame();
    assert!(input.is_key_down(FgeKey::W));
    assert!(!input.is_key_pressed(FgeKey::W));
    assert!(!input.is_key_released(FgeKey::W));

    input.set_key_state(FgeKey::W, true);
    assert!(input.is_key_down(FgeKey::W));
    assert!(!input.is_key_pressed(FgeKey::W));

    // Release W, keep Space held.
    input.set_key_state(FgeKey::W, false);
    assert!(!input.is_key_down(FgeKey::W));
    assert!(!input.is_key_pressed(FgeKey::W));
    assert!(input.is_key_released(FgeKey::W));
    assert!(input.is_key_down(FgeKey::Space));

    // Next frame clears released edge.
    input.begin_frame();
    assert!(!input.is_key_down(FgeKey::W));
    assert!(!input.is_key_pressed(FgeKey::W));
    assert!(!input.is_key_released(FgeKey::W));
    assert!(input.is_key_down(FgeKey::Space));

    // Mouse buttons, position, and wheel accumulation + reset.
    input.set_mouse_button(FgeMouseButton::Left, true);
    input.set_mouse_button(FgeMouseButton::Right, true);
    assert!(input.is_mouse_down(FgeMouseButton::Left));
    assert!(input.is_mouse_down(FgeMouseButton::Right));
    assert!(!input.is_mouse_down(FgeMouseButton::Middle));

    input.set_mouse_button(FgeMouseButton::Left, false);
    assert!(!input.is_mouse_down(FgeMouseButton::Left));
    assert!(input.is_mouse_down(FgeMouseButton::Right));

    input.set_mouse_pos(320.5, 240.25);
    assert_eq!(input.mouse_pos, [320.5, 240.25]);

    input.add_mouse_wheel(0.5, -1.0);
    input.add_mouse_wheel(1.5, 2.5);
    assert_eq!(input.mouse_wheel, [2.0, 1.5]);

    // begin_frame clears mouse_wheel while preserving mouse_pos and held mouse buttons.
    input.begin_frame();
    assert_eq!(input.mouse_wheel, [0.0, 0.0]);
    assert_eq!(input.mouse_pos, [320.5, 240.25]);
    assert!(input.is_mouse_down(FgeMouseButton::Right));
}

// Phase 2 unit tests for `fge-render::world_batch`.

use core::f32::consts::FRAC_PI_4;

use fge_core::{
    camera::Camera2D,
    sim::FULL_UV_RECT,
    world::{EntityWorld, DEFAULT_ENTITY_FLAGS, ENTITY_FLAG_VISIBLE, ENTITY_FLAG_Y_SORT},
};

use crate::{
    context::GpuRenderer,
    error::RenderError,
    instance::SpriteInstance,
    world_batch::{pack_world_instances, pack_world_instances_with_mode, DepthSortMode},
};

#[test]
fn pack_world_instances_empty_world_clears_out_and_preserves_capacity() {
    let world = EntityWorld::new();
    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");

    let mut out = Vec::with_capacity(16);
    out.push(SpriteInstance {
        pos: [1.0, 2.0],
        size: [3.0, 4.0],
        uv_rect: FULL_UV_RECT,
        rotation: 0.5,
        color: [10, 20, 30, 40],
    });
    let cap_before = out.capacity();
    let ptr_before = out.as_ptr();

    assert_eq!(pack_world_instances(&world, None, &mut out), 0);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), cap_before);
    assert_eq!(out.as_ptr(), ptr_before);

    assert_eq!(pack_world_instances(&world, Some(&camera), &mut out), 0);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), cap_before);
}

#[test]
fn pack_world_instances_skips_invisible_entities() {
    let mut world = EntityWorld::with_capacity(4);

    let e_visible_a = world.spawn_sprite([100.0, 200.0], [32.0, 48.0], [255, 0, 0, 255]);
    assert!(world.set_rotation(e_visible_a, 0.25));
    assert!(world.set_uv_rect(e_visible_a, [10, 20, 30, 40]));

    let e_invisible = world.spawn_sprite([150.0, 250.0], [64.0, 64.0], [0, 255, 0, 255]);
    assert!(world.set_flags(e_invisible, DEFAULT_ENTITY_FLAGS & !ENTITY_FLAG_VISIBLE));

    let e_zero_flags = world.spawn_sprite([300.0, 400.0], [16.0, 16.0], [255, 255, 0, 255]);
    assert!(world.set_flags(e_zero_flags, 0));

    let e_visible_b = world.spawn_sprite([500.0, 600.0], [24.0, 12.0], [0, 0, 255, 200]);
    assert!(world.set_flags(e_visible_b, ENTITY_FLAG_VISIBLE));

    let mut out = Vec::new();
    let count = pack_world_instances(&world, None, &mut out);
    assert_eq!(count, 2);
    assert_eq!(out.len(), 2);

    assert_eq!(
        out[0],
        SpriteInstance {
            pos: [100.0, 200.0],
            size: [32.0, 48.0],
            uv_rect: [10, 20, 30, 40],
            rotation: 0.25,
            color: [255, 0, 0, 255],
        }
    );
    assert_eq!(
        out[1],
        SpriteInstance {
            pos: [500.0, 600.0],
            size: [24.0, 12.0],
            uv_rect: FULL_UV_RECT,
            rotation: 0.0,
            color: [0, 0, 255, 200],
        }
    );
}

#[test]
fn pack_world_instances_frustum_culling_skips_offscreen_and_keeps_visible() {
    // Visible bounds: [0, 1280] x [0, 720].
    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");
    let mut world = EntityWorld::with_capacity(6);

    // 1. Center inside viewport -> kept.
    world.spawn_sprite([640.0, 360.0], [20.0, 20.0], [1, 0, 0, 255]);

    // 2. Far offscreen right -> culled.
    world.spawn_sprite([2000.0, 360.0], [20.0, 20.0], [2, 0, 0, 255]);

    // 3. Unrotated just outside left edge (half_x = 10 -> max_x = -2 < 0) -> culled.
    world.spawn_sprite([-12.0, 360.0], [20.0, 20.0], [3, 0, 0, 255]);

    // 4. Rotated at (-12, 360): bounding radius sqrt(10^2 + 10^2) ~= 14.14 > 12 -> kept.
    let e_rotated = world.spawn_sprite([-12.0, 360.0], [20.0, 20.0], [4, 0, 0, 255]);
    assert!(world.set_rotation(e_rotated, FRAC_PI_4));

    // 5. Overlapping right boundary ([1275, 1295] intersects [0, 1280]) -> kept.
    world.spawn_sprite([1285.0, 360.0], [20.0, 20.0], [5, 0, 0, 255]);

    // 6. Inside viewport but invisible -> skipped.
    let e_hidden = world.spawn_sprite([200.0, 200.0], [20.0, 20.0], [6, 0, 0, 255]);
    assert!(world.set_flags(e_hidden, 0));

    let mut out = Vec::new();
    let count = pack_world_instances(&world, Some(&camera), &mut out);
    assert_eq!(count, 3);

    let colors: Vec<[u8; 4]> = out.iter().map(|inst| inst.color).collect();
    assert_eq!(colors, vec![[1, 0, 0, 255], [4, 0, 0, 255], [5, 0, 0, 255]]);
}

#[test]
fn pack_world_instances_sorts_by_z_order_and_y_sort_within_same_layer() {
    let mut world = EntityWorld::with_capacity(6);

    // Layer z = 2.0 (highest, drawn last even with small y).
    let z2 = world.spawn_sprite([10.0, 10.0], [16.0, 16.0], [20, 0, 0, 255]);
    assert!(world.set_z_order(z2, 2.0));
    assert!(world.set_flags(z2, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    // Layer z = 1.0 with Y-sort: spawn out of order (y = 300, 100, 200).
    let z1_bottom = world.spawn_sprite([20.0, 300.0], [16.0, 16.0], [13, 0, 0, 255]);
    assert!(world.set_z_order(z1_bottom, 1.0));
    assert!(world.set_flags(z1_bottom, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    let z1_top = world.spawn_sprite([20.0, 100.0], [16.0, 16.0], [11, 0, 0, 255]);
    assert!(world.set_z_order(z1_top, 1.0));
    assert!(world.set_flags(z1_top, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    let z1_mid = world.spawn_sprite([20.0, 200.0], [16.0, 16.0], [12, 0, 0, 255]);
    assert!(world.set_z_order(z1_mid, 1.0));
    assert!(world.set_flags(z1_mid, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    // Layer z = -5.0 (lowest, drawn first even with large y).
    let z_neg = world.spawn_sprite([30.0, 900.0], [16.0, 16.0], [1, 0, 0, 255]);
    assert!(world.set_z_order(z_neg, -5.0));

    // Layer z = 0.0 (drawn after -5.0 and before 1.0).
    let z0 = world.spawn_sprite([40.0, 500.0], [16.0, 16.0], [5, 0, 0, 255]);
    assert!(world.set_z_order(z0, 0.0));

    // Invisible entity with very low z_order should still be omitted.
    let hidden_z = world.spawn_sprite([0.0, 0.0], [16.0, 16.0], [99, 0, 0, 255]);
    assert!(world.set_z_order(hidden_z, -100.0));
    assert!(world.set_flags(hidden_z, 0));

    let mut out = Vec::new();
    let count = pack_world_instances(&world, None, &mut out);
    assert_eq!(count, 6);

    let ordered_colors: Vec<[u8; 4]> = out.iter().map(|inst| inst.color).collect();
    assert_eq!(
        ordered_colors,
        vec![
            [1, 0, 0, 255],  // z = -5.0
            [5, 0, 0, 255],  // z = 0.0
            [11, 0, 0, 255], // z = 1.0, y = 100
            [12, 0, 0, 255], // z = 1.0, y = 200
            [13, 0, 0, 255], // z = 1.0, y = 300
            [20, 0, 0, 255], // z = 2.0
        ]
    );
}

#[test]
fn pack_world_instances_composite_layer_y_at_huge_open_world_coordinates() {
    let mut world = EntityWorld::with_capacity(4);

    // Layer z = 10.0 with huge open-world Y coordinates (500_000.0 vs 500_005.0).
    let y_high = world.spawn_sprite([100.0, 500_005.0], [16.0, 16.0], [10, 2, 0, 255]);
    assert!(world.set_z_order(y_high, 10.0));
    assert!(world.set_flags(y_high, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    let y_low = world.spawn_sprite([100.0, 500_000.0], [16.0, 16.0], [10, 1, 0, 255]);
    assert!(world.set_z_order(y_low, 10.0));
    assert!(world.set_flags(y_low, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    // Higher layer z = 11.0 at small Y must still sort AFTER z = 10.0 (would invert in FastFloat).
    let upper_layer = world.spawn_sprite([100.0, 10.0], [16.0, 16.0], [11, 0, 0, 255]);
    assert!(world.set_z_order(upper_layer, 11.0));
    assert!(world.set_flags(upper_layer, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    // Lower layer z = 9.0 at huge Y must still sort BEFORE z = 10.0.
    let lower_layer = world.spawn_sprite([100.0, 900_000.0], [16.0, 16.0], [9, 0, 0, 255]);
    assert!(world.set_z_order(lower_layer, 9.0));
    assert!(world.set_flags(lower_layer, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));

    let mut out = Vec::new();
    let count = pack_world_instances_with_mode(&world, None, DepthSortMode::CompositeLayerY, &mut out);
    assert_eq!(count, 4);

    let ordered_colors: Vec<[u8; 4]> = out.iter().map(|inst| inst.color).collect();
    assert_eq!(
        ordered_colors,
        vec![
            [9, 0, 0, 255],  // z = 9.0, y = 900_000.0
            [10, 1, 0, 255], // z = 10.0, y = 500_000.0
            [10, 2, 0, 255], // z = 10.0, y = 500_005.0
            [11, 0, 0, 255], // z = 11.0, y = 10.0
        ]
    );
}

#[test]
fn offscreen_gpu_render_world_smoke_test_1000_sprites() {
    let mut renderer = match GpuRenderer::new_offscreen(1280, 720, 1_000) {
        Ok(renderer) => renderer,
        Err(RenderError::RequestAdapter(err)) => {
            log::warn!("Skipping GPU render_world smoke test (no adapter available): {err}");
            return;
        }
        Err(err) => panic!("unexpected error initializing offscreen renderer: {err}"),
    };

    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");
    let mut world = EntityWorld::with_capacity(1_000);

    for i in 0..1_000u32 {
        let x = ((i * 37) % 1200) as f32 + 40.0;
        let y = ((i * 53) % 640) as f32 + 40.0;
        let id = world.spawn_sprite([x, y], [16.0, 16.0], [(i % 255) as u8, 128, 255, 255]);
        assert!(world.set_z_order(id, (i % 5) as f32));
        if i % 2 == 0 {
            assert!(world.set_flags(id, ENTITY_FLAG_VISIBLE | ENTITY_FLAG_Y_SORT));
        }
    }

    let stats_uncut = renderer
        .render_world(&camera, &world, false, wgpu::Color::BLACK)
        .expect("offscreen render_world (uncut) should succeed");
    assert_eq!(stats_uncut.draw_calls, 1);
    assert_eq!(stats_uncut.submitted_instances, 1_000);
    assert!(!stats_uncut.skipped);

    let stats_culled = renderer
        .render_world(&camera, &world, true, wgpu::Color::BLACK)
        .expect("offscreen render_world (culled) should succeed");
    assert_eq!(stats_culled.draw_calls, 1);
    assert_eq!(stats_culled.submitted_instances, 1_000);
    assert!(!stats_culled.skipped);
}

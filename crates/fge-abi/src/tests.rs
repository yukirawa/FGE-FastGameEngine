// Phase 2 unit tests for `fge-abi` and `include/fge.h`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::process::Command;

use fge_core::handle::EntityId;
use fge_core::input_core::{FgeKey, FgeMouseButton};
use fge_core::world::{ENTITY_FLAG_COLLIDABLE, ENTITY_FLAG_KINEMATIC, ENTITY_FLAG_VISIBLE};

use crate::api::*;
use crate::context::{with_active_context, EngineContext};
use crate::types::{FgeCamera, FgeColor, FgeEntity, FgeUvRect, FgeVec2};

fn new_test_ctx() -> EngineContext {
    EngineContext::new(800.0, 600.0, 0x1234_5678).expect("EngineContext::new should succeed")
}

#[test]
fn abi_calls_outside_active_context_return_safe_defaults() {
    let zero_handle = FgeEntity {
        index: 0,
        generation: 0,
    };
    let invalid_handle = EntityId::INVALID;
    let color = FgeColor {
        r: 255,
        g: 128,
        b: 64,
        a: 255,
    };
    let uv = FgeUvRect {
        u0: 0,
        v0: 0,
        u1: 32768,
        v1: 32768,
    };

    assert_eq!(fge_delta_time(), 0.0);
    assert_eq!(fge_frame_count(), 0);
    assert_eq!(fge_entity_count(), 0);
    fge_request_quit();
    assert_eq!(fge_rand_f32(), 0.0);
    assert_eq!(fge_rand_range(10.0, 20.0), 0.0);

    assert!(fge_entity_spawn().is_null());
    assert!(fge_spawn_sprite(10.0, 20.0, 32.0, 32.0, color).is_null());
    assert!(!fge_entity_despawn(zero_handle));
    assert!(!fge_entity_despawn(invalid_handle));
    assert!(!fge_entity_is_alive(zero_handle));
    assert!(!fge_entity_is_alive(invalid_handle));

    assert!(!fge_set_position(zero_handle, 5.0, 6.0));
    assert_eq!(fge_get_position(zero_handle), FgeVec2::default());
    assert!(!fge_set_velocity(invalid_handle, 1.0, 2.0));
    assert_eq!(fge_get_velocity(invalid_handle), FgeVec2::default());
    assert!(!fge_set_size(zero_handle, 16.0, 16.0));
    assert_eq!(fge_get_size(zero_handle), FgeVec2::default());
    assert!(!fge_set_rotation(zero_handle, 1.0));
    assert_eq!(fge_get_rotation(zero_handle), 0.0);
    assert!(!fge_set_angular_velocity(zero_handle, 2.0));

    assert!(!fge_set_color(zero_handle, color));
    assert_eq!(fge_get_color(zero_handle), FgeColor::default());
    assert!(!fge_set_uv_rect(zero_handle, uv));
    assert!(!fge_set_z_order(zero_handle, 3.5));
    assert!(!fge_set_flags(zero_handle, ENTITY_FLAG_VISIBLE));
    assert_eq!(fge_get_flags(zero_handle), 0);

    assert!(!fge_set_collider(zero_handle, 8.0, 1));
    assert!(!fge_set_tag(zero_handle, 99));
    assert_eq!(fge_get_tag(zero_handle), 0);
    assert!(!fge_set_hp(zero_handle, 50.0));
    assert_eq!(fge_get_hp(zero_handle), 0.0);

    assert_eq!(fge_detect_collisions(0xFFFF, 0xFFFF), 0);
    let pair = fge_get_collision(0);
    assert!(pair.a.is_null() && pair.b.is_null());
    assert_eq!(fge_query_circle(0.0, 0.0, 100.0, 0xFFFF), 0);
    assert!(fge_get_query_entity(0).is_null());

    assert!(!fge_camera_set(10.0, 20.0, 1.5));
    assert_eq!(fge_camera_get(), FgeCamera::default());

    assert!(!fge_key_down(FgeKey::Space as u32));
    assert!(!fge_key_pressed(FgeKey::Space as u32));
    assert!(!fge_key_released(FgeKey::Space as u32));
    assert!(!fge_mouse_down(FgeMouseButton::Left as u32));
    assert_eq!(fge_mouse_pos(), FgeVec2::default());
    assert_eq!(fge_mouse_wheel(), FgeVec2::default());
}

#[test]
fn entity_lifecycle_and_components_inside_active_context() {
    let mut ctx = new_test_ctx();
    let zero_handle = FgeEntity {
        index: 0,
        generation: 0,
    };

    with_active_context(&mut ctx, || {
        let e1 = fge_entity_spawn();
        assert!(!e1.is_null());
        assert!(e1.generation >= 1);
        assert!(fge_entity_is_alive(e1));
        assert_eq!(fge_entity_count(), 1);

        let sprite_color = FgeColor {
            r: 20,
            g: 200,
            b: 140,
            a: 255,
        };
        let e2 = fge_spawn_sprite(12.0, 34.0, 48.0, 64.0, sprite_color);
        assert!(!e2.is_null());
        assert!(fge_entity_is_alive(e2));
        assert_eq!(fge_entity_count(), 2);
        assert_eq!(fge_get_position(e2), FgeVec2 { x: 12.0, y: 34.0 });
        assert_eq!(fge_get_size(e2), FgeVec2 { x: 48.0, y: 64.0 });
        assert_eq!(fge_get_color(e2), sprite_color);

        assert!(fge_set_position(e1, 100.0, -25.0));
        assert_eq!(
            fge_get_position(e1),
            FgeVec2 {
                x: 100.0,
                y: -25.0
            }
        );

        assert!(fge_set_velocity(e1, 5.0, -3.0));
        assert_eq!(fge_get_velocity(e1), FgeVec2 { x: 5.0, y: -3.0 });

        assert!(fge_set_size(e1, 32.0, 16.0));
        assert_eq!(fge_get_size(e1), FgeVec2 { x: 32.0, y: 16.0 });

        assert!(fge_set_rotation(e1, 1.25));
        assert!((fge_get_rotation(e1) - 1.25).abs() < 1e-6);

        assert!(fge_set_angular_velocity(e1, 0.5));

        let new_color = FgeColor {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        assert!(fge_set_color(e1, new_color));
        assert_eq!(fge_get_color(e1), new_color);

        assert!(fge_set_uv_rect(
            e1,
            FgeUvRect {
                u0: 100,
                v0: 200,
                u1: 300,
                v1: 400,
            }
        ));
        assert!(fge_set_z_order(e1, 4.0));

        let custom_flags = ENTITY_FLAG_VISIBLE | ENTITY_FLAG_KINEMATIC;
        assert!(fge_set_flags(e1, custom_flags));
        assert_eq!(fge_get_flags(e1), custom_flags);

        assert!(fge_set_tag(e1, 42));
        assert_eq!(fge_get_tag(e1), 42);

        assert!(fge_set_hp(e1, 75.5));
        assert!((fge_get_hp(e1) - 75.5).abs() < 1e-6);

        let r01 = fge_rand_f32();
        assert!((0.0..1.0).contains(&r01));
        let r_range = fge_rand_range(10.0, 20.0);
        assert!((10.0..20.0).contains(&r_range));

        fge_request_quit();

        // Despawn e1 and verify stale handle rejection alongside {0, 0} and INVALID.
        assert!(fge_entity_despawn(e1));
        assert!(!fge_entity_despawn(e1));
        assert!(!fge_entity_is_alive(e1));
        assert_eq!(fge_entity_count(), 1);

        for bad in [e1, zero_handle, EntityId::INVALID] {
            assert!(!fge_entity_is_alive(bad));
            assert!(!fge_entity_despawn(bad));
            assert!(!fge_set_position(bad, 1.0, 1.0));
            assert_eq!(fge_get_position(bad), FgeVec2::default());
            assert!(!fge_set_velocity(bad, 1.0, 1.0));
            assert_eq!(fge_get_velocity(bad), FgeVec2::default());
            assert!(!fge_set_size(bad, 1.0, 1.0));
            assert_eq!(fge_get_size(bad), FgeVec2::default());
            assert!(!fge_set_rotation(bad, 1.0));
            assert_eq!(fge_get_rotation(bad), 0.0);
            assert!(!fge_set_angular_velocity(bad, 1.0));
            assert!(!fge_set_color(bad, new_color));
            assert_eq!(fge_get_color(bad), FgeColor::default());
            assert!(!fge_set_uv_rect(bad, FgeUvRect::default()));
            assert!(!fge_set_z_order(bad, 0.0));
            assert!(!fge_set_flags(bad, 1));
            assert_eq!(fge_get_flags(bad), 0);
            assert!(!fge_set_collider(bad, 5.0, 1));
            assert!(!fge_set_tag(bad, 1));
            assert_eq!(fge_get_tag(bad), 0);
            assert!(!fge_set_hp(bad, 1.0));
            assert_eq!(fge_get_hp(bad), 0.0);
        }
    });

    assert!(ctx.quit_requested);
}

#[test]
fn step_with_callback_advances_kinematics_frame_count_and_input() {
    let mut ctx = new_test_ctx();

    assert!(ctx.step_with_callback(-0.016, || {}).is_err());
    assert!(ctx.step_with_callback(f32::NAN, || {}).is_err());

    ctx.input.set_key_state(FgeKey::Space, true);
    ctx.input.set_mouse_button(FgeMouseButton::Left, true);
    ctx.input.set_mouse_pos(320.0, 240.0);
    ctx.input.add_mouse_wheel(0.0, 2.0);

    let spawned = ctx
        .step_with_callback(0.5, || {
            assert!((fge_delta_time() - 0.5).abs() < 1e-6);
            assert_eq!(fge_frame_count(), 0);
            assert!(fge_key_down(FgeKey::Space as u32));
            assert!(fge_key_pressed(FgeKey::Space as u32));
            assert!(!fge_key_released(FgeKey::Space as u32));
            assert!(fge_mouse_down(FgeMouseButton::Left as u32));
            assert_eq!(fge_mouse_pos(), FgeVec2 { x: 320.0, y: 240.0 });
            assert_eq!(fge_mouse_wheel(), FgeVec2 { x: 0.0, y: 2.0 });

            assert!(!fge_key_down(9999));
            assert!(!fge_key_pressed(9999));
            assert!(!fge_key_released(9999));
            assert!(!fge_mouse_down(99));

            let e = fge_spawn_sprite(
                10.0,
                20.0,
                16.0,
                16.0,
                FgeColor {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
            assert!(fge_set_velocity(e, 4.0, -2.0));
            assert!(fge_set_angular_velocity(e, 1.5));
            e
        })
        .expect("step 1 should succeed");

    // Frame 2: kinematics integrated (pos: (12, 19), rot: 0.75), edge bits cleared by begin_frame.
    ctx.step_with_callback(0.25, || {
        assert!((fge_delta_time() - 0.25).abs() < 1e-6);
        assert_eq!(fge_frame_count(), 1);
        assert!(fge_key_down(FgeKey::Space as u32));
        assert!(!fge_key_pressed(FgeKey::Space as u32));
        assert_eq!(fge_mouse_wheel(), FgeVec2::default());

        let pos = fge_get_position(spawned);
        assert!((pos.x - 12.0).abs() < 1e-5);
        assert!((pos.y - 19.0).abs() < 1e-5);
        assert!((fge_get_rotation(spawned) - 0.75).abs() < 1e-5);
    })
    .expect("step 2 should succeed");

    // Frame 3: release Space key and verify key_released edge.
    ctx.input.set_key_state(FgeKey::Space, false);
    ctx.input.set_mouse_button(FgeMouseButton::Left, false);
    ctx.step_with_callback(0.1, || {
        assert_eq!(fge_frame_count(), 2);
        assert!(!fge_key_down(FgeKey::Space as u32));
        assert!(!fge_key_pressed(FgeKey::Space as u32));
        assert!(fge_key_released(FgeKey::Space as u32));
        assert!(!fge_mouse_down(FgeMouseButton::Left as u32));
    })
    .expect("step 3 should succeed");
}

#[test]
fn collisions_circle_query_and_oob_indices() {
    let mut ctx = new_test_ctx();
    let white = FgeColor {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    with_active_context(&mut ctx, || {
        let a = fge_spawn_sprite(100.0, 100.0, 20.0, 20.0, white);
        let b = fge_spawn_sprite(120.0, 100.0, 20.0, 20.0, white);
        let c = fge_spawn_sprite(500.0, 500.0, 20.0, 20.0, white);

        assert!(!fge_set_collider(a, -1.0, 0x01));
        assert!(fge_set_collider(a, 15.0, 0x01));
        assert_ne!(fge_get_flags(a) & ENTITY_FLAG_COLLIDABLE, 0);
        assert!(fge_set_collider(b, 15.0, 0x02));
        assert!(fge_set_collider(c, 15.0, 0x02));

        let collision_count = fge_detect_collisions(0x01, 0x02);
        assert_eq!(collision_count, 1);

        let pair = fge_get_collision(0);
        assert_eq!(pair.a, a);
        assert_eq!(pair.b, b);

        let oob_pair = fge_get_collision(1);
        assert!(oob_pair.a.is_null() && oob_pair.b.is_null());
        let oob_max_pair = fge_get_collision(u32::MAX);
        assert!(oob_max_pair.a.is_null() && oob_max_pair.b.is_null());

        let query_count = fge_query_circle(110.0, 100.0, 20.0, 0x03);
        assert_eq!(query_count, 2);

        let hits = [fge_get_query_entity(0), fge_get_query_entity(1)];
        assert!(hits.contains(&a));
        assert!(hits.contains(&b));
        assert!(!hits.contains(&c));

        assert!(fge_get_query_entity(2).is_null());
        assert!(fge_get_query_entity(u32::MAX).is_null());
    });
}

#[test]
fn camera_set_get_and_validation() {
    let mut ctx = new_test_ctx();

    with_active_context(&mut ctx, || {
        let initial = fge_camera_get();
        assert!((initial.center_x - 400.0).abs() < 1e-6);
        assert!((initial.center_y - 300.0).abs() < 1e-6);
        assert!((initial.viewport_w - 800.0).abs() < 1e-6);
        assert!((initial.viewport_h - 600.0).abs() < 1e-6);
        assert!((initial.zoom - 1.0).abs() < 1e-6);

        assert!(fge_camera_set(120.0, -45.0, 2.0));
        let updated = fge_camera_get();
        assert!((updated.center_x - 120.0).abs() < 1e-6);
        assert!((updated.center_y - (-45.0)).abs() < 1e-6);
        assert!((updated.viewport_w - 800.0).abs() < 1e-6);
        assert!((updated.viewport_h - 600.0).abs() < 1e-6);
        assert!((updated.zoom - 2.0).abs() < 1e-6);

        assert!(!fge_camera_set(0.0, 0.0, 0.0));
        assert!(!fge_camera_set(0.0, 0.0, -1.0));
        assert!(!fge_camera_set(f32::NAN, 0.0, 1.0));
        assert_eq!(fge_camera_get(), updated);
    });
}

#[test]
fn with_active_context_restores_thread_local_on_panic() {
    let mut ctx = new_test_ctx();

    let res = catch_unwind(AssertUnwindSafe(|| {
        with_active_context(&mut ctx, || {
            let e = fge_entity_spawn();
            assert!(!e.is_null());
            panic!("intentional unwind test");
        });
    }));
    assert!(res.is_err());

    // Thread-local active context must be cleared by the RAII guard after unwind.
    assert_eq!(fge_entity_count(), 0);
    assert!(fge_entity_spawn().is_null());

    // Subsequent activation on the same context must still succeed.
    let count = with_active_context(&mut ctx, || fge_entity_count());
    assert_eq!(count, 1);
}

#[test]
fn fge_h_passes_c11_and_cpp_strict_syntax_check() {
    let header_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../include/fge.h");
    assert!(
        header_path.exists(),
        "expected header at {}",
        header_path.display()
    );
    let header_str = header_path
        .to_str()
        .expect("header path should be valid UTF-8");

    let c11_out = Command::new("gcc")
        .args([
            "-fsyntax-only",
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            header_str,
        ])
        .output()
        .expect("failed to invoke gcc for C11 syntax check");
    assert!(
        c11_out.status.success(),
        "C11 syntax check failed:\n{}",
        String::from_utf8_lossy(&c11_out.stderr)
    );

    let cpp_out = Command::new("gcc")
        .args([
            "-fsyntax-only",
            "-x",
            "c++",
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            header_str,
        ])
        .output()
        .expect("failed to invoke gcc for C++17 syntax check");
    assert!(
        cpp_out.status.success(),
        "C++17 syntax check failed:\n{}",
        String::from_utf8_lossy(&cpp_out.stderr)
    );
}

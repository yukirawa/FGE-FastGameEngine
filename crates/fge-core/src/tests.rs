use glam::{Vec2, Vec3};

use crate::camera::{Aabb2D, Camera2D};
use crate::error::CoreError;
use crate::sim::{BulkSimulation, FULL_UV_RECT, SimConfig, SpriteSoA};
use crate::stats::FrameStatsTracker;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn approx_eq_f64(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn aabb2d_new_and_from_center_half_extents() {
    let aabb = Aabb2D::new(Vec2::new(1.0, 2.0), Vec2::new(5.0, 8.0));
    assert_eq!(aabb.min, Vec2::new(1.0, 2.0));
    assert_eq!(aabb.max, Vec2::new(5.0, 8.0));

    let centered = Aabb2D::from_center_half_extents(Vec2::new(10.0, 20.0), Vec2::new(5.0, 3.0));
    assert_eq!(centered.min, Vec2::new(5.0, 17.0));
    assert_eq!(centered.max, Vec2::new(15.0, 23.0));
}

#[test]
fn aabb2d_intersects_overlap_edge_touching_and_disjoint() {
    let base = Aabb2D::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));

    // Interior overlap
    let overlapping = Aabb2D::new(Vec2::new(5.0, 5.0), Vec2::new(15.0, 15.0));
    assert!(base.intersects(&overlapping));
    assert!(overlapping.intersects(&base));

    // Containment
    let inside = Aabb2D::new(Vec2::new(2.0, 2.0), Vec2::new(8.0, 8.0));
    assert!(base.intersects(&inside));
    assert!(inside.intersects(&base));

    // Edge-touching along X
    let touch_right = Aabb2D::new(Vec2::new(10.0, 2.0), Vec2::new(15.0, 8.0));
    assert!(base.intersects(&touch_right));
    assert!(touch_right.intersects(&base));

    // Edge-touching along Y
    let touch_bottom = Aabb2D::new(Vec2::new(2.0, 10.0), Vec2::new(8.0, 15.0));
    assert!(base.intersects(&touch_bottom));
    assert!(touch_bottom.intersects(&base));

    // Corner-touching
    let touch_corner = Aabb2D::new(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0));
    assert!(base.intersects(&touch_corner));
    assert!(touch_corner.intersects(&base));

    // Disjoint in X (left & right)
    let left_disjoint = Aabb2D::new(Vec2::new(-5.0, 0.0), Vec2::new(-0.1, 10.0));
    let right_disjoint = Aabb2D::new(Vec2::new(10.1, 0.0), Vec2::new(15.0, 10.0));
    assert!(!base.intersects(&left_disjoint));
    assert!(!base.intersects(&right_disjoint));

    // Disjoint in Y (above & below)
    let above_disjoint = Aabb2D::new(Vec2::new(0.0, -5.0), Vec2::new(10.0, -0.1));
    let below_disjoint = Aabb2D::new(Vec2::new(0.0, 10.1), Vec2::new(10.0, 15.0));
    assert!(!base.intersects(&above_disjoint));
    assert!(!base.intersects(&below_disjoint));
}

#[test]
fn camera2d_validation_errors() {
    let valid = Camera2D::new(Vec2::new(100.0, 100.0), Vec2::new(800.0, 600.0), 1.0);
    assert!(valid.is_ok());

    // Invalid dimensions in Camera2D::new
    for (w, h) in [
        (0.0, 600.0),
        (800.0, 0.0),
        (-10.0, 600.0),
        (800.0, -1.0),
        (f32::NAN, 600.0),
        (800.0, f32::NAN),
        (f32::INFINITY, 600.0),
        (800.0, f32::NEG_INFINITY),
    ] {
        let res = Camera2D::new(Vec2::ZERO, Vec2::new(w, h), 1.0);
        assert!(
            matches!(res, Err(CoreError::InvalidDimensions { .. })),
            "expected InvalidDimensions for ({w}, {h}), got {res:?}"
        );

        let res_tl = Camera2D::from_top_left_viewport(w, h);
        assert!(
            matches!(res_tl, Err(CoreError::InvalidDimensions { .. })),
            "expected InvalidDimensions from_top_left_viewport for ({w}, {h}), got {res_tl:?}"
        );
    }

    // Invalid zoom in Camera2D::new
    for z in [0.0, -0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let res = Camera2D::new(Vec2::ZERO, Vec2::new(800.0, 600.0), z);
        assert!(
            matches!(res, Err(CoreError::InvalidZoom { .. })),
            "expected InvalidZoom for zoom={z}, got {res:?}"
        );
    }
}

#[test]
fn camera2d_viewport_bounds_and_set_viewport_size() {
    let mut cam = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport");
    assert_eq!(cam.center, Vec2::new(640.0, 360.0));
    assert_eq!(cam.viewport_size, Vec2::new(1280.0, 720.0));
    assert_eq!(cam.zoom, 1.0);

    let bounds_1x = cam.visible_bounds();
    assert_eq!(bounds_1x.min, Vec2::new(0.0, 0.0));
    assert_eq!(bounds_1x.max, Vec2::new(1280.0, 720.0));

    // Zoom 2.0 halves visible world extents around center
    let cam_2x = Camera2D::new(Vec2::new(640.0, 360.0), Vec2::new(1280.0, 720.0), 2.0)
        .expect("valid zoomed camera");
    let bounds_2x = cam_2x.visible_bounds();
    assert_eq!(bounds_2x.min, Vec2::new(320.0, 180.0));
    assert_eq!(bounds_2x.max, Vec2::new(960.0, 540.0));

    // set_viewport_size valid + invalid
    cam.set_viewport_size(640.0, 480.0)
        .expect("valid resize");
    assert_eq!(cam.viewport_size, Vec2::new(640.0, 480.0));
    let resized_bounds = cam.visible_bounds();
    assert_eq!(resized_bounds.min, Vec2::new(320.0, 120.0));
    assert_eq!(resized_bounds.max, Vec2::new(960.0, 600.0));

    let err = cam.set_viewport_size(0.0, 480.0);
    assert!(matches!(err, Err(CoreError::InvalidDimensions { .. })));
    assert_eq!(cam.viewport_size, Vec2::new(640.0, 480.0));
}

#[test]
fn camera2d_projection_matrix_maps_world_to_ndc() {
    let w = 1280.0;
    let h = 720.0;
    let cam = Camera2D::from_top_left_viewport(w, h).expect("valid camera");
    let proj = cam.projection_matrix();

    let top_left = proj.project_point3(Vec3::new(0.0, 0.0, 0.0));
    assert!(approx_eq(top_left.x, -1.0, 1e-5), "top_left.x={}", top_left.x);
    assert!(approx_eq(top_left.y, 1.0, 1e-5), "top_left.y={}", top_left.y);

    let bottom_right = proj.project_point3(Vec3::new(w, h, 0.0));
    assert!(
        approx_eq(bottom_right.x, 1.0, 1e-5),
        "bottom_right.x={}",
        bottom_right.x
    );
    assert!(
        approx_eq(bottom_right.y, -1.0, 1e-5),
        "bottom_right.y={}",
        bottom_right.y
    );

    let center = proj.project_point3(Vec3::new(w * 0.5, h * 0.5, 0.0));
    assert!(approx_eq(center.x, 0.0, 1e-5), "center.x={}", center.x);
    assert!(approx_eq(center.y, 0.0, 1e-5), "center.y={}", center.y);
}

#[test]
fn camera2d_is_visible_with_and_without_rotation_expansion() {
    let cam = Camera2D::from_top_left_viewport(100.0, 100.0).expect("valid camera");

    // Clearly inside
    assert!(cam.is_visible(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0), false));
    assert!(cam.is_visible(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0), true));

    // Clearly outside even when rotated
    assert!(!cam.is_visible(Vec2::new(130.0, 50.0), Vec2::new(5.0, 5.0), false));
    assert!(!cam.is_visible(Vec2::new(130.0, 50.0), Vec2::new(5.0, 5.0), true));

    // Half-extents (10, 10) has bounding radius sqrt(200) ~= 14.142.
    // At center (-12, 50):
    // - unrotated AABB is [-22, -2] x [40, 60], disjoint from [0, 100]^2 -> false
    // - rotated expanded AABB extends to -12 + 14.142 = +2.142 >= 0 -> true
    let edge_center = Vec2::new(-12.0, 50.0);
    let half_extents = Vec2::new(10.0, 10.0);
    assert!(!cam.is_visible(edge_center, half_extents, false));
    assert!(cam.is_visible(edge_center, half_extents, true));
}

#[test]
fn sim_config_validate_checks_bounds_sizes_and_speeds() {
    let valid = SimConfig::default();
    assert_eq!(valid.validate(), Ok(()));

    let zero_speed = SimConfig {
        max_speed: 0.0,
        max_angular_speed: 0.0,
        ..valid
    };
    assert_eq!(zero_speed.validate(), Ok(()));

    // Invalid world dimensions
    for (w, h) in [
        (0.0, 720.0),
        (1280.0, 0.0),
        (-100.0, 720.0),
        (f32::NAN, 720.0),
        (1280.0, f32::INFINITY),
    ] {
        let cfg = SimConfig {
            world_width: w,
            world_height: h,
            ..valid
        };
        assert!(matches!(
            cfg.validate(),
            Err(CoreError::InvalidDimensions { .. })
        ));
    }

    // Invalid size ranges
    for (min_s, max_s) in [
        (0.0, 16.0),
        (-1.0, 16.0),
        (16.0, 8.0),
        (f32::NAN, 16.0),
        (4.0, f32::INFINITY),
    ] {
        let cfg = SimConfig {
            min_size: min_s,
            max_size: max_s,
            ..valid
        };
        assert!(matches!(
            cfg.validate(),
            Err(CoreError::InvalidSizeRange { .. })
        ));
    }

    // Invalid speeds
    for (lin, ang) in [
        (-1.0, 1.0),
        (10.0, -0.1),
        (f32::NAN, 1.0),
        (10.0, f32::INFINITY),
    ] {
        let cfg = SimConfig {
            max_speed: lin,
            max_angular_speed: ang,
            ..valid
        };
        assert!(matches!(cfg.validate(), Err(CoreError::InvalidSpeed { .. })));
    }
}

#[test]
fn sprite_soa_capacity_push_clear_and_invariants() {
    let mut soa = SpriteSoA::with_capacity(16);
    assert_eq!(soa.len(), 0);
    assert!(soa.is_empty());
    assert!(soa.debug_assert_soa_invariants());
    assert!(soa.pos_x.capacity() >= 16);
    assert!(soa.pos_y.capacity() >= 16);
    assert!(soa.vel_x.capacity() >= 16);
    assert!(soa.vel_y.capacity() >= 16);
    assert!(soa.size_w.capacity() >= 16);
    assert!(soa.size_h.capacity() >= 16);
    assert!(soa.rotation.capacity() >= 16);
    assert!(soa.ang_vel.capacity() >= 16);
    assert!(soa.uv_rect.capacity() >= 16);
    assert!(soa.color_rgba.capacity() >= 16);

    soa.push(
        [10.0, 20.0],
        [1.5, -2.5],
        [8.0, 12.0],
        0.5,
        0.25,
        FULL_UV_RECT,
        [255, 128, 64, 255],
    );
    assert_eq!(soa.len(), 1);
    assert!(!soa.is_empty());
    assert!(soa.debug_assert_soa_invariants());
    assert_eq!(soa.pos_x[0], 10.0);
    assert_eq!(soa.pos_y[0], 20.0);
    assert_eq!(soa.vel_x[0], 1.5);
    assert_eq!(soa.vel_y[0], -2.5);
    assert_eq!(soa.size_w[0], 8.0);
    assert_eq!(soa.size_h[0], 12.0);
    assert_eq!(soa.rotation[0], 0.5);
    assert_eq!(soa.ang_vel[0], 0.25);
    assert_eq!(soa.uv_rect[0], FULL_UV_RECT);
    assert_eq!(soa.color_rgba[0], [255, 128, 64, 255]);

    soa.clear();
    assert_eq!(soa.len(), 0);
    assert!(soa.is_empty());
    assert!(soa.debug_assert_soa_invariants());
    assert!(soa.pos_x.capacity() >= 16);

    // Corrupting one column violates SoA invariants
    soa.pos_x.push(42.0);
    assert!(!soa.debug_assert_soa_invariants());
}

#[test]
fn sprite_soa_spawn_random_batch_is_deterministic_for_100k() {
    let cfg = SimConfig::default();
    let a = SpriteSoA::spawn_random_batch(100_000, 42, &cfg).expect("valid batch A");
    let b = SpriteSoA::spawn_random_batch(100_000, 42, &cfg).expect("valid batch B");
    let c = SpriteSoA::spawn_random_batch(100_000, 99, &cfg).expect("valid batch C");

    assert_eq!(a.len(), 100_000);
    assert!(a.debug_assert_soa_invariants());
    assert_eq!(a.pos_x, b.pos_x);
    assert_eq!(a.pos_y, b.pos_y);
    assert_eq!(a.vel_x, b.vel_x);
    assert_eq!(a.vel_y, b.vel_y);
    assert_eq!(a.size_w, b.size_w);
    assert_eq!(a.size_h, b.size_h);
    assert_eq!(a.rotation, b.rotation);
    assert_eq!(a.ang_vel, b.ang_vel);
    assert_eq!(a.uv_rect, b.uv_rect);
    assert_eq!(a.color_rgba, b.color_rgba);

    assert_ne!(a.pos_x, c.pos_x);

    // Validate bounds for sampled elements
    for i in 0..a.len() {
        assert!(a.pos_x[i] >= 0.0 && a.pos_x[i] <= cfg.world_width);
        assert!(a.pos_y[i] >= 0.0 && a.pos_y[i] <= cfg.world_height);
        assert!(a.size_w[i] >= cfg.min_size && a.size_w[i] <= cfg.max_size);
        assert!(a.size_h[i] >= cfg.min_size && a.size_h[i] <= cfg.max_size);
        assert!(a.vel_x[i].abs() <= cfg.max_speed);
        assert!(a.vel_y[i].abs() <= cfg.max_speed);
        assert!(a.ang_vel[i].abs() <= cfg.max_angular_speed);
    }

    // Invalid config returns error
    let bad_cfg = SimConfig {
        world_width: -1.0,
        ..cfg
    };
    assert!(matches!(
        SpriteSoA::spawn_random_batch(10, 42, &bad_cfg),
        Err(CoreError::InvalidDimensions { .. })
    ));
}

#[test]
fn bulk_simulation_step_motion_wall_bounce_rotation_and_errors() {
    let mut soa = SpriteSoA::with_capacity(5);
    // 0: interior linear motion + rotation
    soa.push([50.0, 50.0], [10.0, -5.0], [4.0, 4.0], 0.5, 2.0, FULL_UV_RECT, [255; 4]);
    // 1: hits right wall (x=100)
    soa.push([95.0, 50.0], [20.0, 0.0], [4.0, 4.0], 0.0, 0.0, FULL_UV_RECT, [255; 4]);
    // 2: hits left wall (x=0)
    soa.push([5.0, 50.0], [-20.0, 0.0], [4.0, 4.0], 0.0, 0.0, FULL_UV_RECT, [255; 4]);
    // 3: hits bottom wall (y=100)
    soa.push([50.0, 96.0], [0.0, 20.0], [4.0, 4.0], 0.0, 0.0, FULL_UV_RECT, [255; 4]);
    // 4: hits top wall (y=0)
    soa.push([50.0, 4.0], [0.0, -20.0], [4.0, 4.0], 0.0, 0.0, FULL_UV_RECT, [255; 4]);

    soa.step(0.5, 100.0, 100.0).expect("valid step");

    // 0: linear motion & rotation
    assert!(approx_eq(soa.pos_x[0], 55.0, 1e-5));
    assert!(approx_eq(soa.pos_y[0], 47.5, 1e-5));
    assert!(approx_eq(soa.vel_x[0], 10.0, 1e-5));
    assert!(approx_eq(soa.vel_y[0], -5.0, 1e-5));
    assert!(approx_eq(soa.rotation[0], 1.5, 1e-5));

    // 1: right wall bounce & clamp inside [0, 100]
    assert!(soa.pos_x[1] >= 0.0 && soa.pos_x[1] <= 100.0);
    assert!(soa.vel_x[1] < 0.0);
    assert!(approx_eq(soa.vel_x[1], -20.0, 1e-5));

    // 2: left wall bounce & clamp inside [0, 100]
    assert!(soa.pos_x[2] >= 0.0 && soa.pos_x[2] <= 100.0);
    assert!(soa.vel_x[2] > 0.0);
    assert!(approx_eq(soa.vel_x[2], 20.0, 1e-5));

    // 3: bottom wall bounce & clamp inside [0, 100]
    assert!(soa.pos_y[3] >= 0.0 && soa.pos_y[3] <= 100.0);
    assert!(soa.vel_y[3] < 0.0);
    assert!(approx_eq(soa.vel_y[3], -20.0, 1e-5));

    // 4: top wall bounce & clamp inside [0, 100]
    assert!(soa.pos_y[4] >= 0.0 && soa.pos_y[4] <= 100.0);
    assert!(soa.vel_y[4] > 0.0);
    assert!(approx_eq(soa.vel_y[4], 20.0, 1e-5));

    // Zero dt is valid and leaves state unchanged
    let prev_x0 = soa.pos_x[0];
    let prev_rot0 = soa.rotation[0];
    soa.step(0.0, 100.0, 100.0).expect("dt=0 is valid");
    assert_eq!(soa.pos_x[0], prev_x0);
    assert_eq!(soa.rotation[0], prev_rot0);

    // Invalid dt errors
    for bad_dt in [-0.01, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            soa.step(bad_dt, 100.0, 100.0),
            Err(CoreError::InvalidDeltaTime { .. })
        ));
    }

    // Invalid bounds errors
    for (bw, bh) in [(0.0, 100.0), (100.0, -5.0), (f32::NAN, 100.0), (100.0, f32::INFINITY)] {
        assert!(matches!(
            soa.step(0.016, bw, bh),
            Err(CoreError::InvalidDimensions { .. })
        ));
    }
}

#[test]
fn frame_stats_tracker_ignores_invalid_computes_rolling_fps_and_resets() {
    let mut tracker = FrameStatsTracker::with_capacity(8);
    assert_eq!(tracker.frame_count(), 0);
    assert!(tracker.samples_ms().is_empty());
    assert_eq!(tracker.rolling_fps(60), 0.0);

    // Non-finite or negative values must be ignored
    tracker.record_frame_ms(-1.0);
    tracker.record_frame_ms(f64::NAN);
    tracker.record_frame_ms(f64::INFINITY);
    tracker.record_frame_ms(f64::NEG_INFINITY);
    assert_eq!(tracker.frame_count(), 0);

    // Zero frame time produces 0.0 rolling FPS (avoid division by zero)
    tracker.record_frame_ms(0.0);
    assert_eq!(tracker.frame_count(), 1);
    assert_eq!(tracker.rolling_fps(1), 0.0);
    tracker.reset();
    assert_eq!(tracker.frame_count(), 0);
    assert!(tracker.samples_ms().is_empty());

    // Record valid samples: [10.0, 20.0, 10.0, 10.0]
    tracker.record_frame_ms(10.0);
    tracker.record_frame_ms(20.0);
    tracker.record_frame_ms(10.0);
    tracker.record_frame_ms(10.0);
    assert_eq!(tracker.frame_count(), 4);
    assert_eq!(tracker.samples_ms(), &[10.0, 20.0, 10.0, 10.0]);

    // Window == 0 -> 0.0
    assert_eq!(tracker.rolling_fps(0), 0.0);

    // Last 2 frames: 10 + 10 = 20 ms -> 2 / 0.02 = 100.0 fps
    assert!(approx_eq_f64(tracker.rolling_fps(2), 100.0, 1e-6));

    // Window larger than recorded count: 4 frames in 50 ms -> 80.0 fps
    assert!(approx_eq_f64(tracker.rolling_fps(60), 80.0, 1e-6));

    tracker.reset();
    assert_eq!(tracker.frame_count(), 0);
    assert!(tracker.samples_ms().is_empty());
    assert_eq!(tracker.rolling_fps(60), 0.0);
}

#[test]
fn frame_stats_tracker_summarize_empty_and_populated() {
    let empty = FrameStatsTracker::with_capacity(4);
    let empty_summary = empty.summarize(100_000, 100_000, 1, "Vulkan", "TestGPU", true);
    assert_eq!(empty_summary.total_frames, 0);
    assert_eq!(empty_summary.sprite_count, 100_000);
    assert_eq!(empty_summary.rendered_instances, 100_000);
    assert_eq!(empty_summary.draw_calls_per_frame, 1);
    assert_eq!(empty_summary.total_elapsed_ms, 0.0);
    assert_eq!(empty_summary.avg_frame_ms, 0.0);
    assert_eq!(empty_summary.min_frame_ms, 0.0);
    assert_eq!(empty_summary.max_frame_ms, 0.0);
    assert_eq!(empty_summary.p99_frame_ms, 0.0);
    assert_eq!(empty_summary.avg_fps, 0.0);
    assert!(!empty_summary.target_60fps_met);
    assert_eq!(empty_summary.backend_name, "Vulkan");
    assert_eq!(empty_summary.adapter_name, "TestGPU");
    assert!(empty_summary.offscreen);

    // Populated fast run meeting 60 FPS target (avg 10.0 ms = 100.0 FPS)
    let mut fast = FrameStatsTracker::with_capacity(100);
    for i in 0..100 {
        // 98 frames at 10.0 ms, 1 at 5.0 ms, 1 at 15.0 ms -> sum = 1000.0 ms, avg = 10.0 ms
        let ms = match i {
            0 => 5.0,
            99 => 15.0,
            _ => 10.0,
        };
        fast.record_frame_ms(ms);
    }
    let fast_summary = fast.summarize(100_000, 95_000, 1, "Metal", "FastAdapter", false);
    assert_eq!(fast_summary.total_frames, 100);
    assert_eq!(fast_summary.sprite_count, 100_000);
    assert_eq!(fast_summary.rendered_instances, 95_000);
    assert_eq!(fast_summary.draw_calls_per_frame, 1);
    assert!(approx_eq_f64(fast_summary.total_elapsed_ms, 1000.0, 1e-6));
    assert!(approx_eq_f64(fast_summary.avg_frame_ms, 10.0, 1e-6));
    assert!(approx_eq_f64(fast_summary.min_frame_ms, 5.0, 1e-6));
    assert!(approx_eq_f64(fast_summary.max_frame_ms, 15.0, 1e-6));
    assert!(fast_summary.p99_frame_ms >= 10.0 && fast_summary.p99_frame_ms <= 15.0);
    assert!(approx_eq_f64(fast_summary.avg_fps, 100.0, 1e-6));
    assert!(fast_summary.target_60fps_met);
    assert!(!fast_summary.offscreen);

    // Populated slow run missing 60 FPS target (avg 25.0 ms = 40.0 FPS)
    let mut slow = FrameStatsTracker::with_capacity(4);
    for ms in [20.0, 25.0, 25.0, 30.0] {
        slow.record_frame_ms(ms);
    }
    let slow_summary = slow.summarize(10, 10, 1, "Gl", "SlowAdapter", true);
    assert_eq!(slow_summary.total_frames, 4);
    assert!(approx_eq_f64(slow_summary.total_elapsed_ms, 100.0, 1e-6));
    assert!(approx_eq_f64(slow_summary.avg_frame_ms, 25.0, 1e-6));
    assert!(approx_eq_f64(slow_summary.min_frame_ms, 20.0, 1e-6));
    assert!(approx_eq_f64(slow_summary.max_frame_ms, 30.0, 1e-6));
    assert!(slow_summary.p99_frame_ms >= 25.0 && slow_summary.p99_frame_ms <= 30.0);
    assert!(approx_eq_f64(slow_summary.avg_fps, 40.0, 1e-6));
    assert!(!slow_summary.target_60fps_met);
}

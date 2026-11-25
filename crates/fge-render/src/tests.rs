use core::mem::{align_of, offset_of, size_of};

use bytemuck::Zeroable;
use fge_core::{
    camera::Camera2D,
    sim::{BulkSimulation, SimConfig, SpriteSoA, FULL_UV_RECT},
};
use glam::Vec2;

use crate::{
    context::GpuRenderer,
    error::RenderError,
    instance::{pack_soa_instances, CameraUniform, SpriteInstance},
};

#[test]
fn sprite_instance_memory_layout_and_bytemuck_roundtrip() {
    assert_eq!(size_of::<SpriteInstance>(), 32);
    assert_eq!(align_of::<SpriteInstance>(), 4);

    assert_eq!(offset_of!(SpriteInstance, pos), 0);
    assert_eq!(offset_of!(SpriteInstance, size), 8);
    assert_eq!(offset_of!(SpriteInstance, uv_rect), 16);
    assert_eq!(offset_of!(SpriteInstance, rotation), 24);
    assert_eq!(offset_of!(SpriteInstance, color), 28);

    let instance = SpriteInstance {
        pos: [123.5, -456.25],
        size: [32.0, 64.0],
        uv_rect: [0, 16_384, 32_768, u16::MAX],
        rotation: 1.25,
        color: [255, 128, 64, 200],
    };

    let bytes = bytemuck::bytes_of(&instance);
    assert_eq!(bytes.len(), 32);

    let decoded: &SpriteInstance = bytemuck::from_bytes(bytes);
    assert_eq!(*decoded, instance);

    let zeroed = SpriteInstance::zeroed();
    assert_eq!(bytemuck::bytes_of(&zeroed), &[0u8; 32]);

    let slice = [instance, zeroed];
    let slice_bytes: &[u8] = bytemuck::cast_slice(&slice);
    assert_eq!(slice_bytes.len(), 64);
}

#[test]
fn sprite_instance_buffer_layout_matches_shader_contract() {
    let layout = SpriteInstance::buffer_layout();
    assert_eq!(layout.array_stride, 32);
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    assert_eq!(layout.attributes.len(), 5);

    let expected = [
        (0, wgpu::VertexFormat::Float32x2, 0),
        (1, wgpu::VertexFormat::Float32x2, 8),
        (2, wgpu::VertexFormat::Unorm16x4, 16),
        (3, wgpu::VertexFormat::Float32, 24),
        (4, wgpu::VertexFormat::Unorm8x4, 28),
    ];

    for (attr, (location, format, offset)) in layout.attributes.iter().zip(expected) {
        assert_eq!(attr.shader_location, location);
        assert_eq!(attr.format, format);
        assert_eq!(attr.offset, offset);
    }
}

#[test]
fn camera_uniform_layout_and_from_camera() {
    assert_eq!(size_of::<CameraUniform>(), 64);
    assert_eq!(align_of::<CameraUniform>(), 4);

    let top_left_cam =
        Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");
    let uniform = CameraUniform::from_camera(&top_left_cam);
    assert_eq!(
        uniform.view_proj,
        top_left_cam.projection_matrix().to_cols_array_2d()
    );
    assert_eq!(bytemuck::bytes_of(&uniform).len(), 64);

    let zoomed_cam = Camera2D::new(
        Vec2::new(300.0, 200.0),
        Vec2::new(800.0, 600.0),
        2.0,
    )
    .expect("valid zoomed camera");
    let zoomed_uniform = CameraUniform::from_camera(&zoomed_cam);
    assert_eq!(
        zoomed_uniform.view_proj,
        zoomed_cam.projection_matrix().to_cols_array_2d()
    );
}

#[test]
fn pack_soa_instances_empty_returns_zero_and_clears_out() {
    let soa = SpriteSoA::default();
    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");

    let mut out = Vec::with_capacity(8);
    out.push(SpriteInstance::zeroed());
    let prev_capacity = out.capacity();

    let count_uncut = pack_soa_instances(&soa, None, &mut out);
    assert_eq!(count_uncut, 0);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), prev_capacity);

    out.push(SpriteInstance::zeroed());
    let count_culled = pack_soa_instances(&soa, Some(&camera), &mut out);
    assert_eq!(count_culled, 0);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), prev_capacity);
}

#[test]
fn pack_soa_instances_without_culling_preserves_all_and_reuses_capacity() {
    let mut soa = SpriteSoA::with_capacity(3);
    soa.push(
        [100.0, 200.0],
        [10.0, -5.0],
        [16.0, 32.0],
        0.25,
        0.0,
        FULL_UV_RECT,
        [255, 0, 0, 255],
    );
    soa.push(
        [5000.0, -3000.0],
        [0.0, 0.0],
        [8.0, 8.0],
        0.0,
        0.0,
        [100, 200, 300, 400],
        [0, 255, 0, 128],
    );
    soa.push(
        [640.0, 360.0],
        [-2.0, 4.0],
        [24.0, 12.0],
        -1.5,
        0.1,
        [0, 0, 32_768, 32_768],
        [0, 0, 255, 255],
    );

    let mut out = Vec::with_capacity(16);
    // Pre-populate stale entries to verify `pack_soa_instances` clears before packing.
    for _ in 0..5 {
        out.push(SpriteInstance::zeroed());
    }
    let cap_before = out.capacity();
    let ptr_before = out.as_ptr();

    let packed = pack_soa_instances(&soa, None, &mut out);
    assert_eq!(packed, 3);
    assert_eq!(out.len(), 3);
    assert_eq!(out.capacity(), cap_before);
    assert_eq!(out.as_ptr(), ptr_before);

    assert_eq!(
        out[0],
        SpriteInstance {
            pos: [100.0, 200.0],
            size: [16.0, 32.0],
            uv_rect: FULL_UV_RECT,
            rotation: 0.25,
            color: [255, 0, 0, 255],
        }
    );
    assert_eq!(
        out[1],
        SpriteInstance {
            pos: [5000.0, -3000.0],
            size: [8.0, 8.0],
            uv_rect: [100, 200, 300, 400],
            rotation: 0.0,
            color: [0, 255, 0, 128],
        }
    );
    assert_eq!(
        out[2],
        SpriteInstance {
            pos: [640.0, 360.0],
            size: [24.0, 12.0],
            uv_rect: [0, 0, 32_768, 32_768],
            rotation: -1.5,
            color: [0, 0, 255, 255],
        }
    );
}

#[test]
fn pack_soa_instances_with_culling_keeps_visible_and_edge_rotated_sprites() {
    // Visible bounds: [0, 1280] x [0, 720].
    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");
    let mut soa = SpriteSoA::with_capacity(6);

    // 1. Inside viewport -> kept.
    soa.push(
        [640.0, 360.0],
        [0.0, 0.0],
        [20.0, 20.0],
        0.0,
        0.0,
        FULL_UV_RECT,
        [1, 0, 0, 255],
    );
    // 2. Far outside right -> culled.
    soa.push(
        [2000.0, 360.0],
        [0.0, 0.0],
        [20.0, 20.0],
        0.0,
        0.0,
        FULL_UV_RECT,
        [2, 0, 0, 255],
    );
    // 3. Far outside top -> culled.
    soa.push(
        [640.0, -500.0],
        [0.0, 0.0],
        [20.0, 20.0],
        0.75,
        0.0,
        FULL_UV_RECT,
        [3, 0, 0, 255],
    );
    // 4. Unrotated overlapping right boundary ([1275, 1295] intersects [0, 1280]) -> kept.
    soa.push(
        [1285.0, 360.0],
        [0.0, 0.0],
        [20.0, 20.0],
        0.0,
        0.0,
        FULL_UV_RECT,
        [4, 0, 0, 255],
    );
    // 5. Just outside left edge without rotation (half_extent = 10 -> max.x = -2 < 0) -> culled.
    soa.push(
        [-12.0, 360.0],
        [0.0, 0.0],
        [20.0, 20.0],
        0.0,
        0.0,
        FULL_UV_RECT,
        [5, 0, 0, 255],
    );
    // 6. Same position (-12, 360) with rotation != 0: bounding radius sqrt(10^2 + 10^2) ~= 14.14 > 12 -> kept.
    soa.push(
        [-12.0, 360.0],
        [0.0, 0.0],
        [20.0, 20.0],
        core::f32::consts::FRAC_PI_4,
        0.0,
        FULL_UV_RECT,
        [6, 0, 0, 255],
    );

    let mut out = Vec::new();
    let count = pack_soa_instances(&soa, Some(&camera), &mut out);
    assert_eq!(count, 3);
    assert_eq!(out.len(), 3);

    let kept_colors: Vec<[u8; 4]> = out.iter().map(|inst| inst.color).collect();
    assert_eq!(
        kept_colors,
        vec![[1, 0, 0, 255], [4, 0, 0, 255], [6, 0, 0, 255]]
    );
}

#[test]
fn offscreen_gpu_one_draw_call_benchmark_smoke_test() {
    let mut renderer = match GpuRenderer::new_offscreen(1280, 720, 100_000) {
        Ok(renderer) => renderer,
        Err(RenderError::RequestAdapter(err)) => {
            log::warn!("Skipping GPU offscreen smoke test (no adapter available): {err}");
            return;
        }
        Err(err) => panic!("unexpected error initializing offscreen renderer: {err}"),
    };

    assert!(renderer.is_offscreen());
    assert_eq!(renderer.dimensions(), (1280, 720));

    let sim_config = SimConfig::default();
    let mut soa = SpriteSoA::spawn_random_batch(100_000, 42, &sim_config)
        .expect("spawn_random_batch should succeed");
    let mut camera =
        Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");

    let dt = 1.0 / 60.0;
    let bench_start = std::time::Instant::now();
    for _ in 0..10 {
        soa.step(dt, sim_config.world_width, sim_config.world_height)
            .expect("step should succeed");
        let stats = renderer
            .render_soa(&camera, &soa, false, wgpu::Color::BLACK)
            .expect("offscreen render_soa should succeed");
        assert_eq!(stats.draw_calls, 1);
        assert_eq!(stats.submitted_instances, 100_000);
        assert!(!stats.skipped);
    }
    let elapsed_ms = bench_start.elapsed().as_secs_f64() * 1000.0;
    let avg_ms = elapsed_ms / 10.0;
    let avg_fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };
    let info = renderer.adapter_info();
    println!(
        "[offscreen-gpu-smoke] adapter='{}' backend={:?} 10x100k avg={:.2}ms ({:.1} FPS) draw_calls=1",
        info.name, info.backend, avg_ms, avg_fps
    );

    renderer
        .resize(640, 480)
        .expect("resize to 640x480 should succeed");
    assert_eq!(renderer.dimensions(), (640, 480));

    camera
        .set_viewport_size(640.0, 480.0)
        .expect("camera viewport update should succeed");
    let resized_stats = renderer
        .render_soa(&camera, &soa, false, wgpu::Color::BLACK)
        .expect("render_soa after resize should succeed");
    assert_eq!(resized_stats.draw_calls, 1);
    assert_eq!(resized_stats.submitted_instances, 100_000);

    let invalid_resize = renderer
        .resize(0, 0)
        .expect_err("resize(0, 0) must return InvalidResolution");
    assert!(matches!(
        invalid_resize,
        RenderError::InvalidResolution {
            width: 0,
            height: 0
        }
    ));
    assert_eq!(renderer.dimensions(), (640, 480));
}

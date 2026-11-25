// Step 5 unit tests for `fge-render`: `DynamicAtlas` shelf packing, PNG decoding,
// ASCII + Japanese glyph rasterization/caching, and `GpuRenderer::render_world_and_ui`.

use fge_core::{
    camera::Camera2D,
    ui::UiCommandQueue,
    world::EntityWorld,
};

use crate::{
    atlas::{DynamicAtlas, GlyphInfo, TextureId, ATLAS_SIZE, SOLID_WHITE_UV},
    context::GpuRenderer,
    error::RenderError,
};

fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("PNG header write should succeed");
        writer
            .write_image_data(rgba)
            .expect("PNG image data write should succeed");
    }
    out
}

fn solid_rgba(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        bytes.extend_from_slice(&color);
    }
    bytes
}

fn uv_rects_overlap(a: [u16; 4], b: [u16; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

#[test]
fn dynamic_atlas_png_decode_and_shelf_packing() {
    let mut atlas = DynamicAtlas::new();
    assert_eq!(atlas.width, ATLAS_SIZE);
    assert_eq!(atlas.height, ATLAS_SIZE);
    assert_eq!(atlas.rgba_data().len(), (ATLAS_SIZE * ATLAS_SIZE * 4) as usize);

    // Top-left reserved white block should be opaque white.
    assert_eq!(&atlas.rgba_data()[0..4], &[255, 255, 255, 255]);

    // Encode a 4x4 RGBA PNG in memory and load it into DynamicAtlas.
    let mut pixels_4x4 = Vec::with_capacity(4 * 4 * 4);
    for i in 0..16u8 {
        pixels_4x4.extend_from_slice(&[i * 15, 200, 100, 255]);
    }
    let png_4x4 = encode_rgba_png(4, 4, &pixels_4x4);
    let tex_4x4 = atlas
        .load_png_bytes(&png_4x4)
        .expect("4x4 RGBA PNG should decode and pack");
    let uv_4x4 = atlas
        .get_texture_uv(tex_4x4)
        .expect("4x4 texture handle should resolve to UV rect");
    assert_ne!(uv_4x4, SOLID_WHITE_UV);
    assert!(uv_4x4[0] < uv_4x4[2] && uv_4x4[1] < uv_4x4[3]);
    assert!(atlas.take_dirty_rect().is_some());

    // Invalid handles and malformed PNG bytes must fail gracefully.
    assert_eq!(atlas.get_texture_uv(TextureId::INVALID), None);
    assert_eq!(
        atlas.get_texture_uv(TextureId {
            index: 0,
            generation: 0,
        }),
        None
    );
    assert!(atlas.load_png_bytes(b"not a png").is_err());

    // Pack two wide textures (1200x32 each) so the second must wrap to a new shelf row.
    let wide_a = encode_rgba_png(1200, 32, &solid_rgba(1200, 32, [255, 40, 40, 255]));
    let wide_b = encode_rgba_png(1200, 32, &solid_rgba(1200, 32, [40, 255, 40, 255]));
    let small_c = encode_rgba_png(16, 16, &solid_rgba(16, 16, [40, 40, 255, 255]));

    let id_a = atlas.load_png_bytes(&wide_a).expect("wide_a should pack");
    let id_b = atlas.load_png_bytes(&wide_b).expect("wide_b should pack on next shelf");
    let id_c = atlas.load_png_bytes(&small_c).expect("small_c should pack");

    let uv_a = atlas.get_texture_uv(id_a).unwrap();
    let uv_b = atlas.get_texture_uv(id_b).unwrap();
    let uv_c = atlas.get_texture_uv(id_c).unwrap();

    let all_uvs = [SOLID_WHITE_UV, uv_4x4, uv_a, uv_b, uv_c];
    for i in 0..all_uvs.len() {
        assert!(all_uvs[i][0] < all_uvs[i][2]);
        assert!(all_uvs[i][1] < all_uvs[i][3]);
        for j in (i + 1)..all_uvs.len() {
            assert!(
                !uv_rects_overlap(all_uvs[i], all_uvs[j]),
                "atlas UV rects {i} ({:?}) and {j} ({:?}) must not overlap",
                all_uvs[i],
                all_uvs[j]
            );
        }
    }

    // Since 1200 + 1200 > 2048, wide_b must be placed on a lower shelf (v0 >= uv_a[3]).
    assert!(
        uv_b[1] >= uv_a[3],
        "expected wide_b v0 ({}) >= wide_a v1 ({}) due to shelf wrap",
        uv_b[1],
        uv_a[3]
    );
}

#[test]
fn dynamic_atlas_ascii_and_japanese_glyph_rasterization_and_cache_reuse() {
    let mut atlas = DynamicAtlas::new();
    let _ = atlas.take_dirty_rect();

    let text = "勇者 LV.5 スライム討伐";
    let px_size = 24.0;

    let first_pass: Vec<(char, GlyphInfo)> = text
        .chars()
        .map(|ch| (ch, atlas.rasterize_glyph(ch, px_size)))
        .collect();

    for &(ch, info) in &first_pass {
        assert!(
            info.advance > 0.0,
            "glyph '{ch}' must have positive advance (got {})",
            info.advance
        );
        if ch.is_ascii() && !ch.is_whitespace() {
            assert!(
                info.width > 0 && info.height > 0,
                "ASCII glyph '{ch}' must rasterize with non-zero dimensions"
            );
            assert!(info.uv_rect[0] < info.uv_rect[2]);
            assert!(info.uv_rect[1] < info.uv_rect[3]);
        }
    }

    // Clear dirty rect accumulated during first rasterization pass.
    let _ = atlas.take_dirty_rect();

    // Second pass must hit the glyph cache identically with zero new atlas writes.
    for &(ch, expected) in &first_pass {
        let cached = atlas.rasterize_glyph(ch, px_size);
        assert_eq!(
            cached, expected,
            "cached glyph info for '{ch}' must match initial rasterization"
        );
    }
    assert_eq!(
        atlas.take_dirty_rect(),
        None,
        "cache-hit rasterization must not dirty the atlas"
    );
}

#[test]
fn offscreen_render_world_and_ui_smoke_test() {
    let mut renderer = match GpuRenderer::new_offscreen(1280, 720, 1024) {
        Ok(renderer) => renderer,
        Err(RenderError::RequestAdapter(err)) => {
            log::warn!("Skipping GPU offscreen render_world_and_ui smoke test (no adapter): {err}");
            return;
        }
        Err(err) => panic!("unexpected error initializing offscreen renderer: {err}"),
    };

    let camera = Camera2D::from_top_left_viewport(1280.0, 720.0).expect("valid viewport camera");
    let mut world = EntityWorld::with_capacity(4);
    world.spawn_sprite([320.0, 240.0], [32.0, 32.0], [60, 220, 120, 255]);
    world.spawn_sprite([520.0, 300.0], [40.0, 40.0], [220, 80, 80, 255]);

    let mut ui = UiCommandQueue::new();
    ui.push_panel(
        16.0,
        16.0,
        320.0,
        96.0,
        [16, 20, 36, 220],
        [220, 200, 120, 255],
        2.0,
    );
    ui.push_bar(28.0, 52.0, 220.0, 14.0, 0.75, [60, 220, 90, 255], [40, 20, 20, 255]);
    ui.push_screen_text(28.0, 26.0, 20.0, [255, 255, 255, 255], "勇者 LV.5");
    ui.push_world_text(
        520.0,
        260.0,
        18.0,
        [255, 230, 100, 255],
        "スライム討伐 -34",
    );
    ui.push_dialog_box(
        140.0,
        520.0,
        1000.0,
        160.0,
        "村長",
        "勇者よ！北の遺跡に黒曜のゴーレムが目覚めた！",
        12,
    );

    let stats = renderer
        .render_world_and_ui(&camera, &world, &ui, true, wgpu::Color::BLACK)
        .expect("offscreen render_world_and_ui should succeed");
    assert!(!stats.skipped);
    assert!(stats.draw_calls >= 1);
    assert!(stats.submitted_instances > 0);
}

use std::{
    collections::HashMap,
    io::Cursor,
    path::Path,
    sync::OnceLock,
};

use ab_glyph::{Font, FontArc, FontVec, PxScale, ScaleFont};
use fge_core::handle::{EntityId, GenerationalArena};

use crate::{error::RenderError, instance::SpriteInstance};

/// Generational texture handle (`FgeTexture` in `fge.h`).
pub type TextureId = EntityId;

/// Atlas dimension in texels (`2048 x 2048` RGBA8).
pub const ATLAS_SIZE: u32 = 2048;

/// Reserved opaque white block size at the top-left of the atlas (`8 x 8` texels).
const WHITE_BLOCK_SIZE: u32 = 8;

/// Texel padding between shelf allocations to prevent bilinear bleed.
const ATLAS_PADDING: u32 = 2;

/// Normalized `[u0, v0, u1, v1]` rectangle covering the reserved top-left opaque white block.
pub const SOLID_WHITE_UV: [u16; 4] = [32, 32, 224, 224];

/// Candidate system CJK/Latin font paths probed once on startup.
const SYSTEM_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "C:\\Windows\\Fonts\\meiryo.ttc",
    "C:\\Windows\\Fonts\\msgothic.ttc",
];

/// Cached read-only system font handle (loaded at most once per process, lock-free).
static SYSTEM_DEFAULT_FONT: OnceLock<Option<FontArc>> = OnceLock::new();

fn probe_system_font() -> Option<FontArc> {
    SYSTEM_DEFAULT_FONT
        .get_or_init(|| {
            for candidate in SYSTEM_FONT_CANDIDATES {
                let path = Path::new(candidate);
                if !path.is_file() {
                    continue;
                }
                let Ok(bytes) = std::fs::read(path) else {
                    continue;
                };
                if let Ok(font_vec) = FontVec::try_from_vec_and_index(bytes, 0) {
                    log::info!("fge-render: auto-loaded system font '{}'", candidate);
                    return Some(FontArc::new(font_vec));
                }
            }
            log::warn!("fge-render: no system font found; using built-in 8x8 ASCII bitmap fallback");
            None
        })
        .clone()
}

/// Placement metrics for a rasterized glyph inside [`DynamicAtlas`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GlyphInfo {
    pub uv_rect: [u16; 4],
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub advance: f32,
}

/// Dynamic 2048x2048 RGBA8 CPU-mirrored GPU shelf atlas supporting PNG textures,
/// `ab_glyph` UTF-8/Japanese CJK rasterization, and an 8x8 ASCII bitmap fallback.
pub struct DynamicAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_height: u32,
    dirty_rect: Option<[u32; 4]>,
    textures: GenerationalArena,
    texture_uvs: Vec<[u16; 4]>,
    fonts: Vec<FontArc>,
    glyph_cache: HashMap<(char, u16), GlyphInfo>,
}

impl Default for DynamicAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicAtlas {
    /// Creates a new `2048 x 2048` RGBA8 shelf atlas with a reserved `8 x 8` opaque white block
    /// at `(0, 0)` and auto-loads a system CJK/Latin font when available.
    pub fn new() -> Self {
        let width = ATLAS_SIZE;
        let height = ATLAS_SIZE;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for y in 0..WHITE_BLOCK_SIZE {
            for x in 0..WHITE_BLOCK_SIZE {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }

        let mut fonts = Vec::new();
        if let Some(sys_font) = probe_system_font() {
            fonts.push(sys_font);
        }

        Self {
            width,
            height,
            pixels,
            shelf_x: WHITE_BLOCK_SIZE + ATLAS_PADDING,
            shelf_y: 0,
            shelf_height: WHITE_BLOCK_SIZE + ATLAS_PADDING,
            dirty_rect: Some([0, 0, WHITE_BLOCK_SIZE, WHITE_BLOCK_SIZE]),
            textures: GenerationalArena::default(),
            texture_uvs: Vec::new(),
            fonts,
            glyph_cache: HashMap::new(),
        }
    }

    /// Returns `true` if at least one TrueType/OpenType font is loaded.
    pub fn has_truetype_font(&self) -> bool {
        !self.fonts.is_empty()
    }

    /// Returns the number of live textures registered in this atlas.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Returns the number of cached rasterized glyphs.
    pub fn cached_glyph_count(&self) -> usize {
        self.glyph_cache.len()
    }

    /// Decodes PNG bytes (RGB, RGBA, Grayscale, GrayscaleAlpha, or Indexed), packs the RGBA8
    /// image into the shelf atlas with 1px edge extrusion, and returns a generational [`TextureId`].
    pub fn load_png_bytes(&mut self, bytes: &[u8]) -> Result<TextureId, RenderError> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(
            png::Transformations::EXPAND
                | png::Transformations::STRIP_16
                | png::Transformations::ALPHA,
        );
        let mut reader = decoder
            .read_info()
            .map_err(|_| RenderError::SurfaceFatal("failed to read PNG header"))?;

        let (img_w, img_h) = reader.info().size();
        if img_w == 0 || img_h == 0 || img_w > self.width || img_h > self.height {
            return Err(RenderError::InvalidResolution {
                width: img_w,
                height: img_h,
            });
        }

        let out_size = reader
            .output_buffer_size()
            .ok_or(RenderError::SurfaceFatal("PNG decoded buffer size overflow"))?;
        let mut raw = vec![0u8; out_size];
        let info = reader
            .next_frame(&mut raw)
            .map_err(|_| RenderError::SurfaceFatal("failed to decode PNG frame"))?;

        let pixel_count = (info.width as usize) * (info.height as usize);
        let rgba: Vec<u8> = match info.color_type {
            png::ColorType::Rgba => raw[..pixel_count * 4].to_vec(),
            png::ColorType::Rgb => {
                let mut out = Vec::with_capacity(pixel_count * 4);
                for chunk in raw[..pixel_count * 3].chunks_exact(3) {
                    out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                out
            }
            png::ColorType::GrayscaleAlpha => {
                let mut out = Vec::with_capacity(pixel_count * 4);
                for chunk in raw[..pixel_count * 2].chunks_exact(2) {
                    out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
                }
                out
            }
            png::ColorType::Grayscale => {
                let mut out = Vec::with_capacity(pixel_count * 4);
                for &g in &raw[..pixel_count] {
                    out.extend_from_slice(&[g, g, g, 255]);
                }
                out
            }
            png::ColorType::Indexed => {
                return Err(RenderError::SurfaceFatal(
                    "unexpected unexpanded indexed PNG",
                ));
            }
        };

        let extrude = info.width + 2 <= self.width && info.height + 2 <= self.height;
        let uv_rect = self
            .pack_rgba(info.width, info.height, &rgba, extrude)
            .ok_or(RenderError::SurfaceFatal("dynamic atlas is full"))?;

        let id = self.textures.allocate();
        self.texture_uvs.push(uv_rect);
        Ok(id)
    }

    /// Reads a PNG file from disk and loads it into the atlas.
    pub fn load_png_file(&mut self, path: &Path) -> Result<TextureId, RenderError> {
        let bytes = std::fs::read(path)
            .map_err(|_| RenderError::SurfaceFatal("failed to read PNG file"))?;
        self.load_png_bytes(&bytes)
    }

    /// Resolves a generational [`TextureId`] to its normalized `[u16; 4]` atlas UV rectangle.
    pub fn get_texture_uv(&self, id: TextureId) -> Option<[u16; 4]> {
        let dense = self.textures.resolve(id)?;
        self.texture_uvs.get(dense).copied()
    }

    /// Loads a TrueType/OpenType font (or `.ttc` collection, face index 0) as the primary font.
    pub fn load_font_bytes(&mut self, bytes: Vec<u8>) -> Result<(), RenderError> {
        let font_vec = FontVec::try_from_vec_and_index(bytes, 0)
            .map_err(|_| RenderError::SurfaceFatal("failed to parse font bytes"))?;
        self.fonts.insert(0, FontArc::new(font_vec));
        self.glyph_cache.clear();
        Ok(())
    }

    /// Reads a font file from disk and loads it as the primary font.
    pub fn load_font_file(&mut self, path: &Path) -> Result<(), RenderError> {
        let bytes = std::fs::read(path)
            .map_err(|_| RenderError::SurfaceFatal("failed to read font file"))?;
        self.load_font_bytes(bytes)
    }

    /// Returns the recommended vertical line advance in pixels for `px_size`.
    pub fn line_height(&self, px_size: f32) -> f32 {
        let px = sanitize_px_size(px_size);
        if let Some(font) = self.fonts.first() {
            let scaled = font.as_scaled(PxScale::from(px));
            let metrics_h = scaled.ascent() - scaled.descent() + scaled.line_gap();
            if metrics_h.is_finite() && metrics_h > 0.0 {
                return metrics_h.max(px * 1.15);
            }
        }
        px * 1.25
    }

    /// Rasterizes `ch` at quantized pixel size `px_size` (using cached entries on repeat calls).
    pub fn rasterize_glyph(&mut self, ch: char, px_size: f32) -> GlyphInfo {
        let px = sanitize_px_size(px_size);
        let quantized_px = px.round().clamp(6.0, 256.0) as u16;
        let key = (ch, quantized_px);

        if let Some(&cached) = self.glyph_cache.get(&key) {
            return cached;
        }

        let info = self
            .rasterize_from_fonts(ch, quantized_px)
            .unwrap_or_else(|| self.rasterize_bitmap_fallback(ch, quantized_px));

        self.glyph_cache.insert(key, info);
        info
    }

    /// Measures the horizontal advance width of a single line (without newlines).
    pub fn measure_line_width(&mut self, line: &str, px_size: f32) -> f32 {
        let mut width = 0.0f32;
        for ch in line.chars() {
            if ch == '\r' || ch == '\n' {
                continue;
            }
            width += self.rasterize_glyph(ch, px_size).advance;
        }
        width
    }

    /// Lays out multi-line UTF-8 text (supporting `\n` and automatic character wrapping inside
    /// `max_width` for both Latin and Japanese CJK) into `SpriteInstance` quads.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_text(
        &mut self,
        text: &str,
        origin_x: f32,
        origin_y: f32,
        px_size: f32,
        color: [u8; 4],
        max_width: Option<f32>,
        max_height: Option<f32>,
        center_lines: bool,
        drop_shadow: bool,
        out: &mut Vec<SpriteInstance>,
    ) {
        if text.is_empty() || color[3] == 0 {
            return;
        }

        let px = sanitize_px_size(px_size);
        let line_h = self.line_height(px);
        let wrap_limit = max_width.filter(|&w| w.is_finite() && w > 0.0);

        let mut lines: Vec<(Vec<(GlyphInfo, f32)>, f32)> = Vec::new();
        let mut cur_line: Vec<(GlyphInfo, f32)> = Vec::new();
        let mut cur_x = 0.0f32;

        for ch in text.chars() {
            if ch == '\r' {
                continue;
            }
            if ch == '\n' {
                lines.push((core::mem::take(&mut cur_line), cur_x));
                cur_x = 0.0;
                continue;
            }

            let info = self.rasterize_glyph(ch, px);
            if let Some(limit) = wrap_limit {
                if cur_x > 0.0 && cur_x + info.advance > limit {
                    lines.push((core::mem::take(&mut cur_line), cur_x));
                    cur_x = 0.0;
                    if ch == ' ' {
                        continue;
                    }
                }
            }

            cur_line.push((info, cur_x));
            cur_x += info.advance;
        }
        if !cur_line.is_empty() || lines.is_empty() {
            lines.push((cur_line, cur_x));
        }

        let shadow_color = [6u8, 8, 16, ((color[3] as u16 * 200) / 255) as u8];
        let shadow_offset = (px * 0.07).clamp(1.0, 2.0);

        for (line_idx, (glyphs, line_w)) in lines.into_iter().enumerate() {
            let line_y = origin_y + (line_idx as f32) * line_h;
            if let Some(max_h) = max_height {
                if (line_idx as f32 + 1.0) * line_h > max_h + line_h * 0.25 {
                    break;
                }
            }

            let base_x = if center_lines {
                origin_x - line_w * 0.5
            } else {
                origin_x
            };

            if drop_shadow && shadow_color[3] > 0 {
                for &(info, pen_x) in &glyphs {
                    if info.width == 0 || info.height == 0 {
                        continue;
                    }
                    let w = info.width as f32;
                    let h = info.height as f32;
                    out.push(SpriteInstance {
                        pos: [
                            base_x + pen_x + info.offset_x + w * 0.5 + shadow_offset,
                            line_y + info.offset_y + h * 0.5 + shadow_offset,
                        ],
                        size: [w, h],
                        uv_rect: info.uv_rect,
                        rotation: 0.0,
                        color: shadow_color,
                    });
                }
            }

            for (info, pen_x) in glyphs {
                if info.width == 0 || info.height == 0 {
                    continue;
                }
                let w = info.width as f32;
                let h = info.height as f32;
                out.push(SpriteInstance {
                    pos: [
                        base_x + pen_x + info.offset_x + w * 0.5,
                        line_y + info.offset_y + h * 0.5,
                    ],
                    size: [w, h],
                    uv_rect: info.uv_rect,
                    rotation: 0.0,
                    color,
                });
            }
        }
    }

    /// Takes and clears the pending dirty rectangle `[x, y, width, height]` needing GPU upload.
    pub fn take_dirty_rect(&mut self) -> Option<[u32; 4]> {
        self.dirty_rect.take()
    }

    /// Full CPU-mirrored RGBA8 pixel buffer (`ATLAS_SIZE * ATLAS_SIZE * 4` bytes).
    pub fn rgba_data(&self) -> &[u8] {
        &self.pixels
    }

    fn rasterize_from_fonts(&mut self, ch: char, quantized_px: u16) -> Option<GlyphInfo> {
        let scale = PxScale::from(quantized_px as f32);
        let mut rendered: Option<(u32, u32, Vec<u8>, f32, f32, f32)> = None;
        let mut empty_advance: Option<f32> = None;

        for font in &self.fonts {
            let glyph_id = font.glyph_id(ch);
            if glyph_id.0 == 0 && ch != ' ' && ch != '\u{3000}' {
                continue;
            }
            let scaled = font.as_scaled(scale);
            let advance = scaled.h_advance(glyph_id);
            let ascent = scaled.ascent();
            let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(0.0, ascent));

            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let w = bounds.width().ceil().max(0.0) as u32;
                let h = bounds.height().ceil().max(0.0) as u32;
                if w > 0 && h > 0 {
                    let mut rgba = vec![0u8; (w * h * 4) as usize];
                    outlined.draw(|gx, gy, cov| {
                        if gx < w && gy < h {
                            let a = (cov.clamp(0.0, 1.0) * 255.0).round() as u8;
                            let idx = ((gy * w + gx) * 4) as usize;
                            rgba[idx] = 255;
                            rgba[idx + 1] = 255;
                            rgba[idx + 2] = 255;
                            rgba[idx + 3] = a;
                        }
                    });
                    rendered = Some((w, h, rgba, bounds.min.x, bounds.min.y, advance));
                    break;
                }
            }

            let fallback_adv = if advance > 0.0 {
                advance
            } else if ch == '\u{3000}' {
                quantized_px as f32
            } else {
                quantized_px as f32 * 0.45
            };
            empty_advance = Some(fallback_adv);
            break;
        }

        if let Some((w, h, rgba, offset_x, offset_y, advance)) = rendered {
            let uv_rect = self
                .pack_rgba(w, h, &rgba, false)
                .unwrap_or(SOLID_WHITE_UV);
            return Some(GlyphInfo {
                uv_rect,
                width: w,
                height: h,
                offset_x,
                offset_y,
                advance: if advance > 0.0 { advance } else { w as f32 + 1.0 },
            });
        }

        empty_advance.map(|advance| GlyphInfo {
            uv_rect: [0, 0, 0, 0],
            width: 0,
            height: 0,
            offset_x: 0.0,
            offset_y: 0.0,
            advance,
        })
    }

    fn rasterize_bitmap_fallback(&mut self, ch: char, quantized_px: u16) -> GlyphInfo {
        let px = quantized_px as f32;
        if ch == ' ' {
            return GlyphInfo {
                uv_rect: [0, 0, 0, 0],
                width: 0,
                height: 0,
                offset_x: 0.0,
                offset_y: 0.0,
                advance: (px * 0.5).max(4.0),
            };
        }
        if ch == '\u{3000}' {
            return GlyphInfo {
                uv_rect: [0, 0, 0, 0],
                width: 0,
                height: 0,
                offset_x: 0.0,
                offset_y: 0.0,
                advance: px.max(8.0),
            };
        }

        let rows = bitmap_8x8_for_char(ch);
        let out_h = quantized_px.clamp(8, 256) as u32;
        let out_w = ((out_h as f32 * 0.75).round() as u32).clamp(6, 256);
        let mut rgba = vec![0u8; (out_w * out_h * 4) as usize];

        for y in 0..out_h {
            let src_y = ((y * 8) / out_h).min(7) as usize;
            let row_bits = rows[src_y];
            for x in 0..out_w {
                let src_x = ((x * 8) / out_w).min(7) as u32;
                let lit = (row_bits & (0x80u8 >> src_x)) != 0;
                if lit {
                    let idx = ((y * out_w + x) * 4) as usize;
                    rgba[idx] = 255;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 255;
                    rgba[idx + 3] = 255;
                }
            }
        }

        let uv_rect = self
            .pack_rgba(out_w, out_h, &rgba, false)
            .unwrap_or(SOLID_WHITE_UV);

        GlyphInfo {
            uv_rect,
            width: out_w,
            height: out_h,
            offset_x: 0.0,
            offset_y: 0.0,
            advance: out_w as f32 + (px * 0.1).max(1.0),
        }
    }

    fn pack_rgba(
        &mut self,
        w: u32,
        h: u32,
        rgba: &[u8],
        extrude_border: bool,
    ) -> Option<[u16; 4]> {
        if w == 0 || h == 0 {
            return None;
        }
        let border = if extrude_border { 1u32 } else { 0u32 };
        let alloc_w = w.checked_add(border * 2)?;
        let alloc_h = h.checked_add(border * 2)?;
        if alloc_w > self.width || alloc_h > self.height {
            return None;
        }

        if self.shelf_x + alloc_w > self.width {
            self.shelf_y = self.shelf_y.saturating_add(self.shelf_height);
            self.shelf_x = 0;
            self.shelf_height = 0;
        }
        if self.shelf_y + alloc_h > self.height {
            return None;
        }

        let dst_x = self.shelf_x;
        let dst_y = self.shelf_y;
        self.shelf_x = (self.shelf_x + alloc_w + ATLAS_PADDING).min(self.width);
        self.shelf_height = self.shelf_height.max(alloc_h + ATLAS_PADDING);

        let inner_x = dst_x + border;
        let inner_y = dst_y + border;

        for row in 0..h {
            let src_start = (row * w * 4) as usize;
            let src_end = src_start + (w * 4) as usize;
            let dst_start = (((inner_y + row) * self.width + inner_x) * 4) as usize;
            let dst_end = dst_start + (w * 4) as usize;
            self.pixels[dst_start..dst_end].copy_from_slice(&rgba[src_start..src_end]);
        }

        if extrude_border {
            // Extrude left and right columns.
            for row in 0..h {
                let y = inner_y + row;
                self.copy_pixel(inner_x, y, dst_x, y);
                self.copy_pixel(inner_x + w - 1, y, inner_x + w, y);
            }
            // Extrude top and bottom rows (including corners).
            for x in dst_x..(dst_x + alloc_w) {
                self.copy_pixel(x, inner_y, x, dst_y);
                self.copy_pixel(x, inner_y + h - 1, x, inner_y + h);
            }
        }

        self.mark_dirty(dst_x, dst_y, alloc_w, alloc_h);

        let inv_w = 65535.0f64 / (self.width as f64);
        let inv_h = 65535.0f64 / (self.height as f64);
        let u0 = ((inner_x as f64) * inv_w).round().clamp(0.0, 65535.0) as u16;
        let v0 = ((inner_y as f64) * inv_h).round().clamp(0.0, 65535.0) as u16;
        let u1 = (((inner_x + w) as f64) * inv_w).round().clamp(0.0, 65535.0) as u16;
        let v1 = (((inner_y + h) as f64) * inv_h).round().clamp(0.0, 65535.0) as u16;
        Some([u0, v0, u1, v1])
    }

    #[inline]
    fn copy_pixel(&mut self, src_x: u32, src_y: u32, dst_x: u32, dst_y: u32) {
        let s = ((src_y * self.width + src_x) * 4) as usize;
        let d = ((dst_y * self.width + dst_x) * 4) as usize;
        let px = [
            self.pixels[s],
            self.pixels[s + 1],
            self.pixels[s + 2],
            self.pixels[s + 3],
        ];
        self.pixels[d..d + 4].copy_from_slice(&px);
    }

    fn mark_dirty(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.dirty_rect = match self.dirty_rect {
            None => Some([x, y, w, h]),
            Some([dx, dy, dw, dh]) => {
                let min_x = dx.min(x);
                let min_y = dy.min(y);
                let max_x = (dx + dw).max(x + w);
                let max_y = (dy + dh).max(y + h);
                Some([min_x, min_y, max_x - min_x, max_y - min_y])
            }
        };
    }
}

#[inline]
fn sanitize_px_size(px_size: f32) -> f32 {
    if px_size.is_finite() {
        px_size.clamp(6.0, 256.0)
    } else {
        16.0
    }
}

/// Crisp built-in 8x8 monochrome bitmap font covering ASCII `0x20..=0x7E`
/// (with a clean hollow box fallback for out-of-range codepoints when no TTF is installed).
fn bitmap_8x8_for_char(ch: char) -> [u8; 8] {
    let code = ch as u32;
    if !(0x20..=0x7E).contains(&code) {
        return [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00];
    }
    ASCII_8X8[(code - 0x20) as usize]
}

const ASCII_8X8: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x20 ' '
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00], // 0x21 '!'
    [0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x22 '"'
    [0x6C, 0x6C, 0xFE, 0x6C, 0xFE, 0x6C, 0x6C, 0x00], // 0x23 '#'
    [0x18, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x18, 0x00], // 0x24 '$'
    [0x00, 0xC6, 0xCC, 0x18, 0x30, 0x66, 0xC6, 0x00], // 0x25 '%'
    [0x38, 0x6C, 0x38, 0x76, 0xDC, 0xCC, 0x76, 0x00], // 0x26 '&'
    [0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x27 '\''
    [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00], // 0x28 '('
    [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00], // 0x29 ')'
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00], // 0x2A '*'
    [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00], // 0x2B '+'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30], // 0x2C ','
    [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00], // 0x2D '-'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00], // 0x2E '.'
    [0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0x80, 0x00], // 0x2F '/'
    [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00], // 0x30 '0'
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00], // 0x31 '1'
    [0x3C, 0x66, 0x06, 0x1C, 0x30, 0x66, 0x7E, 0x00], // 0x32 '2'
    [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00], // 0x33 '3'
    [0x0E, 0x1E, 0x36, 0x66, 0x7F, 0x06, 0x0F, 0x00], // 0x34 '4'
    [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00], // 0x35 '5'
    [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00], // 0x36 '6'
    [0x7E, 0x66, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x00], // 0x37 '7'
    [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00], // 0x38 '8'
    [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00], // 0x39 '9'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00], // 0x3A ':'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30], // 0x3B ';'
    [0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00], // 0x3C '<'
    [0x00, 0x00, 0x7E, 0x00, 0x00, 0x7E, 0x00, 0x00], // 0x3D '='
    [0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00], // 0x3E '>'
    [0x3C, 0x66, 0x06, 0x1C, 0x18, 0x00, 0x18, 0x00], // 0x3F '?'
    [0x3C, 0x66, 0x6E, 0x6E, 0x60, 0x62, 0x3C, 0x00], // 0x40 '@'
    [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00], // 0x41 'A'
    [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00], // 0x42 'B'
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00], // 0x43 'C'
    [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00], // 0x44 'D'
    [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x7E, 0x00], // 0x45 'E'
    [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x00], // 0x46 'F'
    [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3E, 0x00], // 0x47 'G'
    [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00], // 0x48 'H'
    [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 0x49 'I'
    [0x1E, 0x0C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38, 0x00], // 0x4A 'J'
    [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00], // 0x4B 'K'
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00], // 0x4C 'L'
    [0xC6, 0xEE, 0xFE, 0xD6, 0xC6, 0xC6, 0xC6, 0x00], // 0x4D 'M'
    [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00], // 0x4E 'N'
    [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 0x4F 'O'
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00], // 0x50 'P'
    [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00], // 0x51 'Q'
    [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00], // 0x52 'R'
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00], // 0x53 'S'
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 0x54 'T'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 0x55 'U'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 0x56 'V'
    [0xC6, 0xC6, 0xC6, 0xD6, 0xFE, 0xEE, 0xC6, 0x00], // 0x57 'W'
    [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00], // 0x58 'X'
    [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00], // 0x59 'Y'
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00], // 0x5A 'Z'
    [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00], // 0x5B '['
    [0xC0, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00], // 0x5C '\\'
    [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00], // 0x5D ']'
    [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x5E '^'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00], // 0x5F '_'
    [0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x60 '`'
    [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00], // 0x61 'a'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00], // 0x62 'b'
    [0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00], // 0x63 'c'
    [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00], // 0x64 'd'
    [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00], // 0x65 'e'
    [0x1C, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x00], // 0x66 'f'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x3C], // 0x67 'g'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 0x68 'h'
    [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00], // 0x69 'i'
    [0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38], // 0x6A 'j'
    [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00], // 0x6B 'k'
    [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 0x6C 'l'
    [0x00, 0x00, 0xCC, 0xFE, 0xD6, 0xC6, 0xC6, 0x00], // 0x6D 'm'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 0x6E 'n'
    [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00], // 0x6F 'o'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60], // 0x70 'p'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06], // 0x71 'q'
    [0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x00], // 0x72 'r'
    [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00], // 0x73 's'
    [0x30, 0x30, 0x7C, 0x30, 0x30, 0x36, 0x1C, 0x00], // 0x74 't'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00], // 0x75 'u'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 0x76 'v'
    [0x00, 0x00, 0xC6, 0xC6, 0xD6, 0xFE, 0x6C, 0x00], // 0x77 'w'
    [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00], // 0x78 'x'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x3C], // 0x79 'y'
    [0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00], // 0x7A 'z'
    [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00], // 0x7B '{'
    [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 0x7C '|'
    [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00], // 0x7D '}'
    [0x76, 0xDC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x7E '~'
];

use std::path::Path;

use fge_core::handle::EntityId;

use crate::error::RenderError;

/// Generational texture handle (`FgeTexture` in `fge.h`).
pub type TextureId = EntityId;

/// Atlas dimension in texels (`2048 x 2048` RGBA8).
pub const ATLAS_SIZE: u32 = 2048;

/// Normalized `[u0, v0, u1, v1]` rectangle covering the reserved top-left opaque white block.
pub const SOLID_WHITE_UV: [u16; 4] = [32, 32, 224, 224];

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
}

impl Default for DynamicAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicAtlas {
    pub fn new() -> Self {
        todo!("implemented by impl-atlas-text")
    }

    pub fn load_png_bytes(&mut self, _bytes: &[u8]) -> Result<TextureId, RenderError> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn load_png_file(&mut self, _path: &Path) -> Result<TextureId, RenderError> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn get_texture_uv(&self, _id: TextureId) -> Option<[u16; 4]> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn load_font_bytes(&mut self, _bytes: Vec<u8>) -> Result<(), RenderError> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn load_font_file(&mut self, _path: &Path) -> Result<(), RenderError> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn rasterize_glyph(&mut self, _ch: char, _px_size: f32) -> GlyphInfo {
        todo!("implemented by impl-atlas-text")
    }

    pub fn take_dirty_rect(&mut self) -> Option<[u32; 4]> {
        todo!("implemented by impl-atlas-text")
    }

    pub fn rgba_data(&self) -> &[u8] {
        &self.pixels
    }
}

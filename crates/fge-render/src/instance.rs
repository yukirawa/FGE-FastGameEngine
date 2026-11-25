use bytemuck::{Pod, Zeroable};
use fge_core::{
    camera::Camera2D,
    sim::{BulkSimulation, SpriteSoA},
};

/// Exactly 32-byte GPU instance record (2 instances per 64-byte cache line, zero padding).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct SpriteInstance {
    /// World-space center `(x, y)` -> `@location(0) vec2<f32>`.
    pub pos: [f32; 2],
    /// World-space dimensions `(width, height)` -> `@location(1) vec2<f32>`.
    pub size: [f32; 2],
    /// Normalized UV rectangle `[u0, v0, u1, v1]` -> `@location(2) vec4<f32>` (`Unorm16x4`).
    pub uv_rect: [u16; 4],
    /// Counter-clockwise / screen-space rotation in radians -> `@location(3) f32`.
    pub rotation: f32,
    /// Linear/sRGB RGBA8 color `[r, g, b, a]` -> `@location(4) vec4<f32>` (`Unorm8x4`).
    pub color: [u8; 4],
}

const _: () = assert!(core::mem::size_of::<SpriteInstance>() == 32);
const _: () = assert!(core::mem::align_of::<SpriteInstance>() == 4);

impl SpriteInstance {
    /// Vertex attribute layout matching `shader.wgsl`.
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Unorm16x4,
        3 => Float32,
        4 => Unorm8x4,
    ];

    /// Instance-stepped `VertexBufferLayout` with 32-byte stride.
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// 64-byte orthographic projection uniform uploaded at `@group(0) @binding(0)`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

const _: () = assert!(core::mem::size_of::<CameraUniform>() == 64);

impl CameraUniform {
    /// Builds a `CameraUniform` from `Camera2D::projection_matrix()`.
    pub fn from_camera(camera: &Camera2D) -> Self {
        Self {
            view_proj: camera.projection_matrix().to_cols_array_2d(),
        }
    }
}

/// Packs `SpriteSoA` columns into a contiguous `Vec<SpriteInstance>` ready for `Queue::write_buffer`.
///
/// - Clears `out` without dropping its allocated buffer capacity.
/// - When `cull_camera` is `Some(cam)`, skips sprites outside `cam.visible_bounds()` using `cam.is_visible`.
/// - When `cull_camera` is `None`, packs all `soa.len()` sprites in a tight linear loop.
/// - Returns the number of packed instances (`out.len() as u32`).
pub fn pack_soa_instances(
    soa: &SpriteSoA,
    cull_camera: Option<&Camera2D>,
    out: &mut Vec<SpriteInstance>,
) -> u32 {
    out.clear();
    debug_assert!(soa.debug_assert_soa_invariants());

    let len = soa.len();
    if len == 0 {
        return 0;
    }

    if out.capacity() < len {
        out.reserve(len);
    }

    let pos_x = &soa.pos_x[..len];
    let pos_y = &soa.pos_y[..len];
    let size_w = &soa.size_w[..len];
    let size_h = &soa.size_h[..len];
    let uv_rect = &soa.uv_rect[..len];
    let rotation = &soa.rotation[..len];
    let color_rgba = &soa.color_rgba[..len];

    match cull_camera {
        None => {
            // `TrustedLen` extend eliminates per-element capacity checks while preserving existing allocation.
            out.extend((0..len).map(|i| SpriteInstance {
                pos: [pos_x[i], pos_y[i]],
                size: [size_w[i], size_h[i]],
                uv_rect: uv_rect[i],
                rotation: rotation[i],
                color: color_rgba[i],
            }));
        }
        Some(cam) => {
            // Hoist frustum bounds once outside the hot loop (avoids per-sprite viewport/zoom math).
            let bounds = cam.visible_bounds();
            let min_x = bounds.min.x;
            let max_x = bounds.max.x;
            let min_y = bounds.min.y;
            let max_y = bounds.max.y;

            for i in 0..len {
                let x = pos_x[i];
                let y = pos_y[i];
                let w = size_w[i];
                let h = size_h[i];
                let rot = rotation[i];
                let hx = (w * 0.5).abs();
                let hy = (h * 0.5).abs();
                let (ext_x, ext_y) = if rot != 0.0 {
                    let r = (hx * hx + hy * hy).sqrt();
                    (r, r)
                } else {
                    (hx, hy)
                };
                if x + ext_x >= min_x
                    && x - ext_x <= max_x
                    && y + ext_y >= min_y
                    && y - ext_y <= max_y
                {
                    out.push(SpriteInstance {
                        pos: [x, y],
                        size: [w, h],
                        uv_rect: uv_rect[i],
                        rotation: rot,
                        color: color_rgba[i],
                    });
                }
            }
        }
    }

    out.len() as u32
}

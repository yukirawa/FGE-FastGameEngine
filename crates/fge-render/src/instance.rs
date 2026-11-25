use bytemuck::{Pod, Zeroable};
use fge_core::{camera::Camera2D, sim::SpriteSoA};

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
        todo!("SpriteInstance::buffer_layout")
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
    pub fn from_camera(_camera: &Camera2D) -> Self {
        todo!("CameraUniform::from_camera")
    }
}

/// Packs `SpriteSoA` columns into a contiguous `Vec<SpriteInstance>` ready for `Queue::write_buffer`.
///
/// - Clears `out` without dropping its allocated buffer capacity.
/// - When `cull_camera` is `Some(cam)`, skips sprites outside `cam.visible_bounds()` using `cam.is_visible`.
/// - When `cull_camera` is `None`, packs all `soa.len()` sprites in a tight linear loop.
/// - Returns the number of packed instances (`out.len() as u32`).
pub fn pack_soa_instances(
    _soa: &SpriteSoA,
    _cull_camera: Option<&Camera2D>,
    _out: &mut Vec<SpriteInstance>,
) -> u32 {
    todo!("pack_soa_instances")
}

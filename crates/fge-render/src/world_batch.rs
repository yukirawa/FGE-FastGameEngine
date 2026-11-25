use fge_core::{camera::Camera2D, world::EntityWorld};

use crate::{
    context::{FrameRenderStats, GpuRenderer},
    error::RenderError,
    instance::SpriteInstance,
};

/// Packs visible `EntityWorld` entities into `out`, applying optional `Camera2D` frustum culling
/// and sorting by effective depth (`z_order` + normalized `pos_y` when `ENTITY_FLAG_Y_SORT` is set).
pub fn pack_world_instances(
    _world: &EntityWorld,
    _cull_camera: Option<&Camera2D>,
    _out: &mut Vec<SpriteInstance>,
) -> u32 {
    todo!("pack_world_instances")
}

impl GpuRenderer {
    /// Renders an `EntityWorld` in a single instanced GPU draw call.
    pub fn render_world(
        &mut self,
        _camera: &Camera2D,
        _world: &EntityWorld,
        _cull: bool,
        _clear_color: wgpu::Color,
    ) -> Result<FrameRenderStats, RenderError> {
        todo!("GpuRenderer::render_world")
    }
}

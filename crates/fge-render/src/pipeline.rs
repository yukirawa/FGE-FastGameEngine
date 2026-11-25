use crate::instance::{CameraUniform, SpriteInstance};

/// Single-draw-call GPU instanced 2D sprite/quad pipeline.
pub struct SpriteBatchPipeline {
    pub(crate) render_pipeline: wgpu::RenderPipeline,
    pub(crate) camera_buffer: wgpu::Buffer,
    pub(crate) camera_bind_group: wgpu::BindGroup,
    pub(crate) default_tex_bind_group: wgpu::BindGroup,
    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) instance_capacity: usize,
    pub(crate) target_format: wgpu::TextureFormat,
}

impl SpriteBatchPipeline {
    /// Creates the `RenderPipeline`, camera uniform buffer, 1x1 white fallback texture,
    /// and pre-allocated GPU instance buffer (`max(initial_capacity, 1)`).
    pub fn new(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _target_format: wgpu::TextureFormat,
        _initial_capacity: usize,
    ) -> Self {
        todo!("SpriteBatchPipeline::new")
    }

    /// Current GPU instance capacity before buffer reallocation is required.
    pub fn instance_capacity(&self) -> usize {
        self.instance_capacity
    }

    /// Target texture format this pipeline was compiled for.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Uploads `camera` uniform and `instances` slice to the GPU, growing `instance_buffer` if needed.
    pub fn prepare(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _camera: &CameraUniform,
        _instances: &[SpriteInstance],
    ) {
        todo!("SpriteBatchPipeline::prepare")
    }

    /// Binds pipeline state and issues a single `pass.draw(0..4, 0..instance_count)` call when `instance_count > 0`.
    /// Returns the number of draw calls issued (`0` or `1`).
    pub fn render<'a>(&'a self, _pass: &mut wgpu::RenderPass<'a>, _instance_count: u32) -> u32 {
        todo!("SpriteBatchPipeline::render")
    }
}

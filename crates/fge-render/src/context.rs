use fge_core::{camera::Camera2D, sim::SpriteSoA};

use crate::{
    error::RenderError,
    instance::SpriteInstance,
    pipeline::SpriteBatchPipeline,
};

/// Surface presentation policy for windowed mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PresentPolicy {
    /// Uncapped framerate (`AutoNoVsync` / `Immediate` / `Mailbox` fallback to `Fifo`).
    AutoNoVsync,
    /// VSync-locked presentation (`PresentMode::Fifo`).
    Fifo,
}

/// Telemetry returned from rendering a single frame.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct FrameRenderStats {
    pub submitted_instances: u32,
    pub draw_calls: u32,
    pub skipped: bool,
}

pub(crate) enum RenderTarget {
    Windowed {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    },
}

/// High-throughput `wgpu` 2D renderer supporting both OS windows and headless offscreen GPU targets.
pub struct GpuRenderer {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter_info: wgpu::AdapterInfo,
    pub(crate) target: RenderTarget,
    pub(crate) pipeline: SpriteBatchPipeline,
    pub(crate) scratch_instances: Vec<SpriteInstance>,
}

impl GpuRenderer {
    /// Initializes a windowed `wgpu` renderer on `surface_target`.
    pub fn new_windowed(
        _instance_desc: wgpu::InstanceDescriptor,
        _surface_target: impl Into<wgpu::SurfaceTarget<'static>>,
        _width: u32,
        _height: u32,
        _policy: PresentPolicy,
        _initial_capacity: usize,
    ) -> Result<Self, RenderError> {
        todo!("GpuRenderer::new_windowed")
    }

    /// Initializes an offscreen `wgpu` renderer backed by a GPU render target texture.
    pub fn new_offscreen(
        _width: u32,
        _height: u32,
        _initial_capacity: usize,
    ) -> Result<Self, RenderError> {
        todo!("GpuRenderer::new_offscreen")
    }

    /// Reconfigures the surface or recreates the offscreen render target upon resize (`width > 0 && height > 0`).
    pub fn resize(&mut self, _width: u32, _height: u32) -> Result<(), RenderError> {
        todo!("GpuRenderer::resize")
    }

    /// Metadata of the selected GPU adapter.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Returns `true` if backed by an offscreen texture rather than a window surface.
    pub fn is_offscreen(&self) -> bool {
        matches!(self.target, RenderTarget::Offscreen { .. })
    }

    /// Current render target dimensions `(width, height)`.
    pub fn dimensions(&self) -> (u32, u32) {
        todo!("GpuRenderer::dimensions")
    }

    /// Packs `soa` into `scratch_instances`, uploads to GPU, executes 1 instanced draw call,
    /// and presents (or waits on GPU submission when offscreen).
    pub fn render_soa(
        &mut self,
        _camera: &Camera2D,
        _soa: &SpriteSoA,
        _cull: bool,
        _clear_color: wgpu::Color,
    ) -> Result<FrameRenderStats, RenderError> {
        todo!("GpuRenderer::render_soa")
    }
}

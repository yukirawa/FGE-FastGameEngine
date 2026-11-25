use fge_core::{camera::Camera2D, sim::SpriteSoA};

use crate::{
    error::RenderError,
    instance::{pack_soa_instances, CameraUniform, SpriteInstance},
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
        _texture: wgpu::Texture,
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

fn validate_resolution(width: u32, height: u32) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::InvalidResolution { width, height });
    }
    Ok(())
}

fn select_present_mode(
    policy: PresentPolicy,
    supported: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    match policy {
        PresentPolicy::Fifo => wgpu::PresentMode::Fifo,
        PresentPolicy::AutoNoVsync => {
            if supported.contains(&wgpu::PresentMode::AutoNoVsync) {
                wgpu::PresentMode::AutoNoVsync
            } else if supported.contains(&wgpu::PresentMode::Immediate) {
                wgpu::PresentMode::Immediate
            } else if supported.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Fifo
            }
        }
    }
}

fn create_offscreen_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fge_offscreen_color_target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn encode_and_submit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &SpriteBatchPipeline,
    view: &wgpu::TextureView,
    clear_color: wgpu::Color,
    instance_count: u32,
) -> u32 {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("fge_frame_command_encoder"),
    });
    let draw_calls = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fge_sprite_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pipeline.render(&mut pass, instance_count)
    };
    queue.submit(core::iter::once(encoder.finish()));
    draw_calls
}

fn request_adapter_with_fallback(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, wgpu::RequestAdapterError> {
    match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface,
        ..Default::default()
    })) {
        Ok(adapter) => Ok(adapter),
        Err(_) => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: true,
            compatible_surface,
            ..Default::default()
        })),
    }
}

impl GpuRenderer {
    /// Initializes a windowed `wgpu` renderer on `surface_target`.
    pub fn new_windowed(
        instance_desc: wgpu::InstanceDescriptor,
        surface_target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        policy: PresentPolicy,
        initial_capacity: usize,
    ) -> Result<Self, RenderError> {
        validate_resolution(width, height)?;

        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(surface_target)?;
        let adapter = request_adapter_with_fallback(&instance, Some(&surface))?;

        let adapter_info = adapter.get_info();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err(RenderError::UnsupportedSurface);
        }

        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let present_mode = select_present_mode(policy, &caps.present_modes);
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let pipeline = SpriteBatchPipeline::new(&device, &queue, format, initial_capacity);
        let scratch_instances = Vec::with_capacity(initial_capacity);

        Ok(Self {
            device,
            queue,
            adapter_info,
            target: RenderTarget::Windowed { surface, config },
            pipeline,
            scratch_instances,
        })
    }

    /// Initializes an offscreen `wgpu` renderer backed by a GPU render target texture.
    pub fn new_offscreen(
        width: u32,
        height: u32,
        initial_capacity: usize,
    ) -> Result<Self, RenderError> {
        validate_resolution(width, height)?;

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = request_adapter_with_fallback(&instance, None)?;

        let adapter_info = adapter.get_info();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let (texture, view) = create_offscreen_target(&device, width, height, format);

        let pipeline = SpriteBatchPipeline::new(&device, &queue, format, initial_capacity);
        let scratch_instances = Vec::with_capacity(initial_capacity);

        Ok(Self {
            device,
            queue,
            adapter_info,
            target: RenderTarget::Offscreen {
                _texture: texture,
                view,
                format,
                width,
                height,
            },
            pipeline,
            scratch_instances,
        })
    }

    /// Reconfigures the surface or recreates the offscreen render target upon resize (`width > 0 && height > 0`).
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        validate_resolution(width, height)?;

        match &mut self.target {
            RenderTarget::Windowed { surface, config } => {
                config.width = width;
                config.height = height;
                surface.configure(&self.device, config);
            }
            RenderTarget::Offscreen {
                _texture: texture,
                view,
                format,
                width: target_w,
                height: target_h,
            } => {
                let (new_texture, new_view) =
                    create_offscreen_target(&self.device, width, height, *format);
                *texture = new_texture;
                *view = new_view;
                *target_w = width;
                *target_h = height;
            }
        }

        Ok(())
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
        match &self.target {
            RenderTarget::Windowed { config, .. } => (config.width, config.height),
            RenderTarget::Offscreen { width, height, .. } => (*width, *height),
        }
    }

    /// Packs `soa` into `scratch_instances`, uploads to GPU, executes 1 instanced draw call,
    /// and presents (or waits on GPU submission when offscreen).
    pub fn render_soa(
        &mut self,
        camera: &Camera2D,
        soa: &SpriteSoA,
        cull: bool,
        clear_color: wgpu::Color,
    ) -> Result<FrameRenderStats, RenderError> {
        let cull_camera = if cull { Some(camera) } else { None };

        match &mut self.target {
            RenderTarget::Windowed { surface, config } => {
                let (frame, suboptimal) = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
                    wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        return Ok(FrameRenderStats {
                            skipped: true,
                            ..Default::default()
                        });
                    }
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        surface.configure(&self.device, config);
                        return Ok(FrameRenderStats {
                            skipped: true,
                            ..Default::default()
                        });
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        return Err(RenderError::SurfaceFatal("validation"));
                    }
                };

                let submitted_instances =
                    pack_soa_instances(soa, cull_camera, &mut self.scratch_instances);
                let camera_uniform = CameraUniform::from_camera(camera);

                let draw_calls = {
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    self.pipeline.prepare(
                        &self.device,
                        &self.queue,
                        &camera_uniform,
                        &self.scratch_instances,
                    );
                    encode_and_submit(
                        &self.device,
                        &self.queue,
                        &self.pipeline,
                        &view,
                        clear_color,
                        submitted_instances,
                    )
                };
                self.queue.present(frame);
                if suboptimal {
                    surface.configure(&self.device, config);
                }
                Ok(FrameRenderStats {
                    submitted_instances,
                    draw_calls,
                    skipped: false,
                })
            }
            RenderTarget::Offscreen { view, .. } => {
                let submitted_instances =
                    pack_soa_instances(soa, cull_camera, &mut self.scratch_instances);
                let camera_uniform = CameraUniform::from_camera(camera);

                self.pipeline.prepare(
                    &self.device,
                    &self.queue,
                    &camera_uniform,
                    &self.scratch_instances,
                );
                let draw_calls = encode_and_submit(
                    &self.device,
                    &self.queue,
                    &self.pipeline,
                    view,
                    clear_color,
                    submitted_instances,
                );
                self.device.poll(wgpu::wgt::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })?;
                Ok(FrameRenderStats {
                    submitted_instances,
                    draw_calls,
                    skipped: false,
                })
            }
        }
    }
}

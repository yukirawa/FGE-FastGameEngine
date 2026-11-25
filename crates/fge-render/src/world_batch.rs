use fge_core::{
    camera::Camera2D,
    world::{EntityWorld, ENTITY_FLAG_VISIBLE, ENTITY_FLAG_Y_SORT},
};
use glam::Vec2;

use crate::{
    context::{FrameRenderStats, GpuRenderer, RenderTarget},
    error::RenderError,
    instance::{CameraUniform, SpriteInstance},
    pipeline::SpriteBatchPipeline,
};

fn encode_and_submit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &SpriteBatchPipeline,
    view: &wgpu::TextureView,
    clear_color: wgpu::Color,
    instance_count: u32,
) -> u32 {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("fge_world_frame_command_encoder"),
    });
    let draw_calls = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fge_world_sprite_render_pass"),
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

/// Depth sorting strategy for visible entities in [`pack_world_instances_with_mode`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DepthSortMode {
    /// Fast single-float sort key (`z_order + pos_y * 1e-4`), suited for standard 2D scenes.
    #[default]
    FastFloat,
    /// Composite `(z_order.floor() as i32, pos_y_key, z_order)` key that preserves exact Y-sort
    /// ordering even at open-world coordinates exceeding 1,000,000 px.
    CompositeLayerY,
}

/// Packs visible `EntityWorld` entities into `out`, applying optional `Camera2D` frustum culling
/// and sorting by effective depth using [`DepthSortMode::FastFloat`].
pub fn pack_world_instances(
    world: &EntityWorld,
    cull_camera: Option<&Camera2D>,
    out: &mut Vec<SpriteInstance>,
) -> u32 {
    pack_world_instances_with_mode(world, cull_camera, DepthSortMode::FastFloat, out)
}

/// Packs visible `EntityWorld` entities into `out`, applying optional `Camera2D` frustum culling
/// and sorting by effective depth using the requested [`DepthSortMode`].
pub fn pack_world_instances_with_mode(
    world: &EntityWorld,
    cull_camera: Option<&Camera2D>,
    mode: DepthSortMode,
    out: &mut Vec<SpriteInstance>,
) -> u32 {
    out.clear();

    let len = world.len();
    if len == 0 {
        return 0;
    }

    let pos_x = &world.pos_x[..len];
    let pos_y = &world.pos_y[..len];
    let size_w = &world.size_w[..len];
    let size_h = &world.size_h[..len];
    let uv_rect = &world.uv_rect[..len];
    let rotation = &world.rotation[..len];
    let color_rgba = &world.color_rgba[..len];
    let flags = &world.flags[..len];
    let z_order = &world.z_order[..len];

    let needs_sort = (0..len).any(|i| {
        let f = flags[i];
        (f & ENTITY_FLAG_VISIBLE) != 0 && (z_order[i] != 0.0 || (f & ENTITY_FLAG_Y_SORT) != 0)
    });

    if !needs_sort {
        if out.capacity() < len {
            out.reserve(len);
        }
        for i in 0..len {
            if (flags[i] & ENTITY_FLAG_VISIBLE) == 0 {
                continue;
            }
            let x = pos_x[i];
            let y = pos_y[i];
            let w = size_w[i];
            let h = size_h[i];
            let rot = rotation[i];
            if let Some(cam) = cull_camera {
                if !cam.is_visible(Vec2::new(x, y), Vec2::new(w * 0.5, h * 0.5), rot != 0.0) {
                    continue;
                }
            }
            out.push(SpriteInstance {
                pos: [x, y],
                size: [w, h],
                uv_rect: uv_rect[i],
                rotation: rot,
                color: color_rgba[i],
            });
        }
        return out.len() as u32;
    }

    match mode {
        DepthSortMode::FastFloat => {
            let mut keyed: Vec<(f32, SpriteInstance)> = Vec::with_capacity(len);
            for i in 0..len {
                let f = flags[i];
                if (f & ENTITY_FLAG_VISIBLE) == 0 {
                    continue;
                }
                let x = pos_x[i];
                let y = pos_y[i];
                let w = size_w[i];
                let h = size_h[i];
                let rot = rotation[i];
                if let Some(cam) = cull_camera {
                    if !cam.is_visible(Vec2::new(x, y), Vec2::new(w * 0.5, h * 0.5), rot != 0.0) {
                        continue;
                    }
                }
                let sort_key = if (f & ENTITY_FLAG_Y_SORT) != 0 {
                    z_order[i] + y * 1e-4
                } else {
                    z_order[i]
                };
                keyed.push((
                    sort_key,
                    SpriteInstance {
                        pos: [x, y],
                        size: [w, h],
                        uv_rect: uv_rect[i],
                        rotation: rot,
                        color: color_rgba[i],
                    },
                ));
            }

            keyed.sort_by(|a, b| a.0.total_cmp(&b.0));

            if out.capacity() < keyed.len() {
                out.reserve(keyed.len());
            }
            out.extend(keyed.into_iter().map(|(_, inst)| inst));
        }
        DepthSortMode::CompositeLayerY => {
            let mut keyed: Vec<(i32, f32, f32, SpriteInstance)> = Vec::with_capacity(len);
            for i in 0..len {
                let f = flags[i];
                if (f & ENTITY_FLAG_VISIBLE) == 0 {
                    continue;
                }
                let x = pos_x[i];
                let y = pos_y[i];
                let w = size_w[i];
                let h = size_h[i];
                let rot = rotation[i];
                if let Some(cam) = cull_camera {
                    if !cam.is_visible(Vec2::new(x, y), Vec2::new(w * 0.5, h * 0.5), rot != 0.0) {
                        continue;
                    }
                }
                let z = z_order[i];
                let layer = z.floor() as i32;
                let pos_y_key = if (f & ENTITY_FLAG_Y_SORT) != 0 {
                    y
                } else {
                    0.0
                };
                keyed.push((
                    layer,
                    pos_y_key,
                    z,
                    SpriteInstance {
                        pos: [x, y],
                        size: [w, h],
                        uv_rect: uv_rect[i],
                        rotation: rot,
                        color: color_rgba[i],
                    },
                ));
            }

            keyed.sort_by(|a, b| {
                a.0.cmp(&b.0)
                    .then_with(|| a.1.total_cmp(&b.1))
                    .then_with(|| a.2.total_cmp(&b.2))
            });

            if out.capacity() < keyed.len() {
                out.reserve(keyed.len());
            }
            out.extend(keyed.into_iter().map(|(_, _, _, inst)| inst));
        }
    }

    out.len() as u32
}

impl GpuRenderer {
    /// Renders an `EntityWorld` in a single instanced GPU draw call.
    pub fn render_world(
        &mut self,
        camera: &Camera2D,
        world: &EntityWorld,
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

                let submitted_instances = pack_world_instances_with_mode(
                    world,
                    cull_camera,
                    self.depth_sort_mode,
                    &mut self.scratch_instances,
                );
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
                let submitted_instances = pack_world_instances_with_mode(
                    world,
                    cull_camera,
                    self.depth_sort_mode,
                    &mut self.scratch_instances,
                );
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

use core::f32::consts::FRAC_PI_4;

use fge_core::{
    camera::Camera2D,
    ui::{truncate_utf8_chars, UiCommandQueue, UiDrawCmd},
    world::{EntityWorld, ENTITY_FLAG_VISIBLE, ENTITY_FLAG_Y_SORT},
};
use glam::Vec2;

use crate::{
    atlas::{DynamicAtlas, SOLID_WHITE_UV},
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

fn encode_and_submit_world_and_ui(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &SpriteBatchPipeline,
    view: &wgpu::TextureView,
    clear_color: wgpu::Color,
    world_count: u32,
    ui_count: u32,
) -> u32 {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("fge_world_ui_frame_command_encoder"),
    });
    let draw_calls = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fge_world_ui_render_pass"),
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
        pipeline.render_world_and_ui(&mut pass, world_count, ui_count)
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

#[inline]
fn push_solid_rect(out: &mut Vec<SpriteInstance>, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
        return;
    }
    if w <= 0.0 || h <= 0.0 || color[3] == 0 {
        return;
    }
    out.push(SpriteInstance {
        pos: [x + w * 0.5, y + h * 0.5],
        size: [w, h],
        uv_rect: SOLID_WHITE_UV,
        rotation: 0.0,
        color,
    });
}

fn push_bordered_panel(
    out: &mut Vec<SpriteInstance>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    bg_color: [u8; 4],
    border_color: [u8; 4],
    border_px: f32,
) {
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return;
    }
    let b = if border_px.is_finite() && border_px > 0.0 && border_color[3] > 0 {
        border_px.min(w * 0.5).min(h * 0.5)
    } else {
        0.0
    };

    if b > 0.0 {
        // Interior fill (does not overlap border so translucent panels stay crisp).
        push_solid_rect(out, x + b, y + b, w - b * 2.0, h - b * 2.0, bg_color);
        // Top, bottom, left, right border strips.
        push_solid_rect(out, x, y, w, b, border_color);
        push_solid_rect(out, x, y + h - b, w, b, border_color);
        let side_h = (h - b * 2.0).max(0.0);
        push_solid_rect(out, x, y + b, b, side_h, border_color);
        push_solid_rect(out, x + w - b, y + b, b, side_h, border_color);
    } else {
        push_solid_rect(out, x, y, w, h, bg_color);
    }
}

fn push_border_outline(
    out: &mut Vec<SpriteInstance>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [u8; 4],
    thickness: f32,
) {
    push_bordered_panel(out, x, y, w, h, [0, 0, 0, 0], color, thickness);
}

/// Tessellates `UiCommandQueue` into world-space text instances (appended to `world_out`)
/// and screen-space UI panels, bars, RPG dialog boxes, and text (written into `ui_out`).
pub fn pack_ui_commands(
    atlas: &mut DynamicAtlas,
    ui: &UiCommandQueue,
    world_out: &mut Vec<SpriteInstance>,
    ui_out: &mut Vec<SpriteInstance>,
) {
    ui_out.clear();
    if ui.is_empty() {
        return;
    }

    for cmd in ui.commands() {
        match cmd {
            UiDrawCmd::WorldText {
                world_x,
                world_y,
                font_size,
                color,
                text_span,
            } => {
                if !world_x.is_finite() || !world_y.is_finite() {
                    continue;
                }
                let text = ui.resolve_span(text_span.clone());
                if !text.is_empty() {
                    atlas.layout_text(
                        text,
                        *world_x,
                        *world_y,
                        *font_size,
                        *color,
                        None,
                        None,
                        true,
                        true,
                        world_out,
                    );
                }
            }
            UiDrawCmd::Panel {
                x,
                y,
                w,
                h,
                bg_color,
                border_color,
                border_px,
            } => {
                push_bordered_panel(
                    ui_out,
                    *x,
                    *y,
                    *w,
                    *h,
                    *bg_color,
                    *border_color,
                    *border_px,
                );
            }
            UiDrawCmd::Bar {
                x,
                y,
                w,
                h,
                ratio,
                fill_color,
                bg_color,
            } => {
                let r = if ratio.is_finite() {
                    ratio.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                push_solid_rect(ui_out, *x, *y, *w, *h, *bg_color);
                if r > 0.0 {
                    push_solid_rect(ui_out, *x, *y, *w * r, *h, *fill_color);
                }
            }
            UiDrawCmd::ScreenText {
                x,
                y,
                font_size,
                color,
                text_span,
            } => {
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let text = ui.resolve_span(text_span.clone());
                if !text.is_empty() {
                    atlas.layout_text(
                        text,
                        *x,
                        *y,
                        *font_size,
                        *color,
                        None,
                        None,
                        false,
                        true,
                        ui_out,
                    );
                }
            }
            UiDrawCmd::DialogBox {
                x,
                y,
                w,
                h,
                speaker_span,
                message_span,
                visible_chars,
            } => {
                if !x.is_finite()
                    || !y.is_finite()
                    || !w.is_finite()
                    || !h.is_finite()
                    || *w <= 0.0
                    || *h <= 0.0
                {
                    continue;
                }

                // 1. Soft drop shadow behind dialog box.
                push_solid_rect(ui_out, *x + 4.0, *y + 4.0, *w, *h, [0, 0, 0, 128]);

                // 2. Classic JRPG midnight-navy window frame with warm gold outer border + inner trim.
                push_bordered_panel(
                    ui_out,
                    *x,
                    *y,
                    *w,
                    *h,
                    [14, 20, 48, 236],
                    [232, 214, 162, 255],
                    3.0,
                );
                if *w > 16.0 && *h > 16.0 {
                    push_border_outline(
                        ui_out,
                        *x + 5.0,
                        *y + 5.0,
                        *w - 10.0,
                        *h - 10.0,
                        [96, 130, 204, 185],
                        1.0,
                    );
                }

                // 3. Floating speaker name plate above top-left corner when speaker is present.
                let speaker = ui.resolve_span(speaker_span.clone());
                let speaker_font = (*h * 0.14).clamp(15.0, 21.0);
                if !speaker.is_empty() {
                    let speaker_w = atlas.measure_line_width(speaker, speaker_font);
                    let plate_h = (speaker_font + 12.0).clamp(26.0, 36.0);
                    let plate_w = (speaker_w + 28.0).clamp(96.0, (*w - 24.0).max(96.0));
                    let plate_x = *x + 18.0;
                    let plate_y = (*y - plate_h + 4.0).max(4.0);
                    push_bordered_panel(
                        ui_out,
                        plate_x,
                        plate_y,
                        plate_w,
                        plate_h,
                        [24, 38, 82, 248],
                        [242, 210, 118, 255],
                        2.0,
                    );
                    atlas.layout_text(
                        speaker,
                        plate_x + 14.0,
                        plate_y + (plate_h - speaker_font) * 0.45,
                        speaker_font,
                        [255, 234, 134, 255],
                        Some((plate_w - 24.0).max(32.0)),
                        Some(plate_h),
                        false,
                        true,
                        ui_out,
                    );
                }

                // 4. Wrapped typewriter-sliced message text.
                let full_msg = ui.resolve_span(message_span.clone());
                let shown_msg = truncate_utf8_chars(full_msg, *visible_chars);
                if !shown_msg.is_empty() {
                    let pad_x = 22.0f32;
                    let pad_y = 18.0f32;
                    let msg_font = (*h * 0.16).clamp(16.0, 24.0);
                    let max_w = (*w - pad_x * 2.0).max(40.0);
                    let max_h = (*h - pad_y * 2.0).max(20.0);
                    atlas.layout_text(
                        shown_msg,
                        *x + pad_x,
                        *y + pad_y,
                        msg_font,
                        [246, 248, 255, 255],
                        Some(max_w),
                        Some(max_h),
                        false,
                        true,
                        ui_out,
                    );
                }

                // 5. Classic RPG continue-prompt diamond indicator in the bottom-right corner.
                if *w >= 48.0 && *h >= 40.0 {
                    ui_out.push(SpriteInstance {
                        pos: [*x + *w - 22.0, *y + *h - 18.0],
                        size: [9.0, 9.0],
                        uv_rect: SOLID_WHITE_UV,
                        rotation: FRAC_PI_4,
                        color: [255, 224, 112, 255],
                    });
                }
            }
        }
    }
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
        self.pipeline.upload_atlas_dirty(&self.queue, &mut self.atlas);

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

    /// Renders `world` + world-space text under `camera` followed by screen-space UI (`Panel`,
    /// `Bar`, `DialogBox`, `ScreenText`) under a top-left viewport projection inside a single
    /// `wgpu::RenderPass` (at most 2 draw calls; 1 draw call when no screen-space UI is emitted).
    pub fn render_world_and_ui(
        &mut self,
        camera: &Camera2D,
        world: &EntityWorld,
        ui: &UiCommandQueue,
        cull: bool,
        clear_color: wgpu::Color,
    ) -> Result<FrameRenderStats, RenderError> {
        let cull_camera = if cull { Some(camera) } else { None };
        let (target_w, target_h) = self.dimensions();
        let screen_camera = Camera2D::from_top_left_viewport(target_w as f32, target_h as f32)?;
        let world_camera_uniform = CameraUniform::from_camera(camera);
        let ui_camera_uniform = CameraUniform::from_camera(&screen_camera);

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

                pack_world_instances_with_mode(
                    world,
                    cull_camera,
                    self.depth_sort_mode,
                    &mut self.scratch_instances,
                );
                pack_ui_commands(
                    &mut self.atlas,
                    ui,
                    &mut self.scratch_instances,
                    &mut self.ui_scratch_instances,
                );
                self.pipeline.upload_atlas_dirty(&self.queue, &mut self.atlas);

                let world_count = self.scratch_instances.len() as u32;
                let ui_count = self.ui_scratch_instances.len() as u32;

                let draw_calls = {
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    self.pipeline.prepare_world_and_ui(
                        &self.device,
                        &self.queue,
                        &world_camera_uniform,
                        &self.scratch_instances,
                        &ui_camera_uniform,
                        &self.ui_scratch_instances,
                    );
                    encode_and_submit_world_and_ui(
                        &self.device,
                        &self.queue,
                        &self.pipeline,
                        &view,
                        clear_color,
                        world_count,
                        ui_count,
                    )
                };
                self.queue.present(frame);
                if suboptimal {
                    surface.configure(&self.device, config);
                }
                Ok(FrameRenderStats {
                    submitted_instances: world_count.saturating_add(ui_count),
                    draw_calls,
                    skipped: false,
                })
            }
            RenderTarget::Offscreen { view, .. } => {
                pack_world_instances_with_mode(
                    world,
                    cull_camera,
                    self.depth_sort_mode,
                    &mut self.scratch_instances,
                );
                pack_ui_commands(
                    &mut self.atlas,
                    ui,
                    &mut self.scratch_instances,
                    &mut self.ui_scratch_instances,
                );
                self.pipeline.upload_atlas_dirty(&self.queue, &mut self.atlas);

                let world_count = self.scratch_instances.len() as u32;
                let ui_count = self.ui_scratch_instances.len() as u32;

                self.pipeline.prepare_world_and_ui(
                    &self.device,
                    &self.queue,
                    &world_camera_uniform,
                    &self.scratch_instances,
                    &ui_camera_uniform,
                    &self.ui_scratch_instances,
                );
                let draw_calls = encode_and_submit_world_and_ui(
                    &self.device,
                    &self.queue,
                    &self.pipeline,
                    view,
                    clear_color,
                    world_count,
                    ui_count,
                );
                self.device.poll(wgpu::wgt::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })?;
                Ok(FrameRenderStats {
                    submitted_instances: world_count.saturating_add(ui_count),
                    draw_calls,
                    skipped: false,
                })
            }
        }
    }
}

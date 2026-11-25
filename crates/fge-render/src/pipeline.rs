use crate::{
    atlas::{DynamicAtlas, ATLAS_SIZE},
    instance::{CameraUniform, SpriteInstance},
};

/// Single-draw-call GPU instanced 2D sprite/quad pipeline backed by a dynamic 2048x2048 atlas.
pub struct SpriteBatchPipeline {
    pub(crate) render_pipeline: wgpu::RenderPipeline,
    pub(crate) camera_buffer: wgpu::Buffer,
    pub(crate) camera_bind_group: wgpu::BindGroup,
    pub(crate) ui_camera_buffer: wgpu::Buffer,
    pub(crate) ui_camera_bind_group: wgpu::BindGroup,
    pub(crate) atlas_texture: wgpu::Texture,
    pub(crate) atlas_tex_bind_group: wgpu::BindGroup,
    pub(crate) atlas_upload_scratch: Vec<u8>,
    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) instance_capacity: usize,
    pub(crate) target_format: wgpu::TextureFormat,
}

impl SpriteBatchPipeline {
    fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        let byte_size = (capacity * core::mem::size_of::<SpriteInstance>()) as u64;
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fge_sprite_instance_buffer"),
            size: byte_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required > self.instance_capacity {
            let new_capacity = required
                .next_power_of_two()
                .max(self.instance_capacity.saturating_mul(2));
            self.instance_buffer = Self::create_instance_buffer(device, new_capacity);
            self.instance_capacity = new_capacity;
        }
    }

    /// Creates the `RenderPipeline`, world + UI camera uniform buffers, 2048x2048 dynamic atlas
    /// GPU texture (with top-left 8x8 white block initialized), and pre-allocated instance buffer.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        initial_capacity: usize,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fge_camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            core::mem::size_of::<CameraUniform>() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fge_texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fge_camera_uniform_buffer"),
            size: core::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fge_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let ui_camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fge_ui_camera_uniform_buffer"),
            size: core::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ui_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fge_ui_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ui_camera_buffer.as_entire_binding(),
            }],
        });

        let atlas_tex_size = wgpu::Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        };
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fge_dynamic_atlas_texture"),
            size: atlas_tex_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Seed the top-left 8x8 reserved opaque white block immediately so untextured
        // sprites and UI primitives sample pure white even before the first atlas flush.
        let white_block = [255u8; 8 * 8 * 4];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &white_block,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8 * 4),
                rows_per_image: Some(8),
            },
            wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
        );

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fge_atlas_linear_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let atlas_tex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fge_atlas_texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let instance_capacity = initial_capacity.max(1);
        let instance_buffer = Self::create_instance_buffer(device, instance_capacity);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fge_sprite_pipeline_layout"),
            bind_group_layouts: &[
                Some(&camera_bind_group_layout),
                Some(&texture_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fge_sprite_render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(SpriteInstance::buffer_layout())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            ui_camera_buffer,
            ui_camera_bind_group,
            atlas_texture,
            atlas_tex_bind_group,
            atlas_upload_scratch: Vec::new(),
            instance_buffer,
            instance_capacity,
            target_format,
        }
    }

    /// Current GPU instance capacity before buffer reallocation is required.
    pub fn instance_capacity(&self) -> usize {
        self.instance_capacity
    }

    /// Target texture format this pipeline was compiled for.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Uploads any pending dirty sub-rectangle from `atlas` into the 2048x2048 GPU atlas texture.
    pub fn upload_atlas_dirty(&mut self, queue: &wgpu::Queue, atlas: &mut DynamicAtlas) {
        let Some([x, y, w, h]) = atlas.take_dirty_rect() else {
            return;
        };
        if w == 0 || h == 0 || x >= atlas.width || y >= atlas.height {
            return;
        }
        let clamped_w = w.min(atlas.width - x);
        let clamped_h = h.min(atlas.height - y);
        if clamped_w == 0 || clamped_h == 0 {
            return;
        }

        let copy_dst = wgpu::TexelCopyTextureInfo {
            texture: &self.atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        };
        let copy_size = wgpu::Extent3d {
            width: clamped_w,
            height: clamped_h,
            depth_or_array_layers: 1,
        };

        if x == 0 && clamped_w == atlas.width {
            let start = (y * atlas.width * 4) as usize;
            let end = ((y + clamped_h) * atlas.width * 4) as usize;
            queue.write_texture(
                copy_dst,
                &atlas.pixels[start..end],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(clamped_w * 4),
                    rows_per_image: Some(clamped_h),
                },
                copy_size,
            );
        } else {
            let row_bytes = (clamped_w * 4) as usize;
            let total_bytes = row_bytes * (clamped_h as usize);
            self.atlas_upload_scratch.clear();
            if self.atlas_upload_scratch.capacity() < total_bytes {
                self.atlas_upload_scratch
                    .reserve(total_bytes - self.atlas_upload_scratch.capacity());
            }
            for row in 0..clamped_h {
                let src_start = (((y + row) * atlas.width + x) * 4) as usize;
                let src_end = src_start + row_bytes;
                self.atlas_upload_scratch
                    .extend_from_slice(&atlas.pixels[src_start..src_end]);
            }
            queue.write_texture(
                copy_dst,
                &self.atlas_upload_scratch,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(clamped_w * 4),
                    rows_per_image: Some(clamped_h),
                },
                copy_size,
            );
        }
    }

    /// Uploads `camera` uniform and `instances` slice to the GPU, growing `instance_buffer` if needed.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &CameraUniform,
        instances: &[SpriteInstance],
    ) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera));
        self.ensure_instance_capacity(device, instances.len());

        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }
    }

    /// Uploads both world and screen-space UI camera uniforms and packs `world_instances`
    /// followed by `ui_instances` into the shared GPU instance buffer.
    pub fn prepare_world_and_ui(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world_camera: &CameraUniform,
        world_instances: &[SpriteInstance],
        ui_camera: &CameraUniform,
        ui_instances: &[SpriteInstance],
    ) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(world_camera));
        queue.write_buffer(&self.ui_camera_buffer, 0, bytemuck::bytes_of(ui_camera));

        let total = world_instances.len().saturating_add(ui_instances.len());
        self.ensure_instance_capacity(device, total);

        if !world_instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(world_instances),
            );
        }
        if !ui_instances.is_empty() {
            let ui_offset =
                (world_instances.len() * core::mem::size_of::<SpriteInstance>()) as u64;
            queue.write_buffer(
                &self.instance_buffer,
                ui_offset,
                bytemuck::cast_slice(ui_instances),
            );
        }
    }

    /// Binds pipeline state and issues a single `pass.draw(0..4, 0..instance_count)` call when `instance_count > 0`.
    /// Returns the number of draw calls issued (`0` or `1`).
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, instance_count: u32) -> u32 {
        if instance_count == 0 {
            return 0;
        }
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.atlas_tex_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..4, 0..instance_count);
        1
    }

    /// Issues up to 2 instanced draws inside a single `wgpu::RenderPass`:
    /// batch 1 (`world_count` instances) under `camera_bind_group`, and
    /// batch 2 (`ui_count` instances) under `ui_camera_bind_group`.
    pub fn render_world_and_ui<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        world_count: u32,
        ui_count: u32,
    ) -> u32 {
        if world_count == 0 && ui_count == 0 {
            return 0;
        }

        let stride = core::mem::size_of::<SpriteInstance>() as u64;
        let world_bytes = (world_count as u64) * stride;
        let ui_bytes = (ui_count as u64) * stride;
        let mut draw_calls = 0u32;

        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(1, &self.atlas_tex_bind_group, &[]);

        if world_count > 0 {
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(0..world_bytes));
            pass.draw(0..4, 0..world_count);
            draw_calls += 1;
        }

        if ui_count > 0 {
            pass.set_bind_group(0, &self.ui_camera_bind_group, &[]);
            pass.set_vertex_buffer(
                0,
                self.instance_buffer.slice(world_bytes..(world_bytes + ui_bytes)),
            );
            pass.draw(0..4, 0..ui_count);
            draw_calls += 1;
        }

        draw_calls
    }
}

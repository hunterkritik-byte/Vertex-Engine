//! GPU renderer for Vertex Engine.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const CUBE_VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0,-1.0, 1.0], color: [1.0,0.0,0.0] , normal: [-1.0,-1.0,1.0], uv: [0.0,1.0] },
    Vertex { position: [ 1.0,-1.0, 1.0], color: [0.0,1.0,0.0] , normal: [1.0,-1.0,1.0], uv: [1.0,1.0] },
    Vertex { position: [ 1.0, 1.0, 1.0], color: [0.0,0.0,1.0] , normal: [1.0,1.0,1.0], uv: [1.0,0.0] },
    Vertex { position: [-1.0, 1.0, 1.0], color: [1.0,1.0,0.0] , normal: [-1.0,1.0,1.0], uv: [0.0,0.0] },
    Vertex { position: [-1.0,-1.0,-1.0], color: [1.0,0.0,1.0] , normal: [-1.0,-1.0,-1.0], uv: [1.0,1.0] },
    Vertex { position: [ 1.0,-1.0,-1.0], color: [0.0,1.0,1.0] , normal: [1.0,-1.0,-1.0], uv: [0.0,1.0] },
    Vertex { position: [ 1.0, 1.0,-1.0], color: [1.0,1.0,1.0] , normal: [1.0,1.0,-1.0], uv: [0.0,0.0] },
    Vertex { position: [-1.0, 1.0,-1.0], color: [0.2,0.2,0.2] , normal: [-1.0,1.0,-1.0], uv: [1.0,0.0] },
];

pub const CUBE_INDICES: &[u16] = &[
    0,1,2, 2,3,0, 1,5,6, 6,2,1, 5,4,7, 7,6,5,
    4,0,3, 3,7,4, 3,2,6, 6,7,3, 4,5,1, 1,0,4,
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    albedo: [f32; 4],
    light_direction: [f32; 4],
    light_color_intensity: [f32; 4],
    pbr: [f32; 4],
}

pub struct Renderer<'window> {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
}

impl<'window> Renderer<'window> {
    pub async fn new(window: &'window winit::window::Window) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)
            .map_err(|e| format!("failed to create surface: {e}"))?;
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.map_err(|e| format!("failed to find GPU adapter: {e}"))?;
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("vertex-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }).await.map_err(|e| format!("failed to create GPU device: {e}"))?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = *caps.formats.first().ok_or("GPU surface exposes no formats")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes.iter().copied()
                .find(|m| *m == wgpu::PresentMode::Fifo)
                .unwrap_or(wgpu::PresentMode::AutoVsync),
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vertex-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(std::num::NonZeroU64::new(
                        std::mem::size_of::<CameraUniform>() as u64
                    ).unwrap()),
                },
                count: None,
            }],
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material-texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false
                }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("default-white-texture"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &white_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255,255,255,255],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("default-material-texture"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vertex-pipeline-layout"),
            bind_group_layouts: &[&camera_layout, &texture_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vertex-3d-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format, blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })], compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-vertex-buffer"),
            contents: bytemuck::cast_slice(CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-index-buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let depth_view = Self::create_depth_view(&device, &config);
        Ok(Self {
            instance, surface, device, queue, config, pipeline,
            vertex_buffer, index_buffer, num_indices: CUBE_INDICES.len() as u32,
            camera_buffer, camera_bind_group, texture_layout, texture_bind_group, depth_view,
        })
    }

    fn create_depth_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vertex-depth-buffer"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Decode a PNG/JPEG from disk and upload it to the GPU.
    /// The returned bind group can be retained by the editor and reused per draw.
    pub fn load_texture(&self, path: &std::path::Path) -> Result<wgpu::BindGroup, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("failed to read texture {}: {e}", path.display()))?;
        let decoded = image::load_from_memory(&bytes)
            .map_err(|e| format!("failed to decode texture {}: {e}", path.display()))?
            .to_rgba8();
        let (width, height) = decoded.dimensions();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vertex-material-texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &decoded,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Ok(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("loaded-material-texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        }))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth_view = Self::create_depth_view(&self.device, &self.config);
    }

    pub fn render_to_view(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        view_proj: Mat4,
        model: Mat4,
        albedo: [f32; 4],
        light_direction: [f32; 3],
        light_color: [f32; 3],
        light_intensity: f32,
        metallic: f32,
        roughness: f32,
        texture_bind_group: Option<&wgpu::BindGroup>,
    ) {
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            albedo,
            light_direction: [light_direction[0], light_direction[1], light_direction[2], 0.0],
            light_color_intensity: [light_color[0], light_color[1], light_color[2], light_intensity],
            pbr: [metallic, roughness, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vertex-3d-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.03, g: 0.03, b: 0.04, a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, texture_bind_group.unwrap_or(&self.texture_bind_group), &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }

    pub fn render(&mut self, view_proj: Mat4) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vertex-render-encoder"),
        });
        self.render_to_view(&mut encoder, &view, view_proj, Mat4::IDENTITY, [0.8,0.8,0.85,1.0], [-0.4,-1.0,-0.5], [1.0,0.95,0.9], 2.0);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

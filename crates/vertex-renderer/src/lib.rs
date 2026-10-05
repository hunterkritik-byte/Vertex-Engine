//! GPU rendering foundation for Vertex Engine.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.7, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.7, -0.7, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.7, -0.7, 0.0], color: [0.0, 0.0, 1.0] },
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

pub struct Renderer {
    pub instance: wgpu::Instance,
    pub surface: Option<wgpu::Surface<'static>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            instance: wgpu::Instance::default(),
            surface: None,
        }
    }

    pub fn create_surface(
        &mut self,
        window: &'static winit::window::Window,
    ) -> Result<(), wgpu::CreateSurfaceError> {
        self.surface = Some(self.instance.create_surface(window)?);
        Ok(())
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

//! Mesh data shared by editor and runtime.

use crate::scene::Transform;

#[derive(Debug, Clone, Copy)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub transform: Transform,
}

impl Mesh {
    pub fn new(name: impl Into<String>, vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Self {
        Self {
            name: name.into(),
            vertices,
            indices,
            transform: Transform::default(),
        }
    }

    pub fn triangle(name: impl Into<String>) -> Self {
        Self::new(
            name,
            vec![
                MeshVertex { position: [0.0, 0.7, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.5, 1.0] },
                MeshVertex { position: [-0.7, -0.7, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
                MeshVertex { position: [0.7, -0.7, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
            ],
            vec![0, 1, 2],
        )
    }
}

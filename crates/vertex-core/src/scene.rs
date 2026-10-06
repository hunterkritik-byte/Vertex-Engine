//! Minimal scene representation.

use serde::{Deserialize, Serialize};
use crate::material::{DirectionalLight, Material};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    pub entities: Vec<Entity>,
    pub light: DirectionalLight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub transform: Transform,
    pub material: Material,
    /// Optional parent entity index used to preserve imported node hierarchy.
    #[serde(default)]
    pub parent: Option<usize>,
    /// Source asset path for imported GLTF/GLB content.
    #[serde(default)]
    pub asset_path: Option<String>,
    /// Primitive/node index inside the source asset.
    #[serde(default)]
    pub asset_node: Option<usize>,
    /// Primitive index inside the source mesh.
    #[serde(default)]
    pub asset_primitive: Option<usize>,
}

impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
            material: Material::default(),
            parent: None,
            asset_path: None,
            asset_node: None,
            asset_primitive: None,
        }
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, name: impl Into<String>) -> usize {
        let id = self.entities.len();
        self.entities.push(Entity::new(name));
        id
    }
}

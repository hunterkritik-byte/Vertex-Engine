//! Minimal scene representation.

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub transform: Transform,
}

impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
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

//! Rendering abstraction for Vertex Engine.
//!
//! The renderer backend will be introduced here without coupling the engine
//! core to a specific graphics API.

/// Placeholder renderer backend.
#[derive(Debug, Default)]
pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }
}

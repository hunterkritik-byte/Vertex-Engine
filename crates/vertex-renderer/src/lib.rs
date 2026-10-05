//! GPU rendering foundation for Vertex Engine.
//!
//! The first backend uses wgpu, which can target Vulkan on Linux/Windows
//! while keeping the renderer API portable.

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

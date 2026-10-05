//! Core types and engine lifecycle for Vertex Engine.

pub mod camera;
pub mod mesh;
pub mod scene;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Default)]
pub struct Engine {
    running: bool,
}

impl Engine {
    pub fn new() -> Self { Self::default() }

    pub fn start(&mut self) { self.running = true; }

    pub fn stop(&mut self) { self.running = false; }

    pub fn is_running(&self) -> bool { self.running }
}

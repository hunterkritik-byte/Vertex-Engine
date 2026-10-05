//! Core types and engine lifecycle for Vertex Engine.

/// Vertex Engine version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimal engine state used by the first editor prototype.
#[derive(Debug, Default)]
pub struct Engine {
    running: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

use std::sync::Arc;

use vertex_core::{camera::Camera, mesh::Mesh, scene::Scene, Engine};
use vertex_renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct Editor {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer<'static>>,
    engine: Engine,
    scene: Option<Scene>,
    camera: Camera,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            engine: Engine::default(),
            scene: None,
            camera: Camera::new(),
        }
    }
}

impl ApplicationHandler for Editor {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Vertex Engine — Editor")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
                )
                .expect("failed to create editor window"),
        );

        let renderer = match pollster::block_on(Renderer::new(&window)) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("Vertex renderer initialization failed: {error}");
                event_loop.exit();
                return;
            }
        };

        let mut scene = Scene::new();
        scene.spawn("Main Camera");
        scene.spawn("Triangle Mesh");
        let _mesh = Mesh::triangle("Triangle Mesh");

        self.engine.start();
        self.camera.position = [0.0, 0.0, 5.0];
        self.scene = Some(scene);
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let aspect = renderer.config.width as f32 / renderer.config.height.max(1) as f32;
                    let view_proj = self.camera.view_projection(aspect);
                    match renderer.render(view_proj) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            if let Some(window) = self.window.as_ref() {
                                let size = window.inner_size();
                                renderer.resize(size.width, size.height);
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(wgpu::SurfaceError::Timeout) => {}
                        Err(wgpu::SurfaceError::Other) => {}
                    }
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut Editor::default())
}

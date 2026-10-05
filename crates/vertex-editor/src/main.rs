use vertex_core::{camera::Camera, mesh::Mesh, scene::Scene, Engine};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct Editor {
    window: Option<Window>,
    engine: Engine,
    scene: Option<Scene>,
    camera: Camera,
}

impl ApplicationHandler for Editor {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Vertex Engine — Editor")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
                )
                .expect("failed to create editor window");

            let mut scene = Scene::new();
            scene.spawn("Main Camera");
            scene.spawn("Triangle Mesh");
            let _mesh = Mesh::triangle("Triangle Mesh");

            self.engine.start();
            self.camera.position = [0.0, 0.0, 5.0];
            self.scene = Some(scene);
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut editor = Editor {
        camera: Camera::new(),
        ..Default::default()
    };
    event_loop.run_app(&mut editor)
}

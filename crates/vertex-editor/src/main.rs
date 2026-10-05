use std::sync::Arc;

use egui::ViewportId;
use egui_wgpu::wgpu;
use vertex_core::{camera::Camera, scene::Scene, Engine};
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
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    engine: Engine,
    scene: Scene,
    camera: Camera,
    selected: usize,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            egui_renderer: None,
            engine: Engine::default(),
            scene: Scene::new(),
            camera: Camera::new(),
            selected: 0,
        }
    }
}

impl Editor {
    fn draw_ui(&mut self) {
        egui::TopBottomPanel::top("toolbar").show(&self.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Vertex Engine");
                ui.separator();
                if ui.button("▶ Play").clicked() {
                    self.engine.start();
                }
                ui.button("■ Stop");
                ui.separator();
                ui.label("3D Scene");
            });
        });

        egui::SidePanel::left("hierarchy").default_width(220.0).show(&self.egui_ctx, |ui| {
            ui.heading("Hierarchy");
            ui.separator();
            for (index, entity) in self.scene.entities.iter().enumerate() {
                if ui.selectable_label(self.selected == index, &entity.name).clicked() {
                    self.selected = index;
                }
            }
        });

        egui::SidePanel::right("inspector").default_width(280.0).show(&self.egui_ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();
            if let Some(entity) = self.scene.entities.get_mut(self.selected) {
                ui.label(egui::RichText::new(&entity.name).strong());
                ui.collapsing("Transform", |ui| {
                    for (label, values, speed) in [
                        ("Position", &mut entity.transform.position, 0.05),
                        ("Rotation", &mut entity.transform.rotation, 0.5),
                        ("Scale", &mut entity.transform.scale, 0.05),
                    ] {
                        ui.label(label);
                        ui.horizontal(|ui| {
                            for (axis, value) in values.iter_mut().enumerate() {
                                ui.add(egui::DragValue::new(value)
                                    .speed(speed)
                                    .prefix(["X ", "Y ", "Z "][axis]));
                            }
                        });
                    }
                });
            }
        });

        egui::TopBottomPanel::bottom("status").show(&self.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Vertex Engine v0.1.0");
                ui.separator();
                ui.label(format!("Entities: {}", self.scene.entities.len()));
                ui.separator();
                ui.label("GPU: wgpu");
            });
        });
    }

    fn render_ui(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let Some(state) = self.egui_state.as_mut() else { return };
        let raw_input = state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            self.draw_ui();
            let _ = ctx;
        });
        state.handle_platform_output(window, full_output.platform_output);

        let clipped = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let Some(renderer) = self.egui_renderer.as_mut() else { return };
        for (id, delta) in &full_output.textures_delta.set {
            renderer.update_texture(device, queue, *id, delta);
        }
        renderer.update_buffers(device, queue, encoder, &clipped, &screen);

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vertex-editor-ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let mut pass = pass.forget_lifetime();
            renderer.render(&mut pass, &clipped, &screen);
        }

        for id in &full_output.textures_delta.free {
            renderer.free_texture(id);
        }
    }
}

impl ApplicationHandler for Editor {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let window = Arc::new(event_loop.create_window(
            Window::default_attributes()
                .with_title("Vertex Engine — Editor")
                .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
        ).expect("failed to create editor window"));

        let window_for_renderer = Box::leak(Box::new(window.clone()));
        let renderer = match pollster::block_on(Renderer::new(window_for_renderer)) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("Vertex renderer initialization failed: {error}");
                event_loop.exit();
                return;
            }
        };

        self.scene.spawn("Main Camera");
        self.scene.spawn("Cube");

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &renderer.device,
            renderer.config.format,
            egui_wgpu::RendererOptions::default(),
        );

        self.engine.start();
        self.camera.position = [0.0, 0.0, 5.0];
        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else { return };

        if let Some(state) = self.egui_state.as_mut() {
            let _ = state.on_window_event(&window, &event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let (device, queue, surface, width, height) = {
                    let Some(renderer) = self.renderer.as_ref() else { return };
                    (&renderer.device, &renderer.queue, &renderer.surface, renderer.config.width, renderer.config.height)
                };
                let aspect = width as f32 / height.max(1) as f32;
                let view_proj = self.camera.view_projection(aspect);

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Some(renderer) = self.renderer.as_mut() {
                            renderer.resize(width, height);
                        }
                        return;
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => { event_loop.exit(); return; }
                    Err(_) => return,
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: Some("vertex-frame") }
                );
                if let Some(renderer) = self.renderer.as_ref() {
                    renderer.render_to_view(&mut encoder, &view, view_proj);
                }
                self.render_ui(&window, device, queue, &view, &mut encoder);
                queue.submit(Some(encoder.finish()));
                frame.present();
                window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    EventLoop::new()?.run_app(&mut Editor::default())
}

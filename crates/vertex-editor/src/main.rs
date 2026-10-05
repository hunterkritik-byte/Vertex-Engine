use std::sync::Arc;

use glam::{Mat4, Vec3};

use egui::ViewportId;
use egui_wgpu::wgpu;
use vertex_core::{camera::Camera, scene::Scene, Engine};
use vertex_renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Clone, Copy, PartialEq)]
enum GizmoMode { Translate, Rotate, Scale }

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
    gizmo: GizmoMode,
    viewport_drag: bool,
    last_cursor: Option<(f64, f64)>,
    camera_drag: Option<MouseButton>,
    history: Vec<Scene>,
    redo_stack: Vec<Scene>,
    scene_path: PathBuf,
    mouse_press: Option<(f64, f64)>,
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
            gizmo: GizmoMode::Translate,
            viewport_drag: false,
            last_cursor: None,
            camera_drag: None,
            history: Vec::new(),
            redo_stack: Vec::new(),
            scene_path: PathBuf::from("scene.vertexscene"),
            mouse_press: None,
        }
    }
}

impl Editor {
    fn snapshot(&mut self) {
        self.history.push(self.scene.clone());
        self.redo_stack.clear();
        if self.history.len() > 64 { self.history.remove(0); }
    }

    fn undo(&mut self) {
        if let Some(scene) = self.history.pop() {
            self.redo_stack.push(self.scene.clone());
            self.scene = scene;
            self.selected = self.selected.min(self.scene.entities.len().saturating_sub(1));
        }
    }

    fn redo_scene(&mut self) {
        if let Some(scene) = self.redo_stack.pop() {
            self.history.push(self.scene.clone());
            self.scene = scene;
        }
    }

    fn save_scene(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&self.scene) {
            let _ = fs::write(&self.scene_path, text);
        }
    }

    fn load_scene(&mut self) {
        if let Ok(text) = fs::read_to_string(&self.scene_path) {
            if let Ok(scene) = serde_json::from_str::<Scene>(&text) {
                self.history.push(self.scene.clone());
                self.scene = scene;
                self.selected = 0;
            }
        }
    }

    fn pick_entity(&mut self, cursor: (f64, f64), size: (u32, u32)) {
        let x = (cursor.0 as f32 / size.0.max(1) as f32) * 2.0 - 1.0;
        let y = 1.0 - (cursor.1 as f32 / size.1.max(1) as f32) * 2.0;
        let inv = self.camera.view_projection(size.0 as f32 / size.1.max(1) as f32).inverse();
        let a = inv * Vec4::new(x, y, 0.0, 1.0);
        let b = inv * Vec4::new(x, y, 1.0, 1.0);
        let origin = a.truncate() / a.w;
        let direction = (b.truncate() / b.w - origin).normalize();
        let mut hit = None;
        let mut nearest = f32::MAX;
        for (i, entity) in self.scene.entities.iter().enumerate() {
            let t = &entity.transform;
            let model = Mat4::from_scale_rotation_translation(
                Vec3::from_array(t.scale),
                glam::Quat::from_euler(glam::EulerRot::XYZ, t.rotation[0].to_radians(), t.rotation[1].to_radians(), t.rotation[2].to_radians()),
                Vec3::from_array(t.position),
            );
            let inv_model = model.inverse();
            let o = (inv_model * origin.extend(1.0)).truncate();
            let d = (inv_model * direction.extend(0.0)).truncate().normalize();
            let mut tmin = -f32::INFINITY;
            let mut tmax = f32::INFINITY;
            for axis in 0..3 {
                if d[axis].abs() < 0.0001 {
                    if o[axis].abs() > 1.0 { tmin = 1.0; tmax = 0.0; break; }
                } else {
                    let q1 = (-1.0 - o[axis]) / d[axis];
                    let q2 = (1.0 - o[axis]) / d[axis];
                    tmin = tmin.max(q1.min(q2));
                    tmax = tmax.min(q1.max(q2));
                }
            }
            if tmax >= tmin && tmax >= 0.0 && tmin < nearest {
                nearest = tmin.max(0.0);
                hit = Some(i);
            }
        }
        if let Some(i) = hit { self.selected = i; }
    }

    fn draw_ui(&mut self) {
        egui::TopBottomPanel::top("toolbar").show(&self.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Vertex Engine");
                ui.separator();
                if ui.button("▶ Play").clicked() {
                    self.engine.start();
                }
                if ui.button("■ Stop").clicked() {
                    self.engine.stop();
                }
                ui.separator();
                ui.label("Gizmo:");
                if ui.selectable_label(self.gizmo == GizmoMode::Translate, "Move").clicked() {
                    self.gizmo = GizmoMode::Translate;
                }
                if ui.selectable_label(self.gizmo == GizmoMode::Rotate, "Rotate").clicked() {
                    self.gizmo = GizmoMode::Rotate;
                }
                if ui.selectable_label(self.gizmo == GizmoMode::Scale, "Scale").clicked() {
                    self.gizmo = GizmoMode::Scale;
                }
                ui.separator();
                if ui.button("Undo").clicked() { self.undo(); }\n                if ui.button("Redo").clicked() { self.redo_scene(); }\n                if ui.button("Save").clicked() { self.save_scene(); }\n                if ui.button("Load").clicked() { self.load_scene(); }

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

    fn camera_mouse(&mut self, button: MouseButton, dx: f32, dy: f32) {
        match button {
            MouseButton::Right => {
                self.camera.yaw -= dx * 0.005;
                self.camera.pitch = (self.camera.pitch - dy * 0.005).clamp(-1.5, 1.5);
            }
            MouseButton::Middle => {
                self.camera.position[0] -= dx * 0.01;
                self.camera.position[1] += dy * 0.01;
            }
            _ => {}
        }
    }

    fn render_ui(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let raw_input = match self.egui_state.as_mut() {
            Some(state) => state.take_egui_input(window),
            None => return,
        };

        // Do not keep an egui_winit mutable borrow while building the UI.
        let full_output = {
            let ctx = self.egui_ctx.clone();
            ctx.run(raw_input, |_ctx| {
                self.draw_ui();
            })
        };

        if let Some(state) = self.egui_state.as_mut() {
            state.handle_platform_output(window, full_output.platform_output);
        }

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
            WindowEvent::CursorMoved { position, .. } => {
                if self.viewport_drag {
                    if let Some((last_x, last_y)) = self.last_cursor {
                        let dx = (position.x - last_x) as f32;
                        let dy = (position.y - last_y) as f32;
                        if let Some(entity) = self.scene.entities.get_mut(self.selected) {
                            match self.gizmo {
                                GizmoMode::Translate => {
                                    entity.transform.position[0] += dx * 0.01;
                                    entity.transform.position[1] -= dy * 0.01;
                                }
                                GizmoMode::Rotate => {
                                    entity.transform.rotation[1] += dx * 0.5;
                                    entity.transform.rotation[0] += dy * 0.5;
                                }
                                GizmoMode::Scale => {
                                    let delta = (dx - dy) * 0.005;
                                    for value in &mut entity.transform.scale {
                                        *value = (*value + delta).max(0.05);
                                    }
                                }
                            }
                        }
                    }
                    self.last_cursor = Some((position.x, position.y));
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        self.viewport_drag = true;
                        self.mouse_press = self.last_cursor;
                        self.last_cursor = None;
                    } else {
                        self.viewport_drag = false;
                        if let (Some(start), Some(end)) = (self.mouse_press, self.last_cursor) {
                            let dx = end.0 - start.0;
                            let dy = end.1 - start.1;
                            if dx * dx + dy * dy < 36.0 {
                                self.snapshot();
                                self.pick_entity(end, (window.inner_size().width, window.inner_size().height));
                            }
                        }
                        self.mouse_press = None;
                        self.last_cursor = None;
                    }
                } else if button == MouseButton::Middle || button == MouseButton::Right {
                    self.camera_drag = (state == ElementState::Pressed).then_some(button);
                    if state == ElementState::Pressed { self.last_cursor = None; }
                    else { self.last_cursor = None; }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 0.5,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                };
                self.camera.position[2] = (self.camera.position[2] - amount).clamp(1.0, 100.0);
                window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let (width, height) = match self.renderer.as_ref() {
                    Some(renderer) => (renderer.config.width, renderer.config.height),
                    None => return,
                };
                let aspect = width as f32 / height.max(1) as f32;
                let view_proj = self.camera.view_projection(aspect);
                let model = if let Some(entity) = self.scene.entities.get(self.selected) {
                    let t = &entity.transform;
                    Mat4::from_scale_rotation_translation(
                        Vec3::from_array(t.scale),
                        glam::Quat::from_euler(glam::EulerRot::XYZ, t.rotation[0].to_radians(), t.rotation[1].to_radians(), t.rotation[2].to_radians()),
                        Vec3::from_array(t.position),
                    )
                } else {
                    Mat4::IDENTITY
                };

                let frame = match self.renderer.as_ref().unwrap().surface.get_current_texture() {
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
                let mut encoder = self.renderer.as_ref().unwrap().device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: Some("vertex-frame") }
                );
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.renderer.as_ref().unwrap().render_to_view(&mut encoder, &view, view_proj, model);
                let device = self.renderer.as_ref().unwrap().device.clone();
                let queue = self.renderer.as_ref().unwrap().queue.clone();
                self.render_ui(&window, &device, &queue, &view, &mut encoder);
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

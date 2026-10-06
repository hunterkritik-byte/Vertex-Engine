use std::{collections::HashMap, fs, path::{Path, PathBuf}, sync::Arc};

use glam::{Mat4, Vec3, Vec4};

use egui::ViewportId;
use egui_wgpu::wgpu;
use vertex_core::{asset::scan_assets, camera::Camera, scene::Scene, Engine};
use vertex_renderer::Renderer;
use gltf::image::Format as GltfImageFormat;
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
    gizmo_axis: Option<usize>,
    gizmo_drag_start: Option<(f64, f64)>,
    asset_root: PathBuf,
    texture_cache: HashMap<PathBuf, wgpu::BindGroup>,
    asset_mtimes: HashMap<PathBuf, std::time::SystemTime>,
    thumbnail_cache: HashMap<PathBuf, egui::TextureHandle>,
    mesh_cache: HashMap<(PathBuf, usize, usize), u64>,
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
            gizmo_axis: None,
            gizmo_drag_start: None,
            asset_root: PathBuf::from("assets"),
            texture_cache: HashMap::new(),
            asset_mtimes: HashMap::new(),
            thumbnail_cache: HashMap::new(),
            mesh_cache: HashMap::new(),
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

    fn import_gltf(&mut self, path: &Path) {
        let source = path.to_string_lossy().into_owned();
        self.scene.entities.retain(|e| e.asset_path.as_deref() != Some(&source));
        self.mesh_cache.retain(|(p, _, _), _| p != path);
        let imported = match gltf::import(path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("GLTF import failed for {}: {error}", path.display());
                return;
            }
        };
        let (document, _buffers, images) = imported;
        let asset_path = path.to_string_lossy().into_owned();
        let mut image_paths = Vec::new();
        let texture_dir = self.asset_root.join("imported");
        let _ = fs::create_dir_all(&texture_dir);

        for (index, image) in images.iter().enumerate() {
            let rgba = match image.format {
                GltfImageFormat::R8 => image::DynamicImage::ImageLuma8(
                    image::GrayImage::from_raw(image.width, image.height, image.pixels.clone()).unwrap_or_default()
                ).to_rgba8(),
                GltfImageFormat::R8G8 => image::DynamicImage::ImageLumaA8(
                    image::GrayAlphaImage::from_raw(image.width, image.height, image.pixels.clone()).unwrap_or_default()
                ).to_rgba8(),
                GltfImageFormat::R8G8B8 => image::DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(image.width, image.height, image.pixels.clone()).unwrap_or_default()
                ).to_rgba8(),
                GltfImageFormat::R8G8B8A8 => image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone()).unwrap_or_default(),
                _ => continue,
            };
            let out = texture_dir.join(format!(
                "{}_tex_{index}.png",
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset")
            ));
            if rgba.save(&out).is_ok() {
                image_paths.push(out);
            }
        }

        let base_parent = None;
        let mut node_entities = vec![None; document.nodes().count()];
        for node in document.nodes() {
            let count = node.mesh().map(|m| m.primitives().count()).unwrap_or(0).max(1);
            for primitive_index in 0..count {
                let name = if count == 1 { node.name().unwrap_or("GLTF Node").to_string() } else { format!("{} [Primitive {}]", node.name().unwrap_or("GLTF Node"), primitive_index) };
                let entity_id = self.scene.spawn(name);
                if node_entities[node.index()].is_none() { node_entities[node.index()] = Some(entity_id); }
                if let Some(entity) = self.scene.entities.get_mut(entity_id) {
                entity.asset_path = Some(asset_path.clone());
                entity.asset_node = Some(node.index());
                entity.asset_primitive = Some(primitive_index);
                entity.parent = base_parent;
                {
                    let (translation, rotation, scale) = node.transform().decomposed();
                    entity.transform.position = translation;
                    entity.transform.scale = scale;
                    let q = glam::Quat::from_array([rotation[0], rotation[1], rotation[2], rotation[3]]);
                    let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
                    entity.transform.rotation = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
                }
                if let Some(mesh) = node.mesh() {
                    if let Some(primitive) = mesh.primitives().nth(primitive_index) {
                        let reader = primitive.reader(|buffer| Some(&_buffers[buffer.index()]));
                        let positions: Vec<[f32; 3]> = reader.read_positions().map(|v| v.collect()).unwrap_or_default();
                        let normals: Vec<[f32; 3]> = reader.read_normals().map(|v| v.collect()).unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
                        let uvs: Vec<[f32; 2]> = reader.read_tex_coords(0).map(|v| v.into_f32().collect()).unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
                        let indices: Vec<u32> = reader.read_indices().map(|v| v.into_u32().collect()).unwrap_or_else(|| (0..positions.len() as u32).collect());
                        let vertices: Vec<vertex_renderer::Vertex> = positions.iter().enumerate().map(|(i, &position)| vertex_renderer::Vertex {
                            position,
                            color: [1.0, 1.0, 1.0],
                            normal: normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                            uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                        }).collect();
                        if let Some(renderer) = self.renderer.as_mut() {
                            match renderer.upload_mesh(&vertices, &indices) {
                                Ok(handle) => { self.mesh_cache.insert((path.to_path_buf(), node.index(), primitive_index), handle); }
                                Err(error) => eprintln!("GPU mesh upload failed: {error}"),
                            }
                        }
                        let material = primitive.material();
                        let pbr = material.pbr_metallic_roughness();
                        let base = pbr.base_color_factor();
                        entity.material.albedo = base;
                        entity.material.metallic = pbr.metallic_factor();
                        entity.material.roughness = pbr.roughness_factor();
                        if let Some(texture) = pbr.base_color_texture() {
                            let index = texture.texture().source().index();
                            if let Some(texture_path) = image_paths.get(index) {
                                entity.material.texture_path = Some(texture_path.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }
        }

        // glTF 1.4.1 does not expose Node::parent(); preserve hierarchy by
        // walking each scene's children recursively from the scene roots.
        fn link_children(
            node: gltf::Node<'_>,
            parent_entity: Option<usize>,
            node_entities: &[Option<usize>],
            entities: &mut [vertex_core::scene::Entity],
        ) {
            let entity = node_entities.get(node.index()).copied().flatten();
            if let (Some(child), Some(parent)) = (entity, parent_entity) {
                if let Some(item) = entities.get_mut(child) {
                    item.parent = Some(parent);
                }
            }
            for child_node in node.children() {
                link_children(child_node, entity.or(parent_entity), node_entities, entities);
            }
        }

        for scene in document.scenes() {
            for root in scene.nodes() {
                link_children(root, None, &node_entities, &mut self.scene.entities);
            }
        }

        if let Some(first) = node_entities.iter().flatten().next() {
            self.selected = *first;
        }
        if let Ok(modified) = fs::metadata(path).and_then(|m| m.modified()) {
            self.asset_mtimes.insert(path.to_path_buf(), modified);
        }
        self.save_scene();
        eprintln!("Imported GLTF/GLB: {}", path.display());
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

    fn project_selected(&self, cursor_size: (u32, u32)) -> Option<egui::Pos2> {
        let entity = self.scene.entities.get(self.selected)?;
        let aspect = cursor_size.0 as f32 / cursor_size.1.max(1) as f32;
        let clip = self.camera.view_projection(aspect)
            * Vec4::new(entity.transform.position[0], entity.transform.position[1], entity.transform.position[2], 1.0);
        if clip.w <= 0.0 { return None; }
        let ndc = clip.truncate() / clip.w;
        Some(egui::pos2(
            (ndc.x * 0.5 + 0.5) * cursor_size.0 as f32,
            (-ndc.y * 0.5 + 0.5) * cursor_size.1 as f32,
        ))
    }

    fn gizmo_hit(&self, cursor: (f64, f64), size: (u32, u32)) -> Option<usize> {
        let center = self.project_selected(size)?;
        let p = egui::pos2(cursor.0 as f32, cursor.1 as f32);
        let axes = [egui::vec2(75.0, 0.0), egui::vec2(0.0, -75.0), egui::vec2(-53.0, 53.0)];
        axes.iter().enumerate().find_map(|(i, axis)| {
            let end = center + *axis;
            let v = end - center;
            let w = p - center;
            let t = (w.dot(v) / v.dot(v)).clamp(0.0, 1.0);
            let distance = (w - v * t).length();
            (distance < 12.0).then_some(i)
        })
    }

    fn draw_gizmo(&self, ctx: &egui::Context, size: (u32, u32)) {
        let Some(center) = self.project_selected(size) else { return };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground, egui::Id::new("vertex-transform-gizmo")
        ));
        let axes = [
            (egui::vec2(75.0, 0.0), egui::Color32::RED, "X"),
            (egui::vec2(0.0, -75.0), egui::Color32::GREEN, "Y"),
            (egui::vec2(-53.0, 53.0), egui::Color32::BLUE, "Z"),
        ];
        painter.circle_stroke(center, 9.0, egui::Stroke::new(2.0_f32, egui::Color32::WHITE));
        for (i, (offset, color, label)) in axes.into_iter().enumerate() {
            let color = if self.gizmo_axis == Some(i) { egui::Color32::YELLOW } else { color };
            let end = center + offset;
            painter.line_segment([center, end], egui::Stroke::new(4.0_f32, color));
            painter.circle_filled(end, 8.0, color);
            painter.text(end, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), egui::Color32::WHITE);
        }
        painter.rect_stroke(
            egui::Rect::from_center_size(center, egui::vec2(150.0, 150.0)),
            2.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 110)),
            egui::StrokeKind::Outside,
        );
    }

    fn apply_gizmo_drag(&mut self, axis: usize, dx: f32, dy: f32) {
        if let Some(entity) = self.scene.entities.get_mut(self.selected) {
            let amount = match axis { 0 => dx, 1 => -dy, _ => (dx - dy) * 0.707 };
            match self.gizmo {
                GizmoMode::Translate => entity.transform.position[axis] += amount * 0.01,
                GizmoMode::Rotate => entity.transform.rotation[axis] += amount * 0.5,
                GizmoMode::Scale => {
                    let factor = 1.0 + amount * 0.005;
                    if axis == 0 || axis == 1 || axis == 2 {
                        entity.transform.scale[axis] = (entity.transform.scale[axis] * factor.max(0.05)).max(0.05);
                    }
                }
            }
        }
    }

    fn draw_ui(&mut self) {
        // Collect toolbar actions first. Mutating self from inside an egui closure
        // while self.egui_ctx is borrowed triggers E0500/E0502.
        let mut play = false;
        let mut stop = false;
        let mut undo = false;
        let mut redo = false;
        let mut save = false;
        let mut load = false;
        let mut gizmo_mode = None;

        egui::TopBottomPanel::top("toolbar").show(&self.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Vertex Engine");
                ui.separator();
                if ui.button("▶ Play").clicked() { play = true; }
                if ui.button("■ Stop").clicked() { stop = true; }
                ui.separator();
                ui.label("Gizmo:");
                if ui.selectable_label(self.gizmo == GizmoMode::Translate, "Move").clicked() {
                    gizmo_mode = Some(GizmoMode::Translate);
                }
                if ui.selectable_label(self.gizmo == GizmoMode::Rotate, "Rotate").clicked() {
                    gizmo_mode = Some(GizmoMode::Rotate);
                }
                if ui.selectable_label(self.gizmo == GizmoMode::Scale, "Scale").clicked() {
                    gizmo_mode = Some(GizmoMode::Scale);
                }
                ui.separator();
                if ui.button("Undo").clicked() { undo = true; }
                if ui.button("Redo").clicked() { redo = true; }
                if ui.button("Save").clicked() { save = true; }
                if ui.button("Load").clicked() { load = true; }
            });
        });

        if play { self.engine.start(); }
        if stop { self.engine.stop(); }
        if let Some(mode) = gizmo_mode { self.gizmo = mode; }
        if undo { self.undo(); }
        if redo { self.redo_scene(); }
        if save { self.save_scene(); }
        if load { self.load_scene(); }

        let mut hierarchy_selection = None;
        egui::SidePanel::left("hierarchy").default_width(220.0).show(&self.egui_ctx, |ui| {
            ui.heading("Hierarchy");
            ui.separator();
            for (index, entity) in self.scene.entities.iter().enumerate() {
                if ui.selectable_label(self.selected == index, &entity.name).clicked() {
                    hierarchy_selection = Some(index);
                }
            }
        });
        if let Some(index) = hierarchy_selection {
            self.selected = index;
        }

        let mut asset_action = None;
        egui::SidePanel::left("assets").resizable(true).default_width(240.0).show(&self.egui_ctx, |ui| {
            ui.heading("📦 Asset Browser 2.0");
            ui.small("Drag 3D assets into the Scene View");
            ui.separator();
            for asset in scan_assets(&self.asset_root) {
                if asset.is_directory { ui.label(format!("📂 {}", asset.path.display())); continue; }
                let ext = asset.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                let icon = match ext.as_str() { "glb" | "gltf" => "🧩", "png" | "jpg" | "jpeg" => "🖼️", _ => "📄" };
                let name = asset.path.file_name().and_then(|n| n.to_str()).unwrap_or("asset");
                let response = ui.add(egui::Label::new(format!("{icon} {name}")).sense(egui::Sense::drag()));
                if response.drag_stopped() && matches!(ext.as_str(), "glb" | "gltf") { asset_action = Some(asset.path.clone()); }
                response.context_menu(|menu| {
                    if matches!(ext.as_str(), "glb" | "gltf") && menu.button("Re-import").clicked() {
                        asset_action = Some(asset.path.clone()); menu.close();
                    }
                });
            }
        });
        if let Some(path) = asset_action {
            self.snapshot();
            self.import_gltf(&path);
        }

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
                                ui.add(
                                    egui::DragValue::new(value)
                                        .speed(speed)
                                        .prefix(["X ", "Y ", "Z "][axis]),
                                );
                            }
                        });
                    }
                });
                ui.collapsing("Material", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Albedo");
                        for value in &mut entity.material.albedo[..3] {
                            ui.add(egui::DragValue::new(value).range(0.0..=1.0).speed(0.01));
                        }
                    });
                    ui.add(egui::Slider::new(&mut entity.material.metallic, 0.0..=1.0).text("Metallic"));
                    ui.add(egui::Slider::new(&mut entity.material.roughness, 0.04..=1.0).text("Roughness"));
                    ui.label(if entity.material.texture_id.is_some() { "Texture: assigned" } else { "Texture: none" });
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
                self.draw_gizmo(&ctx, (window.inner_size().width, window.inner_size().height));
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
                if let Some(button) = self.camera_drag {
                    if let Some((last_x, last_y)) = self.last_cursor {
                        self.camera_mouse(button, (position.x-last_x) as f32, (position.y-last_y) as f32);
                    }
                    self.last_cursor = Some((position.x, position.y));
                    window.request_redraw();
                } else if self.viewport_drag {
                    if let Some((last_x, last_y)) = self.last_cursor {
                        let dx = (position.x - last_x) as f32;
                        let dy = (position.y - last_y) as f32;
                        if let Some(axis) = self.gizmo_axis {
                            self.apply_gizmo_drag(axis, dx, dy);
                        } else if let Some(entity) = self.scene.entities.get_mut(self.selected) {
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
                                    for value in &mut entity.transform.scale { *value = (*value + delta).max(0.05); }
                                }
                            }
                        }
                    }
                    self.last_cursor = Some((position.x, position.y));
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    use winit::keyboard::{KeyCode, PhysicalKey};
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyW) => self.gizmo = GizmoMode::Translate,
                        PhysicalKey::Code(KeyCode::KeyE) => self.gizmo = GizmoMode::Rotate,
                        PhysicalKey::Code(KeyCode::KeyR) => self.gizmo = GizmoMode::Scale,
                        PhysicalKey::Code(KeyCode::KeyZ) => self.undo(),
                        PhysicalKey::Code(KeyCode::KeyY) => self.redo_scene(),
                        _ => {}
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        let cursor = self.last_cursor;
                        self.gizmo_axis = cursor.and_then(|p| self.gizmo_hit(p, (window.inner_size().width, window.inner_size().height)));
                        self.gizmo_drag_start = cursor;
                        self.viewport_drag = self.gizmo_axis.is_some();
                        if self.viewport_drag { self.snapshot(); }
                        self.mouse_press = cursor;
                        self.last_cursor = cursor;
                    } else {
                        if let (Some(start), Some(end)) = (self.mouse_press, self.last_cursor) {
                            let dx = end.0 - start.0;
                            let dy = end.1 - start.1;
                            if self.gizmo_axis.is_none() && dx * dx + dy * dy < 36.0 {
                                self.pick_entity(end, (window.inner_size().width, window.inner_size().height));
                            }
                        }
                        self.viewport_drag = false;
                        self.gizmo_axis = None;
                        self.gizmo_drag_start = None;
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
            WindowEvent::DroppedFile(path) => {
                let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
                if matches!(ext.as_deref(), Some("glb") | Some("gltf")) {
                    self.snapshot();
                    self.import_gltf(&path);
                } else {
                    let supported = matches!(ext.as_deref(), Some("png") | Some("jpg") | Some("jpeg"));
                    if supported {
                    let imported = self.renderer.as_ref().and_then(|renderer| {
                        renderer.load_texture(&path).map_err(|error| {
                            eprintln!("Texture import failed: {error}");
                        }).ok()
                    });
                    if let Some(bind_group) = imported {
                        self.texture_cache.insert(path.clone(), bind_group);
                        self.snapshot();
                        if let Some(entity) = self.scene.entities.get_mut(self.selected) {
                            entity.material.texture_path = Some(path.to_string_lossy().into_owned());
                        }
                    }
                    } else {
                        eprintln!("Unsupported dropped asset: {}", path.display());
                    }
                }
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
                let material = self.scene.entities.get(self.selected).map(|e| e.material.clone()).unwrap_or_default();
                let texture_bind_group = material.texture_path.as_ref()
                    .and_then(|path| self.texture_cache.get(Path::new(path)));
                if let Some(entity) = self.scene.entities.get(self.selected) {
                    if let (Some(asset_path), Some(asset_node)) = (&entity.asset_path, entity.asset_node) {
                        if let Some(&mesh_id) = self.mesh_cache.get(&(PathBuf::from(asset_path), asset_node, entity.asset_primitive.unwrap_or(0))) {
                            self.renderer.as_ref().unwrap().render_mesh_to_view(
                                &mut encoder, &view, view_proj, model,
                                material.albedo, self.scene.light.direction, self.scene.light.color,
                                self.scene.light.intensity, material.metallic, material.roughness,
                                texture_bind_group, mesh_id, true,
                            );
                        } else {
                            self.renderer.as_ref().unwrap().render_to_view(
                                &mut encoder, &view, view_proj, model, material.albedo,
                                self.scene.light.direction, self.scene.light.color, self.scene.light.intensity,
                                material.metallic, material.roughness, texture_bind_group,
                            );
                        }
                    } else {
                        self.renderer.as_ref().unwrap().render_to_view(
                            &mut encoder, &view, view_proj, model, material.albedo,
                            self.scene.light.direction, self.scene.light.color, self.scene.light.intensity,
                            material.metallic, material.roughness, texture_bind_group,
                        );
                    }
                }
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

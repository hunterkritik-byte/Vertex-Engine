# Vertex Engine

> A Rust-first, open-source 2D & 3D game engine aiming for a modern Unity/Unreal-style workflow. 🦀🎮

Vertex Engine is an experimental game engine built from the ground up with Rust, wgpu, and an editor UI.

## 🚀 Current Status

**Early development — v0.1.x**

- ✅ Rust workspace architecture
- ✅ wgpu GPU renderer and render pipeline
- ✅ 3D camera and projection matrices
- ✅ Depth-buffer based 3D rendering
- ✅ Real mesh rendering
- ✅ Scene/entity transforms
- ✅ Hierarchy + Inspector editor
- ✅ Viewport object picking
- ✅ W/E/R transform modes
- ✅ Camera orbit, pan and zoom
- ✅ .vertexscene save/load
- ✅ Undo/Redo scene snapshots
- 🚧 Interactive XYZ gizmo dragging
- 🚧 Selected-object outline/highlight
- 🚧 Asset pipeline
- 🚧 Physics, animation and audio
- 🚧 Scripting and game export

## 🎮 Editor Controls

| Input | Action |
|---|---|
| **W** | Translate mode |
| **E** | Rotate mode |
| **R** | Scale mode |
| **Left click** | Select/interact with viewport |
| **Right mouse drag** | Orbit camera |
| **Middle mouse drag** | Pan camera |
| **Mouse wheel** | Zoom |
| **Undo / Redo** | Revert or restore scene changes |
| **Save / Load** | Save or load .vertexscene |

## 🧩 Architecture

```text
Vertex Engine
├── vertex-core
│   ├── Scene / Entities
│   ├── Transforms
│   └── Camera
├── vertex-renderer
│   ├── wgpu GPU backend
│   ├── Shaders
│   ├── Meshes
│   └── Depth rendering
└── vertex-editor
    ├── Hierarchy
    ├── Inspector
    ├── Scene View
    ├── Selection
    └── Transform tools
```

## 🛠️ Run

Requirements: Rust stable, a desktop platform supported by wgpu, and suitable GPU drivers.

```bash
git clone https://github.com/hunterkritik-byte/Vertex-Engine.git
cd Vertex-Engine
cargo run -p vertex-editor
```

Verify the workspace:

```bash
cargo check --workspace
cargo test --workspace
```

## 🗺️ Roadmap

### v0.1 — Rendering Foundation
- [x] GPU renderer
- [x] 3D camera
- [x] Depth buffer
- [x] Mesh rendering
- [x] Basic editor

### v0.2 — Editor Foundation
- [x] Hierarchy
- [x] Inspector
- [x] Scene selection
- [x] Transform modes
- [x] Camera navigation
- [x] Scene persistence
- [x] Undo/Redo
- [ ] Interactive XYZ gizmo
- [ ] Selection outline

### Future
- [ ] Asset browser/importer
- [ ] Materials and textures
- [ ] Lighting and shadows
- [ ] Physics
- [ ] Animation
- [ ] Audio
- [ ] Scripting API
- [ ] Prefabs
- [ ] Packaging and game export

## 🤝 Contributing

Vertex Engine is early-stage. Contributions, renderer improvements, editor tooling, and architecture ideas are welcome.

Before submitting changes:

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
```

## 📄 License

MIT

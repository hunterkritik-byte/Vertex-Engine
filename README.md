# Vertex Engine

> A modern, open-source 2D & 3D game engine built from the ground up with Rust.

Vertex Engine is an experimental game engine focused on a clean architecture, high-performance rendering, and a professional editor workflow.

## Vision

Vertex Engine aims to provide a Unity/Unreal-style development experience while remaining open source and Rust-first.

Planned systems include:

- 2D and 3D rendering
- Scene and entity/component systems
- Editor with hierarchy and inspector
- Asset pipeline
- Physics and collision
- Animation
- Audio
- Input and controller support
- Scripting
- Cross-platform game builds

## Current status

**Early development — v0.1.0**

The repository currently contains the initial Rust workspace, engine core, renderer abstraction, and a runnable editor window.

## Run

Requirements: Rust stable and a supported desktop platform.

```bash
cargo run -p vertex-editor
```

## Architecture

```text
Vertex Engine
├── vertex-core
├── vertex-renderer
└── vertex-editor
```

More engine subsystems will be added incrementally as the architecture stabilizes.

## License

MIT

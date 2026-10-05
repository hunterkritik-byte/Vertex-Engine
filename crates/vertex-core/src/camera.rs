//! Camera primitives shared by the editor and runtime.

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            yaw: 0.0,
            pitch: 0.0,
            fov_y_radians: 60.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    pub fn new() -> Self { Self::default() }

    pub fn translate(&mut self, delta: [f32; 3]) {
        for (p, d) in self.position.iter_mut().zip(delta) {
            *p += d;
        }
    }
}

//! Perspective camera with view and projection matrices.

use glam::{Mat4, Vec3};

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

    pub fn view_matrix(&self) -> Mat4 {
        let position = Vec3::from_array(self.position);
        let direction = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        ).normalize();
        Mat4::look_to_rh(position, direction, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_radians,
            aspect.max(0.01),
            self.near,
            self.far,
        )
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }
}

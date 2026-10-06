use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub albedo: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub texture_id: Option<u32>,
    pub texture_path: Option<String>,
    pub normal_path: Option<String>,
    pub metallic_path: Option<String>,
    pub roughness_path: Option<String>,
}
impl Default for Material {
    fn default() -> Self { Self { albedo: [0.8,0.8,0.85,1.0], metallic: 0.0, roughness: 0.5, texture_id: None, texture_path: None, normal_path: None, metallic_path: None, roughness_path: None } }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
}
impl Default for DirectionalLight {
    fn default() -> Self { Self { direction: [-0.4,-1.0,-0.5], intensity: 2.0, color: [1.0,0.95,0.9] } }
}

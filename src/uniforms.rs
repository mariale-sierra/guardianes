use crate::matrix::Mat4;
use raylib::prelude::Vector3;

/// Datos comunes del cuadro; model_matrix y shader_index cambian por objeto.
pub struct Uniforms {
    pub model_matrix: Mat4,
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub camera_position: Vector3,
    pub light_position: Vector3,
    pub time: f32,
    pub delta_time: f32,
    pub shader_index: usize,
}

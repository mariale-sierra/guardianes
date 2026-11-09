use raylib::prelude::Vector3;

/// Datos interpolados de un píxel candidato, listos para el shader.
pub struct Fragment {
    pub depth: f32,
    pub world_position: Vector3,
    pub normal: Vector3,          // en mundo, normalizada
    pub object_position: Vector3, // coordenadas LOCALES originales (el patrón se queda pegado al objeto)
    pub uv: (f32, f32),
}

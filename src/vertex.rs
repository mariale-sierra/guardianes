use raylib::prelude::Vector3;

/// Vértice del modelo en coordenadas LOCALES (no cambia al mover el objeto).
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub normal: Vector3,
    pub uv: (f32, f32),
}

impl Vertex {
    pub fn new(position: Vector3, normal: Vector3, uv: (f32, f32)) -> Self {
        Vertex { position, normal, uv }
    }
}

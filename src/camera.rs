use crate::matrix::{create_view_matrix, Mat4};
use raylib::prelude::Vector3;

/// Cámara orbital alrededor de `target` (temporal; luego la reemplaza la cámara de la nave).
pub struct Camera {
    pub eye: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Camera {
    pub fn new(distance: f32, pitch: f32) -> Self {
        let mut c = Camera {
            eye: Vector3::new(0.0, 0.0, distance),
            target: Vector3::zero(),
            up: Vector3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch,
            distance,
        };
        c.update_eye();
        c
    }

    pub fn update_eye(&mut self) {
        self.pitch = self.pitch.clamp(-1.5, 1.5);
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.eye = self.target + Vector3::new(cp * sy, sp, cp * cy) * self.distance;
    }

    /// Coloca la cámara del lado del planeta que mira al sol (con un poco de ángulo
    /// para que se vea el terminador). Sirve para inspeccionar planetas.
    pub fn look_from_sun_side(&mut self, planet: Vector3, sun: Vector3) {
        let d = sun - planet;
        self.yaw = d.x.atan2(d.z) + 0.7;
        self.pitch = 0.25;
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        create_view_matrix(self.eye, self.target, self.up)
    }
}

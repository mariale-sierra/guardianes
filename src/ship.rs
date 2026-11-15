// ship.rs
// Nave del jugador: estado, vuelo directo, camara de seguimiento, carga de modelos y shader.
//
// Convenciones (ver CLAUDE.md, seccion 5): Y arriba, la NARIZ apunta a -Z local, arriba +Y, derecha +X.
// Todo movimiento usa `dt` (segundos reales), nunca "una constante por cuadro".

use crate::fragment::Fragment;
use crate::matrix::{create_model_matrix, create_view_matrix, Mat4};
use crate::obj;
use crate::planet_shaders::smoothstep;
use crate::texture;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use raylib::prelude::Vector3;

pub const SHADER_SHIP: usize = 30;

// ------------------------------------------------------------ parametros demostrables (cambiar en vivo)
/// Tamano de la nave en el mundo. El modelo mide 2.0 de largo, asi que la nave mide 2*SHIP_SCALE.
pub const SHIP_SCALE: f32 = 0.22;
/// Velocidad de avance (unidades de mundo por segundo) a acelerador completo.
pub const MOVE_SPEED: f32 = 3.0;
/// Velocidad de giro (radianes por segundo).
pub const TURN_SPEED: f32 = 1.5;
/// Camara de seguimiento: cuanto detras y cuanto arriba de la nave, y a que distancia delante mira.
pub const CAM_BACK: f32 = 1.1;
pub const CAM_UP: f32 = 0.30;
pub const CAM_LOOK_AHEAD: f32 = 0.4;
/// Radio de la esfera de colision de la nave (para que no atraviese planetas).
pub const SHIP_RADIUS: f32 = 0.2;
/// Limite del cabeceo para no pasar por el cenit (donde yaw/pitch se vuelven ambiguos).
pub const PITCH_LIMIT: f32 = 1.3;

/// Lo que el jugador pide en este cuadro, ya normalizado a [-1, 1] (asi ship.rs no depende de raylib).
#[derive(Default, Clone, Copy)]
pub struct ShipInput {
    pub yaw: f32,      // +1 = girar a la izquierda
    pub pitch: f32,    // +1 = subir la nariz
    pub throttle: f32, // +1 = avanzar, -1 = retroceder
}

pub struct Ship {
    pub position: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub scale: f32,
    pub move_speed: f32,
    pub turn_speed: f32,
    /// Indice en la lista de modelos (para el selector de nave).
    pub model: usize,
}

impl Ship {
    pub fn new(position: Vector3) -> Self {
        Ship { position, yaw: 0.0, pitch: 0.0, scale: SHIP_SCALE, move_speed: MOVE_SPEED, turn_speed: TURN_SPEED, model: 0 }
    }

    /// Gira la nave para que la nariz mire hacia `target`.
    pub fn look_at(&mut self, target: Vector3) {
        let f = (target - self.position).normalized();
        self.pitch = f.y.clamp(-1.0, 1.0).asin().clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.yaw = (-f.x).atan2(-f.z); // inversa de forward = (-sin(yaw)cos(pitch), sin(pitch), -cos(yaw)cos(pitch))
    }

    /// "Viaje rapido": coloca la nave a `distance` de `center`, del lado que mira a `towards` (el sol), mirando al centro.
    pub fn place_near(&mut self, center: Vector3, towards: Vector3, distance: f32) {
        let dir = (towards - center).normalized();
        // un poco de altura para ver el planeta de lado y no desde el plano orbital exacto
        let offset = (dir + Vector3::new(0.0, 0.25, 0.0)).normalized() * distance;
        self.position = center + offset;
        self.look_at(center);
    }

    /// Hacia donde apunta la nariz.
    pub fn forward(&self) -> Vector3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vector3::new(-sy * cp, sp, -cy * cp)
    }

    /// "Arriba" de la nave (perpendicular a forward).
    pub fn up(&self) -> Vector3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vector3::new(sy * sp, cp, cy * sp)
    }

    /// Avanza la simulacion `dt` segundos.
    pub fn update(&mut self, input: ShipInput, dt: f32) {
        self.yaw += input.yaw * self.turn_speed * dt;
        self.pitch = (self.pitch + input.pitch * self.turn_speed * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.position = self.position + self.forward() * (input.throttle * self.move_speed * dt);
    }

    /// Colision basica nave-esfera: si la nave quedo dentro de (radio + SHIP_RADIUS) se la empuja hacia afuera,
    /// a lo largo de la recta que une el centro del planeta con la nave. Asi no atraviesa el planeta y se
    /// "desliza" por su superficie. Devuelve true si hubo contacto.
    pub fn collide(&mut self, center: Vector3, radius: f32) -> bool {
        let to_ship = self.position - center;
        let dist = to_ship.length();
        let min_dist = radius + SHIP_RADIUS;
        if dist >= min_dist - 1e-4 {
            return false;
        }
        // si la nave quedo exactamente en el centro (caso raro) se la saca hacia atras
        let dir = if dist > 1e-6 { to_ship * (1.0 / dist) } else { -self.forward() };
        self.position = center + dir * min_dist;
        true
    }

    /// Matriz de modelo: traslacion * rotacion(pitch, yaw, 0) * escala.
    pub fn model_matrix(&self) -> Mat4 {
        create_model_matrix(self.position, self.scale, Vector3::new(self.pitch, self.yaw, 0.0))
    }

    /// Camara de seguimiento: (ojo, objetivo, arriba).
    pub fn follow_camera(&self) -> (Vector3, Vector3, Vector3) {
        let (f, up) = (self.forward(), self.up());
        let eye = self.position - f * CAM_BACK + up * CAM_UP;
        let target = self.position + f * CAM_LOOK_AHEAD;
        (eye, target, up)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let (eye, target, up) = self.follow_camera();
        create_view_matrix(eye, target, up)
    }
}

// ------------------------------------------------------------ modelos

/// Archivos .obj de las naves disponibles (el selector recorre esta lista).
/// Para agregar la segunda nave: subir su .obj a assets/models/ships/, su PNG a assets/textures/ships/,
/// registrar su textura en texture.rs y anadir su ruta aqui.
pub const SHIP_MODEL_FILES: &[&str] = &["assets/models/ships/milano.obj"];

/// Carga todas las naves UNA vez. Si un .obj falla, se avisa y se usa una piramide provisional.
pub fn load_models() -> Vec<Vec<Vertex>> {
    SHIP_MODEL_FILES
        .iter()
        .map(|path| match obj::load(path) {
            Ok(m) if !m.is_empty() => m,
            Ok(_) => {
                eprintln!("aviso: {path} no tiene triangulos; se usa una nave provisional");
                placeholder()
            }
            Err(e) => {
                eprintln!("aviso: {e}; se usa una nave provisional");
                placeholder()
            }
        })
        .collect()
}

/// Tetraedro con la nariz en -Z, para que el juego arranque aunque falle el modelo.
fn placeholder() -> Vec<Vertex> {
    let (nose, l, r, top) = (
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(-0.6, 0.0, 0.8),
        Vector3::new(0.6, 0.0, 0.8),
        Vector3::new(0.0, 0.3, 0.6),
    );
    let mut out = Vec::new();
    // cada cara: tres puntos CCW vistos desde fuera
    for (a, b, c) in [(nose, l, top), (nose, top, r), (nose, r, l), (l, r, top)] {
        let n = (b - a).cross(c - a).normalized();
        for p in [a, b, c] {
            out.push(Vertex::new(p, n, (0.5, 0.5)));
        }
    }
    out
}

// ------------------------------------------------------------ shader de la nave

/// Color = textura (por UV, bilineal) * luz del sol, mas brillo animado en los pixeles "emisivos"
/// (motores cian y luces amarillas de la textura). El brillo pulsa con el tiempo.
pub fn shade(f: &Fragment, u: &Uniforms, l: Vector3) -> Vector3 {
    let n = f.normal;
    let base = match texture::get("milano") {
        // OBJ: v = 1 es ARRIBA; en la imagen la fila 0 es la de arriba => se invierte v al muestrear
        Some(tx) => tx.sample(f.uv.0, 1.0 - f.uv.1),
        None => Vector3::new(0.7, 0.7, 0.75), // sin textura: gris
    };
    // luz difusa con ambiente alto: el sol queda lejos y la nave debe leerse siempre
    let d = n.dot(l).max(0.0);
    let mut out = base * (0.35 + 0.65 * d);

    // emisivo: cian (motores) y amarillo (luces de posicion)
    let cyan = smoothstep(0.55, 0.85, base.z) * smoothstep(0.45, 0.75, base.y) * (1.0 - smoothstep(0.25, 0.5, base.x));
    let yellow = smoothstep(0.85, 0.98, base.x) * smoothstep(0.80, 0.95, base.y) * (1.0 - smoothstep(0.5, 0.75, base.z));
    let pulse = 0.75 + 0.25 * (u.time * 4.0).sin();
    out = out + Vector3::new(0.3, 0.9, 1.0) * (cyan * pulse * 0.5) + Vector3::new(1.0, 0.9, 0.4) * (yellow * 0.5);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_apunta_la_nariz_al_objetivo() {
        let mut s = Ship::new(Vector3::new(1.0, 2.0, 3.0));
        let target = Vector3::new(-4.0, 0.5, -6.0);
        s.look_at(target);
        let want = (target - s.position).normalized();
        let got = s.forward();
        assert!((want - got).length() < 1e-4);
    }

    #[test]
    fn la_nave_no_atraviesa_un_planeta() {
        let center = Vector3::new(5.0, 0.0, 0.0);
        let mut s = Ship::new(Vector3::new(5.2, 0.0, 0.0)); // dentro de un planeta de radio 1
        assert!(s.collide(center, 1.0));
        assert!(((s.position - center).length() - (1.0 + SHIP_RADIUS)).abs() < 1e-3);
        assert!(!s.collide(center, 1.0)); // ya está fuera: no hay contacto
    }

    #[test]
    fn avanzar_de_frente_contra_un_planeta_se_detiene_en_su_superficie() {
        let center = Vector3::zero();
        let mut s = Ship::new(Vector3::new(0.0, 0.0, 6.0)); // mira a -Z, hacia el planeta
        for _ in 0..600 {
            s.update(ShipInput { throttle: 1.0, ..Default::default() }, 1.0 / 60.0);
            s.collide(center, 1.0);
        }
        assert!(s.position.length() >= 1.0 + SHIP_RADIUS - 1e-4);
    }
}

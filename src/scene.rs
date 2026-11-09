// scene.rs
// Sistema solar "Guardianes de la Galaxia".
//
// Índices de mallas (vector `meshes` en main.rs):
pub const MESH_SPHERE: usize = 0;
pub const MESH_KNOWHERE: usize = 1; // esfera deformada con mesh_gen::knowhere_shape
pub const MESH_RING: usize = 2; // toro de mesh_gen::torus(1.0, 0.08, ..)

use crate::planet_shaders::*;
use raylib::prelude::Vector3;

pub struct SceneObject {
    pub name: &'static str,
    pub position: Vector3,
    pub rotation: Vector3,
    pub scale: f32,
    pub shader_index: usize,
    pub mesh: usize,
    // órbita
    pub orbit_radius: f32,
    pub orbit_speed: f32, // rad/s
    pub orbit_phase: f32,
    pub spin_speed: f32, // rad/s (puede ser negativo)
    // jerarquía: si hay padre, la órbita y el offset se miden desde él (no desde el sol)
    pub parent: Option<usize>,
    pub offset: Vector3,
    pub tilt: f32, // inclinación fija sobre X (para anillos)
}

impl SceneObject {
    fn base(name: &'static str, scale: f32, shader: usize) -> Self {
        SceneObject {
            name,
            position: Vector3::zero(),
            rotation: Vector3::zero(),
            scale,
            shader_index: shader,
            mesh: MESH_SPHERE,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            orbit_phase: 0.0,
            spin_speed: 0.0,
            parent: None,
            offset: Vector3::zero(),
            tilt: 0.0,
        }
    }
    fn orbit(mut self, radius: f32, speed: f32, phase: f32, spin: f32) -> Self {
        self.orbit_radius = radius;
        self.orbit_speed = speed;
        self.orbit_phase = phase;
        self.spin_speed = spin;
        self
    }
    fn child(mut self, parent: usize, offset: Vector3) -> Self {
        self.parent = Some(parent);
        self.offset = offset;
        self
    }
}

/// IMPORTANTE: un hijo siempre debe aparecer DESPUÉS de su padre en el vector.
pub fn build_scene() -> Vec<SceneObject> {
    let mut o: Vec<SceneObject> = Vec::new();

    // 0: Sol
    o.push(SceneObject::base("Sol", 1.6, SHADER_SUN).orbit(0.0, 0.0, 0.0, 0.05));

    // 1: Xandar
    o.push(SceneObject::base("Xandar", 0.60, SHADER_XANDAR).orbit(4.2, 0.45, 0.0, 0.50));

    // 2: Tierra
    o.push(SceneObject::base("Tierra", 0.65, SHADER_EARTH).orbit(6.2, 0.33, 2.1, 0.80));

    // 3: Knowhere (geometría deformada)
    let mut k = SceneObject::base("Knowhere", 0.85, SHADER_KNOWHERE).orbit(8.6, 0.24, 4.0, 0.30);
    k.mesh = MESH_KNOWHERE;
    o.push(k);

    // 4: Ego
    o.push(SceneObject::base("Ego", 0.90, SHADER_EGO).orbit(11.0, 0.18, 1.0, 0.25));

    // 5..9: Sovereign = núcleo + 4 esferas fusionadas
    let sov = o.len();
    o.push(SceneObject::base("Sovereign", 0.45, SHADER_SOVEREIGN).orbit(13.4, 0.14, 5.2, 0.40));
    let bits = [
        (Vector3::new(0.50, 0.15, 0.10), 0.30),
        (Vector3::new(-0.42, -0.10, 0.25), 0.27),
        (Vector3::new(0.05, 0.40, -0.40), 0.22),
        (Vector3::new(-0.10, -0.38, -0.30), 0.20),
    ];
    for (off, s) in bits {
        o.push(SceneObject::base("Sovereign", s, SHADER_SOVEREIGN).child(sov, off));
    }

    // 10..13: Orgoscope = núcleo + 3 anillos que giran en sentidos distintos
    let orgo = o.len();
    o.push(SceneObject::base("Orgoscope", 0.50, SHADER_ORGO_CORE).orbit(16.0, 0.11, 3.2, 0.30));
    for (scale, tilt, spin) in [(1.3, 0.0, 0.5), (1.9, 0.35, -0.35), (2.5, -0.25, 0.22)] {
        let mut r = SceneObject::base("Orgoscope", scale, SHADER_ORGO_RING).child(orgo, Vector3::zero());
        r.mesh = MESH_RING;
        r.tilt = tilt;
        r.spin_speed = spin;
        o.push(r);
    }
    o
}

/// Llamar UNA vez por cuadro después de actualizar uniforms.time.
/// Posición = centro (sol o padre) + offset + círculo orbital.
pub fn update_scene(objects: &mut [SceneObject], sun_position: Vector3, time: f32) {
    for i in 0..objects.len() {
        let center = match objects[i].parent {
            Some(p) => objects[p].position, // el padre ya se actualizó
            None => sun_position,
        };
        let o = &mut objects[i];
        let angle = o.orbit_phase + o.orbit_speed * time;
        o.position = Vector3::new(
            center.x + o.offset.x + o.orbit_radius * angle.cos(),
            center.y + o.offset.y,
            center.z + o.offset.z + o.orbit_radius * angle.sin(),
        );
        o.rotation = Vector3::new(o.tilt, o.spin_speed * time, 0.0);
    }
}

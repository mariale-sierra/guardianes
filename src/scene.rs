// scene.rs
// Sistema solar "Guardianes de la Galaxia".
//
// Índices de mallas (vector `meshes` en main.rs):
pub const MESH_SPHERE: usize = 0;
pub const MESH_KNOWHERE: usize = 1; // esfera deformada con mesh_gen::knowhere_shape
pub const MESH_RING: usize = 2; // toro de mesh_gen::torus(1.0, 0.08, ..)
// Modelos de Blender (assets/models/planets/*.obj), cargados en planet_models.rs:
pub const MESH_KNOWHERE_BLEND: usize = 3; // cráneo + muelle, SUBDIVIDIDO y DEFORMADO por código (process_knowhere)
pub const MESH_SOVEREIGN_BLEND: usize = 4; // núcleo + esferas fusionadas
pub const MESH_ORGOSCOPE_BLEND: usize = 5; // modelo completo: núcleo, anillos, ejes y masas
pub const MESH_KNOWHERE_PLAIN: usize = 6; // el mismo Knowhere SIN deformar (para comparar en la demo)

use crate::planet_shaders::*;

/// Tamaño del Orgoscope (el modelo de Blender mide 1 de radio).
pub const ORGOSCOPE_SCALE: f32 = 1.6;
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
    /// Radio de la esfera de colisión de la nave (0 = no colisiona; los hijos usan la del padre).
    pub collision_radius: f32,
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
            collision_radius: scale,
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
        self.collision_radius = 0.0; // el padre ya cubre la colisión
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

    // 3: Knowhere (modelo de Blender; se subdivide y se deforma por código: planet_models::process_knowhere)
    let mut k = SceneObject::base("Knowhere", 0.95, SHADER_KNOWHERE).orbit(8.6, 0.24, 4.0, 0.30);
    k.mesh = MESH_KNOWHERE_BLEND;
    o.push(k);

    // 4: Ego
    o.push(SceneObject::base("Ego", 0.90, SHADER_EGO).orbit(11.0, 0.18, 1.0, 0.25));

    // 5: Sovereign (modelo de Blender completo: núcleo + esferas fusionadas)
    let mut sov = SceneObject::base("Sovereign", 0.85, SHADER_SOVEREIGN).orbit(13.4, 0.14, 5.2, 0.40);
    sov.mesh = MESH_SOVEREIGN_BLEND;
    o.push(sov);

    // 6: Orgoscope (modelo de Blender completo: núcleo + 4 anillos inclinados + ejes + masas)
    let mut orgo = SceneObject::base("Orgoscope", ORGOSCOPE_SCALE, SHADER_ORGO_CORE).orbit(16.0, 0.11, 3.2, 0.30);
    orgo.mesh = MESH_ORGOSCOPE_BLEND;
    o.push(orgo);
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

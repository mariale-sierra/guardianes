// Geometría procedural generada ANTES de rasterizar.
use crate::planet_shaders::{fbm, smoothstep};
use crate::scene::{MESH_KNOWHERE, MESH_RING, MESH_SPHERE};
use crate::vertex::Vertex;
use raylib::prelude::Vector3;
use std::f32::consts::PI;

/// Esfera UV de radio 1, triángulos CCW vistos desde fuera.
pub fn uv_sphere(slices: usize, stacks: usize) -> Vec<Vertex> {
    let point = |i: usize, j: usize| -> Vertex {
        let th = 2.0 * PI * i as f32 / slices as f32;
        let ph = PI * j as f32 / stacks as f32;
        let p = Vector3::new(ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin());
        Vertex::new(p, p, (i as f32 / slices as f32, j as f32 / stacks as f32))
    };
    let mut out = Vec::with_capacity(slices * stacks * 6);
    for i in 0..slices {
        for j in 0..stacks {
            let (a, b, c, d) = (point(i, j), point(i + 1, j), point(i + 1, j + 1), point(i, j + 1));
            out.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    out
}

/// Toro en el plano XZ, triángulos CCW vistos desde fuera.
pub fn torus(major: f32, minor: f32, segments: usize, sides: usize) -> Vec<Vertex> {
    let point = |i: usize, j: usize| -> Vertex {
        let a = 2.0 * PI * (i % segments) as f32 / segments as f32;
        let b = 2.0 * PI * (j % sides) as f32 / sides as f32;
        let center = Vector3::new(major * a.cos(), 0.0, major * a.sin());
        let normal = Vector3::new(b.cos() * a.cos(), b.sin(), b.cos() * a.sin());
        Vertex::new(center + normal * minor, normal, (i as f32 / segments as f32, j as f32 / sides as f32))
    };
    let mut out = Vec::with_capacity(segments * sides * 6);
    for i in 0..segments {
        for j in 0..sides {
            let q = [point(i, j), point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)];
            for k in [0usize, 2, 1, 0, 3, 2] {
                out.push(q[k]);
            }
        }
    }
    out
}

/// Forma de "cabeza" de Knowhere a partir de un punto de la esfera unitaria.
pub fn knowhere_shape(d: Vector3) -> Vector3 {
    let r = 1.0 + 0.22 * (fbm(d * 2.5, 4) - 0.5);
    let mut p = d * r;
    p = Vector3::new(p.x * 0.85, p.y * 1.10, p.z);

    // cuenca ocular hundida
    let eye = Vector3::new(0.55, 0.15, 0.82).normalized();
    p = p * (1.0 - 0.45 * (1.0 - smoothstep(0.0, 0.45, (d - eye).length())));

    // espinas de cristal
    let spikes = [
        Vector3::new(-0.2, 0.95, -0.3),
        Vector3::new(0.3, 0.9, -0.4),
        Vector3::new(0.0, 0.7, -0.8),
        Vector3::new(0.6, -0.6, 0.5),
        Vector3::new(0.3, -0.85, 0.4),
    ];
    for s in spikes {
        p = p * (1.0 + 0.5 * (1.0 - smoothstep(0.0, 0.18, (d - s.normalized()).length())));
    }
    p
}

/// Deforma una esfera y recalcula normales planas por triángulo (suavizadas por posición).
pub fn deform_knowhere(sphere: &[Vertex]) -> Vec<Vertex> {
    let mut v: Vec<Vertex> = sphere
        .iter()
        .map(|s| {
            let mut c = *s;
            c.position = knowhere_shape(s.position.normalized());
            c
        })
        .collect();
    for tri in v.chunks_exact_mut(3) {
        let n = (tri[1].position - tri[0].position).cross(tri[2].position - tri[0].position);
        let n = if n.length() > 1e-8 { n.normalized() } else { tri[0].normal };
        let flat = n;
        for t in tri.iter_mut() {
            // mezcla de la normal radial y la de la cara: relieve visible pero no facetado
            t.normal = (t.position.normalized() * 0.5 + flat * 0.5).normalized();
        }
    }
    v
}

/// Mallas en el orden de scene::MESH_*.
pub fn build_meshes() -> Vec<Vec<Vertex>> {
    let sphere = uv_sphere(48, 32);
    let knowhere = deform_knowhere(&uv_sphere(64, 48));
    let ring = torus(1.0, 0.08, 64, 12);
    let mut meshes = vec![Vec::new(); 3];
    meshes[MESH_SPHERE] = sphere;
    meshes[MESH_KNOWHERE] = knowhere;
    meshes[MESH_RING] = ring;
    meshes
}

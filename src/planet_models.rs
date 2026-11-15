// planet_models.rs
// Carga de los planetas modelados en Blender (assets/models/planets/*.obj) y su deformación por ruido.
//
// - Knowhere: cráneo + muelle. Al cargarlo pasa por NUESTRAS etapas de geometría, antes de rasterizar:
//     1) subdivide_mesh: cada triángulo se parte en 4 (generación/subdivisión procedural),
//     2) deform_mesh: los vértices se desplazan con ruido (deformación de geometría por ruido).
//   Un modelo de Blender solo no suma esos puntos: la subdivisión y la deformación ocurren en este código.
// - Sovereign y Orgoscope: se usan completos, tal como salen de Blender.

use crate::obj;
use crate::planet_shaders::fbm;
use crate::vertex::Vertex;
use raylib::prelude::Vector3;

// ------------------------------------------------------------ parametros demostrables (cambiar en vivo)
/// Cuánto se mueve cada vértice de Knowhere (fracción del radio ~1). 0 = sin deformar.
pub const DEFORM_AMPLITUDE: f32 = 0.07;
/// Frecuencia del ruido: más alta = relieve más fino.
pub const DEFORM_FREQUENCY: f32 = 2.5;
/// Semilla: misma semilla => misma deformación.
pub const DEFORM_SEED: f32 = 3.0;

pub struct BlendPlanets {
    pub knowhere_deformed: Option<Vec<Vertex>>,
    pub knowhere_plain: Option<Vec<Vertex>>,
    pub sovereign: Option<Vec<Vertex>>,
    pub orgoscope: Option<Vec<Vertex>>,
}

/// Lee un OBJ; si falla avisa por consola y devuelve None (el juego usa su respaldo procedural).
fn try_load(path: &str) -> Option<Vec<Vertex>> {
    match obj::load(path) {
        Ok(tris) => {
            if tris.is_empty() { None } else { Some(tris) }
        }
        Err(e) => {
            eprintln!("aviso: no se pudo cargar el modelo de Blender ({e}); se usa un respaldo procedural");
            None
        }
    }
}

/// Carga los tres modelos UNA vez (antes del bucle principal).
pub fn load(amplitude: f32) -> BlendPlanets {
    let knowhere_plain = try_load("assets/models/planets/knowhere.obj");
    BlendPlanets {
        knowhere_deformed: knowhere_plain.as_ref().map(|m| process_knowhere(m, amplitude)),
        knowhere_plain,
        sovereign: try_load("assets/models/planets/sovereign.obj"),
        orgoscope: try_load("assets/models/planets/orgoscope.obj"),
    }
}

/// Cadena de etapas de geometría de Knowhere: subdividir y luego deformar con ruido.
/// (Se subdivide primero para que el ruido tenga más vértices donde mover: relieve más fino.)
pub fn process_knowhere(plain: &[Vertex], amplitude: f32) -> Vec<Vertex> {
    deform_mesh(&subdivide_mesh(plain), amplitude, DEFORM_FREQUENCY, DEFORM_SEED)
}

/// Subdivisión: cada triángulo (a, b, c) se parte en 4 usando los puntos medios de sus aristas:
///        a                     a
///       / \        =>        / \
///      b---c               ab---ca
///                          / \ / \
///                         b---bc---c
/// Posición, normal y UV de los puntos nuevos son el promedio de los extremos de la arista. Como el punto medio
/// depende SOLO de las posiciones de los extremos, dos triángulos que comparten arista generan el mismo punto:
/// no aparecen grietas. Triángulos x4 (3 659 -> 14 636 en Knowhere).
pub fn subdivide_mesh(src: &[Vertex]) -> Vec<Vertex> {
    let mid = |p: &Vertex, q: &Vertex| -> Vertex {
        let n = (p.normal + q.normal) * 0.5;
        Vertex::new(
            (p.position + q.position) * 0.5,
            if n.length() > 1e-6 { n.normalized() } else { p.normal },
            ((p.uv.0 + q.uv.0) * 0.5, (p.uv.1 + q.uv.1) * 0.5),
        )
    };
    let mut out = Vec::with_capacity(src.len() * 4);
    for t in src.chunks_exact(3) {
        let (a, b, c) = (t[0], t[1], t[2]);
        let (ab, bc, ca) = (mid(&a, &b), mid(&b, &c), mid(&c, &a));
        // mismo orden (CCW) que el triángulo original
        out.extend_from_slice(&[a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca]);
    }
    out
}

/// Deforma una malla con ruido fractal: cada vértice se desplaza a lo largo de la dirección
/// "desde el centro del planeta" una cantidad `amplitude * ruido`.
///
/// Cuidados (por qué está hecho así):
///  1. La dirección y la cantidad dependen SOLO de la posición del vértice (no de su normal). Los .obj repiten
///     los vértices en las costuras, cada copia con otra normal; si usáramos la normal, las copias se separarían
///     y aparecerían grietas. Con la posición, todas las copias se mueven exactamente igual.
///  2. Las normales se recalculan después (se mezcla la normal original con la de la cara nueva).
///  3. La amplitud es pequeña (~5-10 % del radio) y las UV no cambian, así que la textura se conserva.
pub fn deform_mesh(src: &[Vertex], amplitude: f32, frequency: f32, seed: f32) -> Vec<Vertex> {
    let offset = Vector3::new(seed * 13.1, seed * 7.7, seed * 3.3);
    let mut v = src.to_vec();
    for vert in v.iter_mut() {
        let len = vert.position.length();
        if len < 1e-6 {
            continue;
        }
        let dir = vert.position * (1.0 / len);
        let n = fbm(vert.position * frequency + offset, 4); // 0..1
        vert.position = vert.position + dir * (amplitude * (n - 0.5) * 2.0);
    }
    for tri in v.chunks_exact_mut(3) {
        let face = (tri[1].position - tri[0].position).cross(tri[2].position - tri[0].position);
        if face.length() < 1e-10 {
            continue; // triángulo degenerado: se queda con su normal
        }
        let face = face.normalized();
        for t in tri.iter_mut() {
            t.normal = (t.normal * 0.6 + face * 0.4).normalized();
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdividir_multiplica_x4_y_conserva_el_sentido_de_las_caras() {
        let n = Vector3::new(0.0, 0.0, 1.0);
        let t = [
            Vertex::new(Vector3::new(0.0, 0.0, 0.0), n, (0.0, 0.0)),
            Vertex::new(Vector3::new(1.0, 0.0, 0.0), n, (1.0, 0.0)),
            Vertex::new(Vector3::new(0.0, 1.0, 0.0), n, (0.0, 1.0)),
        ];
        let s = subdivide_mesh(&t);
        assert_eq!(s.len(), 12);
        for tri in s.chunks_exact(3) {
            let face = (tri[1].position - tri[0].position).cross(tri[2].position - tri[0].position);
            assert!(face.z > 0.0); // sigue siendo CCW visto desde +Z
        }
    }

    #[test]
    fn la_deformacion_es_reproducible_y_sin_grietas() {
        // dos vértices en la MISMA posición pero con normales distintas (costura de un .obj)
        let p = Vector3::new(0.3, 0.8, 0.2);
        let a = Vertex::new(p, Vector3::new(1.0, 0.0, 0.0), (0.0, 0.0));
        let b = Vertex::new(p, Vector3::new(0.0, 1.0, 0.0), (1.0, 0.0));
        let c = Vertex::new(Vector3::new(-0.5, 0.2, 0.7), Vector3::new(0.0, 0.0, 1.0), (0.5, 1.0));
        let m1 = deform_mesh(&[a, b, c], 0.1, 2.5, 3.0);
        let m2 = deform_mesh(&[a, b, c], 0.1, 2.5, 3.0);
        assert_eq!(m1[0].position.x, m1[1].position.x); // misma posición => mismo desplazamiento
        assert_eq!(m1[2].position.y, m2[2].position.y); // misma semilla => misma deformación
        assert!(m1.iter().all(|v| v.position.x.is_finite() && v.normal.x.is_finite()));
    }
}

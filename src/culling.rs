// culling.rs
// Volumen visible de la cámara (frustum) como seis planos, y prueba conservadora esfera–frustum.
//
// Convenciones (las de matrix.rs): Mat4 por filas, vectores columna, `clip = (P·V)·(x, y, z, 1)ᵀ`.
// El volumen visible de OpenGL es `-w <= x, y, z <= w`. Cada una de las seis desigualdades es un
// semiespacio `a·x + b·y + c·z + d >= 0` cuyos coeficientes son suma/resta de filas de M = P·V.
// Los planos quedan en coordenadas de MUNDO (M lleva de mundo a clip), así que se comparan
// directamente con la posición de los objetos en el mundo.

use crate::matrix::Mat4;
use crate::triangle::Stats;
use crate::vertex::Vertex;
use raylib::prelude::Vector3;

/// Índices de los planos dentro de `Frustum::planes`.
pub const PLANE_LEFT: usize = 0;
pub const PLANE_RIGHT: usize = 1;
pub const PLANE_BOTTOM: usize = 2;
pub const PLANE_TOP: usize = 3;
pub const PLANE_NEAR: usize = 4;
pub const PLANE_FAR: usize = 5;

/// Plano que nunca descarta nada: se usa si un plano no se puede normalizar (comportamiento conservador).
const NEVER_CULLS: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

/// Los seis planos del frustum, normalizados, con la normal apuntando HACIA ADENTRO del volumen visible.
/// Un punto `p` está dentro del plano si `a·px + b·py + c·pz + d >= 0`; ese valor es la distancia
/// con signo (en unidades de mundo) de `p` al plano, porque la normal tiene longitud 1.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    planes: [[f64; 4]; 6],
}

impl Frustum {
    /// Construye el frustum de una cámara. Las matrices llegan en f32 (las del renderer) pero el producto
    /// P·V y todo el cálculo de los planos se hace en f64: el plano lejano se obtiene restando filas casi
    /// iguales y en f32 se perdería precisión.
    pub fn from_view_proj(view: &Mat4, proj: &Mat4) -> Frustum {
        let (v, p) = (to_f64(view), to_f64(proj));
        let m = mul_f64(&p, &v); // M = P · V (el mismo orden que usa render_object)
        let (r0, r1, r2, r3) = (m[0], m[1], m[2], m[3]);
        let raw = [
            add(r3, r0, 1.0),  // izquierdo:  w + x >= 0
            add(r3, r0, -1.0), // derecho:    w - x >= 0
            add(r3, r1, 1.0),  // inferior:   w + y >= 0
            add(r3, r1, -1.0), // superior:   w - y >= 0
            add(r3, r2, 1.0),  // cercano:    w + z >= 0
            add(r3, r2, -1.0), // lejano:     w - z >= 0
        ];
        Frustum { planes: raw.map(normalize_plane) }
    }

    /// Los seis planos `[a, b, c, d]` (ver las constantes `PLANE_*` para el orden).
    pub fn planes(&self) -> &[[f64; 4]; 6] {
        &self.planes
    }

    /// `true` = hay que dibujar la esfera; `false` = está COMPLETAMENTE fuera de algún plano y se puede descartar.
    ///
    /// Es conservadora: descarta solo si `distancia < -radio` para algún plano. Una esfera dentro, que cruza
    /// un plano, tangente, que contiene la cámara o que contiene todo el frustum se conserva. Centro o radio
    /// no finitos, o radio negativo, no se pueden clasificar con seguridad: también se conserva.
    pub fn sphere_visible(&self, center: Vector3, radius: f32) -> bool {
        if !(radius.is_finite() && radius >= 0.0 && center.x.is_finite() && center.y.is_finite() && center.z.is_finite()) {
            return true;
        }
        let (cx, cy, cz, r) = (center.x as f64, center.y as f64, center.z as f64, radius as f64);
        for p in &self.planes {
            let distance = p[0] * cx + p[1] * cy + p[2] * cz + p[3];
            if distance < -r {
                return false; // toda la esfera queda del lado de afuera de este plano
            }
            // Si distance es NaN la comparación es falsa y la esfera se conserva.
        }
        true
    }
}

/// Factor y suma del margen de `bounding_radius`. `radio_con_margen = radio_maximo * 1.02 + 0.01`.
///
/// Por qué un margen: el culling decide con el radio de la MALLA FINAL, pero el renderer trabaja en f32 y la
/// escena puede mover/rotar el objeto; un radio justo podría quedar ~1e-6 corto y descartar un objeto cuyo
/// borde aún toca el frustum (parpadeo en los bordes de pantalla). El 2 % cubre errores relativos y el 0.01
/// absoluto cubre mallas muy pequeñas. Descartar de más es un error visible; conservar de más solo cuesta
/// dibujar un objeto que casi no se ve. IMPORTANTE: el margen ya viene incluido aquí; al integrar el culling
/// NO se debe volver a aplicar (solo multiplicar por la escala del objeto).
pub const RADIUS_MARGIN_SCALE: f32 = 1.02;
pub const RADIUS_MARGIN_ABS: f32 = 0.01;

/// Radio de la esfera envolvente de una malla, CENTRADA EN EL ORIGEN LOCAL del objeto (que es donde el
/// renderer coloca `position`), ya con margen. Es la distancia máxima de un vértice al origen: si la
/// geometría no está centrada (p. ej. Sovereign) el radio simplemente crece, y sigue conteniéndola.
///
/// Se calcula UNA vez al preparar las mallas (no por cuadro). Para usarlo en el mundo: `radio * object.scale`
/// (la escala es uniforme y la rotación alrededor del origen no cambia las distancias).
///
/// Resultado conservador: malla vacía o con alguna posición no finita devuelve `f32::INFINITY`, que
/// `Frustum::sphere_visible` trata como "no clasificable" y conserva. No es `collision_radius`.
pub fn bounding_radius(vertices: &[Vertex]) -> f32 {
    if vertices.is_empty() {
        return f32::INFINITY;
    }
    let mut max_sq = 0.0f64; // en f64: x*x no desborda ni pierde precisión en f32
    for v in vertices {
        let p = v.position;
        if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
            return f32::INFINITY;
        }
        let sq = (p.x as f64).powi(2) + (p.y as f64).powi(2) + (p.z as f64).powi(2);
        max_sq = max_sq.max(sq);
    }
    (max_sq.sqrt() as f32) * RADIUS_MARGIN_SCALE + RADIUS_MARGIN_ABS
}

/// Decide si un objeto se dibuja Y actualiza los contadores de culling, en un solo lugar para que
/// `main.rs` y `snapshot.rs` cuenten igual. `center`/`radius` son los del MUNDO (radio local * escala);
/// `triangles` = triángulos de la malla que se habría enviado a `render_object`.
/// Devuelve `true` si hay que dibujarlo. Solo el frustum cuenta como "descarte" aquí.
pub fn consider_object(frustum: &Frustum, center: Vector3, radius: f32, triangles: usize, stats: &mut Stats) -> bool {
    stats.objects_total += 1;
    if frustum.sphere_visible(center, radius) {
        true
    } else {
        stats.objects_culled += 1;
        stats.triangles_culled += triangles as u64;
        false
    }
}

fn to_f64(m: &Mat4) -> [[f64; 4]; 4] {
    let mut r = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = m.0[i][j] as f64;
        }
    }
    r
}

/// a · b en f64 (mismo orden de índices que `impl Mul for Mat4`).
fn mul_f64(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

/// `a + sign * b`, componente a componente.
fn add(a: [f64; 4], b: [f64; 4], sign: f64) -> [f64; 4] {
    [a[0] + sign * b[0], a[1] + sign * b[1], a[2] + sign * b[2], a[3] + sign * b[3]]
}

/// Divide los cuatro coeficientes entre la longitud de la normal (a, b, c) para que `distancia` esté en
/// unidades de mundo. Si la longitud no es finita o es casi cero, devuelve un plano que no descarta nada.
fn normalize_plane(p: [f64; 4]) -> [f64; 4] {
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    if !len.is_finite() || len < 1e-12 {
        return NEVER_CULLS;
    }
    let n = [p[0] / len, p[1] / len, p[2] / len, p[3] / len];
    if n.iter().all(|c| c.is_finite()) { n } else { NEVER_CULLS }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::{create_projection_matrix, create_view_matrix};

    const EPS: f64 = 1e-9;
    // Parámetros reales del renderer (main.rs / snapshot.rs).
    const FOV: f32 = 60.0 * std::f32::consts::PI / 180.0;
    const ASPECT: f32 = 480.0 / 320.0;
    const NEAR: f32 = 0.1;
    const FAR: f32 = 200.0;

    fn v3(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3::new(x, y, z)
    }
    fn proj() -> Mat4 {
        create_projection_matrix(FOV, ASPECT, NEAR, FAR)
    }
    /// Cámara en el origen mirando -Z (la vista es la identidad).
    fn frustum_origin() -> Frustum {
        Frustum::from_view_proj(&create_view_matrix(v3(0.0, 0.0, 0.0), v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0)), &proj())
    }
    fn dist(p: &[f64; 4], c: Vector3) -> f64 {
        p[0] * c.x as f64 + p[1] * c.y as f64 + p[2] * c.z as f64 + p[3]
    }

    #[test]
    fn plane_extraction_matches_analytic_planes() {
        let f = frustum_origin();
        let th = (30f64.to_radians().tan()) * (ASPECT as f64); // tan(semi-FOV horizontal)
        let tv = 30f64.to_radians().tan();
        let expected = [[1.0, 0.0, -th], [-1.0, 0.0, -th], [0.0, 1.0, -tv], [0.0, -1.0, -tv], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0]];
        for (i, (p, e)) in f.planes().iter().zip(expected).enumerate() {
            let len = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
            for k in 0..3 {
                assert!((p[k] - e[k] / len).abs() < 1e-6, "plano {i}, componente {k}: {} vs {}", p[k], e[k] / len);
            }
            let norm = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((norm - 1.0).abs() < EPS, "plano {i} no normalizado: {norm}");
        }
        // los cuatro laterales pasan por la cámara; cercano a 0.1 y lejano a 200 delante de ella
        for i in 0..4 {
            assert!(f.planes()[i][3].abs() < 1e-6);
        }
        assert!(dist(&f.planes()[PLANE_NEAR], v3(0.0, 0.0, -0.1)).abs() < 1e-6);
        assert!(dist(&f.planes()[PLANE_FAR], v3(0.0, 0.0, -200.0)).abs() < 1e-1); // P en f32: el lejano tiene ~1e-2 de error
        // las normales apuntan hacia adentro: un punto en el eje a 10 u está dentro de los seis
        for (i, p) in f.planes().iter().enumerate() {
            assert!(dist(p, v3(0.0, 0.0, -10.0)) > 0.0, "normal invertida en el plano {i}");
        }
    }

    #[test]
    fn planes_follow_a_moved_camera() {
        // Cámara en (3,2,-5) mirando a (-1,0.5,2): el centro del volumen y la posición de la cámara deben ser coherentes.
        let eye = v3(3.0, 2.0, -5.0);
        let view = create_view_matrix(eye, v3(-1.0, 0.5, 2.0), v3(0.0, 1.0, 0.0));
        let f = Frustum::from_view_proj(&view, &proj());
        let fw = (v3(-1.0, 0.5, 2.0) - eye).normalized();
        // distancia con signo al plano cercano de un punto a 5 u: 5 - 0.1
        assert!((dist(&f.planes()[PLANE_NEAR], eye + fw * 5.0) - 4.9).abs() < 1e-4);
        assert!(f.sphere_visible(eye + fw * 5.0, 0.0));
        assert!(!f.sphere_visible(eye - fw * 5.0, 1.0)); // detrás
    }

    #[test]
    fn sphere_inside_is_visible() {
        assert!(frustum_origin().sphere_visible(v3(0.0, 0.0, -10.0), 1.0));
        assert!(frustum_origin().sphere_visible(v3(1.0, -1.0, -20.0), 0.5));
    }

    #[test]
    fn sphere_outside_each_of_the_six_planes_is_culled() {
        let f = frustum_origin();
        assert!(!f.sphere_visible(v3(-20.0, 0.0, -10.0), 1.0), "izquierdo");
        assert!(!f.sphere_visible(v3(20.0, 0.0, -10.0), 1.0), "derecho");
        assert!(!f.sphere_visible(v3(0.0, -20.0, -10.0), 1.0), "inferior");
        assert!(!f.sphere_visible(v3(0.0, 20.0, -10.0), 1.0), "superior");
        // cercano: centro en la cámara (0.1 detrás del plano cercano) y r = 0.05 < 0.1
        assert!(!f.sphere_visible(v3(0.0, 0.0, 0.0), 0.05), "cercano");
        // lejano
        assert!(!f.sphere_visible(v3(0.0, 0.0, -300.0), 1.0), "lejano");
    }

    #[test]
    fn each_plane_alone_culls_its_sphere() {
        // Comprueba plano por plano (no solo "alguno"): la distancia a ESE plano es < -r.
        let f = frustum_origin();
        let cases = [(PLANE_LEFT, v3(-20.0, 0.0, -10.0)), (PLANE_RIGHT, v3(20.0, 0.0, -10.0)), (PLANE_BOTTOM, v3(0.0, -20.0, -10.0)),
            (PLANE_TOP, v3(0.0, 20.0, -10.0)), (PLANE_NEAR, v3(0.0, 0.0, 0.0)), (PLANE_FAR, v3(0.0, 0.0, -300.0))];
        for (i, c) in cases {
            let r = if i == PLANE_NEAR { 0.05 } else { 1.0 };
            assert!(dist(&f.planes()[i], c) < -(r as f64), "plano {i}");
        }
    }

    #[test]
    fn sphere_behind_camera_is_culled() {
        let f = frustum_origin();
        assert!(!f.sphere_visible(v3(0.0, 0.0, 5.0), 1.0));
        assert!(!f.sphere_visible(v3(0.0, 0.0, 50.0), 10.0));
    }

    #[test]
    fn tangent_sphere_is_kept_and_just_outside_is_culled() {
        let f = frustum_origin();
        let n = f.planes()[PLANE_LEFT]; // pasa por el origen
        let (nx, nz) = (n[0] as f32, n[2] as f32);
        let r = 1.0f32;
        // c(off) desplaza el punto base (0,0,-10) `off` unidades en contra de la normal: la distancia pasa de d0 a d0 - off
        let c = |off: f32| v3(-nx * off, 0.0, -10.0 - nz * off);
        let d0 = dist(&n, v3(0.0, 0.0, -10.0)); // distancia del punto base
        let off_tangent = (d0 + r as f64 - 1e-5) as f32; // d = -r + 1e-5
        let off_outside = (d0 + r as f64 + 1e-3) as f32; // d = -r - 1e-3
        assert!(f.sphere_visible(c(off_tangent), r), "tangente (por dentro por 1e-5) debe conservarse");
        assert!(!f.sphere_visible(c(off_outside), r), "fuera por 1e-3 debe descartarse");
    }

    #[test]
    fn partially_inside_sphere_is_kept() {
        let f = frustum_origin();
        // centro justo sobre el plano izquierdo (d = 0): la mitad de la esfera está dentro
        let n = f.planes()[PLANE_LEFT];
        let t = -(n[2] * -10.0) / n[0]; // x tal que n·(x,0,-10) = 0
        assert!(f.sphere_visible(v3(t as f32, 0.0, -10.0), 1.0));
        // cruzando el plano cercano
        assert!(f.sphere_visible(v3(0.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn sphere_containing_camera_or_whole_frustum_is_kept() {
        let f = frustum_origin();
        assert!(f.sphere_visible(v3(0.0, 0.0, 0.0), 2.0)); // la cámara está dentro de la esfera
        assert!(f.sphere_visible(v3(0.0, 0.0, -100.0), 1000.0)); // contiene todo el frustum
    }

    #[test]
    fn zero_radius_is_a_point_test() {
        let f = frustum_origin();
        assert!(f.sphere_visible(v3(0.0, 0.0, -5.0), 0.0));
        assert!(!f.sphere_visible(v3(0.0, 0.0, -250.0), 0.0));
        assert!(!f.sphere_visible(v3(0.0, 0.0, 0.0), 0.0)); // 0.1 detrás del plano cercano
    }

    #[test]
    fn negative_radius_is_kept() {
        let f = frustum_origin();
        assert!(f.sphere_visible(v3(-20.0, 0.0, -10.0), -1.0)); // sería descartable con r>=0, pero r<0 es dato inválido
        assert!(f.sphere_visible(v3(0.0, 0.0, -5.0), -5.0));
    }

    #[test]
    fn non_finite_values_are_kept() {
        let f = frustum_origin();
        let far_out = v3(-20.0, 0.0, -10.0);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(f.sphere_visible(far_out, bad), "radio {bad}");
            assert!(f.sphere_visible(v3(bad, 0.0, -10.0), 1.0), "x {bad}");
            assert!(f.sphere_visible(v3(0.0, bad, -10.0), 1.0), "y {bad}");
            assert!(f.sphere_visible(v3(0.0, 0.0, bad), 1.0), "z {bad}");
        }
    }

    #[test]
    fn degenerate_matrices_never_cull() {
        // Matrices con NaN o ceros: los planos no se pueden normalizar -> se conserva todo.
        let nan = Mat4([[f32::NAN; 4]; 4]);
        let zero = Mat4([[0.0; 4]; 4]);
        for f in [Frustum::from_view_proj(&nan, &nan), Frustum::from_view_proj(&zero, &zero)] {
            assert!(f.sphere_visible(v3(-1000.0, 0.0, 1000.0), 0.1));
            assert!(f.sphere_visible(v3(0.0, 0.0, -10.0), 1.0));
        }
    }

    /// E6c (capturas_P2): ojo a 2.0 u de la Tierra (radio envolvente 0.663), con el CENTRO a 48° del eje de
    /// la cámara (el semicampo horizontal es 40.89°). El centro queda fuera, pero la esfera cruza el borde.
    #[test]
    fn e6c_center_outside_but_sphere_intersects_is_kept() {
        let f = frustum_origin();
        let a = 48f32.to_radians();
        let center = v3(2.0 * a.sin(), 0.0, -2.0 * a.cos()); // a la derecha del eje
        let right = f.planes()[PLANE_RIGHT];
        let d = dist(&right, center);
        assert!(d < 0.0, "el centro debe estar fuera del plano derecho (d = {d})");
        assert!(d > -0.663, "pero a menos de un radio (d = {d})");
        assert!(f.sphere_visible(center, 0.663), "la esfera intersecta el frustum: se conserva");
        assert!(!f.sphere_visible(center, 0.1), "una esfera pequeña en el mismo sitio sí queda fuera");
    }

    // ------------------------------------------------------------ bounding_radius
    fn vert(x: f32, y: f32, z: f32) -> Vertex {
        Vertex::new(v3(x, y, z), v3(0.0, 1.0, 0.0), (0.0, 0.0))
    }
    /// Radio sin margen, para comparar con valores conocidos.
    fn raw(r: f32) -> f32 {
        r * RADIUS_MARGIN_SCALE + RADIUS_MARGIN_ABS
    }

    #[test]
    fn radius_of_vertices_on_a_known_sphere() {
        let m = [vert(1.0, 0.0, 0.0), vert(0.0, 1.0, 0.0), vert(0.0, 0.0, -1.0), vert(0.6, 0.8, 0.0)];
        assert!((bounding_radius(&m) - raw(1.0)).abs() < 1e-6);
    }

    #[test]
    fn farthest_vertex_decides_the_radius() {
        let m = [vert(0.1, 0.1, 0.1), vert(2.0, 3.0, 6.0), vert(-1.0, 0.0, 0.0)]; // |(2,3,6)| = 7
        assert!((bounding_radius(&m) - raw(7.0)).abs() < 1e-5);
    }

    #[test]
    fn negative_coordinates_count_by_distance() {
        let m = [vert(-3.0, -4.0, 0.0), vert(0.0, 0.0, -1.0)]; // |(-3,-4,0)| = 5
        assert!((bounding_radius(&m) - raw(5.0)).abs() < 1e-5);
    }

    #[test]
    fn mesh_centered_at_origin_is_symmetric() {
        let a = [vert(2.0, 0.0, 0.0), vert(-2.0, 0.0, 0.0)];
        let b = [vert(0.0, 2.0, 0.0), vert(0.0, -2.0, 0.0)];
        assert!((bounding_radius(&a) - raw(2.0)).abs() < 1e-6);
        assert_eq!(bounding_radius(&a), bounding_radius(&b));
    }

    #[test]
    fn empty_mesh_is_never_culled() {
        let r = bounding_radius(&[]);
        assert!(r.is_infinite());
        assert!(frustum_origin().sphere_visible(v3(-1000.0, 0.0, 1000.0), r)); // fuera de todo, pero se conserva
    }

    #[test]
    fn non_finite_vertices_give_a_conservative_radius() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for m in [[vert(1.0, 0.0, 0.0), vert(bad, 0.0, 0.0)], [vert(0.0, bad, 0.0), vert(1.0, 0.0, 0.0)], [vert(1.0, 0.0, 0.0), vert(0.0, 0.0, bad)]] {
                let r = bounding_radius(&m);
                assert!(r.is_infinite() && r > 0.0, "{bad}: {r}");
                assert!(frustum_origin().sphere_visible(v3(-1000.0, 0.0, 1000.0), r));
            }
        }
    }

    #[test]
    fn every_vertex_lies_inside_the_radius() {
        // malla pseudoaleatoria determinista (LCG) con valores positivos y negativos
        let mut s = 12345u32;
        let mut rnd = || { s = s.wrapping_mul(1664525).wrapping_add(1013904223); (s >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0 };
        let m: Vec<Vertex> = (0..500).map(|_| vert(rnd() * 3.0, rnd() * 3.0, rnd() * 3.0)).collect();
        let r = bounding_radius(&m);
        assert!(m.iter().all(|v| v.position.length() <= r));
        // y el margen deja holgura real: al menos 0.01 por encima del vértice más lejano
        let max = m.iter().map(|v| v.position.length()).fold(0.0, f32::max);
        assert!(r >= max + RADIUS_MARGIN_ABS);
    }

    #[test]
    fn margin_is_applied_exactly_once() {
        let m = [vert(10.0, 0.0, 0.0)];
        assert!((bounding_radius(&m) - 10.21).abs() < 1e-5); // 10 * 1.02 + 0.01
        assert!((bounding_radius(&[vert(0.0, 0.0, 0.0)]) - 0.01).abs() < 1e-7); // un solo punto en el origen: solo margen absoluto
    }

    #[test]
    fn offset_geometry_like_sovereign_is_still_contained() {
        // geometría lejos del origen (pivote en el núcleo, masas desplazadas): el radio se mide desde el ORIGEN, no desde su centro
        let m = [vert(1.0, 0.0, 0.0), vert(3.0, 0.0, 0.0), vert(2.0, 1.0, 0.0), vert(2.0, -1.0, 0.0)];
        let r = bounding_radius(&m);
        assert!((r - raw(3.0)).abs() < 1e-5); // el más lejano: (3,0,0)
        assert!(m.iter().all(|v| v.position.length() <= r));
        // si en vez de eso se midiera desde el centro de la caja (2,0,0) el radio sería ~1.4 y NO contendría el origen: sería un bug
        assert!(r > 3.0);
    }

    #[test]
    fn duplicated_triangles_do_not_change_the_radius() {
        let tri = [vert(0.0, 0.0, 0.0), vert(2.0, 0.0, 0.0), vert(0.0, 0.0, -1.5)];
        let mut many = Vec::new();
        for _ in 0..50 { many.extend_from_slice(&tri); }
        assert_eq!(bounding_radius(&tri), bounding_radius(&many));
    }

    /// Mallas reales del proyecto (se ejecuta desde la raíz del paquete, donde están assets/).
    /// Comprueba que el radio es finito, positivo y contiene todos los vértices; con `--nocapture` imprime los radios.
    #[test]
    fn real_meshes_have_finite_containing_radii() {
        let names = ["esfera", "Knowhere proc.", "toro", "Knowhere Blender (deformado)", "Sovereign", "Orgoscope", "Knowhere plano"];
        let meshes = crate::mesh_gen::build_meshes();
        let ships = crate::ship::load_models();
        let all = meshes.iter().zip(names.iter().map(|s| s.to_string())).chain(ships.iter().zip((0..).map(|i| format!("nave {i}"))));
        for (m, name) in all {
            let r = bounding_radius(m);
            let max = m.iter().map(|v| v.position.length()).fold(0.0, f32::max);
            println!("radio {name:30} tris {:6}  max|v| {max:.4}  radio con margen {r:.4}", m.len() / 3);
            assert!(r.is_finite() && r > 0.0, "{name}: {r}");
            assert!(m.iter().all(|v| v.position.length() <= r), "{name}");
        }
    }

    // ------------------------------------------------------------ contadores (consider_object)
    #[test]
    fn counters_visible_and_culled_objects() {
        let f = frustum_origin();
        let mut st = Stats::default();
        assert!(consider_object(&f, v3(0.0, 0.0, -10.0), 1.0, 100, &mut st)); // dentro
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (1, 0, 0));
        assert!(!consider_object(&f, v3(-50.0, 0.0, -10.0), 1.0, 250, &mut st)); // fuera
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (2, 1, 250));
        assert!(!consider_object(&f, v3(0.0, 0.0, 30.0), 1.0, 40, &mut st)); // detrás
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (3, 2, 290));
    }

    #[test]
    fn counters_for_ship_visible_and_outside() {
        // La nave pasa por la misma función con su posición y radio de mundo (radio de modelo * escala).
        let f = frustum_origin();
        let ship_r = 1.3685 * crate::ship::SHIP_SCALE;
        let mut st = Stats::default();
        assert!(consider_object(&f, v3(0.0, -0.3, -1.1), ship_r, 102, &mut st)); // delante de la cámara, como en el juego
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (1, 0, 0));
        assert!(!consider_object(&f, v3(0.0, 0.0, 20.0), ship_r, 102, &mut st)); // detrás
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (2, 1, 102));
    }

    #[test]
    fn counters_empty_mesh_counts_no_triangles() {
        let f = frustum_origin();
        let mut st = Stats::default();
        // radio infinito (malla vacía): se conserva, cuenta como objeto considerado y no suma triángulos descartados
        assert!(consider_object(&f, v3(-1000.0, 0.0, 1000.0), bounding_radius(&[]), 0, &mut st));
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (1, 0, 0));
        // malla vacía realmente fuera (radio finito): se descarta pero suma 0 triángulos
        assert!(!consider_object(&f, v3(-1000.0, 0.0, 1000.0), 1.0, 0, &mut st));
        assert_eq!((st.objects_total, st.objects_culled, st.triangles_culled), (2, 1, 0));
    }

    /// Conservación: lo que llega a render_object (`triangles_in`) más lo descartado es EXACTAMENTE la geometría considerada.
    #[test]
    fn triangles_in_plus_culled_equals_total_geometry() {
        use crate::{framebuffer::Framebuffer, matrix::{create_model_matrix, Mat4}, mesh_gen::uv_sphere, render::render_object, uniforms::Uniforms};
        let view = create_view_matrix(v3(0.0, 0.0, 0.0), v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0));
        let u0 = Uniforms { model_matrix: Mat4::identity(), view_matrix: view, projection_matrix: proj(), camera_position: v3(0.0, 0.0, 0.0),
            light_position: v3(0.0, 0.0, 0.0), time: 0.0, delta_time: 0.0, shader_index: 0 };
        let f = Frustum::from_view_proj(&u0.view_matrix, &u0.projection_matrix);
        let (small, big) = (uv_sphere(8, 6), uv_sphere(16, 12)); // 96 y 384 triángulos
        let (rs, rb) = (bounding_radius(&small), bounding_radius(&big));
        // (malla, radio local, posición, escala): 2 dentro, 3 fuera (izquierda, detrás, lejos)
        let objs = [(&small, rs, v3(0.0, 0.0, -5.0), 1.0), (&big, rb, v3(2.0, 0.0, -8.0), 0.5), (&small, rs, v3(-50.0, 0.0, -5.0), 1.0),
            (&big, rb, v3(0.0, 0.0, 10.0), 1.0), (&big, rb, v3(0.0, 0.0, -400.0), 2.0)];
        let total: usize = objs.iter().map(|o| o.0.len() / 3).sum();
        let mut fb = Framebuffer::new(64, 48);
        let (mut st, mut u) = (Stats::default(), u0);
        for (mesh, r, pos, scale) in objs {
            if !consider_object(&f, pos, r * scale, mesh.len() / 3, &mut st) { continue; }
            u.model_matrix = create_model_matrix(pos, scale, v3(0.0, 0.0, 0.0));
            render_object(&mut fb, mesh, &u, true, &mut st);
        }
        assert_eq!((st.objects_total, st.objects_culled), (5, 3));
        assert_eq!(st.triangles_culled as usize, 96 + 384 + 384);
        assert_eq!(st.triangles_in as usize, 96 + 384);
        assert_eq!((st.triangles_in + st.triangles_culled) as usize, total);
    }
}

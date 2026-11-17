// culling.rs
// Volumen visible de la cámara (frustum) como seis planos, y prueba conservadora esfera–frustum.
//
// Convenciones (las de matrix.rs): Mat4 por filas, vectores columna, `clip = (P·V)·(x, y, z, 1)ᵀ`.
// El volumen visible de OpenGL es `-w <= x, y, z <= w`. Cada una de las seis desigualdades es un
// semiespacio `a·x + b·y + c·z + d >= 0` cuyos coeficientes son suma/resta de filas de M = P·V.
// Los planos quedan en coordenadas de MUNDO (M lleva de mundo a clip), así que se comparan
// directamente con la posición de los objetos en el mundo.

use crate::matrix::Mat4;
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
}

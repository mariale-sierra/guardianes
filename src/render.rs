use crate::framebuffer::Framebuffer;
use crate::matrix::Mat4;
use crate::triangle::{draw_triangle, ScreenVertex, Stats};
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use raylib::prelude::Vector3;

/// Vértice en ESPACIO DE RECORTE (clip space): ya multiplicado por MVP pero SIN dividir entre w.
///
/// Por qué existe (y por qué no basta `ScreenVertex`): el recorte contra el plano cercano debe
/// hacerse ANTES de la división de perspectiva. `ScreenVertex` ya tiene x,y en píxeles y `inv_w`;
/// si w <= 0 esa división no tiene sentido. Aquí conservamos (x, y, z, w) para poder calcular
/// intersecciones, y junto con ellos todos los atributos que el rasterizador necesita, de modo que
/// un vértice nuevo creado por el recorte (en la arista entre dos vértices) pueda obtener sus
/// atributos interpolando linealmente: `a + (b - a) * t`. En clip space esa interpolación lineal
/// es correcta (la corrección de perspectiva ocurre después, en el rasterizador).
///
/// La usa `clip_triangle_near`, que `render_object` aplica a cada triángulo antes de proyectar.
#[derive(Clone, Copy)]
pub struct ClipVertex {
    pub clip: [f32; 4],   // (x, y, z, w) en clip space
    pub world: Vector3,   // posición en el mundo (para iluminación)
    pub normal: Vector3,  // normal en el mundo, ya normalizada
    pub local: Vector3,   // posición local del objeto (los patrones quedan pegados al objeto)
    pub uv: (f32, f32),   // coordenadas de textura
}

/// Distancia con signo de un vértice al plano cercano. El plano es `z + w = 0` y lo visible es `>= 0`.
/// (En clip space OpenGL, -w <= z <= w es el volumen visible; z = -w es la cara cercana.)
fn near_distance(v: &ClipVertex) -> f32 {
    v.clip[2] + v.clip[3]
}

/// Punto de la arista a→b donde cruza el plano cercano. `t` es la fracción del recorrido (0 = a, 1 = b);
/// TODOS los atributos se interpolan con el mismo `t`.
fn intersect_near(a: &ClipVertex, b: &ClipVertex, da: f32, db: f32) -> ClipVertex {
    // da y db tienen signos estrictamente opuestos, así que da - db != 0.
    let t = da / (da - db);
    let mut clip = [0.0; 4];
    for i in 0..4 {
        clip[i] = a.clip[i] + (b.clip[i] - a.clip[i]) * t;
    }
    ClipVertex {
        clip,
        world: a.world + (b.world - a.world) * t,
        // La normal interpolada no se renormaliza aquí: draw_triangle ya normaliza la del fragmento.
        normal: a.normal + (b.normal - a.normal) * t,
        local: a.local + (b.local - a.local) * t,
        uv: (a.uv.0 + (b.uv.0 - a.uv.0) * t, a.uv.1 + (b.uv.1 - a.uv.1) * t),
    }
}

/// Recorta un triángulo contra el plano cercano `z + w >= 0` (Sutherland–Hodgman con UN solo plano).
///
/// Devuelve hasta 2 triángulos y cuántos son válidos:
///   3 dentro -> el mismo triángulo        1 dentro -> 1 triángulo recortado
///   2 dentro -> cuadrilátero = 2 triángulos   0 dentro -> ninguno
/// Se recorre el contorno en el MISMO orden de entrada y los vértices nuevos se insertan en su lugar
/// sobre la arista, así que la orientación (CCW/CW) del triángulo original se conserva.
/// Un vértice exactamente sobre el plano (distancia 0) cuenta como dentro y no genera intersección extra.
/// Si algún componente no es finito (NaN/inf), el triángulo se descarta.
pub fn clip_triangle_near(tri: [ClipVertex; 3]) -> ([[ClipVertex; 3]; 2], usize) {
    let none = ([[tri[0]; 3]; 2], 0);
    if tri.iter().any(|v| v.clip.iter().any(|c| !c.is_finite())) {
        return none;
    }
    let d = [near_distance(&tri[0]), near_distance(&tri[1]), near_distance(&tri[2])];

    // Contorno recortado: máximo 4 vértices (un plano agrega a lo sumo 1 vértice a un triángulo).
    let mut poly: [ClipVertex; 4] = [tri[0]; 4];
    let mut n = 0;
    for i in 0..3 {
        let j = (i + 1) % 3;
        if d[i] >= 0.0 {
            poly[n] = tri[i];
            n += 1;
        }
        // Cruce estricto (un lado >0 y el otro <0): solo entonces hay intersección.
        if (d[i] > 0.0 && d[j] < 0.0) || (d[i] < 0.0 && d[j] > 0.0) {
            poly[n] = intersect_near(&tri[i], &tri[j], d[i], d[j]);
            n += 1;
        }
    }
    match n {
        3 => ([[poly[0], poly[1], poly[2]]; 2], 1),
        4 => ([[poly[0], poly[1], poly[2]], [poly[0], poly[2], poly[3]]], 2), // abanico
        _ => none, // 0, 1 o 2 puntos: no queda área
    }
}

/// Dibuja UN objeto. No limpia el framebuffer: todos los objetos del cuadro
/// comparten colores y profundidades.
///
/// Flujo por triángulo: MVP -> ClipVertex -> recorte contra el plano cercano (clip space, ANTES de
/// dividir entre w) -> división por w + viewport -> ScreenVertex -> draw_triangle.
pub fn render_object(
    fb: &mut Framebuffer,
    vertices: &[Vertex],
    uniforms: &Uniforms,
    cull_backfaces: bool,
    stats: &mut Stats,
) {
    // Combinación constante por objeto: se calcula UNA vez, no por vértice.
    let mvp = uniforms.projection_matrix * uniforms.view_matrix * uniforms.model_matrix;
    let (w, h) = (fb.width as f32, fb.height as f32);

    for tri in vertices.chunks_exact(3) {
        stats.triangles_in += 1;

        // Etapa de vértices: a clip space (sin dividir entre w) conservando los atributos.
        let clip_tri = [
            to_clip_vertex(&tri[0], &mvp, uniforms),
            to_clip_vertex(&tri[1], &mvp, uniforms),
            to_clip_vertex(&tri[2], &mvp, uniforms),
        ];

        // Recorte: devuelve 0, 1 o 2 triángulos (un cuadrilátero recortado se parte en dos).
        let (clipped, count) = clip_triangle_near(clip_tri);

        for t in &clipped[..count] {
            // Tras el recorte z + w >= 0; w puede valer 0 solo si z = 0 también, caso que no
            // es visible: se descarta para no dividir entre cero.
            let a = to_screen_vertex(&t[0], w, h);
            let b = to_screen_vertex(&t[1], w, h);
            let c = to_screen_vertex(&t[2], w, h);
            if let (Some(a), Some(b), Some(c)) = (a, b, c) {
                draw_triangle(fb, &a, &b, &c, uniforms, cull_backfaces, stats);
            }
        }
    }
}

/// Modelo -> clip space. La normal y la posición de mundo se calculan aquí para poder interpolarlas
/// si el recorte crea vértices nuevos.
fn to_clip_vertex(v: &Vertex, mvp: &Mat4, uniforms: &Uniforms) -> ClipVertex {
    ClipVertex {
        clip: mvp.mul_vec4(v.position, 1.0),
        world: uniforms.model_matrix.transform_point(v.position),
        normal: uniforms.model_matrix.transform_dir(v.normal).normalized(),
        local: v.position,
        uv: v.uv,
    }
}

/// Clip space -> pantalla: división de perspectiva y transformación al viewport.
/// `inv_w` se calcula con el w DEL VÉRTICE (también el de los vértices nuevos del recorte).
fn to_screen_vertex(v: &ClipVertex, w: f32, h: f32) -> Option<ScreenVertex> {
    let cw = v.clip[3];
    if cw <= 0.0 {
        return None;
    }
    let inv_w = 1.0 / cw;
    let (nx, ny, nz) = (v.clip[0] * inv_w, v.clip[1] * inv_w, v.clip[2] * inv_w);
    Some(ScreenVertex {
        x: (nx + 1.0) * 0.5 * w,
        y: (1.0 - ny) * 0.5 * h,
        z: nz,
        inv_w,
        world: v.world,
        normal: v.normal,
        local: v.local,
        uv: v.uv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    /// Vértice de prueba: clip = (x, y, z, w); world/local/normal/uv derivados para poder verificar interpolación.
    fn cv(x: f32, y: f32, z: f32, w: f32) -> ClipVertex {
        ClipVertex {
            clip: [x, y, z, w],
            world: Vector3::new(x * 10.0, y * 10.0, z * 10.0),
            normal: Vector3::new(x, y, 1.0),
            local: Vector3::new(w, z, y),
            uv: (x, y),
        }
    }
    // Dentro: z + w >= 0. Fuera: z + w < 0.
    fn inside(x: f32, y: f32) -> ClipVertex { cv(x, y, 0.5, 1.0) }   // d = 1.5
    fn outside(x: f32, y: f32) -> ClipVertex { cv(x, y, -1.5, 0.5) } // d = -1.0

    fn tris(r: &([[ClipVertex; 3]; 2], usize)) -> &[[ClipVertex; 3]] { &r.0[..r.1] }
    fn all_finite(v: &ClipVertex) -> bool {
        v.clip.iter().all(|c| c.is_finite()) && v.uv.0.is_finite() && v.normal.x.is_finite() && v.world.x.is_finite()
    }
    /// Área firmada en NDC (x/w, y/w): su signo es la orientación.
    fn signed_area(t: &[ClipVertex; 3]) -> f32 {
        let p = |v: &ClipVertex| (v.clip[0] / v.clip[3], v.clip[1] / v.clip[3]);
        let (a, b, c) = (p(&t[0]), p(&t[1]), p(&t[2]));
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }

    #[test]
    fn tres_dentro_se_conserva() {
        let r = clip_triangle_near([inside(0.0, 0.0), inside(1.0, 0.0), inside(0.0, 1.0)]);
        assert_eq!(r.1, 1);
        assert_eq!(r.0[0][1].clip, [1.0, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn cero_dentro_se_descarta() {
        let r = clip_triangle_near([outside(0.0, 0.0), outside(1.0, 0.0), outside(0.0, 1.0)]);
        assert_eq!(r.1, 0);
    }

    #[test]
    fn un_vertice_dentro_da_un_triangulo_y_vertices_nuevos_en_el_plano() {
        let r = clip_triangle_near([inside(0.0, 0.0), outside(1.0, 0.0), outside(0.0, 1.0)]);
        assert_eq!(r.1, 1);
        for v in &tris(&r)[0] {
            assert!(near_distance(v).abs() < EPS || v.clip == [0.0, 0.0, 0.5, 1.0]);
        }
    }

    #[test]
    fn dos_dentro_da_dos_triangulos_con_vertices_nuevos_en_el_plano() {
        let r = clip_triangle_near([inside(0.0, 0.0), inside(1.0, 0.0), outside(0.0, 1.0)]);
        assert_eq!(r.1, 2);
        let nuevos = tris(&r).iter().flatten().filter(|v| near_distance(v).abs() < EPS).count();
        assert!(nuevos >= 2);
        for v in tris(&r).iter().flatten() {
            assert!(near_distance(v) > -EPS); // nada queda detrás del plano
        }
    }

    #[test]
    fn atributos_se_interpolan_con_el_mismo_t() {
        // a: d = 1 (z=0,w=1), b: d = -1 (z=-2,w=1)  =>  t = 0.5 exacto.
        let a = ClipVertex { clip: [0.0, 0.0, 0.0, 1.0], world: Vector3::new(0.0, 0.0, 0.0), normal: Vector3::new(0.0, 0.0, 1.0), local: Vector3::new(2.0, 0.0, 0.0), uv: (0.0, 0.0) };
        let b = ClipVertex { clip: [4.0, 2.0, -2.0, 1.0], world: Vector3::new(8.0, 4.0, -4.0), normal: Vector3::new(0.0, 1.0, 0.0), local: Vector3::new(4.0, 6.0, 8.0), uv: (1.0, 0.5) };
        let m = intersect_near(&a, &b, 1.0, -1.0);
        assert_eq!(m.clip, [2.0, 1.0, -1.0, 1.0]);
        assert!((m.world - Vector3::new(4.0, 2.0, -2.0)).length() < EPS);
        assert!((m.normal - Vector3::new(0.0, 0.5, 0.5)).length() < EPS);
        assert!((m.local - Vector3::new(3.0, 3.0, 4.0)).length() < EPS);
        assert!((m.uv.0 - 0.5).abs() < EPS && (m.uv.1 - 0.25).abs() < EPS);
    }

    #[test]
    fn se_conserva_la_orientacion() {
        // CCW y CW con 1 y con 2 vértices dentro: todos los triángulos resultantes conservan el signo del área.
        let casos = [
            [inside(0.0, 0.0), outside(1.0, 0.0), outside(0.0, 1.0)],
            [inside(0.0, 0.0), inside(1.0, 0.0), outside(0.0, 1.0)],
            [outside(0.0, 0.0), inside(1.0, 0.0), inside(0.0, 1.0)],
        ];
        for base in casos {
            let ccw = signed_area(&base) > 0.0;
            for t in [base, [base[0], base[2], base[1]]] { // original y con orientación invertida
                let esperado_pos = if t[1].clip == base[1].clip { ccw } else { !ccw };
                let r = clip_triangle_near(t);
                assert!(r.1 >= 1);
                for out in tris(&r) {
                    assert_eq!(signed_area(out) > 0.0, esperado_pos);
                }
            }
        }
    }

    #[test]
    fn vertice_detras_de_la_camara_no_produce_nan() {
        // w negativo (detrás de la cámara) y w = 0 exacto.
        for w_atras in [-3.0, -0.0001, 0.0] {
            let r = clip_triangle_near([inside(0.0, 0.0), cv(1.0, 0.0, 2.0, w_atras), cv(0.0, 1.0, -5.0, w_atras)]);
            for v in tris(&r).iter().flatten() {
                assert!(all_finite(v));
            }
        }
    }

    #[test]
    fn entrada_no_finita_se_descarta() {
        let r = clip_triangle_near([inside(0.0, 0.0), cv(f32::NAN, 0.0, 0.5, 1.0), inside(0.0, 1.0)]);
        assert_eq!(r.1, 0);
        let r = clip_triangle_near([inside(0.0, 0.0), cv(f32::INFINITY, 0.0, 0.5, 1.0), inside(0.0, 1.0)]);
        assert_eq!(r.1, 0);
    }

    #[test]
    fn vertices_sobre_el_plano() {
        let en_plano = |x: f32, y: f32| cv(x, y, -1.0, 1.0); // d = 0 exacto
        // 3 sobre el plano: cuentan como dentro, se conserva.
        assert_eq!(clip_triangle_near([en_plano(0.0, 0.0), en_plano(1.0, 0.0), en_plano(0.0, 1.0)]).1, 1);
        // 1 sobre el plano y 2 fuera: no queda área.
        assert_eq!(clip_triangle_near([en_plano(0.0, 0.0), outside(1.0, 0.0), outside(0.0, 1.0)]).1, 0);
        // 2 sobre el plano y 1 fuera: no queda área.
        assert_eq!(clip_triangle_near([en_plano(0.0, 0.0), en_plano(1.0, 0.0), outside(0.0, 1.0)]).1, 0);
        // 1 sobre el plano y 2 dentro: se conserva entero, sin vértices duplicados.
        let r = clip_triangle_near([en_plano(0.0, 0.0), inside(1.0, 0.0), inside(0.0, 1.0)]);
        assert_eq!(r.1, 1);
        // 1 sobre el plano, 1 dentro, 1 fuera: un triángulo; sin duplicar el vértice del plano.
        let r = clip_triangle_near([en_plano(0.0, 0.0), inside(1.0, 0.0), outside(0.0, 1.0)]);
        assert_eq!(r.1, 1);
        for v in &tris(&r)[0] { assert!(near_distance(v) > -EPS); }
    }
}

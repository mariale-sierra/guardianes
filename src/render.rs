use crate::framebuffer::Framebuffer;
use crate::triangle::{draw_triangle, ScreenVertex, Stats};
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;

const NEAR_EPS: f32 = 0.05;

/// Dibuja UN objeto. No limpia el framebuffer: todos los objetos del cuadro
/// comparten colores y profundidades.
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

    // Etapa de vértices
    let transformed: Vec<Option<ScreenVertex>> = vertices
        .iter()
        .map(|v| {
            let clip = mvp.mul_vec4(v.position, 1.0);
            if clip[3] < NEAR_EPS {
                return None; // detrás / plano cercano (recorte real: ver lección de optimización)
            }
            let inv_w = 1.0 / clip[3];
            let (nx, ny, nz) = (clip[0] * inv_w, clip[1] * inv_w, clip[2] * inv_w);
            Some(ScreenVertex {
                x: (nx + 1.0) * 0.5 * w,
                y: (1.0 - ny) * 0.5 * h,
                z: nz,
                inv_w,
                world: uniforms.model_matrix.transform_point(v.position),
                normal: uniforms.model_matrix.transform_dir(v.normal).normalized(),
                local: v.position,
                uv: v.uv,
            })
        })
        .collect();

    // Ensamblado y rasterización
    for tri in transformed.chunks_exact(3) {
        stats.triangles_in += 1;
        if let (Some(a), Some(b), Some(c)) = (&tri[0], &tri[1], &tri[2]) {
            draw_triangle(fb, a, b, c, uniforms, cull_backfaces, stats);
        }
    }
}

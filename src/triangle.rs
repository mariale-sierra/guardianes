use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::shaders;
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;

/// Vértice ya transformado: posición en pantalla + atributos a interpolar.
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,     // profundidad NDC [-1, 1]
    pub inv_w: f32, // 1 / w_clip, para interpolar con corrección de perspectiva
    pub world: Vector3,
    pub normal: Vector3,
    pub local: Vector3,
    pub uv: (f32, f32),
}

/// Contadores simples para comparar optimizaciones (descarte, LOD...).
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub triangles_in: u64,
    pub triangles_backface: u64,
    pub triangles_rasterized: u64,
    pub fragments_shaded: u64,
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

pub fn draw_triangle(
    fb: &mut Framebuffer,
    v0: &ScreenVertex,
    v1: &ScreenVertex,
    v2: &ScreenVertex,
    uniforms: &Uniforms,
    cull_backfaces: bool,
    stats: &mut Stats,
) {
    let area = edge(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
    if area.abs() < 1e-8 {
        return;
    }
    // Triángulos CCW (en NDC, y arriba) quedan con área NEGATIVA en pantalla (y abajo).
    if cull_backfaces && area > 0.0 {
        stats.triangles_backface += 1;
        return;
    }

    // Rectángulo delimitador recortado al framebuffer ANTES de recorrerlo.
    let min_x = v0.x.min(v1.x).min(v2.x).floor().max(0.0) as i32;
    let max_x = v0.x.max(v1.x).max(v2.x).ceil().min(fb.width as f32 - 1.0) as i32;
    let min_y = v0.y.min(v1.y).min(v2.y).floor().max(0.0) as i32;
    let max_y = v0.y.max(v1.y).max(v2.y).ceil().min(fb.height as f32 - 1.0) as i32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    stats.triangles_rasterized += 1;

    let inv_area = 1.0 / area;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(v1.x, v1.y, v2.x, v2.y, px, py) * inv_area;
            let w1 = edge(v2.x, v2.y, v0.x, v0.y, px, py) * inv_area;
            let w2 = edge(v0.x, v0.y, v1.x, v1.y, px, py) * inv_area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let depth = w0 * v0.z + w1 * v1.z + w2 * v2.z;
            if !(-1.0..=1.0).contains(&depth) {
                continue;
            }
            let index = y as usize * fb.width + x as usize;
            // Prueba de profundidad ANTES de interpolar atributos y sombrear.
            if depth >= fb.depths[index] {
                continue;
            }
            fb.depths[index] = depth;

            // Interpolación con corrección de perspectiva.
            let b0 = w0 * v0.inv_w;
            let b1 = w1 * v1.inv_w;
            let b2 = w2 * v2.inv_w;
            let k = 1.0 / (b0 + b1 + b2);
            let (r0, r1, r2) = (b0 * k, b1 * k, b2 * k);

            let fragment = Fragment {
                depth,
                world_position: v0.world * r0 + v1.world * r1 + v2.world * r2,
                normal: (v0.normal * r0 + v1.normal * r1 + v2.normal * r2).normalized(),
                object_position: v0.local * r0 + v1.local * r1 + v2.local * r2,
                uv: (
                    v0.uv.0 * r0 + v1.uv.0 * r1 + v2.uv.0 * r2,
                    v0.uv.1 * r0 + v1.uv.1 * r1 + v2.uv.1 * r2,
                ),
            };
            fb.colors[index] = shaders::fragment_shader(&fragment, uniforms);
            stats.fragments_shaded += 1;
        }
    }
}

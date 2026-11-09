// planet_shaders.rs
// Shaders de fragmentos propios, sin dependencias de ruido externas.
//
// Convención de entradas (todas las funciones):
//   p  = posición LOCAL del fragmento (fragment.object_position), esfera unitaria
//   n  = normal en mundo, normalizada
//   l  = dirección de la superficie hacia el sol, normalizada
//   t  = uniforms.time
//
// Uso desde shaders::fragment_shader():
//   if let Some(c) = planet_shaders::shade(uniforms.shader_index, p, n, l, uniforms.time) { return c; }

use raylib::prelude::Vector3;
use std::f32::consts::PI;

pub const SHADER_SUN: usize = 20;
pub const SHADER_EARTH: usize = 21;
pub const SHADER_XANDAR: usize = 22;
pub const SHADER_KNOWHERE: usize = 23;
pub const SHADER_EGO: usize = 24;
pub const SHADER_SOVEREIGN: usize = 25;
pub const SHADER_ORGO_CORE: usize = 26;
pub const SHADER_ORGO_RING: usize = 27;

// ---------------------------------------------------------------- utilidades

fn c(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}
fn mix(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    a + (b - a) * t
}
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Ruido de valor 3D en [0, 1].
pub fn noise3(p: Vector3) -> f32 {
    let (ix, iy, iz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (fx, fy, fz) = (p.x - ix, p.y - iy, p.z - iz);
    let (sx, sy, sz) = (
        fx * fx * (3.0 - 2.0 * fx),
        fy * fy * (3.0 - 2.0 * fy),
        fz * fz * (3.0 - 2.0 * fz),
    );
    let (ix, iy, iz) = (ix as i32, iy as i32, iz as i32);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let h = |dx, dy, dz| hash3(ix + dx, iy + dy, iz + dz);
    let x00 = l(h(0, 0, 0), h(1, 0, 0), sx);
    let x10 = l(h(0, 1, 0), h(1, 1, 0), sx);
    let x01 = l(h(0, 0, 1), h(1, 0, 1), sx);
    let x11 = l(h(0, 1, 1), h(1, 1, 1), sx);
    l(l(x00, x10, sy), l(x01, x11, sy), sz)
}

/// Ruido fractal en [0, 1].
pub fn fbm(mut p: Vector3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut norm) = (0.0, 0.5, 0.0);
    for _ in 0..octaves {
        sum += amp * noise3(p);
        norm += amp;
        p = p * 2.03;
        amp *= 0.5;
    }
    sum / norm
}

fn lit(color: Vector3, n: Vector3, l: Vector3, ambient: f32) -> Vector3 {
    let d = n.dot(l).max(0.0);
    color * (ambient + (1.0 - ambient) * d)
}

// ---------------------------------------------------------------- dispatcher

pub fn shade(id: usize, p: Vector3, n: Vector3, l: Vector3, t: f32) -> Option<Vector3> {
    Some(match id {
        SHADER_SUN => sun(p, t),
        SHADER_EARTH => earth(p, n, l, t),
        SHADER_XANDAR => xandar(p, n, l, t),
        SHADER_KNOWHERE => knowhere(p, n, l, t),
        SHADER_EGO => ego(p, n, l, t),
        SHADER_SOVEREIGN => sovereign(p, n, l, t),
        SHADER_ORGO_CORE => orgo_core(p, n, l, t),
        SHADER_ORGO_RING => orgo_ring(p, n, l, t),
        _ => return None,
    })
}

// ---------------------------------------------------------------- SOL (animado)

fn sun(p: Vector3, t: f32) -> Vector3 {
    let warp = fbm(p * 2.0 + Vector3::new(t * 0.10, t * 0.05, 0.0), 3);
    let q = p * 3.5 + Vector3::new(warp * 1.5, t * 0.15, warp);
    let n = fbm(q, 5);
    let hot = c(1.0, 0.92, 0.35);
    let mid = c(1.0, 0.55, 0.08);
    let cold = c(0.75, 0.15, 0.02);
    let base = if n > 0.5 {
        mix(mid, hot, smoothstep(0.5, 0.75, n))
    } else {
        mix(cold, mid, smoothstep(0.25, 0.5, n))
    };
    base * 1.15 // emisivo: sin sombreado
}

// ---------------------------------------------------------------- TIERRA

fn earth(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let h = fbm(p * 2.2 + Vector3::new(3.0, 1.0, 5.0), 5);
    let land = smoothstep(0.50, 0.53, h);
    let lat = p.y.abs();

    let deep = c(0.02, 0.08, 0.30);
    let shallow = c(0.05, 0.30, 0.55);
    let ocean = mix(deep, shallow, smoothstep(0.35, 0.5, h));

    let green = c(0.12, 0.38, 0.12);
    let desert = c(0.62, 0.50, 0.28);
    let dry = smoothstep(0.50, 0.62, fbm(p * 4.0, 3)) * (1.0 - lat);
    let ground = mix(green, desert, dry);
    let mut color = mix(ocean, ground, land);

    // casquetes polares
    color = mix(color, c(0.92, 0.95, 1.0), smoothstep(0.80, 0.90, lat + (h - 0.5) * 0.2));

    // nubes, derivan lento con el tiempo
    let cl = fbm(p * 4.5 + Vector3::new(t * 0.03, 0.0, t * 0.01), 5);
    let clouds = smoothstep(0.55, 0.75, cl);
    color = mix(color, c(1.0, 1.0, 1.0), clouds * 0.85);

    let day = n.dot(l);
    let mut out = lit(color, n, l, 0.10);

    // luces de ciudades en el lado nocturno
    let night = 1.0 - smoothstep(-0.05, 0.12, day);
    let spots = smoothstep(0.72, 0.78, noise3(p * 40.0)) * land * (1.0 - clouds * 0.7);
    out = out + c(1.0, 0.75, 0.3) * (spots * night);
    out
}

// ---------------------------------------------------------------- XANDAR

/// Ciudad radial (rosa de los vientos de 8 puntas + anillos) alrededor de `center`.
/// Devuelve (máscara, color).
fn city(p: Vector3, center: Vector3, size: f32) -> (f32, Vector3) {
    if p.dot(center) <= 0.0 {
        return (0.0, c(0.0, 0.0, 0.0));
    }
    let a = if center.y.abs() < 0.9 { c(0.0, 1.0, 0.0) } else { c(1.0, 0.0, 0.0) };
    let u = a.cross(center).normalized();
    let v = center.cross(u);
    let (x, y) = (p.dot(u), p.dot(v));
    let r = (x * x + y * y).sqrt() / size;
    let theta = y.atan2(x);

    // borde irregular de la isla
    let edge = r + (fbm(p * 7.0, 3) - 0.5) * 0.25;
    let mask = 1.0 - smoothstep(0.80, 0.95, edge);
    if mask <= 0.0 {
        return (0.0, c(0.0, 0.0, 0.0));
    }

    let base = mix(
        c(0.22, 0.38, 0.20),
        c(0.80, 0.76, 0.68),
        smoothstep(0.35, 0.65, fbm(p * 9.0, 3)),
    );
    // anillos concéntricos
    let ring = 1.0 - smoothstep(0.0, 0.2, (r * PI * 5.0).sin().abs());
    // 8 avenidas radiales
    let spoke = (1.0 - smoothstep(0.0, 0.16, (theta * 4.0).sin().abs())) * smoothstep(1.05, 0.05, r);

    let mut col = mix(base, c(0.30, 0.34, 0.38), ring * 0.55);
    col = mix(col, c(0.95, 0.95, 0.98), spoke);
    col = mix(col, c(1.0, 1.0, 1.0), 1.0 - smoothstep(0.08, 0.14, r)); // centro
    (mask, col)
}

fn xandar(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let h = fbm(p * 3.0, 4);
    let deep = c(0.02, 0.28, 0.42);
    let shallow = c(0.18, 0.70, 0.72);
    let mut color = mix(deep, shallow, smoothstep(0.35, 0.6, h));
    // olas / brillo sutil animado
    color = color + c(0.04, 0.06, 0.06) * noise3(p * 30.0 + Vector3::new(t * 0.4, 0.0, 0.0));

    let c1 = city(p, c(0.0, 0.15, 1.0).normalized(), 0.78);
    let c2 = city(p, c(-0.8, -0.4, -0.45).normalized(), 0.32);
    color = mix(color, c1.1, c1.0);
    color = mix(color, c2.1, c2.0);

    // nubes
    let cl = smoothstep(0.62, 0.8, fbm(p * 5.0 + Vector3::new(t * 0.02, 0.0, 0.0), 4));
    color = mix(color, c(1.0, 1.0, 1.0), cl * 0.6);
    lit(color, n, l, 0.12)
}

// ---------------------------------------------------------------- KNOWHERE

fn knowhere(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let r = fbm(p * 3.0, 5);
    let mut color = mix(c(0.06, 0.07, 0.05), c(0.28, 0.24, 0.12), smoothstep(0.3, 0.7, r));
    // vetas de cristal turquesa
    let vein = 1.0 - smoothstep(0.0, 0.022, (fbm(p * 4.0, 4) - 0.52).abs());
    color = mix(color, c(0.1, 0.9, 0.7), vein * 0.7);
    let mut out = lit(color, n, l, 0.10);
    // luces naranjas de las minas (parpadean)
    let s = noise3(p * 55.0);
    let flick = 0.6 + 0.4 * (t * 3.0 + s * 20.0).sin();
    out = out + c(1.0, 0.5, 0.1) * (smoothstep(0.86, 0.89, s) * flick);
    // cuenca ocular: brillo naranja
    let eye = Vector3::new(0.55, 0.15, 0.82).normalized();
    out + c(1.0, 0.45, 0.1) * (1.0 - smoothstep(0.0, 0.30, (p.normalized() - eye).length())) * 0.7
}

// ---------------------------------------------------------------- EGO

fn ego(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let r = fbm(p * 4.0, 5);
    let mut color = mix(c(0.32, 0.05, 0.04), c(0.62, 0.14, 0.08), r);
    // grietas
    let crack = 1.0 - smoothstep(0.0, 0.03, (fbm(p * 7.0, 4) - 0.5).abs());
    color = mix(color, c(0.08, 0.01, 0.01), crack * 0.8);

    // cara en el hemisferio +Z
    let facing = smoothstep(0.25, 0.55, p.z);
    for s in [-1.0f32, 1.0] {
        let d = ((p.x - s * 0.32).powi(2) + (p.y - 0.22).powi(2)).sqrt();
        let glow = 1.0 - smoothstep(0.05, 0.12, d);
        let pulse = 0.8 + 0.2 * (t * 1.5).sin();
        color = mix(color, c(0.2, 1.0, 0.9) * pulse, glow * facing);
        // ceja
        let brow = 1.0 - smoothstep(0.0, 0.04, (p.y - 0.33 - 0.05 * (p.x - s * 0.32).abs()).abs());
        let in_x = 1.0 - smoothstep(0.12, 0.18, (p.x - s * 0.32).abs());
        color = mix(color, c(0.12, 0.01, 0.01), brow * in_x * facing * 0.9);
    }
    // nariz (cresta clara)
    let nose = (1.0 - smoothstep(0.03, 0.08, p.x.abs())) * smoothstep(-0.2, 0.0, p.y) * (1.0 - smoothstep(0.1, 0.2, p.y));
    color = mix(color, c(0.75, 0.25, 0.15), nose * facing * 0.7);
    // boca
    let my = -0.32 + 0.12 * p.x * p.x;
    let mouth = (1.0 - smoothstep(0.015, 0.045, (p.y - my).abs())) * (1.0 - smoothstep(0.24, 0.32, p.x.abs()));
    color = mix(color, c(0.02, 0.0, 0.0), mouth * facing);

    lit(color, n, l, 0.12)
}

// ---------------------------------------------------------------- SOVEREIGN (animado)

fn sovereign(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let gold = mix(c(0.55, 0.38, 0.08), c(1.0, 0.82, 0.30), fbm(p * 5.0, 3));
    // paneles: rejilla en coordenadas esféricas
    let lon = p.z.atan2(p.x);
    let lat = p.y.clamp(-1.0, 1.0).asin();
    let g = ((lon * 10.0).sin().abs().min((lat * 14.0).sin().abs())).min(1.0);
    let seam = 1.0 - smoothstep(0.0, 0.08, g);
    let mut color = mix(gold, c(0.25, 0.16, 0.03), seam * 0.8);
    let mut out = lit(color, n, l, 0.12);
    // luces de ciudad que parpadean
    let s = noise3(p * 45.0);
    let pulse = 0.5 + 0.5 * (t * 2.0 + s * 30.0).sin();
    color = c(1.0, 0.85, 0.4);
    out = out + color * (smoothstep(0.78, 0.82, s) * pulse * 0.9);
    out
}

// ---------------------------------------------------------------- ORGOSCOPE

fn flesh(p: Vector3) -> Vector3 {
    let w = fbm(p * 3.0, 3);
    let n = fbm(p * 6.0 + Vector3::new(w * 2.0, 0.0, w), 5);
    let mut col = mix(c(0.35, 0.10, 0.10), c(0.80, 0.45, 0.40), n);
    let vein = 1.0 - smoothstep(0.0, 0.04, (fbm(p * 9.0, 3) - 0.5).abs());
    col = mix(col, c(0.55, 0.05, 0.10), vein * 0.6);
    col
}

fn orgo_core(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let out = lit(flesh(p), n, l, 0.15);
    // "ojo" luminoso en +Z
    let d = ((p.x).powi(2) + (p.y).powi(2)).sqrt();
    let eye = (1.0 - smoothstep(0.10, 0.30, d)) * smoothstep(0.3, 0.6, p.z);
    let pulse = 1.0 + 0.15 * (t * 2.0).sin();
    let ring = 1.0 - smoothstep(0.0, 0.03, (d - 0.32).abs());
    mix(out, c(0.75, 0.9, 1.0) * 1.6 * pulse, eye) + c(0.5, 0.6, 0.7) * ring * smoothstep(0.3, 0.6, p.z)
}

fn orgo_ring(p: Vector3, n: Vector3, l: Vector3, t: f32) -> Vector3 {
    let out = lit(flesh(p * 1.5), n, l, 0.18);
    let s = noise3(p * 20.0 + Vector3::new(t * 0.2, 0.0, 0.0));
    out + c(1.0, 0.55, 0.15) * smoothstep(0.78, 0.84, s)
}

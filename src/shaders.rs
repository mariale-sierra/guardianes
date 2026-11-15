use crate::fragment::Fragment;
use crate::planet_shaders::{self, SHADER_KNOWHERE, SHADER_ORGO_CORE, SHADER_ORGO_RING, SHADER_SOVEREIGN};
use crate::texture;
use crate::ship::{self, SHADER_SHIP};
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;

/// Cuantas veces se repite la textura de Sovereign por unidad de modelo (mayor = vetas más finas).
pub const SOVEREIGN_TEXTURE_REPEAT: f32 = 1.5;

pub fn fragment_shader(f: &Fragment, u: &Uniforms) -> Vector3 {
    let n = f.normal;
    let l = (u.light_position - f.world_position).normalized();
    // La nave se pinta con su textura (UV), no con la posicion local.
    if u.shader_index == SHADER_SHIP {
        return ship::shade(f, u, l);
    }
    // Planetas de Blender con textura propia (assets/textures/<nombre>.png): se pintan por la UV del modelo
    // (v invertida: en el OBJ v=1 es arriba) con luz difusa. Sin textura se usa el shader procedural de abajo.
    let blend_texture = match u.shader_index {
        SHADER_KNOWHERE => texture::get("knowhere"),
        SHADER_SOVEREIGN => texture::get("sovereign"),
        SHADER_ORGO_CORE => texture::get("orgoscope"),
        _ => None,
    };
    if let Some(tx) = blend_texture {
        // Sovereign: su UV son cientos de islas diminutas y la textura es oscura, asi que muchas caras caian en
        // zonas negras. Se usa mapeo triplanar (cubre todo parejo) y se aclara un poco para que el dorado se lea.
        let color = if u.shader_index == SHADER_SOVEREIGN {
            let c = tx.sample_triplanar(f.object_position, f.object_position.normalized(), SOVEREIGN_TEXTURE_REPEAT);
            Vector3::new(c.x.powf(0.7), c.y.powf(0.7), c.z.powf(0.7)) * 1.25
        } else {
            tx.sample(f.uv.0, 1.0 - f.uv.1)
        };
        return color * (0.16 + 0.84 * n.dot(l).max(0.0));
    }
    // Los anillos (toro) no son esferas unitarias: usan la posición local tal cual.
    let p = if u.shader_index == SHADER_ORGO_RING {
        f.object_position
    } else {
        f.object_position.normalized()
    };
    planet_shaders::shade(u.shader_index, p, n, l, u.time).unwrap_or_else(|| {
        let d = n.dot(l).max(0.0);
        Vector3::new(0.8, 0.8, 0.8) * (0.1 + 0.9 * d)
    })
}

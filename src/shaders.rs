use crate::fragment::Fragment;
use crate::planet_shaders::{self, SHADER_ORGO_RING};
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;

pub fn fragment_shader(f: &Fragment, u: &Uniforms) -> Vector3 {
    let n = f.normal;
    let l = (u.light_position - f.world_position).normalized();
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

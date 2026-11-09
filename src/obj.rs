// Cargador OBJ mínimo (v, vt, vn, f con triangulación en abanico). Lo usaremos para la nave.
use crate::vertex::Vertex;
use raylib::prelude::Vector3;

pub fn load(path: &str) -> Result<Vec<Vertex>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let (mut pos, mut nor, mut uv): (Vec<Vector3>, Vec<Vector3>, Vec<(f32, f32)>) = (vec![], vec![], vec![]);
    let mut out = Vec::new();

    let f = |s: Option<&str>| s.and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => pos.push(Vector3::new(f(it.next()), f(it.next()), f(it.next()))),
            Some("vn") => nor.push(Vector3::new(f(it.next()), f(it.next()), f(it.next()))),
            Some("vt") => uv.push((f(it.next()), f(it.next()))),
            Some("f") => {
                let idx = |n: &str, len: usize| -> Option<usize> {
                    let i: i64 = n.parse().ok()?;
                    if i > 0 { Some(i as usize - 1) } else { Some((len as i64 + i) as usize) }
                };
                let corners: Vec<Vertex> = it
                    .map(|tok| {
                        let mut p = tok.split('/');
                        let vi = p.next().and_then(|n| idx(n, pos.len())).unwrap_or(0);
                        let ti = p.next().and_then(|n| if n.is_empty() { None } else { idx(n, uv.len()) });
                        let ni = p.next().and_then(|n| idx(n, nor.len()));
                        Vertex::new(
                            pos[vi],
                            ni.map(|i| nor[i]).unwrap_or(Vector3::new(0.0, 1.0, 0.0)),
                            ti.map(|i| uv[i]).unwrap_or((0.0, 0.0)),
                        )
                    })
                    .collect();
                for k in 1..corners.len().saturating_sub(1) {
                    out.extend_from_slice(&[corners[0], corners[k], corners[k + 1]]);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

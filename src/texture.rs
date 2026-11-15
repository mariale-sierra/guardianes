// texture.rs
// Muestreo de texturas PNG en nuestro propio renderer (sin GPU).
//
// Idea: una textura es una rejilla de colores. Un fragmento trae una coordenada UV en [0,1]x[0,1];
// la convertimos a coordenadas de pixel y mezclamos los 4 pixeles vecinos (interpolacion BILINEAL)
// para que no se vean "cuadros" al acercarse.
//
//   u = 0..1 de izquierda a derecha, v = 0..1 de ARRIBA hacia abajo de la imagen
//   (igual que uv_sphere: v = 0 es el polo +Y; y que el OBJ exportado por build_milano.py).
//
// El PNG se lee con la libreria `png` (solo lectura de archivos). Todo el muestreo es nuestro.

use raylib::prelude::Vector3;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::OnceLock;

pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// Colores en [0,1], fila por fila empezando por la fila de ARRIBA de la imagen.
    pixels: Vec<Vector3>,
}

impl Texture {
    /// Carga un PNG (RGB o RGBA de 8 bits). Devuelve Err con un mensaje legible si falla.
    pub fn load(path: &str) -> Result<Texture, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        // Normaliza cualquier formato (paleta, 16 bits...) a 8 bits por canal.
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| format!("{path}: {e}"))?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| format!("{path}: {e}"))?;

        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            other => return Err(format!("{path}: formato de color no soportado: {other:?}")),
        };
        let (w, h) = (info.width as usize, info.height as usize);
        let mut pixels = Vec::with_capacity(w * h);
        for i in 0..w * h {
            let p = &buf[i * channels..(i + 1) * channels];
            let (r, g, b) = if channels >= 3 { (p[0], p[1], p[2]) } else { (p[0], p[0], p[0]) };
            pixels.push(Vector3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
        }
        Ok(Texture { width: w, height: h, pixels })
    }

    fn texel(&self, x: i64, y: i64) -> Vector3 {
        // u se repite (wrap) y v se limita (clamp): en una esfera el borde izquierdo y el derecho se tocan.
        let xi = x.rem_euclid(self.width as i64) as usize;
        let yi = y.clamp(0, self.height as i64 - 1) as usize;
        self.pixels[yi * self.width + xi]
    }

    /// Color bilineal en (u, v). El centro del pixel (i, j) esta en ((i+0.5)/w, (j+0.5)/h).
    pub fn sample(&self, u: f32, v: f32) -> Vector3 {
        let x = u * self.width as f32 - 0.5;
        let y = v * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = self.texel(x0, y0) * (1.0 - fx) + self.texel(x0 + 1, y0) * fx;
        let bottom = self.texel(x0, y0 + 1) * (1.0 - fx) + self.texel(x0 + 1, y0 + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Como `sample`, pero la imagen se REPITE en u y en v (para texturas tileables: sin costuras).
    pub fn sample_tiled(&self, u: f32, v: f32) -> Vector3 {
        let x = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let y = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let w = |i: i64, j: i64| self.texel(i, j.rem_euclid(self.height as i64));
        let top = w(x0, y0) * (1.0 - fx) + w(x0 + 1, y0) * fx;
        let bottom = w(x0, y0 + 1) * (1.0 - fx) + w(x0 + 1, y0 + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Mapeo TRIPLANAR: en vez de usar las UV del modelo, proyecta la textura (tileable) desde los 3 ejes y
    /// mezcla las tres muestras segun hacia donde mira la superficie. Cubre TODO el objeto de forma pareja,
    /// sin depender del desenvuelto. `p` = posicion local; `scale` = cuantas veces se repite por unidad.
    /// `dir` = direccion aproximada de la superficie (para objetos redondeados basta la posicion normalizada).
    pub fn sample_triplanar(&self, p: Vector3, dir: Vector3, scale: f32) -> Vector3 {
        let w = Vector3::new(dir.x.abs().powi(4), dir.y.abs().powi(4), dir.z.abs().powi(4));
        let sum = (w.x + w.y + w.z).max(1e-6);
        let on_x = self.sample_tiled(p.z * scale, p.y * scale); // cara que mira a +-X
        let on_y = self.sample_tiled(p.x * scale, p.z * scale); // cara que mira a +-Y
        let on_z = self.sample_tiled(p.x * scale, p.y * scale); // cara que mira a +-Z
        (on_x * w.x + on_y * w.y + on_z * w.z) * (1.0 / sum)
    }

    /// Muestreo equirectangular para esferas: `dir` es la direccion LOCAL desde el centro del planeta.
    /// Es la misma formula con la que mesh_gen::uv_sphere reparte sus UV:
    ///   u = longitud / 2pi  (longitud = atan2(z, x)),   v = colatitud / pi  (v = 0 en el polo +Y).
    /// Por eso funciona igual con la UV interpolada o calculada desde la posicion.
    pub fn sample_sphere(&self, dir: Vector3) -> Vector3 {
        let d = dir.normalized();
        let u = d.z.atan2(d.x) / (2.0 * PI);
        let v = d.y.clamp(-1.0, 1.0).acos() / PI;
        self.sample(u.rem_euclid(1.0), v)
    }
}

// ---------------------------------------------------------------- registro global

/// Nombres y rutas de todas las texturas del proyecto. Para agregar una nueva, solo se anade una linea.
const TEXTURE_FILES: &[(&str, &str)] = &[
    ("xandar", "assets/textures/xandar.png"),
    ("tierra", "assets/textures/tierra.png"),
    ("ego", "assets/textures/ego_cara.png"), // ego_base.png es la misma roca SIN la cara
    ("milano", "assets/textures/ships/milano.png"),
];

/// Texturas OPCIONALES de los planetas de Blender (se muestrean por la UV del modelo). Si el archivo no existe
/// no se avisa: el planeta usa su shader procedural. Para usarlas basta dejar el PNG en assets/textures/.
const OPTIONAL_TEXTURE_FILES: &[(&str, &[&str])] = &[
    ("knowhere", &["assets/textures/knowhere.png", "assets/textures/knowhere_roca.png"]),
    ("sovereign", &["assets/textures/sovereign.png"]),
    ("orgoscope", &["assets/textures/orgoscope.png"]),
];

static TEXTURES: OnceLock<HashMap<&'static str, Texture>> = OnceLock::new();

/// Cargar UNA vez antes del bucle principal. Si falta un archivo se avisa por consola y el objeto
/// usa su shader procedural de respaldo (el juego no se cae).
pub fn load_all() {
    TEXTURES.get_or_init(|| {
        let mut map = HashMap::new();
        for (name, path) in TEXTURE_FILES {
            match Texture::load(path) {
                Ok(t) => {
                    map.insert(*name, t);
                }
                Err(e) => eprintln!("aviso: no se pudo cargar la textura '{name}': {e}"),
            }
        }
        for (name, paths) in OPTIONAL_TEXTURE_FILES {
            if let Some(t) = paths.iter().find_map(|p| Texture::load(p).ok()) {
                map.insert(*name, t);
            }
        }
        map
    });
}

/// Textura por nombre, o None si no se cargo (o si no se llamo a `load_all`).
pub fn get(name: &str) -> Option<&'static Texture> {
    TEXTURES.get().and_then(|m| m.get(name))
}

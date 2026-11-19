//! Campo de estrellas de fondo. Etapa 1 = generación procedural; etapa 2 = proyección matemática a píxeles
//! (`project`); etapa 3 = dibujo en el buffer de color (`draw`). Todavía no está integrado en `main.rs`.
//!
//! Cada estrella es una DIRECCIÓN sobre la esfera celeste (infinitamente lejana), más color y tamaño.
//! La generación es determinista: misma semilla + misma cantidad => exactamente las mismas estrellas.
//! No depende del tiempo, de la cámara ni de nada externo.
use crate::framebuffer::Framebuffer;
use crate::matrix::Mat4;
use raylib::prelude::Vector3;

// ---------------------------------------------------------------- parámetros visuales (fáciles de cambiar)

/// Cantidad inicial de estrellas.
pub const STAR_COUNT: usize = 5000;
/// Semilla fija del cielo: cambiarla da otro cielo (siempre el mismo para la misma semilla).
pub const STAR_SEED: u64 = 0x6A09_E667_F3BC_C908;
/// Fracción de estrellas azuladas y cálidas; el resto (~80 %) son blancas.
pub const FRACTION_BLUE: f32 = 0.10;
pub const FRACTION_WARM: f32 = 0.10;
/// Fracción de estrellas de 2 píxeles; el resto mide 1 píxel.
pub const FRACTION_SIZE_2: f32 = 0.04;
/// Brillo = BRIGHTNESS_MIN + (1 - BRIGHTNESS_MIN) * r³, con r uniforme en [0,1):
/// el cubo hace que haya muchas estrellas tenues y pocas brillantes.
pub const BRIGHTNESS_MIN: f32 = 0.35;
/// Tintes (RGB, antes de multiplicar por el brillo).
const TINT_WHITE: (f32, f32, f32) = (1.0, 1.0, 1.0);
const TINT_BLUE: (f32, f32, f32) = (0.70, 0.82, 1.0);
const TINT_WARM: (f32, f32, f32) = (1.0, 0.86, 0.66);

/// Una estrella: solo lo necesario para dibujarla más adelante.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Dirección unitaria desde el observador (posición en la esfera celeste).
    pub dir: Vector3,
    /// Color RGB ya multiplicado por el brillo; cada componente en [0, 1].
    pub color: Vector3,
    /// Tamaño en píxeles del framebuffer: 1 o 2.
    pub size: u8,
}

/// Generador pseudoaleatorio SplitMix64: 64 bits de estado, determinista, sin dependencias.
/// Cada llamada avanza el estado y mezcla los bits; es suficiente para decorar un cielo.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Número uniforme en [0, 1) usando los 24 bits altos (todo f32 con 24 bits de mantisa es exacto).
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Genera `count` estrellas con la semilla dada.
///
/// Dirección uniforme sobre la esfera (método de CLAUDE.md §7):
///   y = 2r − 1 (uniforme en [−1, 1]; por eso no se amontonan en los polos),
///   ángulo = 2π·r, anillo = √(1 − y²),  dir = (anillo·cos, y, anillo·sin)  (longitud 1).
/// Por estrella se consumen SIEMPRE 5 números en el mismo orden, así que cambiar solo un parámetro
/// visual (p. ej. FRACTION_BLUE) no desordena las posiciones.
pub fn generate(seed: u64, count: usize) -> Vec<Star> {
    let mut rng = SplitMix64(seed);
    let mut stars = Vec::with_capacity(count);
    for _ in 0..count {
        let y = 2.0 * rng.next_f32() - 1.0;
        let angle = std::f32::consts::TAU * rng.next_f32();
        let ring = (1.0 - y * y).max(0.0).sqrt();
        let dir = Vector3::new(ring * angle.cos(), y, ring * angle.sin());

        let r = rng.next_f32();
        let brightness = BRIGHTNESS_MIN + (1.0 - BRIGHTNESS_MIN) * r * r * r;

        let class = rng.next_f32();
        let (tr, tg, tb) = if class < FRACTION_BLUE {
            TINT_BLUE
        } else if class < FRACTION_BLUE + FRACTION_WARM {
            TINT_WARM
        } else {
            TINT_WHITE
        };
        let color = Vector3::new(tr, tg, tb) * brightness;

        let size = if rng.next_f32() < FRACTION_SIZE_2 { 2 } else { 1 };
        stars.push(Star { dir, color, size });
    }
    stars
}

/// El cielo por defecto del proyecto.
pub fn default_sky() -> Vec<Star> {
    generate(STAR_SEED, STAR_COUNT)
}

/// Proyecta la dirección de una estrella (coordenadas de MUNDO) a un píxel del framebuffer.
/// Devuelve `None` si no es visible: detrás de la cámara, fuera del campo visual, valores no finitos
/// o dimensiones inválidas. Si devuelve `Some((x, y))`, SIEMPRE cumple x < width e y < height.
///
/// Convenciones (las mismas que `render.rs`):
///   * Solo se usa la parte ROTACIONAL de `view` (`transform_dir` ignora la traslación): las estrellas están
///     "en el infinito", así que mover la nave no las desplaza (sin paralaje).
///   * Cámara mirando a −Z; proyección OpenGL, así que `clip.w = −z_cámara`. Delante de la cámara => w > 0.
///   * x_píxel = (ndc_x + 1)/2 · ancho,  y_píxel = (1 − ndc_y)/2 · alto  (Y de pantalla invertida respecto a NDC).
///   * Bordes: el rango visible es el intervalo semiabierto [0, ancho) × [0, alto) en píxeles, o sea
///     ndc ∈ [−1, 1). El borde izquierdo/superior (ndc = −1) entra; el derecho/inferior (ndc = +1) cae justo
///     en el índice `ancho`/`alto`, que no existe, y se descarta.
pub fn project(dir: Vector3, view: &Mat4, proj: &Mat4, width: usize, height: usize) -> Option<(usize, usize)> {
    if width == 0 || height == 0 || !(dir.x.is_finite() && dir.y.is_finite() && dir.z.is_finite()) {
        return None;
    }
    let cam = view.transform_dir(dir); // rotación solamente
    let clip = proj.mul_vec4(cam, 0.0); // w=0: dirección, sin traslación
    let cw = clip[3];
    if !(cw.is_finite() && cw > 1e-6) {
        return None; // detrás de la cámara (o sobre su plano); evita dividir entre ~0
    }
    let (nx, ny) = (clip[0] / cw, clip[1] / cw);
    if !(nx.is_finite() && ny.is_finite()) {
        return None;
    }
    let (px, py) = ((nx + 1.0) * 0.5 * width as f32, (1.0 - ny) * 0.5 * height as f32);
    // Comprobar el rango ANTES de convertir a usize (un negativo o NaN daría un índice basura)
    if !(px >= 0.0 && py >= 0.0 && px < width as f32 && py < height as f32) {
        return None;
    }
    // floor y salvaguarda: en f32, un px apenas menor que `width` podría redondear hasta `width`.
    Some(((px as usize).min(width - 1), (py as usize).min(height - 1)))
}

/// Dibuja el campo de estrellas en el framebuffer. Llamar justo DESPUÉS de `fb.clear()` y ANTES de dibujar
/// los objetos 3D.
///
/// * Solo escribe en `fb.colors`; `fb.depths` NO se toca (sigue en INFINITY tras `clear`), así que cualquier
///   planeta o la nave pasa la prueba de profundidad y tapa a las estrellas sin lógica extra.
/// * El color se SUMA al existente y se limita a 1.0 (el fondo es casi negro; si dos estrellas de 2 píxeles se
///   tocan no se recortan entre sí).
/// * Tamaño 1 = un píxel; tamaño 2 = bloque de 2×2 con esquina superior izquierda en el píxel proyectado.
///   Los píxeles que caen fuera del framebuffer se omiten (una estrella en el borde se recorta).
pub fn draw(fb: &mut Framebuffer, stars: &[Star], view: &Mat4, proj: &Mat4) {
    let (w, h) = (fb.width, fb.height);
    for s in stars {
        let Some((x, y)) = project(s.dir, view, proj, w, h) else { continue };
        let size = s.size.max(1) as usize;
        for py in y..(y + size).min(h) {
            for px in x..(x + size).min(w) {
                let c = &mut fb.colors[py * w + px];
                c.x = (c.x + s.color.x).min(1.0);
                c.y = (c.y + s.color.y).min(1.0);
                c.z = (c.z + s.color.z).min(1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 10_000; // muestra grande para las pruebas estadísticas

    #[test]
    fn same_seed_and_count_give_identical_stars() {
        assert_eq!(generate(42, 500), generate(42, 500));
        assert_eq!(default_sky(), default_sky());
    }

    #[test]
    fn different_seeds_give_different_stars() {
        let (a, b) = (generate(1, 500), generate(2, 500));
        assert_ne!(a, b);
        let same = a.iter().zip(&b).filter(|(x, y)| x.dir == y.dir).count();
        assert_eq!(same, 0, "ninguna dirección debería coincidir");
    }

    #[test]
    fn directions_are_unit_length_and_finite() {
        for s in generate(STAR_SEED, N) {
            assert!(s.dir.x.is_finite() && s.dir.y.is_finite() && s.dir.z.is_finite());
            assert!((s.dir.length() - 1.0).abs() < 1e-4, "longitud {}", s.dir.length());
        }
    }

    #[test]
    fn colors_are_finite_and_in_range() {
        for s in generate(STAR_SEED, N) {
            for c in [s.color.x, s.color.y, s.color.z] {
                assert!(c.is_finite() && (BRIGHTNESS_MIN * 0.6..=1.0).contains(&c), "color fuera de rango: {c}");
            }
        }
    }

    #[test]
    fn sizes_are_valid_and_mostly_one_pixel() {
        let stars = generate(STAR_SEED, N);
        assert!(stars.iter().all(|s| s.size == 1 || s.size == 2));
        let big = stars.iter().filter(|s| s.size == 2).count() as f32 / N as f32;
        assert!((big - FRACTION_SIZE_2).abs() < 0.02, "fracción de tamaño 2: {big}");
    }

    #[test]
    fn color_mix_is_about_80_10_10() {
        let stars = generate(STAR_SEED, N);
        let blue = stars.iter().filter(|s| s.color.z > s.color.x * 1.2).count() as f32 / N as f32;
        let warm = stars.iter().filter(|s| s.color.x > s.color.z * 1.2).count() as f32 / N as f32;
        assert!((blue - FRACTION_BLUE).abs() < 0.03, "azuladas: {blue}");
        assert!((warm - FRACTION_WARM).abs() < 0.03, "cálidas: {warm}");
    }

    #[test]
    fn both_hemispheres_are_covered() {
        let stars = generate(STAR_SEED, STAR_COUNT);
        for axis in [|d: Vector3| d.x, |d: Vector3| d.y, |d: Vector3| d.z] {
            let pos = stars.iter().filter(|s| axis(s.dir) > 0.0).count() as f32 / STAR_COUNT as f32;
            assert!((0.42..=0.58).contains(&pos), "reparto desbalanceado: {pos}");
        }
    }

    #[test]
    fn no_artificial_concentration_at_the_poles() {
        // En una esfera uniforme, |y| > 0.9 ocurre con probabilidad 10 % (y es uniforme en [-1,1]).
        // Un muestreo ingenuo (ángulos uniformes) daría ~29 %.
        let stars = generate(STAR_SEED, N);
        let polar = stars.iter().filter(|s| s.dir.y.abs() > 0.9).count() as f32 / N as f32;
        assert!((polar - 0.10).abs() < 0.02, "fracción polar: {polar}");
    }

    #[test]
    fn zero_count_gives_empty_field() {
        assert!(generate(STAR_SEED, 0).is_empty());
    }

    #[test]
    fn generation_is_independent_of_time_and_prefix_is_stable() {
        // Sin entradas de tiempo/cámara: solo semilla y cantidad. Dos llamadas separadas en el tiempo coinciden,
        // y pedir menos estrellas devuelve un prefijo del mismo cielo (las posiciones no cambian con la cantidad).
        let a = generate(7, 300);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let b = generate(7, 300);
        assert_eq!(a, b);
        assert_eq!(&generate(7, 100)[..], &a[..100]);
    }

    // ------------------------------------------------------------------ proyección (etapa 2)
    use crate::matrix::{create_projection_matrix, create_view_matrix};

    const FW: usize = 480;
    const FH: usize = 320;

    /// Proyección real del juego: fov 60°, aspecto 480/320.
    fn game_proj() -> Mat4 {
        create_projection_matrix(60f32.to_radians(), FW as f32 / FH as f32, 0.1, 200.0)
    }
    /// Vista real mirando hacia `forward` desde `eye` (arriba = +Y).
    fn view_looking(eye: Vector3, forward: Vector3) -> Mat4 {
        create_view_matrix(eye, eye + forward, Vector3::new(0.0, 1.0, 0.0))
    }
    fn view_default() -> Mat4 {
        view_looking(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0))
    }
    /// Dirección a `deg` grados del eje −Z, girada alrededor de Y (positivo = hacia +X, la derecha).
    fn yawed(deg: f32) -> Vector3 {
        let a = deg.to_radians();
        Vector3::new(a.sin(), 0.0, -a.cos())
    }
    /// Dirección a `deg` grados del eje −Z, inclinada hacia arriba (positivo = +Y).
    fn pitched(deg: f32) -> Vector3 {
        let a = deg.to_radians();
        Vector3::new(0.0, a.sin(), -a.cos())
    }
    fn p(dir: Vector3) -> Option<(usize, usize)> {
        project(dir, &view_default(), &game_proj(), FW, FH)
    }

    #[test]
    fn front_direction_is_screen_center() {
        assert_eq!(p(Vector3::new(0.0, 0.0, -1.0)), Some((FW / 2, FH / 2)));
    }

    #[test]
    fn direction_behind_is_not_visible() {
        assert_eq!(p(Vector3::new(0.0, 0.0, 1.0)), None);
        assert_eq!(p(Vector3::new(0.3, 0.2, 1.0).normalized()), None);
        assert_eq!(p(Vector3::new(1.0, 0.0, 0.0)), None); // exactamente de lado: w = 0, se descarta
    }

    #[test]
    fn left_right_up_down_map_to_the_right_side_of_the_screen() {
        let (cx, cy) = (FW / 2, FH / 2);
        let left = p(yawed(-20.0)).unwrap();
        let right = p(yawed(20.0)).unwrap();
        let up = p(pitched(15.0)).unwrap();
        let down = p(pitched(-15.0)).unwrap();
        assert!(left.0 < cx && left.1 == cy, "izquierda: {left:?}");
        assert!(right.0 > cx && right.1 == cy, "derecha: {right:?}");
        // Y de pantalla invertida: arriba en el mundo = y de píxel MENOR
        assert!(up.1 < cy && up.0 == cx, "arriba: {up:?}");
        assert!(down.1 > cy && down.0 == cx, "abajo: {down:?}");
        // simetría respecto al centro
        assert!(((cx - left.0) as i32 - (right.0 - cx) as i32).abs() <= 1); // floor a píxel: ±1
        assert!((cy as i32 - up.1 as i32 - (down.1 as i32 - cy as i32)).abs() <= 1);
    }

    #[test]
    fn outside_horizontal_and_vertical_field_is_not_visible() {
        // fov vertical 60° (semiángulo 30°); horizontal: tan(h/2) = 1.5·tan(30°) => semiángulo ≈ 40.9°
        assert!(p(yawed(38.0)).is_some());
        assert_eq!(p(yawed(43.0)), None);
        assert_eq!(p(yawed(-43.0)), None);
        assert!(p(pitched(28.0)).is_some());
        assert_eq!(p(pitched(32.0)), None);
        assert_eq!(p(pitched(-32.0)), None);
    }

    #[test]
    fn exact_edges_follow_the_half_open_rule() {
        // Proyección mínima con m00 = m11 = 1 y w = −z: la dirección (x, y, −1) cae EXACTAMENTE en ndc = (x, y).
        let proj = Mat4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0], [0.0, 0.0, -1.0, 0.0]]);
        let v = Mat4::identity();
        let at = |x: f32, y: f32| project(Vector3::new(x, y, -1.0), &v, &proj, FW, FH);
        assert_eq!(at(-1.0, 1.0), Some((0, 0)), "esquina superior izquierda entra");
        assert_eq!(at(1.0, 0.0), None, "ndc_x = +1 cae en el índice `ancho`: fuera");
        assert_eq!(at(0.0, -1.0), None, "ndc_y = −1 cae en el índice `alto`: fuera");
        assert_eq!(at(-1.0, 0.0), Some((0, FH / 2)));
        assert_eq!(at(0.0, 1.0), Some((FW / 2, 0)));
        // justo dentro del borde derecho/inferior: último píxel
        assert_eq!(at(0.999, -0.999), Some((FW - 1, FH - 1)));
        assert_eq!(at(1.001, 0.0), None);
        assert_eq!(at(-1.001, 0.0), None);
    }

    #[test]
    fn camera_translation_does_not_move_stars() {
        let dir = Vector3::new(0.2, 0.1, -1.0).normalized();
        let base = project(dir, &view_default(), &game_proj(), FW, FH);
        assert!(base.is_some());
        for eye in [Vector3::new(100.0, -50.0, 30.0), Vector3::new(-7.0, 3.0, 999.0)] {
            let moved = view_looking(eye, Vector3::new(0.0, 0.0, -1.0));
            assert_eq!(project(dir, &moved, &game_proj(), FW, FH), base);
        }
    }

    #[test]
    fn rotating_the_camera_90_degrees_changes_visibility() {
        let proj = game_proj();
        let (east, north_ahead) = (Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, -1.0));
        let looking_east = view_looking(Vector3::zero(), east);
        // Mirando a −Z: la estrella en +X no se ve y la de −Z está al centro.
        assert_eq!(project(east, &view_default(), &proj, FW, FH), None);
        assert_eq!(project(north_ahead, &view_default(), &proj, FW, FH), Some((FW / 2, FH / 2)));
        // Girada 90° hacia +X: ahora la de +X está al centro y la de −Z queda a un lado, fuera del campo.
        assert_eq!(project(east, &looking_east, &proj, FW, FH), Some((FW / 2, FH / 2)));
        assert_eq!(project(north_ahead, &looking_east, &proj, FW, FH), None);
        // Y la que antes estaba a la derecha (+X) con la cámara girada queda al frente; la de −X, detrás.
        assert_eq!(project(Vector3::new(-1.0, 0.0, 0.0), &looking_east, &proj, FW, FH), None);
    }

    #[test]
    fn non_finite_inputs_are_rejected() {
        let (v, pr) = (view_default(), game_proj());
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(project(Vector3::new(bad, 0.0, -1.0), &v, &pr, FW, FH), None);
            assert_eq!(project(Vector3::new(0.0, bad, -1.0), &v, &pr, FW, FH), None);
            assert_eq!(project(Vector3::new(0.0, 0.0, bad), &v, &pr, FW, FH), None);
        }
        let mut nan_view = view_default();
        nan_view.0[0][0] = f32::NAN;
        assert_eq!(project(Vector3::new(0.0, 0.0, -1.0), &nan_view, &pr, FW, FH), None);
        assert_eq!(project(Vector3::zero(), &v, &pr, FW, FH), None); // vector nulo: w = 0
    }

    #[test]
    fn invalid_dimensions_are_rejected() {
        let (v, pr, front) = (view_default(), game_proj(), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(project(front, &v, &pr, 0, FH), None);
        assert_eq!(project(front, &v, &pr, FW, 0), None);
        assert_eq!(project(front, &v, &pr, 0, 0), None);
        // dimensión mínima 1x1: el único píxel es el (0, 0)
        assert_eq!(project(front, &v, &pr, 1, 1), Some((0, 0)));
    }

    #[test]
    fn projected_pixels_are_always_inside_the_framebuffer() {
        let proj = game_proj();
        let sky = generate(STAR_SEED, 4000);
        let mut visible = 0;
        // Cámara girando sobre yaw y pitch, y con tamaños de framebuffer distintos (incluidos muy pequeños)
        for &(w, h) in &[(FW, FH), (1usize, 1usize), (7, 3), (960, 640)] {
            for yaw in (0..360).step_by(15) {
                for pitch in [-80.0f32, -30.0, 0.0, 45.0, 89.0] {
                    let (ya, pa) = ((yaw as f32).to_radians(), pitch.to_radians());
                    let forward = Vector3::new(-ya.sin() * pa.cos(), pa.sin(), -ya.cos() * pa.cos());
                    let up = Vector3::new(ya.sin() * pa.sin(), pa.cos(), ya.cos() * pa.sin());
                    let view = create_view_matrix(Vector3::new(5.0, 2.0, -3.0), Vector3::new(5.0, 2.0, -3.0) + forward, up);
                    for s in &sky {
                        if let Some((x, y)) = project(s.dir, &view, &proj, w, h) {
                            assert!(x < w && y < h, "({x}, {y}) fuera de {w}x{h}");
                            visible += 1;
                        }
                    }
                }
            }
        }
        assert!(visible > 0);
    }

    // ------------------------------------------------------------------ dibujo (etapa 3)
    use crate::mesh_gen::uv_sphere;
    use crate::matrix::create_model_matrix;
    use crate::render::render_object;
    use crate::triangle::Stats;
    use crate::uniforms::Uniforms;

    fn fresh_fb() -> Framebuffer {
        let mut fb = Framebuffer::new(FW, FH);
        fb.clear();
        fb
    }
    /// Una estrella roja pura (fácil de distinguir del fondo) en la dirección dada.
    fn star(dir: Vector3, size: u8) -> Star {
        Star { dir, color: Vector3::new(0.8, 0.0, 0.0), size }
    }
    /// Píxeles distintos del fondo (los que alguna estrella tocó).
    fn lit(fb: &Framebuffer) -> Vec<usize> {
        (0..fb.colors.len()).filter(|&i| fb.colors[i] != fb.background).collect()
    }
    fn draw_sky(stars: &[Star], view: &Mat4) -> Framebuffer {
        let mut fb = fresh_fb();
        draw(&mut fb, stars, view, &game_proj());
        fb
    }

    #[test]
    fn visible_star_lights_the_expected_pixel_and_others_do_nothing() {
        let (view, front) = (view_default(), Vector3::new(0.0, 0.0, -1.0));
        let fb = draw_sky(&[star(front, 1)], &view);
        assert_eq!(lit(&fb), vec![FH / 2 * FW + FW / 2]);
        let px = fb.colors[FH / 2 * FW + FW / 2];
        assert!((px.x - (fb.background.x + 0.8)).abs() < 1e-6, "suma sobre el fondo");
        // fuera del campo visual (ángulo grande), detrás de la cámara y con valores no finitos: no se dibuja nada
        let others = [star(yawed(60.0), 1), star(pitched(50.0), 2), star(Vector3::new(0.0, 0.0, 1.0), 2), star(Vector3::new(f32::NAN, 0.0, -1.0), 1)];
        assert!(lit(&draw_sky(&others, &view)).is_empty());
    }

    #[test]
    fn size_one_and_two_cover_the_expected_pixels_and_edges_are_clipped() {
        let (view, front) = (view_default(), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(lit(&draw_sky(&[star(front, 1)], &view)).len(), 1);
        let two = lit(&draw_sky(&[star(front, 2)], &view));
        let (x0, y0) = (FW / 2, FH / 2);
        assert_eq!(two, vec![y0 * FW + x0, y0 * FW + x0 + 1, (y0 + 1) * FW + x0, (y0 + 1) * FW + x0 + 1]);
        // Estrella de 2 píxeles en la esquina inferior derecha: solo cabe 1 píxel y no debe haber pánico ni escrituras fuera
        let proj = Mat4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0], [0.0, 0.0, -1.0, 0.0]]);
        let mut fb = fresh_fb();
        draw(&mut fb, &[star(Vector3::new(0.999, -0.999, -1.0), 2)], &Mat4::identity(), &proj);
        assert_eq!(lit(&fb), vec![FH * FW - 1]);
        assert_eq!(fb.colors.len(), FW * FH);
    }

    #[test]
    fn drawing_never_touches_the_depth_buffer_and_is_deterministic() {
        let sky = generate(STAR_SEED, STAR_COUNT);
        let view = view_default();
        let mut fb = fresh_fb();
        // profundidades de ejemplo distintas de INFINITY en algunos píxeles: deben quedar idénticas
        for i in (0..fb.depths.len()).step_by(97) {
            fb.depths[i] = i as f32 * 0.01;
        }
        let before = fb.depths.clone();
        draw(&mut fb, &sky, &view, &game_proj());
        assert_eq!(fb.depths, before);
        // mismo cielo + misma cámara => misma imagen; con 1500 estrellas hay algo visible
        let (a, b) = (draw_sky(&sky, &view), draw_sky(&sky, &view));
        assert_eq!(a.colors, b.colors);
        assert!(!lit(&a).is_empty());
        // trasladar la cámara no cambia la imagen
        let moved = view_looking(Vector3::new(300.0, -40.0, 12.0), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(draw_sky(&sky, &moved).colors, a.colors);
        // rotar la cámara 90° cambia qué estrellas se ven
        let rotated = view_looking(Vector3::zero(), Vector3::new(1.0, 0.0, 0.0));
        let r = draw_sky(&sky, &rotated);
        assert_ne!(r.colors, a.colors);
        assert!(!lit(&r).is_empty());
    }

    #[test]
    fn a_3d_object_rendered_afterwards_covers_the_stars() {
        // Esfera unitaria a 4 unidades al frente, con el renderer existente. Referencia: la misma esfera sin estrellas.
        let sphere = uv_sphere(24, 16);
        let proj = game_proj();
        let uniforms = Uniforms {
            model_matrix: create_model_matrix(Vector3::new(0.0, 0.0, -4.0), 1.0, Vector3::zero()),
            view_matrix: view_default(),
            projection_matrix: proj,
            camera_position: Vector3::zero(),
            light_position: Vector3::new(0.0, 5.0, 0.0),
            time: 0.0,
            delta_time: 0.0,
            shader_index: 0,
        };
        let mut reference = fresh_fb();
        render_object(&mut reference, &sphere, &uniforms, true, &mut Stats::default());
        let covered: Vec<usize> = (0..reference.depths.len()).filter(|&i| reference.depths[i].is_finite()).collect();
        assert!(covered.len() > 500, "la esfera debe cubrir una zona apreciable");

        // Mucho cielo + una estrella justo en el centro (dentro de la esfera), brillante
        let mut sky = generate(STAR_SEED, 20_000);
        sky.push(Star { dir: Vector3::new(0.0, 0.0, -1.0), color: Vector3::new(1.0, 1.0, 1.0), size: 2 });
        let mut fb = fresh_fb();
        draw(&mut fb, &sky, &view_default(), &proj);
        let mut stats = Stats::default();
        render_object(&mut fb, &sphere, &uniforms, true, &mut stats);

        // Donde la esfera pasó la prueba de profundidad, el color es EXACTAMENTE el de la referencia: ninguna estrella sobrevive.
        // Las profundidades son idénticas a las de la referencia: las estrellas no influyeron en la prueba.
        assert_eq!(fb.depths, reference.depths);
        for &i in &covered {
            assert_eq!(fb.colors[i], reference.colors[i], "una estrella se ve sobre la esfera en el píxel {i}");
        }
        // Fuera de la esfera las estrellas siguen ahí
        let outside_lit = lit(&fb).into_iter().filter(|i| !covered.contains(i)).count();
        assert!(outside_lit > 0);
    }
}

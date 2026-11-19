//! HUD: tres paneles con fondo semitransparente (la escena del centro queda libre).
//!
//!   arriba-izquierda  : ESTADO     (camara, tiempo, Knowhere, pausa)
//!   abajo-izquierda   : CONTROLES  (segun la camara activa, agrupados)
//!   abajo-derecha     : RENDER     (FPS, tiempos y contadores de clipping / backface / frustum culling)
//!
//! Se dibuja con `draw_text` de raylib (solo texto 2D de interfaz; no es dibujo 3D).
//! Cada panel se ajusta al ancho de su linea mas larga, medida con `measure_text`.
use crate::triangle::Stats;
use raylib::prelude::*;

/// Tamano de fuente y espaciado (parametros ajustables del HUD).
pub const FONT_SIZE: i32 = 13;
pub const LINE_HEIGHT: i32 = 18;
/// Margen interno de cada panel y distancia de los paneles al borde de la ventana.
pub const PADDING: i32 = 7;
pub const MARGIN: i32 = 8;
/// Fondo negro con ~45 % de opacidad (115/255).
pub const PANEL_ALPHA: u8 = 115;
/// Ancho maximo de un panel (los dos de abajo deben caber juntos en 960 px con su separacion).
pub const PANEL_MAX_WIDTH: i32 = 440;

/// Datos que el HUD necesita; `main.rs` los rellena cada cuadro.
pub struct HudState {
    pub ship_camera: bool,
    pub time_scale: f32,
    pub paused: bool,
    pub knowhere_deform: bool,
    pub deform_amplitude: f32,
}

/// Metricas de tiempo (se refrescan cada 0.5 s en main).
#[derive(Default, Clone, Copy)]
pub struct Metrics {
    pub fps: f32,
    pub ms_frame: f32,
    pub ms_render: f32,
}

pub fn status_lines(s: &HudState) -> Vec<String> {
    let cam = if s.ship_camera { "Nave" } else { "Orbital" };
    let kn = if s.knowhere_deform { format!("ON {:.2}", s.deform_amplitude) } else { "OFF".to_string() };
    let pause = if s.paused { "PAUSA" } else { "Pausa: no" };
    vec![format!("Camara: {cam} | Tiempo x{} | Knowhere: {kn} | {pause}", s.time_scale)]
}

pub fn controls_lines(ship_camera: bool) -> Vec<&'static str> {
    if ship_camera {
        vec![
            "MOVER    Flechas: girar | W/S: avanzar/retroceder",
            "IR A     1-6: planeta | 0: inicio | TAB: cam. orbital",
            "TIEMPO   , / . : velocidad | P: pausa",
            "KNOWHERE K: deformar on/off | [ ]: intensidad",
        ]
    } else {
        vec![
            "CAMARA   Flechas: orbitar | W/S o rueda: zoom",
            "IR A     1-6: planeta | 0: sistema | TAB: volver a nave",
            "TIEMPO   , / . : velocidad | P: pausa",
            "KNOWHERE K: deformar on/off | [ ]: intensidad",
        ]
    }
}

pub fn render_lines(m: &Metrics, st: &Stats) -> Vec<String> {
    vec![
        format!("FPS {:.0} | {:.1} ms/frame | render {:.1} ms", m.fps, m.ms_frame, m.ms_render),
        format!("Tris: {} | backface: {}", st.triangles_in, st.triangles_backface),
        format!("Raster: {} | clipped: {}", st.triangles_rasterized, st.triangles_clipped),
        format!("Fragmentos: {}", st.fragments_shaded),
        format!("Objetos: {} total, {} descartados", st.objects_total, st.objects_culled),
        format!("Tris descartados (frustum): {}", st.triangles_culled),
    ]
}

/// Ancho en pixeles de un panel con estas lineas (la linea mas ancha + margen interno).
pub fn panel_width<S: AsRef<str>>(rl: &RaylibHandle, lines: &[S]) -> i32 {
    lines.iter().map(|l| rl.measure_text(l.as_ref(), FONT_SIZE)).max().unwrap_or(0) + 2 * PADDING
}

pub fn panel_height(n_lines: usize) -> i32 {
    n_lines as i32 * LINE_HEIGHT + 2 * PADDING - (LINE_HEIGHT - FONT_SIZE)
}

/// Dibuja un panel con su esquina superior izquierda en (x, y).
fn draw_panel<S: AsRef<str>>(d: &mut RaylibDrawHandle, x: i32, y: i32, lines: &[S], text_color: Color) {
    let (w, h) = (panel_width(d, lines), panel_height(lines.len()));
    d.draw_rectangle(x, y, w, h, Color::new(0, 0, 0, PANEL_ALPHA));
    for (i, l) in lines.iter().enumerate() {
        d.draw_text(l.as_ref(), x + PADDING, y + PADDING + i as i32 * LINE_HEIGHT, FONT_SIZE, text_color);
    }
}

/// Dibuja los tres paneles sobre la ventana de `win_w` x `win_h` pixeles.
pub fn draw(d: &mut RaylibDrawHandle, win_w: i32, win_h: i32, state: &HudState, metrics: &Metrics, stats: &Stats) {
    // Estado: arriba a la izquierda
    let status = status_lines(state);
    draw_panel(d, MARGIN, MARGIN, &status, Color::WHITE);

    // Controles: abajo a la izquierda
    let controls = controls_lines(state.ship_camera);
    let ch = panel_height(controls.len());
    draw_panel(d, MARGIN, win_h - MARGIN - ch, &controls, Color::WHITE);

    // Render: abajo a la derecha (alineado al borde derecho segun su ancho real)
    let render = render_lines(metrics, stats);
    let (rw, rh) = (panel_width(d, &render), panel_height(render.len()));
    draw_panel(d, win_w - MARGIN - rw, win_h - MARGIN - rh, &render, Color::LIGHTGRAY);
}

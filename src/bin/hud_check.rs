//! Comprueba los anchos reales del HUD con `measure_text` de raylib (necesita abrir una ventana un instante).
//! Uso: `cargo run --release --bin hud_check`. Sale con error si algun panel no cabe.
use guardianes::hud::*;
use guardianes::triangle::Stats;

fn main() {
    let (win_w, win_h) = (960, 640);
    let (rl, _thread) = raylib::init().size(win_w, win_h).title("hud_check").build();

    // Caso peor: contadores enormes, pausa activa, Knowhere ON
    let big = Stats {
        triangles_in: 99_999_999,
        triangles_backface: 99_999_999,
        triangles_rasterized: 99_999_999,
        triangles_clipped: 99_999_999,
        fragments_shaded: 9_999_999_999,
        objects_total: 99,
        objects_culled: 99,
        triangles_culled: 99_999_999,
    };
    let m = Metrics { fps: 1000.0, ms_frame: 999.9, ms_render: 999.9 };
    let mut ok = true;
    let rl = &rl;
    let mut check = |name: &str, lines: Vec<String>| {
        let w = panel_width(rl, &lines);
        let h = panel_height(lines.len());
        let widest = lines.iter().map(|l| (rl.measure_text(l, FONT_SIZE), l.clone())).max().unwrap();
        let fits = w <= PANEL_MAX_WIDTH;
        ok &= fits;
        println!("{name:<28} panel {w:>3}x{h:<3} (max {PANEL_MAX_WIDTH}) {}  linea mas ancha {} px: \"{}\"", if fits { "OK " } else { "NO CABE" }, widest.0, widest.1);
        h
    };
    let owned = |v: Vec<&str>| v.into_iter().map(String::from).collect::<Vec<_>>();
    let s_on = HudState { ship_camera: true, time_scale: 0.125, paused: true, knowhere_deform: true, deform_amplitude: 0.30 };
    let s_off = HudState { knowhere_deform: false, ..s_on };
    let s_orb = HudState { ship_camera: false, ..s_on };
    let mut total_h = [0; 3];
    check("Estado (nave, ON, pausa)", status_lines(&s_on));
    check("Estado (orbital)", status_lines(&s_orb));
    check("Estado (Knowhere OFF)", status_lines(&s_off));
    total_h[0] = check("Controles (nave)", owned(controls_lines(true)));
    check("Controles (orbital)", owned(controls_lines(false)));
    total_h[1] = check("Render (valores tipicos)", render_lines(&Metrics { fps: 58.0, ms_frame: 17.1, ms_render: 5.4 }, &Stats { triangles_in: 30794, triangles_backface: 16322, triangles_rasterized: 14184, fragments_shaded: 11631, objects_total: 8, objects_culled: 2, triangles_culled: 9548, ..Default::default() }));
    total_h[2] = check("Render (caso peor)", render_lines(&m, &big));

    // Los dos paneles de abajo deben caber uno al lado del otro sin solaparse (peor caso de ancho)
    let left = panel_width(rl, &owned(controls_lines(true))).max(panel_width(rl, &owned(controls_lines(false))));
    let right = panel_width(rl, &render_lines(&m, &big));
    let gap = win_w - 2 * MARGIN - left - right;
    println!("ancho controles {left} + render {right}, hueco entre paneles {gap} px {}", if gap >= MARGIN { "OK" } else { "SOLAPAN" });
    ok &= gap >= MARGIN;
    // Altura: el panel de abajo mas alto + el de estado no deben llegar al otro
    let tallest = total_h[0].max(total_h[2]);
    let free = win_h - 2 * MARGIN - tallest - panel_height(1);
    println!("alto panel inferior mas alto {tallest}, espacio libre en el centro {free} px (de {win_h}) {}", if free > win_h / 2 { "OK" } else { "DEMASIADO ALTO" });
    ok &= free > win_h / 2;
    if !ok {
        std::process::exit(1);
    }
}

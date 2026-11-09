use guardianes::{
    camera::Camera, framebuffer::Framebuffer, matrix::*, mesh_gen, render::render_object, scene::*,
    triangle::Stats, uniforms::Uniforms,
};
use raylib::prelude::*;
use std::time::Instant;

const W: usize = 480;
const H: usize = 320;
const SCALE: i32 = 2;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(W as i32 * SCALE, H as i32 * SCALE)
        .title("Guardianes de la Galaxia")
        .build();
    rl.set_target_fps(60);

    let meshes = mesh_gen::build_meshes();
    let mut objects = build_scene();
    let sun_position = Vector3::zero();

    let mut camera = Camera::new(24.0, 0.8);
    let mut focus: Option<usize> = None;

    let mut uniforms = Uniforms {
        model_matrix: Mat4::identity(),
        view_matrix: camera.get_view_matrix(),
        projection_matrix: create_projection_matrix(60f32.to_radians(), W as f32 / H as f32, 0.1, 200.0),
        camera_position: camera.eye,
        light_position: sun_position,
        time: 0.0,
        delta_time: 0.0,
        shader_index: 0,
    };

    let mut fb = Framebuffer::new(W, H);
    let img = Image::gen_image_color(W as i32, H as i32, Color::BLACK);
    let mut texture = rl.load_texture_from_image(&thread, &img).expect("textura");

    let mut paused = false;
    let (mut acc_t, mut acc_render, mut frames) = (0.0f32, 0.0f64, 0u32);
    let mut stats = Stats::default();
    let mut info = String::new();

    // Teclas 1..6 -> planeta a observar
    let focus_keys = [
        (KeyboardKey::KEY_ONE, "Xandar", 2.6),
        (KeyboardKey::KEY_TWO, "Tierra", 2.6),
        (KeyboardKey::KEY_THREE, "Knowhere", 3.2),
        (KeyboardKey::KEY_FOUR, "Ego", 3.4),
        (KeyboardKey::KEY_FIVE, "Sovereign", 3.2),
        (KeyboardKey::KEY_SIX, "Orgoscope", 9.0),
    ];

    while !rl.window_should_close() {
        // 1. Entradas y tiempo
        let dt = rl.get_frame_time();
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            paused = !paused;
        }
        uniforms.delta_time = if paused { 0.0 } else { dt };
        uniforms.time += uniforms.delta_time;

        if rl.is_key_down(KeyboardKey::KEY_LEFT) { camera.yaw -= 1.5 * dt; }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) { camera.yaw += 1.5 * dt; }
        if rl.is_key_down(KeyboardKey::KEY_UP) { camera.pitch += 1.2 * dt; }
        if rl.is_key_down(KeyboardKey::KEY_DOWN) { camera.pitch -= 1.2 * dt; }
        if rl.is_key_down(KeyboardKey::KEY_W) { camera.distance -= camera.distance * 0.8 * dt; }
        if rl.is_key_down(KeyboardKey::KEY_S) { camera.distance += camera.distance * 0.8 * dt; }
        camera.distance *= 1.0 - rl.get_mouse_wheel_move() * 0.1;
        camera.distance = camera.distance.clamp(1.5, 60.0);

        if rl.is_key_pressed(KeyboardKey::KEY_ZERO) {
            focus = None;
            camera.distance = 24.0;
        }
        for (key, name, dist) in focus_keys {
            if rl.is_key_pressed(key) {
                focus = objects.iter().position(|o| o.name == name);
                camera.distance = dist;
                if let Some(i) = focus {
                    camera.look_from_sun_side(objects[i].position, sun_position);
                }
            }
        }

        // 2. Órbitas y rotaciones
        update_scene(&mut objects, sun_position, uniforms.time);

        // 3. Vista
        camera.target = focus.map(|i| objects[i].position).unwrap_or(sun_position);
        camera.update_eye();
        uniforms.view_matrix = camera.get_view_matrix();
        uniforms.camera_position = camera.eye;

        // 4. Limpiar UNA vez y dibujar todo
        let t0 = Instant::now();
        fb.clear();
        let mut frame_stats = Stats::default();
        for o in &objects {
            uniforms.model_matrix = create_model_matrix(o.position, o.scale, o.rotation);
            uniforms.shader_index = o.shader_index;
            render_object(&mut fb, &meshes[o.mesh], &uniforms, true, &mut frame_stats);
        }
        acc_render += t0.elapsed().as_secs_f64() * 1000.0;
        stats = frame_stats;

        // 5. Presentar
        texture.update_texture(&fb.bytes()).expect("update");
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(&texture, Vector2::new(0.0, 0.0), 0.0, SCALE as f32, Color::WHITE);
        d.draw_text("Flechas: orbitar | W/S o rueda: zoom | 1-6: planetas | 0: sistema | P: pausa", 10, 8, 14, Color::WHITE);
        d.draw_text(&info, 10, H as i32 * SCALE - 22, 14, Color::LIGHTGRAY);
        drop(d);

        // Métricas cada 0.5 s
        frames += 1;
        acc_t += dt;
        if acc_t >= 0.5 {
            info = format!(
                "FPS {:.0} | {:.1} ms/frame | render {:.1} ms | tris {} (backface {}, raster {}) | frags {}",
                frames as f32 / acc_t,
                acc_t * 1000.0 / frames as f32,
                acc_render / frames as f64,
                stats.triangles_in, stats.triangles_backface, stats.triangles_rasterized, stats.fragments_shaded
            );
            acc_t = 0.0; acc_render = 0.0; frames = 0;
        }
    }
}

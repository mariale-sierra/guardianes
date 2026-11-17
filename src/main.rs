use guardianes::{
    camera::Camera, framebuffer::Framebuffer, matrix::*, mesh_gen, render::render_object, scene::*,
    planet_models,
    ship::{self, Ship, ShipInput, SHADER_SHIP},
    texture,
    triangle::Stats, uniforms::Uniforms,
};
use raylib::prelude::*;
use std::time::Instant;

const W: usize = 480;
const H: usize = 320;
const SCALE: i32 = 2;

/// Distancia del viaje rápido = radio del planeta * este factor (+0.4): la nave queda a esa distancia del centro.
const TRAVEL_DISTANCE_FACTOR: f32 = 2.6;
/// Si la nave se aleja más de esto del planeta al que está anclada, se suelta.
const ANCHOR_RELEASE_DISTANCE: f32 = 7.0;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(W as i32 * SCALE, H as i32 * SCALE)
        .title("Guardianes de la Galaxia")
        .build();
    rl.set_target_fps(60);

    texture::load_all(); // PNG de planetas y naves, una sola vez
    let mut meshes = mesh_gen::build_meshes();
    // Knowhere: subdivisión + deformación por ruido (planet_models::process_knowhere). K la activa/desactiva, [ ] cambian su intensidad.
    let mut knowhere_deform = true;
    let mut deform_amplitude = planet_models::DEFORM_AMPLITUDE;
    let ship_meshes = ship::load_models();
    let mut objects = build_scene();
    let sun_position = Vector3::zero();

    // Nave del jugador: empieza a unas unidades del sistema, mirando hacia el sol (-Z).
    let mut ship = Ship::new(Vector3::new(0.0, 1.0, 9.0));
    // TAB alterna entre la camara de la nave (por defecto) y la camara orbital de inspeccion.
    let mut ship_camera = true;

    // Viaje rápido (teclas 1-6 en modo nave): la nave aparece cerca del planeta y SE MUEVE CON ÉL (sigue su órbita),
    // así se puede mirar el planeta sin perseguirlo. Se suelta con 0 (vuelta al inicio) o al alejarse mucho.
    let mut anchor: Option<usize> = None;
    let mut anchor_prev = Vector3::zero();
    let ship_start = ship.position;
    // Velocidad del simulación (planetas, giros, shaders animados). , = más lento  . = más rápido
    let mut time_scale = 1.0f32;

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
        if rl.is_key_pressed(KeyboardKey::KEY_COMMA) { time_scale = (time_scale * 0.5).max(0.125); }
        if rl.is_key_pressed(KeyboardKey::KEY_PERIOD) { time_scale = (time_scale * 2.0).min(2.0); }
        let ship_dt = if paused { 0.0 } else { dt }; // la nave siempre usa tiempo real
        uniforms.delta_time = if paused { 0.0 } else { dt * time_scale }; // el mundo, con la escala elegida
        uniforms.time += uniforms.delta_time;

        if rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            ship_camera = !ship_camera;
        }

        if ship_camera {
            // Vuelo directo: flechas = giro, W/S = avanzar/retroceder. Todo escala con dt.
            let axis = |plus: KeyboardKey, minus: KeyboardKey| -> f32 {
                (rl.is_key_down(plus) as i32 - rl.is_key_down(minus) as i32) as f32
            };
            let input = ShipInput {
                yaw: axis(KeyboardKey::KEY_LEFT, KeyboardKey::KEY_RIGHT),
                pitch: axis(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
                throttle: axis(KeyboardKey::KEY_W, KeyboardKey::KEY_S),
            };
            ship.update(input, ship_dt); // en pausa dt = 0: la nave tambien se congela

            // Viaje rápido a un planeta (1-6) o regreso al inicio (0)
            for (key, name, _) in focus_keys {
                if rl.is_key_pressed(key) {
                    if let Some(i) = objects.iter().position(|o| o.name == name) {
                        let radius = objects[i].collision_radius.max(0.5);
                        ship.place_near(objects[i].position, sun_position, radius * TRAVEL_DISTANCE_FACTOR + 0.4);
                        anchor = Some(i);
                        anchor_prev = objects[i].position;
                    }
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ZERO) {
                ship.position = ship_start;
                ship.yaw = 0.0;
                ship.pitch = 0.0;
                anchor = None;
            }
        } else {
            // Camara orbital de inspeccion (la de antes)
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
        }

        // Deformación de Knowhere en vivo
        if rl.is_key_pressed(KeyboardKey::KEY_K) {
            knowhere_deform = !knowhere_deform;
        }
        let step = (rl.is_key_pressed(KeyboardKey::KEY_RIGHT_BRACKET) as i32 - rl.is_key_pressed(KeyboardKey::KEY_LEFT_BRACKET) as i32) as f32;
        if step != 0.0 {
            deform_amplitude = (deform_amplitude + step * 0.02).clamp(0.0, 0.30);
            // se recalcula la malla desde la original sin deformar (la deformación no se acumula)
            let plain = meshes[MESH_KNOWHERE_PLAIN].clone();
            meshes[MESH_KNOWHERE_BLEND] = planet_models::process_knowhere(&plain, deform_amplitude);
        }

        // 2. Órbitas y rotaciones
        update_scene(&mut objects, sun_position, uniforms.time);

        // Si la nave viaja "anclada" a un planeta, se desplaza lo mismo que él se movió en este cuadro
        if let Some(i) = anchor {
            ship.position = ship.position + (objects[i].position - anchor_prev);
            anchor_prev = objects[i].position;
            if (ship.position - objects[i].position).length() > ANCHOR_RELEASE_DISTANCE {
                anchor = None; // ya se alejó: vuelo libre
            }
        }

        // Colisión: la nave no atraviesa ningún cuerpo (esfera envolvente de cada objeto)
        for o in &objects {
            if o.collision_radius > 0.0 {
                ship.collide(o.position, o.collision_radius);
            }
        }

        // 3. Vista
        if ship_camera {
            uniforms.view_matrix = ship.view_matrix();
            uniforms.camera_position = ship.follow_camera().0;
        } else {
            camera.target = focus.map(|i| objects[i].position).unwrap_or(sun_position);
            camera.update_eye();
            uniforms.view_matrix = camera.get_view_matrix();
            uniforms.camera_position = camera.eye;
        }

        // 4. Limpiar UNA vez y dibujar todo
        let t0 = Instant::now();
        fb.clear();
        let mut frame_stats = Stats::default();
        for o in &objects {
            uniforms.model_matrix = create_model_matrix(o.position, o.scale, o.rotation);
            uniforms.shader_index = o.shader_index;
            // con K apagada, Knowhere se dibuja con su malla sin deformar
            let mesh = if o.mesh == MESH_KNOWHERE_BLEND && !knowhere_deform { MESH_KNOWHERE_PLAIN } else { o.mesh };
            render_object(&mut fb, &meshes[mesh], &uniforms, true, &mut frame_stats);
        }
        // La nave se dibuja despues de los planetas; comparte color y z-buffer con ellos.
        uniforms.model_matrix = ship.model_matrix();
        uniforms.shader_index = SHADER_SHIP;
        render_object(&mut fb, &ship_meshes[ship.model], &uniforms, true, &mut frame_stats);
        acc_render += t0.elapsed().as_secs_f64() * 1000.0;
        stats = frame_stats;

        // 5. Presentar
        texture.update_texture(&fb.bytes()).expect("update");
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(&texture, Vector2::new(0.0, 0.0), 0.0, SCALE as f32, Color::WHITE);
        let help = if ship_camera {
            "Flechas: girar | W/S: avanzar | 1-6: ir a un planeta | 0: inicio | , . : velocidad del tiempo | TAB: camara orbital | K: deformar Knowhere ([ ] intensidad) | P: pausa"
        } else {
            "Flechas: orbitar | W/S o rueda: zoom | 1-6: planetas | 0: sistema | TAB: volver a la nave | P: pausa"
        };
        d.draw_text(help, 10, 8, 14, Color::WHITE);
        d.draw_text(&format!("Tiempo x{time_scale}   {info}"), 10, H as i32 * SCALE - 22, 14, Color::LIGHTGRAY);
        drop(d);

        // Métricas cada 0.5 s
        frames += 1;
        acc_t += dt;
        if acc_t >= 0.5 {
            info = format!(
                "FPS {:.0} | {:.1} ms/frame | render {:.1} ms | tris {} (backface {}, raster {}, clipped {}) | frags {}",
                frames as f32 / acc_t,
                acc_t * 1000.0 / frames as f32,
                acc_render / frames as f64,
                stats.triangles_in, stats.triangles_backface, stats.triangles_rasterized, stats.triangles_clipped, stats.fragments_shaded
            );
            acc_t = 0.0; acc_render = 0.0; frames = 0;
        }
    }
}

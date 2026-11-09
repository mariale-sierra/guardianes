# Guardianes de la Galaxia — Software Renderer

Sistema solar con tema de Guardianes de la Galaxia, renderizado con un pipeline
de software propio (raylib solo se usa para ventana, entrada y mostrar el framebuffer).

## Requisitos para ejecutar
- Rust reciente (>= 1.77). Actualiza con `rustup update`.
- `cmake` y un compilador de C (raylib-sys compila raylib la primera vez).
  - Linux: `sudo apt install build-essential cmake libclang-dev libgl1-mesa-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev`
  - macOS: `brew install cmake`
  - Windows: Visual Studio Build Tools + CMake

## Ejecutar
```
cargo run --release
```
Siempre usa `--release`: un renderer por software es muy lento sin optimizar.

## Controles (cámara temporal, la nave viene después)
| Tecla | Acción |
|---|---|
| Flechas | Orbitar la cámara |
| W / S o rueda | Acercar / alejar |
| 1-6 | Enfocar Xandar, Tierra, Knowhere, Ego, Sovereign, Orgoscope |
| 0 | Volver a la vista del sistema |
| P | Pausar la simulación |

El título/pie de pantalla muestra FPS, ms/frame, tiempo de render y contadores de triángulos.

## Estructura
| Archivo | Contenido |
|---|---|
| `main.rs` | Bucle principal: entradas, tiempo, órbitas, dibujo, presentación |
| `scene.rs` | Objetos del sistema, jerarquía padre-hijo, órbitas y rotaciones |
| `planet_shaders.rs` | Shaders de fragmentos de cada cuerpo + ruido propio |
| `mesh_gen.rs` | Esfera UV, toro, deformación de Knowhere |
| `render.rs` / `triangle.rs` | Etapa de vértices y rasterizador con z-buffer |
| `matrix.rs`, `camera.rs`, `framebuffer.rs`, `obj.rs`... | Soporte |
| `bin/snapshot.rs` | Renderiza un cuadro sin ventana a PPM (para pruebas) |

## Características del menú implementadas hasta ahora
- 6 planetas con identidad propia + sol (`scene.rs`, `planet_shaders.rs`)
- 8 shaders distintos (`planet_shaders.rs`); animados: sol, nubes de la Tierra, luces de Knowhere/Sovereign, ojo del Orgoscope
- Deformación de geometría por ruido: Knowhere (`mesh_gen::knowhere_shape`)
- Generación procedural de geometría: toros del Orgoscope (`mesh_gen::torus`)

Pendiente: nave, movimiento, estrellas, modo local, optimizaciones.

## Integrantes y contribuciones
(completar)

## Uso de IA
Base del código generada con asistencia de IA (Claude); revisada y explicada por el equipo.

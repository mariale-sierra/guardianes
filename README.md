# Guardianes de la Galaxia — Software Renderer

Proyecto final de **Gráficas por Computadora** (2026) · Lenguaje: **Rust** · Temática: **Guardianes de la Galaxia**

Un pequeño sistema solar explorable con la nave Milano de Star-Lord, dibujado con un **renderizador 3D por software escrito por nosotros**: transformaciones Modelo/Vista/Proyección, recorte, rasterización de triángulos, z-buffer, shaders de fragmentos y muestreo de texturas son código propio. **raylib se usa únicamente para abrir la ventana, leer el teclado y mostrar el framebuffer ya terminado**; no se usa ninguna función 3D, de modelos, cámara, materiales, shaders de GPU ni física de raylib, ni ningún motor de juego.

Los planetas orbitan alrededor del sol y rotan sobre su eje; la nave puede acercarse a cualquiera de ellos, rodearlo y alejarse **sin pantallas de carga ni cambio de escena**. También hay una cámara orbital de inspección.

## Vista previa

> **PENDIENTE:** el repositorio todavía no incluye capturas versionadas del juego. Las texturas y modelos que se ven en pantalla están en [Modelos y texturas](#modelos-y-texturas); el recorrido completo estará en el [video de demostración](#video-de-demostración).

## Requisitos y ejecución

- **Rust ≥ 1.77** con Cargo (`rustup update`).
- **CMake** y un compilador de C: la dependencia `raylib` compila la librería nativa la primera vez.
  - Linux: `sudo apt install build-essential cmake libclang-dev libgl1-mesa-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev`
  - macOS: `brew install cmake`
  - Windows: Visual Studio Build Tools + CMake
- Dependencias de Cargo (`Cargo.toml`): `raylib 5.5` (ventana, entrada, mostrar el framebuffer) y `png 0.17` (solo **leer** los archivos PNG; todo el muestreo de texturas es nuestro).

```bash
cargo run --release
```

Usa siempre `--release`: un renderizador por software es muy lento sin optimizar. Ejecuta desde la raíz del repositorio, porque `assets/` se carga con rutas relativas. El framebuffer interno es de **480×320** y se muestra a **960×640** (×2).

```bash
cargo test --release     # 69 pruebas unitarias
```

Otros binarios:

| Comando | Para qué |
|---|---|
| `cargo run --release --bin snapshot -- <foco 0..6> <tiempo> <salida.ppm> [face] [ship]` | Renderiza **un cuadro sin ventana** y lo guarda en PPM; imprime los contadores del cuadro. Sirve para pruebas visuales reproducibles. |
| `cargo run --release --bin hud_check` | Mide con `measure_text` que cada panel del HUD cabe en su espacio (abre una ventana un instante). |

## Controles

Todos los movimientos dependen del **tiempo transcurrido**, no del número de cuadros.

**Cámara de la nave** (modo por defecto)

| Tecla | Acción |
|---|---|
| Flechas ← → | Girar la nave (yaw) |
| Flechas ↑ ↓ | Cabeceo (pitch, limitado a ±1.3 rad) |
| W / S | Avanzar / retroceder |
| 1–6 | Viaje rápido junto a Xandar, Tierra, Knowhere, Ego, Sovereign u Orgoscope; la nave queda «anclada» y acompaña su órbita hasta alejarse |
| 0 | Volver a la posición inicial |

**Cámara orbital de inspección**

| Tecla | Acción |
|---|---|
| Flechas | Orbitar |
| W / S o rueda del ratón | Acercar / alejar |
| 1–6 | Enfocar un planeta (mismo orden que arriba) |
| 0 | Vista general del sistema |

**En ambos modos**

| Tecla | Acción |
|---|---|
| TAB | Alternar cámara de la nave / cámara orbital |
| `,` y `.` | Velocidad del tiempo ×0.5 / ×2 (de ×0.125 a ×2; la nave siempre usa tiempo real) |
| P | Pausa (congela planetas, shaders animados y nave) |
| K | Activar/desactivar la deformación por ruido de Knowhere |
| `[` y `]` | Disminuir/aumentar la intensidad de la deformación (0 a 0.30, pasos de 0.02) |
| Esc | Cerrar la ventana |

**HUD** (tres paneles semitransparentes): arriba a la izquierda, el estado (cámara, tiempo, Knowhere, pausa); abajo a la izquierda, los controles de la cámara activa; abajo a la derecha, las métricas de render (FPS, ms/frame, tiempo de render, triángulos, backface, rasterizados, *clipped*, fragmentos, objetos totales/descartados y triángulos descartados por frustum culling).

## Sistema planetario

El sol está en el origen. Las órbitas son circulares y se calculan como `fase + velocidad · tiempo`.

| Cuerpo | Radio orbital | Qué lo distingue | Malla |
|---|---|---|---|
| **Sol** | — | Granulación solar animada | Esfera UV procedural |
| **Xandar** | 4.2 | Ciudad radial de 8 avenidas, textura propia | Esfera UV procedural |
| **Tierra** | 6.2 | Continentes y nubes que derivan con el tiempo, luces de ciudades en el lado nocturno | Esfera UV procedural |
| **Knowhere** | 8.6 | Cabeza celestial con estación industrial; relieve por ruido | Blender (cráneo CC0 modificado) + **subdivisión y deformación en nuestro código** |
| **Ego** | 11 | Roca con una cara | Esfera UV procedural |
| **Sovereign** | 13.4 | Esferas doradas fusionadas con paneles y bandas | Modelo de Blender (6 476 triángulos) |
| **The Orgoscope** | 16 | Núcleo con «ojo» y cuatro anillos, ejes y masas | Modelo de Blender (6 840 triángulos) |

## Modelos y texturas

Los modelos son `.obj` triangulados con normales y UV (Forward −Z, Up Y); las texturas son PNG de hasta 1024 px. Se cargan **una sola vez** antes del bucle principal.

| Recurso | Archivo | Triángulos |
|---|---|---|
| Knowhere | [`assets/models/planets/knowhere.obj`](assets/models/planets/knowhere.obj) | 3 659 (14 636 tras subdividir) |
| Sovereign | [`assets/models/planets/sovereign.obj`](assets/models/planets/sovereign.obj) | 6 476 |
| The Orgoscope | [`assets/models/planets/orgoscope.obj`](assets/models/planets/orgoscope.obj) | 6 840 |
| Milano (nave) | [`assets/models/ships/milano.obj`](assets/models/ships/milano.obj) | 102 |

| Textura | Archivo | Tamaño |
|---|---|---|
| Xandar | [`xandar.png`](assets/textures/xandar.png) | 1024×512 |
| Tierra | [`tierra.png`](assets/textures/tierra.png) | 1024×512 |
| Ego | [`ego_cara.png`](assets/textures/ego_cara.png) (y [`ego_base.png`](assets/textures/ego_base.png), la misma roca sin cara) | 1024×512 |
| Knowhere | [`knowhere_roca.png`](assets/textures/knowhere_roca.png) | 1024×1024 |
| Sovereign | [`sovereign.png`](assets/textures/sovereign.png) | 1024×1024 |
| The Orgoscope | [`orgoscope.png`](assets/textures/orgoscope.png) | 1024×1024 |
| Milano | [`ships/milano.png`](assets/textures/ships/milano.png) | 1024×1024 |

Si falta un `.obj` o una textura, el juego avisa por consola y usa un respaldo (esfera/Knowhere procedural y shader procedural). Cada modelo de Blender se regenera con su script; ver [`assets/README.md`](assets/README.md) y los README de [`assets/blender/`](assets/blender/).

## Pipeline del renderizador

Orden de cada cuadro (`src/main.rs`): entradas → tiempo → órbitas y rotaciones → nave y colisión → cámara y uniformes → `clear()` (una sola vez) → **estrellas** → planetas y nave → HUD → presentar la textura.

| Etapa | Qué hace | Dónde |
|---|---|---|
| **Matrices** | `Mat4` propia (fila-mayor, vectores columna). Modelo = T·Ry·Rx·Rz·S, vista tipo *look-at*, proyección en perspectiva (FOV 60°, plano cercano 0.1, lejano 200) | `matrix.rs` |
| **Etapa de vértices** | Cada vértice se multiplica por MVP y queda en *clip space* con sus atributos (posición mundo, normal, posición local, UV) | `render.rs` |
| **Recorte del plano cercano** | Sutherland–Hodgman con un solo plano (`z + w ≥ 0`) en espacio de recorte, **antes** de dividir entre w. Un triángulo que cruza el plano se convierte en 1 o 2 triángulos con atributos interpolados, así al acercarse mucho a un planeta no se pierden triángulos. Cuenta `triangles_clipped` | `render.rs` |
| **Frustum culling** | Seis planos extraídos de P·V (en `f64`) y prueba conservadora de esfera envolvente por objeto; un objeto completamente fuera no se transforma ni se rasteriza. Radio calculado una vez por malla, con margen (`r·1.02 + 0.01`). Cuenta objetos y triángulos descartados | `culling.rs` |
| **Rasterización** | Coordenadas baricéntricas dentro de la caja del triángulo, recortada al framebuffer; descarte de caras traseras (triángulos CCW) | `triangle.rs` |
| **Z-buffer** | Prueba de profundidad por píxel; el shader solo se ejecuta si el fragmento pasa | `triangle.rs`, `framebuffer.rs` |
| **Interpolación con corrección de perspectiva** | Atributos interpolados con `1/w` | `triangle.rs` |
| **Shaders de fragmentos** | Funciones propias que reciben posición, normal, luz y tiempo y devuelven un color; iluminación difusa desde el sol. Ruido de valor 3D y `fbm` propios. `shaders.rs` despacha según el objeto: la nave usa `ship::shade`; Sovereign, Knowhere y el núcleo del Orgoscope se pintan con su textura (Sovereign con mapeo triplanar y corrección gamma); el resto usa `planet_shaders::shade`. Los shaders procedurales de Knowhere, Sovereign y Orgoscope son solo el respaldo si falta la textura | `shaders.rs`, `planet_shaders.rs`, `ship.rs` |
| **Texturas y UV** | PNG leídos con la librería `png`; muestreo **bilineal** propio, también repetido (*tiled*), triplanar (Sovereign) y esférico (planetas). La Milano se pinta con su textura por UV, con brillo pulsante en las zonas emisivas | `texture.rs` |
| **Deformación procedural** | Los vértices de Knowhere (ya subdivididos) se desplazan con ruido `fbm` (amplitud 0.07 por defecto, ajustable en vivo con `[ ]`; K alterna entre la malla original y la procesada) | `planet_models.rs` |
| **Generación de esferas** | `uv_sphere(slices, stacks)` genera desde parámetros geométricos todos los vértices, normales y UV de las esferas del Sol, Tierra, Xandar y Ego (48×32) al iniciar, antes de rasterizar | `mesh_gen.rs` |
| **Subdivisión procedural** | Antes de rasterizar, cada triángulo de Knowhere se parte en 4 con puntos medios (sin grietas); 3 659 → 14 636 triángulos. Es una operación **distinta** de la deformación: por sí sola no cambia significativamente la forma (los puntos medios caen sobre las caras planas); el relieve lo introduce la deformación posterior | `planet_models.rs` |
| **Estrellas de fondo** | 5 000 direcciones sobre la esfera celeste con generador `SplitMix64` y semilla fija (mismo cielo en cada ejecución). Se proyectan con **solo la rotación** de la vista (sin paralaje), se dibujan antes de los objetos y escriben únicamente color, no profundidad: planetas y nave las tapan y las estadísticas no cambian | `stars.rs` |
| **Nave y cámara** | Yaw/pitch con movimiento por `dt`, cámara de seguimiento detrás de la nave, colisión nave-esfera con cada cuerpo, viaje rápido anclado a la órbita del planeta | `ship.rs`, `main.rs` |
| **HUD** | Tres paneles con fondo semitransparente; el ancho de cada panel se mide con `measure_text` | `hud.rs` |

## Características del menú de puntos

Los puntos son los que **pretendemos reclamar**; solo valen si la característica funciona en la demostración y se explica, y la nota máxima es 100. No son una calificación garantizada.

| Característica | Implementación | Evidencia | Pts. a reclamar |
|---|---|---|---|
| Planetas distinguibles que orbitan, rotan y se ven de cerca | 6 planetas con identidad propia (`scene.rs`, shaders y modelos de Blender); se reclaman 5 | Ejecutar el juego; teclas 1–6 | 50 |
| Shaders propios con efecto distinguible (4) | **Sol** (`planet_shaders::sun`), **Tierra** (`earth_textured`), **Ego** (`ego_textured`) y **Sovereign** (rama triplanar + gamma de `shaders::fragment_shader`). Xandar (`xandar_textured`) queda como shader adicional | Teclas 0, 2, 4 y 5; ver [Guía de demostración](#guía-de-demostración) | 24 |
| Shader animado | Granulación del sol, nubes de la Tierra, ojos de Ego que laten (y brillo pulsante de la nave) | Sol, Tierra y Ego en pantalla | 8 |
| Deformación de geometría por ruido | Knowhere, `planet_models::deform_mesh`; K y `[ ]` en vivo | Tecla 3, luego K / `[ ]` | 10 |
| Generación/subdivisión procedural antes de rasterizar | (A) `mesh_gen::uv_sphere` genera las esferas de Sol, Tierra, Xandar y Ego desde parámetros; (B) `planet_models::subdivide_mesh` subdivide Knowhere de 3 659 a 14 636 triángulos. Ambas ocurren en Rust antes de rasterizar | HUD: triángulos de Knowhere con K activada y desactivada | 10 |
| Descartar objetos fuera de vista | Frustum culling con esferas envolventes (`culling.rs`) | Contadores «Objetos» y «Tris descartados» del HUD | 7 |
| **Total del menú reclamado** | | | **109** (tope de la nota: 100) |

La deformación y la generación/subdivisión se reclaman como logros distintos. Del Orgoscope se usa su modelo de Blender; la función `mesh_gen::torus` **existe pero no está activa en la escena actual** (se construye como `MESH_RING`, que ningún objeto utiliza), por lo que no se presenta como evidencia.

Además están implementados, sin que dependamos de ellos para la meta: **colisión básica** nave-esfera (`ship::collide`, valdría 4 pts), recorte del plano cercano, estrellas de fondo y viaje rápido. **No** implementados: mapeo de normales, LOD, modelos de superficie, distribución procedural sobre planetas, modo de control local, gravedad ni aceleración progresiva.

## Rendimiento

Mediciones **sin ventana** (solo el tiempo de `render_object` a 480×320, compilación `--release`), no FPS reales de la aplicación. Comparan el commit `fe8117c` (sin culling) con `ee68464` (con culling): 15 corridas intercaladas de 40 cuadros tras 5 de calentamiento; imágenes idénticas en cada caso. La evidencia completa se conserva fuera del repositorio.

| Escenario | Triángulos enviados | Tiempo de render |
|---|---|---|
| Vista general (todo a la vista) | 40 240 → 40 240 | 2.24 → 2.21 ms (sin diferencia relevante) |
| Espacio vacío (casi todo fuera de vista) | 40 342 → **102** (−99.7 %) | 0.976 → 0.204 ms (**−79 %**) |
| Un solo planeta a la vista | 40 342 → 3 174 (−92 %) | 1.50 → 0.80 ms (−47 %) |
| Muy cerca de un planeta | 40 342 → 3 174 (−92 %) | 16.0 → 14.8 ms (−8 %) |

**Conservadurismo:** el culling se comparó contra el render sin culling en **27 749 poses** (52 barridos de cámara, incluida Knowhere deformada): **0 imágenes distintas** y 0 descartes sospechosos. Una prueba de sensibilidad con radios reducidos sí produce diferencias, así que la comprobación detecta errores.

**Estrellas:** dibujar 5 000 estrellas añade del orden de **0.04–0.06 ms por cuadro** (~2 % del render) en el mismo tipo de banco sin ventana; no cambian las estadísticas de clipping ni de culling.

## Estructura del repositorio

| Ruta | Contenido |
|---|---|
| `src/main.rs` | Bucle principal: entradas, tiempo, órbitas, dibujo, presentación |
| `src/scene.rs` | Objetos del sistema, jerarquía padre-hijo, órbitas y rotaciones |
| `src/render.rs`, `src/triangle.rs`, `src/culling.rs` | Etapa de vértices y recorte, rasterizador con z-buffer, frustum culling |
| `src/shaders.rs`, `src/planet_shaders.rs` | Shaders de fragmentos y ruido propio |
| `src/texture.rs`, `src/obj.rs` | Carga y muestreo de texturas; cargador de OBJ |
| `src/mesh_gen.rs`, `src/planet_models.rs` | Esfera UV (activa) y toro (existe, no activo en la escena) procedurales; carga de modelos de Blender, subdivisión y deformación de Knowhere |
| `src/ship.rs`, `src/camera.rs` | Nave, vuelo, colisión y cámara de seguimiento; cámara orbital |
| `src/stars.rs`, `src/hud.rs` | Campo de estrellas; interfaz |
| `src/matrix.rs`, `src/framebuffer.rs`, `src/vertex.rs`, `src/fragment.rs`, `src/uniforms.rs` | Estructuras base |
| `src/bin/snapshot.rs`, `src/bin/hud_check.rs` | Herramientas sin ventana / de medición del HUD |
| `assets/models/` | `.obj` que carga el motor (`planets/`, `ships/`) |
| `assets/blender/` | Archivos `.blend` y scripts reproducibles (Knowhere, Orgoscope, Sovereign, Milano) |
| `assets/textures/` | PNG de planetas y de la nave (`ships/`) |

## Integrantes y contribuciones

Según el historial de Git y el reparto del equipo:

| Integrante | Contribuciones |
|---|---|
| **Alejandra Sierra (24405)** | **Nave Milano**: modelo original (`assets/blender/milano/`), su UV y su textura, y el movimiento de la nave (`ship.rs`). **Cámara y navegación**: cámara de seguimiento, cámara orbital, **colisión** nave-esfera y **viaje rápido** anclado a la órbita del planeta. **Sistema de texturas** (`texture.rs`: carga de PNG y muestreo bilineal, tiled, triplanar y esférico). **Ajustes UV y texturizado de planetas**: texturas PNG y actualización de los modelos y `.blend` de los planetas (commit «texturas y nave»). **Integración** de nave, texturas y modelos en el motor (`main.rs`, `scene.rs`, `shaders.rs`, `planet_models.rs`) y el código base inicial del renderizador (primer commit) |
| **Alejandro Pérez (24136)** | **Modelado en Blender** de Knowhere, The Orgoscope y Sovereign (modelos y scripts de `assets/blender/`). **Recorte del plano cercano**. **Frustum culling** con contadores. **HUD**. **Campo de estrellas**. **Pruebas y mediciones de rendimiento**. **Documentación** (este README) |

Cada integrante explica en la demostración su contribución técnica y puede modificar en vivo un parámetro relevante (radio u órbita de un planeta, velocidad de la nave, intensidad de la deformación, semilla o cantidad de estrellas).

## Créditos y recursos externos

- **Knowhere** parte de la calavera *Human Skull* (skull-Low4K) de **CDmir**, publicada en [OpenGameArt](https://opengameart.org/content/human-skull-0) bajo licencia **CC0** (dominio público; no exige atribución, pero la damos). El original sin modificar está en [`assets/blender/knowhere/source_cdmir_skull/`](assets/blender/knowhere/source_cdmir_skull/) con su [`PROVENANCE.md`](assets/blender/knowhere/source_cdmir_skull/PROVENANCE.md); el modelo final es un derivado generado por [`build_knowhere.py`](assets/blender/knowhere/build_knowhere.py).
- Sovereign y The Orgoscope son geometría generada por script de Blender, sin recursos externos.
- **Texturas PNG** (`assets/textures/`): **PENDIENTE** de que el equipo declare aquí su procedencia (herramienta y autoría). El repositorio solo registra que se incorporaron en el commit «texturas y nave»; no se afirma cómo se elaboraron.
- Librerías: [`raylib`](https://crates.io/crates/raylib) 5.5 (ventana, entrada, mostrar el framebuffer) y [`png`](https://crates.io/crates/png) 0.17 (decodificar PNG). Herramienta: Blender 5.2.
- Los personajes, lugares y la ambientación de *Guardianes de la Galaxia* pertenecen a Marvel/Disney; este es un proyecto académico sin fines comerciales.

## Uso de IA

Base del código generada con asistencia de IA (Claude); puntualmente para la navegacion entre planetas revisada y explicada por el equipo.

Detalle actualizado:

- **Código del proyecto:** la base inicial del renderizador se generó con asistencia de IA (Claude). El recorte del plano cercano, el frustum culling, el HUD y el campo de estrellas se desarrollaron con **Claude Code**; los commits correspondientes lo declaran con `Co-Authored-By`. No afirmamos que todo el código se haya escrito a mano.
- **Modelos y scripts de Blender:** los scripts que generan Knowhere, Orgoscope y Sovereign se desarrollaron con ayuda de IA (Claude), como indican los README de `assets/`.
- **Recursos externos:** solo los de la sección anterior; la IA no aportó recursos de terceros.
- **Revisión y decisiones:** las decisiones de diseño (alcance, parámetros visuales, qué características reclamar), la revisión de cada cambio, las pruebas y la verificación visual en el juego las hizo el equipo, que explica y puede modificar el código en la demostración. La IA propuso e implementó cambios en etapas pequeñas que el equipo aprobó una por una.

## Guía de demostración

Todo se hace con los controles de la tabla de [Controles](#controles); no hay atajos ocultos. Para ver las animaciones con calma: `,` ralentiza el tiempo y P lo pausa.

| Qué mostrar | Cómo |
|---|---|
| **Sol** (shader propio, animado) | Tecla 0 (inicio). La granulación se mueve con el tiempo; con P se congela y con `,` se ralentiza. No recibe sombreado: es emisivo |
| **Tierra** | Tecla 2. Las nubes derivan con el tiempo; las luces de ciudades aparecen solo en el lado nocturno, así que con TAB (cámara orbital) y las flechas se rodea el planeta hasta ver el lado oscuro |
| **Ego** | Tecla 4. Los ojos cian de la cara **laten** con `sin(tiempo)`; la cara queda a la vista cuando el planeta rota hacia la cámara (se puede ralentizar con `,`) |
| **Sovereign** | Tecla 5. Textura dorada con mapeo triplanar (cubre todo el objeto sin depender del desenvuelto UV) y corrección gamma; se ve más clara que la textura cruda |
| **Xandar** (adicional) | Tecla 1. Ciudad radial de la textura, brillo animado en el agua y nubes ligeras |
| **Subdivisión y deformación de Knowhere** | Tecla 3 y mirar el panel de render. Con K activa, Knowhere usa la malla subdividida (14 636 triángulos) y deformada; con K desactivada, la original (3 659 triángulos): cambian el relieve y el total de triángulos. Para separar las dos operaciones: baja la intensidad con `[` hasta 0 (malla subdividida **sin** deformar: casi la misma forma que K apagada, pero con 4 veces más triángulos en el HUD) y súbela con `]` (hasta 0.30) para ver aparecer el relieve |
| **Recorte del plano cercano** | TAB para la cámara orbital, tecla 0 para centrar el Sol y acerca con W o la rueda: la superficie sigue entera aunque la cámara la roce. Al llegar muy cerca (el Sol mide 1.6; en pruebas sin ventana el contador pasó de 0 a más de 0 al bajar de unas 1.7 a 1.65 unidades), el valor **clipped** del panel de render deja de ser 0: son los triángulos que cruzan el plano cercano y se recortan. Al seguir acercándose y entrar en el Sol, solo se dibuja la cara visible desde fuera, por lo que su interior no aparece |
| **Frustum culling** | En cualquier modo, gira hasta que no haya cuerpos delante (espacio vacío): en el panel de render suben **Objetos descartados** y **Tris descartados (frustum)**, y bajan Tris y Fragmentos. Al volver a mirar los planetas, esos contadores disminuyen. Medición sin ventana de este efecto en [Rendimiento](#rendimiento) |
| **Órbitas y rotaciones** | Vista general (tecla 0 en cámara orbital): todos los planetas giran alrededor del sol y sobre su eje; con `,` y `.` cambia la velocidad del tiempo y con P se pausa. El movimiento depende del tiempo transcurrido, no de los cuadros |
| **Movimiento de la nave y cámaras** | Flechas y W/S para volar; 1–6 para ir junto a un planeta (la nave acompaña su órbita); acércate a un planeta con W y verás que la colisión impide atravesarlo; TAB alterna con la cámara orbital |

## Video de demostración

> **PENDIENTE:** aquí irá el enlace al video del recorrido (vuelo espacial → acercamiento a un planeta → vuelta alrededor → alejamiento, sin cortes ni pantallas de carga). *Sustituir este aviso por el enlace real cuando el video esté publicado.*

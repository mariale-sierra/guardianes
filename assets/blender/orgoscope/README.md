# The Orgoscope — modelo definitivo

Recurso de Blender **independiente** (no integrado en el motor Rust; `mesh_gen::torus` no se toca).
Geometría congelada = v2 aprobada. Solo se añadieron UV a núcleo, masas y ejes (en v2 se exportaban sin UV); las posiciones de vértices no cambiaron.

## Archivos
| Archivo | Para qué |
|---|---|
| `../../models/planets/orgoscope.obj` | Modelo exportado (triangulado, normales, UV, Forward −Z / Up Y). **Es el archivo a integrar.** |
| `orgoscope.blend` | Piezas separadas, editables. **Abrir este para UV/texturas.** |
| `build_orgoscope.py` | Regenera todo, desde la raíz del repositorio: `/Applications/Blender.app/Contents/MacOS/Blender --background --python assets/blender/orgoscope/build_orgoscope.py` (autocontenido; escribe también `renders/`, local e ignorado por git). `PLANETS_OUT=<carpeta>` cambia dónde se escribe el OBJ. |

## Estructura (10 piezas, líneas `o` en el OBJ) — 6,840 triángulos
| Pieza | Tris | Descripción |
|---|---|---|
| `ring_outer_grey` | 768 | Toro R=0.949, tubo 0.051 |
| `ring_pink` | 768 | Toro R=0.806, tubo 0.066 |
| `ring_inner_grey` | 640 | Toro R=0.643, tubo 0.041 |
| `ring_blue_rope` | 576 | Toro R=0.480, tubo 0.049 (liso, no trenzado) |
| `core` | 1088 | Esfera deformada (ruido suave + 3 lóbulos), aplastada en Y, con cavidad frontal y reborde |
| `eye_bezel` | 384 | Toro pequeño en el borde de la cavidad |
| `strut_a`, `strut_b` | 28 + 28 | Ejes diametrales hacia el anillo exterior |
| `mass_a`, `mass_b` | 1280 + 1280 | Masas orgánicas donde el anillo exterior y el rosa se acercan más |

Todos los anillos comparten el centro (0,0,0); son toros exactos (error de circularidad/grosor ~1e-7). Radio englobante = 1 (lo define el anillo exterior). El "ojo" mira a +Z en el OBJ.
Los parámetros editables están al inicio del script (`RINGS`, `CORE_*`, `EYE_*`, `MASS_*`, `STRUT_*`).

## Pendiente para texturizado
- UV: anillos y bisel tienen UV por vuelta mayor/menor del tubo (u,v∈[0,1], costuras en la vuelta). Núcleo, masas y ejes usan el desenvuelto automático de `bmesh` (esfera/icosfera/cilindro): **no se revisó su distorsión ni su aprovechamiento de textura**; las UV de las piezas **no están empaquetadas en un atlas común** (cada pieza usa 0–1 completo).
- El OBJ no lleva materiales; el color por pieza solo existe en el `.blend` (para renders).
- Anillo azul liso (la referencia es una soga trenzada) y anillos sin perforaciones: son detalles de textura/shader.
- Las masas se solapan con el anillo rosa/exterior (objetos separados, no soldados).
- `obj.rs` fusiona todas las piezas en una sola lista de triángulos; si se quiere distinguirlas (anillos vs. masas) en el shader habrá que cargar piezas por separado o usar posición/UV.
- Tamaño: 6,840 tris, en la parte alta del presupuesto (3k–8k). No hay versión LOD.

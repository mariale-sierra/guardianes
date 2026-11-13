# Knowhere — modelo definitivo

Cabeza celestial gigantesca, deteriorada, con una estación industrial excavada en el lateral del cráneo.
Recurso **independiente** para el renderer: `mesh_gen::knowhere_shape` (deformación procedural en Rust, 10 puntos de la rúbrica) **no se toca ni se sustituye**.
Estado: preparado para integrar. **No** está integrado en el renderer; no se modificó ningún archivo de Rust.

## Archivos
| Archivo | Para qué sirve |
|---|---|
| `../../models/planets/knowhere.obj` | Modelo triangulado con normales y UV. **Es el archivo a integrar en el motor.** |
| `knowhere.blend` | Escena editable de Blender 5.2 (10 piezas separadas, material provisional sin texturas). **Abrir este para trabajar UV/texturas.** |
| `build_knowhere.py` | Script autocontenido que reproduce todo el modelo desde la calavera CC0. |
| `source_cdmir_skull/` | Calavera original CC0 (`skull-Low4K.obj` + `.mtl`, entrada del script) y `PROVENANCE.md` (autor, licencia, URL). |

## Regenerar el modelo
Desde la raíz del repositorio:
```bash
/Applications/Blender.app/Contents/MacOS/Blender --background --python assets/blender/knowhere/build_knowhere.py
```
Escribe `knowhere.blend` (y `renders/`, carpeta local ignorada por git) junto al script y el OBJ en `assets/models/planets/`. Es determinista (semillas fijas) y solo depende de `source_cdmir_skull/skull-Low4K.obj`.
Variables opcionales: `PLANETS_OUT=<carpeta>` y `KNOWHERE_OUT=<carpeta>` para probar sin tocar el repositorio; `KNOWHERE_JAW_FIX=0` reproduce la variante D aprobada (sin la última corrección de mandíbula).

## Cadena de etapas del script
1. **v3** — deformación orgánica suave de la calavera (bóveda, frente, pómulos, asimetría).
2. **v4** — cráter irregular en el lateral superior izquierdo (subdivisión local + excavación).
3. **v5** — zona industrial dentro del cráter: puerto de atraque con túnel (`dock_tube`, `dock_flange`), 2 vigas, 2 soportes, 3 peldaños.
4. **D** — cara lisa: nariz eliminada y rellena, sin dientes, mandíbula cerrada y unida.
5. **Corrección de mandíbula** — punta más corta (compresión vertical 0.72), terminación más ancha (×1.55 en el extremo), borde inferior fracturado, irregular y asimétrico. Solo vértices con `z < -0.50`.
6. **Normalización** uniforme (origen en el centro de la caja, radio máximo 1.0).

## Estadísticas
- **OBJ**: 1 871 vértices · 3 659 triángulos · 2 454 `vt` · 1 804 `vn` · 10 objetos `o`.
- Cráneo: 1 725 vértices, 3 405 triángulos (una sola isla). Estructuras industriales: 254 triángulos en 9 piezas cerradas.
- Presupuesto del proyecto (planeta 3 k–8 k triángulos): dentro.
- Caja: x ±0.5325, y ±0.7649 (arriba), z ±0.6967 · centro (0,0,0) · radio máximo 1.0000.

### Orden de los triángulos en el OBJ (para separar piezas al integrar)
`obj.rs` ignora las líneas `o` y devuelve **un solo `Vec<Vertex>`**; estas son las posiciones de triángulo por pieza (el índice de vértice es `3 × triángulo`):

| Pieza | Triángulos (índice) | Cantidad |
|---|---|---|
| `knowhere_skull` | 0 .. 3398 | 3399 |
| `dock_tube` | 3399 .. 3490 | 92 |
| `dock_flange` | 3491 .. 3574 | 84 |
| `beam_main` | 3575 .. 3586 | 12 |
| `beam_diag` | 3587 .. 3598 | 12 |
| `post_a` | 3599 .. 3610 | 12 |
| `post_b` | 3611 .. 3622 | 12 |
| `step0` | 3623 .. 3634 | 12 |
| `step1` | 3635 .. 3646 | 12 |
| `step2` | 3647 .. 3658 | 12 |

## Convenciones de ejes
- En Blender: Z arriba, la cara mira a −Y.
- OBJ exportado con **Forward −Z, Up Y**: **Y arriba y la cara mira a +Z** (mismo eje de "cara" de `mesh_gen::knowhere_shape` y de `snapshot ... face`).
- La cavidad industrial queda en el lado **−X**. Verificado: z medio de la mandíbula = +0.48.
- Triángulos CCW vistos desde fuera; sin materiales (`usemtl`/`mtllib` ausentes).

## UV y normales
- UV de la calavera original (atlas del autor), conservadas en el cráneo. `u` y `v` están en [0, 1] y no hay triángulos que crucen la costura.
- `obj.rs` **no invierte `v`**: el OBJ usa `v = 1` arriba (convenio estándar). Si una textura sale al revés, invertir al muestrear.
- **Limitación de UV:** 3 triángulos del relleno de la nariz tienen área UV ≈ 0; la zona de la boca/nariz fue modificada por código y su UV es aproximada; las paredes del cráter están estiradas. Para texturas pintadas habrá que desenvolver de nuevo o proyectar localmente.
- Normales: unitarias (0.9999–1.0001); suaves, con aristas duras en las paredes del cráter; estructuras con sombreado plano.

## Resultado de las verificaciones (hechas al preparar el modelo)
- `obj.rs` (`src/obj.rs` real del proyecto, comprobado en un entorno temporal): carga sin errores; 3 659 triángulos; 0 degenerados; 0 valores no finitos.
- 0 triángulos degenerados; 0 caras realmente invertidas tras corregir 1 interior; 9 astillas de área <1e-4 (ruido del booleano) con normales de vértice ambiguas.
- Cavidad presente: el suelo del cráter queda a 0.329 del centro frente a 0.616 en el lado opuesto; `dock_tube` y `dock_flange` presentes.
- Aristas abiertas: **61** en el cráneo, todas en el interior/base de la cabeza (z entre −0.32 y −0.49 en ejes Blender) y no en la superficie exterior visible. No hay aristas no-manifold.

## Nota: 3 665 triángulos en el .blend vs 3 659 en el OBJ
El cráneo conserva n-gonos en Blender (suma de n−2 = 3 405 triángulos); al exportar con *Triangulated Mesh* el exportador de Blender genera 3 399 (6 menos). Se comprobó que es propio del exportador (reexportar da siempre 3 399), que los 1 725 vértices del cráneo siguen usados y que no hay caras degeneradas ni geometría alterada. El OBJ es la referencia para el motor.
## Materiales
El `.mtl` de la calavera original apuntaba a texturas (`tex/*.png`) que no se distribuyen. El script las sustituye por un material provisional (`knowhere_placeholder`) sin imágenes: sin cambios de geometría ni UV, y sin avisos de recursos faltantes al abrir el `.blend`.

## Limitaciones conocidas
- El renderer por software dibuja caras por ambos lados solo si se desactiva el backface culling; el interior de la base tiene aristas abiertas.
- Todo el modelo comparte un solo `Vec<Vertex>` al cargarse con `obj.rs`; para dar otro shader a las estructuras hay que separar por los rangos de la tabla.
- Estructuras muy pequeñas a 480×320: el puerto mide pocas decenas de píxeles.

## Procedencia y licencia
Calavera base: *Human Skull* (skull-Low4K) de **CDmir**, OpenGameArt, **CC0**. Detalle en `source_cdmir_skull/PROVENANCE.md`.
Las deformaciones, el cráter, la zona industrial, la cara lisa y la corrección de mandíbula las generó el script de esta carpeta, con ayuda de IA (Claude); declararlo en el README principal del proyecto.

# Sovereign — modelo definitivo (geometría aprobada)

Recurso de Blender **independiente** (no integrado en el motor Rust). Estructura planetaria dorada: núcleo + 5 esferas secundarias unidas por collares, cuellos y puentes, con bandas, ornamentos concéntricos y paneles hexagonales. Referencia: render del filme (esferas fusionadas, luces de ciudad) y la ilustración dorada (discos concéntricos, cúpulas anilladas).

## Archivos
| Archivo | Para qué |
|---|---|
| `../../models/planets/sovereign.obj` | Exportado: triangulado, normales, UV, Forward −Z / Up Y, sin materiales. **Es el archivo a integrar.** |
| `sovereign.blend` | 36 piezas separadas, con materiales de previsualización (sin imágenes). **Abrir este para UV/texturas.** |
| `build_sovereign.py` | Regenera todo, desde la raíz del repositorio: `/Applications/Blender.app/Contents/MacOS/Blender --background --python assets/blender/sovereign/build_sovereign.py` (también escribe `renders/`, local e ignorado por git). `PLANETS_OUT=<carpeta>` cambia dónde se escribe el OBJ. |

## Parámetros editables (inicio del script)
`CORE_RADIUS`, `CORE_SEG/RINGS`, `SPHERES` (centro, radio, resolución), `LINKS` (qué se conecta con qué), `COLLAR_TUBE`, `NECK_RADIUS`, `FLANGE_TUBE`, `BANDS`, `ORNAMENTS`, `PLATES`, `MATS`. No hay azar: mismas posiciones siempre. Collares, cuellos y bridas se **calculan** a partir de las esferas (círculo de intersección o hueco entre ellas), así que mover/redimensionar una esfera arrastra su conexión.

## Estructura — 6,476 triángulos (objetivo 3k–6k, máx. 8k)
Escalas relativas al núcleo = 1.0 antes de normalizar (radio final del núcleo ≈ 0.395).
| Esfera | Radio | Posición (x,y,z, Blender) | Conexión |
|---|---|---|---|
| `core` | 1.00 | (0,0,0) | — |
| `sphere_b_upper_right` | 0.66 | (1.42, 0.30, 0.30) | collar con núcleo |
| `sphere_c_top` | 0.36 | (0.50, 0.50, 1.05) | collar con núcleo + puente cilíndrico a B |
| `sphere_d_right` | 0.30 | (2.05, 0.05, −0.10) | collar con B |
| `sphere_e_lower_left` | 0.58 | (−1.02, −0.30, −0.92) | collar con núcleo |
| `sphere_f_far_lower_left` | 0.22 | (−1.60, −0.50, −1.30) | cuello cilíndrico + 2 bridas a E |

Detalles: 2 bandas en el núcleo (clara inclinada + oscura), bandas en B, C, D; discos concéntricos (3 anillos + botón) en la cara frontal del núcleo, 2 en B, "puerto" de 2 anillos en E; 7 paneles hexagonales en relieve.

## Pivote de rotación (origen del OBJ = centro del núcleo)
El origen del OBJ es el **centro del núcleo** (verificado: (0,0,0) con error < 1e-6). Como `Mat4` rota todo alrededor del origen del modelo, `spin` hace girar la estructura sobre un eje que pasa por el núcleo, y las esferas secundarias orbitan en torno a él como una sola pieza.
El cambio fue una **traslación pura** de (+0.1136, −0.0217, 0.0) en ejes OBJ respecto a la versión anterior (versión anterior archivada fuera del repositorio): mismos tamaños, mismas posiciones relativas, misma escala (0.3948). Verificado numéricamente (desviación 1.5e-6). Consecuencia: la caja envolvente ya no está centrada (centro ≈ (0.114, −0.022, 0)) y el radio englobante desde el pivote es 0.9447, no 1.

## Pendiente / limitaciones
- 6,476 tris: ligeramente sobre el objetivo de 6k (bajo el máximo de 8k). Si se necesita menos: bajar `CORE_SEG/RINGS`, quitar un anillo de `ORNAMENTS` o `BANDS`. Sin versión LOD.
- Las esferas **se solapan sin soldarse** (piezas cerradas independientes); hay triángulos ocultos dentro de otras esferas.
- La silueta a 160×160 es un cúmulo compacto: se lee como "esferas unidas" pero B/D se funden con el núcleo en algunas vistas.
- Los anillos de bandas/ornamentos son relieves salientes; no hay zonas hundidas (sin booleanos).
- Radio englobante desde el pivote = 0.9447 (no exactamente 1; cabe en la esfera unidad, así que un culling con radio 1 sigue siendo conservador).
- UV: esferas con UV de esfera UV (polo en Z Blender = Y del OBJ), toros por vuelta mayor/menor, cada pieza usa 0–1 completo, **sin atlas común**; sin revisar distorsión en polos.
- `obj.rs` ignora las líneas `o`: las 36 piezas se cargan como una sola lista; para distinguir oro claro/oscuro en el shader habría que usar posición/UV.
- El orden de líneas `vt` del exportador de Blender varía entre ejecuciones; la geometría es idéntica.

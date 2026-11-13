# Assets del proyecto

## Modelos de planetas (Blender)
| Modelo | OBJ para el motor | Blend editable | Script |
|---|---|---|---|
| Knowhere | `models/planets/knowhere.obj` | `blender/knowhere/knowhere.blend` | `blender/knowhere/build_knowhere.py` |
| The Orgoscope | `models/planets/orgoscope.obj` | `blender/orgoscope/orgoscope.blend` | `blender/orgoscope/build_orgoscope.py` |
| Sovereign | `models/planets/sovereign.obj` | `blender/sovereign/sovereign.blend` | `blender/sovereign/build_sovereign.py` |

**Para texturizar (Blender 5.2):** abre el `.blend` del planeta, edita UV y crea la textura, y guarda el PNG en `textures/` (≤1024 px). Cada carpeta de `blender/<planeta>/` tiene un README con estructura, UV y limitaciones.
**No** edites los `.obj` a mano: son salida de exportación. Si cambias el `.blend`, reexporta con Selection Only · Triangulated Mesh · Write Normals · Include UVs · Forward −Z · Up Y, o ajusta el script.

Los modelos son recursos independientes: `mesh_gen::knowhere_shape` (deformación por ruido) y `mesh_gen::torus` (toros procedurales) en Rust no se sustituyen.

## Procedencia y licencia
- Knowhere parte de la calavera *Human Skull* (skull-Low4K) de CDmir, OpenGameArt, **CC0** (`blender/knowhere/source_cdmir_skull/PROVENANCE.md`).
- Orgoscope y Sovereign son geometría generada por script (sin recursos externos).
- Los scripts y modelos se desarrollaron con ayuda de IA (Claude); se declara también en el README principal.

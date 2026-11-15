# Milano (Star-Lord) — nave del jugador

Es el modelo ORIGINAL de Ale Sierra (`lab3_original.blend`, forma sin cambios), desenvuelto y pintado por script.

| Archivo | Para qué |
|---|---|
| `lab3_original.blend` | Modelo original (objeto `Cube` + Mirror modifier "Simetría X") |
| `milano.blend` | Salida: modelo con el espejo aplicado, UV y la textura conectada (verlo en Material Preview) |
| `build_milano.py` | Script: aplica el espejo, centra/escala, gira la nariz, UV (Smart UV Project), pinta y exporta |
| `../../models/ships/milano.obj` | Para el motor (triangulado, normales, UV, Forward −Z, Up Y) |
| `../../textures/ships/milano.png` | Textura 1024×1024 |

Regenerar (desde la raíz): `python assets/blender/milano/build_milano.py` (`pip install bpy numpy pillow`)
o `blender -b -P assets/blender/milano/build_milano.py`.

## Cómo se pinta
Para cada triángulo se recorren sus texels en la UV, se interpola su posición 3D y el color sale de una
función de esa posición (`colorize`): base azul-gris con desgaste, franjas naranjas en V (niveles de `y + s·|x|`),
franja central, puntas de ala con borde naranja, cabina de vidrio y brillo cian atrás. Así la pintura
no depende del desenvuelto. En el juego, `ship::shade` hace brillar los píxeles cian y amarillos.

## Parámetros fáciles de cambiar (arriba del script)
`SHIP_SPAN` (ancho final), `CHEVRON_SLOPE` (inclinación de las franjas), `SEED`; los colores están en `colorize`.

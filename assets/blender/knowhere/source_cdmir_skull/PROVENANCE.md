# Procedencia: cráneo base de Knowhere

- Modelo: "Human Skull" (skull-Low4K)
- Autor: CDmir (OpenGameArt)
- Página: https://opengameart.org/content/human-skull-0  (publicado 2018-09-23)
- Licencia: CC0 (dominio público). No exige atribución; la damos igualmente en el README.
- Descargado el 2026-11-12 desde https://opengameart.org/sites/default/files/skull-obj.zip
  y https://opengameart.org/sites/default/files/skull-Low4K.blend
- Descripción del autor: 4538 tris, con texturas.
- En esta carpeta: skull-Low4K.obj / skull-Low4K.mtl, sin modificar (extraídos del zip original; las texturas del autor no se incluyen porque el modelo definitivo no las usa; el .mtl las nombra pero el script las sustituye por un material provisional). El zip y el .blend originales se conservan fuera del repositorio; se pueden volver a descargar de las URL anteriores.
- Los derivados (modificados) NO van aquí; este directorio es solo la base original.

## Derivado en este proyecto
- Modelo definitivo de Knowhere: `../../../models/planets/knowhere.obj` / `../knowhere.blend`, generado por `../build_knowhere.py`
  a partir de `skull-Low4K.obj` (aquí incluido sin modificar; `skull-obj.zip` es el archivo original descargado, con sus texturas).
- El derivado se publica con el proyecto; la licencia CC0 de la calavera original no exige atribución, pero se declara por transparencia.

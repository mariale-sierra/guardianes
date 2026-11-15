# build_milano.py
# Toma la nave ORIGINAL de Ale Sierra (lab3_original.blend, sin cambiar su forma), la desenvuelve (UV),
# la pinta con una textura procedural y exporta OBJ + PNG para el motor.
#
# Uso (desde la raiz del proyecto):   python assets/blender/milano/build_milano.py     (necesita: pip install bpy numpy pillow)
#                              o:     blender -b -P assets/blender/milano/build_milano.py
#
# Salidas:  assets/blender/milano/milano.blend   (modelo con la textura conectada)
#           assets/models/ships/milano.obj       (triangulado, normales, UV, nariz hacia -Z, arriba +Y)
#           assets/textures/ships/milano.png     (1024x1024)
#
# COMO SE PINTA: la UV la genera Smart UV Project. Para cada triangulo se recorren sus texels en el
# espacio UV, se interpola su posicion 3D y el color se calcula con una funcion de esa posicion 3D
# (franjas, cabina, motores...). Asi la pintura no depende de como quedo el desenvuelto.

import math
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SRC_BLEND = HERE / "lab3_original.blend"
OUT_BLEND = HERE / "milano.blend"
OUT_OBJ = ROOT / "assets/models/ships/milano.obj"
OUT_PNG = ROOT / "assets/textures/ships/milano.png"

# ------------------------------------------------------------ parametros demostrables
SHIP_SPAN = 2.6        # envergadura final (ancho total) en unidades del motor
TEX_SIZE = 1024
SEED = 7
CHEVRON_SLOPE = 0.55   # inclinacion de las franjas en V (mayor = V mas cerrada)


def load_original():
    """Abre la nave original, aplica el Mirror modifier y la deja centrada, con la nariz hacia +Y y a escala."""
    bpy.ops.wm.open_mainfile(filepath=str(SRC_BLEND))
    ob = bpy.data.objects["Cube"]
    bpy.context.view_layer.objects.active = ob
    for m in list(ob.modifiers):
        bpy.ops.object.modifier_apply(modifier=m.name)       # el espejo X ya es simetrico respecto al origen del objeto
    me = ob.data
    bb = [v.co for v in me.vertices]
    c = Vector(((max(p.x for p in bb) + min(p.x for p in bb)) / 2,
                (max(p.y for p in bb) + min(p.y for p in bb)) / 2,
                (max(p.z for p in bb) + min(p.z for p in bb)) / 2))
    span = max(p.x for p in bb) - min(p.x for p in bb)
    k = SHIP_SPAN / span
    for v in me.vertices:
        p = (v.co - c) * k
        # El modelo original tiene la punta (nariz) hacia -Y. Se gira 180 grados sobre Z (x,y,z)->(-x,-y,z)
        # para construir con la nariz en +Y; el exportador la convierte a -Z. Es una rotacion: no invierte caras.
        v.co = Vector((-p.x, -p.y, p.z))
    me.update()
    ob.matrix_world.identity()
    ob.name = "milano"
    me.name = "milano_mesh"
    for p in me.polygons:
        p.use_smooth = False
    return ob


def unwrap(ob):
    """Smart UV Project (equivalente a UV > Smart UV Project en Edit Mode)."""
    me = ob.data
    for uv in list(me.uv_layers):
        me.uv_layers.remove(uv)
    me.uv_layers.new(name="UVMap")
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.015)
    bpy.ops.object.mode_set(mode="OBJECT")


# ------------------------------------------------------------ pintura
def smooth(e0, e1, v):
    k = np.clip((v - e0) / (e1 - e0), 0, 1)
    return k * k * (3 - 2 * k)


def hash3(ix, iy, iz):
    h = (ix * 374761393 + iy * 668265263 + iz * 2147483647 + SEED * 97) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65535.0


def noise3(p):
    """Ruido de valor 3D (interpolacion trilineal), p: (N,3)."""
    i = np.floor(p).astype(np.int64)
    f = p - i
    f = f * f * (3 - 2 * f)
    out = 0.0
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                w = (f[:, 0] if dx else 1 - f[:, 0]) * (f[:, 1] if dy else 1 - f[:, 1]) * (f[:, 2] if dz else 1 - f[:, 2])
                out = out + w * hash3(i[:, 0] + dx, i[:, 1] + dy, i[:, 2] + dz)
    return out


def colorize(pos, nrm, span):
    """Color de cada punto 3D de la nave. pos y nrm: (N,3). Eje: x ancho, y adelante (nariz +Y), z arriba."""
    x, y, z = pos[:, 0], pos[:, 1], pos[:, 2]
    ax = np.abs(x)
    blue = np.array([96, 134, 164]) / 255
    dark = np.array([40, 62, 84]) / 255
    orange = np.array([228, 120, 28]) / 255
    glass = np.array([34, 78, 96]) / 255
    glow = np.array([100, 225, 255]) / 255

    wear = 0.6 * noise3(pos * 3.0) + 0.4 * noise3(pos * 11.0)
    col = blue[None, :] * (0.78 + 0.40 * wear[:, None])
    # las caras que miran hacia abajo son un poco mas oscuras
    down = smooth(0.0, -0.6, nrm[:, 2])
    col = col * (1 - 0.18 * down[:, None])

    # franjas naranjas en V siguiendo las alas: niveles de (y + s*|x|)
    s = CHEVRON_SLOPE
    t = (y + s * ax) / (span * 0.16)
    band = (np.abs(np.sin(t * math.pi * 0.5 + 0.35)) > 0.55).astype(float)
    band = smooth(0.0, 1.0, band)
    band = np.where(ax < span * 0.40, band, 0.0)
    # franja central (lomo)
    stripe = 1 - smooth(span * 0.025, span * 0.045, ax)
    paint = np.clip(band + stripe * 0.9, 0, 1)
    col = col * (1 - paint[:, None]) + orange[None, :] * (0.88 + 0.25 * wear[:, None]) * paint[:, None]

    # puntas de ala oscuras con borde naranja
    tip = smooth(span * 0.38, span * 0.44, ax)
    col = col * (1 - 0.25 * tip[:, None]) + dark[None, :] * 0.25 * tip[:, None]
    edge = (1 - smooth(0.0, span * 0.012, np.abs(ax - span * 0.485)))
    col = col * (1 - 0.8 * edge[:, None]) + orange[None, :] * 0.8 * edge[:, None]

    # cabina: vidrio sobre el lomo, hacia la nariz
    top = smooth(0.2, 0.6, nrm[:, 2])
    cab = (1 - smooth(span * 0.05, span * 0.075, ax)) * smooth(y.min() + (y.max() - y.min()) * 0.40,
                                                                 y.min() + (y.max() - y.min()) * 0.46, y) \
          * (1 - smooth(y.min() + (y.max() - y.min()) * 0.78, y.min() + (y.max() - y.min()) * 0.84, y)) * top
    spec = 0.5 + 0.5 * np.sin(y * 9 + x * 5)
    col = col * (1 - cab[:, None]) + (glass[None, :] * (0.9 + 0.5 * spec[:, None])) * cab[:, None]

    # motores: brillo cian en las caras que miran hacia atras, cerca del centro
    back = smooth(0.35, 0.7, -nrm[:, 1])
    eng = back * (1 - smooth(span * 0.05, span * 0.08, ax)) * (1 - smooth(y.min() + (y.max() - y.min()) * 0.15, y.min() + (y.max() - y.min()) * 0.30, y))
    eng = eng * 0.9
    col = col * (1 - eng[:, None]) + glow[None, :] * eng[:, None]

    # lineas de panel
    gx = np.abs(np.sin(x * 9.0)) ** 0.1
    gy = np.abs(np.sin(y * 7.0 + z * 3.0)) ** 0.1
    line = 1 - np.minimum(gx, gy)
    col = col * (1 - 0.30 * smooth(0.03, 0.10, line)[:, None])
    return np.clip(col, 0, 1)


def paint_texture(ob):
    me = ob.data
    me.calc_loop_triangles()
    uvl = me.uv_layers[0].data
    span = max(v.co.x for v in me.vertices) * 2
    N = TEX_SIZE
    pts_px, pts_py, pts_pos, pts_nrm = [], [], [], []
    for tri in me.loop_triangles:
        uv = np.array([uvl[li].uv[:] for li in tri.loops]) * N            # coordenadas en pixeles (v hacia arriba)
        pos = np.array([me.vertices[vi].co[:] for vi in tri.vertices])
        nrm = np.array(tri.normal[:])
        x0, x1 = int(np.floor(uv[:, 0].min())) - 1, int(np.ceil(uv[:, 0].max())) + 1
        y0, y1 = int(np.floor(uv[:, 1].min())) - 1, int(np.ceil(uv[:, 1].max())) + 1
        xs, ys = np.meshgrid(np.arange(max(x0, 0), min(x1, N)), np.arange(max(y0, 0), min(y1, N)))
        px, py = xs.ravel() + 0.5, ys.ravel() + 0.5
        a, b, c = uv
        den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(den) < 1e-12:
            continue
        l0 = ((b[1] - c[1]) * (px - c[0]) + (c[0] - b[0]) * (py - c[1])) / den
        l1 = ((c[1] - a[1]) * (px - c[0]) + (a[0] - c[0]) * (py - c[1])) / den
        l2 = 1 - l0 - l1
        inside = (l0 >= -0.02) & (l1 >= -0.02) & (l2 >= -0.02)       # pequeno margen: evita huecos en las aristas
        if not inside.any():
            continue
        w = np.stack([l0[inside], l1[inside], l2[inside]], axis=1)
        pts_px.append(xs.ravel()[inside]); pts_py.append(ys.ravel()[inside])
        pts_pos.append(w @ pos); pts_nrm.append(np.tile(nrm, (inside.sum(), 1)))
    px = np.concatenate(pts_px); py = np.concatenate(pts_py)
    pos = np.concatenate(pts_pos); nrm = np.concatenate(pts_nrm)
    col = colorize(pos, nrm, span)

    img = np.zeros((N, N, 3), np.float32)
    mask = np.zeros((N, N), bool)
    img[py, px] = col
    mask[py, px] = True
    # relleno de los bordes de las islas (3 pasadas) para que el filtro bilineal no mezcle con negro
    for _ in range(4):
        grown = img.copy(); gm = mask.copy()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            sh_m = np.roll(mask, (dy, dx), axis=(0, 1)); sh_i = np.roll(img, (dy, dx), axis=(0, 1))
            new = sh_m & ~gm
            grown[new] = sh_i[new]; gm |= new
        img, mask = grown, gm
    img[~mask] = np.array([0.30, 0.43, 0.55])
    out = np.flipud(img)                       # la fila 0 del PNG es la de ARRIBA (v = 1)
    OUT_PNG.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray((out * 255).astype(np.uint8), "RGB").save(OUT_PNG)


def main():
    ob = load_original()
    unwrap(ob)
    paint_texture(ob)

    mat = bpy.data.materials.new("milano_mat")
    mat.use_nodes = True
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = bpy.data.images.load(str(OUT_PNG))
    mat.node_tree.links.new(tex.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
    ob.data.materials.clear()
    ob.data.materials.append(mat)

    for o in bpy.data.objects:
        o.select_set(o is ob)
    bpy.ops.wm.obj_export(
        filepath=str(OUT_OBJ), export_selected_objects=True, export_triangulated_mesh=True,
        export_normals=True, export_uv=True, export_materials=False,
        forward_axis="NEGATIVE_Z", up_axis="Y",
    )
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT_BLEND), relative_remap=True)
    d = ob.dimensions
    print("triangulos:", len(ob.data.loop_triangles) or sum(len(p.vertices) - 2 for p in ob.data.polygons),
          "| dims x,y,z:", round(d.x, 3), round(d.y, 3), round(d.z, 3))


main()

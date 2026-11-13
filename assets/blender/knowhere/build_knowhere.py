"""
KNOWHERE — modelo definitivo. Script ÚNICO y AUTOCONTENIDO: reproduce todo el modelo desde la calavera CC0 original.
Ejecutar (desde cualquier carpeta):
    /Applications/Blender.app/Contents/MacOS/Blender --background --python build_knowhere.py
Entrada : source_cdmir_skull/skull-Low4K.obj  (calavera de CDmir, CC0 — ver PROVENANCE.md)
Salida  : knowhere.blend y renders/*.png (junto a este script; renders es local), ../../models/planets/knowhere.obj

Variables de entorno opcionales (solo para verificación):
    KNOWHERE_JAW_FIX=0     omite la corrección final de mandíbula (reproduce la variante D aprobada)
    KNOWHERE_OUT=<dir>     carpeta de salida alternativa (por defecto, la de este script)
    KNOWHERE_COMPARE_OBJ=<ruta.obj>   OBJ de la variante D, para generar los renders antes/después

Cadena de etapas (todas conservadas tal cual fueron aprobadas):
  1  v3   deformación orgánica suave de la calavera (bóveda, frente, pómulos, asimetría) + normalización (radio 1, centrado)
  2  v4   cráter irregular en el lateral superior izquierdo: subdivisión local + excavación
  3  v5   zona industrial dentro del cráter: puerto de atraque con túnel, 2 vigas, 2 soportes, 3 peldaños (objetos separados)
  4  D    cara lisa: se elimina la nariz (relleno + ajuste a superficie suave); dientes fuera; mandíbula cerrada y unida
  5  fix  corrección de mandíbula: acortar la punta, ensanchar la terminación, borde inferior fracturado e irregular

Ejes Blender: Z arriba, la cara mira a -Y.  OBJ exportado: Forward -Z, Up Y  (la cara queda en +Z del OBJ).
mesh_gen::knowhere_shape (Rust) NO se toca: este modelo es un recurso independiente.
"""
import bpy, bmesh, math, os, random, tempfile
import numpy as np
from mathutils import Vector, Matrix, noise
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.environ.get("KNOWHERE_OUT", HERE); os.makedirs(OUT, exist_ok=True)   # .blend y renders/
# El OBJ definitivo va a assets/models/planets/ (o a PLANETS_OUT, para probar sin tocar el repositorio).
PLANETS_DIR = os.path.abspath(os.environ.get("PLANETS_OUT", os.path.join(HERE, "..", "..", "models", "planets")))
os.makedirs(PLANETS_DIR, exist_ok=True)
RENDERS = os.path.join(OUT, "renders"); os.makedirs(RENDERS, exist_ok=True)
SRC_OBJ = os.path.join(HERE, "source_cdmir_skull", "skull-Low4K.obj")
JAW_FIX = os.environ.get("KNOWHERE_JAW_FIX", "1") != "0"
COMPARE_OBJ = os.environ.get("KNOWHERE_COMPARE_OBJ", "")
R = math.radians


def sstep(x, e0, e1):
    """smoothstep (admite e0 > e1 para caída invertida)."""
    if e0 == e1: return 0.0
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0))); return t * t * (3 - 2 * t)

def tris(o): return sum(len(p.vertices) - 2 for p in o.data.polygons)

def islands(bm):
    seen = set(); out = []
    for v in bm.verts:
        if v.index in seen: continue
        st = [v]; comp = []; seen.add(v.index)
        while st:
            a = st.pop(); comp.append(a)
            for e in a.link_edges:
                b = e.other_vert(a)
                if b.index not in seen: seen.add(b.index); st.append(b)
        out.append(comp)
    return sorted(out, key=len, reverse=True)


bpy.ops.wm.read_factory_settings(use_empty=True)

# =====================================================================================================================
# ETAPA 1 (v3): deformación orgánica suave. Solo se mueven vértices (misma topología y UV que la calavera original).
# =====================================================================================================================
VAULT_SCALE = 0.13; VAULT_BACK = 0.12; VAULT_UP = 0.05; FOREHEAD = 0.08
CHEEK_OUT = 0.09; CHEEK_FWD = 0.05
ASYM_SHIFT = 0.035; ASYM_SCALE = 0.05; ASYM_NOISE = 0.028
V3_SEED = 5

def deform_v3(p, c):
    q = p.copy()
    w_top = sstep(p.z, -0.10, 0.80); w_post = sstep(p.y, -0.20, 1.10)
    wv = w_top * (0.35 + 0.65 * w_post)
    q = c + (q - c) * (1 + VAULT_SCALE * wv)
    q.y += VAULT_BACK * w_post * w_top; q.z += VAULT_UP * w_top * w_post
    wf = sstep(-p.y, 0.25, 1.0) * sstep(p.z, 0.10, 0.70) * sstep(p.z, 1.10, 0.75) * (1.0 - sstep(abs(p.x), 0.35, 0.85))
    q.y -= FOREHEAD * wf; q.z += 0.02 * wf
    side = 1.0 if p.x >= 0 else -1.0
    bell_z = sstep(p.z, -0.62, -0.40) * sstep(p.z, -0.12, -0.32)
    wc = sstep(abs(p.x), 0.30, 0.62) * bell_z * sstep(-p.y, 0.15, 0.65)
    k = 1.0 + (0.35 if p.x < 0 else 0.0)
    q.x += side * CHEEK_OUT * wc * k; q.y -= CHEEK_FWD * wc * k
    q.x += ASYM_SHIFT * wv
    if p.x > 0: q.x += p.x * ASYM_SCALE * wv
    n = Vector((noise.noise(p * 1.7 + Vector((V3_SEED, 0, 0))), noise.noise(p * 1.7 + Vector((0, V3_SEED, 0))),
                noise.noise(p * 1.7 + Vector((0, 0, V3_SEED)))))
    q += n * ASYM_NOISE * wv
    return q

bpy.ops.wm.obj_import(filepath=SRC_OBJ)
skull = [x for x in bpy.context.scene.objects if x.type == 'MESH'][0]; skull.name = "knowhere_skull"
bpy.ops.object.select_all(action='DESELECT'); skull.select_set(True); bpy.context.view_layer.objects.active = skull
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)   # el importador solo ROTA el objeto: se hornea (Z arriba)
# El .mtl del cráneo original referencia texturas (tex/*.png) que NO se incluyen en el repositorio: se sustituye el
# material importado por uno provisional sin imágenes (no cambia geometría ni UV; las texturas se harán aparte).
for _m in list(skull.data.materials): pass
_mat = bpy.data.materials.new("knowhere_placeholder"); _mat.diffuse_color = (0.72, 0.70, 0.64, 1.0)
skull.data.materials.clear(); skull.data.materials.append(_mat)
for _img in list(bpy.data.images): bpy.data.images.remove(_img)
for _m in list(bpy.data.materials):
    if _m.users == 0: bpy.data.materials.remove(_m)
me = skull.data
tris_base = tris(skull)
for v in me.vertices: v.co = deform_v3(v.co, Vector((0.0, 0.22, 0.10)))
me.update()
try: bpy.ops.mesh.customdata_custom_splitnormals_clear()
except Exception: pass
bpy.ops.object.shade_smooth()
# normalizar: centro del bbox en el origen y radio englobante 1
_lo = Vector((min(v.co.x for v in me.vertices), min(v.co.y for v in me.vertices), min(v.co.z for v in me.vertices)))
_hi = Vector((max(v.co.x for v in me.vertices), max(v.co.y for v in me.vertices), max(v.co.z for v in me.vertices)))
_cen = (_lo + _hi) / 2
for v in me.vertices: v.co -= _cen
_maxr = max(v.co.length for v in me.vertices)
for v in me.vertices: v.co /= _maxr
me.update()
print("ETAPA 1 (v3): tris", tris(skull), "(base", tris_base, ") escala normalización", round(1 / _maxr, 4))

# =====================================================================================================================
# ETAPA 2 (v4): cráter irregular. Subdivisión local (1 nivel) + excavación con contorno poligonal.
# =====================================================================================================================
SKULL_CENTER = Vector((0.0, 0.05, 0.10))
CRATER_DIR = Vector((-0.80, 0.0, 0.60)).normalized()
R0 = 0.265; POLY_N = 9; POLY_VAR = (0.70, 1.35); DEPTH = 0.24; WALL = 0.16; STEP_DEPTH = 0.07; SUBDIV_RADIUS = 1.7
V4_SEED = 21
rng4 = random.Random(V4_SEED)

bm = bmesh.new(); bm.from_mesh(me)
bvh3 = BVHTree.FromBMesh(bm)                         # superficie ORIGINAL (sin cráter): también se usa para validar la etapa 3
hit, nrm, _, _ = bvh3.ray_cast(SKULL_CENTER, CRATER_DIR)
assert hit is not None, "el rayo no toca el cráneo"
n_c = nrm.normalized()
if n_c.dot(CRATER_DIR) < 0: n_c = -n_c
C0 = hit.copy()
e1 = n_c.cross(Vector((0, 0, 1))).normalized(); e2 = n_c.cross(e1).normalized()
angs = sorted(((i + rng4.uniform(-0.3, 0.3)) / POLY_N) * 2 * math.pi for i in range(POLY_N))
rads = [R0 * rng4.uniform(*POLY_VAR) for _ in range(POLY_N)]
def contour(theta):
    theta %= 2 * math.pi
    for i in range(POLY_N):
        a0, a1 = angs[i], angs[(i + 1) % POLY_N] + (2 * math.pi if i == POLY_N - 1 else 0)
        t = theta if theta >= a0 else theta + 2 * math.pi
        if a0 <= t <= a1:
            f = (t - a0) / (a1 - a0); return rads[i] * (1 - f) + rads[(i + 1) % POLY_N] * f
    return R0
bm3_copy = bmesh.new(); bm3_copy.from_mesh(me)       # copia de la superficie original (para validar que las estructuras no sobresalgan)
rsel = R0 * POLY_VAR[1] * SUBDIV_RADIUS
edges = [e for e in bm.edges if all((v.co - C0).length < rsel for v in e.verts)]
bmesh.ops.subdivide_edges(bm, edges=edges, cuts=1, use_grid_fill=True)
bm.to_mesh(me); me.update()
excavated = set()
for v in bm.verts:
    w = v.co - C0; wa = w.dot(n_c)
    if abs(wa) > 0.22: continue
    wt = w - n_c * wa; t = wt.length
    th = math.atan2(wt.dot(e2), wt.dot(e1))
    Rr = contour(th) * (1 + 0.07 * noise.noise(Vector((math.cos(th) * 4, math.sin(th) * 4, V4_SEED))))
    s = t / Rr
    if s >= 1.0: continue
    wall = sstep(s, 1.0, 1.0 - WALL)
    floor = DEPTH * (0.80 + 0.30 * noise.noise(C0 + wt * 4.0))
    floor += 0.10 * wt.dot(e1) / R0 * 0.5
    if noise.noise(C0 * 1.0 + wt * 2.2 + Vector((7, 7, 7))) > 0.05: floor += STEP_DEPTH
    d = max(0.0, floor) * wall
    v.co -= n_c * d
    if d > 1e-4: excavated.add(v.index)
bm.normal_update()
for f in bm.faces: f.smooth = True                   # todo suave, salvo aristas duras (>35°) en las paredes del cráter
for e in bm.edges:
    e.smooth = True
    if any(v.index in excavated for v in e.verts) and len(e.link_faces) == 2 and e.calc_face_angle(0) > math.radians(35):
        e.smooth = False
bm.to_mesh(me); bm.free(); me.update()
print("ETAPA 2 (v4): tris", tris(skull), "vértices excavados", len(excavated))

# =====================================================================================================================
# ETAPA 3 (v5): zona industrial dentro del cráter (objetos separados). Base local: e1 atrás, e2 (= n x e1) ABAJO, n exterior.
# =====================================================================================================================
DOCK_R_OUT, DOCK_R_IN, FLANGE_R = 0.10, 0.062, 0.145
DOCK_AB = (-0.03, -0.03)
BEAM = 0.058
STRUCT_COLOR = (0.30, 0.34, 0.40, 1); BONE_COLOR = (0.80, 0.75, 0.65, 1)
basis = Matrix(((e1.x, e2.x, n_c.x), (e1.y, e2.y, n_c.y), (e1.z, e2.z, n_c.z))).to_4x4()

def place(obj, a, b, depth, rot_n=0.0):
    obj.matrix_world = Matrix.Translation(C0 + e1 * a - e2 * b - n_c * depth) @ basis @ Matrix.Rotation(-rot_n, 4, 'Z')
    bpy.context.view_layer.objects.active = obj; obj.select_set(True)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True); obj.select_set(False)
    return obj
def box(name, size, a, b, depth, rot_n=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1); o = bpy.context.active_object; o.name = name; o.scale = size
    bpy.ops.object.transform_apply(scale=True)
    return place(o, a, b, depth, rot_n)
def cyl(name, r, length, a, b, depth, verts=12):
    bpy.ops.mesh.primitive_cylinder_add(vertices=verts, radius=r, depth=length); o = bpy.context.active_object; o.name = name
    return place(o, a, b, depth)
def boolean(target, cutter, op, solver='EXACT', hole_tolerant=False):
    bpy.context.view_layer.objects.active = target
    m = target.modifiers.new("b", 'BOOLEAN'); m.operation = op; m.object = cutter; m.solver = solver
    if hole_tolerant: m.use_hole_tolerant = True
    bpy.ops.object.modifier_apply(modifier=m.name); bpy.data.objects.remove(cutter, do_unlink=True)

structs = []
da, db = DOCK_AB
tube = cyl("dock_tube", DOCK_R_OUT, 0.30, da, db, 0.27, 12)
boolean(tube, cyl("dock_hole", DOCK_R_IN, 0.30, da, db, 0.265, 12), 'DIFFERENCE')
flange = cyl("dock_flange", FLANGE_R, 0.045, da, db, 0.145, 8)
boolean(flange, cyl("flange_hole", DOCK_R_IN, 0.2, da, db, 0.145, 12), 'DIFFERENCE')
structs += [tube, flange]
structs.append(box("beam_main", (0.46, BEAM, BEAM), 0.0, 0.15, 0.085))
structs.append(box("beam_diag", (0.42, BEAM, BEAM), 0.02, -0.14, 0.115, rot_n=R(38)))
structs.append(box("post_a", (0.065, 0.065, 0.36), 0.15, 0.15, 0.25))
structs.append(box("post_b", (0.065, 0.065, 0.36), -0.15, 0.15, 0.25))
for k, (b_, top) in enumerate(((-0.12, 0.215), (0.01, 0.17), (0.14, 0.125))):
    h = 0.45 - top
    structs.append(box(f"step{k}", (0.16, 0.13, h), 0.20, b_ - 0.0, top + h / 2))
for s in structs:
    bpy.context.view_layer.objects.active = s; s.select_set(True); bpy.ops.object.shade_flat(); s.select_set(False)
    s.color = STRUCT_COLOR
skull.color = BONE_COLOR
bvh_orig = BVHTree.FromBMesh(bm3_copy)
protr = 0
for s in structs:
    for v in s.data.vertices:
        d = v.co - SKULL_CENTER; h = bvh_orig.ray_cast(SKULL_CENTER, d.normalized())
        if h[0] is not None and d.length - h[3] > 0.01: protr += 1
print("ETAPA 3 (v5): estructuras", sum(tris(s) for s in structs), "tris | vértices que sobresalen de la superficie original:", protr)

# =====================================================================================================================
# ETAPA 4 (D): cara lisa. Dientes fuera, nariz eliminada y rellena, mandíbula cerrada y unida, suavizado.
# =====================================================================================================================
NOSE_C = (0.0, -0.15); NOSE_AX = (0.14, 0.24)
JAW_RAISE = 0.15; SMOOTH_ITERS = 30; SMOOTH_FACTOR = 0.5

def eye_guard(p):
    d = math.sqrt(((abs(p.x) - 0.17) / 0.17) ** 2 + ((p.z - 0.05) / 0.16) ** 2)
    return sstep(d, 1.15, 1.7) if p.y < -0.25 else 1.0
def eye_guard_n(p):
    d = math.sqrt(((abs(p.x) - 0.185) / 0.12) ** 2 + ((p.z - 0.05) / 0.17) ** 2)
    return sstep(d, 1.0, 1.35) if p.y < -0.25 else 1.0
def rr_del(p): return math.sqrt((p.x / 0.12) ** 2 + ((p.z + 0.17) / 0.20) ** 2)
def rr_nose(p): return math.sqrt(((p.x - NOSE_C[0]) / NOSE_AX[0]) ** 2 + ((p.z - NOSE_C[1]) / NOSE_AX[1]) ** 2)

def separate(obj, idx, name):
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='DESELECT'); bpy.ops.object.mode_set(mode='OBJECT')
    for i in idx: obj.data.vertices[i].select = True
    bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_mode(type='VERT')
    names = set(bpy.data.objects.keys())
    bpy.ops.mesh.separate(type='SELECTED'); bpy.ops.object.mode_set(mode='OBJECT')
    new = [bpy.data.objects[nm] for nm in bpy.data.objects.keys() if nm not in names][0]; new.name = name
    return new

# (a) los dientes (33 islas pequeñas) salen del cráneo y se DESCARTAN (la cara definitiva no lleva dentadura)
bm = bmesh.new(); bm.from_mesh(skull.data); bm.verts.ensure_lookup_table()
teeth_idx = [v.index for c in islands(bm) if len(c) < 60 for v in c]; bm.free()
teeth = separate(skull, teeth_idx, "teeth_tmp")
bm = bmesh.new(); bm.from_mesh(skull.data); bm.verts.ensure_lookup_table()
jaw_ids = [v.index for v in islands(bm)[1]]; bm.free()
jaw = separate(skull, jaw_ids, "jaw_tmp")
n_teeth_tris = tris(teeth); bpy.data.objects.remove(teeth, do_unlink=True)
cr = skull

# (b) nariz: se borran las caras de la bolsa nasal, se cierra el hueco y la zona se ajusta a una superficie suave (mínimos cuadrados)
bm = bmesh.new(); bm.from_mesh(cr.data); bm.verts.ensure_lookup_table()
pocket = [f for f in bm.faces if any(rr_del(v.co) < 1.0 and v.co.z < -0.01 and v.co.y > -0.52 for v in f.verts)]
bmesh.ops.delete(bm, geom=pocket, context='FACES')
bnd = [e for e in bm.edges if len(e.link_faces) == 1 and rr_del(e.verts[0].co) < 1.5 and e.verts[0].co.z < 0.0]
res = bmesh.ops.holes_fill(bm, edges=bnd, sides=256)
bmesh.ops.triangulate(bm, faces=res["faces"])
ring = [v.co for v in bm.verts if 1.0 < rr_nose(v.co) < 1.8 and v.co.y < -0.5 and v.co.z < 0.02 and eye_guard_n(v.co) == 1.0]
A = np.array([[1, c.x ** 2, c.z, c.z ** 2, c.x * c.z] for c in ring]); bvec = np.array([c.y for c in ring])
coef, *_ = np.linalg.lstsq(A, bvec, rcond=None)
yt = lambda x, z: coef[0] + coef[1] * x * x + coef[2] * z + coef[3] * z * z + coef[4] * x * z
for v in bm.verts:
    r = rr_nose(v.co)
    if r < 1.0:
        w = sstep(r, 1.0, 0.6) * eye_guard_n(v.co)
        v.co.y += w * (yt(v.co.x, v.co.z) - v.co.y)
bm.normal_update(); bm.to_mesh(cr.data); bm.free()

# (c) boca: el borde alveolar frontal de la mandíbula sube hasta solapar el maxilar; booleano de unión exacto
for v in jaw.data.vertices:
    p = v.co
    v.co.z += JAW_RAISE * sstep(p.z, -0.78, -0.60) * sstep(-p.y, 0.0, 0.30)
jaw.data.update()
t_before = tris(cr) + tris(jaw)
boolean(cr, jaw, 'UNION', hole_tolerant=True)
assert tris(cr) > 0.7 * t_before, "el booleano de unión colapsó la malla"

# (d) suavizado (Laplaciano con pesos) de la nariz y de la banda de la boca; las cuencas están protegidas
bm = bmesh.new(); bm.from_mesh(cr.data); bm.verts.ensure_lookup_table()
def zone_w(p):
    wn = sstep(rr_nose(p), 2.0, 1.3) if p.y < -0.3 else 0.0
    wm = (sstep(abs(p.x), 0.50, 0.40) * sstep(p.z, -0.26, -0.34) * sstep(p.z, -0.82, -0.72)) if p.y < -0.12 else 0.0
    return max(wn * eye_guard_n(p), wm * eye_guard(p))
W = {v.index: zone_w(v.co) for v in bm.verts}
for _ in range(SMOOTH_ITERS):
    new = {}
    for v in bm.verts:
        w = W[v.index]
        if w <= 0.0 or not v.link_edges: continue
        avg = sum((e.other_vert(v).co for e in v.link_edges), Vector()) / len(v.link_edges)
        new[v.index] = v.co + (avg - v.co) * (SMOOTH_FACTOR * w)
    for i, p in new.items(): bm.verts[i].co = p
for f in bm.faces: f.smooth = True
for e in bm.edges: e.smooth = True
bm.normal_update(); bm.to_mesh(cr.data); bm.free(); cr.data.update()
print("ETAPA 4 (D): tris cráneo", tris(cr), "(dientes descartados:", n_teeth_tris, "tris)")

# =====================================================================================================================
# ETAPA 5: corrección de MANDÍBULA. Solo vértices con z < Z0 (la punta inferior); nada por encima de Z0 se mueve.
#   - acortar: compresión vertical suave de lo que queda por debajo de Z0
#   - ensanchar la terminación: escala en X creciente hacia abajo
#   - borde inferior fracturado: cada vértice por debajo de una altura de corte IRREGULAR y ASIMÉTRICA se recorta a esa altura
# =====================================================================================================================
Z0 = -0.50               # por encima de esta altura no se toca NADA (cuencas, mejillas, nariz, pómulos, boca)
JAW_K = 0.72             # factor de compresión vertical de la punta (1 = sin cambio)
JAW_WIDEN = 0.55         # ensanchamiento relativo de la terminación (x1.55 en el extremo)
CUT_Z = -0.675           # altura media del borde inferior fracturado
CUT_NOISE = 0.045        # irregularidad del borde (ruido)
CUT_TILT = 0.085         # inclinación asimétrica (el lado +X queda más alto)
JAW_SEED = 17

def jaw_fix(p):
    d = Z0 - p.z
    if d <= 0.0: return p
    q = p.copy()
    q.x *= 1.0 + JAW_WIDEN * sstep(d, 0.0, 0.27)                         # ensanchar
    q.z += (1.0 - JAW_K) * d * sstep(d, 0.0, 0.12)                       # acortar (compresión suave: sin escalón en Z0)
    zc = CUT_Z + CUT_NOISE * noise.noise(Vector((q.x * 6.0, q.y * 6.0, JAW_SEED))) + CUT_TILT * q.x
    if q.x > 0.05 and q.y < -0.50: zc += 0.035                           # esquina frontal derecha astillada
    if q.z < zc:
        q.z = zc + 0.012 * noise.noise(Vector((q.x * 17.0, q.y * 17.0, JAW_SEED + 1)))   # recorte (casi plano, con rugosidad)
    return q

pre_fix = [v.co.copy() for v in cr.data.vertices]
if JAW_FIX:
    for v in cr.data.vertices: v.co = jaw_fix(v.co)
    cr.data.update()
    bm = bmesh.new(); bm.from_mesh(cr.data); bm.normal_update()
    for f in bm.faces: f.smooth = True
    bm.to_mesh(cr.data); bm.free(); cr.data.update()
moved = [i for i, p in enumerate(pre_fix) if (cr.data.vertices[i].co - p).length > 1e-9]
above = [i for i in moved if pre_fix[i].z >= Z0]
print("ETAPA 5 (mandíbula):", "ACTIVADA" if JAW_FIX else "OMITIDA", "| vértices movidos", len(moved), "| movidos con z>=Z0 (debe ser 0):", len(above),
      "| z mín antes %.3f después %.3f" % (min(p.z for p in pre_fix), min(v.co.z for v in cr.data.vertices)))
# orientación consistente de las caras (el booleano/relleno dejó alguna con el giro invertido en el interior de la base).
# Solo cambia el SENTIDO del giro; las posiciones no se tocan.
_bm = bmesh.new(); _bm.from_mesh(cr.data)
bmesh.ops.recalc_face_normals(_bm, faces=_bm.faces)
_bm.normal_update()
_flipped = [f for f in _bm.faces if (lambda nb: nb and all(f.normal.dot(g.normal) < -0.3 for g in nb))([g for e in f.edges for g in e.link_faces if g is not f])]
for f in _flipped: f.normal_flip()                   # caras cuya normal es opuesta a TODOS sus vecinos (junto a bordes abiertos)
print("caras invertidas corregidas:", len(_flipped))
for _f in _bm.faces: _f.smooth = True
_bm.normal_update(); _bm.to_mesh(cr.data); _bm.free(); cr.data.update()
cr.name = "knowhere_skull"

# =====================================================================================================================
# ETAPA 6: normalización uniforme (convención del proyecto: origen en el centro, radio englobante = 1). No cambia proporciones.
# Antes se exporta una copia SIN normalizar solo para los renders antes/después (misma escala que la variante D).
# =====================================================================================================================
def export_obj(path):
    bpy.ops.object.select_all(action='DESELECT')
    for o in bpy.data.objects:
        if o.type == 'MESH': o.select_set(True)
    bpy.context.view_layer.objects.active = cr
    bpy.ops.wm.obj_export(filepath=path, export_selected_objects=True, export_triangulated_mesh=True,
                          export_normals=True, export_uv=True, export_materials=False, forward_axis='NEGATIVE_Z', up_axis='Y')
PRENORM_OBJ = os.path.join(tempfile.gettempdir(), "knowhere_prenorm_tmp.obj")   # temporal, fuera del repositorio
export_obj(PRENORM_OBJ)
allv = [v.co for o in bpy.data.objects if o.type == 'MESH' for v in o.data.vertices]
lo = Vector((min(c.x for c in allv), min(c.y for c in allv), min(c.z for c in allv)))
hi = Vector((max(c.x for c in allv), max(c.y for c in allv), max(c.z for c in allv)))
NORM_OFFSET = (lo + hi) / 2
NORM_SCALE = 1.0 / max((c - NORM_OFFSET).length for c in allv)
for o in bpy.data.objects:
    if o.type == 'MESH':
        for v in o.data.vertices: v.co = (v.co - NORM_OFFSET) * NORM_SCALE
        o.data.update()
tx = lambda p: (p - NORM_OFFSET) * NORM_SCALE        # mismo cambio de coordenadas para cámaras de primer plano
print("ETAPA 6 (normalización): desplazamiento", [round(c, 4) for c in NORM_OFFSET], "escala", round(NORM_SCALE, 4))

# =====================================================================================================================
# EXPORTAR
# =====================================================================================================================
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(OUT, "knowhere.blend"))
export_obj(os.path.join(PLANETS_DIR, "knowhere.obj"))
print("TRIS TOTALES:", sum(tris(o) for o in bpy.data.objects if o.type == 'MESH'), "| piezas:", [o.name for o in bpy.data.objects if o.type == 'MESH'])

# =====================================================================================================================
# RENDERS de referencia (mismas cámaras e iluminación siempre) — Workbench
#   ref_*        modelo definitivo (normalizado)
#   jawfix_before_*  variante D (antes)      jawfix_after_*  mismo modelo SIN normalizar (después), para comparar a igual escala
# =====================================================================================================================
bpy.ops.wm.read_factory_settings(use_empty=True)
for f in os.listdir(RENDERS):
    if f.endswith(".png"): os.remove(os.path.join(RENDERS, f))      # solo PNG generados por este script
groups = {}
def load_obj(tag, path):
    pre = set(bpy.data.objects); bpy.ops.wm.obj_import(filepath=path)
    objs = [o for o in bpy.data.objects if o not in pre]
    for o in objs:
        bpy.ops.object.select_all(action='DESELECT'); o.select_set(True); bpy.context.view_layer.objects.active = o
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
        o.color = BONE_COLOR if o.name.startswith("knowhere_skull") else STRUCT_COLOR; o.name = f"{tag}_{o.name}"
    groups[tag] = objs
load_obj("ref", os.path.join(PLANETS_DIR, "knowhere.obj"))
load_obj("after", PRENORM_OBJ)
if COMPARE_OBJ and os.path.exists(COMPARE_OBJ): load_obj("before", COMPARE_OBJ)
scn = bpy.context.scene; scn.render.engine = 'BLENDER_WORKBENCH'; sh = scn.display.shading
sh.light = 'STUDIO'; sh.color_type = 'OBJECT'; sh.show_cavity = True
scn.world = bpy.data.worlds.new("w"); scn.world.color = (0.03, 0.03, 0.05)
cd = bpy.data.cameras.new("c"); cam = bpy.data.objects.new("c", cd); scn.collection.objects.link(cam); scn.camera = cam
Z = Vector((0, 0, 0))
def views(norm):
    f = tx if norm else (lambda p: p)
    return {"front": (Vector((0, -1, 0.05)).normalized() * 4.4, Z, 70),
            "profile_industrial": (Vector((-1, 0, 0.05)).normalized() * 4.4, Z, 70),
            "profile_opposite": (Vector((1, 0, 0.05)).normalized() * 4.4, Z, 70),
            "3q_industrial": (Vector((-0.65, -0.72, 0.28)).normalized() * 4.4, Z, 70),
            "3q_opposite": (Vector((0.65, -0.72, 0.28)).normalized() * 4.4, Z, 70),
            "cavity_closeup": (f(C0 + (n_c * 1.0 + e2 * 0.35 - e1 * 0.25).normalized() * 1.25), f(C0 - n_c * 0.15), 50),
            "chin_closeup": (f(Vector((0.3, -1.0, -0.2)).normalized() * 2.2 + Vector((0, 0, -0.3))), f(Vector((0, -0.5, -0.55))), 50)}
def shoot(tag, view, res, name, flat=False):
    for t, g in groups.items():
        for o in g: o.hide_render = (t != tag)
    p, target, lens = views(tag == "ref")[view]; cam.location = p; cd.lens = lens
    cam.rotation_euler = (target - p).to_track_quat('-Z', 'Y').to_euler()
    scn.render.resolution_x = scn.render.resolution_y = res
    if flat: sh.light = 'FLAT'; sh.color_type = 'SINGLE'; sh.single_color = (0, 0, 0); sh.show_cavity = False; scn.world.color = (1, 1, 1)
    else: sh.light = 'STUDIO'; sh.color_type = 'OBJECT'; sh.show_cavity = True; scn.world.color = (0.03, 0.03, 0.05)
    scn.render.filepath = os.path.join(RENDERS, name); bpy.ops.render.render(write_still=True)
for view in views(True): shoot("ref", view, 800, f"ref_{view}_800.png")
for view in ("3q_industrial", "front", "3q_opposite"): shoot("ref", view, 160, f"ref_{view}_160.png")
for tag, pre in (("after", "jawfix_after"), ("before", "jawfix_before")):
    if tag not in groups: continue
    for view in views(False): shoot(tag, view, 800, f"{pre}_{view}_800.png")
    for view in ("3q_industrial", "front", "3q_opposite"): shoot(tag, view, 160, f"{pre}_{view}_160.png")
    for view in ("front", "profile_opposite", "3q_opposite"): shoot(tag, view, 400, f"{pre}_{view}_400_sil.png", flat=True)
if os.path.exists(PRENORM_OBJ): os.remove(PRENORM_OBJ)       # archivo temporal propio
print("OK")

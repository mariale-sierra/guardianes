"""
SOVEREIGN — modelo definitivo (estructura planetaria dorada de esferas conectadas, Guardianes de la Galaxia Vol. 2).
Ejecutar:
    /Applications/Blender.app/Contents/MacOS/Blender --background --python build_sovereign.py
Salida: sovereign.blend (junto al script), ../../models/planets/sovereign.obj, renders/*.png y renders/report.txt (locales)

Ejes Blender: Z arriba, la cámara "frontal" mira desde -Y (la cara ornamentada del núcleo mira a -Y).
OBJ: Forward -Z, Up Y (Blender (x,y,z) -> OBJ (x,z,-y)); la cara frontal queda hacia +Z, igual que Knowhere/Orgoscope.
Determinista: no usa aleatoriedad (SEED existe solo por si se quiere variar; hoy no se usa).

Todo es paramétrico: edita la zona de PARÁMETROS y vuelve a ejecutar. Unidades pre-normalización (el núcleo mide 1.0);
al final todo se centra (caja envolvente) y se escala para que el radio englobante sea 1.
"""
import bpy, bmesh, math, os
from mathutils import Vector, Matrix

HERE = os.path.dirname(os.path.abspath(__file__))
# El OBJ definitivo va a assets/models/planets/ (carpeta del motor); el .blend queda junto al script.
# Para probar sin tocar el repositorio: PLANETS_OUT=<carpeta> (y en Knowhere KNOWHERE_OUT=<carpeta>).
PLANETS_DIR = os.path.abspath(os.environ.get("PLANETS_OUT", os.path.join(HERE, "..", "..", "models", "planets")))
os.makedirs(PLANETS_DIR, exist_ok=True)
RENDERS = os.path.join(HERE, "renders"); os.makedirs(RENDERS, exist_ok=True)
R = math.radians
SEED = 0   # reservado; el modelo no usa azar (mismas posiciones siempre)

# ============================================================ PARÁMETROS (zona editable) ============================
# --- Núcleo: la esfera dominante
CORE_RADIUS = 1.0
CORE_SEG, CORE_RINGS = 26, 16            # segmentos / anillos de la esfera UV (más = más triángulos)

# --- Esferas secundarias: centro (x,y,z) en unidades del núcleo, radio, resolución (seg, rings), color.
#     Dispuestas en 3D (no en fila): B arriba-derecha y adelante, C arriba, D derecha-atrás, E abajo-izquierda, F punta.
SPHERES = [
    dict(name="sphere_b_upper_right",  center=(1.42, 0.30, 0.30),  radius=0.66, seg=20, rings=12, mat="gold"),
    dict(name="sphere_c_top",          center=(0.50, 0.50, 1.05),  radius=0.36, seg=16, rings=10, mat="gold_light"),
    dict(name="sphere_d_right",        center=(2.05, 0.05, -0.10), radius=0.30, seg=16, rings=10, mat="amber"),
    dict(name="sphere_e_lower_left",   center=(-1.02, -0.30, -0.92), radius=0.58, seg=20, rings=12, mat="gold_light"),
    dict(name="sphere_f_far_lower_left", center=(-1.60, -0.50, -1.30), radius=0.22, seg=14, rings=8, mat="gold"),
]

# --- Conexiones: ("core" o nombre de esfera, ídem). Si las esferas se solapan -> COLLAR (toro sobre el círculo de
#     intersección). Si hay hueco -> CUELLO cilíndrico + 2 bridas (anillos) en los puntos de salida.
COLLAR_TUBE = 0.085          # grosor del tubo del collar
COLLAR_SEG, COLLAR_TSEG = 28, 5
NECK_RADIUS = 0.10           # radio del cilindro del cuello/puente
NECK_SEG = 10
FLANGE_TUBE = 0.055          # grosor de las bridas
FLANGE_SEG, FLANGE_TSEG = 14, 4
LINKS = [
    ("core", "sphere_b_upper_right"),
    ("core", "sphere_c_top"),
    ("core", "sphere_e_lower_left"),
    ("sphere_b_upper_right", "sphere_d_right"),
    ("sphere_b_upper_right", "sphere_c_top"),        # hueco -> puente cilíndrico
    ("sphere_e_lower_left", "sphere_f_far_lower_left"),  # hueco -> cuello cilíndrico
]

# --- Bandas circulares (toros "de latitud"): (esfera, eje, ángulo polar en grados, grosor del tubo, segmentos, tseg, material)
CORE_BAND_AXIS = (0.22, -0.12, 1.0)
BANDS = [
    ("core", CORE_BAND_AXIS, 90, 0.075, 32, 4, "gold_light"),   # ecuador inclinado
    ("core", CORE_BAND_AXIS, 56, 0.055, 28, 4, "dark"),         # banda superior
    ("sphere_b_upper_right", (0.1, -0.1, 1.0), 90, 0.055, 22, 4, "gold_light"),
    ("sphere_c_top", (0.3, 0.0, 1.0), 90, 0.045, 16, 4, "dark"),
    ("sphere_d_right", (0.0, 0.2, 1.0), 90, 0.040, 14, 4, "gold_light"),
]

# --- Ornamentos concéntricos (discos de anillos como en la referencia): (esfera, dirección sobre la esfera,
#     [ángulos angulares de cada anillo en grados], grosor del tubo, segmentos, tseg, material, disco_central_radio_ang)
ORNAMENTS = [
    ("core", (-0.35, -1.0, 0.15), [13, 24, 35], 0.040, 24, 4, "dark", 8),
    ("sphere_b_upper_right", (0.25, -1.0, 0.25), [15, 28], 0.034, 20, 4, "dark", 6),
    ("sphere_e_lower_left", (-0.65, -0.55, -0.55), [15, 28], 0.045, 20, 4, "dark", 8),   # "puerto" anillado como en la ref.
]
# --- Paneles hexagonales grandes en relieve: (esfera, dirección, radio angular en grados, altura, material)
PLATES = [
    ("core", (0.75, -0.65, 0.10), 11, 0.050, "gold_light"),
    ("core", (-0.20, -0.45, -0.88), 12, 0.050, "dark"),
    ("core", (0.40, -0.55, 0.73), 9, 0.045, "gold_light"),
    ("core", (-0.85, -0.45, 0.25), 10, 0.045, "amber"),
    ("sphere_b_upper_right", (0.7, -0.6, -0.3), 12, 0.040, "gold_light"),
    ("sphere_d_right", (0.5, -0.8, 0.2), 16, 0.035, "dark"),
    ("sphere_c_top", (0.0, -0.9, 0.3), 18, 0.030, "dark"),
]
HEX_SEG = 6

# --- Materiales de previsualización (viewport/Workbench). El OBJ NO lleva materiales.
MATS = {
    "gold":       (0.95, 0.68, 0.16),
    "gold_light": (1.00, 0.88, 0.45),
    "amber":      (0.78, 0.42, 0.08),
    "dark":       (0.16, 0.12, 0.10),
}


# ============================================================ UTILIDADES ============================================
def tris(o): return sum(len(p.vertices) - 2 for p in o.data.polygons)
def V3(t): return Vector(t)

def make_obj(name, bm, mat):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new(name); bm.to_mesh(me); bm.free()
    for pl in me.polygons: pl.use_smooth = True
    o = bpy.data.objects.new(name, me); bpy.context.scene.collection.objects.link(o)
    o.data.materials.append(MAT[mat])
    return o

def place(bm, center, axis):
    """Lleva la geometría construida con eje +Z al eje dado, trasladada a 'center'."""
    q = Vector((0, 0, 1)).rotation_difference(Vector(axis).normalized()).to_matrix().to_4x4()
    bmesh.ops.transform(bm, matrix=Matrix.Translation(center) @ q, verts=bm.verts)

def torus_bm(Rm, rt, seg, tseg):
    """Toro con eje +Z, con UV (u = vuelta mayor, v = vuelta del tubo)."""
    bm = bmesh.new(); uv = bm.loops.layers.uv.new("UVMap")
    vs = []
    for i in range(seg):
        a = 2 * math.pi * i / seg
        row = []
        for j in range(tseg):
            b = 2 * math.pi * j / tseg
            rho = Rm + rt * math.cos(b)
            row.append(bm.verts.new((rho * math.cos(a), rho * math.sin(a), rt * math.sin(b))))
        vs.append(row)
    for i in range(seg):
        for j in range(tseg):
            f = bm.faces.new((vs[i][j], vs[(i + 1) % seg][j], vs[(i + 1) % seg][(j + 1) % tseg], vs[i][(j + 1) % tseg]))
            for lp, (ui, vj) in zip(f.loops, ((i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1))):
                lp[uv].uv = (ui / seg, vj / tseg)
    return bm

def cyl_bm(r1, r2, depth, seg):
    """Cilindro/tronco de cono con eje +Z centrado en el origen, con UV."""
    bm = bmesh.new(); bm.loops.layers.uv.new("UVMap")
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r1, radius2=r2, depth=depth, calc_uvs=True)
    return bm

def sphere_bm(radius, seg, rings):
    bm = bmesh.new(); bm.loops.layers.uv.new("UVMap")
    bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=rings, radius=radius, calc_uvs=True)
    return bm


# ============================================================ ESCENA / MATERIALES ===================================
bpy.ops.wm.read_factory_settings(use_empty=True)
MAT = {}
for k, c in MATS.items():
    m = bpy.data.materials.new(k); m.diffuse_color = (*c, 1)
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = (*c, 1); bsdf.inputs["Metallic"].default_value = 0.85
        bsdf.inputs["Roughness"].default_value = 0.35
    MAT[k] = m

objs = []
# Registro de esferas: nombre -> (centro, radio)
SPH = {"core": (Vector((0, 0, 0)), CORE_RADIUS)}
for s in SPHERES: SPH[s["name"]] = (V3(s["center"]), s["radius"])

# ---- núcleo y esferas
core = make_obj("core", sphere_bm(CORE_RADIUS, CORE_SEG, CORE_RINGS), "gold"); objs.append(core)
for s in SPHERES:
    o = make_obj(s["name"], sphere_bm(s["radius"], s["seg"], s["rings"]), s["mat"])
    o.location = (0, 0, 0)
    me = o.data
    for v in me.vertices: v.co += V3(s["center"])         # geometría ya en posición final
    objs.append(o)

# ---- conexiones
def link_geometry(na, nb, idx):
    ca, ra = SPH[na]; cb, rb = SPH[nb]
    d = (cb - ca).length; ax = (cb - ca).normalized()
    out = []
    if d < ra + rb - 1e-6:
        # solapadas: collar sobre el círculo de intersección
        a = (d * d + ra * ra - rb * rb) / (2 * d)
        h = math.sqrt(max(ra * ra - a * a, 1e-6))
        bm = torus_bm(h, COLLAR_TUBE, COLLAR_SEG, COLLAR_TSEG); place(bm, ca + ax * a, ax)
        out.append(make_obj(f"collar_{idx}_{na}__{nb}", bm, "gold_light"))
    else:
        # con hueco: cuello cilíndrico entre los puntos de superficie + 2 bridas
        p0 = ca + ax * (ra - 0.04); p1 = cb - ax * (rb - 0.04)
        L = (p1 - p0).length
        bm = cyl_bm(NECK_RADIUS, NECK_RADIUS, L, NECK_SEG); place(bm, (p0 + p1) / 2, ax)
        out.append(make_obj(f"neck_{idx}_{na}__{nb}", bm, "amber"))
        for tag, c, r, sgn in (("a", ca, ra, 1), ("b", cb, rb, -1)):
            # reborde donde el cuello sale de la esfera: toro de radio ~ cuello*1.5, sobre la superficie
            rr = NECK_RADIUS * 1.5
            off = math.sqrt(max(r * r - rr * rr, 1e-6))
            bm = torus_bm(rr, FLANGE_TUBE, FLANGE_SEG, FLANGE_TSEG); place(bm, c + ax * sgn * off, ax)
            out.append(make_obj(f"flange_{idx}{tag}", bm, "gold_light"))
    return out

for i, (na, nb) in enumerate(LINKS): objs += link_geometry(na, nb, i)

# ---- bandas circulares
for i, (nm, axis, theta, tube, seg, tseg, mat) in enumerate(BANDS):
    c, r = SPH[nm]; ax = V3(axis).normalized(); th = R(theta)
    bm = torus_bm(r * math.sin(th), tube, seg, tseg); place(bm, c + ax * r * math.cos(th), ax)
    objs.append(make_obj(f"band_{i}_{nm}", bm, mat))

# ---- ornamentos concéntricos (anillos alrededor de una dirección + disco central)
for i, (nm, dirn, angs, tube, seg, tseg, mat, disc_deg) in enumerate(ORNAMENTS):
    c, r = SPH[nm]; n = V3(dirn).normalized()
    for k, a in enumerate(angs):
        bm = torus_bm(r * math.sin(R(a)), tube, seg, tseg); place(bm, c + n * r * math.cos(R(a)), n)
        objs.append(make_obj(f"ornament_{i}_ring{k}_{nm}", bm, mat))
    dr = r * math.sin(R(disc_deg)); h = 0.04
    bm = cyl_bm(dr, dr * 0.8, h, 12); place(bm, c + n * (r * math.cos(R(disc_deg)) + h / 2 - 0.01), n)
    objs.append(make_obj(f"ornament_{i}_disc_{nm}", bm, "gold_light"))

# ---- paneles hexagonales en relieve
for i, (nm, dirn, ang, h, mat) in enumerate(PLATES):
    c, r = SPH[nm]; n = V3(dirn).normalized(); pr = r * math.sin(R(ang))
    bm = cyl_bm(pr, pr * 0.82, h, HEX_SEG); place(bm, c + n * (r * math.cos(R(ang)) + h / 2 - 0.02), n)
    objs.append(make_obj(f"plate_{i}_{nm}", bm, mat))

# ============================================================ NORMALIZACIÓN =========================================
bpy.context.view_layer.update()
allv = [v.co.copy() for o in objs for v in o.data.vertices]
lo = Vector((min(c.x for c in allv), min(c.y for c in allv), min(c.z for c in allv)))
hi = Vector((max(c.x for c in allv), max(c.y for c in allv), max(c.z for c in allv)))
# Escala: la misma que antes (calculada desde el centro de la caja envolvente) -> tamaños y proporciones congelados.
BBOX_C = (lo + hi) / 2; SC = 1.0 / max((c - BBOX_C).length for c in allv)
# PIVOTE: el origen del OBJ es el CENTRO DEL NÚCLEO (el planeta rota sobre su eje por el núcleo). Es solo una traslación.
OFF = Vector(SPH["core"][0])
for o in objs:
    for v in o.data.vertices: v.co = (v.co - OFF) * SC
    o.data.update()
print("normalización: offset", [round(c, 4) for c in OFF], "escala", round(SC, 4))

# ============================================================ EXPORTAR ==============================================
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, "sovereign.blend"))
bpy.ops.object.select_all(action='DESELECT')
for o in objs: o.select_set(True)
bpy.context.view_layer.objects.active = objs[0]
bpy.ops.wm.obj_export(filepath=os.path.join(PLANETS_DIR, "sovereign.obj"), export_selected_objects=True, export_triangulated_mesh=True,
                      export_normals=True, export_uv=True, export_materials=False, forward_axis='NEGATIVE_Z', up_axis='Y')

rep = ["SOVEREIGN final — informe de construcción", ""]
total = 0
for o in objs:
    t = tris(o); total += t; rep.append(f"{o.name:44s} tris={t:5d} verts={len(o.data.vertices)}")
rep += ["", f"TOTAL triángulos: {total}", f"Escala aplicada: {SC:.5f}  (offset de centrado {[round(c,4) for c in OFF]})"]
cn = Vector((0, 0, 0)); cn = (cn - OFF) * SC
rep.append(f"Centro del núcleo tras normalizar (Blender): {[round(c,3) for c in cn]}  radio núcleo = {CORE_RADIUS*SC:.3f}")
open(os.path.join(RENDERS, "report.txt"), "w").write("\n".join(rep) + "\n")
print("\n".join(rep))

# ============================================================ RENDERS (Workbench) ===================================
for f in os.listdir(RENDERS):
    if f.startswith("sov_") and f.endswith(".png"): os.remove(os.path.join(RENDERS, f))
scn = bpy.context.scene; scn.render.engine = 'BLENDER_WORKBENCH'; sh = scn.display.shading
sh.light = 'STUDIO'; sh.color_type = 'MATERIAL'
scn.world = bpy.data.worlds.new("w"); scn.world.color = (0.03, 0.03, 0.05)
cd = bpy.data.cameras.new("c"); cam = bpy.data.objects.new("c", cd); scn.collection.objects.link(cam); scn.camera = cam
Z = Vector((0, 0, 0))

def shoot(name, pos, res, flat=False, lens=60):
    cam.location = pos
    cam.rotation_euler = (Z - pos).to_track_quat('-Z', 'Y').to_euler()
    cd.type = 'PERSP'; cd.lens = lens
    scn.render.resolution_x = scn.render.resolution_y = res
    if flat: sh.light = 'FLAT'; sh.color_type = 'SINGLE'; sh.single_color = (0, 0, 0); scn.world.color = (1, 1, 1)
    else: sh.light = 'STUDIO'; sh.color_type = 'MATERIAL'; scn.world.color = (0.03, 0.03, 0.05)
    sh.show_object_outline = False
    scn.render.filepath = os.path.join(RENDERS, name); bpy.ops.render.render(write_still=True)

D = 3.3
Vv = lambda x, y, z: Vector((x, y, z)).normalized() * D
shoot("sov_front_800.png", Vv(0, -1, 0.0001), 800)
shoot("sov_profile_left_800.png", Vv(-1, 0, 0.0001), 800)
shoot("sov_profile_right_800.png", Vv(1, 0, 0.0001), 800)
shoot("sov_3q_left_800.png", Vv(-0.65, -0.72, 0.3), 800)
shoot("sov_3q_right_800.png", Vv(0.65, -0.72, 0.3), 800)
shoot("sov_top_800.png", Vv(0, -0.0001, 1), 800)
shoot("sov_shaded_160.png", Vv(0.3, -0.9, 0.3), 160)
shoot("sov_front_sil_160.png", Vv(0.3, -0.9, 0.3), 160, flat=True)
print("OK")

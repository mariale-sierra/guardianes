"""
THE ORGOSCOPE — modelo DEFINITIVO (= v2 aprobada, geometría congelada; script autocontenido, no depende de otros archivos).
Núcleo irregular con cavidad ("ojo"), 4 anillos toroidales exactos, 2 masas orgánicas, 2 ejes de soporte.
Ejecutar:
    /Applications/Blender.app/Contents/MacOS/Blender --background --python build_orgoscope.py
Salida: orgoscope.blend (junto al script), ../../models/planets/orgoscope.obj, renders/*.png y renders/report.txt (locales)

Ejes Blender: Z arriba; el "ojo" del núcleo mira a -Y (hacia la cámara frontal de referencia).
OBJ: Forward -Z, Up Y  (el ojo queda en +Z del OBJ, igual que la cara de Knowhere).

TODO es paramétrico: cambia RINGS / CORE_* abajo y vuelve a ejecutar.
Cada anillo es un toro MATEMÁTICO (radio mayor R, radio del tubo r constantes): no hay ruido en los anillos,
así la circularidad y el grosor uniforme están garantizados (y se verifican numéricamente al final).
"""
import bpy, bmesh, math, os
from mathutils import Vector, Matrix, noise

HERE = os.path.dirname(os.path.abspath(__file__))
# El OBJ definitivo va a assets/models/planets/ (carpeta del motor); el .blend queda junto al script.
# Para probar sin tocar el repositorio: PLANETS_OUT=<carpeta> (y en Knowhere KNOWHERE_OUT=<carpeta>).
PLANETS_DIR = os.path.abspath(os.environ.get("PLANETS_OUT", os.path.join(HERE, "..", "..", "models", "planets")))
os.makedirs(PLANETS_DIR, exist_ok=True)
RENDERS = os.path.join(HERE, "renders"); os.makedirs(RENDERS, exist_ok=True)
R = math.radians

# ------------------------------------------------------------------ PARÁMETROS (zona editable) -----------------------
# Cada anillo: R = radio mayor, r = radio del tubo, major_axis_deg = ángulo (en el plano de la imagen frontal, medido
# desde +X hacia +Z) del eje alrededor del que se inclina el anillo, tilt_deg = inclinación respecto al plano de la
# imagen (0° = de frente, 90° = de canto). Todos comparten el centro (0,0,0). Estimado de la referencia (ver informe).
RINGS = [
    dict(name="ring_outer_grey",  R=0.93, r=0.050, major_axis_deg=18, tilt_deg=32, seg=48, tseg=8, color=(0.62, 0.62, 0.68)),
    dict(name="ring_pink",        R=0.79, r=0.065, major_axis_deg=35, tilt_deg=44, seg=48, tseg=8, color=(0.80, 0.30, 0.30)),
    dict(name="ring_inner_grey",  R=0.63, r=0.040, major_axis_deg=80, tilt_deg=58, seg=40, tseg=8, color=(0.62, 0.62, 0.68)),
    dict(name="ring_blue_rope",   R=0.47, r=0.048, major_axis_deg=75, tilt_deg=72, seg=36, tseg=8, color=(0.15, 0.30, 0.85)),
]
CORE_RADIUS = 0.36          # radio base del núcleo
CORE_FLATTEN_Y = 0.80       # aplastamiento a lo largo del eje de visión (Y): disco bulboso, no esfera perfecta
CORE_NOISE_AMP = 0.12       # amplitud de la deformación orgánica (baja frecuencia a propósito: render 480×320)
CORE_NOISE_SCALE = 1.4      # frecuencia del ruido (pequeña = formas grandes)
CORE_NOISE_SEED = 7.0
EYE_ANGLE_DEG = 30
EYE_RIM_BUMP = 0.12         # reborde elevado alrededor de la cavidad (define el borde a baja resolución)
# Protuberancias grandes y suaves del núcleo: (dirección, ancho angular en grados, amplitud relativa). Lejos del ojo (-Y).
CORE_LOBES = [((0.8, 0.3, 0.5), 32, 0.22), ((-0.6, 0.5, -0.6), 30, 0.18), ((0.2, 0.7, -0.7), 28, 0.15)]
# Masas orgánicas: se colocan automáticamente en los 2 puntos donde el anillo exterior y el rosa se acercan más.
MASS_SEMI_AXES = [(0.32, 0.17, 0.17), (0.25, 0.16, 0.21)]   # (tangente, radial, normal-al-anillo), antes de deformar
MASS_NOISE_AMP = 0.30
MASS_SEEDS = [3.0, 11.0]
MASS_COLORS = [(0.78, 0.33, 0.30), (0.74, 0.40, 0.34)]
MASS_MAX_RADIUS = 0.975     # (unidades pre-normalización; el anillo exterior llega a 0.98) para no salirse de la esfera unidad          # semiángulo de la cavidad circular ("ojo") en la cara frontal
EYE_DEPTH = 0.52            # fracción del radio que se hunde la cavidad
STRUT_R = 0.045             # radio de los 2 ejes de soporte
STRUT_SEG = 8


def sstep(x, e0, e1):
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0))); return t * t * (3 - 2 * t)

def tris(o): return sum(len(p.vertices) - 2 for p in o.data.polygons)

def ring_matrix(p):
    """Rotación del anillo: eje de inclinación en el plano XZ (Ry), luego inclinación (Rx). El toro nace con normal +Y."""
    return Matrix.Rotation(R(-p["major_axis_deg"]), 3, 'Y') @ Matrix.Rotation(R(p["tilt_deg"]), 3, 'X')


def make_obj(name, bm, color, smooth=True):
    me = bpy.data.meshes.new(name); bm.to_mesh(me); bm.free()
    o = bpy.data.objects.new(name, me); bpy.context.scene.collection.objects.link(o)
    if smooth:
        for pl in me.polygons: pl.use_smooth = True
    o.color = (*color, 1)
    return o


def torus_bm(Rm, rt, seg, tseg, M):
    """Toro con UV (u = vuelta mayor, v = vuelta del tubo). Vértices en el plano XZ (normal +Y) y luego rotados por M."""
    bm = bmesh.new(); uv = bm.loops.layers.uv.new("UVMap")
    vs = []
    for i in range(seg):
        a = 2 * math.pi * i / seg; ca, sa = math.cos(a), math.sin(a)
        row = []
        for j in range(tseg):
            b = 2 * math.pi * j / tseg
            rho = Rm + rt * math.cos(b)
            row.append(bm.verts.new(M @ Vector((rho * ca, rt * math.sin(b), rho * sa))))
        vs.append(row)
    for i in range(seg):
        for j in range(tseg):
            i2, j2 = (i + 1) % seg, (j + 1) % tseg
            f = bm.faces.new((vs[i][j], vs[i2][j], vs[i2][j2], vs[i][j2]))
            us = (i / seg, (i + 1) / seg, (i + 1) / seg, i / seg)
            vv = (j / tseg, j / tseg, (j + 1) / tseg, (j + 1) / tseg)
            for l, u, v in zip(f.loops, us, vv): l[uv].uv = (u, v)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


bpy.ops.wm.read_factory_settings(use_empty=True)
objs = []

# ------------------------------------------------------------------ ANILLOS ---------------------------------------------
ring_info = []
for p in RINGS:
    M = ring_matrix(p)
    o = make_obj(p["name"], torus_bm(p["R"], p["r"], p["seg"], p["tseg"], M), p["color"])
    objs.append(o); ring_info.append((p, M))

# ------------------------------------------------------------------ NÚCLEO ---------------------------------------------
bm = bmesh.new(); bm.loops.layers.uv.new("UVMap")   # sin capa UV previa, calc_uvs no genera nada
bmesh.ops.create_uvsphere(bm, u_segments=32, v_segments=18, radius=1.0, calc_uvs=True)
# create_uvsphere tiene el eje polar en Z; giramos para que el polo quede en X (los polos quedan a un lado, la cara -Y limpia)
bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=Matrix.Rotation(R(90), 3, 'Y'))
cos_out, cos_in = math.cos(R(EYE_ANGLE_DEG)), math.cos(R(EYE_ANGLE_DEG * 0.55))
for v in bm.verts:
    d = v.co.normalized()
    n = noise.noise(d * CORE_NOISE_SCALE + Vector((CORE_NOISE_SEED, 0, 0)))
    rad = CORE_RADIUS * (1 + CORE_NOISE_AMP * n)
    for ld, wd, amp in CORE_LOBES:                                   # protuberancias gaussianas grandes
        ang = math.degrees(d.angle(Vector(ld).normalized())); rad *= 1 + amp * math.exp(-(ang / wd) ** 2)
    ang_eye = math.degrees(d.angle(Vector((0, -1, 0))))
    w = sstep(d.dot(Vector((0, -1, 0))), cos_out, cos_in)          # 0 fuera del ojo, 1 en el centro del ojo
    rad *= 1 + EYE_RIM_BUMP * math.exp(-((ang_eye - EYE_ANGLE_DEG) / 5.0) ** 2)   # reborde
    rad *= 1 - EYE_DEPTH * w
    p = d * rad; p.y *= CORE_FLATTEN_Y
    v.co = p
core_bm_verts = [v.co.copy() for v in bm.verts]
bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
core = make_obj("core", bm, (0.85, 0.42, 0.40)); objs.append(core)

# Bisel del ojo: toro pequeño en el borde de la cavidad (posición medida sobre la malla, no a ojo)
rim = [c for c in core_bm_verts if abs(math.degrees(math.acos(max(-1, min(1, c.normalized().dot(Vector((0, -1, 0))))))) - EYE_ANGLE_DEG) < 5]
rim_y = sum(c.y for c in rim) / len(rim); rim_r = sum(math.hypot(c.x, c.z) for c in rim) / len(rim)
bezel = make_obj("eye_bezel", torus_bm(rim_r, 0.026, 32, 6, Matrix.Translation((0, rim_y, 0)).to_3x3()), (0.55, 0.55, 0.6))
bezel.location = (0, rim_y * 0 , 0)
# (el toro ya está en el plano XZ con normal Y; solo hay que desplazarlo en Y)
bezel.location = (0, rim_y, 0); objs.append(bezel)

# ------------------------------------------------------------------ EJES DE SOPORTE (2) ---------------------------------
# Eje diametral a lo largo del eje de inclinación del anillo exterior: nace en el núcleo y termina en el anillo exterior.
p0, M0 = ring_info[0]
axis = M0 @ Vector((1, 0, 0))                       # dirección de un punto del anillo exterior (a=0)
for sgn, nm in ((1, "strut_a"), (-1, "strut_b")):
    start = axis * sgn * CORE_RADIUS * 0.6
    end = axis * sgn * p0["R"]
    d = end - start; L = d.length
    bm = bmesh.new(); bm.loops.layers.uv.new("UVMap")   # sin capa UV previa, calc_uvs no genera nada
    bmesh.ops.create_cone(bm, cap_ends=True, segments=STRUT_SEG, radius1=STRUT_R, radius2=STRUT_R, depth=L, calc_uvs=True)
    q = Vector((0, 0, 1)).rotation_difference(d.normalized()).to_matrix()
    bmesh.ops.transform(bm, matrix=Matrix.Translation(start + d / 2) @ q.to_4x4(), verts=bm.verts)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    objs.append(make_obj(nm, bm, (0.85, 0.75, 0.8)))


# ------------------------------------------------------------------ MASAS ORGÁNICAS (2) -----------------------------------
# Son objetos APARTE: no tocan los vértices de los anillos. Se centran donde el anillo exterior y el rosa quedan más cerca.
def ring_pt(p, M, a): return M @ Vector((p["R"] * math.cos(a), 0, p["R"] * math.sin(a)))
po, Mo = ring_info[0]; pp, Mp = ring_info[1]
NS = 720
dmin = []                                           # distancia mínima entre las dos curvas centrales, por ángulo del exterior
for i in range(NS):
    a = 2 * math.pi * i / NS; q = ring_pt(po, Mo, a)
    best = min(((q - ring_pt(pp, Mp, 2 * math.pi * k / NS)).length, k) for k in range(NS))
    dmin.append((best[0], i, best[1]))
cands = [dmin[i] for i in range(NS) if dmin[i][0] <= dmin[i - 1][0] and dmin[i][0] < dmin[(i + 1) % NS][0]]
cands.sort()
picked = []
for c in cands:
    if all(abs(((c[1] - q[1] + NS / 2) % NS) - NS / 2) > NS / 6 for q in picked): picked.append(c)
    if len(picked) == 2: break
print("masas en ángulos del anillo exterior (grados):", [c[1] * 360 / NS for c in picked], "distancias", [round(c[0], 3) for c in picked])
for mi, (dist, ia, ib) in enumerate(picked):
    a = 2 * math.pi * ia / NS
    P1 = ring_pt(po, Mo, a); P2 = ring_pt(pp, Mp, 2 * math.pi * ib / NS)
    ctr = (P1 + P2) / 2
    rad = ctr.normalized()                                           # dirección radial desde el centro común
    tan = Mo @ Vector((-math.sin(a), 0, math.cos(a)))                # tangente del anillo exterior en ese punto
    tan = (tan - rad * tan.dot(rad)).normalized(); nor = rad.cross(tan).normalized()
    ax = MASS_SEMI_AXES[mi]
    bm = bmesh.new(); bm.loops.layers.uv.new("UVMap"); bmesh.ops.create_icosphere(bm, subdivisions=4, radius=1.0, calc_uvs=True)
    lobes = [Vector((0.6, 0.2, 0.5)).normalized(), Vector((-0.7, -0.3, 0.4)).normalized(), Vector((0.1, 0.8, -0.5)).normalized()]
    for v in bm.verts:
        d = v.co.normalized()
        n = noise.noise(d * 1.6 + Vector((MASS_SEEDS[mi], 3.0, 1.0)))
        f = 1 + MASS_NOISE_AMP * n
        for k, ld in enumerate(lobes): f *= 1 + (0.25 - 0.1 * k) * math.exp(-(math.degrees(d.angle(ld)) / 30) ** 2)
        d = d * f
        v.co = ctr + tan * (d.x * ax[0]) + rad * (d.y * ax[1]) + nor * (d.z * ax[2])
    # si se sale de la esfera unidad, se desplaza hacia el centro (sin escalar)
    for _ in range(60):
        mx = max(v.co.length for v in bm.verts)
        if mx <= MASS_MAX_RADIUS: break
        bmesh.ops.translate(bm, verts=bm.verts, vec=-rad * (mx - MASS_MAX_RADIUS + 0.002))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    objs.append(make_obj(f"mass_{'ab'[mi]}", bm, MASS_COLORS[mi]))

# ------------------------------------------------------------------ NORMALIZACIÓN (radio englobante = 1, centrado) -------
bpy.context.view_layer.update()   # matrix_world debe reflejar la ubicación del bisel antes de hornearla
allv = [v.co for o in objs[:len(RINGS)] for v in o.data.vertices]   # solo anillos: misma escala que v1
lo = Vector((min(c.x for c in allv), min(c.y for c in allv), min(c.z for c in allv)))
hi = Vector((max(c.x for c in allv), max(c.y for c in allv), max(c.z for c in allv)))
OFF = (lo + hi) / 2; SC = 1.0 / max((c - OFF).length for c in allv)
for o in objs:
    o.location = (0, 0, 0)  # las ubicaciones ya están horneadas salvo el bisel
for o in objs:
    mw = o.matrix_world.copy()
    for v in o.data.vertices: v.co = (mw @ v.co - OFF) * SC
    o.location = (0, 0, 0); o.data.update()
print("normalización: offset", [round(c, 4) for c in OFF], "escala", round(SC, 4))
for o in objs: o.data.update()
ring_info = [(dict(p, R=p["R"] * SC, r=p["r"] * SC), M) for p, M in ring_info]

# ------------------------------------------------------------------ EXPORTAR -------------------------------------------
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, "orgoscope.blend"))
bpy.ops.object.select_all(action='DESELECT')
for o in objs: o.select_set(True)
bpy.context.view_layer.objects.active = objs[0]
bpy.ops.wm.obj_export(filepath=os.path.join(PLANETS_DIR, "orgoscope.obj"), export_selected_objects=True, export_triangulated_mesh=True,
                      export_normals=True, export_uv=True, export_materials=False, forward_axis='NEGATIVE_Z', up_axis='Y')

# ------------------------------------------------------------------ VERIFICACIÓN NUMÉRICA DE LOS ANILLOS ----------------
rep = ["THE ORGOSCOPE final — informe", ""]
total = 0
for o in objs:
    t = tris(o); total += t; rep.append(f"{o.name:18s} tris={t:5d} verts={len(o.data.vertices)}")
rep += ["", f"TOTAL triángulos: {total}", "", "Verificación de anillos (frame local del anillo; ideal: dev=0):"]
for (p, M), o in zip(ring_info, objs[:len(ring_info)]):
    Mi = M.inverted(); devs_tube = []; devs_center = []
    for v in o.data.vertices:
        l = Mi @ v.co                                  # coordenadas en el plano propio (normal +Y)
        rho = math.hypot(l.x, l.z)
        dist_tube = math.hypot(rho - p["R"], l.y)      # distancia al círculo central ideal = radio del tubo
        devs_tube.append(abs(dist_tube - p["r"]))
    normal = M @ Vector((0, 1, 0))
    rep.append(f"{p['name']:18s} R={p['R']:.4f} r={p['r']:.4f} normal=({normal.x:+.3f},{normal.y:+.3f},{normal.z:+.3f}) "
               f"max|dist-r|={max(devs_tube):.2e}")
open(os.path.join(RENDERS, "report.txt"), "w").write("\n".join(rep) + "\n")
print("\n".join(rep))

# ------------------------------------------------------------------ RENDERS (Workbench) ----------------------------------
for f in os.listdir(RENDERS):
    if f.startswith("final_") and f.endswith(".png"): os.remove(os.path.join(RENDERS, f))   # solo los de esta versión
scn = bpy.context.scene; scn.render.engine = 'BLENDER_WORKBENCH'; sh = scn.display.shading
sh.light = 'STUDIO'; sh.color_type = 'OBJECT'
scn.world = bpy.data.worlds.new("w"); scn.world.color = (0.03, 0.03, 0.05)
cd = bpy.data.cameras.new("c"); cam = bpy.data.objects.new("c", cd); scn.collection.objects.link(cam); scn.camera = cam
Z = Vector((0, 0, 0))

def shoot(name, pos, res, up=Vector((0, 0, 1)), ortho=False, flat=False, wire=False, lens=70):
    cam.location = pos
    cam.rotation_euler = (Z - pos).to_track_quat('-Z', 'Y').to_euler() if up == Vector((0, 0, 1)) else \
        (Z - pos).to_track_quat('-Z', 'Z').to_euler()
    cd.type = 'ORTHO' if ortho else 'PERSP'; cd.ortho_scale = 2.3; cd.lens = lens
    scn.render.resolution_x = scn.render.resolution_y = res
    if flat: sh.light = 'FLAT'; sh.color_type = 'SINGLE'; sh.single_color = (0, 0, 0); scn.world.color = (1, 1, 1)
    else: sh.light = 'STUDIO'; sh.color_type = 'OBJECT'; scn.world.color = (0.03, 0.03, 0.05)
    sh.show_object_outline = False
    scn.render.filepath = os.path.join(RENDERS, name); bpy.ops.render.render(write_still=True)

D = 4.6
V = lambda x, y, z: Vector((x, y, z)).normalized() * D
shoot("final_front_800.png", V(0, -1, 0.0001), 800)
shoot("final_profile_800.png", V(1, 0, 0.0001), 800)
shoot("final_3q_left_800.png", V(-0.65, -0.72, 0.3), 800)
shoot("final_3q_right_800.png", V(0.65, -0.72, 0.3), 800)
shoot("final_top_800.png", V(0, -0.0001, 1), 800, up=Vector((0, 1, 0)))
shoot("final_shaded_160.png", V(-0.65, -0.72, 0.3), 160)
shoot("final_front_160.png", V(0, -1, 0.0001), 160)
shoot("final_front_sil_160.png", V(0, -1, 0.0001), 160, flat=True)
# vistas técnicas: cámara ortográfica a lo largo de la normal de cada anillo → el anillo debe verse como círculo perfecto
for (p, M) in ring_info:
    nrm = M @ Vector((0, 1, 0))
    shoot(f"final_check_{p['name']}_ortho_600.png", nrm * D, 600, ortho=True, up=Vector((0, 1, 0)) if abs(nrm.z) > 0.9 else Vector((0, 0, 1)))
shoot("final_front_ortho_600.png", V(0, -1, 0.0001), 600, ortho=True)
# vista de canto del anillo exterior: grosor visible a ambos lados (ortográfica a lo largo de su eje de inclinación)
shoot("final_edgeon_outer_ortho_600.png", (ring_info[0][1] @ Vector((1, 0, 0))) * D, 600, ortho=True)
print("OK")

"""Shared toolkit for the procedural model generators in `tools/models/`.

Every asset is built from code (lofted tubes, ribbons, bevelled boxes), given
procedural Cycles materials, then either baked to plain image textures or
given flat per-facet colours (the low-poly look), optionally skinned to a
joint list in the game's own coordinates, and exported as glTF.

Works headless (`blender -b --python tools/models/<asset>.py`) and inside a
live Blender session (the Blender MCP runs the same file): each asset lives
in its own collection, which a rebuild clears and refills.

Axes: Blender Z up. The glTF exporter maps Blender (x, y, z) to Bevy
(x, z, -y), so Blender +Y is Bevy's forward -Z.
"""

import math
import os
import random

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector, noise

TAU = math.tau
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FRONT = Vector((0, 1, 0))  # Bevy forward (-Z)
UP = Vector((0, 0, 1))


# ---------------------------------------------------------------- maths
def V(x, y, z):
    return Vector((x, y, z))


def clamp(x, a=0.0, b=1.0):
    return a if x < a else b if x > b else x


def sstep(a, b, x):
    t = clamp((x - a) / (b - a))
    return t * t * (3 - 2 * t)


def lerp(a, b, t):
    return a + (b - a) * t


def gauss(x, w):
    return math.exp(-((x / w) ** 2))


def adiff(a, b):
    return (a - b + math.pi) % TAU - math.pi


def tab(x, xs, ys):
    """Piecewise-smooth lookup through (xs, ys)."""
    if x <= xs[0]:
        return ys[0]
    for i in range(1, len(xs)):
        if x <= xs[i]:
            t = (x - xs[i - 1]) / (xs[i] - xs[i - 1])
            return lerp(ys[i - 1], ys[i], 0.5 * t + 0.5 * sstep(0.0, 1.0, t))
    return ys[-1]


def nz(p, s=1.0, o=0.0):
    return noise.noise(Vector(p) * s + Vector((o, o * 0.7, o * 1.3)))


def fbm(p, s=1.0, octaves=3, o=0.0):
    tot, amp, f = 0.0, 1.0, s
    for _ in range(octaves):
        tot += amp * nz(p, f, o)
        amp *= 0.5
        f *= 2.03
    return tot


def ellipse(th, a_n, b_b):
    c, s = math.cos(th), math.sin(th)
    return 1.0 / math.sqrt((c / a_n) ** 2 + (s / b_b) ** 2)


def superellipse(th, a_n, b_b, p=4.0):
    """A rounded box cross-section (p=2 is an ellipse, larger is boxier)."""
    c, s = abs(math.cos(th)), abs(math.sin(th))
    return (((c / a_n) ** p) + ((s / b_b) ** p)) ** (-1.0 / p)


# ---------------------------------------------------------------- paths and lofts
def catmull(ctrl, n):
    c = [Vector(p) for p in ctrl]
    out = []
    for k in range(n):
        u = k / (n - 1) * (len(c) - 1)
        i = min(int(u), len(c) - 2)
        t = u - i
        p0, p1, p2, p3 = c[max(i - 1, 0)], c[i], c[i + 1], c[min(i + 2, len(c) - 1)]
        out.append(0.5 * (2 * p1 + (p2 - p0) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t * t
                          + (3 * p1 - p0 - 3 * p2 + p3) * t * t * t))
    return out


def resample(ctrl, n):
    dense = catmull(ctrl, max(n * 8, 16))
    acc = [0.0]
    for a, b in zip(dense, dense[1:]):
        acc.append(acc[-1] + (b - a).length)
    total = acc[-1]
    out, k = [], 0
    for i in range(n):
        s = total * i / (n - 1)
        while k < len(acc) - 2 and acc[k + 1] < s:
            k += 1
        t = (s - acc[k]) / max(acc[k + 1] - acc[k], 1e-9)
        out.append(dense[k].lerp(dense[k + 1], t))
    return out, total


def frames(pts, ref, cyclic=False, transport=True):
    n = len(pts)
    tans = []
    for i in range(n):
        if cyclic:
            a, b = pts[(i - 1) % (n - 1)], pts[(i + 1) % (n - 1)]
        else:
            a, b = pts[max(i - 1, 0)], pts[min(i + 1, n - 1)]
        tans.append((b - a).normalized())
    out = []
    nrm = ref - ref.project(tans[0])
    if nrm.length < 1e-5:
        nrm = tans[0].orthogonal()
    nrm.normalize()
    for t in tans:
        base = nrm if transport else ref
        nrm = base - base.project(t)
        if nrm.length < 1e-5:
            nrm = t.orthogonal()
        nrm.normalize()
        out.append((t, nrm, t.cross(nrm)))
    return out


class Ctx:
    """What a radius/displacement callback knows about a vertex."""

    __slots__ = ("i", "k", "t", "th", "p", "d", "q", "z", "T", "N", "B")


def grid(rows, uvs, closed=False, keep=None):
    bm = bmesh.new()
    vs = [[bm.verts.new(p) for p in row] for row in rows]
    uvl = bm.loops.layers.uv.new("Proc")
    coll = bm.faces.layers.int.new("col")
    n = len(rows[0])
    cols = n if closed else n - 1
    for i in range(len(rows) - 1):
        for j in range(cols):
            j2 = (j + 1) % n
            quad = (vs[i][j], vs[i][j2], vs[i + 1][j2], vs[i + 1][j])
            if keep is not None:
                c = (rows[i][j] + rows[i][j2] + rows[i + 1][j2] + rows[i + 1][j]) / 4
                if not keep(i, j, c):
                    continue
            try:
                f = bm.faces.new(quad)
            except ValueError:
                continue
            f[coll] = j
            for loop, (a, b) in zip(f.loops, ((i, j), (i, j + 1), (i + 1, j + 1), (i + 1, j))):
                loop[uvl].uv = uvs[a][b]
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    return bm


def tube(ctrl, n, sides, radius, ref=UP, span=None, closed=True, keep=None, disp=None, cap=False, loop=False,
         ru=0.08, raw=False, transport=True):
    """Loft a cross-section along a smooth path through `ctrl`.

    `radius(c)`/`disp(c)` get a `Ctx`. Angle 0 points along the frame normal
    (the projection of `ref`), increasing toward tangent x normal.
    """
    if raw:
        pts = [Vector(p) for p in ctrl]
        total = sum((b - a).length for a, b in zip(pts, pts[1:]))
    else:
        pts, total = resample(ctrl, n)
    fr = frames(pts, ref, cyclic=loop, transport=transport)
    rows, uvs = [], []
    c = Ctx()
    acc = 0.0
    for i, (p, (T, N, B)) in enumerate(zip(pts, fr)):
        if i:
            acc += (p - pts[i - 1]).length
        t = acc / total if total else 0.0
        a0, a1 = span(t) if span else (0.0, TAU)
        row, uvr = [], []
        ncol = sides if closed else sides + 1
        for k in range(sides + 1):
            th = a0 + (a1 - a0) * k / sides
            uvr.append((th * ru, acc))
            if k >= ncol:
                continue
            c.i, c.k, c.t, c.th, c.p, c.z, c.T, c.N, c.B = i, k, t, th, p, p.z, T, N, B
            c.d = N * math.cos(th) + B * math.sin(th)
            c.q = p + c.d * 0.05
            r = radius(c) if callable(radius) else radius
            q = p + c.d * r
            if disp is not None:
                c.q = q
                q = q + disp(c)
            row.append(q)
        rows.append(row)
        uvs.append(uvr)
    bm = grid(rows, uvs, closed=closed, keep=keep)
    if loop:
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-6)
    if cap:
        bmesh.ops.holes_fill(bm, edges=[e for e in bm.edges if e.is_boundary], sides=0)
    return bm


def ribbon(ctrl, width, n, twist=0.0, taper=0.7, seed=0.0, ref=UP):
    pts, total = resample(ctrl, n)
    fr = frames(pts, ref)
    rows, uvs = [], []
    acc = 0.0
    for i, (p, (T, N, B)) in enumerate(zip(pts, fr)):
        if i:
            acc += (p - pts[i - 1]).length
        t = acc / total
        ang = twist * t + 0.6 * nz((seed, t * 2.0, 0.3))
        side = B * math.cos(ang) + N * math.sin(ang)
        w = width * (1.0 - taper * sstep(0.55, 1.0, t))
        rows.append([p - side * w / 2, p, p + side * w / 2])
        uvs.append([(0.0, acc), (width / 2, acc), (width, acc)])
    return grid(rows, uvs)


def box(center, size, bevel=0.0, segments=2, rot=None):
    """An axis box (optionally rotated), bevelled in bmesh, with box-projected UVs in metres."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=Vector(size), verts=bm.verts)
    if bevel > 0:
        bmesh.ops.bevel(bm, geom=bm.edges[:] + bm.verts[:], offset=min(bevel, min(size) * 0.49), segments=segments,
                        affect="EDGES", profile=0.5)
    if rot is not None:
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=rot)
    bmesh.ops.translate(bm, vec=Vector(center), verts=bm.verts)
    box_uv(bm)
    return bm


def box_uv(bm, scale=1.0):
    """Tri-planar-ish metre UVs on the "Proc" layer (per face, by dominant normal)."""
    uvl = bm.loops.layers.uv.get("Proc") or bm.loops.layers.uv.new("Proc")
    for f in bm.faces:
        n = f.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        for lp in f.loops:
            co = lp.vert.co
            if ax == 0:
                lp[uvl].uv = (co.y * scale, co.z * scale)
            elif ax == 1:
                lp[uvl].uv = (co.x * scale, co.z * scale)
            else:
                lp[uvl].uv = (co.x * scale, co.y * scale)
    return bm


def cylinder(a, b, r, sides=16, cap=True, r2=None):
    return tube([a, b], 2, sides, lambda c: lerp(r, r if r2 is None else r2, c.t), ref=UP if abs(
        (Vector(b) - Vector(a)).normalized().dot(UP)) < 0.9 else FRONT, raw=True, cap=cap, ru=r)


def transform(bm, m):
    bmesh.ops.transform(bm, matrix=m, verts=bm.verts)
    return bm


def merge(*bms):
    """Merge bmeshes into one (keeps the Proc UV layer)."""
    out = bmesh.new()
    me = bpy.data.meshes.new("_tmp")
    for b in bms:
        b.to_mesh(me)
        out.from_mesh(me)
        b.free()
    bpy.data.meshes.remove(me)
    return out


# ---------------------------------------------------------------- detail patterns
def _prof(t, width):
    return np.sqrt(np.clip(1.0 - ((t - 0.5) / (0.5 * width)) ** 2, 0.0, 1.0))


def pat_weave(n, threads, gap, slub, seed):
    rng = np.random.default_rng(seed)
    c = (np.arange(n) + 0.5) / n * threads
    X, Y = np.meshgrid(c, c)
    ix, iy = np.floor(X).astype(int) % threads, np.floor(Y).astype(int) % threads
    tx, ty = X % 1.0, Y % 1.0
    ww = rng.uniform(1 - slub, 1 + slub, threads)
    wf = rng.uniform(1 - slub, 1 + slub, threads)
    warp = _prof(tx, np.clip((1 - gap) * ww[ix], 0.1, 1.0))
    weft = _prof(ty, np.clip((1 - gap) * wf[iy], 0.1, 1.0))
    over = (ix + iy) % 2 == 0
    arc_w = 0.55 + 0.45 * np.sin(np.pi * ty)
    arc_f = 0.55 + 0.45 * np.sin(np.pi * tx)
    h = np.where(over, np.maximum(warp * arc_w, weft * 0.4), np.maximum(weft * arc_f, warp * 0.4))
    tone = np.where(over, rng.uniform(0.7, 1.3, threads)[ix], rng.uniform(0.7, 1.3, threads)[iy]) * (0.45 + 0.55 * h)
    return h, tone / tone.max()


def pat_rope(n, seed):
    rng = np.random.default_rng(seed)
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    phase = (U + Vv) * 3.0
    h = _prof(phase % 1.0, 0.95) * (0.8 + 0.2 * _prof((U * 24 + Vv * 60) % 1.0, 0.8))
    tone = rng.uniform(0.8, 1.2, 3)[np.floor(phase).astype(int) % 3] * (0.5 + 0.5 * h)
    return h, tone / tone.max()


def pat_planks(n, boards, seed):
    """Wood grain along U: long streaks, a few knots, board seams across V."""
    rng = np.random.default_rng(seed)
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    board = np.floor(Vv * boards).astype(int)
    tv = (Vv * boards) % 1.0
    phase = rng.uniform(0, 6.28, boards)[board]
    grain = 0.5 + 0.5 * np.sin(tv * 40 + np.sin(U * 6.28 * 2 + phase) * 3 + phase)
    grain = grain ** 3
    seam = np.clip(np.minimum(tv, 1 - tv) / 0.04, 0, 1)
    h = (0.6 + 0.4 * (1 - grain)) * seam
    tone = rng.uniform(0.75, 1.2, boards)[board] * (0.7 + 0.3 * (1 - grain)) * (0.5 + 0.5 * seam)
    return h, tone / tone.max()


def pat_tread(n, lugs):
    """Chunky tyre lugs: alternating blocks across the tread."""
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    row = np.floor(Vv * lugs).astype(int)
    tv = (Vv * lugs) % 1.0
    shift = np.where(row % 2 == 0, 0.0, 0.5)
    tu = (U + shift) % 1.0
    block = (np.abs(tu - 0.5) < 0.36) & (np.abs(tv - 0.5) < 0.32)
    h = block.astype(float)
    return h, 0.6 + 0.4 * h


def image_from(name, h, tone):
    size = h.shape[0]
    if name in bpy.data.images:
        bpy.data.images.remove(bpy.data.images[name])
    im = bpy.data.images.new(name, size, size, alpha=False)
    im.colorspace_settings.name = "Non-Color"  # before the pixels: changing it later blanks the buffer
    px = np.ones((size, size, 4), np.float32)
    px[..., 0] = h
    px[..., 1] = tone
    px[..., 2] = 0.0
    im.pixels.foreach_set(px.ravel())
    im.pack()
    return im


# ---------------------------------------------------------------- materials
class NodeKit:
    def __init__(self, mat):
        self.t = mat.node_tree
        self.t.nodes.clear()
        self.x = 0

    def new(self, kind, **inputs):
        node = self.t.nodes.new(kind)
        node.location = (self.x, 0)
        self.x += 180
        for key, val in inputs.items():
            self.set(node.inputs[key], val)
        return node

    def set(self, sock, val):
        if isinstance(val, bpy.types.NodeSocket):
            self.t.links.new(val, sock)
        else:
            sock.default_value = val

    def math(self, op, a, b=0.0, c=None):
        node = self.new("ShaderNodeMath")
        node.operation = op
        self.set(node.inputs[0], a)
        self.set(node.inputs[1], b)
        if c is not None:
            self.set(node.inputs[2], c)
        return node.outputs[0]

    def remap(self, v, a, b, c, d):
        node = self.new("ShaderNodeMapRange")
        node.clamp = True
        self.set(node.inputs["Value"], v)
        node.inputs["From Min"].default_value = a
        node.inputs["From Max"].default_value = b
        node.inputs["To Min"].default_value = c
        node.inputs["To Max"].default_value = d
        return node.outputs["Result"]

    def mix(self, fac, a, b):
        """Linear blend of two colours (sockets or tuples) by a factor."""
        node = self.new("ShaderNodeMix")
        node.data_type = "RGBA"
        self.set(node.inputs[0], fac)
        self.set(node.inputs[6], a if isinstance(a, bpy.types.NodeSocket) else (*a, 1.0))
        self.set(node.inputs[7], b if isinstance(b, bpy.types.NodeSocket) else (*b, 1.0))
        return node.outputs[2]

    def bsdf(self):
        bsdf = self.new("ShaderNodeBsdfPrincipled")
        out = self.new("ShaderNodeOutputMaterial")
        self.t.links.new(bsdf.outputs[0], out.inputs["Surface"])
        return bsdf


def surface(name, ca, cb, rough, detail=None, tile=(0.1, 0.1), dh=0.4, dk=0.3, nscale=4.0, grime=0.0,
            dirt=0.0, ao=0.6, aod=0.08, bump=0.0, bump_scale=20.0, metal=0.0, scuff=0.0, extra=None):
    """A layered procedural surface: two-tone noise, an optional tiling
    detail pattern (height in R, tone in G), grime, dirt low down, AO,
    bump. `extra(k, colour, factor) -> colour` can add masks (paint wear,
    rust, mud). Every material also carries the "rag" alpha mask."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_backface_culling = False
    k = NodeKit(m)
    tc = k.new("ShaderNodeTexCoord")
    obj = tc.outputs["Object"]
    bsdf = k.bsdf()
    tone = k.new("ShaderNodeTexNoise", Vector=obj, Scale=nscale, Detail=6.0, Roughness=0.6)
    ramp = k.new("ShaderNodeValToRGB", Fac=tone.outputs["Fac"])
    ramp.color_ramp.elements[0].position = 0.3
    ramp.color_ramp.elements[1].position = 0.72
    ramp.color_ramp.elements[0].color = (*cb, 1.0)
    ramp.color_ramp.elements[1].color = (*ca, 1.0)
    colour = ramp.outputs["Color"]
    factor = 1.0
    height = 0.0
    if detail is not None:
        uv = k.new("ShaderNodeUVMap")
        uv.uv_map = "Proc"
        mp = k.new("ShaderNodeMapping", Vector=uv.outputs["UV"])
        mp.inputs["Scale"].default_value = (1.0 / tile[0], 1.0 / tile[1], 1.0)
        img = k.new("ShaderNodeTexImage", Vector=mp.outputs["Vector"])
        img.image = detail
        img.interpolation = "Cubic"
        sep = k.new("ShaderNodeSeparateColor", Color=img.outputs["Color"])
        factor = k.math("MULTIPLY_ADD", sep.outputs["Green"], 2 * dk, 1 - dk)
        height = k.math("MULTIPLY", sep.outputs["Red"], dh)
    if grime:
        gr = k.new("ShaderNodeTexNoise", Vector=obj, Scale=2.7, Detail=4.0)
        factor = k.math("MULTIPLY", factor, k.remap(gr.outputs["Fac"], 0.42, 0.68, 1.0, grime))
    if dirt:
        sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
        factor = k.math("MULTIPLY", factor, k.remap(sep.outputs["Z"], dirt * 0.05, dirt, 0.5, 1.0))
    rn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=9.0, Detail=3.0)
    r_sock = k.remap(rn.outputs["Fac"], 0.3, 0.7, rough - 0.07, min(rough + 0.07, 1.0))
    if ao:
        aon = k.new("ShaderNodeAmbientOcclusion", Distance=aod)
        aon.samples = 16
        factor = k.math("MULTIPLY", factor, k.remap(k.math("POWER", aon.outputs["AO"], 1.5), 0.0, 1.0, 1.0 - ao, 1.0))
    if bump:
        bn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=bump_scale, Detail=5.0, Roughness=0.65)
        height = k.math("MULTIPLY_ADD", bn.outputs["Fac"], bump, height)
    metal_sock = metal
    if extra is not None:
        colour, r_sock, metal_sock, height = extra(k, obj, colour, r_sock, metal_sock, height)
    attr = k.new("ShaderNodeAttribute")
    attr.attribute_name = "rag"
    attr.attribute_type = "GEOMETRY"
    t1 = k.new("ShaderNodeTexNoise", Vector=obj, Scale=24.0, Detail=8.0, Roughness=0.72)
    t2 = k.new("ShaderNodeTexNoise", Vector=obj, Scale=110.0, Detail=3.0)
    edge = k.math("ADD", attr.outputs["Fac"], k.math("MULTIPLY_ADD", t1.outputs["Fac"], 0.2, -0.1))
    edge = k.math("ADD", edge, k.math("MULTIPLY_ADD", t2.outputs["Fac"], 0.05, -0.025))
    alpha = k.math("GREATER_THAN", edge, 0.0)
    alpha.node.name = "ALPHA"
    k.t.links.new(alpha, bsdf.inputs["Alpha"])
    factor = k.math("MULTIPLY", factor, k.remap(edge, 0.0, 0.03, 0.55, 1.0))
    col = k.new("ShaderNodeVectorMath")
    col.operation = "SCALE"
    k.set(col.inputs[0], colour)
    k.set(col.inputs["Scale"], factor)
    k.t.links.new(col.outputs[0], bsdf.inputs["Base Color"])
    k.set(bsdf.inputs["Roughness"], r_sock)
    k.set(bsdf.inputs["Metallic"], metal_sock)
    if not isinstance(height, float):
        b = k.new("ShaderNodeBump", Height=height, Strength=0.6, Distance=0.004)
        k.t.links.new(b.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def emissive(name, colour, strength):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    k = NodeKit(m)
    b = k.bsdf()
    b.inputs["Base Color"].default_value = (*colour, 1.0)
    b.inputs["Emission Color"].default_value = (*colour, 1.0)
    b.inputs["Emission Strength"].default_value = strength
    b.inputs["Roughness"].default_value = 0.3
    return m


def facet_material(name, rough=0.85):
    """Flat per-facet colour from the "Col" attribute (the low-poly look)."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    k = NodeKit(m)
    b = k.bsdf()
    ca = k.new("ShaderNodeVertexColor")
    ca.layer_name = "Col"
    k.t.links.new(ca.outputs["Color"], b.inputs["Base Color"])
    b.inputs["Roughness"].default_value = rough
    return m


# ---------------------------------------------------------------- live-session plumbing
def view3d_override():
    """Context for operators when called from the MCP (no active area)."""
    wm = bpy.context.window_manager
    for win in wm.windows:
        for area in win.screen.areas:
            if area.type == "VIEW_3D":
                region = next(r for r in area.regions if r.type == "WINDOW")
                return dict(window=win, area=area, region=region, screen=win.screen)
    return {}


def run_op(op, **kw):
    with bpy.context.temp_override(**view3d_override()):
        return op(**kw)


def select_only(objs, active=None):
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = active or (objs[0] if objs else None)


def setup_cycles(scene, samples=16):
    scene.render.engine = "CYCLES"
    prefs = bpy.context.preferences.addons["cycles"].preferences
    if prefs.compute_device_type == "NONE":
        for kind in ("OPTIX", "CUDA"):
            try:
                prefs.compute_device_type = kind
                break
            except TypeError:
                continue
    prefs.get_devices()
    for d in prefs.devices:
        d.use = d.type == prefs.compute_device_type
    scene.cycles.device = "GPU" if prefs.compute_device_type != "NONE" else "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = False


# ---------------------------------------------------------------- an asset
class Asset:
    """One model: its collection, parts, texture sets and export."""

    def __init__(self, name, seed):
        self.name = name
        self.seed = seed
        noise.seed_set(seed)
        self.rng = random.Random(seed)
        self.parts = []  # (object, set, weights)
        sc = bpy.context.scene
        old = bpy.data.collections.get(name)
        if old:
            for o in list(old.all_objects):
                bpy.data.objects.remove(o, do_unlink=True)
            bpy.data.collections.remove(old)
        bpy.data.orphans_purge(do_local_ids=True, do_linked_ids=False, do_recursive=True)
        self.coll = bpy.data.collections.new(name)
        sc.collection.children.link(self.coll)
        self.mats = {}
        self.sets = {}
        self.temp = []  # helper objects (boolean cutters) removed once modifiers are applied
        self.painters = {}  # object name -> facet colour function  # material name -> texture set ("facet" = flat colours, None = keep as is)

    def mat(self, key, material, tex_set):
        self.mats[key] = material
        self.sets[key] = tex_set
        return material

    def part(self, name, bm, mat, weights=None, smooth=True, subsurf=0, solidify=0.0, decimate=None, bevel=0.0,
             paint=None):
        lay = bm.verts.layers.float.get("rag")
        torn = lay is not None and any(v[lay] < 0.999 for v in bm.verts)
        if lay is None:
            lay = bm.verts.layers.float.new("rag")
            for v in bm.verts:
                v[lay] = 1.0
        if "Proc" not in bm.loops.layers.uv:
            box_uv(bm)
        me = bpy.data.meshes.new(f"{self.name}_{name}")
        bm.to_mesh(me)
        bm.free()
        if smooth:
            me.shade_smooth()
        else:
            me.shade_flat()
        ob = bpy.data.objects.new(f"{self.name}_{name}", me)
        ob["torn"] = torn
        self.coll.objects.link(ob)
        ob.data.materials.append(self.mats[mat])
        if bevel:
            m = ob.modifiers.new("bevel", "BEVEL")
            m.width = bevel
            m.segments = 2
            m.limit_method = "ANGLE"
        if subsurf:
            m = ob.modifiers.new("sub", "SUBSURF")
            m.levels = m.render_levels = subsurf
        if solidify:
            m = ob.modifiers.new("solid", "SOLIDIFY")
            m.thickness = solidify
            m.offset = 0.0
        if decimate:
            m = ob.modifiers.new("dec", "DECIMATE")
            m.ratio = decimate
        if paint is not None:
            self.painters[ob.name] = paint
        self.parts.append((ob, self.sets[mat], weights))
        return ob

    # -- finishing
    def cutter(self, name, bm):
        """A hidden helper mesh for boolean cuts."""
        lay = bm.verts.layers.float.new("rag")  # cut walls are whole cloth/paint, not torn
        for v in bm.verts:
            v[lay] = 1.0
        me = bpy.data.meshes.new(f"{self.name}_cut_{name}")
        bm.to_mesh(me)
        bm.free()
        ob = bpy.data.objects.new(me.name, me)
        self.coll.objects.link(ob)
        ob.hide_render = True
        ob.display_type = "WIRE"
        self.temp.append(ob)
        return ob

    def cut(self, ob, cutter):
        if not cutter.data.materials:
            cutter.data.materials.append(ob.data.materials[0])  # the cut walls take the host's material
        m = ob.modifiers.new(cutter.name, "BOOLEAN")
        m.object = cutter
        m.operation = "DIFFERENCE"
        m.solver = "EXACT"

    def _apply(self):
        dg = bpy.context.evaluated_depsgraph_get()
        for ob, _s, _w in self.parts:
            if ob.modifiers:
                me = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
                old = ob.data
                ob.modifiers.clear()
                ob.data = me
                bpy.data.meshes.remove(old)
            if not ob.get("torn") and "rag" in ob.data.attributes:
                # modifiers (exact booleans) leave new vertices at 0: whole parts stay whole
                att = ob.data.attributes["rag"]
                att.data.foreach_set("value", np.ones(len(att.data), np.float32))
        for ob in self.temp:
            me = ob.data
            bpy.data.objects.remove(ob, do_unlink=True)
            bpy.data.meshes.remove(me)
        self.temp = []

    def _weights(self):
        for ob, _s, weights in self.parts:
            if weights is None:
                continue
            groups = {}
            for v in ob.data.vertices:
                ws = {weights: 1.0} if isinstance(weights, str) else weights(v.co)
                total = sum(ws.values())
                for b, w in ws.items():
                    if w <= 1e-3:
                        continue
                    if b not in groups:
                        groups[b] = ob.vertex_groups.new(name=b)
                    groups[b].add([v.index], w / total, "REPLACE")

    def _join(self):
        sets = {}
        for ob, s, _ in self.parts:
            sets.setdefault(s or ob.name, []).append(ob)
        out = {}
        for s, obs in sets.items():
            target = obs[0]
            if len(obs) > 1:
                select_only(obs, target)
                with bpy.context.temp_override(**view3d_override(), active_object=target,
                                               selected_editable_objects=obs, object=target):
                    bpy.ops.object.join()
            target.name = f"{self.name}_{s.strip('=')}"
            target.data.name = target.name
            bm = bmesh.new()
            bm.from_mesh(target.data)
            bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
            bm.to_mesh(target.data)
            bm.free()
            out[s] = target
        return out

    def _unwrap(self, ob):
        me = ob.data
        bake_uv = me.uv_layers.new(name="Bake")
        me.uv_layers.active = bake_uv
        bake_uv.active_render = True
        select_only([ob])
        with bpy.context.temp_override(**view3d_override(), active_object=ob, object=ob):
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=0.004, area_weight=0.0,
                                     correct_aspect=True, scale_to_bounds=False)
            bpy.ops.uv.pack_islands(margin=0.004, rotate=True)
            bpy.ops.object.mode_set(mode="OBJECT")

    def _bake(self, ob, s, size):
        imgs = {}
        for key, alpha in (("color", True), ("rough", False), ("normal", False), ("alpha", False)):
            nm = f"{self.name}_{s}_{key}"
            if nm in bpy.data.images:
                bpy.data.images.remove(bpy.data.images[nm])
            imgs[key] = bpy.data.images.new(nm, size, size, alpha=alpha)
            if key != "color":
                imgs[key].colorspace_settings.name = "Non-Color"
        targets = []
        for slot in ob.material_slots:
            if slot.material is None:
                continue
            nt = slot.material.node_tree
            node = nt.nodes.new("ShaderNodeTexImage")
            targets.append((nt, node))
        select_only([ob])
        sc = bpy.context.scene
        engine, samples = sc.render.engine, None
        setup_cycles(sc)
        samples = sc.cycles.samples
        sc.render.bake.margin = 16
        for key, kind, n in (("color", "DIFFUSE", 64), ("rough", "ROUGHNESS", 1), ("normal", "NORMAL", 8),
                             ("alpha", "EMIT", 8)):
            for nt, node in targets:
                node.image = imgs[key]
                nt.nodes.active = node
            sc.cycles.samples = n
            ov = dict(**view3d_override(), active_object=ob, object=ob, selected_objects=[ob])
            if kind == "EMIT":
                saved = []
                for nt, _node in targets:
                    out = next(x for x in nt.nodes if x.type == "OUTPUT_MATERIAL")
                    src = out.inputs["Surface"].links[0].from_socket
                    em = nt.nodes.new("ShaderNodeEmission")
                    nt.links.new(nt.nodes["ALPHA"].outputs[0], em.inputs["Color"])
                    nt.links.new(em.outputs[0], out.inputs["Surface"])
                    saved.append((nt, out, src, em))
                with bpy.context.temp_override(**ov):
                    bpy.ops.object.bake(type="EMIT", margin=16, uv_layer="Bake")
                for nt, out, src, em in saved:
                    nt.links.new(src, out.inputs["Surface"])
                    nt.nodes.remove(em)
            else:
                kw = dict(pass_filter={"COLOR"}) if kind == "DIFFUSE" else {}
                if kind == "NORMAL":
                    kw = dict(normal_space="TANGENT")
                with bpy.context.temp_override(**ov):
                    bpy.ops.object.bake(type=kind, margin=16, uv_layer="Bake", **kw)
        for nt, node in targets:
            nt.nodes.remove(node)
        sc.cycles.samples = samples
        try:
            sc.render.engine = engine
        except TypeError:
            pass
        n = size * size * 4
        px = np.empty(n, np.float32)
        imgs["color"].pixels.foreach_get(px)
        al = np.empty(n, np.float32)
        imgs["alpha"].pixels.foreach_get(al)
        px[3::4] = al[0::4]
        imgs["color"].pixels.foreach_set(px)
        bpy.data.images.remove(imgs.pop("alpha"))
        imgs["normal"].scale(size // 2, size // 2)
        imgs["rough"].scale(size // 4, size // 4)
        for im in imgs.values():
            im.pack()
        return imgs

    def _final_material(self, s, imgs, masked, metal=0.0):
        m = bpy.data.materials.new(f"{self.name}_{s}")
        m.use_backface_culling = False
        k = NodeKit(m)
        uv = k.new("ShaderNodeUVMap")
        uv.uv_map = "Bake"
        b = k.bsdf()
        col = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
        col.image = imgs["color"]
        k.t.links.new(col.outputs["Color"], b.inputs["Base Color"])
        if masked:
            k.t.links.new(k.math("ROUND", col.outputs["Alpha"]), b.inputs["Alpha"])
        rough = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
        rough.image = imgs["rough"]
        k.t.links.new(rough.outputs["Color"], b.inputs["Roughness"])
        nrm = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
        nrm.image = imgs["normal"]
        nmap = k.new("ShaderNodeNormalMap", Color=nrm.outputs["Color"])
        k.t.links.new(nmap.outputs["Normal"], b.inputs["Normal"])
        b.inputs["Metallic"].default_value = metal
        return m

    def paint_facets(self, ob, colour_fn, jitter=0.06):
        """Flat colour per face from `colour_fn(centre, normal) -> (r, g, b)` (linear), jittered."""
        me = ob.data
        if "Col" in me.color_attributes:
            me.color_attributes.remove(me.color_attributes["Col"])
        attr = me.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
        rng = random.Random(self.seed + 7)
        cols = np.empty(len(me.loops) * 4, np.float32)
        for poly in me.polygons:
            c = colour_fn(poly.center, poly.normal)
            j = 1.0 + rng.uniform(-jitter, jitter)
            rgba = (clamp(c[0] * j), clamp(c[1] * j), clamp(c[2] * j), 1.0)
            for li in poly.loop_indices:
                cols[li * 4: li * 4 + 4] = rgba
        attr.data.foreach_set("color", cols)
        me.color_attributes.active_color = attr
        me.shade_flat()

    def rig(self, objs, joints):
        """Bones at `joints` [(name, parent, bevy_xyz)], pointing up with no roll:
        identity rest rotations in glTF."""
        name = f"{self.name}_rig"
        arm = bpy.data.armatures.new(name)
        rig = bpy.data.objects.new(self.name, arm)
        self.coll.objects.link(rig)
        select_only([rig])
        with bpy.context.temp_override(**view3d_override(), active_object=rig, object=rig):
            bpy.ops.object.mode_set(mode="EDIT")
            for jn, parent, (bx, by, bz) in joints:
                eb = arm.edit_bones.new(jn)
                eb.head = V(bx, -bz, by)
                eb.tail = eb.head + V(0, 0, 0.08)
                eb.roll = 0.0
                if parent:
                    eb.parent = arm.edit_bones[parent]
            bpy.ops.object.mode_set(mode="OBJECT")
        for ob in objs:
            ob.parent = rig
            mod = ob.modifiers.new("Armature", "ARMATURE")
            mod.object = rig
        return rig

    def finish(self, tex=2048, out=None, joints=None, metal=None):
        """Apply, weight, join per set, bake textured sets, rig, export."""
        self._apply()
        for ob, _s, _w in self.parts:
            if ob.name in self.painters:
                self.paint_facets(ob, self.painters[ob.name])
        if joints:
            self._weights()
        joined = self._join()
        metal = metal or {}
        for s, ob in joined.items():
            me = ob.data
            if s == "facet" or s.startswith("="):
                pass  # flat facet colours, or plain materials kept as they are
            elif s in self.sets.values():
                rag = me.attributes.get("rag")
                vals = np.empty(len(me.vertices), np.float32)
                if rag is not None:
                    rag.data.foreach_get("value", vals)
                masked = rag is not None and bool((vals < 0.1).any())
                self._unwrap(ob)
                size = tex.get(s, tex.get("*", 1024)) if isinstance(tex, dict) else tex
                imgs = self._bake(ob, s, size)
                mat = self._final_material(s, imgs, masked, metal.get(s, 0.0))
                me.materials.clear()
                me.materials.append(mat)
                if "Proc" in me.uv_layers:
                    me.uv_layers.remove(me.uv_layers["Proc"])
            for nm in ("rag", "col"):
                if nm in me.attributes:
                    me.attributes.remove(me.attributes[nm])
        objs = list(joined.values())
        rig = self.rig(objs, joints) if joints else None
        if out:
            self.export(out, objs + ([rig] if rig else []))
        return joined, rig

    def export(self, path, objs):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        select_only(objs)
        with bpy.context.temp_override(**view3d_override()):
            bpy.ops.export_scene.gltf(
                filepath=path, export_format="GLB", use_selection=True, export_yup=True, export_apply=False,
                export_skins=True, export_influence_nb=4, export_animations=False, export_tangents=True,
                export_materials="EXPORT", export_image_format="AUTO", export_cameras=False, export_lights=False,
                export_copyright=f"{self.name}: original procedural model (tools/models)")
        print("wrote", path)


# ---------------------------------------------------------------- presentation renders
def render_views(coll, out_dir, views, samples=128, size=(900, 900), look_at=None, bg=(0.42, 0.41, 0.39),
                 night=False):
    """Render `views` [(name, yaw, pitch, dist_scale)] of a collection in a
    temporary scene (the user's scene and camera are untouched)."""
    os.makedirs(out_dir, exist_ok=True)
    objs = [o for o in coll.all_objects if o.type == "MESH"]
    dg = bpy.context.evaluated_depsgraph_get()
    lo, hi = V(1e9, 1e9, 1e9), V(-1e9, -1e9, -1e9)
    for o in objs:
        for c in o.evaluated_get(dg).bound_box:
            w = o.matrix_world @ Vector(c)
            lo = V(min(lo.x, w.x), min(lo.y, w.y), min(lo.z, w.z))
            hi = V(max(hi.x, w.x), max(hi.y, w.y), max(hi.z, w.z))
    centre = look_at or (lo + hi) / 2
    radius = (hi - lo).length / 2
    sc = bpy.data.scenes.new("_render")
    try:
        sc.collection.children.link(coll)
        setup_cycles(sc, samples)
        sc.cycles.use_denoising = True
        sc.view_settings.view_transform = "AgX"
        sc.view_settings.look = "AgX - Medium High Contrast"
        world = bpy.data.worlds.new("_render_world")
        sc.world = world
        k = NodeKit(world) if False else None
        world.use_nodes = True
        bgn = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
        bgn.inputs["Color"].default_value = (*((0.02, 0.025, 0.04) if night else bg), 1)
        bgn.inputs["Strength"].default_value = 0.4 if night else 0.35
        me = bpy.data.meshes.new("_floor")
        bm = bmesh.new()
        bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=radius * 20)
        bm.to_mesh(me)
        bm.free()
        floor = bpy.data.objects.new("_floor", me)
        floor.location.z = lo.z
        fm = bpy.data.materials.new("_floor")
        fk = NodeKit(fm)
        fb = fk.bsdf()
        fb.inputs["Base Color"].default_value = ((0.012,) * 3 if night else (0.5, 0.49, 0.47)) + (1,)
        fb.inputs["Roughness"].default_value = 0.9
        me.materials.append(fm)
        sc.collection.objects.link(floor)
        cam = bpy.data.objects.new("_cam", bpy.data.cameras.new("_cam"))
        sc.collection.objects.link(cam)
        sc.camera = cam
        cam.data.lens = 60
        cam.data.sensor_fit = "AUTO"
        lights = []
        for nm, e, col in (("key", 1.0, (1, 0.97, 0.92)), ("fill", 0.27, (0.9, 0.93, 1.0)), ("rim", 0.8, (1, 1, 1))):
            ld = bpy.data.lights.new("_" + nm, "AREA")
            ld.color = col
            lo_ = bpy.data.objects.new("_" + nm, ld)
            sc.collection.objects.link(lo_)
            lights.append((lo_, e))
        whole = centre
        for view in views:
            name, yaw, pitch, dscale = view[:4]
            centre = Vector(view[4]) if len(view) > 4 else whole
            dist = radius / math.tan(math.radians(12)) * dscale
            d = V(math.sin(yaw) * math.cos(pitch), -math.cos(yaw) * math.cos(pitch), math.sin(pitch))
            cam.location = centre + d * dist
            cam.rotation_euler = (centre - cam.location).to_track_quat("-Z", "Y").to_euler()
            for (lo_, e), (dy, h, dd) in zip(lights, ((0.7, 1.2, 1.3), (-1.1, 0.4, 1.3), (math.pi - 0.5, 1.2, 1.1))):
                a = yaw + dy
                lo_.location = centre + V(math.sin(a), -math.cos(a), h * 0.8) * radius * 3 * dd
                lo_.rotation_euler = (centre - lo_.location).to_track_quat("-Z", "Y").to_euler()
                lo_.data.size = radius * 1.5
                lo_.data.energy = 520 * e * (radius / 1.5) ** 2 * (0.3 if night and e < 0.5 else 1.0)
                if night:
                    lo_.data.color = (0.62, 0.72, 1.0) if e == 1.0 else lo_.data.color
            sc.render.resolution_x, sc.render.resolution_y = size
            sc.render.filepath = os.path.join(out_dir, f"{name}.png")
            with bpy.context.temp_override(**view3d_override()):
                bpy.ops.render.render(write_still=True, scene=sc.name)
            print("rendered", name)
    finally:
        for o in list(sc.collection.objects):
            data = o.data
            bpy.data.objects.remove(o, do_unlink=True)
            if data is not None and data.users == 0:
                if isinstance(data, bpy.types.Mesh):
                    bpy.data.meshes.remove(data)
                elif isinstance(data, bpy.types.Light):
                    bpy.data.lights.remove(data)
                elif isinstance(data, bpy.types.Camera):
                    bpy.data.cameras.remove(data)
        if sc.world:
            bpy.data.worlds.remove(sc.world)
        bpy.data.scenes.remove(sc)

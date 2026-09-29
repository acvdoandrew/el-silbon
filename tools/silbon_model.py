"""El Silbón, after the concept sheet, as a rigged and texture-baked glTF.

Everything is built from code here (no scans, no downloaded or AI meshes):
lofted tubes and ribbons for the body, rags, hat, sack and femurs, small
tileable weave/braid/burlap patterns made with numpy, procedural Cycles
materials, then one bake per texture set so the glTF carries plain images.

Deterministic (fixed seed). Run with Blender 5.2 (system PATH first, so
Blender finds its own Python, not a mise/venv one):

    PATH=/usr/bin:$PATH blender -b --factory-startup \
        --python tools/silbon_model.py -- [--quick] [--tex 2048] [--out FILE] [--blend FILE]

Output (default `assets/models/silbon.glb`): three textured meshes (skin,
cloth, gear) plus the eyes, skinned to a joint hierarchy whose names and
rest offsets match `JointKind` in `src/world/silbon.rs` (Body, HipL/R,
KneeL/R, Torso, Head, ShoulderL/R, ElbowL/R, Sack, Coat; identity rest
rotations). Y up, faces -Z, feet at the origin, about 3 m tall.
"""

import math
import os
import random
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector, noise

TAU = math.tau
ARGV = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def opt(name, default):
    return ARGV[ARGV.index(name) + 1] if name in ARGV else default


QUICK = "--quick" in ARGV
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = opt("--out", os.path.join(ROOT, "assets", "models", "silbon.glb"))
BLEND = opt("--blend", None)
TEX = int(opt("--tex", 512 if QUICK else 2048))
SEED = 0x51B

noise.seed_set(SEED)
RNG = random.Random(SEED)

# ---------------------------------------------------------------- the rig
# Joint positions in the game's own space (Bevy: Y up, forward -Z), the
# same constants as `silbon.rs`: HIP_Y 1.66, THIGH/SHIN 0.84, CHEST 0.84,
# UPPER_ARM 0.74, head at CHEST + 0.16.
HIP_Y, THIGH, CHEST, UPPER_ARM = 1.66, 0.84, 0.84, 0.74
SHOULDER_Y = HIP_Y + CHEST - 0.03
JOINTS = [
    # name, parent, position (Bevy space)
    ("Body", None, (0.0, 0.0, 0.0)),
    ("HipL", "Body", (-0.1, HIP_Y, 0.0)),
    ("KneeL", "HipL", (-0.1, HIP_Y - THIGH, 0.0)),
    ("HipR", "Body", (0.1, HIP_Y, 0.0)),
    ("KneeR", "HipR", (0.1, HIP_Y - THIGH, 0.0)),
    ("Torso", "Body", (0.0, HIP_Y, 0.0)),
    ("Coat", "Torso", (0.0, HIP_Y, 0.0)),
    ("Head", "Torso", (0.0, HIP_Y + CHEST + 0.16, -0.01)),
    ("ShoulderL", "Torso", (-0.21, SHOULDER_Y, 0.0)),
    ("ElbowL", "ShoulderL", (-0.21, SHOULDER_Y - UPPER_ARM, 0.0)),
    ("ShoulderR", "Torso", (0.21, SHOULDER_Y, 0.0)),
    ("ElbowR", "ShoulderR", (0.21, SHOULDER_Y - UPPER_ARM, 0.0)),
    ("Sack", "Torso", (0.05, SHOULDER_Y + 0.01, 0.12)),
]

# Modelling happens in "character space": Blender Z up, facing -Y, so the
# character's left is +X. Bevy's HipL (x = -0.1) is the leg at +X here. At
# the end everything turns 180 degrees about Z (facing +Y), which the glTF
# exporter maps to Bevy's forward -Z.
SIDES = {1: "L", -1: "R"}


# ---------------------------------------------------------------- helpers
def V(x, y, z):
    return Vector((x, y, z))


FRONT = V(0, -1, 0)
UP = V(0, 0, 1)


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
    if x <= xs[0]:
        return ys[0]
    for i in range(1, len(xs)):
        if x <= xs[i]:
            t = (x - xs[i - 1]) / (xs[i] - xs[i - 1])
            return lerp(ys[i - 1], ys[i], sstep(0.0, 1.0, t) * 0.5 + t * 0.5)
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
    """Radius of an ellipse with semi-axis a along the frame normal, b along the binormal."""
    c, s = math.cos(th), math.sin(th)
    return 1.0 / math.sqrt((c / a_n) ** 2 + (s / b_b) ** 2)


def catmull(ctrl, n):
    c = [Vector(p) for p in ctrl]
    out = []
    for k in range(n):
        u = k / (n - 1) * (len(c) - 1)
        i = min(int(u), len(c) - 2)
        t = u - i
        p0, p1, p2, p3 = c[max(i - 1, 0)], c[i], c[i + 1], c[min(i + 2, len(c) - 1)]
        out.append(
            0.5
            * (
                2 * p1
                + (p2 - p0) * t
                + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t * t
                + (3 * p1 - p0 - 3 * p2 + p3) * t * t * t
            )
        )
    return out


def resample(ctrl, n):
    """Smooth path through `ctrl`, `n` points evenly spaced by arc length."""
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


def tube(
    ctrl,
    n,
    sides,
    radius,
    ref=FRONT,
    span=None,
    closed=True,
    keep=None,
    disp=None,
    cap=False,
    loop=False,
    ru=0.08,
    raw=False,
    transport=True,
):
    """Loft a cross-section along a smooth path.

    `radius(c)` and `disp(c)` receive a `Ctx`; `span(t)` gives the angle range
    of an open sheet (`closed=False`). Angle 0 points along the frame normal
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
            c.q = p + c.d * 0.05  # a stand-in surface point for noise lookups
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


def ribbon(ctrl, width, n, twist=0.0, taper=0.7, seed=0.0, ref=FRONT, notch=True, fray=True):
    """A hanging strip of cloth/hair: two columns wide, jagged at its end."""
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
        w = width * (1.0 - taper * sstep(0.55, 1.0, t)) * (1 + 0.25 * nz((seed, t * 6, 1.1)))
        row = [p - side * w / 2, p, p + side * w / 2]
        if notch and i == n - 1:
            row[1] = p - T * width * (0.4 + 0.6 * abs(nz((seed, 9.1, 2.2))))
        rows.append(row)
        uvs.append([(0.0, acc), (width / 2, acc), (width, acc)])
    bm = grid(rows, uvs)
    if fray:
        bm.verts.ensure_lookup_table()
        lay = bm.verts.layers.float.new("rag")
        for idx, v in enumerate(bm.verts):
            i, k = divmod(idx, 3)
            left = total * (1 - i / (n - 1))
            v[lay] = min(left - 0.03, 0.03 if k != 1 else 0.08)
    return bm


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
    tw = rng.uniform(0.7, 1.3, threads)
    tf = rng.uniform(0.7, 1.3, threads)
    tone = np.where(over, tw[ix], tf[iy]) * (0.45 + 0.55 * h)
    return h, tone / tone.max()


def pat_braid(n, bands, strands, seed):
    rng = np.random.default_rng(seed)
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    band = np.floor(Vv * bands).astype(int)
    tv = (Vv * bands) % 1.0
    s = np.where(tv < 0.5, 1.0, -1.0)
    phase = U * strands + s * tv * 2.0 + band * 0.37
    idx = np.floor(phase).astype(int) % strands
    strand = _prof(phase % 1.0, 0.86)
    edge = np.sin(np.pi * tv) ** 0.35
    mid = 1.0 - 0.55 * np.exp(-(((tv - 0.5) / 0.035) ** 2))
    h = strand * edge * mid
    tone = rng.uniform(0.65, 1.3, (bands, strands))[band % bands, idx] * (0.4 + 0.6 * h)
    return h, tone / tone.max()


def pat_rope(n, seed):
    rng = np.random.default_rng(seed)
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    phase = (U + Vv) * 3.0
    h = _prof(phase % 1.0, 0.95)
    fib = _prof((U * 24 + Vv * 60) % 1.0, 0.8)
    h = h * (0.8 + 0.2 * fib)
    tone = rng.uniform(0.8, 1.2, 3)[np.floor(phase).astype(int) % 3] * (0.5 + 0.5 * h)
    return h, tone / tone.max()


def pat_streaks(n, seed):
    rng = np.random.default_rng(seed)
    col = rng.random(n)
    col = np.convolve(np.tile(col, 3), np.ones(3) / 3, "same")[n : 2 * n]
    h = np.tile(col, (n, 1))
    return h, 0.6 + 0.4 * h


def image_from(name, h, tone):
    size = h.shape[0]
    im = bpy.data.images.new(name, size, size, alpha=False)
    # colourspace first: changing it afterwards regenerates the blank buffer
    im.colorspace_settings.name = "Non-Color"
    px = np.ones((size, size, 4), np.float32)
    px[..., 0] = h
    px[..., 1] = tone
    px[..., 2] = 0.0
    im.pixels.foreach_set(px.ravel())
    im.pack()
    return im


DETAIL = {}


def make_details():
    DETAIL["fabric"] = image_from("pat_fabric", *pat_weave(512, 24, 0.12, 0.25, 1))
    DETAIL["homespun"] = image_from("pat_homespun", *pat_weave(512, 16, 0.18, 0.35, 2))
    DETAIL["burlap"] = image_from("pat_burlap", *pat_weave(512, 12, 0.38, 0.4, 3))
    DETAIL["braid"] = image_from("pat_braid", *pat_braid(512, 4, 36, 4))
    DETAIL["rope"] = image_from("pat_rope", *pat_rope(256, 5))
    DETAIL["streaks"] = image_from("pat_streaks", *pat_streaks(256, 6))


# ---------------------------------------------------------------- materials
# name: (set, colour a, colour b, roughness, spec)
MAT_SPECS = {
    "skin": ("skin", (0.2, 0.165, 0.13), (0.085, 0.066, 0.052), 0.62,
             dict(nscale=6, fine=0.35, wrinkle=0.5, ao=0.9, aod=0.05, grime=0.55)),
    "face": ("skin", (0.035, 0.028, 0.022), (0.012, 0.01, 0.008), 0.7,
             dict(nscale=8, fine=0.3, wrinkle=0.4, ao=0.9, aod=0.03)),
    "claw": ("skin", (0.03, 0.026, 0.022), (0.008, 0.007, 0.006), 0.22,
             dict(nscale=30, fine=0.1, wrinkle=0.15, ao=0.3, streak=True)),
    "hair": ("skin", (0.03, 0.024, 0.02), (0.006, 0.005, 0.004), 0.55,
             dict(detail="streaks", tile=(0.02, 1.0), dh=0.15, dk=0.4, ao=0.5)),
    "coat": ("cloth", (0.095, 0.062, 0.037), (0.04, 0.028, 0.019), 0.93,
             dict(detail="homespun", tile=(0.07, 0.07), dh=0.35, dk=0.28, dirt=1.4, ao=0.8, grime=0.5, wrinkle=0.6)),
    "tan": ("cloth", (0.17, 0.12, 0.075), (0.075, 0.053, 0.034), 0.95,
            dict(detail="homespun", tile=(0.09, 0.09), dh=0.4, dk=0.3, dirt=2.0, ao=0.8, grime=0.6, wrinkle=0.6)),
    "pants": ("cloth", (0.085, 0.074, 0.062), (0.035, 0.031, 0.027), 0.9,
              dict(detail="fabric", tile=(0.06, 0.06), dh=0.3, dk=0.22, dirt=0.9, ao=0.8, grime=0.6, wrinkle=0.6)),
    "sash": ("cloth", (0.2, 0.14, 0.085), (0.09, 0.065, 0.04), 0.9,
             dict(detail="rope", tile=(0.05, 0.05), dh=0.5, dk=0.3, ao=0.7)),
    "straw": ("gear", (0.12, 0.092, 0.062), (0.042, 0.032, 0.022), 0.85,
              dict(detail="braid", tile=(0.1, 0.1), dh=0.6, dk=0.4, ao=0.85, grime=0.5)),
    "burlap": ("gear", (0.25, 0.185, 0.115), (0.11, 0.08, 0.05), 0.96,
               dict(detail="burlap", tile=(0.07, 0.07), dh=0.7, dk=0.45, dirt=1.9, ao=0.8, grime=0.55, wrinkle=0.4)),
    "void": ("gear", (0.012, 0.01, 0.008), (0.004, 0.003, 0.003), 1.0, dict()),
    "rope": ("gear", (0.30, 0.235, 0.16), (0.15, 0.11, 0.075), 0.93,
             dict(detail="rope", tile=(0.04, 0.04), dh=0.8, dk=0.35, ao=0.6)),
    "bone": ("gear", (0.4, 0.36, 0.28), (0.19, 0.155, 0.11), 0.55,
             dict(nscale=5, fine=0.25, wrinkle=0.25, ao=0.7, grime=0.7, dirt=2.4)),
    "leather": ("gear", (0.075, 0.056, 0.042), (0.025, 0.019, 0.015), 0.5,
                dict(nscale=5, fine=0.35, wrinkle=0.8, ao=0.8, grime=0.6, scuff=True, dirt=0.25)),
}
MATS = {}


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


def build_material(name, spec):
    _, ca, cb, rough, o = spec
    m = bpy.data.materials.new(name)
    m.use_backface_culling = False
    k = NodeKit(m)
    tc = k.new("ShaderNodeTexCoord")
    obj = tc.outputs["Object"]
    bsdf = k.new("ShaderNodeBsdfPrincipled")
    out = k.new("ShaderNodeOutputMaterial")
    k.t.links.new(bsdf.outputs[0], out.inputs["Surface"])

    tone = k.new("ShaderNodeTexNoise", Vector=obj, Scale=o.get("nscale", 4.0), Detail=6.0, Roughness=0.6)
    ramp = k.new("ShaderNodeValToRGB", Fac=tone.outputs["Fac"])
    ramp.color_ramp.elements[0].position = 0.3
    ramp.color_ramp.elements[1].position = 0.72
    ramp.color_ramp.elements[0].color = (*cb, 1.0)
    ramp.color_ramp.elements[1].color = (*ca, 1.0)
    factor = 1.0
    height = 0.0

    if "detail" in o:
        uv = k.new("ShaderNodeUVMap")
        uv.uv_map = "Proc"
        tile = o["tile"]
        mp = k.new("ShaderNodeMapping", Vector=uv.outputs["UV"])
        mp.inputs["Scale"].default_value = (1.0 / tile[0], 1.0 / tile[1], 1.0)
        img = k.new("ShaderNodeTexImage", Vector=mp.outputs["Vector"])
        img.image = DETAIL[o["detail"]]
        img.interpolation = "Cubic"
        sep = k.new("ShaderNodeSeparateColor", Color=img.outputs["Color"])
        dk = o.get("dk", 0.3)
        factor = k.math("MULTIPLY_ADD", sep.outputs["Green"], 2 * dk, 1 - dk)
        height = k.math("MULTIPLY", sep.outputs["Red"], o.get("dh", 0.4))

    if o.get("streak"):
        st = k.new("ShaderNodeTexWave", Vector=obj, Scale=60.0, Distortion=4.0)
        st.wave_type = "RINGS"
        factor = k.math("MULTIPLY", factor, k.remap(st.outputs["Fac"], 0, 1, 0.7, 1.3))

    if o.get("grime"):
        gr = k.new("ShaderNodeTexNoise", Vector=obj, Scale=2.7, Detail=4.0)
        factor = k.math("MULTIPLY", factor, k.remap(gr.outputs["Fac"], 0.42, 0.68, 1.0, o["grime"]))

    if o.get("dirt"):
        sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
        dz = o["dirt"]
        factor = k.math("MULTIPLY", factor, k.remap(sep.outputs["Z"], dz * 0.05, dz, 0.45, 1.0))

    if o.get("scuff"):
        sc = k.new("ShaderNodeTexNoise", Vector=obj, Scale=14.0, Detail=8.0, Roughness=0.7)
        scuff = k.remap(sc.outputs["Fac"], 0.6, 0.72, 0.0, 1.0)
        factor = k.math("MULTIPLY", factor, k.math("MULTIPLY_ADD", scuff, 1.6, 1.0))
        bsdf_rough = k.math("MULTIPLY_ADD", scuff, 0.35, rough)
    else:
        rn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=9.0, Detail=3.0)
        bsdf_rough = k.remap(rn.outputs["Fac"], 0.3, 0.7, rough - 0.07, min(rough + 0.07, 1.0))
    k.set(bsdf.inputs["Roughness"], bsdf_rough)

    if o.get("ao"):
        ao = k.new("ShaderNodeAmbientOcclusion", Distance=o.get("aod", 0.08))
        ao.samples = 16
        aof = k.math("POWER", ao.outputs["AO"], 1.6)
        factor = k.math("MULTIPLY", factor, k.remap(aof, 0.0, 1.0, 1.0 - o["ao"], 1.0))

    # Torn edges: every part carries a "rag" field (metres to the nearest
    # tear, negative where the cloth is gone); noise frays the boundary and
    # the bake turns it into an alpha mask. Fraying edges are dirtier.
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
    k.set(col.inputs[0], ramp.outputs["Color"])
    k.set(col.inputs["Scale"], factor)
    k.t.links.new(col.outputs[0], bsdf.inputs["Base Color"])

    if o.get("fine") or o.get("wrinkle"):
        fine = k.new("ShaderNodeTexNoise", Vector=obj, Scale=180.0, Detail=4.0)
        wr = k.new("ShaderNodeTexNoise", Vector=obj, Scale=22.0, Detail=5.0, Roughness=0.65)
        vor = k.new("ShaderNodeTexVoronoi", Vector=obj, Scale=90.0)
        extra = k.math("MULTIPLY", fine.outputs["Fac"], o.get("fine", 0.0) * 0.5)
        extra = k.math("MULTIPLY_ADD", wr.outputs["Fac"], o.get("wrinkle", 0.0), extra)
        extra = k.math("MULTIPLY_ADD", vor.outputs["Distance"], o.get("fine", 0.0) * 0.3, extra)
        height = k.math("ADD", height, extra)
    if not isinstance(height, float):
        bump = k.new("ShaderNodeBump", Height=height, Strength=0.6, Distance=0.004)
        k.t.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def make_materials():
    for name, spec in MAT_SPECS.items():
        MATS[name] = build_material("src_" + name, spec)
    eye = bpy.data.materials.new("Silbon_Eyes")
    eye.use_backface_culling = False
    k = NodeKit(eye)
    bsdf = k.new("ShaderNodeBsdfPrincipled")
    out = k.new("ShaderNodeOutputMaterial")
    k.t.links.new(bsdf.outputs[0], out.inputs["Surface"])
    bsdf.inputs["Base Color"].default_value = (1.0, 0.93, 0.7, 1.0)
    bsdf.inputs["Emission Color"].default_value = (1.0, 0.9, 0.62, 1.0)
    bsdf.inputs["Emission Strength"].default_value = 18.0
    bsdf.inputs["Roughness"].default_value = 0.3
    MATS["eye"] = eye


# ---------------------------------------------------------------- parts
COLL = None
PARTS = []  # (object, set name, weight function)


def part(name, bm, mat, weights, subsurf=0, solidify=0.0):
    if "rag" not in bm.verts.layers.float:
        lay = bm.verts.layers.float.new("rag")
        for v in bm.verts:
            v[lay] = 1.0
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.shade_smooth()
    ob = bpy.data.objects.new(name, me)
    COLL.objects.link(ob)
    ob.data.materials.append(MATS[mat])
    if subsurf:
        m = ob.modifiers.new("sub", "SUBSURF")
        m.levels = m.render_levels = subsurf
    if solidify:
        m = ob.modifiers.new("solid", "SOLIDIFY")
        m.thickness = solidify
        m.offset = 0.0
    group = "eyes" if mat == "eye" else MAT_SPECS[mat][0]
    PARTS.append((ob, group, weights))
    return ob


def chain(bones, cuts, width=0.06):
    """Weights down a vertical chain: bones[0] above cuts[0], and so on."""

    def f(co):
        out, rest = {}, 1.0
        for b, c in zip(bones, cuts):
            up = sstep(c - width, c + width, co.z)
            if rest * up > 1e-3:
                out[b] = rest * up
            rest *= 1.0 - up
        if rest > 1e-3:
            out[bones[-1]] = out.get(bones[-1], 0.0) + rest
        return out

    return f


def by_side(make):
    def f(co):
        return make(1 if co.x >= 0 else -1)(co)

    return f


def leg_chain(s):
    return chain(["Torso", "Hip" + SIDES[s], "Knee" + SIDES[s]], [HIP_Y - 0.04, HIP_Y - THIGH], 0.07)


def arm_chain(s):
    return chain(["Torso", "Shoulder" + SIDES[s], "Elbow" + SIDES[s]], [SHOULDER_Y - 0.02, SHOULDER_Y - UPPER_ARM], 0.06)


def body_weights(co):
    if co.z > 2.36 and abs(co.x) > 0.13:
        s = 1 if co.x > 0 else -1
        w = sstep(0.13, 0.2, abs(co.x)) * sstep(2.36, 2.45, co.z)
        base = chain(["Head", "Torso"], [2.64], 0.05)(co)
        return {**{b: v * (1 - w) for b, v in base.items()}, "Shoulder" + SIDES[s]: w}
    if co.z < HIP_Y - 0.05:
        return leg_chain(1 if co.x >= 0 else -1)(co)
    return chain(["Head", "Torso"], [2.64], 0.05)(co)


def coat_weights(co):
    base = chain(["Torso", "Coat"], [HIP_Y - 0.06], 0.1)(co)
    if co.z > 2.3 and abs(co.x) > 0.16:
        s = 1 if co.x > 0 else -1
        w = 0.6 * sstep(0.16, 0.23, abs(co.x))
        base = {b: v * (1 - w) for b, v in base.items()}
        base["Shoulder" + SIDES[s]] = w
    return base


# ---- torso, neck, head
TZ = [1.46, 1.58, 1.70, 1.82, 1.94, 2.04, 2.14, 2.26, 2.36, 2.44, 2.51, 2.58]
TRX = [0.135, 0.15, 0.125, 0.096, 0.1, 0.13, 0.148, 0.156, 0.165, 0.182, 0.13, 0.055]
TRY = [0.092, 0.098, 0.082, 0.066, 0.072, 0.094, 0.104, 0.108, 0.103, 0.09, 0.07, 0.05]
TORSO_PATH = [(0, 0.0, 1.56), (0, 0.0, 1.66), (0, 0.012, 1.9), (0, 0.03, 2.15), (0, 0.045, 2.35), (0, 0.035, 2.5), (0, 0.015, 2.6)]


def torso_radius(c):
    z, th = c.z, c.th
    r = ellipse(th, tab(z, TZ, TRY), tab(z, TZ, TRX))
    front, side = max(0.0, math.cos(th)), abs(math.sin(th))
    # a hollow belly under the ribs
    r *= 1 - 0.2 * front**2 * sstep(1.7, 1.8, z) * (1 - sstep(1.96, 2.06, z))
    lo = 2.08 - 0.1 * side
    ribs = sstep(lo - 0.01, lo + 0.02, z) * (1 - sstep(2.34, 2.41, z)) * sstep(-0.35, 0.25, math.cos(th))
    phase = (z + 0.07 * side**1.4) / 0.052
    rib = max(0.0, math.sin(TAU * phase)) ** 1.6
    r += ribs * (0.0075 * rib - 0.004)
    r -= 0.007 * ribs * gauss(adiff(th, 0.0), 0.13)
    r += 0.006 * gauss(z - lo, 0.012) * sstep(-0.2, 0.4, math.cos(th))
    # collarbones, spine and shoulder blades
    r += 0.01 * gauss(z - (2.45 - 0.025 * side), 0.013) * sstep(0.0, 0.5, math.cos(th))
    back = adiff(th, math.pi)
    r += 0.009 * gauss(back, 0.12) * (0.55 + 0.45 * math.sin(TAU * z / 0.034)) * (1 - sstep(2.45, 2.55, z))
    for sgn in (-1, 1):
        r += 0.013 * gauss(adiff(th, math.pi + sgn * 0.85), 0.35) * gauss(z - 2.3, 0.07)
    # pelvis crests
    for sgn in (-1, 1):
        r += 0.01 * gauss(adiff(th, sgn * 1.1), 0.3) * gauss(z - 1.7, 0.03)
    return r


def torso_disp(c):
    return c.d * 0.0025 * fbm(c.q, 9.0, 2, 2.0)


def build_body():
    bm = tube(TORSO_PATH, 92, 40, torso_radius, disp=torso_disp, ru=0.16, cap=True)
    part("torso", bm, "skin", body_weights)

    def neck_r(c):
        r = 0.042 - 0.012 * c.t
        for sgn in (-1, 1):
            r += 0.005 * gauss(adiff(c.th, sgn * 0.6), 0.3) * (1 - c.t * 0.3)
        r += 0.006 * gauss(adiff(c.th, 0), 0.25) * gauss(c.z - 2.61, 0.02)
        return r

    bm = tube([(0, 0.03, 2.47), (0, 0.015, 2.56), (0, -0.004, 2.63), (0, -0.016, 2.69)], 18, 16, neck_r, ru=0.05)
    part("neck", bm, "face", body_weights)
    build_head()


HZ = [2.62, 2.655, 2.70, 2.745, 2.79, 2.835, 2.875, 2.90]
HRY = [0.016, 0.052, 0.07, 0.082, 0.088, 0.084, 0.06, 0.012]
HRX = [0.016, 0.042, 0.055, 0.066, 0.075, 0.074, 0.056, 0.012]
HEAD_PATH = [(0, -0.058, 2.62), (0, -0.046, 2.66), (0, -0.028, 2.72), (0, -0.015, 2.78), (0, -0.008, 2.84), (0, -0.004, 2.9)]
EYE_Z, EYE_TH = 2.768, 0.44


def head_radius_at(z, th):
    r = ellipse(th, tab(z, HZ, HRY), tab(z, HZ, HRX))
    for sgn in (-1, 1):
        r -= 0.02 * gauss(adiff(th, sgn * EYE_TH), 0.2) * gauss(z - EYE_Z, 0.019)
        r -= 0.012 * gauss(adiff(th, sgn * 0.95), 0.3) * gauss(z - 2.715, 0.025)
        r += 0.006 * gauss(adiff(th, sgn * 0.8), 0.25) * gauss(z - 2.745, 0.01)
    r += 0.007 * gauss(adiff(th, 0), 0.9) * gauss(z - 2.792, 0.009)
    r -= 0.012 * gauss(adiff(th, 0), 0.12) * gauss(z - 2.735, 0.015)
    r -= 0.004 * gauss(adiff(th, 0), 0.5) * gauss(z - 2.688, 0.004)
    return r


def build_head():
    pts, _ = resample(HEAD_PATH, 40)
    bm = tube(HEAD_PATH, 40, 36, lambda c: head_radius_at(c.z, c.th), ru=0.07, cap=True)
    part("head", bm, "face", "Head")
    # the eyes: small, deep in the sockets, pale and lit
    zs = [p.z for p in pts]
    i = min(range(len(pts)), key=lambda j: abs(zs[j] - EYE_Z))
    p = pts[i]
    for sgn in (-1, 1):
        th = sgn * EYE_TH
        d = FRONT * math.cos(th) + V(1, 0, 0) * math.sin(th)
        centre = p + d * (head_radius_at(p.z, th) - 0.001)
        eb = bmesh.new()
        bmesh.ops.create_uvsphere(eb, u_segments=12, v_segments=8, radius=0.0105, matrix=Matrix.Translation(centre))
        eb.loops.layers.uv.new("Proc")
        part("eye_" + SIDES[sgn], eb, "eye", "Head")


# ---- arms and hands
AZ = [1.02, 1.1, 1.3, 1.5, 1.66, 1.74, 1.82, 2.0, 2.2, 2.38, 2.46, 2.52]
AR = [0.021, 0.024, 0.03, 0.033, 0.031, 0.034, 0.029, 0.03, 0.033, 0.042, 0.046, 0.034]


def arm_path(s):
    return [
        (s * 0.15, 0.012, 2.5),
        (s * 0.205, 0.004, 2.46),
        (s * 0.224, 0.0, 2.3),
        (s * 0.234, 0.0, 2.0),
        (s * 0.245, 0.0, 1.74),
        (s * 0.256, -0.008, 1.45),
        (s * 0.266, -0.014, 1.12),
        (s * 0.27, -0.016, 1.02),
    ]


def build_arm(s):
    def r(c):
        rr = tab(c.z, AZ, AR)
        rr += 0.011 * gauss(adiff(c.th, math.pi), 0.5) * gauss(c.z - 1.74, 0.03)
        rr += 0.004 * gauss(adiff(c.th, math.pi), 0.35) * sstep(1.72, 1.2, c.z) * (1 - sstep(1.2, 1.03, c.z))
        rr *= 1 - 0.22 * abs(math.sin(c.th)) * sstep(1.6, 1.3, c.z)
        return rr

    bm = tube(arm_path(s), 64, 16, r, disp=torso_disp, ru=0.04)
    part("arm_" + SIDES[s], bm, "skin", arm_chain(s))
    build_hand(s)


def claw(tip, direction, s, name, length=0.088, r0=0.0092):
    pts = [tip - direction * 0.012]
    d = direction.copy()
    p = tip - direction * 0.012
    for _ in range(6):
        p = p + d * (length + 0.012) / 6
        pts.append(p.copy())
        d = Matrix.Rotation(s * 0.25, 3, "Y") @ d
    bm = tube(pts, 20, 10, lambda c: max(r0 * (1 - c.t) ** 0.9, 0.0004) * (1 + 0.22 * math.cos(c.th)),
              ref=V(-s, 0, 0), ru=0.01, cap=True, raw=False)
    part(name, bm, "claw", "Elbow" + SIDES[s])


def finger(base, spread, lengths, bends, s, name, rad=0.0115, fwd=None):
    d = fwd.copy() if fwd else V(0, math.sin(spread), -math.cos(spread))
    pts = [base + V(0, 0, 0.025), base]
    joints = []
    p = base.copy()
    for ln, a in zip(lengths, bends):
        d = Matrix.Rotation(s * a, 3, "Y") @ d
        p = p + d * ln
        pts.append(p.copy())
        joints.append(p.copy())
    total = sum(lengths) + 0.025
    jt = []
    acc = 0.025
    for ln in lengths[:-1]:
        acc += ln
        jt.append(acc / total)

    def r(c):
        rr = lerp(rad, rad * 0.66, c.t)
        for t in jt:
            rr += rad * 0.45 * gauss(c.t - t, 0.035)
            rr -= rad * 0.12 * (gauss(c.t - t + 0.09, 0.04) + gauss(c.t - t - 0.09, 0.04))
        rr += rad * 0.2 * gauss(c.t - 0.07, 0.05)
        return rr * (1 + 0.09 * math.cos(4 * c.th))

    bm = tube(pts, 40, 12, r, ref=V(s, 0, 0), ru=0.02, cap=True)
    part(name, bm, "skin", "Elbow" + SIDES[s])
    claw(p, d, s, name + "_claw")


def build_hand(s):
    wrist = V(s * 0.27, -0.016, 1.02)
    knuck_z = 0.9

    back = -s * math.pi / 2  # the back of the hand faces away from the body (binormal is -X)

    def palm_r(c):
        rr = ellipse(c.th, 0.022 + 0.012 * sstep(0.0, 0.55, c.t), 0.016 - 0.003 * sstep(0.3, 1.0, c.t))
        for off in (-0.55, -0.18, 0.18, 0.55):
            a = adiff(c.th, back + off)
            rr += 0.003 * gauss(a, 0.09) * sstep(0.15, 0.5, c.t)  # tendons
            rr += 0.007 * gauss(a, 0.2) * gauss(c.t - 0.95, 0.06)  # knuckles
        rr += 0.005 * gauss(adiff(c.th, back + 0.9), 0.35) * gauss(c.t - 0.2, 0.08)  # the wrist bone
        return rr

    bm = tube([wrist + V(0, 0, 0.04), wrist, V(s * 0.272, -0.02, 0.96), V(s * 0.272, -0.022, knuck_z + 0.012)], 22,
              28, palm_r, ru=0.03, cap=True)
    part("palm_" + SIDES[s], bm, "skin", "Elbow" + SIDES[s])
    for i, y in enumerate((-0.031, -0.012, 0.007, 0.025)):
        base = V(s * 0.276, -0.016 + (y + 0.016) * 0.35, 1.0)
        head = V(s * 0.279, y, knuck_z + 0.004)

        def mr(c):
            return 0.0068 + 0.0032 * gauss(c.t - 0.93, 0.08) + 0.0016 * gauss(c.t - 0.05, 0.1)

        bm = tube([base, base.lerp(head, 0.5) + V(s * 0.003, 0, 0), head], 14, 8, mr, ref=V(s, 0, 0), ru=0.02,
                  cap=True)
        part(f"metacarpal{i}_{SIDES[s]}", bm, "skin", "Elbow" + SIDES[s])
    ys = [-0.049, -0.03, -0.011, 0.007]  # + 0.018: the metacarpal heads
    lens = [0.19, 0.215, 0.2, 0.162]
    spreads = [-0.1, -0.035, 0.03, 0.09]
    for i in range(4):
        L = lens[i]
        bends = [0.16 + 0.06 * i + RNG.uniform(-0.05, 0.05), 0.42 + RNG.uniform(-0.08, 0.08),
                 0.36 + RNG.uniform(-0.06, 0.06)]
        finger(V(s * 0.277, ys[i] + 0.018, knuck_z), spreads[i], [L * 0.42, L * 0.32, L * 0.26], bends, s,
               f"finger{i}_{SIDES[s]}", rad=0.0122 - 0.0007 * i)
    thumb_dir = V(-s * 0.35, -0.55, -0.76).normalized()
    finger(V(s * 0.262, -0.05, 0.985), 0.0, [0.075, 0.06], [0.1, 0.35], s, "thumb_" + SIDES[s], rad=0.0125,
           fwd=thumb_dir)


# ---- legs, pants, belt, boots
PZ = [0.5, 0.6, 0.7, 0.84, 1.0, 1.2, 1.45, 1.7]
PR = [0.052, 0.062, 0.061, 0.066, 0.064, 0.072, 0.082, 0.09]


def build_legs():
    for s in (1, -1):
        path = [(s * 0.075, 0.0, 1.72), (s * 0.09, 0.0, 1.5), (s * 0.1, 0.0, 1.2), (s * 0.1, -0.004, 0.84),
                (s * 0.1, 0.0, 0.62), (s * 0.1, 0.0, 0.5)]

        def pr(c):
            return tab(c.z, PZ, PR)

        def pdisp(c, s=s):
            n = fbm(c.q, 7.0, 3, 5.0 + s)
            bunch = 0.006 * math.sin(c.z * 70 + 3 * n) * sstep(0.85, 0.62, c.z)
            fold = 0.004 * math.sin(c.th * 3 + c.z * 11 + n)
            return c.d * (0.007 * n + bunch + fold)

        bm = tube(path, 70, 22, pr, disp=pdisp, ru=0.07)
        if s == 1:
            def knee_hole(v):
                q = v.co
                front = -q.y / max(math.hypot(q.x - 0.1, q.y), 1e-6)
                return max((0.3 - front) * 0.12, abs(q.z - 0.84) - 0.05)

            cull(bm, set_rag(bm, knee_hole))
        part("pants_" + SIDES[s], bm, "pants", leg_chain(s))
        if s == 1:
            def knee_r(c):
                return 0.048 + 0.012 * gauss(adiff(c.th, 0), 0.6) * gauss(c.z - 0.84, 0.035)

            bm = tube([(0.1, 0.0, 1.0), (0.1, -0.004, 0.84), (0.1, 0.0, 0.68)], 20, 14, knee_r, ru=0.05)
            part("knee", bm, "skin", leg_chain(1))
        build_boot(s)

    def pelvis_r(c):
        z = c.z
        rx = tab(z, [1.44, 1.52, 1.6, 1.7, 1.76], [0.09, 0.148, 0.166, 0.168, 0.158])
        ry = tab(z, [1.44, 1.52, 1.6, 1.7, 1.76], [0.07, 0.1, 0.11, 0.112, 0.104])
        r = ellipse(c.th, ry, rx)
        r += 0.004 * math.sin(c.th * 6 + z * 30) * sstep(1.66, 1.74, z)  # a gathered waistband
        return r

    bm = tube([(0, 0.0, 1.44), (0, 0.0, 1.6), (0, 0.0, 1.76)], 24, 36, pelvis_r, disp=torso_disp, ru=0.15, cap=True)
    part("pelvis", bm, "pants", by_side(leg_chain))
    # a cloth sash for a belt, knotted at the front
    ring = [V(0.178 * math.sin(a), -0.125 * math.cos(a), 1.665 - 0.012 * math.cos(a)) for a in
            [TAU * i / 48 for i in range(49)]]
    bm = tube(ring, 49, 12, lambda c: ellipse(c.th, 0.02, 0.008) * (1 + 0.1 * nz(c.q, 30.0)), ref=UP, loop=True,
              raw=True, ru=0.03, transport=False)
    part("sash", bm, "sash", "Torso")
    knot = V(0.07, -0.135, 1.66)
    bm = tube([knot + V(-0.02, 0.005, 0.0), knot, knot + V(0.02, 0.005, -0.005)], 8, 10,
              lambda c: 0.022 * math.sin(math.pi * (0.1 + 0.8 * c.t)), ref=UP, ru=0.03, cap=True)
    part("sash_knot", bm, "sash", "Torso")
    for i, (dx, ln) in enumerate(((0.0, 0.5), (0.03, 0.36))):
        start = knot + V(dx, -0.008, -0.01)
        ctrl = [start, start + V(0.01, -0.02, -ln * 0.3), start + V(0.02 + dx, -0.025, -ln * 0.7),
                start + V(0.015, -0.02, -ln)]
        part(f"sash_end{i}", ribbon(ctrl, 0.045, 14, twist=0.8, seed=11 + i), "sash", "Coat")


def build_boot(s):
    x = s * 0.1

    def shaft_r(c):
        z = c.z
        r = tab(z, [0.1, 0.16, 0.3, 0.45, 0.6, 0.69], [0.062, 0.059, 0.06, 0.064, 0.07, 0.078])
        n = fbm(c.q, 9.0, 2, 13.0 + s)
        r += 0.0045 * math.sin(z * 75 + 4 * n + c.th) * sstep(0.45, 0.2, z)
        r += 0.003 * n
        r *= 1 + 0.06 * math.cos(c.th - s * 0.8) * sstep(0.5, 0.68, z)
        return r

    bm = tube([(x, 0.0, 0.69), (x, 0.0, 0.5), (x, 0.004, 0.3), (x, 0.01, 0.16), (x, 0.015, 0.1)], 48, 26, shaft_r,
              ru=0.07)
    part("boot_shaft_" + SIDES[s], bm, "leather", "Knee" + SIDES[s], solidify=0.006)
    f = Matrix.Rotation(-s * 0.1, 3, "Z") @ FRONT
    base = V(x, 0.015, 0.0)

    def at(a, h):
        return base + f * a + UP * h

    foot = [at(-0.06, 0.1), at(-0.03, 0.075), at(0.06, 0.058), at(0.15, 0.05), at(0.22, 0.045), at(0.275, 0.036)]
    fa = [-0.06, -0.03, 0.06, 0.15, 0.22, 0.275]
    up_r = [0.06, 0.052, 0.04, 0.032, 0.027, 0.02]
    side_r = [0.052, 0.056, 0.053, 0.054, 0.046, 0.032]

    def foot_r(c):
        a = lerp(fa[0], fa[-1], c.t)
        r = ellipse(c.th, tab(a, fa, up_r), tab(a, fa, side_r))
        if math.cos(c.th) < 0:
            r *= 1 - 0.3 * (-math.cos(c.th)) ** 2
        r += 0.002 * fbm(c.q, 20.0, 2, 17.0)
        return r

    bm = tube(foot, 30, 22, foot_r, ref=UP, ru=0.06, cap=True)
    part("boot_foot_" + SIDES[s], bm, "leather", "Knee" + SIDES[s])
    sole = [at(-0.07, 0.014), at(0.0, 0.014), at(0.15, 0.014), at(0.29, 0.016)]
    bm = tube(sole, 24, 16, lambda c: ellipse(c.th, 0.013, tab(c.t, [0, 0.3, 0.8, 1], [0.048, 0.054, 0.048, 0.026])),
              ref=UP, ru=0.05, cap=True)
    part("boot_sole_" + SIDES[s], bm, "leather", "Knee" + SIDES[s])
    heel = [at(-0.045, 0.0), at(-0.045, 0.05)]
    bm = tube(heel, 4, 16, lambda c: 0.037 * (1 + 0.1 * math.cos(4 * c.th)), ref=f, ru=0.04, cap=True)
    part("boot_heel_" + SIDES[s], bm, "leather", "Knee" + SIDES[s])


# ---- the rags: an open shirt/coat, sleeves, a tan cape, loose strips
def set_rag(bm, fn):
    """Write the "rag" field (metres; negative where the cloth is torn away)."""
    lay = bm.verts.layers.float.get("rag") or bm.verts.layers.float.new("rag")
    for v in bm.verts:
        v[lay] = fn(v)
    return lay


def cull(bm, lay, margin=-0.08):
    """Drop faces the fraying noise can never bring back."""
    dead = [f for f in bm.faces if all(v[lay] < margin for v in f.verts)]
    bmesh.ops.delete(bm, geom=dead, context="FACES")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")


def holes(threshold, top, seed):
    def f(co):
        if co.z > top:
            return 1.0
        return (threshold - fbm(co, 5.0, 3, seed)) * 0.25 + 0.2 * sstep(top - 0.15, top, co.z)

    return f


def strips_plan(cols, split_lo, split_hi, hem_lo, hem_hi, p_split, seed, wmin=3, wmax=6):
    """Columns grouped into strips: per column its strip; per strip (hem, tear top)."""
    rng = random.Random(seed)
    col_sid, strips = [], []
    while len(col_sid) < cols:
        w = rng.randint(wmin, wmax)
        tear = rng.uniform(split_lo, split_hi) if rng.random() < p_split else -9.0
        strips.append((rng.uniform(hem_lo, hem_hi), tear))
        col_sid += [len(strips) - 1] * w
    return col_sid[:cols], strips


def rag(bm, col_sid, strips, axis, hole=None, extra=None, sway=0.05, seed=0.0, fray_below=2.35):
    """Tear a lofted sheet into hanging strips and write its "rag" field.

    The edge between two strips splits below the left one's tear top, and
    strips free on both sides drift apart as they hang. The field is the
    height above the strip's hem, fraying along free edges, with moth holes.
    """
    cl = bm.faces.layers.int["col"]

    def sid_of(f):
        return col_sid[min(f[cl], len(col_sid) - 1)]

    cut = []
    for e in bm.edges:
        if len(e.link_faces) != 2:
            continue
        a, b = (sid_of(f) for f in e.link_faces)
        if a != b and max(v.co.z for v in e.verts) < strips[min(a, b)][1] + 1e-4:
            cut.append(e)
    bmesh.ops.split_edges(bm, edges=cut)
    lay = bm.verts.layers.float.new("rag")
    ax = Vector((axis[0], axis[1]))
    first = {}
    for j, sid in enumerate(col_sid):
        first.setdefault(sid, [j, j])[1] = j + 1
    for v in bm.verts:
        f0 = v.link_faces[0] if v.link_faces else None
        s = sid_of(f0) if f0 else 0
        hem, tear = strips[s]
        left = strips[s - 1][1] if s > 0 else -9.0
        j0, j1 = first[s]
        col = min(f[cl] for f in v.link_faces) if f0 else j0
        u = clamp((col + (0.5 if v.is_boundary else 0.0) - j0) / max(j1 - j0, 1))
        shape = (s * 7919) % 3
        if shape == 0:
            cut = 0.14 * abs(u - 0.5) * 2  # pointed tail
        elif shape == 1:
            cut = 0.12 * (u if s % 2 else 1 - u)  # slanted tear
        else:
            cut = 0.08 * (1 - abs(u - 0.5) * 2)  # notched
        r = v.co.z - hem - cut + 0.05 * nz(v.co, 6.0, seed + s)
        if v.is_boundary and v.co.z < fray_below:
            r = min(r, 0.012)
        if hole:
            r = min(r, hole(v.co))
        if extra:
            r = min(r, extra(v.co))
        v[lay] = r
        free = min(tear, left)
        if free > -5:
            depth = clamp((free - v.co.z) / 0.5)
            out = Vector((v.co.x, v.co.y)) - ax
            out = out.normalized() if out.length > 1e-6 else Vector((0, 1))
            side = Vector((-out.y, out.x))
            a = nz((s * 1.37, seed, 0.3), 1.0) * sway * depth
            b = nz((s * 1.37, seed, 5.1), 1.0) * sway * depth
            off = out * (abs(a) * 0.8) + side * b
            v.co += V(off.x, off.y, 0.0)
    cull(bm, lay)


def build_coat():
    cols = 72
    col_sid, strips = strips_plan(cols, 1.2, 1.6, 0.9, 1.22, 0.7, 21)
    path = [(0, 0.02, 2.62), (0, 0.035, 2.45), (0, 0.042, 2.2), (0, 0.03, 1.95), (0, 0.02, 1.66), (0, 0.03, 1.35),
            (0, 0.05, 1.0), (0, 0.06, 0.78)]

    def gap(z):
        if z > 2.3:
            return lerp(0.8, 1.2, sstep(2.3, 2.6, z))
        if z > HIP_Y:
            return lerp(0.55, 0.8, sstep(HIP_Y, 2.3, z))
        return lerp(0.55, 0.75, sstep(HIP_Y, 0.9, z))

    n = 110
    pts, _ = resample(path, n)
    zs = [p.z for p in pts]

    def span(t):
        g = gap(zs[min(int(round(t * (n - 1))), n - 1)])
        return (g, TAU - g)

    def r(c):
        z = c.z
        if z > 2.44:
            rx = 0.07 + 0.172 * math.sqrt(max(0.0, 1 - ((z - 2.44) / 0.17) ** 2))
            ry = tab(z, TZ, TRY) + 0.025
        elif z > HIP_Y:
            rx = max(tab(z, TZ, TRX) + 0.024, 0.145)
            ry = max(tab(z, TZ, TRY) + 0.024, 0.105)
        else:
            rx = 0.18 + (HIP_Y - z) * 0.11
            ry = 0.128 + (HIP_Y - z) * 0.09
        rr = ellipse(c.th, ry, rx)
        g = gap(z)
        edge = min(c.th - g, TAU - g - c.th) * rr
        rr += 0.014 * math.exp(-edge / 0.025) * sstep(1.6, 2.2, z)
        return rr

    def disp(c):
        low = sstep(1.62, 1.0, c.z)
        n_ = fbm(c.q, 5.0, 3, 31.0)
        fold = (0.005 + 0.012 * low) * math.sin(c.th * 9 + 2 * n_ + c.z * 3)
        crease = 0.004 * math.sin(c.z * 40 + 5 * n_) * sstep(2.3, 1.9, c.z) * (1 - low)
        return c.d * (0.009 * n_ + fold + crease)

    bm = tube(path, n, cols, r, span=span, closed=False, disp=disp, ru=0.19)
    rag(bm, col_sid, strips, (0.0, 0.03), hole=holes(0.62, 2.2, 35.0), sway=0.05, seed=3.0)
    part("coat", bm, "coat", coat_weights)

    for s in (1, -1):
        col_sid, strips = strips_plan(20, 1.45, 1.62, 1.42, 1.6, 0.5, 40 + s, 2, 4)
        sp = [(s * 0.16, 0.014, 2.53), (s * 0.21, 0.004, 2.47), (s * 0.228, 0.0, 2.3), (s * 0.238, 0.0, 2.0),
              (s * 0.248, 0.0, 1.74), (s * 0.258, -0.008, 1.45), (s * 0.262, -0.012, 1.36)]

        def sr(c):
            return tab(c.z, [1.36, 1.5, 1.74, 2.2, 2.45, 2.53], [0.062, 0.056, 0.052, 0.052, 0.058, 0.04])

        def sdisp(c, s=s):
            n_ = fbm(c.q, 7.0, 3, 50.0 + s)
            return c.d * (0.008 * n_ + 0.006 * math.sin(c.th * 5 + c.z * 18 + n_))

        bm = tube(sp, 70, 20, sr, disp=sdisp, ru=0.05)
        rag(bm, col_sid, strips, (s * 0.25, 0.0), hole=holes(0.66, 2.3, 45.0 + s), sway=0.02, seed=7.0 + s)
        part("sleeve_" + SIDES[s], bm, "coat", arm_chain(s))


def build_shawl():
    """A tan cape over the shoulders and upper arms, one long ragged back panel."""
    cols = 84
    col_sid, strips = strips_plan(cols, 1.75, 2.15, 1.3, 1.7, 0.6, 61)
    path = [(0, 0.03, 2.67), (0, 0.035, 2.55), (0, 0.045, 2.4), (0, 0.055, 2.1), (0, 0.065, 1.7), (0, 0.075, 1.25)]
    n = 72
    pts, _ = resample(path, n)
    zs = [p.z for p in pts]

    def span(t):
        z = zs[min(int(round(t * (n - 1))), n - 1)]
        g = lerp(0.85, 1.3, sstep(2.4, 2.64, z))
        return (g, TAU - g)

    def r(c):
        z = c.z
        if z > 2.42:
            k = math.sqrt(max(0.0, 1 - ((z - 2.42) / 0.25) ** 2))
            rx = 0.085 + 0.225 * k
            ry = 0.075 + 0.09 * k
        else:
            rx = lerp(0.31, 0.2, sstep(2.3, 2.0, z))
            ry = 0.165 + 0.02 * (2.42 - z)
        if math.cos(c.th) < 0:  # the back lies flatter, hanging from the shoulder blades
            ry *= 1 - 0.3 * (-math.cos(c.th)) * sstep(2.6, 2.35, z)
        return ellipse(c.th, ry, rx)

    def disp(c):
        low = sstep(2.38, 1.6, c.z)
        n_ = fbm(c.q, 4.0, 3, 71.0)
        fold = (0.01 + 0.014 * low) * math.sin(c.th * 11 + 2.5 * n_ + 2 * c.z)
        droop = -0.02 * sstep(2.42, 2.3, c.z) * abs(math.sin(c.th))
        return c.d * (0.01 * n_ + fold + droop)

    def back_only(co):
        below = co.z - (2.24 + 0.03 * nz(co, 9.0, 4.0))
        back = min(co.y - 0.02, 0.19 - abs(co.x)) * 0.8
        return max(below, back)

    bm = tube(path, n, cols, r, span=span, closed=False, disp=disp, ru=0.25)
    rag(bm, col_sid, strips, (0.0, 0.05), hole=holes(0.6, 2.3, 75.0), extra=back_only, sway=0.05, seed=9.0,
        fray_below=2.5)
    part("shawl", bm, "tan", coat_weights)


def hang(name, start, ln, width, mat, weights, seed, out=None, twist=0.0):
    out = out if out is not None else V(start.x, start.y - 0.03, 0).normalized()
    ctrl = [start + V(0, 0, 0.03), start, start + out * 0.015 + V(0, 0, -ln * 0.35),
            start + out * 0.03 + V(0.005, 0.0, -ln * 0.7), start + out * 0.025 + V(0.0, 0.005, -ln)]
    part(name, ribbon(ctrl, width, max(6, int(ln / 0.035)), twist=twist, seed=seed), mat, weights)


def build_loose_strips():
    rng = random.Random(81)
    # from the cape's front edge, down over the coat's lapels
    for i in range(10):
        s = 1 if i % 2 else -1
        x = s * rng.uniform(0.1, 0.175)
        z0 = rng.uniform(2.26, 2.36)
        start = V(x, -0.135 - 0.02 * rng.random(), z0)
        hang(f"strip_f{i}", start, rng.uniform(0.35, 0.75), rng.uniform(0.03, 0.055), "tan", coat_weights, 100 + i,
             out=V(s * 0.2, -1, 0).normalized(), twist=rng.uniform(-1, 1))
    # from the back panel's hem
    for i in range(6):
        x = rng.uniform(-0.17, 0.17)
        start = V(x, 0.2 + 0.02 * rng.random(), rng.uniform(1.4, 1.6))
        hang(f"strip_b{i}", start, rng.uniform(0.2, 0.4), rng.uniform(0.03, 0.05), "tan", coat_weights, 150 + i,
             out=V(0, 1, 0), twist=rng.uniform(-1, 1))
    # from the coat's hem
    for i in range(12):
        a = rng.uniform(0.7, TAU - 0.7)
        z0 = rng.uniform(1.0, 1.2)
        rr = 0.18 + (HIP_Y - z0) * 0.11 + 0.015
        start = V(rr * math.sin(a), -rr * 0.75 * math.cos(a) + 0.03, z0)
        hang(f"strip_c{i}", start, rng.uniform(0.2, 0.4), rng.uniform(0.03, 0.05), "coat", coat_weights, 200 + i,
             twist=rng.uniform(-1, 1))
    # from the sleeve cuffs
    for s in (1, -1):
        for i in range(4):
            a = rng.uniform(0, TAU)
            start = V(s * 0.257 + 0.058 * math.cos(a), -0.01 + 0.058 * math.sin(a), 1.52)
            hang(f"strip_a{SIDES[s]}{i}", start, rng.uniform(0.15, 0.3), 0.03, "coat", arm_chain(s),
                 300 + i + 10 * s, out=V(math.cos(a), math.sin(a), 0), twist=0.5)



# ---- hair
def build_hair():
    rng = random.Random(91)
    angles = [rng.uniform(0.75, TAU - 0.75) for _ in range(64)]
    angles += [0.0, 0.2, -0.2, 0.65, -0.65, 0.1, -0.1, 0.8, -0.8, 0.28, -0.28, 0.06, -0.14, 0.72, -0.72, 0.9, -0.9]
    for i, th in enumerate(angles):
        front = math.cos(th) > 0.3
        root_z = 2.83 + rng.uniform(-0.01, 0.02)
        ln = rng.uniform(0.18, 0.3) if front else rng.uniform(0.28, 0.5)
        pts = []
        for k in range(8):
            z = root_z - ln * k / 7
            hz = clamp(z, 2.64, 2.9)
            d = FRONT * math.cos(th) + V(1, 0, 0) * math.sin(th)
            rr = head_radius_at(hz, th) if z > 2.64 else 0.06
            centre_y = tab(hz, [2.62, 2.66, 2.72, 2.78, 2.84, 2.9], [-0.058, -0.046, -0.028, -0.015, -0.008, -0.004])
            if th == 0.0 or abs(math.sin(th)) < 0.25:
                rr = max(rr, 0.1)
            base = V(0, centre_y, z) + d * (rr + 0.012 + 0.03 * (k / 7) ** 1.5)
            base += V(nz((i, k * 0.4, 1.0), 1.0) * 0.012, nz((i, k * 0.4, 2.0), 1.0) * 0.012, 0)
            pts.append(base)
        part(f"hair{i}", ribbon(pts, rng.uniform(0.008, 0.016), 10, twist=rng.uniform(-2, 2), seed=400 + i,
                                taper=0.85, ref=FRONT, fray=False), "hair", "Head")


# ---- the sombrero
HAT_POS = V(0, -0.012, 2.83)
HAT_TILT = 0.1


def hat_matrix():
    return Matrix.Translation(HAT_POS) @ Matrix.Rotation(HAT_TILT, 4, "X")


def build_hat():
    m = hat_matrix()
    rows, uvs = [], []
    nr, nt = 26, 104
    r0, r1 = 0.13, 0.62
    edge = [0.62 + 0.018 * nz((math.cos(TAU * j / nt) * 3, math.sin(TAU * j / nt) * 3, 0.5)) for j in range(nt + 1)]
    edge[nt] = edge[0]
    for i in range(nr + 1):
        row, uvr = [], []
        for j in range(nt + 1):
            th = TAU * j / nt
            rmax = edge[j]
            u = i / nr
            r = lerp(r0, rmax, u)
            w = (r - 0.14) / 0.48
            cz = -0.105 * max(w, 0.0) ** 1.35
            cz += 0.04 * max(w, 0) ** 2 * nz((math.cos(th) * 1.4, math.sin(th) * 1.4, 3.0))
            cz += 0.012 * max(w, 0) ** 3 * math.sin(th * 7 + 1.0)
            p = V(r * math.sin(th), -r * math.cos(th), cz)
            uvr.append((th * r, r))
            if j < nt:
                row.append(m @ p)
        rows.append(row)
        uvs.append(uvr)

    bm = grid(rows, uvs, closed=True)
    bm.verts.ensure_lookup_table()
    lay = bm.verts.layers.float.new("rag")
    for idx, v in enumerate(bm.verts):
        i, j = divmod(idx, nt)
        v[lay] = (edge[j] - lerp(r0, edge[j], i / nr)) + 0.008
    part("hat_brim", bm, "straw", "Head", solidify=0.012)

    def crown_r(c):
        lz = c.t * 0.215 - 0.02
        r = 0.152 - 0.022 * sstep(-0.02, 0.18, lz)
        r *= 1.0 - 0.12 * sstep(0.17, 0.195, lz) ** 2  # the rolled edge of a flat top
        r += 0.004 * nz(c.q, 18.0, 4.0) + 0.0012 * math.sin(c.t * 60)
        return r

    def crown_disp(c):
        lz = c.t * 0.215 - 0.02
        return UP * (-0.012 * gauss(math.sin(c.th), 0.35) * sstep(0.14, 0.195, lz))

    ctrl = [m @ V(0, 0, -0.02 + 0.215 * k / 4) for k in range(5)]
    bm = tube(ctrl, 40, 60, crown_r, disp=crown_disp, ref=m.to_3x3() @ FRONT, ru=0.14, cap=True)
    part("hat_crown", bm, "straw", "Head")
    ring = [m @ V(0.152 * math.sin(a), -0.152 * math.cos(a), 0.018) for a in [TAU * i / 48 for i in range(49)]]
    bm = tube(ring, 49, 8, lambda c: ellipse(c.th, 0.012, 0.005), ref=m.to_3x3() @ UP, loop=True, raw=True, ru=0.02,
              transport=False)
    part("hat_band", bm, "rope", "Head")
    # frayed straw sticking out of the rim
    rng = random.Random(101)
    for i in range(240):
        j = rng.randrange(nt)
        th = TAU * j / nt + rng.uniform(-0.02, 0.02)
        r = edge[j] - rng.uniform(0.0, 0.03)
        w = (r - 0.14) / 0.48
        cz = -0.105 * w**1.35
        start = V(r * math.sin(th), -r * math.cos(th), cz)
        # most straws hang from the ragged rim; a few stick out
        out = V(math.sin(th), -math.cos(th), rng.uniform(-3.0, 0.1)).normalized()
        out = (out + V(rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4), 0)).normalized()
        ln = rng.uniform(0.02, 0.05) if rng.random() < 0.6 else rng.uniform(0.06, 0.13)
        pts = [m @ (start - out * 0.01), m @ start, m @ (start + out * ln)]
        bm = tube(pts, 4, 3, lambda c: 0.0018 * (1 - 0.7 * c.t), ref=UP, ru=0.005)
        part(f"straw{i}", bm, "straw", "Head")


# ---- the sack, its rope, the femurs
SZ = [1.72, 1.76, 1.84, 1.98, 2.12, 2.24, 2.34, 2.41, 2.46, 2.51, 2.57]
SR = [0.02, 0.1, 0.18, 0.22, 0.212, 0.172, 0.108, 0.058, 0.05, 0.072, 0.09]
SACK_PATH = [(-0.06, 0.42, 1.72), (-0.06, 0.45, 1.84), (-0.055, 0.46, 2.02), (-0.05, 0.42, 2.2), (-0.05, 0.35, 2.36),
             (-0.05, 0.31, 2.45), (-0.05, 0.3, 2.52), (-0.05, 0.3, 2.57)]
SACK_HOLES = [(math.pi + 0.35, 2.08, 0.03), (math.pi - 0.6, 1.93, 0.022), (math.pi + 1.1, 2.2, 0.018)]


def sack_centre(z):
    return V(-0.05, tab(z, [1.72, 1.84, 2.02, 2.2, 2.36, 2.45, 2.57], [0.42, 0.45, 0.46, 0.42, 0.35, 0.31, 0.3]), z)


def build_sack():
    def r(c):
        z = c.z
        rr = tab(z, SZ, SR)
        rr = ellipse(c.th, rr * 0.8, rr)
        gather = sstep(2.26, 2.42, z) * (1 - sstep(2.46, 2.5, z))
        n = fbm(c.q, 5.0, 3, 121.0)
        rr *= 1 + 0.16 * math.sin(c.th * 9 + 2 * n) * gather
        rr *= 1 + 0.2 * math.sin(c.th * 7 + 3 * n) * sstep(2.5, 2.57, z)
        rr += 0.026 * n * (1 - gather) + 0.012 * nz(c.q, 14.0, 5.0)
        rr += 0.012 * math.sin(c.th * 5 + 4 * nz(c.q, 3.0, 6.0) + c.z * 6) * (1 - gather) * sstep(1.75, 1.95, z)
        rr += 0.006 * abs(nz(c.q, 22.0, 7.0))
        # lumps of bone pressing the burlap out
        rr += 0.02 * gauss(adiff(c.th, math.pi + 0.4), 0.4) * gauss(z - 2.02, 0.12)
        rr += 0.016 * gauss(adiff(c.th, -1.2), 0.3) * gauss(z - 1.9, 0.06)
        # squashed against the back
        if math.cos(c.th) > 0:
            rr *= 1 - 0.2 * math.cos(c.th) ** 2 * sstep(1.8, 2.0, z)
        return rr

    def torn(v):
        q = v.co
        rel = q - sack_centre(q.z)
        th = math.atan2(rel.x, -rel.y)
        out = 2.56 - q.z
        for hth, hz, hr in SACK_HOLES:
            out = min(out, math.hypot(adiff(th, hth) * 0.2, q.z - hz) - hr)
        return out

    bm = tube(SACK_PATH, 72, 56, r, ru=0.3, cap=False)
    cull(bm, set_rag(bm, torn), margin=-0.06)
    bmesh.ops.holes_fill(bm, edges=[e for e in bm.edges if e.is_boundary and e.verts[0].co.z < 1.75], sides=0)
    part("sack", bm, "burlap", "Sack")
    bm = tube(SACK_PATH[:-2], 24, 16, lambda c: ellipse(c.th, 0.8, 1.0) * tab(c.z, SZ, SR) * 0.9, ru=0.2, cap=True)
    part("sack_void", bm, "void", "Sack")

    # the cinch: a few turns of rope around the neck, a knot, a loose end
    cz = 2.445
    ctr = sack_centre(cz)
    helix = []
    for k in range(80):
        a = TAU * 2.3 * k / 79
        rr = 0.058 + 0.004 * math.sin(a * 3)
        helix.append(ctr + V(rr * math.sin(a), -rr * 0.8 * math.cos(a), 0.018 * k / 79 - 0.009))
    bm = tube(helix, 80, 8, 0.0095, ref=UP, raw=True, ru=0.03)
    part("rope_cinch", bm, "rope", "Sack")
    knot = ctr + V(-0.06, -0.01, 0.0)
    bm = tube([knot + V(0.015, 0, 0.01), knot, knot + V(-0.012, 0, -0.012)], 8, 10,
              lambda c: 0.02 * math.sin(math.pi * (0.1 + 0.8 * c.t)), ref=UP, ru=0.03, cap=True)
    part("rope_knot", bm, "rope", "Sack")
    bm = tube([knot, knot + V(-0.02, 0.02, -0.08), knot + V(-0.01, 0.04, -0.2)], 20, 8,
              lambda c: 0.009 * (1 - 0.3 * c.t), ref=UP, ru=0.03, cap=True)
    part("rope_end", bm, "rope", "Sack")
    # the strap: from the knot over his right shoulder and back under the arm
    strap = [knot + V(-0.01, -0.02, 0.02), V(-0.11, 0.2, 2.6), V(-0.16, 0.05, 2.675), V(-0.19, -0.08, 2.59),
             V(-0.175, -0.125, 2.38), V(-0.165, -0.075, 2.19), V(-0.165, 0.08, 2.12), V(-0.12, 0.25, 2.14),
             V(-0.08, 0.33, 2.18)]
    bm = tube(strap, 90, 8, 0.011, ref=UP, ru=0.035)

    def strap_w(co):
        return {"Sack": sstep(0.15, 0.3, co.y), "Torso": 1 - sstep(0.15, 0.3, co.y)}

    part("rope_strap", bm, "rope", strap_w)

    femurs = [
        (V(-0.09, 0.34, 2.3), V(-0.16, 0.66, 2.84), 0.0),
        (V(-0.02, 0.33, 2.32), V(-0.05, 0.69, 2.9), 1.4),
        (V(-0.13, 0.37, 2.27), V(-0.25, 0.6, 2.76), 2.6),
    ]
    for i, (a, b, rot) in enumerate(femurs):
        build_femur(a, b, rot, f"femur{i}")


def build_femur(a, b, rot, name):
    axis = (b - a).normalized()
    side = axis.cross(UP).normalized()
    mid = (a + b) / 2 + side * 0.015
    ctrl = [a, a.lerp(mid, 0.5), mid, mid.lerp(b, 0.5), b]

    def r(c):
        rr = 0.019 + 0.004 * sstep(0.5, 0.0, c.t)
        knob = sstep(0.84, 0.97, c.t)
        rr += 0.022 * knob
        rr *= 1 + 0.35 * math.cos(2 * (c.th - rot)) * knob
        rr *= 1 - 0.55 * sstep(0.975, 1.0, c.t)
        rr += 0.0015 * nz(c.q, 40.0, 11.0)
        return rr

    bm = tube(ctrl, 40, 16, r, ref=UP, ru=0.04, cap=True)
    part(name, bm, "bone", "Sack")
    # a ball head on one end, as a femur has
    top = b - axis * 0.02
    head_c = top + side * 0.035 * math.cos(rot) + axis.cross(side) * 0.035 * math.sin(rot) - axis * 0.015
    hb = bmesh.new()
    bmesh.ops.create_uvsphere(hb, u_segments=14, v_segments=10, radius=0.026, matrix=Matrix.Translation(head_c))
    hb.loops.layers.uv.new("Proc")
    uvl = hb.loops.layers.uv["Proc"]
    for f in hb.faces:
        for lp in f.loops:
            lp[uvl].uv = (lp.vert.co.x * 3, lp.vert.co.z * 3)
    part(name + "_head", hb, "bone", "Sack")


# ---------------------------------------------------------------- assembly
def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    global COLL
    COLL = bpy.data.collections.new("Silbon")
    bpy.context.scene.collection.children.link(COLL)


def apply_and_weight():
    dg = bpy.context.evaluated_depsgraph_get()
    for ob, _group, weights in PARTS:
        if ob.modifiers:
            me = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
            old = ob.data
            ob.modifiers.clear()
            ob.data = me
            bpy.data.meshes.remove(old)
    for ob, _group, weights in PARTS:
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


def join_sets():
    sets = {}
    for ob, group, _ in PARTS:
        sets.setdefault(group, []).append(ob)
    out = {}
    for group, obs in sets.items():
        target = obs[0]
        with bpy.context.temp_override(active_object=target, selected_editable_objects=obs, object=target):
            bpy.ops.object.join()
        target.name = "Silbon_" + group.capitalize()
        target.data.name = target.name
        out[group] = target
    return out


def unwrap(ob):
    me = ob.data
    bake_uv = me.uv_layers.new(name="Bake")
    me.uv_layers.active = bake_uv
    bake_uv.active_render = True
    bpy.context.view_layer.objects.active = ob
    for o in bpy.context.view_layer.objects:
        o.select_set(o == ob)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=0.004, area_weight=0.0,
                             correct_aspect=True, scale_to_bounds=False)
    bpy.ops.uv.pack_islands(margin=0.004, rotate=True)
    bpy.ops.object.mode_set(mode="OBJECT")


def setup_cycles(samples):
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    prefs = bpy.context.preferences.addons["cycles"].preferences
    for kind in ("OPTIX", "CUDA"):
        try:
            prefs.compute_device_type = kind
            prefs.get_devices()
            if any(d.type == kind for d in prefs.devices):
                for d in prefs.devices:
                    d.use = d.type == kind
                sc.cycles.device = "GPU"
                break
        except TypeError:
            continue
    sc.cycles.samples = samples
    sc.cycles.use_denoising = False


def bake_set(ob, group):
    size = TEX
    imgs = {
        "color": bpy.data.images.new(f"silbon_{group}_color", size, size, alpha=True),
        "alpha": bpy.data.images.new(f"silbon_{group}_alpha", size, size, alpha=False),
        "rough": bpy.data.images.new(f"silbon_{group}_rough", size, size, alpha=False),
        "normal": bpy.data.images.new(f"silbon_{group}_normal", size, size, alpha=False),
    }
    imgs["rough"].colorspace_settings.name = "Non-Color"
    imgs["normal"].colorspace_settings.name = "Non-Color"
    targets = []
    for slot in ob.material_slots:
        nt = slot.material.node_tree
        node = nt.nodes.new("ShaderNodeTexImage")
        node.name = "bake_target"
        targets.append((nt, node))
    for o in bpy.context.view_layer.objects:
        o.select_set(o == ob)
    bpy.context.view_layer.objects.active = ob
    sc = bpy.context.scene
    sc.render.bake.margin = 16
    imgs["alpha"].colorspace_settings.name = "Non-Color"
    passes = (("color", "DIFFUSE", 64), ("rough", "ROUGHNESS", 1), ("normal", "NORMAL", 8), ("alpha", "EMIT", 8))
    for key, kind, samples in passes:
        for nt, node in targets:
            node.image = imgs[key]
            nt.nodes.active = node
        sc.cycles.samples = samples
        if kind == "EMIT":
            # the torn-cloth mask, baked as light
            saved = []
            for nt, _node in targets:
                out = next(n for n in nt.nodes if n.type == "OUTPUT_MATERIAL")
                src = out.inputs["Surface"].links[0].from_socket
                em = nt.nodes.new("ShaderNodeEmission")
                nt.links.new(nt.nodes["ALPHA"].outputs[0], em.inputs["Color"])
                nt.links.new(em.outputs[0], out.inputs["Surface"])
                saved.append((nt, out, src, em))
            bpy.ops.object.bake(type="EMIT", margin=16, uv_layer="Bake")
            for nt, out, src, em in saved:
                nt.links.new(src, out.inputs["Surface"])
                nt.nodes.remove(em)
        elif kind == "DIFFUSE":
            bpy.ops.object.bake(type="DIFFUSE", pass_filter={"COLOR"}, margin=16, uv_layer="Bake")
        elif kind == "NORMAL":
            bpy.ops.object.bake(type="NORMAL", normal_space="TANGENT", margin=16, uv_layer="Bake")
        else:
            bpy.ops.object.bake(type=kind, margin=16, uv_layer="Bake")
    for nt, node in targets:
        nt.nodes.remove(node)
    px = np.empty(size * size * 4, np.float32)
    imgs["color"].pixels.foreach_get(px)
    al = np.empty(size * size * 4, np.float32)
    imgs["alpha"].pixels.foreach_get(al)
    px[3::4] = al[0::4]
    imgs["color"].pixels.foreach_set(px)
    # Normals and roughness carry less than colour: ship them smaller.
    imgs["normal"].scale(size // 2, size // 2)
    imgs["rough"].scale(size // 4, size // 4)
    for im in imgs.values():
        im.pack()
    has_holes = bool((al[0::4] < 0.5).any())
    bpy.data.images.remove(imgs.pop("alpha"))
    return imgs, has_holes


def final_material(group, imgs, masked):
    m = bpy.data.materials.new("Silbon_" + group.capitalize())
    m.use_backface_culling = False
    k = NodeKit(m)
    uv = k.new("ShaderNodeUVMap")
    uv.uv_map = "Bake"
    bsdf = k.new("ShaderNodeBsdfPrincipled")
    out = k.new("ShaderNodeOutputMaterial")
    k.t.links.new(bsdf.outputs[0], out.inputs["Surface"])
    col = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
    col.image = imgs["color"]
    k.t.links.new(col.outputs["Color"], bsdf.inputs["Base Color"])
    if masked:
        # alpha clip at 0.5; the glTF exporter reads Round as alphaMode MASK
        k.t.links.new(k.math("ROUND", col.outputs["Alpha"]), bsdf.inputs["Alpha"])
    rough = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
    rough.image = imgs["rough"]
    k.t.links.new(rough.outputs["Color"], bsdf.inputs["Roughness"])
    nrm = k.new("ShaderNodeTexImage", Vector=uv.outputs["UV"])
    nrm.image = imgs["normal"]
    nmap = k.new("ShaderNodeNormalMap", Color=nrm.outputs["Color"])
    k.t.links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
    bsdf.inputs["Metallic"].default_value = 0.0
    return m


def build_rig(objs):
    arm = bpy.data.armatures.new("SilbonRig")
    rig = bpy.data.objects.new("Silbon", arm)
    COLL.objects.link(rig)
    for o in bpy.context.view_layer.objects:
        o.select_set(o == rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for name, parent, (bx, by, bz) in JOINTS:
        eb = arm.edit_bones.new(name)
        eb.head = V(bx, -bz, by)
        eb.tail = eb.head + V(0, 0, 0.12)
        eb.roll = 0.0
        eb.use_connect = False
        if parent:
            eb.parent = arm.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    for ob in objs:
        ob.parent = rig
        mod = ob.modifiers.new("Armature", "ARMATURE")
        mod.object = rig
    return rig


def stats(objs):
    tris = 0
    for ob in objs:
        ob.data.calc_loop_triangles()
        tris += len(ob.data.loop_triangles)
        print(f"  {ob.name}: {len(ob.data.vertices)} verts, {len(ob.data.loop_triangles)} tris, "
              f"groups {sorted(g.name for g in ob.vertex_groups)}")
    print(f"  total tris: {tris}")


def main():
    reset_scene()
    make_details()
    make_materials()
    build_body()
    for s in (1, -1):
        build_arm(s)
    build_legs()
    build_coat()
    build_shawl()
    build_loose_strips()
    build_hair()
    build_hat()
    build_sack()
    print(f"built {len(PARTS)} parts")
    apply_and_weight()
    sets = join_sets()
    for ob in sets.values():
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
        bm.to_mesh(ob.data)
        bm.free()
    # turn to face +Y (glTF/Bevy forward -Z)
    turn = Matrix.Rotation(math.pi, 4, "Z")
    for ob in sets.values():
        ob.data.transform(turn)
    setup_cycles(16)
    for group, ob in sets.items():
        if group == "eyes":
            continue
        unwrap(ob)
        print(f"baking {group}...", flush=True)
        imgs, masked = bake_set(ob, group)
        mat = final_material(group, imgs, masked)
        ob.data.materials.clear()
        ob.data.materials.append(mat)
        ob.data.uv_layers.remove(ob.data.uv_layers["Proc"])
    eyes = sets["eyes"]
    eyes.data.uv_layers.remove(eyes.data.uv_layers["Proc"])
    for ob in sets.values():
        for name in ("rag", "col"):
            if name in ob.data.attributes:
                ob.data.attributes.remove(ob.data.attributes[name])
    rig = build_rig(list(sets.values()))
    stats(list(sets.values()))
    for m in list(bpy.data.materials):
        if m.name.startswith("src_"):
            bpy.data.materials.remove(m)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    bpy.ops.export_scene.gltf(
        filepath=OUT,
        export_format="GLB",
        export_yup=True,
        export_apply=False,
        export_skins=True,
        export_influence_nb=4,
        export_animations=False,
        export_tangents=True,
        export_materials="EXPORT",
        export_image_format="AUTO",
        export_cameras=False,
        export_lights=False,
        export_copyright="El Silbón — original procedural model (tools/silbon_model.py)",
    )
    print("wrote", OUT)
    if BLEND:
        bpy.ops.wm.save_as_mainfile(filepath=BLEND, compress=True)
        print("saved", BLEND)


main()

"""The four playable survivors, after the "El Silbón — Survivors" sheet, in
the sheet's faceted low-poly style: flat-shaded facets for the forms, one
baked texture per survivor for what facets cannot carry (the skirt's
flowers, the plaid, denim, woven straw, the faces).

- El Llanero, ranch hand: a cream liquiliqui (stand collar, buttons, four
  flapped pockets), wide trousers, a black flat-brimmed hat, dark
  alpargatas.
- La Coplera, song keeper: a white blouse with a ruffled scoop neckline and
  ruffled puff sleeves, a gathered red skirt with cream flowers and a
  ruffled hem, hair in a low bun tied with a red ribbon, alpargatas.
- El Encargado, ranch caretaker: an older man, grey hair and moustache, a
  khaki work shirt with rolled sleeves and flapped pockets, a belt, dark
  trousers tucked into rubber boots, a red-striped towel at the hip, a
  straw hat.
- El Muchacho, young local: an open short-sleeved blue plaid shirt over a
  white V-neck tee, jeans, a belt, sneakers, a red bracelet.

One body for all four, dressed per survivor. Each holds a torch in the
right hand where the game hangs the teammate's beam (Bevy (0.27, 0.95,
-0.36)). Skinned to the joints `src/world/avatar.rs` moves (see JOINTS).
Bevy space, facing -Z, feet on the ground, about 1.75 m (1.8 m in a hat).

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/survivors.py -- \\
        [--only llanero] [--renders DIR] [--portraits DIR] [--blend FILE] [--tex 2048]

Inside a live Blender (the MCP): run the file with ARGS = ["--live"] to
build and look without exporting.
"""

import functools
import importlib
import math
import os
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402

importlib.reload(modelkit)
from modelkit import (  # noqa: E402
    FRONT, ROOT, TAU, V, Asset, NodeKit, clamp, ellipse, gauss, lerp, nz, render_views, resample, sstep, tab,
    tube,
)

ARGS = globals().get("ARGS") or (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])


def arg(name, default=None):
    return ARGS[ARGS.index(name) + 1] if name in ARGS else default


ONLY = arg("--only").split(",") if "--only" in ARGS else None
RENDERS = arg("--renders")
PORTRAITS = arg("--portraits")
BLEND = arg("--blend")
TEX = int(arg("--tex", 2048))
LIVE = "--live" in ARGS
NAMES = ["llanero", "coplera", "encargado", "muchacho"]


def out_path(name):
    return os.path.join(ROOT, "assets", "models", f"survivor_{name}.glb")


# ---------------------------------------------------------------- the rig
# Joints in Bevy space (x, up, back); every bone points up with no roll, so
# rest rotations are identity. The character's left is Bevy -X.
HIP_Z, KNEE_Z, ANKLE_Z = 0.93, 0.5, 0.085
SHOULDER_Z, ELBOW_Z, WRIST_Z = 1.42, 1.14, 0.87
JOINTS = [
    ("Body", None, (0.0, 0.0, 0.0)),
    ("Pelvis", "Body", (0.0, 0.97, 0.0)),
    ("HipL", "Pelvis", (-0.095, HIP_Z, 0.0)),
    ("KneeL", "HipL", (-0.1, KNEE_Z, 0.0)),
    ("FootL", "KneeL", (-0.1, ANKLE_Z, 0.015)),
    ("HipR", "Pelvis", (0.095, HIP_Z, 0.0)),
    ("KneeR", "HipR", (0.1, KNEE_Z, 0.0)),
    ("FootR", "KneeR", (0.1, ANKLE_Z, 0.015)),
    ("Torso", "Pelvis", (0.0, 1.03, 0.0)),
    ("Chest", "Torso", (0.0, 1.25, 0.0)),
    ("Neck", "Chest", (0.0, 1.48, 0.005)),
    ("Head", "Neck", (0.0, 1.56, 0.0)),
    ("ShoulderL", "Chest", (-0.18, SHOULDER_Z, 0.0)),
    ("ElbowL", "ShoulderL", (-0.21, ELBOW_Z, 0.0)),
    ("HandL", "ElbowL", (-0.235, WRIST_Z, -0.01)),
    ("ShoulderR", "Chest", (0.18, SHOULDER_Z, 0.0)),
    ("ElbowR", "ShoulderR", (0.215, ELBOW_Z, 0.0)),
    ("HandR", "ElbowR", (0.258, 0.955, -0.115)),
]

# ---------------------------------------------------------------- colours (linear)
BRASS = (0.4, 0.27, 0.08)
TORCH = (0.03, 0.03, 0.032)
LENS = (0.9, 0.85, 0.62)
WHITE = (0.92, 0.92, 0.92)  # patterned parts: the pattern carries the colour
JUTE = (0.42, 0.32, 0.18)

# Pattern ids (the "pat" face attribute the baked material reads).
P_NONE, P_FLORAL, P_PLAID, P_DENIM, P_STRAW, P_FACE, P_WEAVE, P_JUTE, P_CANVAS = range(9)
HEAD_Z = 1.64
HEAD_S = 1.07  # the sheet's heads are a little large: everything above the neck scales about HEAD_PIVOT
HEAD_PIVOT = (0.0, 0.0, 1.55)
AO_DIST = float(arg("--ao-dist", 0.05))
AO_STRENGTH = float(arg("--ao", 0.5))


def mixc(a, b, t):
    t = clamp(t)
    return tuple(lerp(x, y, t) for x, y in zip(a, b))


def mottle(c, base, amt=0.1, s=12.0, o=0.0):
    k = 1.0 + amt * nz(c, s, o)
    return tuple(x * k for x in base)


def flat(colour):
    return lambda c, n: colour


def shade(colour, k):
    return tuple(x * k for x in colour)


def side_name(x):
    return "L" if x < 0 else "R"


class Section:
    """A stand-in for a tube's context when a radius is asked for directly."""

    def __init__(self, t, th=0.0):
        self.t, self.th = t, th


# ---------------------------------------------------------------- weights (Blender coords: x, fwd, up)
def body_w(co):
    t1 = sstep(0.95, 1.07, co.z)
    t2 = sstep(1.15, 1.3, co.z)
    t3 = sstep(1.47, 1.52, co.z)
    return {"Pelvis": 1 - t1, "Torso": t1 * (1 - t2), "Chest": t2 * (1 - t3), "Neck": t3}


def neck_w(co):
    n = sstep(1.44, 1.5, co.z)
    h = sstep(1.53, 1.6, co.z)
    return {"Chest": 1 - n, "Neck": n * (1 - h), "Head": h}


def arm_w(co):
    s = side_name(co.x)
    sh = sstep(1.47, 1.37, co.z)
    el = sstep(1.19, 1.09, co.z)
    if s == "R":  # the torch forearm reaches forward: the wrist is along +y
        hd = sstep(0.07, 0.11, co.y) * sstep(1.05, 0.98, co.z)
    else:
        hd = sstep(0.92, 0.86, co.z)
    return {"Chest": 1 - sh, f"Shoulder{s}": sh * (1 - el), f"Elbow{s}": el * (1 - hd), f"Hand{s}": hd}


def hand_w(co):
    return {f"Hand{side_name(co.x)}": 1.0}


def leg_w(co):
    s = side_name(co.x)
    h = sstep(1.0, 0.88, co.z)
    k = sstep(0.55, 0.45, co.z)
    f = sstep(0.14, 0.08, co.z)
    return {"Pelvis": 1 - h, f"Hip{s}": h * (1 - k), f"Knee{s}": k * (1 - f), f"Foot{s}": f}


def foot_w(co):
    s = side_name(co.x)
    f = sstep(0.2, 0.12, co.z)
    return {f"Knee{s}": 1 - f, f"Foot{s}": f}


def skirt_w(co):
    h = sstep(0.98, 0.6, co.z) * 0.75
    left = sstep(0.1, -0.1, co.x)
    return {"Pelvis": 1 - h, "HipL": h * left, "HipR": h * (1 - left)}


# ---------------------------------------------------------------- shapes
def blob(centre, radii, sides=12, rings=8, rot=None):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=sides, v_segments=rings, radius=1.0)
    bmesh.ops.scale(bm, vec=V(*radii), verts=bm.verts)
    if rot is not None:
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=rot)
    bmesh.ops.translate(bm, vec=V(*centre), verts=bm.verts)
    return bm


def box(centre, size, rot=None, bevel=0.0):
    return modelkit.box(centre, size, bevel=bevel, rot=rot)


def tag(bm, pat):
    """Stamp every face with a pattern id for the baked material."""
    lay = bm.faces.layers.int.get("pat") or bm.faces.layers.int.new("pat")
    for f in bm.faces:
        f[lay] = pat
    return bm


def vloft(zs, width, depth, sides=24, rows=None, shape=None, keep=None, cap=False, ru=None, x=0.0, y=0.0,
          sq=2.0, span=None):
    """A loft up the Z axis; cross-sections are superellipses of half
    `width`/`depth` (tables over `zs`), reshaped by `shape(c, r)`.
    Angle 0 faces forward (+Y), pi/2 is the character's left (-X)."""
    n = rows or max(4, int(abs(zs[-1] - zs[0]) / 0.025))
    path = [V(x, y, lerp(zs[0], zs[-1], i / (n - 1))) for i in range(n)]
    mean = (sum(width) + sum(depth)) / (len(width) + len(depth))

    def r(c):
        z = c.p.z
        a, b = tab(z, zs, depth), tab(z, zs, width)
        ct, st = abs(math.cos(c.th)), abs(math.sin(c.th))
        rr = ((ct / a) ** sq + (st / b) ** sq) ** (-1.0 / sq)
        return shape(c, rr) if shape else rr

    return tube(path, n, sides, r, ref=FRONT, raw=True, keep=keep, cap=cap, ru=ru or mean, span=span,
                closed=span is None)


def limb(pts, n, sides, rfun, cap=True, ru=None, disp=None, keep=None):
    return tube(pts, n, sides, rfun, ref=FRONT, cap=cap, ru=ru or 0.05, disp=disp, keep=keep)


def lathe_z(centre, profile, sides=24, ru=None, cap=False, disp=None, squash=1.0):
    """A surface of revolution about Z through (radius, height) points."""
    cx, cy, cz = centre
    pts = [V(cx, cy, cz + h) for _r, h in profile]
    order = sorted(range(len(profile)), key=lambda i: profile[i][1])
    hs = [profile[i][1] for i in order]
    rs = [profile[i][0] for i in order]

    def rf(c):
        r = tab(c.p.z - cz, hs, rs)
        return ellipse(c.th, r * squash, r)

    return tube(pts, len(pts), sides, rf, ref=FRONT, raw=True, cap=cap, ru=ru or max(rs), disp=disp)


def shell_part(centre, radii, keep, sides=18, rings=12, bump=None):
    bm = blob(centre, radii, sides, rings)
    if bump:
        for v in bm.verts:
            d = (v.co - V(*centre)).normalized()
            v.co += d * bump(v.co)
    gone = [f for f in bm.faces if not keep(f.calc_center_median())]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    return bm


def combed(co):
    """Grooves combed from the brow back over the crown, a little ragged."""
    return 0.0035 * max(0.0, math.sin(co.x * 170.0 + 0.3 * math.sin(co.y * 40.0))) + 0.002 * nz(co, 60.0, 5.0)


def hair_short(c):
    """Short hair: the crown, the back and above the ears, a rounded hairline."""
    cz = HEAD_Z
    if c.y > 0.045:
        return c.z > cz + 0.05 - 0.2 * max(0.0, c.y - 0.075) + 1.2 * c.x * c.x
    if abs(c.x) > 0.058 and c.y > -0.03:
        return c.z > cz + 0.008
    return c.z > cz - 0.06


# ---------------------------------------------------------------- the body
# Torso: heights, half-widths (x), half-depths (y).
TORSO_Z = [0.84, 0.92, 1.02, 1.12, 1.22, 1.3, 1.37, 1.42, 1.455, 1.48, 1.495]
MALE_W = [0.164, 0.17, 0.156, 0.16, 0.172, 0.18, 0.182, 0.168, 0.134, 0.087, 0.056]
MALE_D = [0.104, 0.11, 0.1, 0.102, 0.112, 0.114, 0.104, 0.09, 0.075, 0.06, 0.05]
FEMALE_W = [0.17, 0.174, 0.128, 0.136, 0.146, 0.152, 0.15, 0.14, 0.114, 0.078, 0.052]
FEMALE_D = [0.108, 0.112, 0.09, 0.098, 0.116, 0.108, 0.094, 0.082, 0.07, 0.058, 0.048]


class Person:
    def __init__(self, name, seed, skin, female=False, belly=0.0, age=0.0):
        self.name = name
        self.a = Asset(f"survivor_{name}", seed)
        # quieter per-facet variation than the toolkit's default
        self.a.paint_facets = functools.partial(Asset.paint_facets, self.a, jitter=0.035)
        self.a.mat("cloth", cloth_material(f"survivor_{name}", female, age), "cloth")
        self.skin = skin
        self.in_head = False
        self.female = female
        self.belly = belly
        self.age = age
        self.W = list(FEMALE_W if female else MALE_W)
        self.D = list(FEMALE_D if female else MALE_D)
        if belly:
            for i, z in enumerate(TORSO_Z):
                k = gauss(z - 1.05, 0.12) * belly
                self.D[i] += 0.02 * k
                self.W[i] += 0.01 * k

    def part(self, name, bm, weights, paint, pat=P_NONE, solidify=0.0):
        tag(bm, pat)
        if self.in_head:
            px, py, pz = HEAD_PIVOT
            bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation((px, py, pz))
                                @ Matrix.Scale(HEAD_S, 4) @ Matrix.Translation((-px, -py, -pz)))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])  # outward, so the baked AO looks out
        return self.a.part(name, bm, "cloth", weights, smooth=False, paint=paint, solidify=solidify)

    def skin_paint(self, c, n):
        return mottle(c, self.skin, 0.05, 25.0)

    # -- surface helpers
    def front_at(self, z, x=0.0, grow=0.0):
        """Where the torso's front surface is at height z and across-offset x."""
        d = tab(z, TORSO_Z, self.D) + grow
        w = tab(z, TORSO_Z, self.W) + grow
        return d * math.sqrt(max(0.0, 1.0 - (x / w) ** 2)) if abs(x) < w else 0.0

    def torso_shape(self, grow=0.0, chest=1.0):
        female, belly = self.female, self.belly

        def shape(c, rr):
            z, th = c.p.z, c.th
            s, fr = math.sin(th), math.cos(th)
            if fr > 0:
                if female:
                    for sg in (-1, 1):
                        rr += 0.012 * chest * gauss(s - sg * 0.38, 0.42) * gauss(z - 1.27, 0.07) * fr
                else:
                    for sg in (-1, 1):  # pectorals
                        rr += 0.01 * chest * gauss(s - sg * 0.45, 0.3) * gauss(z - 1.3, 0.05) * fr
                rr += 0.02 * belly * gauss(z - 1.02, 0.08) * fr ** 2
            else:
                for sg in (-1, 1):  # shoulder blades
                    rr += 0.01 * gauss(s - sg * 0.5, 0.28) * gauss(z - 1.34, 0.06)
                rr -= 0.006 * gauss(s, 0.12) * sstep(1.0, 1.2, z)  # the spine's groove
                rr += 0.012 * gauss(z - 0.9, 0.05) * (0.5 + 0.5 * abs(s))  # seat
            # the deltoids round the top corners out to the arms
            rr += 0.014 * gauss(abs(s) - 1.0, 0.35) * gauss(z - 1.4, 0.035)
            return rr + grow

        return shape

    def torso(self, paint, pat=P_NONE, grow=0.0, bottom=0.84, top=1.495, keep=None, name="torso", flare=0.0,
              chest=1.0, span=None):
        zs, w, d = [], [], []
        for z, a, b in zip(TORSO_Z, self.W, self.D):
            if bottom - 0.001 <= z <= top + 0.001:
                zs.append(z)
                w.append(a)
                d.append(b)
        if bottom < zs[0]:
            zs.insert(0, bottom)
            w.insert(0, w[0] + flare)
            d.insert(0, d[0] + flare * 0.8)
        if top > zs[-1]:
            zs.append(top)
            w.append(w[-1])
            d.append(d[-1])
        bm = vloft(zs, w, d, sides=28 if span is None else 40, shape=self.torso_shape(grow, chest), keep=keep,
                   cap=keep is None and span is None, sq=2.2, span=span)
        return self.part(name, bm, body_w, paint, pat)

    def neck(self, paint=None, top=1.6):
        pts = [V(0, 0.0, 1.44), V(0, 0.004, 1.52), V(0, 0.01, top)]
        bm = limb(pts, 6, 12, lambda c: ellipse(c.th, lerp(0.05, 0.046, c.t), lerp(0.055, 0.05, c.t)))
        self.part("neck", bm, neck_w, paint or self.skin_paint)

    # -- the head: an ellipsoid sculpted into a face
    def head(self, hair, hair_keep, brows=None, hair_bump=None, extra=None, jaw=1.0):
        self.in_head = True
        try:
            self._head(hair, hair_keep, brows, hair_bump, extra, jaw)
        finally:
            self.in_head = False

    def _head(self, hair, hair_keep, brows, hair_bump, extra, jaw):
        cz = HEAD_Z
        centre = V(0, 0.006, cz)
        head = blob(tuple(centre), (0.078, 0.094, 0.11), 30, 22)
        female = self.female
        for v in head.verts:
            q = v.co - centre
            ax = abs(q.x)
            front = q.y > 0.0
            if q.z < -0.005:  # the jaw narrows to the chin
                k = sstep(-0.005, -0.1, q.z)
                q.x *= 1 - (0.3 if female else 0.24 / jaw) * k
                if not front:
                    q.y *= 1 - 0.45 * k
            if front:
                fy = sstep(0.03, 0.08, q.y)
                q.y += 0.011 * gauss(q.x, 0.028) * gauss(q.z + 0.088, 0.02) * fy  # chin
                q.y += 0.007 * gauss(q.z - 0.03, 0.012) * gauss(ax - 0.03, 0.03) * fy  # brow ridge
                q.y -= 0.008 * gauss(q.z - 0.006, 0.012) * gauss(ax - 0.032, 0.015) * fy  # eye sockets
                q.x += math.copysign(0.005, q.x) * gauss(q.z + 0.012, 0.02) * sstep(0.0, 0.05, q.y)  # cheekbones
                q.y -= 0.006 * gauss(ax - 0.05, 0.02) * gauss(q.z + 0.05, 0.03) * fy  # under the cheekbones
                q.y += 0.004 * gauss(q.x, 0.02) * gauss(q.z + 0.066, 0.01) * fy  # lips
            elif q.z > -0.02:
                q.y *= 1.04  # a full back of the skull
            if q.z > 0.07:  # a flatter crown
                q.z = 0.07 + (q.z - 0.07) * 0.9
            v.co = centre + q
        self.part("head", head, neck_w, self.face_paint, P_FACE)
        # the nose: a bridge down to a rounded tip with nostril wings
        nose = limb([V(0, 0.087, cz + 0.02), V(0, 0.097, cz - 0.006), V(0, 0.108, cz - 0.03)], 6, 8,
                    lambda c: ellipse(c.th, lerp(0.006, 0.013, c.t), lerp(0.006, 0.014, c.t)))
        self.part("nose", nose, "Head", self.skin_paint)
        for s in (-1, 1):
            self.part(f"nostril{s}", blob((s * 0.011, 0.098, cz - 0.03), (0.009, 0.011, 0.008), 8, 5), "Head",
                      self.skin_paint)
            ear = blob((s * 0.079, -0.004, cz - 0.004), (0.011, 0.022, 0.031), 10, 7,
                       rot=Matrix.Rotation(s * 0.25, 3, "Z"))
            self.part(f"ear{s}", ear, "Head", self.skin_paint)
            # the eyes: small dark lenses in the sockets (the face texture paints the rest)
            eye = blob((s * 0.032, 0.081, cz + 0.006), (0.0115, 0.006, 0.0062), 10, 6)
            self.part(f"eye{s}", eye, "Head", lambda c, n: (0.04, 0.025, 0.018)
                      if abs(abs(c.x) - 0.032) < 0.006 else (0.62, 0.58, 0.52))
            if brows:
                brow = limb([V(s * 0.014, 0.09, cz + 0.024), V(s * 0.034, 0.091, cz + 0.031),
                             V(s * 0.052, 0.082, cz + 0.026)], 5, 6,
                            lambda c: ellipse(c.th, 0.004, lerp(0.006, 0.0035, c.t)))
                self.part(f"brow{s}", brow, "Head", flat(brows))
        hs = shell_part((0, 0.004, cz + 0.004), (0.084, 0.1, 0.116), hair_keep, 48, 28, hair_bump)
        self.part("hair", hs, "Head", hair)
        if extra:
            extra()

    def face_paint(self, c, n):
        col = mottle(c, self.skin, 0.05, 25.0)
        q = (c - V(*HEAD_PIVOT)) / HEAD_S + V(*HEAD_PIVOT) - V(0, 0, HEAD_Z)
        # warmth on the cheeks, a shaved jaw's shadow on the men
        col = mixc(col, shade(self.skin, 1.18), 0.5 * gauss(abs(q.x) - 0.045, 0.02) * gauss(q.z + 0.02, 0.02))
        if not self.female:
            col = mixc(col, shade(self.skin, 0.8), 0.35 * sstep(-0.04, -0.08, q.z) * sstep(0.0, 0.05, q.y))
        return col

    # -- arms: skin, and a sleeve to `cut` (0..1 along the arm)
    def arm_path(self, s):
        if s > 0:  # the torch arm: the forearm comes forward to the beam
            return [V(0.158, 0.0, 1.405), V(0.195, 0.0, 1.29), V(0.215, 0.0, 1.14), V(0.235, 0.045, 1.035),
                    V(0.258, 0.115, 0.955)]
        return [V(-0.158, 0.0, 1.405), V(-0.195, 0.006, 1.28), V(-0.21, 0.01, 1.14), V(-0.224, 0.006, 1.0),
                V(-0.235, 0.0, 0.87)]

    def arm_radius(self, t):
        f = 0.92 if self.female else 1.06
        return f * tab(t, [0, 0.1, 0.3, 0.48, 0.58, 0.72, 1.0], [0.054, 0.05, 0.046, 0.04, 0.045, 0.041, 0.028])

    def arm(self, s, sleeve, cut, pat=P_NONE, puff=0.0, cuff=0.0, roll=None, ruffle=None, loose=1.1):
        dense, _ = resample(self.arm_path(s), 60)
        sk = tube(dense, 30, 12, lambda c: self.arm_radius(c.t) * 0.96, ref=FRONT, raw=True, cap=True, ru=0.04)
        self.part(f"arm{s}", sk, arm_w, self.skin_paint)
        k = max(2, int(round(59 * cut)) + 1)

        def sr(c):
            t = c.t * cut
            rr = self.arm_radius(t) * loose + 0.004
            rr += puff * gauss(c.t - 0.5, 0.35)
            rr += cuff * sstep(0.82, 1.0, c.t)
            rr += 0.003 * math.sin(c.th * 3 + t * 20) * gauss(t - 0.5, 0.12)  # folds at the elbow
            return rr

        sl = tube(dense[:k], k, 14, sr, ref=FRONT, raw=True, cap=True, ru=0.06)
        self.part(f"sleeve{s}", sl, arm_w, sleeve, pat, solidify=0.003)
        dome = shell_part((s * 0.163, 0.0, 1.402), (0.066, 0.064, 0.046), lambda q: q.z > 1.395, 16, 10)
        self.part(f"shoulder{s}", dome, arm_w, sleeve, pat)
        end, nxt = dense[k - 1], dense[min(k, 59)]
        axis = (nxt - end).normalized()
        rim = sr(Section(1.0))
        if roll is not None:
            ring = tube([end - axis * 0.015, end + axis * 0.03], 3, 14, lambda c: rim + 0.006, ref=FRONT, raw=True,
                        cap=False, ru=0.06)
            self.part(f"roll{s}", ring, arm_w, roll, pat, solidify=0.004)
        if ruffle is not None:
            ruf = tube([end - axis * 0.01, end + axis * 0.035], 3, 36,
                       lambda c: (rim + 0.012 * c.t) * (1 + 0.1 * math.sin(c.th * 11)), ref=FRONT, raw=True,
                       cap=False, ru=0.06)
            self.part(f"ruffle{s}", ruf, arm_w, ruffle, P_WEAVE, solidify=0.003)
        return dense

    # -- hands: a palm, four fingers and a thumb
    def finger(self, name, pts, r0):
        bm = tube(pts, 6, 7, lambda c: lerp(r0, r0 * 0.78, c.t), ref=FRONT, cap=True, ru=0.01)
        self.part(name, bm, hand_w, self.skin_paint)

    def hands(self):
        f = 0.93 if self.female else 1.0
        # left: relaxed at the side, palm inward, fingers softly curled
        w = V(-0.236, 0.0, 0.868)
        palm = blob(tuple(w + V(0.002, 0.004, -0.045 * f)), (0.017 * f, 0.036 * f, 0.042 * f), 10, 7)
        self.part("palmL", palm, hand_w, self.skin_paint)
        for i in range(4):
            y = (0.021 - i * 0.014) * f
            base = w + V(0.004, y, -0.082 * f)
            ln = [0.028, 0.03, 0.03, 0.023][i] * f
            self.finger(f"fingerL{i}", [base, base + V(0.004, 0.006, -ln), base + V(0.012, 0.014, -ln * 1.7)],
                        0.0082 * f)
        th = w + V(0.014, 0.03 * f, -0.035 * f)
        self.finger("thumbL", [th, th + V(0.008, 0.016, -0.02), th + V(0.012, 0.02, -0.042)], 0.009 * f)
        # right: a fist round the torch, the thumb along the top
        g = V(0.268, 0.165, 0.945)
        self.part("fistR", blob(tuple(g), (0.034 * f, 0.042 * f, 0.036 * f), 12, 8), hand_w, self.skin_paint)
        for i in range(4):
            y = g.y + (0.026 - i * 0.016) * f
            ring = [V(g.x + 0.03 * math.cos(a), y, g.z + 0.03 * math.sin(a)) for a in (0.9, 0.1, -0.9, -1.8)]
            self.finger(f"fingerR{i}", ring, 0.0085 * f)
        self.finger("thumbR", [g + V(-0.018, -0.02, 0.025), g + V(-0.012, 0.012, 0.034), g + V(-0.004, 0.035, 0.03)],
                    0.009 * f)
        body = tube([V(0.27, 0.1, 0.945), V(0.27, 0.29, 0.95)], 2, 12, lambda c: lerp(0.019, 0.024, c.t), ref=FRONT,
                    raw=True, cap=True, ru=0.02)
        self.part("torch", body, hand_w, flat(TORCH))
        head = tube([V(0.27, 0.285, 0.95), V(0.27, 0.34, 0.95)], 2, 14, lambda c: lerp(0.028, 0.04, c.t), ref=FRONT,
                    raw=True, cap=True, ru=0.04)
        self.part("torch_head", head, hand_w, lambda c, n: LENS if n.y > 0.8 else shade(TORCH, 1.8))

    # -- legs
    def legs(self, paint, pat=P_NONE, top=1.0, hem=0.08, r_top=0.09, r_knee=0.064, r_hem=0.06, folds=1.0,
             skin=False):
        for s in (-1, 1):
            x = s * 0.1
            pts = [V(x * 0.96, 0.004, top), V(x * 1.02, 0.01, 0.75), V(x, 0.016, KNEE_Z), V(x, 0.0, 0.28)]
            pts = [q for q in pts if q.z > hem + 0.04] + [V(x, -0.004, hem)]
            low = max(0.3, hem + 0.06)

            def r(c, s=s):
                z = c.p.z
                if skin:  # a calf, an ankle
                    rr = tab(z, [hem, 0.14, 0.3, 0.4, 0.5, 0.75, top],
                             [0.032, 0.036, 0.054, 0.056, 0.048, 0.066, 0.076])
                    if math.cos(c.th) < 0:
                        rr += 0.008 * gauss(z - 0.36, 0.06)
                    return rr
                rr = tab(z, [hem, low, KNEE_Z, 0.75, top], [r_hem, r_knee * 0.96, r_knee, r_top * 0.9, r_top])
                if s * math.sin(c.th) < 0:
                    rr *= 0.95
                # folds: bunched at the ankle, a crease behind the knee
                rr += folds * 0.004 * math.sin(c.th * 4 + z * 60) * sstep(hem + 0.15, hem + 0.02, z)
                rr += folds * 0.003 * math.sin(c.th * 3 + z * 35) * gauss(z - KNEE_Z, 0.06)
                return rr

            bm = tube(pts, 26, 14, r, ref=FRONT, cap=True, ru=0.07)
            self.part(f"leg{s}", bm, leg_w, paint, pat)

    def shoe(self, s, kind, upper, sole, upper_pat=P_CANVAS):
        x = s * 0.1

        def ur(c):  # a rounded toe box on a flat sole
            t = c.t
            h = tab(t, [0, 0.2, 0.6, 1.0], [0.05, 0.055, 0.04, 0.02]) * (1.15 if kind == "sneaker" else 1.0)
            w = tab(t, [0, 0.3, 0.7, 1.0], [0.036, 0.042, 0.046, 0.032])
            rr = ellipse(c.th, h, w)
            if math.cos(c.th) < 0:
                rr = min(rr, 0.012 / max(abs(math.cos(c.th)), 0.2))
            return rr

        up = tube([V(x, -0.055, 0.035), V(x, 0.03, 0.034), V(x + s * 0.004, 0.14, 0.026)], 16, 14, ur,
                  ref=V(0, 0, 1), cap=True, ru=0.04)
        self.part(f"shoe{s}", up, foot_w, upper, upper_pat)
        sol = tube([V(x, -0.06, 0.012), V(x, 0.05, 0.012), V(x + s * 0.004, 0.152, 0.012)], 12, 16,
                   lambda c: ellipse(c.th, 0.013, tab(c.t, [0, 0.3, 0.7, 1.0], [0.038, 0.046, 0.05, 0.03])),
                   ref=V(0, 0, 1), cap=True, ru=0.04)
        self.part(f"sole{s}", sol, foot_w, sole, P_JUTE if kind == "alpargata" else P_NONE)
        if kind == "sneaker":  # laces across the top
            for i in range(4):
                self.part(f"lace{s}{i}", box((x, i * 0.022, 0.083 - i * 0.006), (0.034, 0.006, 0.006)), foot_w,
                          flat((0.7, 0.7, 0.68)))

    def boot(self, s, colour, top=0.42):
        x = s * 0.1
        shaft = lathe_z((x, -0.004, 0.0), [(0.07, 0.07), (0.066, 0.2), (0.07, top - 0.02), (0.074, top)], 18)
        self.part(f"boot{s}", shaft, foot_w, lambda c, n: mottle(c, colour, 0.15, 20.0))
        rim = lathe_z((x, -0.004, 0.0), [(0.075, top - 0.012), (0.078, top + 0.004)], 18)
        self.part(f"bootrim{s}", rim, foot_w, flat(shade(colour, 1.4)))
        self.shoe(s, "boot", lambda c, n: mottle(c, colour, 0.15, 20.0), flat(shade(colour, 0.7)), P_NONE)

    def belt(self, z=0.99, colour=(0.05, 0.026, 0.012), buckle=BRASS, height=0.04):
        zs = [z - height / 2, z + height / 2]
        w = [tab(q, TORSO_Z, self.W) + 0.012 for q in zs]
        d = [tab(q, TORSO_Z, self.D) + 0.012 for q in zs]
        bm = vloft(zs, w, d, sides=28, rows=2, shape=self.torso_shape(0.0))
        self.part("belt", bm, body_w, lambda c, n: mottle(c, colour, 0.15, 30.0), solidify=0.004)
        f = self.front_at(z, 0.0) + 0.022
        self.part("buckle", box((0, f, z), (0.052, 0.008, 0.038), bevel=0.004), body_w,
                  lambda c, n: shade(buckle, 1.4) if n.y > 0.8 else buckle)
        for k in (-1, 1):  # belt loops
            fx = k * 0.1
            self.part(f"loop{k}", box((fx, self.front_at(z, fx) + 0.016, z), (0.012, 0.008, 0.05)), body_w,
                      flat(shade(colour, 1.3)))

    def buttons(self, zs, colour, grow=0.0, dx=0.0):
        for i, z in enumerate(zs):
            f = self.front_at(z, dx, grow) + 0.004
            self.part(f"button{len(self.a.parts)}", blob((dx, f, z), (0.0075, 0.004, 0.0075), 8, 5), body_w,
                      flat(colour))

    def pocket(self, name, x, z, w, h, paint, pat=P_NONE, grow=0.0, flap=True, button=None):
        f = self.front_at(z, x, grow) + 0.002
        rot = Matrix.Rotation(-math.atan2(x, max(f, 0.05)) * 0.35, 3, "Z")
        self.part(name, box((x, f, z), (w, 0.008, h), rot=rot, bevel=0.002), body_w,
                  lambda c, n: shade(paint(c, n), 0.95), pat)
        if flap:
            fb = box((x, f + 0.004, z + h * 0.4), (w * 1.08, 0.009, h * 0.34), rot=rot, bevel=0.002)
            self.part(name + "_flap", fb, body_w, lambda c, n: shade(paint(c, n), 0.84), pat)
        if button:
            self.part(name + "_btn", blob((x, f + 0.01, z + h * 0.32), (0.006, 0.004, 0.006), 8, 5), body_w,
                      flat(button))

    def collar_stand(self, paint, pat=P_NONE, height=0.05):
        """The liquiliqui's stand collar: a band round the neck, closed at the front."""
        bm = vloft([1.47, 1.47 + height], [0.064, 0.06], [0.062, 0.057], sides=24, rows=3, ru=0.065)
        self.part("collar", bm, neck_w, paint, pat, solidify=0.005)

    def collar_open(self, paint, pat=P_NONE, spread=0.014, gap=0.45, lapel=1.0):
        """A turned-down shirt collar: from the neck it folds out over the
        shoulders, open at the front, its points reaching down the chest."""
        def r(c):  # t runs from the fold at the neck (0) to the outer edge (1)
            return ellipse(c.th, 0.06, 0.066) + (0.022 + spread) * c.t

        def disp(c):
            fr = max(math.cos(c.th), 0.0)
            return V(0, 0.012 * lapel * fr ** 2 * c.t, -0.035 * lapel * sstep(0.35, 0.95, fr) * c.t)

        bm = tube([V(0, -0.01, 1.515), V(0, -0.008, 1.5), V(0, -0.004, 1.465)], 3, 30, r, ref=FRONT, raw=True,
                  cap=False, ru=0.07, span=lambda t: (gap, TAU - gap), closed=False, disp=disp)
        self.part("collar", bm, neck_w, paint, pat, solidify=0.004)

    def hat(self, crown_r, crown_h, brim_r, z0, paint, band, pat=P_NONE, pinch=0.0, curl=0.0, top=0.0, droop=0.0):
        def disp(c):
            q = c.q
            rr = math.hypot(q.x, q.y - 0.004)
            k = sstep(crown_r * 1.1, brim_r, rr)
            front = (q.y - 0.004) / max(rr, 1e-4)
            return V(0, 0, curl * k * k - droop * k * max(front, 0.0) - droop * 0.5 * k * max(-front, 0.0))

        brim = tube([V(0, 0.004, z0), V(0, 0.004, z0 + 0.01)], 2, 40, lambda c: brim_r, ref=FRONT, raw=True,
                    cap=True, ru=brim_r, disp=disp)
        self.part("brim", brim, "Head", paint, pat)

        def cr(c):
            t = c.t
            rr = ellipse(c.th, crown_r * 1.12, crown_r) * (1.0 - top * t * t)
            if t > 0.7:  # a pinched, dented top
                rr *= 1.0 - pinch * sstep(0.7, 1.0, t) * (0.5 + 0.5 * abs(math.sin(c.th)))
            return rr

        crown = tube([V(0, 0.004, z0), V(0, 0.004, z0 + crown_h)], 8, 24, cr, ref=FRONT, raw=True, cap=True,
                     ru=crown_r)
        self.part("crown", crown, "Head", paint, pat)
        bandm = tube([V(0, 0.004, z0 + 0.006), V(0, 0.004, z0 + 0.03)], 2, 24,
                     lambda c: ellipse(c.th, crown_r * 1.12, crown_r) + 0.003, ref=FRONT, raw=True, cap=False,
                     ru=crown_r)
        self.part("band", bandm, "Head", band, solidify=0.002)

    def finish(self, out):
        return self.a.finish(tex={"cloth": TEX}, out=out, joints=JOINTS)


# ---------------------------------------------------------------- the baked material and its patterns
def np_image(name, rgba):
    h, w = rgba.shape[:2]
    if name in bpy.data.images:
        bpy.data.images.remove(bpy.data.images[name])
    im = bpy.data.images.new(name, w, h, alpha=True, float_buffer=True)
    im.colorspace_settings.name = "Non-Color"
    im.pixels.foreach_set(np.ascontiguousarray(rgba, dtype=np.float32).ravel())
    im.pack()
    return im


def smooth_edge(d, w):
    return np.clip(0.5 - d / w, 0.0, 1.0)


def pat_floral(n=512):
    """Cream five-petalled flowers with small buds on a red ground."""
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    ground = np.array([0.3, 0.042, 0.024])
    petal_c = np.array([0.74, 0.5, 0.4])
    centre_c = np.array([0.42, 0.1, 0.045])
    mask = np.zeros((n, n))
    cmask = np.zeros((n, n))
    for (cx, cy, rad, petals, rot) in ((0.25, 0.28, 0.2, 5, 0.3), (0.75, 0.78, 0.2, 5, 1.1),
                                       (0.78, 0.26, 0.085, 4, 0.0), (0.24, 0.76, 0.085, 4, 0.7)):
        for ox in (-1, 0, 1):
            for oy in (-1, 0, 1):
                dx, dy = U - cx - ox, Vv - cy - oy
                r = np.hypot(dx, dy)
                a = np.arctan2(dy, dx) + rot
                edge = rad * (0.42 + 0.58 * np.abs(np.cos(petals * a / 2)) ** 0.6)
                m = smooth_edge(r - edge, 0.012)
                vein = (np.abs(np.sin(petals * a / 2)) < 0.12) & (r < edge * 0.8) & (r > rad * 0.25)
                mask = np.maximum(mask, np.where(vein, m * 0.7, m))
                cmask = np.maximum(cmask, smooth_edge(r - rad * 0.2, 0.01))
    col = ground * (1 - mask[..., None]) + petal_c * mask[..., None]
    col = col * (1 - cmask[..., None]) + centre_c * cmask[..., None]
    img = np.ones((n, n, 4), np.float32)
    img[..., :3] = col
    return img


def pat_plaid(n=512):
    """A blue twill plaid: navy bands, lighter lines, a diagonal weave."""
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    base = np.array([0.095, 0.15, 0.3])

    def bands(t):
        k = np.ones_like(t)
        k *= 1 - 0.4 * smooth_edge(np.abs(t - 0.25) - 0.13, 0.01)  # a wide navy band
        k *= 1 + 0.55 * smooth_edge(np.abs(t - 0.62) - 0.018, 0.006)  # a light line
        k *= 1 + 0.3 * smooth_edge(np.abs(t - 0.82) - 0.01, 0.005)
        k *= 1 - 0.25 * smooth_edge(np.abs(t - 0.02) - 0.02, 0.006)
        return k

    k = np.sqrt(bands(U) * bands(Vv))
    twill = 0.92 + 0.08 * np.sin((U + Vv) * n * 0.9)
    img = np.ones((n, n, 4), np.float32)
    img[..., :3] = base * (k * twill)[..., None]
    return img


def pat_tone(h, lo, hi, tint=(1.0, 1.0, 1.0)):
    """A grey multiplier image from a 0..1 tone map."""
    n = h.shape[0]
    img = np.ones((n, n, 4), np.float32)
    v = lo + (hi - lo) * np.clip(h, 0, 1)
    for i in range(3):
        img[..., i] = v * tint[i]
    return img


def pat_denim(n=256):
    c = (np.arange(n) + 0.5) / n
    U, Vv = np.meshgrid(c, c)
    rng = np.random.default_rng(7)
    twill = 0.5 + 0.5 * np.sin((U * 3 + Vv) * n * 0.5)
    slub = rng.uniform(0.85, 1.15, n)[(Vv * n).astype(int) % n]
    return pat_tone(twill * 0.6 + 0.4 * (slub - 0.85) / 0.3, 0.78, 1.12, (1.0, 1.02, 1.08))


def face_decal(name, female, age, brow, lip):
    """The face, painted: eyes, lids, brows, nostrils, lips, and lines with
    age. Covers x -0.07..0.07, z HEAD_Z-0.1..HEAD_Z+0.06 (Blender units);
    row 0 is the bottom, as Blender stores pixels."""
    n = 256
    X, Z = np.meshgrid(np.linspace(-0.07, 0.07, n), np.linspace(-0.1, 0.06, n))
    img = np.zeros((n, n, 4), np.float32)

    def paint(mask, colour):
        a = np.clip(mask, 0, 1)[..., None]
        img[..., :3] = img[..., :3] * (1 - a) + np.array(colour) * a
        img[..., 3:] = np.maximum(img[..., 3:], a)

    for s in (-1, 1):
        ex, ez = s * 0.032, 0.006
        # the almond of the eye: white, a dark iris, a heavy upper lid
        e = ((X - ex) / 0.0125) ** 2 + ((Z - ez) / np.where(Z > ez, 0.0064, 0.0042)) ** 2
        paint(smooth_edge(e - 1.0, 0.25), (0.6, 0.55, 0.5))
        iris = np.hypot(X - ex, Z - ez)
        paint(smooth_edge(iris - 0.0056, 0.0012) * (e < 1.1), (0.07, 0.04, 0.025))
        paint(smooth_edge(iris - 0.0024, 0.001) * (e < 1.1), (0.01, 0.008, 0.007))
        paint(smooth_edge(np.hypot(X - ex - 0.0018, Z - ez - 0.0018) - 0.0011, 0.0008), (0.8, 0.78, 0.75))
        lid = np.abs(np.sqrt(np.maximum(e, 0)) - 1.0) < (0.32 if female else 0.22)
        paint(lid * (Z > ez - 0.001) * 0.9, (0.03, 0.018, 0.014))
        if female:  # lashes flicking out at the outer corner
            out = s * (X - ex)
            paint(smooth_edge(np.abs(Z - ez - 0.004 - (out - 0.011) * 0.5) - 0.0012, 0.001) * (out > 0.009)
                  * (out < 0.016), (0.02, 0.012, 0.01))
        # the brow, over the socket
        bz = 0.027 + 0.004 * np.exp(-((X - s * 0.03) / 0.012) ** 2)
        bw = 0.0035 if female else 0.0055
        taper = 1 - np.clip((np.abs(X) - 0.014) / 0.045, 0, 1) * 0.6
        paint(smooth_edge(np.abs(Z - bz) - bw * taper, 0.0012) * (np.abs(X) > 0.012) * (np.abs(X) < 0.055)
              * (s * X > 0), brow)
        # a nostril, and the fold from the nose to the mouth
        paint(smooth_edge(np.hypot((X - s * 0.0085) / 1.4, Z + 0.033) - 0.0028, 0.0012) * 0.8, (0.08, 0.03, 0.02))
        fold = np.abs(X - s * (0.02 - (Z + 0.035) * 0.25)) < 0.0015
        paint(fold * (Z < -0.03) * (Z > -0.07) * (0.2 + 0.45 * age), (0.12, 0.05, 0.03))
        if age > 0.5:  # crow's feet
            for k in (-1, 0, 1):
                out = s * X
                paint((np.abs(Z - ez - k * 0.004 - (out - 0.048) * 0.3 * k) < 0.0007) * (out > 0.047) * (out < 0.058)
                      * 0.6, (0.12, 0.05, 0.03))
    # the mouth: a darker line, an upper and a fuller lower lip
    mz = -0.066
    width = 0.021 if female else 0.023
    curve = np.clip(1 - (X / width) ** 2, 0, 1)
    paint(smooth_edge(np.abs(Z - (mz + 0.003)) - 0.0028 * curve, 0.001) * (np.abs(X) < width) * 0.85, shade(lip, 0.8))
    paint(smooth_edge(np.abs(Z - (mz - 0.0035)) - 0.0034 * curve, 0.001) * (np.abs(X) < width * 0.9) * 0.8, lip)
    paint(smooth_edge(np.abs(Z - mz) - 0.0007, 0.0006) * (np.abs(X) < width * 1.02), (0.05, 0.015, 0.012))
    if age > 0.5:  # a lined forehead
        for k in range(3):
            paint((np.abs(Z - 0.043 - k * 0.006 - 0.002 * np.cos(X * 60)) < 0.0008) * (np.abs(X) < 0.04) * 0.5,
                  (0.12, 0.05, 0.03))
    edge = np.clip(np.minimum.reduce([X + 0.07, 0.07 - X, Z + 0.1, 0.06 - Z]) / 0.006, 0, 1)
    img[..., 3] *= edge
    return np_image(name, img)


def cloth_material(name, female, age):
    """Facet colours ("Col") times a pattern chosen per face ("pat"), a
    painted face on the head, contact shadow (AO) and a fabric bump —
    everything the bake flattens into one texture."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_backface_culling = False
    k = NodeKit(m)
    bsdf = k.bsdf()
    vc = k.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    colour = vc.outputs["Color"]
    at = k.new("ShaderNodeAttribute")
    at.attribute_name = "pat"
    at.attribute_type = "GEOMETRY"
    pid = at.outputs["Fac"]
    uv = k.new("ShaderNodeUVMap")
    uv.uv_map = "Proc"
    weave, _t = modelkit.pat_weave(256, 24, 0.12, 0.2, 11)
    _sh, straw_t = modelkit.pat_weave(256, 10, 0.2, 0.3, 5)
    _rh, rope_t = modelkit.pat_rope(256, 3)
    pats = {
        P_FLORAL: (np_image(f"{name}_floral", pat_floral()), 0.15),
        P_PLAID: (np_image(f"{name}_plaid", pat_plaid()), 0.14),
        P_DENIM: (np_image(f"{name}_denim", pat_denim()), 0.012),
        P_STRAW: (np_image(f"{name}_straw", pat_tone(straw_t, 0.6, 1.15)), 0.035),
        P_WEAVE: (np_image(f"{name}_weave", pat_tone(weave, 0.9, 1.06)), 0.02),
        P_JUTE: (np_image(f"{name}_jute", pat_tone(rope_t, 0.6, 1.2)), 0.03),
        P_CANVAS: (np_image(f"{name}_canvas", pat_tone(weave, 0.82, 1.1)), 0.012),
    }
    for p, (img, tile) in pats.items():
        mp = k.new("ShaderNodeMapping", Vector=uv.outputs["UV"])
        mp.inputs["Scale"].default_value = (1.0 / tile, 1.0 / tile, 1.0)
        tex = k.new("ShaderNodeTexImage", Vector=mp.outputs["Vector"])
        tex.image = img
        tex.interpolation = "Cubic"
        mul = k.new("ShaderNodeMix")
        mul.data_type = "RGBA"
        mul.blend_type = "MULTIPLY"
        k.set(mul.inputs[0], 1.0)
        k.set(mul.inputs[6], colour)
        k.set(mul.inputs[7], tex.outputs["Color"])
        colour = k.mix(k.math("COMPARE", pid, float(p), 0.5), colour, mul.outputs[2])
    # the face, projected from the front onto the head
    tc = k.new("ShaderNodeTexCoord")
    sep = k.new("ShaderNodeSeparateXYZ", Vector=tc.outputs["Object"])
    z0 = HEAD_PIVOT[2] + (HEAD_Z - 0.1 - HEAD_PIVOT[2]) * HEAD_S
    u = k.math("MULTIPLY_ADD", sep.outputs["X"], 1.0 / (0.14 * HEAD_S), 0.5)
    v = k.math("MULTIPLY_ADD", sep.outputs["Z"], 1.0 / (0.16 * HEAD_S), -z0 / (0.16 * HEAD_S))
    comb = k.new("ShaderNodeCombineXYZ", X=u, Y=v)
    brow = (0.28, 0.27, 0.25) if age > 0.5 else (0.02, 0.014, 0.012)
    lip = (0.24, 0.05, 0.045) if female else (0.2, 0.075, 0.055)
    dec = k.new("ShaderNodeTexImage", Vector=comb.outputs["Vector"])
    dec.image = face_decal(f"{name}_face", female, age, brow, lip)
    dec.extension = "CLIP"
    dec.interpolation = "Cubic"
    geo = k.new("ShaderNodeNewGeometry")
    nsep = k.new("ShaderNodeSeparateXYZ", Vector=geo.outputs["Normal"])
    facing = k.remap(nsep.outputs["Y"], 0.1, 0.4, 0.0, 1.0)
    fmask = k.math("MULTIPLY", k.math("MULTIPLY", k.math("COMPARE", pid, float(P_FACE), 0.5), dec.outputs["Alpha"]),
                   facing)
    colour = k.mix(fmask, colour, dec.outputs["Color"])
    # contact shadow in the folds, under collars, hats and arms
    ao = k.new("ShaderNodeAmbientOcclusion", Distance=AO_DIST)
    ao.samples = 16
    ao.only_local = True
    shadow = k.remap(ao.outputs["AO"], 0.0, 1.0, 1.0 - AO_STRENGTH, 1.0)
    sc = k.new("ShaderNodeVectorMath")
    sc.operation = "SCALE"
    k.set(sc.inputs[0], colour)
    k.set(sc.inputs["Scale"], shadow)
    k.t.links.new(sc.outputs[0], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.82
    bn = k.new("ShaderNodeTexNoise", Vector=tc.outputs["Object"], Scale=160.0, Detail=3.0)
    bump = k.new("ShaderNodeBump", Height=bn.outputs["Fac"], Strength=0.12, Distance=0.002)
    k.t.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    alpha = k.new("ShaderNodeValue")  # the bake's alpha pass looks for this node
    alpha.outputs[0].default_value = 1.0
    alpha.name = "ALPHA"
    return m


# ---------------------------------------------------------------- the four
def llanero():
    p = Person("llanero", 0x11A0, (0.25, 0.105, 0.05))
    cream = (0.72, 0.66, 0.52)

    def suit(c, n):
        return mottle(c, cream, 0.05, 10.0)

    btn = (0.1, 0.08, 0.06)
    # the liquiliqui: a closed jacket to the hips
    p.torso(suit, P_WEAVE, grow=0.008, bottom=0.78, flare=0.016)
    p.neck()
    p.collar_stand(suit, P_WEAVE)
    p.buttons([1.505], btn, grow=0.02)
    p.buttons([1.42, 1.32, 1.22, 1.12, 1.02, 0.92], btn, grow=0.008)
    for s in (-1, 1):
        p.pocket(f"chest{s}", s * 0.085, 1.3, 0.072, 0.07, suit, P_WEAVE, grow=0.008, button=btn)
        p.pocket(f"hip{s}", s * 0.09, 0.88, 0.078, 0.085, suit, P_WEAVE, grow=0.014, button=btn)
        p.arm(s, suit, 1.0, P_WEAVE, cuff=0.004)
    p.hands()
    p.legs(suit, P_WEAVE, r_top=0.1, r_knee=0.074, r_hem=0.07)
    navy = (0.018, 0.02, 0.03)
    for s in (-1, 1):
        p.shoe(s, "alpargata", lambda c, n: mottle(c, navy, 0.12, 30.0), flat(JUTE))
    p.head(lambda c, n: mottle(c, (0.016, 0.012, 0.011), 0.2, 40.0), hair_short, brows=(0.02, 0.014, 0.012),
           hair_bump=combed)
    black = (0.018, 0.017, 0.017)
    p.hat(0.104, 0.1, 0.235, 1.72, lambda c, n: mottle(c, black, 0.2, 18.0), flat((0.006, 0.006, 0.006)),
          P_WEAVE, pinch=0.25, curl=0.01, top=0.06)
    return p


def coplera():
    p = Person("coplera", 0xC0B1, (0.21, 0.085, 0.04), female=True)
    white = (0.8, 0.78, 0.73)

    def blouse(c, n):
        return mottle(c, white, 0.04, 14.0) if c.z < 1.43 else p.skin_paint(c, n)

    p.torso(blouse, P_WEAVE, bottom=0.92)
    p.neck()

    # the ruffled scoop neckline: a flared band dipping at the front
    def ruffle_r(c):
        base = ellipse(c.th, 0.098, 0.162) + 0.01 + 0.03 * c.t
        return base * (1.0 + 0.08 * math.sin(c.th * 16))

    def ruffle_d(c):
        return V(0, 0, -0.03 * max(math.cos(c.th), 0.0) ** 2)

    ruf = tube([V(0, 0.0, 1.455), V(0, 0.0, 1.39)], 3, 64, ruffle_r, ref=FRONT, raw=True, cap=False, ru=0.14,
               disp=ruffle_d)
    p.part("ruffle", ruf, body_w, lambda c, n: mottle(c, white, 0.05, 16.0), P_WEAVE, solidify=0.004)
    for s in (-1, 1):
        p.arm(s, lambda c, n: mottle(c, white, 0.04, 14.0), 0.34, P_WEAVE, puff=0.024,
              ruffle=lambda c, n: mottle(c, white, 0.05, 16.0))
    p.hands()
    p.legs(p.skin_paint, top=0.72, skin=True)

    def sr(c):
        z = c.p.z
        rr = ellipse(c.th, tab(z, [0.45, 0.6, 0.78, 0.92, 1.035], [0.24, 0.225, 0.19, 0.14, 0.096]),
                     tab(z, [0.45, 0.6, 0.78, 0.92, 1.035], [0.28, 0.262, 0.228, 0.18, 0.134]))
        gather = sstep(0.98, 0.7, z)  # pleats from the gathered waist down
        return rr * (1.0 + 0.03 * gather * math.sin(c.th * 14 + 0.8) + 0.012 * math.sin(c.th * 31) * gather)

    n = 40
    skirt = tube([V(0, 0.0, lerp(1.04, 0.52, i / (n - 1))) for i in range(n)], n, 84, sr, ref=FRONT, raw=True,
                 cap=False, ru=0.24)
    p.part("skirt", skirt, skirt_w, flat(WHITE), P_FLORAL, solidify=0.004)

    def hr(c):  # the ruffled hem tier
        rr = ellipse(c.th, lerp(0.232, 0.27, c.t), lerp(0.27, 0.305, c.t))
        return rr * (1.0 + 0.05 * math.sin(c.th * 26) * c.t + 0.02 * math.sin(c.th * 14 + 0.8))

    hem = tube([V(0, 0.0, 0.54), V(0, 0.0, 0.42)], 6, 104, hr, ref=FRONT, raw=True, cap=False, ru=0.25)
    p.part("hem", hem, skirt_w, flat(WHITE), P_FLORAL, solidify=0.004)
    wb = vloft([1.0, 1.045], [0.136, 0.138], [0.098, 0.1], sides=28, rows=2)
    p.part("waistband", wb, body_w, flat((0.24, 0.03, 0.018)), solidify=0.003)
    navy = (0.018, 0.02, 0.03)
    for s in (-1, 1):
        p.shoe(s, "alpargata", lambda c, n: mottle(c, navy, 0.12, 30.0), flat(JUTE))
    hair = (0.012, 0.009, 0.008)

    def hair_keep(c):
        cz = HEAD_Z
        if c.y > 0.05:
            return c.z > cz + 0.064 - 0.1 * max(0.0, c.y - 0.075) + 1.8 * c.x * c.x
        if abs(c.x) > 0.058 and c.y > -0.02:
            return c.z > cz - 0.018
        return c.z > cz - 0.08

    def hair_paint(c, n):
        col = mottle(c, hair, 0.35, 60.0)
        if abs(c.x) < 0.005 and c.z > 1.7 and c.y > -0.02:  # the parting
            return shade(col, 0.4)
        return col

    def bun():
        centre = V(0, -0.105, 1.588)
        b = blob(tuple(centre), (0.05, 0.042, 0.044), 16, 10)
        for v in b.verts:  # twisted
            d = v.co - centre
            v.co += d.normalized() * 0.005 * math.sin(math.atan2(d.z, d.x) * 6 + d.y * 80)
        p.part("bun", b, "Head", hair_paint)
        red = (0.42, 0.012, 0.012)
        p.part("ribbon", blob((0.0, -0.078, 1.612), (0.052, 0.014, 0.016), 12, 6), "Head", flat(red))
        for s in (-1, 1):
            loop = blob((s * 0.03, -0.085, 1.628), (0.022, 0.008, 0.016), 10, 6, rot=Matrix.Rotation(s * 0.5, 3, "Y"))
            p.part(f"bow{s}", loop, "Head", flat(red))
            tail = modelkit.ribbon([V(s * 0.012, -0.088, 1.61), V(s * 0.03, -0.1, 1.56), V(s * 0.028, -0.106, 1.515)],
                                   0.02, 5, ref=FRONT)
            p.part(f"ribbon_tail{s}", tail, "Head", flat(red), solidify=0.003)

    p.head(hair_paint, hair_keep, extra=bun, hair_bump=combed)
    return p


def encargado():
    p = Person("encargado", 0xE7CA, (0.3, 0.13, 0.065), belly=1.0, age=1.0)
    khaki = (0.5, 0.42, 0.28)
    trousers = (0.06, 0.056, 0.05)

    def shirt(c, n):
        return mottle(c, khaki, 0.06, 12.0)

    btn = (0.24, 0.22, 0.18)
    p.torso(lambda c, n: shirt(c, n) if c.z > 0.975 else mottle(c, trousers, 0.1, 14.0), P_WEAVE, grow=0.006,
            bottom=0.9)
    p.neck()
    p.collar_open(shirt, P_WEAVE)
    p.buttons([1.4, 1.3, 1.2, 1.1, 1.02], btn, grow=0.006)
    for s in (-1, 1):
        p.pocket(f"chest{s}", s * 0.088, 1.29, 0.078, 0.082, shirt, P_WEAVE, grow=0.006, button=btn)
        p.arm(s, shirt, 0.6, P_WEAVE, roll=shirt)
    p.hands()
    # tucked into the boots, bloused over their tops
    p.legs(lambda c, n: mottle(c, trousers, 0.12, 14.0), P_WEAVE, hem=0.36, r_top=0.102, r_knee=0.078, r_hem=0.084,
           folds=2.2)
    p.belt(z=0.99)
    rubber = (0.016, 0.016, 0.017)
    for s in (-1, 1):
        p.boot(s, rubber)
    red = (0.4, 0.03, 0.03)

    def towel(c, n):
        v = (c.z * 44.0) % 4.0
        base = mottle(c, (0.72, 0.7, 0.66), 0.05, 20.0)
        if c.z < 0.82 and 1.0 < v < 1.7:
            return red
        if c.z < 0.82 and 2.3 < v < 2.6:
            return mixc(base, red, 0.6)
        return base

    tw = modelkit.ribbon([V(0.135, 0.088, 0.99), V(0.15, 0.1, 0.86), V(0.155, 0.095, 0.7)], 0.1, 10, ref=FRONT,
                         taper=0.0)
    p.part("towel", tw, leg_w, towel, P_WEAVE, solidify=0.006)
    grey = (0.34, 0.33, 0.31)

    def moustache():
        m = limb([V(-0.034, 0.093, HEAD_Z - 0.066), V(-0.016, 0.104, HEAD_Z - 0.05), V(0, 0.106, HEAD_Z - 0.048),
                  V(0.016, 0.104, HEAD_Z - 0.05), V(0.034, 0.093, HEAD_Z - 0.066)], 12, 8,
                 lambda c: ellipse(c.th, 0.006 + 0.004 * math.sin(c.t * math.pi),
                                   0.01 * math.sin(c.t * math.pi) + 0.003))
        p.part("moustache", m, "Head", lambda c, n: mottle(c, grey, 0.2, 80.0))

    p.head(lambda c, n: mottle(c, grey, 0.2, 60.0), hair_short, brows=grey, extra=moustache, jaw=1.1,
           hair_bump=combed)
    straw = (0.64, 0.48, 0.22)
    p.hat(0.11, 0.1, 0.255, 1.72, lambda c, n: mottle(c, straw, 0.12, 40.0), flat((0.05, 0.03, 0.014)), P_STRAW,
          pinch=0.35, curl=0.028, droop=0.012)
    return p


def muchacho():
    p = Person("muchacho", 0x3C40, (0.25, 0.1, 0.048))
    tee = (0.8, 0.79, 0.76)
    jeans = (0.1, 0.2, 0.42)

    def tee_paint(c, n):
        if c.z < 0.955:
            return mottle(c, jeans, 0.08, 20.0)
        vz = 1.47 - 0.9 * abs(c.x)  # the V of the neck shows skin
        if c.y > 0 and c.z > vz - 0.06 and abs(c.x) < 0.06:
            return p.skin_paint(c, n)
        return mottle(c, tee, 0.03, 16.0) if c.z < 1.47 else p.skin_paint(c, n)

    p.torso(tee_paint, bottom=0.9)
    p.neck()

    def open_front(t):  # the overshirt is left open down the front, wider at the neck
        g = 0.5 + 0.35 * sstep(0.6, 0.95, t)
        return (g, TAU - g)

    p.torso(flat(WHITE), P_PLAID, grow=0.014, bottom=0.86, top=1.495, name="overshirt", flare=0.01, span=open_front)
    p.collar_open(flat(WHITE), P_PLAID, spread=0.026, gap=0.8, lapel=1.4)
    for s in (-1, 1):
        p.pocket(f"chest{s}", s * 0.105, 1.3, 0.07, 0.074, flat(WHITE), P_PLAID, grow=0.014)
        p.arm(s, flat(WHITE), 0.42, P_PLAID, cuff=0.006)
    p.hands()
    bracelet = tube([V(-0.232, 0.0, 0.9), V(-0.234, 0.0, 0.884)], 2, 12, 0.036, ref=FRONT, raw=True, cap=False,
                    ru=0.04)
    p.part("bracelet", bracelet, arm_w, flat((0.45, 0.02, 0.02)), solidify=0.005)
    p.legs(lambda c, n: mottle(c, jeans, 0.08, 20.0), P_DENIM, r_top=0.095, r_knee=0.069, r_hem=0.064, folds=1.4)
    p.belt(z=0.975, colour=(0.035, 0.02, 0.01), buckle=(0.3, 0.3, 0.3))
    for s in (-1, 1):
        p.shoe(s, "sneaker", flat((0.03, 0.03, 0.035)), flat((0.75, 0.74, 0.72)))
    hair = (0.014, 0.011, 0.01)

    def messy(co):
        return 0.022 * max(0.0, nz(co, 40.0, 3.0)) + 0.014 * sstep(1.68, 1.77, co.z)

    p.head(lambda c, n: mottle(c, hair, 0.35, 50.0), hair_short, brows=(0.018, 0.013, 0.011), hair_bump=messy)
    return p


BUILDERS = {"llanero": llanero, "coplera": coplera, "encargado": encargado, "muchacho": muchacho}


# ---------------------------------------------------------------- presentation
def portrait(coll, path, size=(360, 540)):
    """The menu's portrait: three-quarter front, head to shins, a warm key
    and a cold rim, on a transparent background."""
    sc = bpy.data.scenes.new("_portrait")
    made = []
    try:
        sc.collection.children.link(coll)
        modelkit.setup_cycles(sc, 128)
        sc.cycles.use_denoising = True
        sc.render.film_transparent = True
        sc.view_settings.view_transform = "AgX"
        sc.view_settings.look = "AgX - Medium High Contrast"
        world = bpy.data.worlds.new("_portrait_world")
        made.append(world)
        sc.world = world
        world.use_nodes = True
        bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
        bg.inputs["Color"].default_value = (0.02, 0.025, 0.045, 1)
        bg.inputs["Strength"].default_value = 0.6
        cam = bpy.data.objects.new("_pcam", bpy.data.cameras.new("_pcam"))
        sc.collection.objects.link(cam)
        sc.camera = cam
        cam.data.lens = 70
        centre = V(0, 0, 1.2)
        yaw = 0.42
        cam.location = centre + V(math.sin(yaw), math.cos(yaw), 0.06) * 3.35
        cam.rotation_euler = (centre - cam.location).to_track_quat("-Z", "Y").to_euler()
        for nm, at, energy, colour in (("key", V(1.4, 2.2, 1.9), 260, (1.0, 0.82, 0.62)),
                                       ("rim", V(-1.6, -1.8, 2.3), 420, (0.55, 0.68, 1.0)),
                                       ("fill", V(-2.0, 1.6, 1.0), 55, (0.6, 0.7, 1.0))):
            ld = bpy.data.lights.new("_p" + nm, "AREA")
            ld.energy, ld.color, ld.size = energy, colour, 1.2
            lo = bpy.data.objects.new("_p" + nm, ld)
            sc.collection.objects.link(lo)
            lo.location = at
            lo.rotation_euler = (V(0, 0, 1.2) - at).to_track_quat("-Z", "Y").to_euler()
        sc.render.resolution_x, sc.render.resolution_y = size
        sc.render.filepath = path
        with bpy.context.temp_override(**modelkit.view3d_override()):
            bpy.ops.render.render(write_still=True, scene=sc.name)
        print("portrait", path)
    finally:
        for o in list(sc.collection.objects):
            data = o.data
            bpy.data.objects.remove(o, do_unlink=True)
            if isinstance(data, bpy.types.Light):
                bpy.data.lights.remove(data)
            elif isinstance(data, bpy.types.Camera):
                bpy.data.cameras.remove(data)
        for w in made:
            bpy.data.worlds.remove(w)
        bpy.data.scenes.remove(sc)


def main():
    built = {}
    names = ONLY or NAMES
    for name in names:
        p = BUILDERS[name]()
        joined, rig = p.finish(None if LIVE else out_path(name))
        tris = 0
        for ob in joined.values():
            ob.data.calc_loop_triangles()
            tris += len(ob.data.loop_triangles)
        print(f"survivor_{name} tris", tris)
        built[name] = p
        if LIVE:  # side by side for a look in the viewport; exports stay at the origin
            rig.location.x = 0.9 * names.index(name)
        if RENDERS:
            views = [("front", math.pi, 0.06, 0.95), ("three", 2.5, 0.08, 0.95), ("back", 0.0, 0.06, 0.95),
                     ("face", 2.8, 0.05, 0.22, (0, 0.05, 1.63))]
            render_views(p.a.coll, os.path.join(RENDERS, name), views, samples=96, size=(700, 1000))
        if PORTRAITS:
            os.makedirs(PORTRAITS, exist_ok=True)
            portrait(p.a.coll, os.path.join(PORTRAITS, f"{name}.png"))
    if BLEND:
        bpy.ops.wm.save_as_mainfile(filepath=BLEND, copy=True)
    return built


if __name__ == "__main__":
    main()

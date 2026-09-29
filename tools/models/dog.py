"""Tureco, after the "Tureco" concept sheet: a lean llanero farm dog in the
sheet's faceted low-poly style. Brown coat with a dark saddle, cream chest,
throat, belly and socks, a dark muzzle, tall upright ears, amber eyes, dark
claws, a dark-tipped tail, a worn leather collar with a brass ring and a
rope loop.

Flat per-facet colours (no textures). Skinned to the joints the game moves
(`src/world/dog.rs`): Body, Head (0, 0.68, -0.38), four legs at y 0.46,
plus Jaw (for the bark) and Tail. Bevy space, facing -Z, paws on the ground.

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/dog.py -- [--renders DIR]
"""

import importlib
import math
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402

importlib.reload(modelkit)
from modelkit import (  # noqa: E402
    FRONT, ROOT, TAU, UP, V, Asset, clamp, ellipse, facet_material, gauss, lerp, nz, render_views, sstep, tab, tube,
)

ARGS = globals().get("ARGS") or (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
STAGE = "preview" if "--preview" in ARGS else "final"
OUT = os.path.join(ROOT, "assets", "models", "tureco.glb")
RENDERS = ARGS[ARGS.index("--renders") + 1] if "--renders" in ARGS else None

# Linear colours after the sheet's fur swatches.
FUR = (0.42, 0.165, 0.055)
FUR_LIGHT = (0.58, 0.28, 0.11)
SADDLE = (0.14, 0.055, 0.02)
CREAM = (0.66, 0.44, 0.25)
MUZZLE = (0.045, 0.022, 0.012)
BLACK = (0.01, 0.009, 0.008)
EAR_IN = (0.3, 0.14, 0.08)
LEATHER = (0.085, 0.05, 0.03)
BRASS = (0.42, 0.28, 0.09)
ROPE = (0.36, 0.26, 0.15)
IRIS = (0.3, 0.12, 0.02)

JOINTS = [
    ("Body", None, (0.0, 0.0, 0.0)),
    ("Head", "Body", (0.0, 0.68, -0.38)),
    ("Jaw", "Head", (0.0, 0.685, -0.42)),
    ("LegFL", "Body", (-0.08, 0.46, -0.24)),
    ("LegFR", "Body", (0.08, 0.46, -0.24)),
    ("LegBL", "Body", (-0.08, 0.46, 0.24)),
    ("LegBR", "Body", (0.08, 0.46, 0.24)),
    ("Tail", "Body", (0.0, 0.55, 0.37)),
]


def mixc(a, b, t):
    t = clamp(t)
    return tuple(lerp(x, y, t) for x, y in zip(a, b))


def mottle(c, base, amt=0.18, s=9.0):
    k = 1.0 + amt * nz(c, s, 3.0)
    return tuple(x * k for x in base)


def flat(colour):
    return lambda c, n: colour


# ---------------------------------------------------------------- painters (Blender coords: +Y forward, -X left)
def paint_torso(c, n):
    col = mottle(c, mixc(FUR, FUR_LIGHT, 0.5 + 0.5 * nz(c, 4.0, 1.0)))
    col = mixc(col, SADDLE, sstep(0.3, 0.85, n.z) * sstep(-0.42, -0.25, c.y) * (1 - sstep(0.16, 0.3, c.y)))
    under = sstep(-0.15, -0.65, n.z)
    chest = sstep(0.14, 0.3, c.y) * sstep(0.2, 0.6, n.y) * (1 - sstep(0.55, 0.64, c.z))
    return mixc(col, CREAM, max(under, chest))


def paint_neck(c, n):
    col = mottle(c, FUR)
    col = mixc(col, SADDLE, sstep(0.3, 0.9, n.z - n.y) * 0.8)
    return mixc(col, CREAM, sstep(0.0, 0.5, n.y - n.z * 0.5) * (1 - sstep(0.66, 0.74, c.z)))


def paint_head(c, n):
    col = mottle(c, mixc(FUR_LIGHT, FUR, sstep(0.72, 0.78, c.z)))
    col = mixc(col, SADDLE, sstep(0.5, 0.95, n.z) * sstep(0.34, 0.44, c.y) * 0.7)
    col = mixc(col, CREAM, sstep(0.3, 0.9, abs(n.x)) * gauss(c.z - 0.705, 0.02) * sstep(0.4, 0.46, c.y) * 0.7)
    return mixc(col, MUZZLE, sstep(0.49, 0.57, c.y))


def paint_jaw(c, n):
    col = mixc(mottle(c, FUR_LIGHT), CREAM, sstep(-0.2, -0.8, n.z))
    return mixc(col, MUZZLE, sstep(0.5, 0.55, c.y))


def paint_ear(c, n):
    return mixc(mottle(c, SADDLE, 0.3), EAR_IN, sstep(0.0, 0.5, n.y))


def paint_leg(side):
    def f(c, n):
        col = mottle(c, FUR)
        inner = sstep(0.2, 0.8, -side * n.x)
        col = mixc(col, CREAM, max(inner * 0.6, sstep(0.17, 0.08, c.z)))
        col = mixc(col, CREAM, sstep(0.3, 0.8, n.y) * sstep(0.46, 0.3, c.z) * 0.5)
        return col

    return f


def paint_tail(c, n):
    col = mottle(c, FUR)
    col = mixc(col, CREAM, sstep(-0.1, -0.7, n.y) * 0.5)
    return mixc(col, SADDLE, sstep(0.42, 0.3, c.z))


# ---------------------------------------------------------------- weights
def leg_w(bone):
    def f(co):
        up = sstep(0.43, 0.52, co.z)
        return {bone: 1.0 - up, "Body": up}

    return f


def neck_w(co):
    t = sstep(0.31, 0.38, co.y)
    return {"Body": 1.0 - t, "Head": t}


def tail_w(co):
    t = sstep(-0.35, -0.41, co.y)
    return {"Body": 1.0 - t, "Tail": t}


# ---------------------------------------------------------------- geometry
def blob(centre, radii, sides=10, rings=6):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=sides, v_segments=rings, radius=1.0)
    bmesh.ops.scale(bm, vec=V(*radii), verts=bm.verts)
    bmesh.ops.translate(bm, vec=V(*centre), verts=bm.verts)
    return bm


def build_body(a):
    ys = [-0.41, -0.33, -0.18, -0.02, 0.12, 0.22, 0.31, 0.37]
    top = [0.55, 0.6, 0.59, 0.6, 0.62, 0.635, 0.63, 0.6]
    bot = [0.49, 0.43, 0.46, 0.43, 0.355, 0.35, 0.41, 0.5]
    wid = [0.055, 0.1, 0.088, 0.098, 0.114, 0.116, 0.1, 0.062]
    path = [V(0, y, (t + b) / 2) for y, t, b in zip(ys, top, bot)]

    def r(c):
        y = c.p.y
        h = (tab(y, ys, top) - tab(y, ys, bot)) / 2
        w = tab(y, ys, wid)
        rr = ellipse(c.th, h, w)
        if math.cos(c.th) < 0:  # a keel of a chest, narrow underneath
            rr *= 1 - 0.28 * math.cos(c.th) ** 2 * sstep(-0.05, 0.12, y)
        for sgn in (-1, 1):  # shoulder blades and hips
            rr += 0.012 * gauss(math.sin(c.th) - sgn * 0.7, 0.3) * gauss(y - 0.2, 0.06)
            rr += 0.012 * gauss(math.sin(c.th) - sgn * 0.6, 0.3) * gauss(y + 0.3, 0.06)
        rr += 0.004 * math.sin(c.th * 2) * gauss(y - 0.05, 0.08)  # ribs through the coat
        return rr

    bm = tube(path, 30, 20, r, ref=UP, cap=True, ru=0.1)
    a.part("torso", bm, "facet", "Body", smooth=False, decimate=0.45, paint=paint_torso)
    neck = tube([V(0, 0.2, 0.585), V(0, 0.29, 0.64), V(0, 0.35, 0.7), V(0, 0.39, 0.735)], 12, 14,
                lambda c: ellipse(c.th, lerp(0.095, 0.068, c.t), lerp(0.085, 0.06, c.t)), ref=UP, cap=True, ru=0.1)
    a.part("neck", neck, "facet", neck_w, smooth=False, decimate=0.6, paint=paint_neck)


def build_head(a):
    path = [V(0, 0.322, 0.75), V(0, 0.378, 0.76), V(0, 0.436, 0.744), V(0, 0.488, 0.719), V(0, 0.536, 0.703),
            V(0, 0.574, 0.694)]

    def r(c):
        t = c.t
        h = tab(t, [0.0, 0.12, 0.35, 0.5, 0.7, 0.9, 1.0], [0.034, 0.068, 0.066, 0.05, 0.041, 0.033, 0.015])
        w = tab(t, [0.0, 0.12, 0.35, 0.5, 0.7, 0.9, 1.0], [0.042, 0.08, 0.074, 0.05, 0.037, 0.03, 0.014])
        rr = ellipse(c.th, h * 1.1, w * 1.14)
        side = abs(math.sin(c.th))
        rr -= 0.01 * gauss(side - 0.75, 0.2) * gauss(t - 0.44, 0.06) * (math.cos(c.th) > 0)  # eye sockets
        rr += 0.01 * gauss(side - 0.95, 0.2) * gauss(t - 0.36, 0.1)  # cheeks
        if math.cos(c.th) < 0:
            rr *= 1 - 0.35 * sstep(0.45, 0.6, t) * math.cos(c.th) ** 2  # the jaw sits below
        return rr

    a.part("head", tube(path, 26, 16, r, ref=UP, cap=True, ru=0.06), "facet", "Head", smooth=False, decimate=0.55,
           paint=paint_head)
    jaw = tube([V(0, 0.4, 0.699), V(0, 0.47, 0.684), V(0, 0.546, 0.677)], 12, 10,
               lambda c: ellipse(c.th, lerp(0.024, 0.011, c.t), lerp(0.036, 0.016, c.t)), ref=UP, cap=True, ru=0.04)
    a.part("jaw", jaw, "facet", "Jaw", smooth=False, decimate=0.7, paint=paint_jaw)
    a.part("nose", blob((0, 0.577, 0.697), (0.021, 0.016, 0.015), 8, 5), "facet", "Head", smooth=False,
           paint=flat(BLACK))
    a.part("mouth", tube([V(0, 0.45, 0.69), V(0, 0.552, 0.683)], 2, 8,
                         lambda c: ellipse(c.th, 0.005, lerp(0.034, 0.02, c.t)), ref=UP, raw=True, cap=True, ru=0.02),
           "facet", "Head", smooth=False, paint=flat(BLACK))
    for s in (1, -1):
        eye = blob((s * 0.052, 0.466, 0.752), (0.011, 0.012, 0.011), 8, 5)
        a.part(f"eye{s}", eye, "facet", "Head", smooth=False,
               paint=lambda c, n: IRIS if n.y > 0.55 and abs(n.x) < 0.8 else BLACK)
        base, tip = V(s * 0.05, 0.365, 0.79), V(s * 0.088, 0.338, 0.94)

        def ear_r(c):
            w = lerp(0.046, 0.003, c.t ** 0.85)
            return ellipse(c.th, w * 0.62, w)

        ear = tube([base, (base + tip) / 2 + V(s * 0.004, 0.006, 0), tip], 8, 10, ear_r, ref=FRONT,
                   span=lambda t: (0.3 * math.pi, 1.7 * math.pi), closed=False, ru=0.03)
        a.part(f"ear{s}", ear, "facet", "Head", smooth=False, solidify=0.006, paint=paint_ear)


def paw(a, centre, side, bone, name):
    x, y, z = centre
    a.part(name, blob((x, y, z), (0.026, 0.038, 0.021), 8, 5), "facet", bone, smooth=False,
           paint=lambda c, n: mottle(c, CREAM, 0.1))
    for i in range(4):
        dx = (i - 1.5) * 0.012
        tip = V(x + dx, y + 0.046, 0.004)
        bm = tube([V(x + dx, y + 0.03, 0.011), tip], 2, 5, lambda c: lerp(0.0042, 0.0012, c.t), ref=UP, raw=True,
                  cap=True, ru=0.01)
        a.part(f"{name}_claw{i}", bm, "facet", bone, smooth=False, paint=flat(BLACK))


def build_legs(a):
    for s in (1, -1):
        side = "R" if s > 0 else "L"
        x = s * 0.072
        front = [V(x * 0.62, 0.232, 0.52), V(x, 0.228, 0.44), V(x * 1.05, 0.212, 0.33), V(x, 0.222, 0.2), V(x, 0.232, 0.1),
                 V(x, 0.246, 0.05), V(x, 0.262, 0.03)]

        def fr(c):
            z = c.p.z
            return ellipse(c.th, tab(z, [0.03, 0.1, 0.2, 0.33, 0.44, 0.52], [0.023, 0.021, 0.026, 0.038, 0.055,
                                                                              0.05]),
                           tab(z, [0.03, 0.1, 0.2, 0.33, 0.44, 0.52], [0.021, 0.018, 0.021, 0.03, 0.043, 0.038]))

        a.part(f"leg_f{side}", tube(front, 18, 10, fr, ref=FRONT, cap=True, ru=0.04), "facet", leg_w(f"LegF{side}"),
               smooth=False, decimate=0.6, paint=paint_leg(s))
        paw(a, (x, 0.28, 0.021), s, f"LegF{side}", f"paw_f{side}")
        xb = s * 0.078
        rear = [V(xb * 0.6, -0.255, 0.53), V(xb, -0.21, 0.455), V(xb, -0.17, 0.35), V(xb, -0.235, 0.24), V(xb, -0.3, 0.14),
                V(xb, -0.292, 0.07), V(xb, -0.272, 0.03)]

        def rr(c):
            t = c.t
            return ellipse(c.th, tab(t, [0, 0.22, 0.45, 0.65, 0.8, 1], [0.065, 0.078, 0.04, 0.029, 0.022, 0.022]),
                           tab(t, [0, 0.22, 0.45, 0.65, 0.8, 1], [0.042, 0.054, 0.031, 0.022, 0.019, 0.02]))

        a.part(f"leg_b{side}", tube(rear, 20, 10, rr, ref=FRONT, cap=True, ru=0.04), "facet", leg_w(f"LegB{side}"),
               smooth=False, decimate=0.6, paint=paint_leg(s))
        paw(a, (xb, -0.255, 0.021), s, f"LegB{side}", f"paw_b{side}")


def build_tail(a):
    path = [V(0, -0.37, 0.565), V(0, -0.445, 0.545), V(0, -0.505, 0.47), V(0, -0.53, 0.37), V(0, -0.52, 0.27),
            V(0, -0.495, 0.2)]

    def r(c):
        rr = tab(c.t, [0, 0.3, 0.6, 0.85, 1], [0.032, 0.044, 0.04, 0.026, 0.007])
        return rr * (1 + 0.15 * nz(c.q, 30.0, 2.0))

    a.part("tail", tube(path, 16, 9, r, ref=UP, cap=True, ru=0.04), "facet", tail_w, smooth=False, decimate=0.7,
           paint=paint_tail)


def build_collar(a):
    centre = V(0, 0.305, 0.648)
    axis = V(0, 0.62, 0.78).normalized()
    u = V(1, 0, 0)
    w = axis.cross(u)
    ring = []
    for i in range(33):
        t = TAU * i / 32
        rr = 0.083 + 0.004 * math.sin(t)
        ring.append(centre + (u * math.cos(t) + w * math.sin(t)) * rr)
    band = tube(ring, 33, 6, lambda c: ellipse(c.th, 0.017, 0.006), ref=axis, loop=True, raw=True, ru=0.02,
                transport=False)
    a.part("collar", band, "facet", neck_w, smooth=False,
           paint=lambda c, n: mottle(c, LEATHER, 0.25, 30.0))
    low = centre - w * 0.09  # under the throat
    for k, t in enumerate((-0.9, -0.3, 0.3, 0.9)):
        p = centre + (u * math.cos(-math.pi / 2 + t) + w * math.sin(-math.pi / 2 + t)) * 0.09
        a.part(f"stud{k}", blob(p, (0.006, 0.006, 0.006), 6, 4), "facet", neck_w, smooth=False, paint=flat(BRASS))
    ring2 = [low + V(0, 0.012 * math.sin(t), -0.014 - 0.014 * math.cos(t)) + V(0.014 * 0, 0, 0) for t in
             [TAU * i / 12 for i in range(13)]]
    a.part("ring", tube(ring2, 13, 5, 0.0035, ref=V(1, 0, 0), loop=True, raw=True, ru=0.01, transport=False),
           "facet", neck_w, smooth=False, paint=flat(BRASS))
    loop = [low + V(0.0, 0.0, -0.028), low + V(0.012, 0.01, -0.05), low + V(0.006, 0.012, -0.085),
            low + V(-0.008, 0.008, -0.09), low + V(-0.012, 0.0, -0.06), low + V(-0.002, -0.004, -0.03)]
    a.part("rope", tube(loop, 16, 6, 0.0055, ref=V(0, 1, 0), ru=0.01, cap=True), "facet", neck_w, smooth=False,
           paint=lambda c, n: mottle(c, ROPE, 0.2, 60.0))


def build():
    a = Asset("tureco", 0x7DC)
    a.mat("facet", facet_material("tureco_fur", 0.9), "facet")
    build_body(a)
    build_head(a)
    build_legs(a)
    build_tail(a)
    build_collar(a)
    return a


def main():
    a = build()
    if STAGE == "preview":
        return a
    joined, rig = a.finish(out=OUT, joints=JOINTS)
    tris = 0
    for ob in joined.values():
        ob.data.calc_loop_triangles()
        tris += len(ob.data.loop_triangles)
    print("tureco tris", tris)
    if RENDERS:
        views = [("three", 2.35, 0.12, 1.0), ("front", math.pi, 0.08, 1.0), ("side", math.pi / 2, 0.05, 1.0),
                 ("back", 0.0, 0.1, 1.0), ("head", 2.6, 0.12, 0.32, (0, 0.47, 0.75)),
                 ("paw", 2.4, 0.3, 0.2, (0.075, 0.27, 0.05))]
        render_views(a.coll, RENDERS, views, samples=160, size=(1000, 900))
    return a


if __name__ == "__main__":
    main()

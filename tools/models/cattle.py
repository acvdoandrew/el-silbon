"""The corral herd, after the "Corral Herd" concept sheet: zebu (Brahman-
like) cattle of the llanos in the sheet's faceted low-poly style. One
parametric body gives three variants: the cow (dusty white-grey, modest
hump, loose dewlap, short curved horns), the bull (larger, light brown-grey,
big hump, thick neck) and the calf (reddish brown, slim, horn nubs).

Flat per-facet colours, no rig (the game moves each cow as one mesh, see
`src/world/herd.rs`). Each variant is its own glTF, facing -Z in Bevy,
hooves on the ground.

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/cattle.py -- [--renders DIR]
"""

import importlib
import math
import os
import sys

import bmesh
from mathutils import Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402

importlib.reload(modelkit)
from modelkit import (  # noqa: E402
    FRONT, ROOT, UP, V, Asset, clamp, ellipse, facet_material, gauss, lerp, nz, render_views, sstep, tab, tube,
)

ARGS = globals().get("ARGS") or (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
STAGE = "preview" if "--preview" in ARGS else "final"
RENDERS = ARGS[ARGS.index("--renders") + 1] if "--renders" in ARGS else None
ONLY = ARGS[ARGS.index("--only") + 1].split(",") if "--only" in ARGS else None

BLACK = (0.012, 0.011, 0.01)
HOOF = (0.02, 0.017, 0.015)
HORN = (0.05, 0.045, 0.04)
EAR_IN = (0.3, 0.1, 0.035)

VARIANTS = {
    "cow": dict(scale=1.0, hump=0.13, neck=1.0, dewlap=1.0, horn=1.0, head=1.15, legs=1.0, udder=True,
                coat=(0.56, 0.5, 0.41), coat_dark=(0.38, 0.33, 0.26), seed=0xC01),
    "bull": dict(scale=1.12, hump=0.3, neck=1.35, dewlap=1.45, horn=1.2, head=1.2, legs=0.97, udder=False,
                 coat=(0.4, 0.3, 0.2), coat_dark=(0.22, 0.17, 0.12), seed=0xB01),
    "calf": dict(scale=0.6, hump=0.05, neck=0.85, dewlap=0.4, horn=0.25, head=1.18, legs=1.12, udder=False,
                 coat=(0.36, 0.13, 0.05), coat_dark=(0.22, 0.07, 0.03), seed=0xCA1),
}


def mixc(a, b, t):
    t = clamp(t)
    return tuple(lerp(x, y, t) for x, y in zip(a, b))


def mottle(c, base, amt=0.12, s=5.0):
    k = 1.0 + amt * nz(c, s, 3.0)
    return tuple(x * k for x in base)


def flat(colour):
    return lambda c, n: colour


def blob(centre, radii, sides=10, rings=6):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=sides, v_segments=rings, radius=1.0)
    bmesh.ops.scale(bm, vec=V(*radii), verts=bm.verts)
    bmesh.ops.translate(bm, vec=V(*centre), verts=bm.verts)
    return bm


def build_variant(name, p):
    a = Asset(name, p["seed"])
    a.mat("facet", facet_material(f"{name}_coat", 0.85), "facet")
    lift = (p["legs"] - 1.0) * 0.72  # longer legs raise the whole body
    coat, dark = p["coat"], p["coat_dark"]

    def paint_body(c, n):
        col = mottle(c, coat)
        # darker over the hump, neck and shoulders (as the bulls are), lighter underneath
        col = mixc(col, dark, sstep(0.2, 0.75, c.y) * sstep(1.05 + lift, 1.35 + lift, c.z) * 0.55)
        col = mixc(col, tuple(x * 1.1 for x in coat), sstep(-0.3, -0.8, n.z) * 0.5)
        return col

    def paint_leg(c, n):
        col = mottle(c, coat)
        col = mixc(col, dark, gauss(c.z - 0.47 * p["legs"], 0.06) * 0.8)  # knees
        return mixc(col, dark, sstep(0.2, 0.1, c.z))  # fetlocks

    # ---- body: a deep barrel with a hump over the withers
    ys = [-0.86, -0.78, -0.55, -0.25, 0.1, 0.4, 0.62, 0.74]
    top = [1.22, 1.3, 1.31, 1.27, 1.28, 1.33, 1.3, 1.16]
    bot = [1.0, 0.82, 0.72, 0.65, 0.64, 0.66, 0.69, 0.92]
    wid = [0.12, 0.25, 0.29, 0.31, 0.3, 0.27, 0.24, 0.13]
    path = [V(0, y, (t + b) / 2 + lift) for y, t, b in zip(ys, top, bot)]

    def body_r(c):
        y = c.p.y
        h = (tab(y, ys, top) - tab(y, ys, bot)) / 2
        w = tab(y, ys, wid)
        r = ellipse(c.th, h, w)
        up = math.cos(c.th)
        side = math.sin(c.th)
        for sgn in (-1, 1):  # hook bones and pin bones of the hips
            r += 0.045 * gauss(side - sgn * 0.75, 0.2) * gauss(y + 0.55, 0.08) * (up > 0)
        r += 0.02 * gauss(abs(side) - 0.95, 0.2) * gauss(y - 0.45, 0.12)  # shoulders
        # the zebu hump rises out of the withers
        r += p["hump"] * gauss(y - 0.45, 0.13) * max(0.0, up) ** 2.5 * gauss(side, 0.55)
        return r

    a.part("body", tube(path, 40, 26, body_r, ref=UP, cap=True, ru=0.2), "facet", None, smooth=False,
           decimate=0.42, paint=paint_body)

    # ---- neck and head
    nk = p["neck"]
    neck = tube([V(0, 0.5, 1.08 + lift), V(0, 0.72, 1.15 + lift), V(0, 0.9, 1.21 + lift)], 10, 14,
                lambda c: ellipse(c.th, lerp(0.28, 0.15, c.t) * nk, lerp(0.21, 0.12, c.t) * nk), ref=UP, cap=True,
                ru=0.2)
    a.part("neck", neck, "facet", None, smooth=False, decimate=0.6, paint=paint_body)
    hs = p["head"]
    anchor = V(0, 0.9, 1.22 + lift)

    def H(x, y, z):  # head-local point, scaled about the poll
        return anchor + (V(x, y, z) - V(0, 0.9, 1.22)) * hs

    head = tube([H(0, 0.86, 1.3), H(0, 0.95, 1.3), H(0, 1.05, 1.21), H(0, 1.14, 1.1), H(0, 1.2, 1.03),
                 H(0, 1.235, 0.99)], 20, 14,
                lambda c: ellipse(c.th, hs * tab(c.t, [0, 0.2, 0.55, 0.85, 1], [0.075, 0.105, 0.09, 0.08, 0.055]),
                                  hs * tab(c.t, [0, 0.2, 0.55, 0.85, 1], [0.09, 0.13, 0.095, 0.09, 0.065])),
                ref=FRONT, cap=True, ru=0.1)

    def paint_head(c, n):
        col = mottle(c, coat)
        col = mixc(col, dark, sstep(0.3, 0.9, n.z) * 0.3)
        return mixc(col, BLACK, sstep(1.19, 1.225, (c.y - anchor.y) / hs + 0.9))

    a.part("head", head, "facet", None, smooth=False, decimate=0.6, paint=paint_head)
    a.part("muzzle", blob(tuple(H(0, 1.215, 0.995)), (0.058 * hs, 0.034 * hs, 0.046 * hs), 10, 6), "facet", None,
           smooth=False, paint=flat(BLACK))
    for s in (1, -1):
        a.part(f"eye{s}", blob(tuple(H(s * 0.098, 1.05, 1.2)), (0.018 * hs, 0.026 * hs, 0.018 * hs), 8, 5), "facet",
               None, smooth=False, paint=flat(BLACK))
        # drooping zebu ears
        base, tip = H(s * 0.12, 0.99, 1.23), H(s * 0.27, 1.01, 1.02)
        ear = tube([base, (base + tip) / 2 + V(s * 0.03, 0, 0.03), tip], 8, 10,
                   lambda c: ellipse(c.th, 0.014 * hs, tab(c.t, [0, 0.35, 0.8, 1], [0.035, 0.075, 0.06, 0.012]) * hs),
                   ref=FRONT, cap=True, ru=0.05)
        a.part(f"ear{s}", ear, "facet", None, smooth=False,
               paint=lambda c, n, s=s: mixc(mottle(c, coat), EAR_IN, sstep(0.0, 0.6, n.y + 0.3 * s * n.x)))
        if p["horn"] > 0:
            hn = p["horn"]
            pts = [H(s * 0.07, 0.93, 1.33), H(s * (0.07 + 0.11 * hn), 0.92, 1.36), H(s * (0.08 + 0.15 * hn), 0.93,
                                                                                    1.36 + 0.1 * hn),
                   H(s * (0.05 + 0.14 * hn), 0.96, 1.36 + 0.17 * hn)]
            horn = tube(pts, 10, 8, lambda c: lerp(0.034 * min(hn, 1.2), 0.004, c.t ** 0.8) * hs, ref=FRONT,
                        cap=True, ru=0.03)
            a.part(f"horn{s}", horn, "facet", None, smooth=False,
                   paint=lambda c, n: mixc(HORN, BLACK, sstep(1.4 + lift, 1.5 + lift, c.z)))
    # the dewlap: a loose fold of skin from the jaw to the brisket
    dw = p["dewlap"]
    def fold(c):
        w = 0.035 + 0.012 * dw
        return ellipse(c.th, 0.05 + 0.07 * dw * math.sin(math.pi * c.t) ** 0.7, w)

    under = [V(0, 0.95, 1.05 + lift), V(0, 0.84, 0.93 + lift), V(0, 0.72, 0.8 + lift), V(0, 0.6, 0.72 + lift)]
    a.part("dewlap", tube(under, 12, 8, fold, ref=UP, cap=True, ru=0.1), "facet", None, smooth=False,
           decimate=0.7, paint=paint_body)

    # ---- legs and hooves
    L = p["legs"]
    for s in (1, -1):
        for front, y0 in ((True, 0.47), (False, -0.6)):
            x = s * 0.17
            if front:
                pts = [V(x, y0, 1.0), V(x, y0 + 0.02, 0.78), V(x, y0 + 0.01, 0.48), V(x, y0, 0.2), V(x, y0 + 0.02,
                                                                                                     0.06)]
                rad = [0.13, 0.095, 0.068, 0.056, 0.06]
            else:
                pts = [V(x, y0, 1.05), V(x, y0 + 0.08, 0.82), V(x, y0 - 0.06, 0.55), V(x, y0 - 0.02, 0.2),
                       V(x, y0, 0.06)]
                rad = [0.17, 0.125, 0.075, 0.056, 0.06]
            pts = [V(q.x, q.y, q.z * L if q.z < 0.8 else q.z + lift) for q in pts]
            zs = [q.z for q in pts]

            def lr(c, zs=zs, rad=rad):
                return tab(c.p.z, zs[::-1], rad[::-1]) * (1 + 0.1 * math.cos(2 * c.th))

            leg = tube(pts, 16, 10, lr, ref=FRONT, cap=True, ru=0.06)
            a.part(f"leg{s}{front}", leg, "facet", None, smooth=False, decimate=0.65, paint=paint_leg)
            for k in (-1, 1):  # cloven hooves
                hoof = tube([V(x + k * 0.022, pts[-1].y + 0.01, 0.07), V(x + k * 0.025, pts[-1].y + 0.035, 0.0)], 2, 7,
                            lambda c: lerp(0.03, 0.036, c.t), ref=FRONT, raw=True, cap=True, ru=0.03)
                a.part(f"hoof{s}{front}{k}", hoof, "facet", None, smooth=False, paint=flat(HOOF))

    # ---- tail with its black switch
    tail = [V(0, -0.84, 1.25 + lift), V(0, -0.9, 1.12 + lift), V(0, -0.91, 0.85 + lift), V(0, -0.9, 0.62 * L)]
    a.part("tail", tube(tail, 14, 7, lambda c: lerp(0.035, 0.018, c.t), ref=FRONT, cap=True, ru=0.03), "facet",
           None, smooth=False, decimate=0.8, paint=lambda c, n: mottle(c, coat))
    sw = [V(0, -0.9, 0.66 * L), V(0, -0.905, 0.52 * L), V(0, -0.9, 0.36 * L)]
    a.part("switch", tube(sw, 8, 7, lambda c: 0.045 * math.sin(math.pi * (0.12 + 0.88 * c.t)) ** 0.6 + 0.004,
                          ref=FRONT, cap=True, ru=0.03), "facet", None, smooth=False, paint=flat(BLACK))
    if p["udder"]:
        a.part("udder", blob((0, -0.42, 0.7 + lift), (0.11, 0.13, 0.08), 10, 6), "facet", None, smooth=False,
               paint=flat((0.42, 0.3, 0.26)))

    # scale the whole animal
    S = Matrix.Scale(p["scale"], 4)
    for ob, _s, _w in a.parts:
        ob.data.transform(S)
    # painters see the unscaled animal, so every variant is coloured alike
    k = 1.0 / p["scale"]
    for key, f in list(a.painters.items()):
        a.painters[key] = (lambda f: lambda c, n: f(c * k, n))(f)
    return a


def main():
    built = []
    for name, p in VARIANTS.items():
        if ONLY and name not in ONLY:
            continue
        a = build_variant(name, p)
        built.append(a)
        if STAGE == "preview":
            continue
        joined, _ = a.finish(out=os.path.join(ROOT, "assets", "models", f"{name}.glb"))
        tris = 0
        for ob in joined.values():
            ob.data.calc_loop_triangles()
            tris += len(ob.data.loop_triangles)
        print(name, "tris", tris)
        if RENDERS:
            views = [("three", 2.35, 0.1, 1.0), ("side", math.pi / 2, 0.05, 1.0), ("front", math.pi, 0.06, 1.0),
                     ("back", 0.0, 0.08, 1.0)]
            render_views(a.coll, os.path.join(RENDERS, name), views, samples=128, size=(1000, 800))
    return built


if __name__ == "__main__":
    main()

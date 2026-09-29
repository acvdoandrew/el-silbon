"""The corral herd, after the "Corral Herd" concept sheet: zebu (Brahman-
like) cattle of the llanos in the sheet's faceted low-poly style. One
parametric body gives three variants: the cow (dusty white-grey, modest
hump, loose dewlap, short curved horns), the bull (larger, light brown-grey,
big hump, thick neck) and the calf (reddish brown, slim, horn nubs).

Flat per-facet colours, skinned to a small rig per variant (Body, Neck,
Head, EarL, EarR, Tail) that `src/world/herd.rs` moves: grazing, looking
round, flicking ears and swishing tails. Each variant is its own glTF,
facing -Z in Bevy, hooves on the ground.

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
EAR_IN = (0.3, 0.16, 0.1)
MUZZLE = (0.04, 0.036, 0.034)

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
    bot = [1.0, 0.8, 0.69, 0.61, 0.6, 0.63, 0.67, 0.9]
    wid = [0.13, 0.28, 0.33, 0.35, 0.34, 0.31, 0.27, 0.14]
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

    a.part("body", tube(path, 40, 26, body_r, ref=UP, cap=True, ru=0.2), "facet", "Body", smooth=False,
           decimate=0.42, paint=paint_body)

    # ---- neck and head
    nk = p["neck"]
    neck = tube([V(0, 0.5, 1.08 + lift), V(0, 0.72, 1.15 + lift), V(0, 0.9, 1.21 + lift)], 10, 14,
                lambda c: ellipse(c.th, lerp(0.28, 0.15, c.t) * nk, lerp(0.21, 0.12, c.t) * nk), ref=UP, cap=True,
                ru=0.2)
    a.part("neck", neck, "facet", neck_w, smooth=False, decimate=0.6, paint=paint_body)
    hs = p["head"]
    anchor = V(0, 0.9, 1.22 + lift)

    def H(x, y, z):  # head-local point, scaled about the poll
        return anchor + (V(x, y, z) - V(0, 0.9, 1.22)) * hs

    head = tube([H(0, 0.86, 1.3), H(0, 0.95, 1.3), H(0, 1.05, 1.21), H(0, 1.14, 1.1), H(0, 1.2, 1.03),
                 H(0, 1.235, 0.99)], 20, 14,
                lambda c: ellipse(c.th, hs * tab(c.t, [0, 0.2, 0.55, 0.85, 1], [0.085, 0.12, 0.1, 0.09, 0.062]),
                                  hs * tab(c.t, [0, 0.2, 0.55, 0.85, 1], [0.11, 0.16, 0.112, 0.1, 0.075])),
                ref=FRONT, cap=True, ru=0.1)

    def paint_head(c, n):
        col = mottle(c, coat)
        col = mixc(col, dark, sstep(0.3, 0.9, n.z) * 0.3)
        return mixc(col, MUZZLE, sstep(1.18, 1.215, (c.y - anchor.y) / hs + 0.9))

    a.part("head", head, "facet", "Head", smooth=False, decimate=0.6, paint=paint_head)
    a.part("muzzle", blob(tuple(H(0, 1.212, 0.99)), (0.068 * hs, 0.04 * hs, 0.055 * hs), 12, 7), "facet", "Head",
           smooth=False, paint=lambda c, n: mottle(c, MUZZLE, 0.2, 40.0))
    for s in (1, -1):
        a.part(f"nostril{s}", blob(tuple(H(s * 0.028, 1.245, 0.995)), (0.012 * hs, 0.008 * hs, 0.014 * hs), 8, 5),
               "facet", "Head", smooth=False, paint=flat(BLACK))
        a.part(f"eye{s}", blob(tuple(H(s * 0.108, 1.05, 1.2)), (0.02 * hs, 0.03 * hs, 0.02 * hs), 10, 6), "facet",
               "Head", smooth=False, paint=lambda c, n: BLACK if n.y > -0.2 else (0.12, 0.08, 0.06))
        # big drooping zebu ears: coat outside, pinkish brown within
        base, tip = H(s * 0.125, 0.99, 1.24), H(s * 0.27, 1.02, 0.99)
        ear = tube([base, (base + tip) / 2 + V(s * 0.04, 0.01, 0.02), tip], 10, 12,
                   lambda c: ellipse(c.th, 0.022 * hs, tab(c.t, [0, 0.3, 0.75, 1], [0.036, 0.078, 0.066, 0.014]) * hs),
                   ref=FRONT, cap=True, ru=0.05)
        a.part(f"ear{s}", ear, "facet", f"Ear{'L' if s < 0 else 'R'}", smooth=False,
               paint=lambda c, n, s=s: mixc(mottle(c, coat), EAR_IN, sstep(0.1, 0.7, n.y - 0.4 * n.z)))
        if p["horn"] > 0:
            hn = p["horn"]
            pts = [H(s * 0.07, 0.93, 1.33), H(s * (0.07 + 0.11 * hn), 0.92, 1.36), H(s * (0.08 + 0.15 * hn), 0.93,
                                                                                    1.36 + 0.1 * hn),
                   H(s * (0.05 + 0.14 * hn), 0.96, 1.36 + 0.17 * hn)]
            horn = tube(pts, 10, 8, lambda c: lerp(0.034 * min(hn, 1.2), 0.004, c.t ** 0.8) * hs, ref=FRONT,
                        cap=True, ru=0.03)
            a.part(f"horn{s}", horn, "facet", "Head", smooth=False,
                   paint=lambda c, n: mixc(HORN, BLACK, sstep(1.4 + lift, 1.5 + lift, c.z)))
    # the dewlap: a loose fold of skin from the jaw to the brisket
    dw = p["dewlap"]
    def fold(c):
        w = 0.035 + 0.012 * dw
        return ellipse(c.th, 0.05 + 0.07 * dw * math.sin(math.pi * c.t) ** 0.7, w)

    under = [V(0, 0.95, 1.05 + lift), V(0, 0.84, 0.93 + lift), V(0, 0.72, 0.8 + lift), V(0, 0.6, 0.72 + lift)]
    a.part("dewlap", tube(under, 12, 8, fold, ref=UP, cap=True, ru=0.1), "facet", neck_w, smooth=False,
           decimate=0.7, paint=paint_body)

    # ---- legs and hooves
    L = p["legs"]
    for s in (1, -1):
        for front, y0 in ((True, 0.47), (False, -0.6)):
            x = s * 0.17
            if front:
                pts = [V(x, y0, 1.0), V(x, y0 + 0.02, 0.78), V(x, y0 + 0.01, 0.48), V(x, y0, 0.2), V(x, y0 + 0.02,
                                                                                                     0.06)]
                rad = [0.15, 0.115, 0.08, 0.06, 0.066]
            else:
                pts = [V(x, y0, 1.05), V(x, y0 + 0.08, 0.82), V(x, y0 - 0.06, 0.55), V(x, y0 - 0.02, 0.2),
                       V(x, y0, 0.06)]
                rad = [0.2, 0.15, 0.085, 0.06, 0.066]
            pts = [V(q.x, q.y, q.z * L if q.z < 0.8 else q.z + lift) for q in pts]
            zs = [q.z for q in pts]

            def lr(c, zs=zs, rad=rad):
                r = tab(c.p.z, zs[::-1], rad[::-1]) * (1 + 0.1 * math.cos(2 * c.th))
                return r + 0.012 * gauss(c.p.z - zs[2], 0.05)  # the knobbly knee or hock

            leg = tube(pts, 16, 10, lr, ref=FRONT, cap=True, ru=0.06)
            a.part(f"leg{s}{front}", leg, "facet", "Body", smooth=False, decimate=0.65, paint=paint_leg)
            for k in (-1, 1):  # cloven hooves
                hoof = tube([V(x + k * 0.022, pts[-1].y + 0.01, 0.07), V(x + k * 0.025, pts[-1].y + 0.035, 0.0)], 2, 7,
                            lambda c: lerp(0.03, 0.036, c.t), ref=FRONT, raw=True, cap=True, ru=0.03)
                a.part(f"hoof{s}{front}{k}", hoof, "facet", "Body", smooth=False, paint=flat(HOOF))

    # ---- tail with its black switch
    tail = [V(0, -0.84, 1.25 + lift), V(0, -0.9, 1.12 + lift), V(0, -0.91, 0.85 + lift), V(0, -0.9, 0.62 * L)]
    a.part("tail", tube(tail, 14, 7, lambda c: lerp(0.035, 0.018, c.t), ref=FRONT, cap=True, ru=0.03), "facet",
           "Tail", smooth=False, decimate=0.8, paint=lambda c, n: mottle(c, coat))
    sw = [V(0, -0.9, 0.66 * L), V(0, -0.905, 0.52 * L), V(0, -0.9, 0.36 * L)]
    a.part("switch", tube(sw, 8, 7, lambda c: 0.045 * math.sin(math.pi * (0.12 + 0.88 * c.t)) ** 0.6 + 0.004,
                          ref=FRONT, cap=True, ru=0.03), "facet", "Tail", smooth=False, paint=flat(BLACK))
    if p["udder"]:
        a.part("udder", blob((0, -0.42, 0.7 + lift), (0.11, 0.13, 0.08), 10, 6), "facet", "Body", smooth=False,
               paint=flat((0.42, 0.3, 0.26)))

    # scale the whole animal
    S = Matrix.Scale(p["scale"], 4)
    for ob, _s, _w in a.parts:
        ob.data.transform(S)
    # painters and weights see the unscaled animal, so every variant is alike
    k = 1.0 / p["scale"]
    for key, f in list(a.painters.items()):
        a.painters[key] = (lambda f: lambda c, n: f(c * k, n))(f)
    a.parts = [(ob, st, (lambda f: lambda co: f(co * k))(w) if callable(w) else w) for ob, st, w in a.parts]
    return a


def joints(p):
    """The rig in Bevy space (x, up, back), scaled with the animal: the neck
    swings from the withers, the head from the poll, the ears from their
    roots, the tail from its head."""
    lift = (p["legs"] - 1.0) * 0.72
    hs = p["head"]
    anchor = (0.0, 0.9, 1.22 + lift)

    def head_pt(x, y, z):  # the same head-local mapping as the model
        return (anchor[0] + (x - 0.0) * hs, anchor[1] + (y - 0.9) * hs, anchor[2] + (z - 1.22) * hs)

    def bevy(b):  # Blender (x, fwd, up) to Bevy (x, up, back), scaled
        s = p["scale"]
        return (b[0] * s, b[2] * s, -b[1] * s)

    return [
        ("Body", None, (0.0, 0.0, 0.0)),
        ("Neck", "Body", bevy((0.0, 0.52, 1.12 + lift))),
        ("Head", "Neck", bevy(head_pt(0.0, 0.92, 1.28))),
        ("EarL", "Head", bevy(head_pt(-0.125, 0.99, 1.24))),
        ("EarR", "Head", bevy(head_pt(0.125, 0.99, 1.24))),
        ("Tail", "Body", bevy((0.0, -0.84, 1.25 + lift))),
    ]


def neck_w(co):
    t = sstep(0.5, 0.64, co.y)
    h = sstep(0.86, 0.95, co.y)
    return {"Body": 1.0 - t, "Neck": t * (1.0 - h), "Head": t * h}


def main():
    built = []
    for name, p in VARIANTS.items():
        if ONLY and name not in ONLY:
            continue
        a = build_variant(name, p)
        built.append(a)
        if STAGE == "preview":
            continue
        out = None if "--live" in ARGS else os.path.join(ROOT, "assets", "models", f"{name}.glb")
        joined, _ = a.finish(out=out, joints=joints(p))
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

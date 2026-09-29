"""Two props after the "Original prop studies" sheets:

- The returned bones (`bone_bundle.glb`): a sagging pear-shaped burlap sack
  (39 x 25 x 59 cm with its bones), a tight double-cord mouth, one repaired
  side (a dark cloth patch with stitches), a dark wet hem, and three unequal
  femur ends breaking the opening.
- The foreman's radio (`radio.glb`): a horizontal two-tone case (charcoal
  shell, ivory front), a generous speaker grille, a recessed amber dial with
  its needle and ticks, two knobs, a repaired (taped) handle, tape on one
  corner and an aerial that never sits straight (35 x 16 x 46 cm). The dial
  glass is its own material, `radio_dial`, so the game can light it when the
  radio is powered.

Both baked to textures, origin at the base centre, facing -Z in Bevy.

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/props.py -- [--only bundle,radio] [--renders DIR]
"""

import importlib
import math
import os
import sys

import bmesh
import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402

importlib.reload(modelkit)
from modelkit import (  # noqa: E402
    FRONT, ROOT, TAU, UP, V, Asset, box, clamp, ellipse, emissive, fbm, gauss, image_from, lerp, nz, pat_rope,
    pat_weave, render_views, sstep, superellipse, tab, tube,
)

ARGS = globals().get("ARGS") or (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
STAGE = "preview" if "--preview" in ARGS else "final"
RENDERS = ARGS[ARGS.index("--renders") + 1] if "--renders" in ARGS else None
ONLY = ARGS[ARGS.index("--only") + 1].split(",") if "--only" in ARGS else ["bundle", "radio"]


def bar(a, b, r, sides=10):
    return tube([a, b], 2, sides, r, ref=UP if abs((b - a).normalized().z) < 0.9 else V(1, 0, 0), raw=True,
                cap=True, ru=r)


# ================================================================= the bone bundle
BURLAP = (0.34, 0.215, 0.11)
BURLAP_DARK = (0.16, 0.1, 0.05)


def wet_hem(k, obj, colour, rough, metal, height):
    sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
    n = k.new("ShaderNodeTexNoise", Vector=obj, Scale=12.0, Detail=6.0)
    edge = k.math("ADD", sep.outputs["Z"], k.math("MULTIPLY_ADD", n.outputs["Fac"], 0.04, -0.02))
    wet = k.remap(edge, 0.1, 0.05, 0.0, 1.0)
    colour = k.mix(wet, colour, (0.05, 0.035, 0.02))
    mud = k.remap(edge, 0.035, 0.0, 0.0, 1.0)
    colour = k.mix(mud, colour, (0.06, 0.045, 0.03))
    rough = k.math("SUBTRACT", rough, k.math("MULTIPLY", wet, 0.35))
    return colour, rough, metal, height


def build_bundle():
    a = Asset("bone_bundle", 0xB0B)
    s = modelkit.surface
    burlap = image_from("bb_pat_burlap", *modelkit.pat_weave(512, 12, 0.36, 0.4, 3))
    fabric = image_from("bb_pat_cloth", *modelkit.pat_weave(512, 24, 0.12, 0.25, 1))
    rope = image_from("bb_pat_rope", *pat_rope(256, 5))
    a.mat("burlap", s("bb_src_burlap", BURLAP, BURLAP_DARK, 0.95, detail=burlap, tile=(0.05, 0.05), dh=0.7, dk=0.4,
                      ao=0.8, aod=0.05, grime=0.6, bump=0.2, bump_scale=30.0, extra=wet_hem), "tex")
    a.mat("patch", s("bb_src_patch", (0.06, 0.045, 0.035), (0.025, 0.02, 0.016), 0.9, detail=fabric, tile=(0.04, 0.04),
                     dh=0.4, dk=0.3, ao=0.6, extra=wet_hem), "tex")
    a.mat("rope", s("bb_src_rope", (0.4, 0.31, 0.2), (0.2, 0.15, 0.09), 0.9, detail=rope, tile=(0.03, 0.03), dh=0.8,
                    dk=0.3, ao=0.6), "tex")
    a.mat("bone", s("bb_src_bone", (0.62, 0.55, 0.42), (0.36, 0.3, 0.21), 0.55, nscale=8.0, ao=0.7, aod=0.03,
                    grime=0.65, bump=0.25, bump_scale=60.0), "tex")
    a.mat("void", s("bb_src_void", (0.012, 0.01, 0.008), (0.004, 0.003, 0.003), 1.0, ao=0.0), "tex")

    ZS = [0.0, 0.02, 0.07, 0.16, 0.26, 0.33, 0.375, 0.395, 0.42, 0.45]
    RS = [0.13, 0.172, 0.195, 0.196, 0.178, 0.135, 0.085, 0.075, 0.1, 0.122]

    def sack_r(c):
        z = c.p.z
        r = ellipse(c.th, tab(z, ZS, RS) * 0.64, tab(z, ZS, RS))
        gather = sstep(0.28, 0.37, z) * (1 - sstep(0.4, 0.42, z))
        n = fbm(c.q, 7.0, 3, 21.0)
        r *= 1 + 0.18 * math.sin(c.th * 9 + 2 * n) * gather  # gathered folds under the cord
        r *= 1 + 0.12 * math.sin(c.th * 7 + 3 * n) * sstep(0.42, 0.45, z)  # a frilled mouth
        r += 0.01 * n * (1 - gather) + 0.006 * nz(c.q, 18.0, 5.0)
        r += 0.012 * math.sin(c.th * 5 + 4 * nz(c.q, 3.0, 6.0) + z * 12) * sstep(0.05, 0.2, z) * (1 - gather)
        r += 0.018 * gauss(adiff(c.th, 0.5), 0.35) * gauss(z - 0.2, 0.07)  # bone ends pressing out
        r += 0.014 * gauss(adiff(c.th, 3.6), 0.3) * gauss(z - 0.12, 0.05)
        if z < 0.06:  # it slumps where it sits
            r *= 1 + 0.08 * sstep(0.06, 0.0, z) * math.cos(c.th) ** 2
        return r

    def adiff(x, y):
        return (x - y + math.pi) % TAU - math.pi

    def sack_disp(c):
        return V(0, 0.015 * sstep(0.3, 0.0, c.z), 0)  # the belly sags forward

    path = [V(0, 0, z) for z in (0.0, 0.1, 0.2, 0.3, 0.38, 0.45)]
    bm = tube(path, 34, 28, sack_r, disp=sack_disp, ref=FRONT, ru=0.2)
    bmesh.ops.holes_fill(bm, edges=[e for e in bm.edges if e.is_boundary and e.verts[0].co.z < 0.01], sides=0)
    a.part("sack", bm, "burlap", smooth=True, solidify=0.004)
    a.part("inside", tube([V(0, 0.005, 0.3), V(0, 0.005, 0.43)], 2, 16, lambda c: ellipse(c.th, 0.06, 0.09),
                          ref=FRONT, raw=True, cap=True, ru=0.1), "void")

    # the double cord and its knot
    for k, (z, r0) in enumerate(((0.382, 0.09), (0.398, 0.087))):
        ring = [V((r0 + 0.004 * math.sin(t * 3 + k)) * math.sin(t), -(r0 * 0.66) * math.cos(t) - 0.002, z + 0.003 *
                  math.sin(t * 2)) for t in [TAU * i / 40 for i in range(41)]]
        ring = [V(q.x, -q.y, q.z) for q in ring]
        a.part(f"cord{k}", tube(ring, 41, 8, 0.0055, ref=UP, loop=True, raw=True, ru=0.02, transport=False), "rope")
    knot = V(0.01, 0.058, 0.39)
    a.part("knot", tube([knot + V(-0.012, 0, 0.004), knot, knot + V(0.012, 0.004, -0.004)], 8, 8,
                        lambda c: 0.012 * math.sin(math.pi * (0.1 + 0.8 * c.t)), ref=UP, ru=0.02, cap=True), "rope")
    for k, (dx, ln) in enumerate(((-0.012, 0.13), (0.014, 0.09))):
        end = [knot, knot + V(dx * 0.5, 0.012, -ln * 0.4), knot + V(dx, 0.014, -ln)]
        a.part(f"cord_end{k}", tube(end, 12, 6, lambda c: 0.005 * (1 - 0.4 * c.t), ref=UP, ru=0.02, cap=True), "rope")
    # the repair: a dark cloth patch sewn onto one side, stitches along its top
    def on_sack(th, z, lift):
        r = sack_r_eval(sack_r, th, z) + lift
        return V(-r * math.sin(th), r * math.cos(th) + 0.015 * sstep(0.3, 0.0, z), z)

    rows = [[on_sack(lerp(-1.05, -0.62, j / 6), lerp(0.1, 0.24, i / 6), 0.005) for j in range(7)] for i in range(7)]
    uvs = [[(j * 0.02, i * 0.023) for j in range(8)] for i in range(7)]
    a.part("patch", modelkit.grid(rows, uvs), "patch", smooth=True, solidify=0.003)
    for j in range(6):
        th = lerp(-1.02, -0.66, (j + 0.5) / 6)
        a.part(f"stitch{j}", bar(on_sack(th, 0.228, 0.008), on_sack(th, 0.252, 0.007), 0.0018, 5), "rope")

    # three femur ends of unequal height breaking the opening
    for i, (base, top, rot) in enumerate(((V(-0.03, -0.01, 0.25), V(-0.07, -0.015, 0.59), 0.4),
                                          (V(0.035, 0.0, 0.25), V(0.075, 0.02, 0.55), 1.7),
                                          (V(0.06, -0.025, 0.24), V(0.1, -0.03, 0.49), 2.8))):
        femur(a, base, top, rot, f"femur{i}")
    return a


def sack_r_eval(fn, th, z):
    """Evaluate the sack's radius function at an angle and height, as the loft does
    (normal +Y, binormal -X)."""
    c = modelkit.Ctx()
    c.th, c.p, c.z, c.t, c.i, c.k = th, V(0, 0, z), z, 0.0, 0, 0
    c.d = V(-math.sin(th), math.cos(th), 0)
    c.q = c.p + c.d * 0.05
    return fn(c)


def femur(a, base, top, rot, name):
    axis = (top - base).normalized()
    side = axis.cross(UP).normalized()
    mid = (base + top) / 2 + side * 0.006

    def r(c):
        rr = 0.013 + 0.003 * sstep(0.5, 0.0, c.t)
        knob = sstep(0.8, 0.95, c.t)
        rr += 0.016 * knob
        rr *= 1 + 0.35 * math.cos(2 * (c.th - rot)) * knob
        rr *= 1 - 0.55 * sstep(0.972, 1.0, c.t)
        return rr + 0.001 * nz(c.q, 80.0, 11.0)

    a.part(name, tube([base, mid, top], 30, 14, r, ref=UP, cap=True, ru=0.03), "bone")
    head_c = top - axis * 0.018 + side * 0.02 * math.cos(rot) + axis.cross(side) * 0.02 * math.sin(rot)
    hb = bmesh.new()
    bmesh.ops.create_uvsphere(hb, u_segments=12, v_segments=8, radius=0.017,
                              matrix=modelkit.Matrix.Translation(head_c))
    a.part(name + "_head", hb, "bone")


# ================================================================= the radio
CHARCOAL = (0.03, 0.03, 0.028)
IVORY = (0.52, 0.46, 0.36)
AMBER = (1.0, 0.45, 0.08)


def plastic_wear(k, obj, colour, rough, metal, height):
    bev = k.new("ShaderNodeBevel", Radius=0.006)
    bev.samples = 8
    geo = k.new("ShaderNodeNewGeometry")
    dot = k.new("ShaderNodeVectorMath")
    dot.operation = "DOT_PRODUCT"
    k.t.links.new(bev.outputs["Normal"], dot.inputs[0])
    k.t.links.new(geo.outputs["Normal"], dot.inputs[1])
    edge = k.remap(dot.outputs["Value"], 0.99, 0.9, 0.0, 1.0)
    n = k.new("ShaderNodeTexNoise", Vector=obj, Scale=60.0, Detail=6.0)
    wear = k.math("MULTIPLY", edge, k.remap(n.outputs["Fac"], 0.4, 0.6, 0.0, 1.0))
    colour = k.mix(wear, colour, (0.14, 0.13, 0.12))
    sc = k.new("ShaderNodeTexWave", Vector=obj, Scale=40.0, Distortion=12.0)
    scratch = k.remap(sc.outputs["Fac"], 0.97, 1.0, 0.0, 0.6)
    colour = k.mix(scratch, colour, (0.1, 0.1, 0.095))
    rough = k.math("ADD", rough, k.math("MULTIPLY", wear, 0.2))
    return colour, rough, metal, height


def ivory_wear(k, obj, colour, rough, metal, height):
    colour, rough, metal, height = plastic_wear(k, obj, colour, rough, metal, height)
    n = k.new("ShaderNodeTexNoise", Vector=obj, Scale=14.0, Detail=8.0, Roughness=0.7)
    grime = k.remap(n.outputs["Fac"], 0.45, 0.7, 0.0, 0.5)
    colour = k.mix(grime, colour, (0.2, 0.16, 0.1))
    return colour, rough, metal, height


def build_radio():
    a = Asset("radio", 0x4AD)
    s = modelkit.surface
    a.mat("shell", s("radio_src_shell", CHARCOAL, (0.018, 0.018, 0.017), 0.45, nscale=10.0, ao=0.7, aod=0.02,
                     bump=0.05, bump_scale=200.0, extra=plastic_wear), "tex")
    a.mat("front", s("radio_src_front", IVORY, (0.36, 0.3, 0.22), 0.55, nscale=6.0, ao=0.75, aod=0.02, bump=0.04,
                     bump_scale=150.0, extra=ivory_wear), "tex")
    a.mat("grille", s("radio_src_grille", (0.02, 0.022, 0.02), (0.008, 0.008, 0.008), 0.7, ao=0.9, aod=0.02), "tex")
    a.mat("metal", s("radio_src_metal", (0.34, 0.33, 0.3), (0.16, 0.155, 0.14), 0.35, nscale=20.0, ao=0.4,
                     bump=0.03), "tex")
    a.mat("tape", s("radio_src_tape", (0.36, 0.26, 0.14), (0.22, 0.15, 0.08), 0.8, nscale=30.0, ao=0.5, bump=0.1,
                    bump_scale=120.0), "tex")
    a.mat("dial", emissive("radio_dial", AMBER, 0.6), "=dial")
    a.mat("ink", emissive("radio_dial_ink", (0.03, 0.02, 0.01), 0.0), "=dial")

    W, D, H, Z0 = 0.35, 0.16, 0.22, 0.012
    front_y = D / 2
    a.part("case", box((0, 0, Z0 + H / 2), (W, D, H), 0.012, 3), "shell", smooth=True)
    a.part("face", box((0, front_y + 0.002, Z0 + H / 2), (W - 0.02, 0.006, H - 0.02), 0.003), "front", smooth=True)
    # speaker grille: a dark field with slats
    gx0, gx1, gz0, gz1 = 0.012, 0.155, Z0 + 0.025, Z0 + H - 0.03
    a.part("grille", box(((gx0 + gx1) / 2, front_y + 0.004, (gz0 + gz1) / 2), (gx1 - gx0, 0.004, gz1 - gz0)),
           "grille", smooth=False)
    for i in range(12):
        z = lerp(gz0 + 0.008, gz1 - 0.008, i / 11)
        a.part(f"slat{i}", box(((gx0 + gx1) / 2, front_y + 0.0065, z), (gx1 - gx0 - 0.008, 0.003, 0.004), 0.001),
               "shell", smooth=False)
    a.part("grille_rim", box(((gx0 + gx1) / 2, front_y + 0.0055, (gz0 + gz1) / 2), (gx1 - gx0 + 0.008, 0.002,
                                                                                    gz1 - gz0 + 0.008), 0.002),
           "shell", smooth=False)
    # the recessed dial: a dark bezel, amber glass, ticks and the needle
    dx0, dx1, dz0, dz1 = -0.155, -0.02, Z0 + H - 0.075, Z0 + H - 0.028
    a.part("bezel", box(((dx0 + dx1) / 2, front_y + 0.006, (dz0 + dz1) / 2), (dx1 - dx0 + 0.012, 0.008,
                                                                              dz1 - dz0 + 0.012), 0.003),
           "shell", smooth=False)
    a.part("dial", box(((dx0 + dx1) / 2, front_y + 0.009, (dz0 + dz1) / 2), (dx1 - dx0, 0.003, dz1 - dz0)), "dial",
           smooth=False)
    for i in range(15):
        x = lerp(dx0 + 0.008, dx1 - 0.008, i / 14)
        h = 0.012 if i % 2 == 0 else 0.006
        a.part(f"tick{i}", box((x, front_y + 0.0108, dz0 + 0.006 + h / 2), (0.0012, 0.0006, h)), "ink", smooth=False)
    a.part("needle", box((dx0 + (dx1 - dx0) * 0.62, front_y + 0.011, (dz0 + dz1) / 2), (0.0015, 0.0008,
                                                                                          dz1 - dz0 - 0.004)),
           "ink", smooth=False)
    # knobs
    for i, x in enumerate((-0.12, -0.055)):
        kz = Z0 + 0.055
        a.part(f"knob{i}", tube([V(x, front_y + 0.004, kz), V(x, front_y + 0.026, kz)], 2, 24,
                                lambda c: 0.021 * (1 + 0.04 * (math.cos(c.th * 12) > 0.5)) - 0.002 * c.t, ref=UP,
                                raw=True, cap=True, ru=0.02), "shell", smooth=True)
        a.part(f"mark{i}", box((x, front_y + 0.0265, kz + 0.011), (0.002, 0.0008, 0.009)), "front", smooth=False)
    # the handle, repaired with a wrap of tape
    for sx in (-1, 1):
        a.part(f"post{sx}", box((sx * 0.115, 0.0, Z0 + H + 0.028), (0.022, 0.03, 0.058), 0.004), "shell")
    a.part("grip", box((0.0, 0.0, Z0 + H + 0.062), (0.252, 0.034, 0.02), 0.005), "shell")
    a.part("wrap", box((0.01, 0.0, Z0 + H + 0.062), (0.05, 0.038, 0.024), 0.004), "tape")
    a.part("wrap2", box((-0.03, 0.0, Z0 + H + 0.062), (0.012, 0.038, 0.024), 0.003, rot=modelkit.Matrix.Rotation(
        -0.2, 4, "Z")), "tape")
    # tape across one front corner
    a.part("corner_tape", box((-0.13, front_y - 0.02, Z0 + H + 0.001), (0.08, 0.05, 0.002), 0.001,
                              rot=modelkit.Matrix.Rotation(0.25, 4, "Z")), "tape", smooth=False)
    # the aerial: three telescoping sections, each a little off true
    p = V(-0.142, -0.035, Z0 + H)
    a.part("aerial_base", tube([p, p + V(0, 0, 0.012)], 2, 12, 0.009, ref=FRONT, raw=True, cap=True, ru=0.01),
           "metal")
    d = V(-0.12, -0.06, 1.0).normalized()
    for i, (ln, r, bend) in enumerate(((0.08, 0.0045, 0.0), (0.075, 0.0035, 0.05), (0.07, 0.0026, -0.04))):
        d = (modelkit.Matrix.Rotation(bend, 3, "X") @ d).normalized()
        q = p + d * ln
        a.part(f"aerial{i}", bar(p, q, r, 8), "metal")
        p = q
    tip = bmesh.new()
    bmesh.ops.create_uvsphere(tip, u_segments=10, v_segments=6, radius=0.0055, matrix=modelkit.Matrix.Translation(p))
    a.part("aerial_tip", tip, "shell")
    for sx in (-1, 1):
        for sy in (-1, 1):
            a.part(f"foot{sx}{sy}", box((sx * 0.13, sy * 0.05, Z0 / 2), (0.04, 0.02, Z0), 0.003), "grille")
    return a


def main():
    built = []
    for name in ONLY:
        a = build_bundle() if name == "bundle" else build_radio()
        built.append(a)
        if STAGE == "preview":
            continue
        joined, _ = a.finish(tex={"*": 1024}, out=os.path.join(ROOT, "assets", "models", f"{a.name}.glb"))
        tris = sum(len(ob.data.polygons) for ob in joined.values())
        print(a.name, "tris", tris)
        if RENDERS:
            views = [("three", 2.5, 0.25, 1.0), ("front", math.pi, 0.08, 1.0), ("side", math.pi / 2, 0.08, 1.0),
                     ("back", 0.0, 0.2, 1.0)]
            render_views(a.coll, os.path.join(RENDERS, a.name), views, samples=160, size=(900, 900))
    return built


if __name__ == "__main__":
    main()

"""The extraction truck, after the "Truck" concept sheet: a 1970s-80s rural
utility truck of the llanos. Faded olive paint over cream, worn to primer
and rust at the edges, mud low down; angular hood and fenders, round
headlamps, a heavy steel bumper with tow hooks, a wooden cargo bed on steel
stakes, a roll bar, chunky all-terrain tyres, a simple cab interior.

Faces +X like the game's truck (`src/world/vehicles.rs`), wheels on the
ground at z = 0, about 5.4 m long. Lamp lenses are separate materials
(`truck_lamp_head`, `truck_lamp_tail`, `truck_lamp_amber`) so the game can
switch them with the engine.

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/truck.py -- [--quick] [--renders DIR]
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
    ROOT, TAU, UP, V, Asset, box, box_uv, clamp, emissive, fbm, image_from, lerp, nz, pat_planks, pat_tread,
    render_views, superellipse, tube,
)

ARGS = globals().get("ARGS") or (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
QUICK = "--quick" in ARGS
STAGE = "preview" if "--preview" in ARGS else "final"
OUT = os.path.join(ROOT, "assets", "models", "truck.glb")
RENDERS = ARGS[ARGS.index("--renders") + 1] if "--renders" in ARGS else None

# Linear colours from the sheet's swatches.
OLIVE = (0.105, 0.12, 0.052)
OLIVE_DARK = (0.055, 0.065, 0.03)
CREAM = (0.52, 0.44, 0.28)
CREAM_DARK = (0.3, 0.25, 0.16)
PRIMER = (0.2, 0.19, 0.17)
RUST = (0.085, 0.032, 0.014)
MUD = (0.075, 0.056, 0.035)

WHEEL_R = 0.5
AXLES = (1.55, -1.6)
TRACK = 0.86


# ---------------------------------------------------------------- materials
def paint_extra(cream_above):
    def f(k, obj, colour, rough, metal, height):
        sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
        two = k.math("GREATER_THAN", sep.outputs["Z"], cream_above)
        cream = k.new("ShaderNodeTexNoise", Vector=obj, Scale=5.0, Detail=6.0)
        cream_col = k.mix(cream.outputs["Fac"], CREAM_DARK, CREAM)
        base = k.mix(two, colour, cream_col)
        # worn coats: big flaking patches show the other colour, then primer
        wn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=3.2, Detail=10.0, Roughness=0.72)
        wear = k.remap(wn.outputs["Fac"], 0.6, 0.625, 0.0, 1.0)
        under = k.mix(two, CREAM_DARK, OLIVE_DARK)
        base = k.mix(wear, base, under)
        wn2 = k.new("ShaderNodeTexNoise", Vector=obj, Scale=7.0, Detail=10.0, Roughness=0.75)
        chip = k.remap(wn2.outputs["Fac"], 0.66, 0.68, 0.0, 1.0)
        base = k.mix(chip, base, PRIMER)
        # edges: chipped to rust (bevel normal against the true normal)
        bev = k.new("ShaderNodeBevel", Radius=0.025)
        bev.samples = 8
        geo = k.new("ShaderNodeNewGeometry")
        dot = k.new("ShaderNodeVectorMath")
        dot.operation = "DOT_PRODUCT"
        k.t.links.new(bev.outputs["Normal"], dot.inputs[0])
        k.t.links.new(geo.outputs["Normal"], dot.inputs[1])
        edge = k.remap(dot.outputs["Value"], 0.985, 0.9, 0.0, 1.0)
        en = k.new("ShaderNodeTexNoise", Vector=obj, Scale=18.0, Detail=8.0, Roughness=0.7)
        rust = k.math("MULTIPLY", edge, k.remap(en.outputs["Fac"], 0.4, 0.6, 0.0, 1.0))
        low = k.remap(sep.outputs["Z"], 0.95, 0.6, 0.0, 1.0)
        rn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=6.0, Detail=8.0, Roughness=0.7)
        rust = k.math("MAXIMUM", rust, k.math("MULTIPLY", low, k.remap(rn.outputs["Fac"], 0.55, 0.62, 0.0, 1.0)))
        base = k.mix(rust, base, RUST)
        # mud splashed up from the wheels
        mn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=4.0, Detail=10.0, Roughness=0.75)
        mud = k.math("MULTIPLY", k.remap(sep.outputs["Z"], 1.15, 0.55, 0.0, 1.0),
                     k.remap(mn.outputs["Fac"], 0.35, 0.55, 0.0, 1.0))
        base = k.mix(mud, base, MUD)
        rough = k.math("ADD", rough, k.math("ADD", k.math("MULTIPLY", rust, 0.3), k.math("MULTIPLY", mud, 0.35)))
        height = k.math("ADD", height, k.math("ADD", k.math("MULTIPLY", rust, 0.25), k.math("MULTIPLY", mud, 0.4)))
        return base, rough, metal, height

    return f


def metal_extra(k, obj, colour, rough, metal, height):
    rn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=5.0, Detail=10.0, Roughness=0.72)
    rust = k.remap(rn.outputs["Fac"], 0.52, 0.62, 0.0, 0.9)
    rn2 = k.new("ShaderNodeTexNoise", Vector=obj, Scale=30.0, Detail=6.0)
    rust_col = k.mix(rn2.outputs["Fac"], RUST, (0.13, 0.05, 0.02))
    base = k.mix(rust, colour, rust_col)
    sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
    mn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=4.0, Detail=8.0)
    mud = k.math("MULTIPLY", k.remap(sep.outputs["Z"], 0.9, 0.3, 0.0, 1.0), k.remap(mn.outputs["Fac"], 0.4, 0.55, 0, 1))
    base = k.mix(mud, base, MUD)
    rough = k.math("ADD", rough, k.math("MULTIPLY", rust, 0.35))
    height = k.math("ADD", height, k.math("MULTIPLY", rn2.outputs["Fac"], k.math("MULTIPLY", rust, 0.5)))
    return base, rough, metal, height


def wood_extra(k, obj, colour, rough, metal, height):
    # olive paint on the boards, worn through to grey wood along the grain
    wn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=4.0, Detail=10.0, Roughness=0.75)
    bare = k.remap(wn.outputs["Fac"], 0.56, 0.6, 0.0, 1.0)
    pn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=2.0, Detail=4.0)
    paint = k.mix(pn.outputs["Fac"], OLIVE_DARK, OLIVE)
    base = k.mix(bare, paint, colour)
    sep = k.new("ShaderNodeSeparateXYZ", Vector=obj)
    mud = k.remap(sep.outputs["Z"], 1.25, 1.02, 0.0, 0.8)
    base = k.mix(mud, base, MUD)
    rough = k.math("ADD", rough, k.math("MULTIPLY", bare, 0.15))
    return base, rough, metal, height


def tyre_extra(k, obj, colour, rough, metal, height):
    mn = k.new("ShaderNodeTexNoise", Vector=obj, Scale=5.0, Detail=10.0, Roughness=0.7)
    mud = k.remap(mn.outputs["Fac"], 0.42, 0.58, 0.0, 0.9)
    base = k.mix(mud, colour, MUD)
    rough = k.math("ADD", rough, k.math("MULTIPLY", mud, 0.05))
    return base, rough, metal, height


def materials(a):
    s = modelkit.surface
    a.mat("paint", s("truck_src_paint", OLIVE, OLIVE_DARK, 0.58, nscale=2.5, ao=0.7, aod=0.12, bump=0.08,
                     bump_scale=40.0, extra=paint_extra(1.58)), "body")
    planks = image_from("truck_pat_planks", *pat_planks(512, 4, 12))
    a.mat("wood", s("truck_src_wood", (0.15, 0.13, 0.1), (0.06, 0.05, 0.04), 0.85, detail=planks,
                    tile=(0.9, 0.56), dh=0.6, dk=0.35, ao=0.7, extra=wood_extra), "wood")
    a.mat("metal", s("truck_src_metal", (0.045, 0.042, 0.038), (0.018, 0.017, 0.016), 0.5, nscale=6.0, ao=0.7,
                     bump=0.1, bump_scale=60.0, extra=metal_extra), "metal")
    a.mat("dark", s("truck_src_dark", (0.03, 0.028, 0.026), (0.012, 0.011, 0.01), 0.7, nscale=6.0, ao=0.8,
                    bump=0.05), "metal")
    a.mat("rubber", s("truck_src_rubber", (0.03, 0.028, 0.026), (0.012, 0.011, 0.01), 0.88, ao=0.8, bump=0.1,
                      bump_scale=90.0, extra=tyre_extra), "rubber")
    glass = bpy.data.materials.get("truck_glass") or bpy.data.materials.new("truck_glass")
    k = modelkit.NodeKit(glass)
    b = k.bsdf()
    b.inputs["Base Color"].default_value = (0.012, 0.016, 0.02, 1)
    b.inputs["Roughness"].default_value = 0.08
    b.inputs["Alpha"].default_value = 0.55
    glass.surface_render_method = "BLENDED"
    a.mat("glass", glass, "=glass")
    a.mat("head", emissive("truck_lamp_head", (1.0, 0.78, 0.42), 8.0), "=lamp_head")
    a.mat("tail", emissive("truck_lamp_tail", (0.9, 0.05, 0.02), 2.0), "=lamp_tail")
    a.mat("amber", emissive("truck_lamp_amber", (1.0, 0.42, 0.05), 2.0), "=lamp_amber")


# ---------------------------------------------------------------- geometry helpers
def prism(profile, y0, y1):
    """Extrude a side profile [(x, z)] across y0..y1."""
    bm = bmesh.new()
    a = [bm.verts.new((x, y0, z)) for x, z in profile]
    b = [bm.verts.new((x, y1, z)) for x, z in profile]
    n = len(profile)
    bm.faces.new(a)
    bm.faces.new(b[::-1])
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((a[i], b[i], b[j], a[j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    box_uv(bm)
    return bm


def bar(a, b, r, sides=10):
    return tube([a, b], 2, sides, r, ref=UP if abs((b - a).normalized().z) < 0.9 else V(1, 0, 0), raw=True,
                cap=True, ru=r)


# ---------------------------------------------------------------- the truck
def build_body(a):
    # hood: an octagonal box, higher at the windscreen
    hood = tube([V(1.18, 0, 1.24), V(2.42, 0, 1.2)], 2, 8,
                lambda c: superellipse(c.th + math.pi / 8, 0.27 - 0.02 * c.t, 0.74, 2.6), ref=UP, raw=True,
                cap=True, ru=0.3)
    a.part("hood", hood, "paint", smooth=False, bevel=0.012)
    for s in (1, -1):
        prof = [(2.46, 0.62), (2.46, 1.17), (2.38, 1.25), (0.98, 1.25), (0.98, 0.62), (1.03, 0.62), (1.14, 0.97),
                (1.32, 1.11), (1.8, 1.11), (1.98, 0.97), (2.1, 0.62)]
        y0, y1 = (0.7, 1.0) if s > 0 else (-1.0, -0.7)
        a.part(f"fender{s}", prism(prof, y0, y1), "paint", smooth=False, bevel=0.015)
        # running board and the cab step
        a.part(f"step{s}", box((0.55, s * 0.98, 0.72), (0.9, 0.12, 0.05), 0.01), "metal", smooth=False)
        # headlamp bezel and lens, amber lamp below
        a.part(f"bezel{s}", box((2.47, s * 0.86, 1.03), (0.04, 0.26, 0.26), 0.012), "dark", smooth=False)
        lens = tube([V(2.48, s * 0.86, 1.03), V(2.515, s * 0.86, 1.03)], 2, 20,
                    lambda c: 0.1 - 0.015 * c.t ** 2, ref=UP, raw=True, cap=True, ru=0.1)
        a.part(f"headlamp{s}", lens, "head")
        a.part(f"amber{s}", box((2.475, s * 0.86, 0.8), (0.03, 0.12, 0.06), 0.006), "amber", smooth=False)
        # mirror arm and mirror
        a.part(f"mirror_arm{s}", bar(V(1.15, s * 0.95, 1.62), V(1.22, s * 1.2, 1.86), 0.012), "dark")
        a.part(f"mirror{s}", box((1.22, s * 1.22, 1.95), (0.05, 0.13, 0.24), 0.015), "dark", smooth=False)
    # the grille: a dark recess with slats, and its paint surround
    a.part("grille_back", box((2.425, 0, 1.2), (0.03, 1.06, 0.46)), "dark", smooth=False)
    for i in range(7):
        z = 1.0 + 0.4 * (i + 0.5) / 7
        a.part(f"slat{i}", box((2.45, 0, z), (0.04, 1.04, 0.022), 0.005), "metal", smooth=False)
    a.part("grille_frame_t", box((2.44, 0, 1.45), (0.05, 1.14, 0.05), 0.01), "paint", smooth=False)
    a.part("grille_frame_b", box((2.44, 0, 0.95), (0.05, 1.14, 0.05), 0.01), "paint", smooth=False)
    # the bumper: a heavy steel beam, bolted, with two tow hooks
    a.part("bumper", box((2.6, 0, 0.72), (0.2, 2.14, 0.22), 0.02), "metal", smooth=False)
    for s in (1, -1):
        a.part(f"bolt{s}", box((2.705, s * 0.8, 0.72), (0.02, 0.05, 0.05), 0.008), "metal", smooth=False)
        ring = [V(2.74 + 0.07 * math.sin(t), s * 0.55, 0.62 - 0.07 * math.cos(t)) for t in
                [TAU * i / 24 for i in range(25)]]
        a.part(f"hook{s}", tube(ring, 25, 8, 0.018, ref=V(0, 1, 0), loop=True, raw=True, ru=0.02,
                                transport=False), "metal")
        a.part(f"hook_mount{s}", box((2.705, s * 0.55, 0.7), (0.04, 0.08, 0.1), 0.01), "metal", smooth=False)


def build_cab(a):
    prof = [(0.12, 1.0), (1.2, 1.0), (1.2, 1.53), (1.06, 2.2), (0.98, 2.26), (0.2, 2.26), (0.12, 2.18)]
    cab = a.part("cab", prism(prof, -0.96, 0.96), "paint", smooth=False, bevel=0.02)
    a.part("roof", box((0.59, 0, 2.275), (0.78, 1.86, 0.04), 0.012), "paint", smooth=False)
    # windows: the windscreen on the slope, doors and the back
    cab.modifiers.new("shell", "SOLIDIFY").thickness = 0.035  # hollow, so the windows see inside
    tilt = modelkit.Matrix.Rotation(math.pi / 2 - math.atan2(2.2 - 1.53, 1.06 - 1.2), 4, "Y")

    def screen(depth, w, h):
        bm = box((1.13, 0, 1.865), (depth, w, h), 0.01)
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(1.13, 0, 1.865), matrix=tilt)
        return bm

    a.cut(cab, a.cutter("windscreen", screen(0.2, 1.6, 0.56)))
    a.part("windscreen", screen(0.012, 1.62, 0.58), "glass", smooth=False)
    win = [(0.3, 1.62), (1.1, 1.62), (0.99, 2.12), (0.3, 2.12)]
    for s in (1, -1):
        a.cut(cab, a.cutter(f"side{s}", prism(win, s * 0.85, s * 1.05)))
        a.part(f"side_window{s}", prism(win, s * 0.94 - 0.005, s * 0.94 + 0.005), "glass", smooth=False)
        a.part(f"handle{s}", box((0.45, s * 0.975, 1.48), (0.12, 0.02, 0.025), 0.006), "dark", smooth=False)
        a.part(f"hinge{s}a", box((1.12, s * 0.968, 1.35), (0.05, 0.02, 0.08), 0.005), "dark", smooth=False)
    a.cut(cab, a.cutter("back", box((0.12, 0, 1.9), (0.2, 1.2, 0.4))))
    a.part("back_window", box((0.14, 0, 1.9), (0.01, 1.22, 0.42), 0.01), "glass", smooth=False)
    # a glimpse inside: dash, wheel, bench
    a.part("dash", box((1.08, 0, 1.52), (0.16, 1.8, 0.12), 0.02), "dark", smooth=False)
    wheel = [V(0.95 + 0.17 * math.sin(t) * 0.5, 0.45 + 0.17 * math.cos(t), 1.66 + 0.17 * math.sin(t) * 0.85) for t in
             [TAU * i / 32 for i in range(33)]]
    a.part("steering", tube(wheel, 33, 8, 0.014, ref=V(1, 0, 0.5), loop=True, raw=True, ru=0.02,
                            transport=False), "dark")
    a.part("column", bar(V(1.1, 0.45, 1.45), V(0.95, 0.45, 1.66), 0.02), "dark")
    a.part("bench", box((0.35, 0, 1.3), (0.4, 1.7, 0.18), 0.04), "dark", smooth=False)
    a.part("bench_back", box((0.2, 0, 1.55), (0.1, 1.7, 0.45), 0.04), "dark", smooth=False)


def build_bed(a):
    x0, x1 = -2.82, 0.02
    a.part("bed_floor", box(((x0 + x1) / 2, 0, 1.06), (x1 - x0, 2.02, 0.09), 0.01), "metal", smooth=False)
    boards = (1.18, 1.36, 1.54)
    for s in (1, -1):
        for i, z in enumerate(boards):
            a.part(f"board{s}{i}", box(((x0 + x1) / 2, s * 0.99, z), (x1 - x0 - 0.02, 0.05, 0.155), 0.012),
                   "wood", smooth=False)
        a.part(f"top_rail{s}", box(((x0 + x1) / 2, s * 0.99, 1.7), (x1 - x0, 0.07, 0.05), 0.01), "metal",
               smooth=False)
        for i in range(7):
            x = lerp(x0 + 0.04, x1 - 0.04, i / 6)
            a.part(f"stake{s}{i}", box((x, s * 1.025, 1.4), (0.06, 0.03, 0.72), 0.008), "metal", smooth=False)
            a.part(f"stake_bolt{s}{i}", box((x, s * 1.043, 1.27), (0.03, 0.01, 0.03), 0.004), "metal",
                   smooth=False)
    for i, z in enumerate(boards):
        a.part(f"gate{i}", box((x0 - 0.01, 0, z), (0.05, 1.96, 0.155), 0.012), "wood", smooth=False)
        a.part(f"front_board{i}", box((x1 - 0.02, 0, z), (0.05, 1.96, 0.155), 0.012), "wood", smooth=False)
    for y in (-0.6, 0.0, 0.6):
        a.part(f"gate_stake{y}", box((x0 - 0.045, y, 1.4), (0.03, 0.06, 0.72), 0.008), "metal", smooth=False)
    a.part("gate_rail", box((x0 - 0.01, 0, 1.7), (0.07, 2.02, 0.05), 0.01), "metal", smooth=False)
    # under the bed: cross members, mud guards, lights, rear bumper
    for x in (-2.6, -1.9, -1.2, -0.5):
        a.part(f"cross{x}", box((x, 0, 0.95), (0.08, 1.9, 0.1), 0.01), "metal", smooth=False)
    for s in (1, -1):
        a.part(f"guard{s}", box((AXLES[1], s * TRACK, 0.99), (1.2, 0.4, 0.04), 0.01), "metal", smooth=False)
        a.part(f"flap{s}", box((AXLES[1] - 0.62, s * TRACK, 0.8), (0.015, 0.36, 0.34), 0.004), "dark",
               smooth=False)
        a.part(f"tail_box{s}", box((x0 - 0.05, s * 0.78, 0.92), (0.06, 0.28, 0.12), 0.01), "dark", smooth=False)
        a.part(f"tail{s}", box((x0 - 0.08, s * 0.84, 0.92), (0.02, 0.12, 0.08), 0.005), "tail", smooth=False)
        a.part(f"reverse{s}", box((x0 - 0.08, s * 0.7, 0.92), (0.02, 0.1, 0.08), 0.005), "dark", smooth=False)
    a.part("rear_bumper", box((x0 - 0.02, 0, 0.8), (0.12, 1.8, 0.12), 0.015), "metal", smooth=False)
    # the roll bar behind the cab
    for s in (1, -1):
        a.part(f"hoop_post{s}", bar(V(0.03, s * 0.86, 1.1), V(0.03, s * 0.86, 2.36), 0.045), "dark")
        a.part(f"hoop_corner{s}", bar(V(0.03, s * 0.86, 2.36), V(0.03, s * 0.78, 2.44), 0.045), "dark")
        a.part(f"hoop_brace{s}", bar(V(0.03, s * 0.86, 2.2), V(-0.55, s * 0.9, 1.72), 0.03), "dark")
    a.part("hoop_top", bar(V(0.03, 0.78, 2.44), V(0.03, -0.78, 2.44), 0.045), "dark")


def build_chassis(a):
    for s in (1, -1):
        a.part(f"rail{s}", box((-0.2, s * 0.45, 0.78), (5.2, 0.1, 0.16), 0.01), "metal", smooth=False)
    a.part("fuel_tank", tube([V(0.15, -0.72, 0.78), V(0.9, -0.72, 0.78)], 2, 16,
                             lambda c: superellipse(c.th, 0.17, 0.17, 3.0), ref=UP, raw=True, cap=True, ru=0.2),
           "metal")
    for x in (0.25, 0.8):
        a.part(f"strap{x}", tube([V(x, -0.72, 0.78), V(x + 0.04, -0.72, 0.78)], 2, 16,
                                 lambda c: superellipse(c.th, 0.178, 0.178, 3.0), ref=UP, raw=True, cap=True,
                                 ru=0.2), "dark")
    a.part("toolbox", box((-0.4, 0.8, 0.8), (0.6, 0.3, 0.3), 0.02), "paint", smooth=False)
    for x in AXLES:
        a.part(f"axle{x}", bar(V(x, -TRACK + 0.1, WHEEL_R), V(x, TRACK - 0.1, WHEEL_R), 0.06), "dark")
    diff = tube([V(AXLES[1], 0, WHEEL_R - 0.16), V(AXLES[1], 0, WHEEL_R + 0.16)], 8, 14,
                lambda c: 0.17 * math.sin(math.pi * (0.08 + 0.84 * c.t)) ** 0.6, ref=V(1, 0, 0), ru=0.15, cap=True)
    a.part("diff", diff, "dark")
    a.part("exhaust", bar(V(-0.2, -0.45, 0.6), V(-2.95, -0.45, 0.6), 0.04), "metal")


def build_wheel(a, x, s):
    cy = s * TRACK
    n, lugs = 144, 32

    def tyre_r(c):
        r = superellipse(c.th, 0.175, 0.14, 3.2)
        out = -s * math.sin(c.th)  # 1 on the tread
        u = (c.t * lugs + (0.5 if math.cos(c.th) > 0 else 0.0)) % 1.0
        block = 1.0 if u < 0.55 else 0.0
        r += 0.028 * block * clamp((out - 0.35) / 0.3)
        r += 0.006 * nz(c.q, 30.0, 3.0)
        return r

    core = WHEEL_R - 0.14 - 0.028  # the lugs stand proud of the tread and carry the truck
    ring = [V(x + core * math.cos(t), cy, WHEEL_R + core * math.sin(t)) for t in
            [TAU * i / n for i in range(n + 1)]]
    bm = tube(ring, n + 1, 28, tyre_r, ref=V(0, s, 0), loop=True, raw=True, ru=0.2, transport=False)
    a.part(f"tyre{x}{s}", bm, "rubber", smooth=False)
    # the rim: a dished steel wheel with a hub and eight nuts
    prof = [(0.0, 0.335), (0.012, 0.34), (0.03, 0.325), (0.05, 0.3), (0.075, 0.22), (0.085, 0.2), (0.09, 0.14),
            (0.093, 0.12), (0.096, 0.06), (0.098, 0.001)]
    face = cy + s * 0.12
    pts = [V(x, face - s * d, WHEEL_R) for d, _r in prof]
    bm = tube(pts, len(pts), 32, lambda c: prof[c.i][1], ref=UP, raw=True, ru=0.2)
    a.part(f"rim{x}{s}", bm, "metal", smooth=True)
    for i in range(8):
        t = TAU * i / 8
        p = V(x + 0.14 * math.cos(t), face - s * 0.09, WHEEL_R + 0.14 * math.sin(t))
        a.part(f"nut{x}{s}{i}", bar(p, p + V(0, s * 0.025, 0), 0.016, 6), "metal", smooth=False)
    a.part(f"hub{x}{s}", bar(V(x, face - s * 0.095, WHEEL_R), V(x, face - s * 0.05, WHEEL_R), 0.07, 16), "dark")


def build():
    a = Asset("truck", 0x7C2)
    materials(a)
    build_body(a)
    build_cab(a)
    build_bed(a)
    build_chassis(a)
    for x in AXLES:
        for s in (1, -1):
            build_wheel(a, x, s)
    return a


def main():
    a = build()
    if STAGE == "preview":
        return a
    tex = {"*": 256} if QUICK else {"body": 2048, "*": 1024}
    joined, _ = a.finish(tex=tex, out=OUT, metal={"metal": 0.35})
    tris = 0
    for ob in joined.values():
        ob.data.calc_loop_triangles()
        tris += len(ob.data.loop_triangles)
    print("truck tris", tris)
    if RENDERS:
        views = [("three", 2.35, 0.12, 1.0), ("front", math.pi / 2, 0.05, 1.0), ("side", math.pi, 0.05, 1.0),
                 ("back", -math.pi / 2, 0.08, 1.0), ("front_detail", 2.0, 0.08, 0.28, (2.5, 0.6, 1.0)),
                 ("wheel", 2.9, 0.05, 0.22, (AXLES[0], TRACK, WHEEL_R)),
                 ("bed_detail", -2.3, 0.2, 0.35, (-2.6, 0.9, 1.3)), ("interior", 1.9, 0.25, 0.2, (0.9, 0.3, 1.6))]
        render_views(a.coll, RENDERS, views, samples=96 if QUICK else 192, size=(1200, 800))
    return a


if __name__ == "__main__":
    main()

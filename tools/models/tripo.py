"""Bring a Tripo-generated model into the game (headless Blender):

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/tripo.py -- NAME [--measure] [--src DIR]

NAME is one of `MODELS` below. The source is the Tripo export (a single
textured mesh, longest side 1.0, feet at 0, facing glTF +Z) in `--src`
(default ~/Downloads). The script turns it to the game's facing (glTF -Z,
or +X for the cattle and the radio, see `models.rs`), scales it to its
real size, decimates it, resamples its texture to 2K and skins it to the
joints `src/world` animates by name, then writes `assets/models/NAME.glb`.

Every joint has an identity rest rotation in glTF (the game sets joint
rotations outright). Weights come from Blender's heat weighting on a
temporary armature whose bones follow the limbs; the mesh is then bound to
the export armature, whose bones point up with no roll (`modelkit.rig`).

`--measure` writes front and side orthographic renders on a 10 cm grid with
the joints as dots (to `--shots`, default /tmp/tripo), and stops. A build
also renders the rig in a test pose there, to check the weights by eye.
"""
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Euler, Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402
from modelkit import V, select_only, view3d_override  # noqa: E402

ARGV = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def opt(name, default):
    return ARGV[ARGV.index(name) + 1] if name in ARGV else default


ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = opt("--src", os.path.expanduser("~/Downloads"))
SHOTS = opt("--shots", "/tmp/tripo")
TEX = int(opt("--tex", 2048))


def b(x, y, z):
    """Bevy (Y up, forward -Z) to Blender (Z up, forward +Y)."""
    return V(x, -z, y)


# ---------------------------------------------------------------- the models
# fit: ("height", m) | ("length", m) scales the source's longest side;
# ("scale", k) multiplies it outright.
# turn: degrees about the vertical that bring the source's front to the
# game's front (Blender +Y, glTF -Z). The cattle are then turned a further
# quarter at export (they face +X in glTF, `Model::placement`).
# joints: (name, parent, bevy position, tail or None, deforms). A tail of
# None points at the first child (or up, for a leaf).
# tris: decimation target.


def survivor(hip=0.93, knee=0.5, ankle=0.085, chest=1.25, neck=1.48, head=1.56, top=1.72,
             shoulder=(0.18, 1.42), elbow=(0.23, 1.14), wrist=(0.27, 0.88), hand=(0.29, 0.78),
             hip_x=0.095, knee_x=0.1, ankle_x=0.1, toe=-0.12):
    sx, sy = shoulder
    ex, ey = elbow
    wx, wy = wrist
    hx, hy = hand
    return survivor_joints(hip, knee, ankle, chest, neck, head, top, sx, sy, ex, ey, wx, wy, hx, hy, hip_x, knee_x,
                           ankle_x, toe)


def survivor_spec(src, height, **kw):
    full = dict(shoulder=(0.18, 1.42), elbow=(0.23, 1.14), wrist=(0.27, 0.88), hand=(0.29, 0.78))
    radius = kw.pop("radius", [0.1, 0.075, 0.085])
    full.update(kw)
    return dict(src=src, turn=180, fit=("height", height), tris=26000, joints=survivor(**kw),
                chains=survivor_arms(full["shoulder"], full["elbow"], full["wrist"], full["hand"], radius))


def beast(length, height, spine, neck, poll, nose, tail=None, ears=None):
    """A grazer's rig from landmarks given as (fraction of the length from the
    rump, fraction of the height): Body along the spine, Neck from its base to
    the poll, Head from the poll to the nose, the Tail, the ears (base and tip
    as (x, fraction of height, fraction of length) offsets)."""
    def at(f, x=0.0):
        fz, fy = f
        return (x, fy * height, length / 2 - fz * length)
    j = [
        ("Body", None, at(spine[0]), at(spine[1]), True),
        ("Neck", "Body", at(neck), at(poll), True),
        ("Head", "Neck", at(poll), at(nose), True),
    ]
    if tail:
        j.append(("Tail", "Body", at(tail[0]), at(tail[1]), True))
    if ears:
        (bx, by), (tx, ty) = ears
        fz = poll[0]
        j.append(("EarL", "Head", at((fz, by), -bx), at((fz, ty), -tx), True))
        j.append(("EarR", "Head", at((fz, by), bx), at((fz, ty), tx), True))
    return j


def legs(length, height, belly):
    """Everything below the belly line, either side of the tail, on the Body:
    otherwise the front legs lean towards the neck's bone and swing when the
    head goes down to graze, and the hind legs follow the tail's swish."""
    h = belly * height / 2
    return [("Body", (sgn * 0.35, h, 0.0), (0.3, h, length / 2)) for sgn in (-1, 1)]


def arms(side_bones, points, radius, root, **extra):
    """Arm chains for both sides from the right side's points (Bevy +X)."""
    out = []
    for sgn, side in ((-1, "L"), (1, "R")):
        out.append(dict(bones=[bn + side for bn in side_bones], root=root, radius=radius,
                        points=[(sgn * abs(x), y, z) for x, y, z in points], **extra))
    return out


def survivor_arms(shoulder, elbow, wrist, hand, radius):
    return arms(["Shoulder", "Elbow", "Hand"],
                [(shoulder[0], shoulder[1], 0.0), (elbow[0], elbow[1], 0.0), (wrist[0], wrist[1], -0.01),
                 (hand[0] + 0.02, hand[1] - 0.06, -0.01)],
                radius, "Chest")


def survivor_joints(hip, knee, ankle, chest, neck, head, top, sx, sy, ex, ey, wx, wy, hx, hy, hip_x, knee_x,
                    ankle_x, toe):
    return [
        ("Body", None, (0.0, 0.0, 0.0), (0.0, 0.3, 0.0), False),
        ("Pelvis", "Body", (0.0, hip + 0.04, 0.0), (0.0, 1.03, 0.0), True),
        ("HipL", "Pelvis", (-hip_x, hip, 0.0), None, True),
        ("KneeL", "HipL", (-knee_x, knee, 0.0), None, True),
        ("FootL", "KneeL", (-ankle_x, ankle, 0.015), (-ankle_x, 0.02, toe), True),
        ("HipR", "Pelvis", (hip_x, hip, 0.0), None, True),
        ("KneeR", "HipR", (knee_x, knee, 0.0), None, True),
        ("FootR", "KneeR", (ankle_x, ankle, 0.015), (ankle_x, 0.02, toe), True),
        ("Torso", "Pelvis", (0.0, 1.03, 0.0), None, True),
        ("Chest", "Torso", (0.0, chest, 0.0), (0.0, neck, 0.0), True),
        ("Neck", "Chest", (0.0, neck, 0.005), None, True),
        ("Head", "Neck", (0.0, head, 0.0), (0.0, top, 0.0), True),
        ("ShoulderL", "Chest", (-sx, sy, 0.0), None, True),
        ("ElbowL", "ShoulderL", (-ex, ey, 0.0), None, True),
        ("HandL", "ElbowL", (-wx, wy, -0.01), (-hx, hy, -0.01), True),
        ("ShoulderR", "Chest", (sx, sy, 0.0), None, True),
        ("ElbowR", "ShoulderR", (ex, ey, 0.0), None, True),
        ("HandR", "ElbowR", (wx, wy, -0.01), (hx, hy, -0.01), True),
    ]


MODELS = {
    "survivor_llanero": survivor_spec("el llanero 3d tripo", 1.82, radius=[0.13, 0.11, 0.09], shoulder=(0.2, 1.43), elbow=(0.27, 1.11), wrist=(0.32, 0.9),
                                             hand=(0.34, 0.75)),
    "survivor_coplera": survivor_spec("la coplera 3d tripo", 1.7, hip=0.87, knee=0.47, ankle=0.08, chest=1.18, neck=1.42, head=1.47,
                                             top=1.66, shoulder=(0.19, 1.35), elbow=(0.255, 1.09),
                                             wrist=(0.31, 0.84), hand=(0.32, 0.74), hip_x=0.09, knee_x=0.09,
                                             ankle_x=0.1),
    "survivor_encargado": survivor_spec("el encargado 3d tripo", 1.82, shoulder=(0.2, 1.4), elbow=(0.28, 1.11), wrist=(0.33, 0.87),
                                               hand=(0.34, 0.74)),
    "survivor_muchacho": survivor_spec("el muchacho 3d tripo", 1.78, shoulder=(0.19, 1.41), elbow=(0.23, 1.12), wrist=(0.29, 0.88),
                                              hand=(0.31, 0.73)),
    # His legs are the game's own (HIP_Y 1.66, THIGH/SHIN 0.84 in silbon.rs); the
    # Head sits so that silbon.rs's FACE (0, 0.109, -0.063) lands between his eyes
    # (measured at 2.405 m, 0.154 m ahead).
    "silbon": dict(
        src="el silbon tripo 3d", turn=180, fit=("height", 2.59), tris=40000,
        joints=[
            ("Body", None, (0.0, 0.0, 0.0), (0.0, 0.3, 0.0), False),
            ("HipL", "Body", (-0.1, 1.66, 0.0), None, True),
            ("KneeL", "HipL", (-0.105, 0.84, 0.0), (-0.105, 0.05, 0.0), True),
            ("HipR", "Body", (0.1, 1.66, 0.0), None, True),
            ("KneeR", "HipR", (0.105, 0.84, 0.0), (0.105, 0.05, 0.0), True),
            ("Torso", "Body", (0.0, 1.66, 0.0), (0.0, 2.12, 0.0), True),
            ("Coat", "Torso", (0.0, 1.66, 0.0), (0.0, 1.9, 0.0), False),
            ("Head", "Torso", (0.0, 2.296, -0.091), (0.0, 2.59, -0.05), True),
            ("ShoulderL", "Torso", (-0.24, 2.14, 0.0), None, True),
            ("ElbowL", "ShoulderL", (-0.36, 1.575, -0.05), (-0.43, 0.93, -0.1), True),
            ("ShoulderR", "Torso", (0.24, 2.14, 0.0), None, True),
            ("ElbowR", "ShoulderR", (0.36, 1.575, -0.05), (0.43, 0.93, -0.1), True),
            ("Sack", "Torso", (-0.2, 2.12, 0.12), (-0.3, 1.6, 0.3), True),
        ],
        chains=arms(["Shoulder", "Elbow"], [(0.24, 2.14, 0.0), (0.36, 1.575, -0.05), (0.43, 0.9, -0.1)],
                    [0.12, 0.11], "Torso", tight=0.06, tight_hand=0.13),
        zones=[("Sack", (-0.3, 1.8, 0.33), (0.2, 0.32, 0.22)), ("Sack", (-0.22, 2.25, 0.2), (0.12, 0.16, 0.16))],
    ),
    # Tureco: legs from the shoulder and hip to the wrist and hock (paws fold
    # there); the lower jaw is the thin wedge under the mouth line.
    "tureco": dict(
        src="tureco 3d tripo model", turn=180, fit=("length", 1.18), tris=10000,
        joints=[
            ("Body", None, (0.0, 0.5, 0.2), (0.0, 0.5, -0.33), True),
            ("Head", "Body", (0.0, 0.692, -0.408), (0.0, 0.74, -0.6), True),
            ("Jaw", "Head", (0.0, 0.7, -0.47), (0.0, 0.68, -0.58), True),
            ("Tail", "Body", (0.0, 0.552, 0.217), (0.0, 0.278, 0.58), True),
            ("LegFL", "Body", (-0.09, 0.45, -0.331), (-0.09, 0.106, -0.338), True),
            ("PawFL", "LegFL", (-0.09, 0.106, -0.338), (-0.09, 0.01, -0.41), True),
            ("LegFR", "Body", (0.09, 0.45, -0.331), (0.09, 0.106, -0.338), True),
            ("PawFR", "LegFR", (0.09, 0.106, -0.338), (0.09, 0.01, -0.41), True),
            ("LegBL", "Body", (-0.08, 0.513, 0.204), (-0.08, 0.144, 0.287), True),
            ("PawBL", "LegBL", (-0.08, 0.144, 0.287), (-0.08, 0.01, 0.255), True),
            ("LegBR", "Body", (0.08, 0.513, 0.204), (0.08, 0.144, 0.287), True),
            ("PawBR", "LegBR", (0.08, 0.144, 0.287), (0.08, 0.01, 0.255), True),
        ],
        force=[("Jaw", (0.0, 0.672, -0.52), (0.05, 0.022, 0.07))],
    ),
    "cow": dict(src="low-poly cow 3d model", turn=180, fit=("length", 2.29), tris=10000,
                joints=beast(2.29, 1.753, ((0.12, 0.74), (0.66, 0.78)), (0.757, 0.67), (0.865, 0.88), (0.99, 0.71),
                             tail=((0.065, 0.7), (0.01, 0.19)), ears=((0.12, 0.8), (0.35, 0.78))),
                force=legs(2.29, 1.753, 0.45)),
    "bull": dict(src="low-poly bull 3d model", turn=180, fit=("length", 2.58), tris=10000,
                 joints=beast(2.58, 1.632, ((0.12, 0.81), (0.72, 0.9)), (0.8, 0.77), (0.89, 0.9), (0.99, 0.65),
                              tail=((0.12, 0.81), (0.01, 0.2)), ears=((0.12, 0.8), (0.24, 0.8))),
                 force=legs(2.58, 1.632, 0.45)),
    "calf": dict(src="goat low-poly 3d model", turn=180, fit=("length", 1.38), tris=8000,
                 joints=beast(1.38, 1.126, ((0.11, 0.82), (0.74, 0.85)), (0.77, 0.77), (0.896, 0.91), (0.99, 0.75),
                              tail=((0.1, 0.83), (0.02, 0.35)), ears=((0.09, 0.86), (0.24, 0.84))),
                 force=legs(1.38, 1.126, 0.5)),
    "horse": dict(src="horse 3d model", turn=180, fit=("length", 2.4), tris=10000,
                  joints=beast(2.4, 1.992, ((0.195, 0.645), (0.62, 0.7)), (0.65, 0.645), (0.886, 0.94), (0.99, 0.73),
                               tail=((0.187, 0.65), (0.016, 0.1))),
                  force=legs(2.4, 1.992, 0.48)),
    "capybara": dict(src="capybara 3d model", turn=180, fit=("length", 1.2), tris=30000,
                     joints=beast(1.2, 0.799, ((0.12, 0.65), (0.69, 0.74)), (0.72, 0.7), (0.8, 0.84), (0.99, 0.79)),
                     force=legs(1.2, 0.799, 0.3)),
    "caiman": dict(src="crocodile 3d model", turn=90, fit=("length", 2.5), tris=10000,
                   joints=beast(2.5, 0.6, ((0.42, 0.42), (0.765, 0.56)), (0.765, 0.56), (0.85, 0.69), (1.0, 0.86),
                                tail=((0.39, 0.49), (0.0, 0.42)))),
    "egret": dict(src="egret 3d model", turn=180, fit=("height", 0.95), tris=8000,
                  joints=beast(0.696, 0.95, ((0.03, 0.37), (0.5, 0.58)), (0.53, 0.66), (0.71, 0.93), (1.0, 0.84)),
                  force=legs(0.696, 0.95, 0.33)),
    # The truck faces +X like the old one (`vehicles.rs`); its lamp lenses are
    # separate shapes in the materials the game lights while the engine runs.
    "truck": dict(src="rusty pickup truck 3d model", turn=90, fit=("length", 5.78), tris=30000, joints=[],
                  lenses=[("truck_lamp_head", (2.54, 1.33, z), 0.15, (1.0, 0.85, 0.6)) for z in (0.905, -0.874)]
                  + [("truck_lamp_amber", (2.45, 1.49, z), 0.05, (1.0, 0.45, 0.05)) for z in (1.31, -1.31)]
                  + [("truck_lamp_tail", (-2.8, 0.95, z), 0.07, (0.9, 0.05, 0.02)) for z in (1.0, -1.0)]),
}


# Rotations (degrees about the bone's own X, Y, Z: X is the game's X, Y is up)
# that bend every limb, to see the weights in one frame.
TEST_POSES = {
    # The game's own motions: the torch arm raised ahead, the other swung
    # back, a stride, a glance; the arms barely ever go out to the side.
    "survivor": {"ShoulderR": (75, 0, 8), "ElbowR": (35, 0, 0), "ShoulderL": (-35, 0, -4), "HipL": (40, 0, 0),
                 "KneeL": (-60, 0, 0), "HipR": (-20, 0, 0), "Head": (0, 35, 0), "Chest": (8, 0, 0)},
    "tureco": {"LegFL": (35, 0, 0), "PawFL": (-50, 0, 0), "LegBR": (-30, 0, 0), "PawBR": (40, 0, 0),
               "Head": (-20, 30, 0), "Jaw": (25, 0, 0), "Tail": (0, 0, 40)},
    "beast": {"Neck": (-40, 0, 0), "Head": (0, 25, 0), "Tail": (0, 0, 35), "EarL": (0, 0, 35)},
    "silbon": {"ShoulderR": (70, 0, 0), "ElbowR": (35, 0, 0), "ShoulderL": (-30, 0, 0), "HipL": (40, 0, 0),
               "KneeL": (-60, 0, 0), "HipR": (-20, 0, 0), "Head": (0, 35, 0), "Sack": (25, 0, 0)},
}


# ---------------------------------------------------------------- the mesh
def load(spec):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.join(SRC, spec["src"] + ".glb"))
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    for o in meshes:
        o.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    for o in list(bpy.data.objects):
        if o is not ob:
            bpy.data.objects.remove(o)
    ob.parent = None
    ob.matrix_world = Matrix.Identity(4)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    me = ob.data
    co = np.empty(len(me.vertices) * 3)
    me.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    kind, val = spec["fit"]
    longest = (co.max(0) - co.min(0)).max()
    k = val if kind == "scale" else val / longest
    if kind == "height":
        k = val / (co[:, 2].max() - co[:, 2].min())
    rot = Matrix.Rotation(math.radians(spec["turn"]), 4, "Z")
    me.transform(rot @ Matrix.Scale(k, 4))
    co = np.empty(len(me.vertices) * 3)
    me.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    # Feet on the ground, centred over the origin.
    lo, hi = co.min(0), co.max(0)
    me.transform(Matrix.Translation(V(-(lo[0] + hi[0]) / 2, -(lo[1] + hi[1]) / 2, -lo[2])))
    me.update()
    ob.name = me.name = spec["name"]
    return ob


def weld(ob):
    """glTF splits vertices along every UV seam; heat weighting needs them
    joined (the UVs live on the face corners and survive)."""
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    before = len(bm.verts)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
    bm.to_mesh(ob.data)
    bm.free()
    ob.data.update()
    return before - len(ob.data.vertices)


def decimate(ob, target):
    tris = sum(len(p.vertices) - 2 for p in ob.data.polygons)
    if tris <= target:
        return tris
    mod = ob.modifiers.new("decimate", "DECIMATE")
    mod.ratio = target / tris
    mod.use_collapse_triangulate = True
    select_only([ob])
    with bpy.context.temp_override(**view3d_override(), active_object=ob, object=ob):
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return sum(len(p.vertices) - 2 for p in ob.data.polygons)


def shrink_textures(ob, size):
    for mat in ob.data.materials:
        if not mat or not mat.node_tree:
            continue
        for n in mat.node_tree.nodes:
            if n.type == "TEX_IMAGE" and n.image and max(n.image.size) > size:
                n.image.scale(size, size)


# ---------------------------------------------------------------- the rig
def tails(joints):
    out = {}
    for name, parent, pos, tail, _ in joints:
        if tail is not None:
            out[name] = b(*tail)
            continue
        child = next((j for j in joints if j[1] == name), None)
        out[name] = b(*child[2]) if child else b(*pos) + V(0, 0, 0.08)
    return out


def read_weights(ob, names):
    """Vertex weights as a dense (vertices x bones) matrix."""
    col = {g.index: names.index(g.name) for g in ob.vertex_groups if g.name in names}
    w = np.zeros((len(ob.data.vertices), len(names)), np.float32)
    for v in ob.data.vertices:
        for g in v.groups:
            if g.group in col:
                w[v.index, col[g.group]] = g.weight
    return w


def write_weights(ob, names, w):
    ob.vertex_groups.clear()
    groups = [ob.vertex_groups.new(name=n) for n in names]
    w = w / np.maximum(w.sum(1, keepdims=True), 1e-9)
    for j, g in enumerate(groups):
        for i in np.nonzero(w[:, j] > 1e-3)[0]:
            g.add([int(i)], float(w[i, j]), "REPLACE")


def neighbours(me):
    e = np.empty(len(me.edges) * 2, np.int64)
    me.edges.foreach_get("vertices", e)
    return e.reshape(-1, 2)


def flood(w, edges, steps=400):
    """Unweighted vertices take the mean of their weighted neighbours, ring by ring."""
    for _ in range(steps):
        empty = w.sum(1) < 1e-4
        if not empty.any():
            break
        acc = np.zeros_like(w)
        cnt = np.zeros(len(w), np.float32)
        for a, c in ((0, 1), (1, 0)):
            src, dst = edges[:, a], edges[:, c]
            ok = ~empty[src] & empty[dst]
            np.add.at(acc, dst[ok], w[src[ok]])
            np.add.at(cnt, dst[ok], 1.0)
        got = cnt > 0
        if not got.any():
            break
        w[got] = acc[got] / cnt[got, None]
    return w


def polyline(co, pts, closest=False):
    """For each vertex: distance to the polyline, which segment, how far along
    it (and, with `closest`, the nearest point on it)."""
    best = np.full(len(co), np.inf)
    seg = np.zeros(len(co), np.int64)
    frac = np.zeros(len(co))
    near = np.zeros_like(co)
    for i in range(len(pts) - 1):
        a, d = pts[i], pts[i + 1] - pts[i]
        t = ((co - a) @ d) / max(d @ d, 1e-9)
        tc = np.clip(t, 0.0, 1.0)
        p = a + tc[:, None] * d
        dist = np.linalg.norm(co - p, axis=1)
        if i == 0:
            dist = np.where(t < 0, np.inf, dist)
        better = dist < best
        best[better], seg[better], frac[better] = dist[better], i, tc[better]
        near[better] = p[better]
    return (best, seg, frac, near) if closest else (best, seg, frac)


def islands(edges, n):
    """Connected pieces of the mesh: a label per vertex."""
    parent = np.arange(n)

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for a, c in edges:
        ra, rc = find(a), find(c)
        if ra != rc:
            parent[ra] = rc
    return np.array([find(i) for i in range(n)])


def segments(joints):
    ends = tails(joints)
    names = [n for n, _, _, _, d in joints if d]
    heads = np.array([tuple(b(*p)) for n, _, p, _, d in joints if d])
    tips = np.array([tuple(ends[n]) for n, _, _, _, d in joints if d])
    return names, heads, tips


def seg_dist(co, heads, tips):
    """Distance from every vertex to every bone segment (vertices x bones)."""
    out = np.empty((len(co), len(heads)))
    for j, (h, t) in enumerate(zip(heads, tips)):
        d = t - h
        k = np.clip(((co - h) @ d) / max(d @ d, 1e-9), 0.0, 1.0)
        out[:, j] = np.linalg.norm(co - (h + k[:, None] * d), axis=1)
    return out


def weigh(ob, joints, chains, zones, force=(), sharp=6.0, smooth=4):
    """Weights by geometry, piece by piece.

    A Tripo model is a few dozen separate pieces (shoes, trousers, a jacket,
    hands, a skirt). Each vertex blends its nearest bones by distance, but a
    piece may only use the bones that belong to it: a piece that is mostly
    arm (inside one of the `chains` tubes) follows that arm alone; a piece
    the arm barely touches (a skirt by a hanging hand) may not use the arm at
    all; a piece with sleeves uses the arm only inside the tube. A piece
    mostly inside a `zone` (a sack) follows that zone's bone alone."""
    names, heads, tips = segments(joints)
    co = np.empty(len(ob.data.vertices) * 3)
    ob.data.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    edges = neighbours(ob.data)
    label = islands(edges, len(co))
    dist = seg_dist(co, heads, tips)
    arm_cols = {}
    tube = {}
    upper = {}
    tight = {}
    shoulder = {}
    tips = {}
    narrow = {}
    for chain in chains:
        pts = np.array([tuple(b(*p)) for p in chain["points"]])
        dd, seg, along, near = polyline(co, pts, closest=True)
        # Thinner on the side towards the body below the shoulder, so the
        # cloth under the arm (a jacket's side) stays with the chest.
        side = np.sign(pts[0][0]) or 1.0
        inward = ((co[:, 0] - near[:, 0]) * side < 0) & (co[:, 2] < pts[0][2] - 0.08)
        r = np.array(chain["radius"])[seg]
        k = chain["bones"][0]
        tube[k] = dd < r
        upper[k] = tube[k] & (seg == 0)
        shoulder[k] = (seg == 0) & (along < 0.3)
        tips[k] = tube[k] & (seg == len(pts) - 2) & (along > 0.75)
        # Within a piece that is also body (a jacket), the arm keeps only the
        # tube's outer side next to the upper arm and forearm.
        inner = np.minimum(r, chain.get("inner", 0.05))
        soft = np.clip((r - dd) / np.maximum(r - inner, 1e-6), 0.0, 1.0)
        tight[k] = np.where(inward & (seg <= 1), soft, (dd < r).astype(float))
        if "tight" in chain:
            # A bare arm through a ragged coat: the arm alone, close round its bones.
            # The hand's fingers (or claws) spread wider than the arm.
            hand = (seg == len(pts) - 2) & (along > 0.45)
            rt = np.where(hand, chain.get("tight_hand", chain["tight"]), chain["tight"])
            tight[k] = np.clip((rt * 1.4 - dd) / (rt * 0.4), 0.0, 1.0)
            rn = chain["tight"]
            narrow[k] = np.clip((rn * 1.4 - dd) / (rn * 0.4), 0.0, 1.0)
        arm_cols[chain["bones"][0]] = [names.index(n) for n in chain["bones"]]
    all_arm = [c for cols in arm_cols.values() for c in cols]
    hand_whole = {}
    for k in tube:
        held = 0
        for isl in np.unique(label[tips[k]]):
            rows = label == isl
            if tube[k][rows].mean() > 0.45:
                held += (rows & tips[k]).sum()
        hand_whole[k] = held > 0.5 * max(tips[k].sum(), 1)
    allowed = np.ones((len(co), len(names)), bool)
    fade = np.ones((len(co), len(names)))
    report = []
    for isl in np.unique(label):
        rows = label == isl
        n = rows.sum()
        zone = None
        for bone, centre, half in zones:
            c, h = np.array(tuple(b(*centre))), np.abs(np.array(tuple(b(*half))))
            if (np.all(np.abs(co[rows] - c) <= h, axis=1)).mean() > 0.6:
                zone = names.index(bone)
        if zone is not None:
            allowed[rows] = False
            allowed[rows, zone] = True
            report.append(f"{n}:{names[zone]}")
            continue
        fracs = {k: t[rows].mean() for k, t in tube.items()}
        if os.environ.get("TRIPO_DEBUG") and n > 150:
            lo_, hi_ = co[rows].min(0), co[rows].max(0)
            print("  piece", n, "x", round(lo_[0], 2), round(hi_[0], 2), "z", round(lo_[2], 2), round(hi_[2], 2),
                  {k: (round(float(f), 3), int(upper[k][rows].sum())) for k, f in fracs.items()})
        whole = [k for k, f in fracs.items() if f > 0.45]
        if whole:
            k = whole[0]
            allowed[rows] = False
            allowed[np.ix_(rows, arm_cols[k])] = True
            root = next(c["root"] for c in chains if c["bones"][0] == k)
            allowed[rows & shoulder[k], names.index(root)] = True
            report.append(f"{n}:{k}")
            continue
        for k, cols in arm_cols.items():
            # A sleeve reaches the upper arm; a skirt by a hanging hand never does.
            if upper[k][rows].sum() < 20:
                allowed[np.ix_(rows, cols)] = False
            else:
                # A hand already in a piece of its own needs no wider tube here.
                tk = narrow[k] if (hand_whole[k] and k in narrow) else tight[k]
                outside = rows & (tk <= 0.0)
                allowed[np.ix_(outside, cols)] = False
                fade[np.ix_(rows, cols)] = np.minimum(fade[np.ix_(rows, cols)], tk[rows, None])
    for bone, centre, half in force:
        c, h = np.array(tuple(b(*centre))), np.abs(np.array(tuple(b(*half))))
        inside = np.all(np.abs(co - c) <= h, axis=1)
        allowed[inside] = False
        allowed[inside, names.index(bone)] = True
    w = np.where(allowed, fade / np.maximum(dist, 0.004) ** sharp, 0.0)
    for _ in range(smooth):
        acc = np.zeros_like(w)
        cnt = np.zeros(len(w))
        np.add.at(acc, edges[:, 0], w[edges[:, 1]])
        np.add.at(acc, edges[:, 1], w[edges[:, 0]])
        np.add.at(cnt, edges[:, 0], 1)
        np.add.at(cnt, edges[:, 1], 1)
        w = w / np.maximum(w.sum(1, keepdims=True), 1e-12)
        acc = acc / np.maximum(acc.sum(1, keepdims=True), 1e-12)
        w = np.where(allowed, 0.5 * w + 0.5 * acc, 0.0)
    # Four influences at most, as the export keeps.
    w = w / np.maximum(w.sum(1, keepdims=True), 1e-12)
    keep = np.argsort(-w, axis=1)[:, :4]
    mask = np.zeros_like(w, bool)
    np.put_along_axis(mask, keep, True, axis=1)
    w = np.where(mask, w, 0.0)
    write_weights(ob, names, w)
    return f"{len(np.unique(label))} pieces; single-bone pieces {report}"


def bind(ob, spec):
    joints = spec["joints"]
    note = weigh(ob, joints, spec.get("chains", []), spec.get("zones", []), spec.get("force", []))
    kit = modelkit.Asset.__new__(modelkit.Asset)
    kit.name = spec["name"]
    kit.coll = bpy.context.scene.collection
    rig = kit.rig([ob], [(n, p, pos) for n, p, pos, _, _ in joints])
    return rig, note


def nearest_fill(ob, joints):
    """Vertices nothing reached take the nearest deforming bone."""
    ends = tails(joints)
    bones = [(n, b(*p), ends[n]) for n, _, p, _, d in joints if d]
    vg = {g.name: g for g in ob.vertex_groups}
    for n, _, _ in bones:
        if n not in vg:
            vg[n] = ob.vertex_groups.new(name=n)
    idx = {g.index for g in ob.vertex_groups}
    for v in ob.data.vertices:
        if any(g.group in idx and g.weight > 1e-4 for g in v.groups):
            continue
        best, bd = None, 1e9
        for n, h, t in bones:
            d = (t - h)
            k = max(0.0, min(1.0, (v.co - h).dot(d) / max(d.length_squared, 1e-9)))
            dist = (v.co - (h + d * k)).length
            if dist < bd:
                best, bd = n, dist
        vg[best].add([v.index], 1.0, "REPLACE")


# ---------------------------------------------------------------- renders
def grid_render(ob, joints, out):
    """Front and side orthographic views on a 10 cm grid, joints as dots."""
    os.makedirs(out, exist_ok=True)
    sc = bpy.context.scene
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.display.shading.light = "FLAT"
    sc.display.shading.color_type = "TEXTURE"
    sc.render.film_transparent = False
    sc.world = bpy.data.worlds.new("w")
    sc.world.color = (0.75, 0.75, 0.75)
    co = np.empty(len(ob.data.vertices) * 3)
    ob.data.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    lo, hi = Vector(co.min(0)), Vector(co.max(0))
    span = max(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z) * 1.08
    res = 1000
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = span
    sc.render.resolution_x = sc.render.resolution_y = res
    cz = (lo.z + hi.z) / 2
    views = {
        # name: camera location, rotation, (image-right axis, sign)
        "front": (V(0, 50, cz), Euler((math.radians(90), 0, math.radians(180))), (0, -1)),
        "side": (V(50, 0, cz), Euler((math.radians(90), 0, math.radians(90))), (1, 1)),
    }
    for name, (loc, rot, (axis, sign)) in views.items():
        cam.location, cam.rotation_euler = loc, rot
        path = os.path.join(out, f"{name}.png")
        sc.render.filepath = path
        bpy.ops.render.render(write_still=True)
        img = bpy.data.images.load(path)
        px = np.array(img.pixels[:]).reshape(res, res, 4)

        def to_px(h, v):
            return (int((h * sign) / span * res + res / 2), int((v - cz) / span * res + res / 2))

        step = 0.1
        k = math.floor(-span / 2 / step)
        while k * step <= span / 2:
            x, _ = to_px(k * step * sign, cz)
            if 0 <= x < res:
                px[:, x, :3] = px[:, x, :3] * 0.5 + (np.array([0.8, 0.2, 0.2]) if k % 5 == 0 else 0.25) * 0.5
            k += 1
        k = math.floor((cz - span / 2) / step)
        while k * step <= cz + span / 2:
            _, y = to_px(0, k * step)
            if 0 <= y < res:
                px[y, :, :3] = px[y, :, :3] * 0.5 + (np.array([0.8, 0.2, 0.2]) if k % 5 == 0 else 0.25) * 0.5
            k += 1
        for jn, _, pos, _, _ in joints:
            p = b(*pos)
            x, y = to_px(p[axis], p.z)
            px[max(0, y - 4):y + 5, max(0, x - 4):x + 5, :3] = (0.1, 0.9, 0.2) if "L" in jn[-1:] else (0.1, 0.4, 1.0)
        img.pixels[:] = px.ravel()
        img.save()
    print("measure:", out, "lo", tuple(round(c, 3) for c in lo), "hi", tuple(round(c, 3) for c in hi))


def pose_render(ob, rig, spec, out):
    """The rig bent: knees, an arm up, the head turned; checks the weights."""
    poses = spec.get("test_pose", {})
    for name, (rx, ry, rz) in poses.items():
        pb = rig.pose.bones.get(name)
        if pb:
            pb.rotation_mode = "XYZ"
            pb.rotation_euler = (math.radians(rx), math.radians(ry), math.radians(rz))
    grid_render(ob, [], os.path.join(out, "pose"))
    for pb in rig.pose.bones:
        pb.rotation_euler = (0, 0, 0)


def lenses(spec):
    """Discs over the model's lamps, in materials named as the game expects."""
    import bmesh
    out = []
    for name, (x, y, z), radius, colour in spec.get("lenses", []):
        mat = bpy.data.materials.get(name)
        if mat is None:
            mat = bpy.data.materials.new(name)
            mat.use_nodes = True
            bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
            bsdf.inputs["Base Color"].default_value = (*colour, 1.0)
            bsdf.inputs["Emission Color"].default_value = (*colour, 1.0)
            bsdf.inputs["Emission Strength"].default_value = 2.0
        me = bpy.data.meshes.new(name)
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=20, radius1=radius, radius2=radius, depth=0.03)
        bm.to_mesh(me)
        bm.free()
        me.materials.append(mat)
        o = bpy.data.objects.new(name, me)
        bpy.context.scene.collection.objects.link(o)
        # The disc faces along the truck's length (Blender X).
        o.rotation_euler = (0.0, math.radians(90), 0.0)
        o.location = b(x, y, z)
        out.append(o)
    return out


def export(ob, rig, path, extra=()):
    objs = [ob] + ([rig] if rig else []) + list(extra)
    select_only(objs)
    with bpy.context.temp_override(**view3d_override()):
        bpy.ops.export_scene.gltf(
            filepath=path, export_format="GLB", use_selection=True, export_yup=True, export_apply=False,
            export_skins=bool(rig), export_influence_nb=4, export_animations=False, export_tangents=False,
            export_materials="EXPORT", export_image_format="JPEG", export_jpeg_quality=88,
            export_cameras=False, export_lights=False,
            export_copyright=f"{ob.name}: Tripo-generated, prepared by tools/models/tripo.py")
    print("wrote", path, f"{os.path.getsize(path) / 1e6:.1f} MB")


def main():
    name = ARGV[0]
    spec = dict(MODELS[name], name=name)
    if spec["joints"] and "test_pose" not in spec:
        spec["test_pose"] = TEST_POSES.get(name.split("_")[0], TEST_POSES["beast"])
    ob = load(spec)
    shots = os.path.join(SHOTS, name)
    if "--measure" in ARGV:
        grid_render(ob, spec["joints"], shots)
        return
    welded = weld(ob)
    tris = decimate(ob, spec["tris"])
    print(f"{name}: welded {welded} seam vertices; {len(ob.data.vertices)} vertices after decimation")
    shrink_textures(ob, TEX)
    rig = None
    if spec["joints"]:
        rig, missed = bind(ob, spec)
        print(f"{name}: {tris} tris; {missed}")
        pose_render(ob, rig, spec, shots)
    else:
        print(f"{name}: {tris} tris, static")
    extra = lenses(spec)
    if extra:
        grid_render(ob, [], os.path.join(shots, "lenses"))
    export(ob, rig, os.path.join(ROOT, "assets", "models", f"{name}.glb"), extra)


main()

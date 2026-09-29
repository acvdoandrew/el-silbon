"""Review renders of any exported model (headless), for the review sheets:

    PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/review.py -- IN.glb OUTDIR [VIEWS]

VIEWS: a comma list of name:yaw:pitch:distance[:cx:cy:cz] (Blender coords;
an imported model faces +Y, so yaw pi is its front). The Silbón's sheet
(`assets/models/src/renders/sheet.png`) is six of these tiled with ffmpeg:
front:3.1416:0.06:0.9, side:1.5708:0.05:0.9, back:0:0.06:0.9,
three:2.5:0.08:0.9, hat:2.75:0.12:0.3:0:0:2.7, face:3.0:0.02:0.2:0:0.1:2.62."""
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modelkit  # noqa: E402

args = sys.argv[sys.argv.index("--") + 1:]
src, out = args[0], args[1]
bpy.ops.wm.read_factory_settings(use_empty=True)
coll = bpy.data.collections.new("review")
bpy.context.scene.collection.children.link(coll)
bpy.ops.import_scene.gltf(filepath=src)
for o in list(bpy.context.selected_objects):
    for c in o.users_collection:
        c.objects.unlink(o)
    coll.objects.link(o)
views = []
spec = args[2] if len(args) > 2 else "three:2.5:0.08:0.95,front:3.1416:0.06:0.95,side:1.5708:0.05:0.95,back:0:0.06:0.95"
for v in spec.split(","):
    parts = v.split(":")
    name, yaw, pitch, dist = parts[0], float(parts[1]), float(parts[2]), float(parts[3])
    if len(parts) > 4:
        views.append((name, yaw, pitch, dist, tuple(float(x) for x in parts[4:7])))
    else:
        views.append((name, yaw, pitch, dist))
size = (900, 900) if "--square" in args else (900, 1100)
modelkit.render_views(coll, out, views, samples=96, size=size)

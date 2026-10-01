#!/usr/bin/env python3
"""Cut El Silbón — The Return, trailer 3 (the Madrina), in English or Spanish.

Inputs:
  --lang en|es   the narration, cards and subtitles
  --frames DIR   the rendered shots (`el_silbon --trailer --shots X` writes
                 X/trailer/<NN_shot>/00000.png…)
  --stems DIR    the generated sound: DIR/<lang>/n1…n11.mp3 (narration),
                 DIR/m1…m4.mp3 (score), DIR/sfx_*.mp3 (hits)
  --out FILE     the finished trailer (.mp4, 1920x1080, 30 fps)

The game's own sounds (whistles, the catch, thunder, rain, Tureco…) come
from assets/audio. One timeline of segments, so the picture and the sound
are cut from the same numbers. See trailer/script_v3.md.
"""

import argparse
import json
import os
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, ".."))
AUDIO = os.path.join(ROOT, "assets", "audio")
SERIF = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Regular.ttf")
ITALIC = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Italic.ttf")
FPS = 30
W, H = 1920, 1080
BONE = "0xEDE6D8"
GOLD = "0xC8A060"
RED = "0xB51A1A"
DIM = "0x8A857A"


def spaced(text):
    """Wide-tracked capitals, the trailer's card style."""
    return "  ".join(" ".join(word) for word in text.split(" "))


# ---------------------------------------------------------------------------
# Words: (english, spanish)
# ---------------------------------------------------------------------------

CARDS = {
    "legend_1": ("INSPIRED BY THE LEGEND", "INSPIRADO EN LA LEYENDA"),
    "legend_2": ("OF THE VENEZUELAN LLANOS", "DE LOS LLANOS VENEZOLANOS"),
    "bones": ("FIVE BUNDLES OF HIS FATHER'S BONES", "CINCO ATADOS CON LOS HUESOS DE SU PADRE"),
    "road": ("ONE ROAD OUT", "UNA SOLA SALIDA"),
    "dawn": ("OR LAST UNTIL DAWN", "O AGUANTA HASTA EL ALBA"),
    "friends": ("UP TO FOUR FRIENDS", "HASTA CUATRO AMIGOS"),
    "lies": ("THE WHISTLE LIES", "EL SILBIDO MIENTE"),
    "refrain_2": ("If you hear him far away, he is already here.", "Hay silbidos que nunca se deben seguir."),
    "players": ("SOLO OR WITH UP TO FOUR FRIENDS", "SOLO O CON HASTA CUATRO AMIGOS"),
    "langs": ("ENGLISH  ·  ESPAÑOL", "ESPAÑOL  ·  ENGLISH"),
    "dev": ("IN DEVELOPMENT", "EN DESARROLLO"),
}

# Narration, the Madrina: (stem, segment, offset, english, spanish)
VOICE = [
    ("n1", "cold", 0.2, "Shh… listen. Do you hear it?", "Shh… escucha. ¿Lo oyes?"),
    ("n2", "gate", 0.3,
     "My madrina told us this story every time it rained… so we would never cross the llano at night.",
     "Mi madrina nos echaba este cuento cada vez que llovía… para que nunca cruzáramos el llano de noche."),
    ("n3", "ceiba", 0.2, "There was a boy… who killed his own father.", "Había un muchacho… que mató a su propio padre."),
    ("n4", "tureco", 0.5, "His grandfather tied him to a post… whipped him… and set the dog on him.",
     "Su abuelo lo amarró a un botalón… lo azotó… y le soltó el perro."),
    ("n5", "face", 0.3, "Then he cursed him. To carry his father's bones in a sack… forever.",
     "Y lo maldijo. A cargar los huesos de su padre en un saco… para siempre."),
    ("n6", "fields", 3.2, "Now he walks the llano in the rain… whistling… so everyone knows he is coming.",
     "Ahora anda por el llano cuando llueve… silbando… para que todos sepan que viene."),
    ("n7", "carry", 0.3,
     "Some nights, his bones spill across the ranch. Bring them home to the ceiba… and he will hunt you for every one.",
     "Hay noches en que sus huesos se riegan por el hato. Llévalos a la ceiba… y te cazará por cada uno."),
    ("n8", "party", 0.2, "Stay together. Never leave anyone alone in the dark.",
     "No se separen. Nunca dejen a nadie solo en la oscuridad."),
    ("n9", "velo", 0.4, "And remember… if you hear him close… he is far away.",
     "Y acuérdate… si lo oyes cerca… está lejos."),
    ("n10", "black", 0.2, "But if you hear him far away…", "Pero si lo oyes lejos…"),
    ("n11", "title", 0.6, "…he is already here.", "…ya está aquí."),
]

LANG = 0  # 0 English, 1 Spanish; set from --lang


def word(key):
    return CARDS[key][LANG]


def card(*lines):
    return {"lines": list(lines)}


# ---------------------------------------------------------------------------
# The picture: (id, kind, seconds, options)
# ---------------------------------------------------------------------------

def segments():
    big = lambda key, color=BONE, size=50: (spaced(word(key)), SERIF, size, color, 0)
    return [
        ("cold", "black", 3.5, {}),
        ("legend", "card", 3.5, card((spaced(word("legend_1")), SERIF, 26, "0xC8C0B0", -22),
                                     (spaced(word("legend_2")), SERIF, 26, "0xC8C0B0", 22))),
        ("gate", "shot", 5.0, {"dir": "01_gate", "fade_in": 1.0, "fade_out": 0.35}),
        ("caiman", "shot", 4.0, {"dir": "23_caiman", "fade_in": 0.35, "fade_out": 0.35}),
        ("ceiba", "shot", 4.5, {"dir": "02_ceiba", "fade_in": 0.35, "fade_out": 0.35}),
        ("tureco", "shot", 4.0, {"dir": "05_tureco", "fade_in": 0.35, "fade_out": 0.35}),
        ("corral", "shot", 3.5, {"dir": "03_corral", "fade_in": 0.35, "fade_out": 0.35}),
        ("face", "shot", 5.0, {"dir": "22_face", "fade_in": 0.5, "fade_out": 0.3}),
        ("fields", "shot", 4.0, {"dir": "04_fields_flash"}),
        ("sack", "shot", 4.0, {"dir": "24_sack", "fade_out": 0.3}),
        ("card_bones", "card", 1.8, card(big("bones", size=40))),
        ("carry", "shot", 3.5, {"dir": "06_carry"}),
        ("lay", "shot", 4.5, {"dir": "17_lay"}),
        ("count", "shot", 4.0, {"dir": "25_count", "fade_out": 0.3}),
        ("card_road", "card", 1.6, card(big("road"))),
        ("truck", "shot", 3.5, {"dir": "08_truck"}),
        ("cano", "shot", 4.0, {"dir": "19_cano"}),
        ("card_dawn", "card", 1.8, card(big("dawn", GOLD))),
        ("card_friends", "card", 1.6, card(big("friends"))),
        ("party", "shot", 3.5, {"dir": "16_party"}),
        ("downed", "shot", 4.0, {"dir": "20_downed", "fade_out": 0.4}),
        ("card_lies", "card", 2.0, card(big("lies", RED, 56))),
        ("velo", "shot", 5.0, {"dir": "21_velo"}),
        ("hide", "shot", 4.5, {"dir": "12_hide"}),
        ("chase", "shot", 3.5, {"dir": "13_chase"}),
        ("reveal", "shot", 2.2, {"dir": "14_reveal"}),
        # The catch, to the black (s = 1.75 of its clock).
        ("catch", "shot", 2.8, {"dir": "15_catch", "frames": 84}),
        ("black", "black", 3.0, {}),
        ("title", "card", 9.0, {"fade_in": 0.2, "fade_out": 1.0, "lines": [
            (spaced("EL SILBÓN"), SERIF, 136, BONE, -90, 1.4),
            (spaced("THE RETURN"), SERIF, 36, GOLD, 30, 1.8),
            ("Si lo oyes lejos, ya está aquí.", ITALIC, 36, "0xD8D0C0", 150, 3.4),
            (word("refrain_2"), ITALIC, 24, DIM, 200, 4.0),
        ]}),
        ("end", "card", 5.0, {"fade_in": 0.6, "fade_out": 1.2, "lines": [
            (spaced(word("players")), SERIF, 30, "0xD8D0C0", -40),
            (word("langs"), SERIF, 22, DIM, 10),
            (spaced(word("dev")), SERIF, 24, GOLD, 60),
        ]}),
    ]


SEGMENTS = []
START = {}
TOTAL = 0.0


def lay_out():
    global SEGMENTS, START, TOTAL
    SEGMENTS = segments()
    t = 0.0
    for sid, _, dur, _ in SEGMENTS:
        START[sid] = t
        t += dur
    TOTAL = t


def at(sid, offset=0.0):
    return START[sid] + offset


# Everything else heard: (file, start seconds, gain dB, options)
def sounds(stems):
    g = lambda name: os.path.join(AUDIO, name)
    s = lambda name: os.path.join(stems, name)
    catch = at("catch")
    black = at("black")
    lay = at("lay", 2.3)
    return [
        # Beds: rain the whole night until the catch's silence; the llano's
        # insects until the lie; the dread under the game; frogs by water.
        (g("rain_loop.wav"), 0.0, -21, {"loop": True, "until": at("catch", 0.05), "fade_in": 2.5, "fade_out": 0.05}),
        (g("rain_loop.wav"), black, -27, {"loop": True, "until": TOTAL, "fade_in": 1.5, "fade_out": 3.0}),
        (g("ambience_llano.wav"), at("gate"), -24, {"loop": True, "until": at("card_lies"), "fade_in": 2.0, "fade_out": 1.0}),
        (g("dread_drone.wav"), at("card_bones"), -22, {"loop": True, "until": at("card_lies"), "fade_in": 3.0, "fade_out": 0.3}),
        (g("frogs_loop.wav"), at("caiman"), -18, {"loop": True, "until": at("ceiba", 0.6), "fade_in": 0.4, "fade_out": 0.6}),
        (g("frogs_loop.wav"), at("cano"), -19, {"loop": True, "until": at("card_dawn"), "fade_in": 0.4, "fade_out": 0.3}),
        # The score: the harp of the llanos.
        (s("m1.mp3"), at("legend", 0.2), -7, {"until": at("card_bones", 0.4), "fade_out": 2.0}),
        (s("m2.mp3"), at("card_bones"), -9, {"until": at("card_lies"), "fade_out": 0.2}),
        # (down under the whispered rule, so every word of it is heard)
        (s("m3.mp3"), at("velo") - 0.2, -8, {"until": catch, "fade_out": 0.05,
                                             "duck": [(at("velo", 0.2), at("hide", 0.3), -9)]}),
        (s("m4.mp3"), at("title") - 0.3, -3, {"until": TOTAL, "fade_out": 3.0}),
        # The whistle: far off in the dark as she asks if you hear it; over
        # his walk; loud on the lie (gone before her whisper); in the hide;
        # once more at the end.
        # The far ones are trailer 1's clean takes (`git show
        # f75473b:assets/audio/whistle_faint_N.wav`): the game's far whistle
        # now comes in wind-torn fragments, which sounds broken out of context.
        (s("clean_whistle_faint_0.wav"), at("cold", 1.5), -8, {}),
        (g("whistle_mid_2.wav"), at("sack", 0.4), -11, {}),
        (g("whistle_loud_3.wav"), at("card_lies", 0.35), -6, {"until": at("velo", 0.3), "fade_out": 0.6}),
        (g("whistle_mid_1.wav"), at("hide", 0.5), -10, {}),
        (s("clean_whistle_faint_3.wav"), at("end", 1.2), -13, {}),
        # Hits on the cards and the title.
        (s("sfx_boom.mp3"), at("legend", 0.1), -14, {}),
        (s("sfx_braam.mp3"), at("card_bones"), -3, {}),
        (s("sfx_braam.mp3"), at("card_road"), -4, {}),
        (s("sfx_braam.mp3"), at("card_friends"), -4, {}),
        (s("sfx_boom.mp3"), at("card_lies"), -3, {}),
        (s("sfx_boom.mp3"), at("title"), -4, {}),
        # The tale's own sounds: Tureco, his stare, his sack, the bones.
        (g("dog_growl.wav"), at("tureco", 0.8), -10, {}),
        (g("dog_bark.wav"), at("tureco", 4.4), -7, {}),
        (g("omen_swell.wav"), at("face"), -9, {}),
        (g("thunder_a.wav"), at("fields", 3.0), -9, {}),
        (g("bones_rattle.wav"), at("sack", 0.6), -13, {}),
        (g("thunder_b.wav"), at("sack", 3.0), -10, {"until": at("card_bones"), "fade_out": 0.4}),
        # La Rabia: the bundle set down and the llano answers; then he counts.
        (g("bones_set.wav"), lay, -6, {}),
        (g("sting_rage.wav"), lay + 0.05, -6, {}),
        (g("counting.wav"), at("count", 0.3), -9, {}),
        (g("bones_rattle.wav"), at("count", 1.0), -12, {}),
        # The road out, the dawn, the caño, a friend down.
        (g("engine_start.wav"), at("truck", 0.3), -10, {"until": at("cano"), "fade_out": 0.3}),
        (g("step_water_0.wav"), at("cano", 0.4), -11, {}),
        (g("step_water_1.wav"), at("cano", 1.2), -12, {}),
        (g("step_water_2.wav"), at("cano", 2.0), -11, {}),
        (g("step_water_0.wav"), at("cano", 2.8), -12, {}),
        (g("dawn.wav"), at("card_dawn") - 0.1, -9, {"until": at("card_friends", 0.6), "fade_out": 0.8}),
        (g("thunder_b.wav"), at("downed", 0.8), -13, {"until": at("card_lies"), "fade_out": 0.5}),
        # The hunt: whooshes on the cuts, a heart, his hunt, lightning, a
        # riser into the silence.
        (s("sfx_whoosh.mp3"), at("hide") - 0.6, -5, {}),
        (s("sfx_whoosh.mp3"), at("chase") - 0.6, -3, {}),
        (s("sfx_whoosh.mp3"), at("reveal") - 0.6, -1, {}),
        (g("heartbeat.wav"), at("hide"), -10, {"loop": True, "until": at("reveal", 1.5), "fade_in": 1.0, "fade_out": 0.3}),
        (g("sting_hunt.wav"), at("chase"), -8, {}),
        (g("sting_reveal.wav"), at("reveal", 0.35), -6, {"until": catch, "fade_out": 0.04}),
        (g("thunder_a.wav"), at("reveal", 0.8), -8, {"until": catch}),
        (s("sfx_riser.mp3"), catch - 5.0, -11, {"until": catch, "fade_out": 0.03}),
        # The catch, at the game's own cues (omen.rs): a breath in the
        # silence, his whistle in your ear, a blow at each flash, his shriek
        # as he bends over you, the slam of the black.
        (g("catch_breath.wav"), at("catch", 0.09), -4, {}),
        (g("whistle_ear.wav"), at("catch", 0.45), -3, {}),
        (g("sting_caught.wav"), at("catch", 1.07), -6, {}),
        (g("catch_hit_0.wav"), at("catch", 1.07), -4, {}),
        (g("catch_hit_1.wav"), at("catch", 1.27), -3, {}),
        (g("catch_hit_2.wav"), at("catch", 1.47), -2, {}),
        (g("catch_hit_3.wav"), at("catch", 1.69), -1, {}),
        (g("catch_shriek.wav"), at("catch", 1.69), -10, {"until": black}),
        (g("catch_slam.wav"), black, 0, {}),
        (g("bones_rattle.wav"), black + 0.2, -6, {}),
        (g("ringing.wav"), black, -10, {}),
    ]


def run(cmd):
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


def alpha(start, fade_in, end, fade_out):
    return (
        f"if(lt(t,{start}),0,if(lt(t,{start + fade_in}),(t-{start})/{fade_in},"
        f"if(lt(t,{end - fade_out}),1,if(lt(t,{end}),({end}-t)/{fade_out},0))))"
    )


def esc(text):
    return text.replace("\\", "\\\\").replace(":", "\\:").replace("'", "’").replace(",", "\\,")


def text_filter(text, font, size, color, dy, start, end, fade=0.5):
    return (
        f"drawtext=fontfile={font}:text='{esc(text)}':fontsize={size}:fontcolor={color}:"
        f"x=(w-text_w)/2:y=(h-text_h)/2+{dy}:alpha='{alpha(start, fade, end, fade)}'"
    )


def build_segment(i, seg, frames_dir, tmp):
    sid, kind, dur, opt = seg
    out = os.path.join(tmp, f"{i:02d}_{sid}.mp4")
    enc = ["-c:v", "libx264", "-preset", "medium", "-crf", "14", "-pix_fmt", "yuv420p", "-r", str(FPS)]
    fades = []
    if opt.get("fade_in"):
        fades.append(f"fade=t=in:st=0:d={opt['fade_in']}")
    if opt.get("fade_out"):
        fades.append(f"fade=t=out:st={dur - opt['fade_out']}:d={opt['fade_out']}")
    if kind == "shot":
        n = opt.get("frames", int(round(dur * FPS)))
        have = len([f for f in os.listdir(os.path.join(frames_dir, opt["dir"])) if f.endswith(".png")])
        if have < n:
            raise SystemExit(f"{opt['dir']}: {have} frames rendered, the cut needs {n}")
        vf = ",".join(["scale=1920:1080"] + fades) or "null"
        run(["ffmpeg", "-y", "-framerate", str(FPS), "-i", os.path.join(frames_dir, opt["dir"], "%05d.png"),
             "-frames:v", str(n), "-vf", vf] + enc + [out])
    else:
        filters = []
        for line in opt.get("lines", []):
            text, font, size, color, dy = line[:5]
            delay = line[5] if len(line) > 5 else 0.25
            filters.append(text_filter(text, font, size, color, dy, delay, dur - 0.2, fade=min(0.5, dur / 4)))
        vf = ",".join(filters + fades) or "null"
        run(["ffmpeg", "-y", "-f", "lavfi", "-i", f"color=c=0x050506:s={W}x{H}:r={FPS}:d={dur}",
             "-vf", vf] + enc + [out])
    return out


VOICE_LEN = {}
# The whispered rule came out about 5 dB under her other lines: lift it so
# it carries over the hunt's music (dB).
VOICE_GAIN = {"n9": 4}


def build_video(frames_dir, tmp):
    parts = [build_segment(i, seg, frames_dir, tmp) for i, seg in enumerate(SEGMENTS)]
    listing = os.path.join(tmp, "parts.txt")
    with open(listing, "w") as f:
        for p in parts:
            f.write(f"file '{os.path.abspath(p)}'\n")
    joined = os.path.join(tmp, "joined.mp4")
    run(["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", listing, "-c", "copy", joined])
    # Subtitles for the narration (not over the title's own words), and a
    # little film grain over everything.
    subs = []
    for stem, sid, off, *texts in VOICE:
        if sid == "title":
            continue
        start = at(sid, off)
        subs.append(text_filter(texts[LANG], ITALIC, 34, "0xE6E0D4", 420, start + 0.1,
                                start + VOICE_LEN[stem] + 0.4, fade=0.3))
    graded = os.path.join(tmp, "graded.mp4")
    run(["ffmpeg", "-y", "-i", joined, "-vf", ",".join(subs + ["noise=alls=3:allf=t"]),
         "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-tune", "grain", "-pix_fmt", "yuv420p", graded])
    return graded


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path],
                         check=True, capture_output=True, text=True).stdout
    return float(out.strip())


def voice_path(stems, stem):
    return os.path.join(stems, ("en", "es")[LANG], f"{stem}.mp3")


def check_voice():
    """Say where a line runs into the next one; nudge offsets, don't guess."""
    lines = sorted((at(sid, off), stem) for stem, sid, off, *_ in VOICE)
    for (a, sa), (b, sb) in zip(lines, lines[1:]):
        end = a + VOICE_LEN[sa]
        if end > b - 0.3:
            print(f"warning: {sa} ends at {end:.2f} s, {sb} starts at {b:.2f} s")


def build_audio(stems, tmp):
    inputs, chains = [], []
    events = [(voice_path(stems, stem), at(sid, off), VOICE_GAIN.get(stem, 0), {}) for stem, sid, off, *_ in VOICE]
    events += sounds(stems)
    for k, (path, start, gain, opt) in enumerate(events):
        if opt.get("loop"):
            inputs += ["-stream_loop", "-1", "-i", path]
        else:
            inputs += ["-i", path]
        chain = [f"[{k}:a]aformat=sample_rates=48000:channel_layouts=stereo"]
        if "until" in opt:
            chain.append(f"atrim=0:{max(0.05, opt['until'] - start)}")
        if opt.get("fade_in"):
            chain.append(f"afade=t=in:st=0:d={opt['fade_in']}")
        if opt.get("fade_out") and "until" in opt:
            length = max(0.05, opt["until"] - start)
            chain.append(f"afade=t=out:st={max(0.0, length - opt['fade_out'])}:d={opt['fade_out']}")
        chain.append(f"volume={gain}dB")
        for a, b, db in opt.get("duck", []):
            # down by `db` between a and b (timeline seconds), ramped over 0.4 s
            a, b, low = a - start, b - start, 10 ** (db / 20)
            ramp = f"clip((t-{a})/0.4,0,1)*clip(({b}-t)/0.4,0,1)"
            chain.append(f"volume='1-{1 - low:.4f}*{ramp}':eval=frame")
        chain.append(f"adelay={int(start * 1000)}|{int(start * 1000)}")
        chains.append(",".join(chain) + f"[a{k}]")
    mix = "".join(f"[a{k}]" for k in range(len(events)))
    graph = ";".join(chains) + f";{mix}amix=inputs={len(events)}:normalize=0:dropout_transition=0,atrim=0:{TOTAL}," \
        "alimiter=limit=0.9:level=false[mixed]"
    raw = os.path.join(tmp, "mix.wav")
    run(["ffmpeg", "-y"] + inputs + ["-filter_complex", graph, "-map", "[mixed]", "-ar", "48000", raw])
    return normalise(raw, tmp)


def normalise(raw, tmp):
    """Two-pass loudnorm (linear, so the mix keeps its dynamics) to -14 LUFS,
    then a limiter under -1.5 dB so the encode keeps its peaks below 0."""
    target = "I=-14:TP=-2:LRA=14"
    probe = subprocess.run(["ffmpeg", "-hide_banner", "-i", raw, "-af", f"loudnorm={target}:print_format=json",
                            "-f", "null", "-"], capture_output=True, text=True).stderr
    m = json.loads(probe[probe.rindex("{"):probe.rindex("}") + 1])
    measured = (f"measured_I={m['input_i']}:measured_TP={m['input_tp']}:measured_LRA={m['input_lra']}:"
                f"measured_thresh={m['input_thresh']}:offset={m['target_offset']}:linear=true")
    final = os.path.join(tmp, "mix_norm.wav")
    run(["ffmpeg", "-y", "-i", raw, "-af",
         f"loudnorm={target}:{measured},aresample=192000,alimiter=limit=0.79:level=false:attack=1:release=50,"
         "aresample=48000", "-ar", "48000", final])
    return final


def main():
    global LANG
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--lang", choices=["en", "es"], required=True)
    p.add_argument("--frames", required=True)
    p.add_argument("--stems", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--keep", help="keep the intermediate files in this folder")
    p.add_argument("--sound-only", action="store_true", help="reuse the kept picture (--keep), remix the sound")
    p.add_argument("--plan", action="store_true", help="print the timeline and voice overlaps, render nothing")
    a = p.parse_args()
    LANG = ("en", "es").index(a.lang)
    lay_out()
    for stem, *_ in VOICE:
        VOICE_LEN[stem] = duration(voice_path(a.stems, stem))
    print(f"timeline {TOTAL:.1f} s")
    check_voice()
    if a.plan:
        for sid, kind, dur, _ in SEGMENTS:
            print(f"{START[sid]:7.2f}  {dur:4.1f}  {kind:5}  {sid}")
        return
    tmp = a.keep or tempfile.mkdtemp(prefix="silbon_trailer3_")
    os.makedirs(tmp, exist_ok=True)
    graded = os.path.join(tmp, "graded.mp4")
    if a.sound_only and os.path.exists(graded):
        video = graded
    else:
        video = build_video(a.frames, tmp)
    print("picture done")
    audio = build_audio(a.stems, tmp)
    print("sound done")
    run(["ffmpeg", "-y", "-i", video, "-i", audio, "-map", "0:v", "-map", "1:a", "-c:v", "copy",
         "-c:a", "aac", "-b:a", "320k", "-shortest", "-movflags", "+faststart", a.out])
    if not a.keep:
        shutil.rmtree(tmp, ignore_errors=True)
    print(f"wrote {a.out}")


if __name__ == "__main__":
    main()

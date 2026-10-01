#!/usr/bin/env python3
"""Cut El Silbón — The Return, trailer 2, in English or Spanish.

Inputs:
  --lang en|es   the narration, cards and subtitles
  --frames DIR   the rendered shots (`el_silbon --trailer --shots X` writes
                 X/trailer/<NN_shot>/00000.png…)
  --stems DIR    the generated sound: DIR/<lang>/vo1…vo9.mp3 (narration),
                 DIR/m1…m4.mp3 (score), DIR/sfx_*.mp3 (hits)
  --out FILE     the finished trailer (.mp4, 1920x1080, 30 fps)

The game's own sounds (whistles, stings, thunder, rain, the radio…) come
from assets/audio. Everything is placed on one timeline of segments, so the
picture and the sound are cut from the same numbers. See trailer/script.md.
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
    "by": ("A GAME BY ANDREW ACEVEDO MIRENA", "UN JUEGO DE ANDREW ACEVEDO MIRENA"),
    "bones": ("FIVE BUNDLES OF BONES", "CINCO ATADOS DE HUESOS"),
    "road": ("ONE ROAD OUT", "UNA SOLA SALIDA"),
    "dawn": ("OR LAST UNTIL DAWN", "O AGUANTA HASTA EL ALBA"),
    "friends": ("UP TO FOUR FRIENDS", "HASTA CUATRO AMIGOS"),
    "behind": ("LEAVE NO ONE IN THE DARK", "NO DEJES A NADIE EN LA OSCURIDAD"),
    "lies": ("THE WHISTLE LIES", "EL SILBIDO MIENTE"),
    "refrain_2": ("If you hear him far away, he is already here.", "Hay silbidos que nunca se deben seguir."),
    "players": ("SOLO OR WITH UP TO FOUR FRIENDS", "SOLO O CON HASTA CUATRO AMIGOS"),
    "langs": ("ENGLISH  ·  ESPAÑOL", "ESPAÑOL  ·  ENGLISH"),
    "dev": ("IN DEVELOPMENT", "EN DESARROLLO"),
}

# Narration: (stem, segment, offset, english, spanish)
VOICE = [
    ("vo1", "gate", 0.3,
     "Out on the llano, when it rains at night… nobody goes outside.",
     "En el llano, cuando llueve de noche… nadie sale."),
    ("vo2", "ceiba", 1.6,
     "They say a boy killed his own father.",
     "Dicen que un muchacho mató a su propio padre."),
    ("vo3", "corral", 2.1,
     "And his grandfather cursed him to carry the bones in a sack… forever.",
     "Y el abuelo lo maldijo a cargar los huesos en un saco… para siempre."),
    ("vo4", "tureco", 0.3,
     "And to whistle, so everyone would know he was coming.",
     "Y a silbar, para que todos sepan que viene."),
    ("vo5", "lay", 0.2,
     "Every bone you bring home… angers him more.",
     "Cada hueso que vuelve a la ceiba… lo enfurece más."),
    ("vo6", "velo", 0.4,
     "Sometimes you won't see him at all. You will only hear him.",
     "A veces no lo vas a ver. Solo lo vas a oír."),
    ("vo7", "hide", 0.5,
     "If you hear him close… he is far away.",
     "Si lo oyes cerca… está lejos."),
    ("vo8", "black", 0.3,
     "But if you hear him far away…",
     "Pero si lo oyes lejos…"),
    ("vo9", "title", 0.5,
     "…he's already here.",
     "…ya está aquí."),
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
        ("intro", "card", 4.0, card((spaced(word("by")), SERIF, 30, "0xC8C0B0", 0))),
        ("gate", "shot", 5.0, {"dir": "01_gate", "fade_in": 1.2, "fade_out": 0.35}),
        ("ceiba", "shot", 4.5, {"dir": "02_ceiba", "fade_in": 0.35, "fade_out": 0.35}),
        ("corral", "shot", 4.0, {"dir": "03_corral", "fade_in": 0.35, "fade_out": 0.35}),
        ("fields", "shot", 4.5, {"dir": "04_fields_flash", "fade_in": 0.35, "fade_out": 0.35}),
        ("tureco", "shot", 4.0, {"dir": "05_tureco", "fade_in": 0.35, "fade_out": 0.5}),
        ("beat", "black", 1.0, {}),
        ("crossing", "shot", 4.0, {"dir": "10_crossing", "fade_in": 0.5}),
        ("card_bones", "card", 1.6, card(big("bones"))),
        ("carry", "shot", 3.5, {"dir": "06_carry"}),
        ("lay", "shot", 4.5, {"dir": "17_lay"}),
        ("radio", "shot", 3.5, {"dir": "18_radio", "fade_in": 0.2}),
        ("power", "shot", 3.0, {"dir": "07_power"}),
        ("cano", "shot", 4.0, {"dir": "19_cano"}),
        ("card_road", "card", 1.6, card(big("road"))),
        ("truck", "shot", 3.5, {"dir": "08_truck"}),
        ("card_dawn", "card", 1.8, card(big("dawn", GOLD))),
        ("card_friends", "card", 1.6, card(big("friends"))),
        ("party", "shot", 3.5, {"dir": "16_party"}),
        ("friends", "shot", 4.5, {"dir": "09_friends"}),
        ("downed", "shot", 4.0, {"dir": "20_downed"}),
        ("card_behind", "card", 1.8, card(big("behind", size=44))),
        ("card_lies", "card", 2.0, card(big("lies", RED, 56))),
        ("velo", "shot", 5.0, {"dir": "21_velo"}),
        ("hat", "shot", 3.0, {"dir": "11_hat"}),
        ("hide", "shot", 4.5, {"dir": "12_hide"}),
        ("chase", "shot", 3.5, {"dir": "13_chase"}),
        ("reveal", "shot", 2.2, {"dir": "14_reveal"}),
        # Cut on the catch's black (s = 1.75 of the catch clock).
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
    lay = at("lay", 2.3)
    return [
        # Beds: rain the whole night, the insects until the hunt, the dread
        # under the middle; all gone in the catch's silence.
        (g("rain_loop.wav"), 0.0, -20, {"loop": True, "until": catch, "fade_in": 3.0, "fade_out": 0.05}),
        (g("rain_loop.wav"), at("black"), -26, {"loop": True, "until": TOTAL, "fade_in": 1.5, "fade_out": 3.0}),
        (g("ambience_llano.wav"), at("gate"), -24, {"loop": True, "until": at("card_lies"), "fade_in": 2.0, "fade_out": 1.0}),
        (g("dread_drone.wav"), at("crossing"), -22, {"loop": True, "until": at("card_lies"), "fade_in": 3.0, "fade_out": 0.3}),
        (g("frogs_loop.wav"), at("cano"), -18, {"loop": True, "until": at("card_road"), "fade_in": 0.4, "fade_out": 0.3}),
        # Score.
        (s("m1.mp3"), 0.0, 2, {"until": at("crossing", 3.6), "fade_out": 2.0}),
        (s("m2.mp3"), at("card_bones"), -9, {"until": at("card_lies"), "fade_out": 0.15}),
        (s("m3.mp3"), at("velo") - 0.2, -6, {"until": catch, "fade_out": 0.04}),
        (s("m4.mp3"), at("title") - 0.3, -2, {"until": TOTAL, "fade_out": 3.0}),
        # The whistle: faint in the dark before anything, one across the
        # yard at the truck, loud on the lie, and on through the veil.
        (g("whistle_faint_0.wav"), 1.8, -12, {}),
        (g("whistle_mid_2.wav"), at("truck", 0.8), -10, {}),
        (g("whistle_loud_3.wav"), at("card_lies", 0.35), -6, {}),
        (g("whistle_mid_1.wav"), at("velo", 2.9), -7, {}),
        # Hits on the cards and the title.
        (s("sfx_braam.mp3"), at("card_bones"), -2, {}),
        (s("sfx_braam.mp3"), at("card_road"), -3, {}),
        (s("sfx_braam.mp3"), at("card_friends"), -3, {}),
        (s("sfx_boom.mp3"), at("card_lies"), -3, {}),
        (s("sfx_boom.mp3"), at("title"), -4, {}),
        # La Rabia: the bundle set down, the llano answers.
        (g("bones_set.wav"), lay, -6, {}),
        (g("sting_rage.wav"), lay + 0.05, -6, {}),
        # The radio: a squeal on the dial, its ident, the numbers' pips.
        (g("radio_static.wav"), at("radio"), -14, {"until": at("power"), "fade_out": 0.2}),
        (g("radio_squeal.wav"), at("radio", 0.1), -12, {}),
        (g("radio_ident.wav"), at("radio", 0.9), -9, {}),
        (g("radio_pip.wav"), at("radio", 2.6), -8, {}),
        (g("radio_pip.wav"), at("radio", 2.85), -8, {}),
        (g("radio_pip_long.wav"), at("radio", 3.1), -9, {"until": at("power", 0.4)}),
        # The windmill's lamps.
        (g("power_on.wav"), at("power", 0.2), -10, {}),
        # The caño.
        (g("step_water_0.wav"), at("cano", 0.4), -10, {}),
        (g("step_water_1.wav"), at("cano", 1.1), -11, {}),
        (g("step_water_2.wav"), at("cano", 1.9), -10, {}),
        (g("step_water_0.wav"), at("cano", 2.6), -11, {}),
        (g("step_water_1.wav"), at("cano", 3.3), -10, {}),
        # The truck turns over; the rooster's dawn under its card.
        (g("engine_start.wav"), at("truck", 0.3), -10, {"until": at("card_dawn"), "fade_out": 0.3}),
        (g("dawn.wav"), at("card_dawn") - 0.1, -9, {"until": at("card_friends", 0.6), "fade_out": 0.8}),
        # A friend down: the groan, the cry for help.
        (g("downed_groan_1.wav"), at("downed", 0.5), -8, {}),
        (g("call_help.wav"), at("downed", 2.0), -9, {}),
        # Lightning: thunder after each strike, a sting when it shows him.
        (g("thunder_a.wav"), at("fields", 3.4), -8, {}),
        (g("sting_reveal.wav"), at("friends", 3.6), -10, {}),
        (g("thunder_b.wav"), at("friends", 4.1), -10, {"until": at("card_behind"), "fade_out": 0.5}),
        (g("sting_reveal.wav"), at("reveal", 0.35), -6, {"until": catch, "fade_out": 0.04}),
        (g("thunder_a.wav"), at("reveal", 0.8), -8, {"until": catch}),
        # The hunt: whooshes on the cuts, his hunt, a heart, a riser into
        # the silence.
        (s("sfx_whoosh.mp3"), at("hide") - 0.6, -5, {}),
        (s("sfx_whoosh.mp3"), at("chase") - 0.6, -3, {}),
        (s("sfx_whoosh.mp3"), at("reveal") - 0.6, -1, {}),
        (g("heartbeat.wav"), at("hide"), -10, {"loop": True, "until": at("reveal", 1.5), "fade_in": 1.0, "fade_out": 0.3}),
        (g("sting_hunt.wav"), at("chase"), -8, {}),
        (s("sfx_riser.mp3"), catch - 5.0, -11, {"until": catch, "fade_out": 0.03}),
        # The catch: silence, his whistle in your ear, him, the black.
        (g("whistle_ear.wav"), at("catch", 0.45), -2, {}),
        (g("sting_caught.wav"), at("catch", 1.07), -2, {}),
        (g("bones_rattle.wav"), at("black", 0.2), -6, {}),
        (g("ringing.wav"), at("black"), -10, {}),
        # One more, far away, as the title fades.
        (g("whistle_faint_3.wav"), at("end", 1.2), -14, {}),
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
    events = [(voice_path(stems, stem), at(sid, off), 0, {}) for stem, sid, off, *_ in VOICE]
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
    tmp = a.keep or tempfile.mkdtemp(prefix="silbon_trailer2_")
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

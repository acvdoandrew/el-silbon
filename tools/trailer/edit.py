#!/usr/bin/env python3
"""Cut the El Silbón teaser.

Inputs:
  --frames DIR   the rendered shots (`el_silbon --trailer --shots X` writes
                 X/trailer/<NN_shot>/00000.png…)
  --stems DIR    the generated narration, music and hits (vo1…vo5, m1…m4,
                 sfx_braam, sfx_boom, sfx_riser, sfx_whoosh as .mp3)
  --out FILE     the finished trailer (.mp4, 1920x1080, 30 fps)

The game's own sounds (whistles, stings, thunder, rain, ringing…) come from
assets/audio. Everything is placed on one timeline of segments below, so
the picture and the sound are cut from the same numbers.
"""

import argparse
import os
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
AUDIO = os.path.join(ROOT, "assets", "audio")
SERIF = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Regular.ttf")
ITALIC = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Italic.ttf")
FPS = 30
W, H = 1920, 1080


def spaced(text):
    """Wide-tracked capitals, the trailer's card style."""
    return "  ".join(" ".join(word) for word in text.split(" "))


# ---------------------------------------------------------------------------
# The picture: (id, kind, seconds, options)
# ---------------------------------------------------------------------------

SEGMENTS = [
    ("intro", "card", 5.0, {"lines": [
        (spaced("A GAME BY ANDREW ACEVEDO MIRENA"), SERIF, 24, "0xC8C0B0", -70),
        (spaced("MADE WITH CLAUDE CODE"), SERIF, 44, "0xEDE6D8", 0),
        ("Every mesh, texture and sound in the game is generated in code.", ITALIC, 24, "0x9A9488", 70),
    ]}),
    ("gate", "shot", 5.0, {"dir": "01_gate", "fade_in": 1.2, "fade_out": 0.35}),
    ("ceiba", "shot", 4.5, {"dir": "02_ceiba", "fade_in": 0.35, "fade_out": 0.35}),
    ("corral", "shot", 4.0, {"dir": "03_corral", "fade_in": 0.35, "fade_out": 0.35}),
    ("fields", "shot", 4.5, {"dir": "04_fields_flash", "fade_in": 0.35, "fade_out": 0.35}),
    ("tureco", "shot", 4.0, {"dir": "05_tureco", "fade_in": 0.35, "fade_out": 0.5}),
    ("beat", "black", 1.2, {}),
    ("crossing", "shot", 4.5, {"dir": "10_crossing", "fade_in": 0.5}),
    ("card_bones", "card", 1.6, {"lines": [(spaced("FIVE BUNDLES OF BONES"), SERIF, 50, "0xEDE6D8", 0)]}),
    ("carry", "shot", 4.0, {"dir": "06_carry"}),
    ("card_road", "card", 1.6, {"lines": [(spaced("ONE ROAD OUT"), SERIF, 50, "0xEDE6D8", 0)]}),
    ("truck", "shot", 4.0, {"dir": "08_truck"}),
    ("card_friends", "card", 1.6, {"lines": [(spaced("UP TO FOUR FRIENDS"), SERIF, 50, "0xEDE6D8", 0)]}),
    ("party", "shot", 3.5, {"dir": "16_party"}),
    ("friends", "shot", 5.0, {"dir": "09_friends"}),
    ("card_lies", "card", 2.0, {"lines": [(spaced("THE WHISTLE LIES"), SERIF, 56, "0xB51A1A", 0)]}),
    ("hat", "shot", 3.0, {"dir": "11_hat"}),
    ("hide", "shot", 4.5, {"dir": "12_hide"}),
    ("chase", "shot", 3.5, {"dir": "13_chase"}),
    ("reveal", "shot", 2.2, {"dir": "14_reveal"}),
    # Cut on the catch's black (s = 1.75 of the catch clock).
    ("catch", "shot", 2.8, {"dir": "15_catch", "frames": 84}),
    ("black", "black", 2.4, {}),
    ("title", "card", 10.0, {"fade_in": 0.2, "fade_out": 1.0, "lines": [
        (spaced("EL SILBÓN"), SERIF, 136, "0xEDE6D8", -90),
        (spaced("THE RETURN"), SERIF, 36, "0xC8A060", 30),
        ("Si lo oyes lejos, ya está aquí.", ITALIC, 36, "0xD8D0C0", 150, 3.0),
        ("If you hear him far away, he is already here.", ITALIC, 24, "0x8A857A", 200, 3.6),
    ]}),
    ("end", "card", 5.0, {"fade_in": 0.6, "fade_out": 1.2, "lines": [
        (spaced("SOLO  OR  WITH  UP  TO  FOUR  FRIENDS"), SERIF, 30, "0xD8D0C0", -30),
        (spaced("IN DEVELOPMENT"), SERIF, 24, "0xC8A060", 30),
        ("Trailer narration, music and sound design generated with ElevenLabs.", ITALIC, 20, "0x6E6A62", 170),
    ]}),
]


def starts():
    t, out = 0.0, {}
    for sid, _, dur, _ in SEGMENTS:
        out[sid] = t
        t += dur
    return out, t


START, TOTAL = starts()


def at(sid, offset=0.0):
    return START[sid] + offset


# Narration: (stem, segment, offset, subtitle)
VOICE = [
    ("vo1", "gate", 0.5, "Out on the llano… when it rains at night… nobody goes outside."),
    ("vo2", "ceiba", 1.8, "They say he killed… his own father."),
    ("vo3", "corral", 1.6, "And his grandfather cursed him… to carry the bones. Forever."),
    ("vo4", "tureco", 0.8, "If you hear him close… he is far away."),
    ("vo5", "beat", 0.0, "But if you hear him far away… he's already here."),
]

# Everything else heard: (file, start seconds, gain dB, options)
def sounds(stems):
    g = lambda name: os.path.join(AUDIO, name)
    s = lambda name: os.path.join(stems, name)
    catch = at("catch")
    out = [
        # Beds: rain the whole night, the insects until the chase, the dread
        # under the middle; all gone in the catch's silence.
        (g("rain_loop.wav"), 0.0, -20, {"loop": True, "until": catch, "fade_in": 3.0, "fade_out": 0.05}),
        (g("rain_loop.wav"), at("black"), -26, {"loop": True, "until": TOTAL, "fade_in": 1.5, "fade_out": 3.0}),
        (g("ambience_llano.wav"), at("gate"), -24, {"loop": True, "until": at("hat"), "fade_in": 2.0, "fade_out": 1.0}),
        (g("dread_drone.wav"), at("crossing"), -20, {"loop": True, "until": at("card_lies"), "fade_in": 3.0, "fade_out": 0.3}),
        # Score (ElevenLabs).
        (s("m1.mp3"), 0.0, -8, {"until": at("crossing", 3.8), "fade_out": 2.0}),
        (s("m2.mp3"), at("card_bones"), -8, {"until": at("card_lies"), "fade_out": 0.15}),
        (s("m3.mp3"), at("hat") - 0.2, -7, {"until": catch, "fade_out": 0.04}),
        (s("m4.mp3"), at("title") - 0.3, -2, {"until": TOTAL, "fade_out": 3.0}),
        # The whistle: faint in the dark before anything, loud on the lie.
        (g("whistle_faint_0.wav"), 2.4, -12, {}),
        (g("whistle_mid_2.wav"), at("truck", 1.2), -8, {}),
        (g("whistle_loud_3.wav"), at("card_lies", 0.35), -6, {}),
        # Hits on the cards and the title.
        (s("sfx_braam.mp3"), at("card_bones"), -6, {}),
        (s("sfx_braam.mp3"), at("card_road"), -6, {}),
        (s("sfx_braam.mp3"), at("card_friends"), -6, {}),
        (s("sfx_boom.mp3"), at("card_lies"), -3, {}),
        (s("sfx_boom.mp3"), at("title"), -4, {}),
        # Lightning: thunder after each strike, a sting when it shows him.
        (g("thunder_a.wav"), at("fields", 3.4), -8, {}),
        (g("sting_reveal.wav"), at("friends", 3.6), -10, {}),
        (g("thunder_b.wav"), at("friends", 4.1), -10, {}),
        (g("sting_reveal.wav"), at("reveal", 0.35), -6, {"until": catch, "fade_out": 0.04}),
        (g("thunder_a.wav"), at("reveal", 0.8), -8, {"until": catch}),
        # The chase: whooshes on the cuts, his hunt, a heart, a riser into
        # the silence.
        (s("sfx_whoosh.mp3"), at("hide") - 0.6, -10, {}),
        (s("sfx_whoosh.mp3"), at("chase") - 0.6, -8, {}),
        (s("sfx_whoosh.mp3"), at("reveal") - 0.6, -6, {}),
        (g("heartbeat.wav"), at("hide"), -10, {"loop": True, "until": at("reveal", 1.5), "fade_in": 1.0, "fade_out": 0.3}),
        (g("sting_hunt.wav"), at("chase"), -8, {}),
        (s("sfx_riser.mp3"), catch - 5.0, -8, {"until": catch, "fade_out": 0.03}),
        # The catch: silence, his whistle in your ear, him, the black.
        (g("whistle_ear.wav"), at("catch", 0.45), -2, {}),
        (g("sting_caught.wav"), at("catch", 1.07), -2, {}),
        (g("bones_rattle.wav"), at("black"), -4, {}),
        (g("ringing.wav"), at("black"), -8, {}),
        # One more, far away, as the title fades.
        (g("whistle_faint_3.wav"), at("end", 1.5), -14, {}),
    ]
    return out


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
        vf = ",".join(["scale=1920:1080"] + fades) or "null"
        run(["ffmpeg", "-y", "-framerate", str(FPS), "-i", os.path.join(frames_dir, opt["dir"], "%05d.png"),
             "-frames:v", str(n), "-vf", vf] + enc + [out])
    else:
        filters = []
        for line in opt.get("lines", []):
            text, font, size, color, dy = line[:5]
            delay = line[5] if len(line) > 5 else 0.3
            filters.append(text_filter(text, font, size, color, dy, delay, dur - 0.25, fade=0.6))
        vf = ",".join(filters + fades) or "null"
        run(["ffmpeg", "-y", "-f", "lavfi", "-i", f"color=c=0x050506:s={W}x{H}:r={FPS}:d={dur}",
             "-vf", vf] + enc + [out])
    return out


def build_video(frames_dir, tmp):
    parts = [build_segment(i, seg, frames_dir, tmp) for i, seg in enumerate(SEGMENTS)]
    listing = os.path.join(tmp, "parts.txt")
    with open(listing, "w") as f:
        for p in parts:
            f.write(f"file '{p}'\n")
    joined = os.path.join(tmp, "joined.mp4")
    run(["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", listing, "-c", "copy", joined])
    # Subtitles for the narration, and a little film grain over everything.
    subs = []
    for stem, sid, off, text in VOICE:
        start = at(sid, off)
        subs.append(text_filter(text, ITALIC, 34, "0xE6E0D4", 420, start + 0.1, start + VOICE_LEN[stem] + 0.4, fade=0.35))
    graded = os.path.join(tmp, "graded.mp4")
    run(["ffmpeg", "-y", "-i", joined, "-vf", ",".join(subs + ["noise=alls=3:allf=t"]),
         "-c:v", "libx264", "-preset", "slow", "-crf", "20", "-tune", "grain", "-pix_fmt", "yuv420p", graded])
    return graded


VOICE_LEN = {}


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path],
                         check=True, capture_output=True, text=True).stdout
    return float(out.strip())


def build_audio(stems, tmp):
    inputs, chains = [], []
    events = [(os.path.join(stems, f"{stem}.mp3"), at(sid, off), -1, {}) for stem, sid, off, _ in VOICE]
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
    final = os.path.join(tmp, "mix_norm.wav")
    run(["ffmpeg", "-y", "-i", raw, "-af", "loudnorm=I=-14:TP=-1.5:LRA=14", "-ar", "48000", final])
    return final


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--frames", required=True)
    p.add_argument("--stems", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--keep", help="keep the intermediate files in this folder")
    a = p.parse_args()
    for stem, _, _, _ in VOICE:
        VOICE_LEN[stem] = duration(os.path.join(a.stems, f"{stem}.mp3"))
    tmp = a.keep or tempfile.mkdtemp(prefix="silbon_trailer_")
    os.makedirs(tmp, exist_ok=True)
    print(f"timeline {TOTAL:.1f} s; working in {tmp}")
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

"""The machinery the trailer cuts share: a timeline of segments (rendered
shots, title cards, black), timed text over the picture, and one sound mix,
all cut from the same numbers. A cut script (`cut4.py`) describes the
segments, the words and the sounds; this module renders and mixes them.

Picture: each segment is encoded on its own, then joined and graded once
(text, letterbox, static bursts, film grain). Sound: every event is placed
on the timeline with `adelay`, mixed, then two-pass loudnorm to -14 LUFS
with the peaks held under -1.5 dBTP.
"""

import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, ".."))
AUDIO = os.path.join(ROOT, "assets", "audio")
SERIF = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Regular.ttf")
ITALIC = os.path.join(ROOT, "assets", "fonts", "NotoSerif-Italic.ttf")
SANS = os.path.join(ROOT, "assets", "fonts", "NotoSans-Regular.ttf")
FPS = 30

BONE = "0xEDE6D8"
GOLD = "0xC8A060"
RED = "0xB51A1A"
DIM = "0x8A857A"
PALE = "0xD8D0C0"

# An AM radio in a dark room: the band narrowed, squeezed and a little
# driven, with the box's own small echo.
RADIO_FX = [
    "highpass=f=320",
    "lowpass=f=3200",
    "acompressor=threshold=-22dB:ratio=5:attack=4:release=90:makeup=4",
    "asoftclip=type=atan:param=1.6",
    "aecho=0.85:0.4:9|17:0.22|0.12",
]


def spaced(text):
    """Wide-tracked capitals, the cards' style."""
    return "  ".join(" ".join(word) for word in text.split(" "))


def run(cmd):
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path],
                         check=True, capture_output=True, text=True).stdout
    return float(out.strip())


def alpha(start, fade_in, end, fade_out):
    return (
        f"if(lt(t,{start}),0,if(lt(t,{start + fade_in}),(t-{start})/{fade_in},"
        f"if(lt(t,{end - fade_out}),1,if(lt(t,{end}),({end}-t)/{fade_out},0))))"
    )


def esc(text):
    return (text.replace("\\", "\\\\").replace(":", "\\:").replace("'", "’")
            .replace(",", "\\,").replace("%", "\\%"))


def text_filter(text, font, size, color, dy, start, end, fade=0.5, box=False, x=None):
    """Centred text, `dy` pixels below the middle, faded in and out."""
    border = ":borderw=2:bordercolor=0x000000@0.55" if box else ""
    xs = x if x is not None else "(w-text_w)/2"
    return (
        f"drawtext=fontfile={font}:text='{esc(text)}':fontsize={size}:fontcolor={color}{border}:"
        f"x={xs}:y=(h-text_h)/2+{dy}:alpha='{alpha(start, fade, end, fade)}'"
    )


class Cut:
    """One trailer: segments laid end to end, text over them, one mix."""

    def __init__(self, segments, vertical=False):
        self.segments = segments
        self.vertical = vertical
        self.w, self.h = (1080, 1920) if vertical else (1920, 1080)
        self.start = {}
        t = 0.0
        for sid, _, dur, _ in segments:
            if sid in self.start:
                raise SystemExit(f"segment {sid} twice")
            self.start[sid] = t
            t += dur
        self.total = t

    def at(self, sid, offset=0.0):
        return self.start[sid] + offset

    def end(self, sid):
        for s, _, dur, _ in self.segments:
            if s == sid:
                return self.start[sid] + dur
        raise KeyError(sid)

    def plan(self, frames_dir=None):
        for sid, kind, dur, opt in self.segments:
            what = opt.get("dir", "")
            print(f"{self.start[sid]:7.2f}  {dur:4.1f}  {kind:6}  {sid:14} {what}")
            if frames_dir and kind == "shot":
                folder = os.path.join(frames_dir, what)
                have = len([f for f in os.listdir(folder) if f.endswith(".png")]) if os.path.isdir(folder) else 0
                need = opt.get("from", 0) + int(round(dur * FPS))
                if have < need:
                    print(f"warning: {sid} needs {need} frames of {what}, {have} rendered")
        print(f"total {self.total:.1f} s")

    # ---------------------------------------------------------------- picture

    def build_segment(self, i, seg, frames_dir, tmp):
        sid, kind, dur, opt = seg
        out = os.path.join(tmp, f"{i:02d}_{sid}.mp4")
        enc = ["-c:v", "libx264", "-preset", "medium", "-crf", "14", "-pix_fmt", "yuv420p", "-r", str(FPS)]
        fades = []
        if opt.get("fade_in"):
            fades.append(f"fade=t=in:st=0:d={opt['fade_in']}")
        if opt.get("fade_out"):
            fades.append(f"fade=t=out:st={dur - opt['fade_out']:.3f}:d={opt['fade_out']}")
        if kind == "shot":
            n = int(round(dur * FPS))
            first = opt.get("from", 0)
            folder = os.path.join(frames_dir, opt["dir"])
            have = len([f for f in os.listdir(folder) if f.endswith(".png")])
            if have < first + n:
                raise SystemExit(f"{opt['dir']}: {have} frames rendered, the cut needs {first + n}")
            vf = [f"scale={self.w}:{self.h}"]
            if opt.get("slow"):
                # hold each frame `slow` times (a slow-motion beat)
                vf.append(f"setpts={opt['slow']}*PTS")
            vf += fades
            run(["ffmpeg", "-y", "-framerate", str(FPS), "-start_number", str(first),
                 "-i", os.path.join(folder, "%05d.png"), "-vf", ",".join(vf), "-frames:v", str(n)]
                + enc + [out])
        elif kind == "freeze":
            # One rendered frame held (a strike's lit instant), the lens
            # pushing slowly in on `focus` (x, y as fractions of a 16:9 frame).
            path = os.path.join(frames_dir, opt["dir"], f"{opt['frame']:05d}.png")
            n = int(round(dur * FPS))
            cx, cy = opt["focus"]
            if self.vertical:
                # same vertical field of view, so the offset grows 1920/1080
                # in pixels and is measured against a 1080-wide frame
                cx = 0.5 + (cx - 0.5) * (1920 / 1080) ** 2
            z = opt.get("zoom", 2.0)
            ease = f"(1-pow(1-on/{n},2))"
            zoom = f"1+{z - 1}*{ease}"
            vf = [f"scale={self.w * 4}:{self.h * 4}",
                  f"zoompan=z='{zoom}':x='max(0,min(iw-iw/zoom,{cx}*iw-iw/zoom/2))':"
                  f"y='max(0,min(ih-ih/zoom,{cy}*ih-ih/zoom/2))':d={n}:s={self.w}x{self.h}:fps={FPS}"]
            if opt.get("dim"):
                vf.append(f"eq=brightness=-{opt['dim']}*t/{dur}:eval=frame")
            vf += fades
            run(["ffmpeg", "-y", "-loop", "1", "-i", path, "-vf", ",".join(vf), "-frames:v", str(n)] + enc + [out])
        else:
            filters = []
            for line in opt.get("lines", []):
                text, font, size, color, dy = line[:5]
                delay = line[5] if len(line) > 5 else 0.25
                until = line[6] if len(line) > 6 else dur - 0.2
                filters.append(text_filter(text, font, self.scale(size), color, self.scale(dy), delay, until,
                                           fade=min(0.5, dur / 4)))
            vf = ",".join(filters + fades) or "null"
            run(["ffmpeg", "-y", "-f", "lavfi", "-i", f"color=c=0x050506:s={self.w}x{self.h}:r={FPS}:d={dur}",
                 "-vf", vf] + enc + [out])
        return out

    def scale(self, px):
        """Type and offsets are set for 1920 wide; a phone frame is 1080."""
        return int(round(px * (0.75 if self.vertical else 1.0)))

    def build_video(self, frames_dir, tmp, overlays=(), letterbox=0, static=()):
        """`overlays`: (text, font, size, color, dy, start, end, fade) in
        timeline seconds; `letterbox`: bar height in px; `static`: (start,
        end) windows of a burst of picture noise."""
        parts = [self.build_segment(i, seg, frames_dir, tmp) for i, seg in enumerate(self.segments)]
        listing = os.path.join(tmp, "parts.txt")
        with open(listing, "w") as f:
            for p in parts:
                f.write(f"file '{os.path.abspath(p)}'\n")
        joined = os.path.join(tmp, "joined.mp4")
        run(["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", listing, "-c", "copy", joined])
        vf = []
        for a, b in static:
            vf.append(f"noise=alls=70:allf=t+u:enable='between(t,{a},{b})'")
            vf.append(f"eq=brightness=0.06:enable='between(t,{a},{b})'")
        if letterbox:
            vf.append(f"drawbox=x=0:y=0:w=iw:h={letterbox}:color=black:t=fill")
            vf.append(f"drawbox=x=0:y=ih-{letterbox}:w=iw:h={letterbox}:color=black:t=fill")
        for text, font, size, color, dy, start, end, *rest in overlays:
            fade = rest[0] if rest else 0.3
            vf.append(text_filter(text, font, self.scale(size), color, self.scale(dy), start, end,
                                  fade=fade, box=True))
        vf.append("noise=alls=3:allf=t")
        graded = os.path.join(tmp, "graded.mp4")
        run(["ffmpeg", "-y", "-i", joined, "-vf", ",".join(vf), "-c:v", "libx264", "-preset", "slow",
             "-crf", "18", "-tune", "grain", "-pix_fmt", "yuv420p", graded])
        return graded

    # ------------------------------------------------------------------ sound

    def build_audio(self, events, tmp):
        """`events`: (file, start s, gain dB, options). Options: loop,
        until (timeline s), fade_in, fade_out, duck [(a, b, dB)], fx [filters],
        trim_from (s into the file)."""
        inputs, chains = [], []
        for k, (path, start, gain, opt) in enumerate(events):
            if not os.path.exists(path):
                raise SystemExit(f"missing sound {path}")
            if opt.get("loop"):
                inputs += ["-stream_loop", "-1", "-i", path]
            else:
                inputs += ["-i", path]
            chain = [f"[{k}:a]aformat=sample_rates=48000:channel_layouts=stereo"]
            if opt.get("trim_from"):
                chain.append(f"atrim=start={opt['trim_from']},asetpts=PTS-STARTPTS")
            chain += opt.get("fx", [])
            if "until" in opt:
                chain.append(f"atrim=0:{max(0.05, opt['until'] - start):.3f}")
            if opt.get("fade_in"):
                chain.append(f"afade=t=in:st=0:d={opt['fade_in']}")
            if opt.get("fade_out") and "until" in opt:
                length = max(0.05, opt["until"] - start)
                chain.append(f"afade=t=out:st={max(0.0, length - opt['fade_out']):.3f}:d={opt['fade_out']}")
            chain.append(f"volume={gain}dB")
            for a, b, db in opt.get("duck", []):
                a, b, low = a - start, b - start, 10 ** (db / 20)
                ramp = f"clip((t-{a})/0.4,0,1)*clip(({b}-t)/0.4,0,1)"
                chain.append(f"volume='1-{1 - low:.4f}*{ramp}':eval=frame")
            ms = int(round(start * 1000))
            chain.append(f"adelay={ms}|{ms}")
            chains.append(",".join(chain) + f"[a{k}]")
        mix = "".join(f"[a{k}]" for k in range(len(events)))
        graph = (";".join(chains) + f";{mix}amix=inputs={len(events)}:normalize=0:dropout_transition=0,"
                 f"atrim=0:{self.total},alimiter=limit=0.9:level=false[mixed]")
        script = os.path.join(tmp, "mix.graph")
        with open(script, "w") as f:
            f.write(graph)
        raw = os.path.join(tmp, "mix.wav")
        run(["ffmpeg", "-y"] + inputs + ["-/filter_complex", script, "-map", "[mixed]", "-ar", "48000", raw])
        return normalise(raw, tmp)

    def mux(self, video, audio, out):
        run(["ffmpeg", "-y", "-i", video, "-i", audio, "-map", "0:v", "-map", "1:a", "-c:v", "copy",
             "-c:a", "aac", "-b:a", "320k", "-shortest", "-movflags", "+faststart", out])


def normalise(raw, tmp):
    """Two-pass linear loudnorm to -14 LUFS, then a true-peak-safe limiter."""
    target = "I=-14:TP=-2:LRA=16"
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

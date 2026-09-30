#!/usr/bin/env python3
"""Whistle lab: hear the Silbón's whistle exactly as the game mixes it,
without building or playing the game.

Every level and threshold is read from `src/tuning.rs` (the three variant
gains, the rain and ambience beds and the duck under a whistle, the
distance-to-variant inversion, the stalking cadence) and the volume curve
from `src/mix.rs` (a slider even in decibels, the mix's headroom, the
default master and the loops' glide), and the sounds are the game's own
files in `assets/audio/`. Edit the gains in tuning.rs, or the sound design
in `tools/gen_audio.py` (then pass --regen), and listen again.

    python3 tools/whistle_lab.py                # the three distances, one take
    python3 tools/whistle_lab.py ladder --all-takes
    python3 tools/whistle_lab.py ab             # loud and faint back to back
    python3 tools/whistle_lab.py approach       # he walks in from 60 m to 3 m
    python3 tools/whistle_lab.py night          # two minutes of him stalking
    python3 tools/whistle_lab.py night --seconds 300 --seed 9
    python3 tools/whistle_lab.py measure --check          # the shipped files
    python3 tools/whistle_lab.py measure --check --synth  # the synthesized fallback

Common options: --regen (rebuild the whistle files first, ~3 s), --dry (no
rain or insects), --tense (the insects hushed, as while he warns or hunts),
--master M (the master slider, 0..1, default the game's; --volume is the
same), --ambience A (the Ambience slider, under the bed only: his whistle
follows the master alone), --rain R (0.6 drizzle .. 1.0 downpour), --out
FILE (write a WAV instead of playing it). The sliders are read the game's
way: 0 is silence, 1 is 0 dB, and the whole mix sits under full scale, so
the default master plays everything about 12 dB down.

The `approach` and `night` modes mirror `perception.rs` (the inversion and
`CueDirector::stalk_gap`) in a few lines of Python; the Rust tests cover the
game's own behaviour.

`measure` plays nothing: it prints a few features of each of the twelve
files (air, brightness, slap, ring, gaps, level) and, with --check, fails
unless every pair of distances is told apart by clear margins, at the
game's slowest, own and fastest playback speed, each distance's four takes
staying closer together than the distances are to each other, and unless
every file is equally loud (the game's gains being the only level
difference). --synth measures the synthesized fallback (built into a
temporary directory), --dir any other set of twelve files.
"""

import argparse
import cmath
import math
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import time
import wave
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
AUDIO = os.path.join(ROOT, "assets", "audio")
SR = 44100
VARIANTS = ("loud", "mid", "faint")
SEEMS = {
    "loud": "LOUD — as if right beside you",
    "mid": "middling — somewhere across the grass",
    "faint": "faint — far, far away",
}


# --------------------------------------------------------------------------
# The game's numbers
# --------------------------------------------------------------------------

def tuning():
    """The default Tuning, read from src/tuning.rs."""
    text = open(os.path.join(ROOT, "src", "tuning.rs"), encoding="utf-8").read()
    body = text[text.index("impl Default for Tuning") :]
    num = r"(-?[0-9]+(?:\.[0-9]+)?)"
    out = {}
    for name, a, b in re.findall(rf"^\s*(\w+): \({num}, {num}\),", body, re.M):
        out.setdefault(name, (float(a), float(b)))
    for name, a in re.findall(rf"^\s*(\w+): {num},", body, re.M):
        out.setdefault(name, float(a))
    need = [
        "gain_loud", "gain_mid", "gain_faint", "ambience_gain", "rain_gain", "whistle_duck",
        "ambience_hush", "cue_near_distance", "cue_far_distance", "cue_loud_above", "cue_mid_above",
        "stalk_phrase_interval", "stalk_answer_chance", "stalk_answer", "stalk_silence_chance",
        "stalk_silence", "whistle_speed",
    ]
    missing = [n for n in need if n not in out]
    if missing:
        sys.exit(f"whistle_lab: could not read {missing} from src/tuning.rs")
    return out


def mix_numbers():
    """The volume curve's constants, read from src/mix.rs."""
    text = open(os.path.join(ROOT, "src", "mix.rs"), encoding="utf-8").read()
    out = {name: float(v) for name, v in re.findall(r"^pub const (\w+): f32 = (-?[0-9]+(?:\.[0-9]+)?);", text, re.M)}
    missing = [n for n in ("SLIDER_FLOOR_DB", "MIX_HEADROOM_DB", "DEFAULT_MASTER", "GLIDE_RATE") if n not in out]
    if missing:
        sys.exit(f"whistle_lab: could not read {missing} from src/mix.rs")
    return out


def from_db(db):
    return 10.0 ** (db / 20.0)


def slider_gain(pos, m):
    """A slider's position as a linear gain: even in dB, silent at 0 (mix::slider_gain)."""
    if pos <= 0.0:
        return 0.0
    return from_db(m["SLIDER_FLOOR_DB"] * (1.0 - min(pos, 1.0)))


def bus_gain(master, bus, m):
    """A voice's gain on a bus under the master (mix::gain); his whistle,
    on the master alone, passes `bus` 1."""
    return from_db(m["MIX_HEADROOM_DB"]) * slider_gain(master, m) * slider_gain(bus, m)


def db_text(g):
    return f"{20.0 * math.log10(g):.1f} dB" if g > 0.0 else "silent"


def takes():
    text = open(os.path.join(ROOT, "src", "perception.rs"), encoding="utf-8").read()
    m = re.search(r"pub const WHISTLE_TAKES: u8 = (\d+);", text)
    return int(m.group(1)) if m else 4


def gain(t, variant):
    return t["gain_" + variant]


def seeming(distance, t):
    """Inverted: truly near seems far (0), truly far seems close (1)."""
    span = max(1e-3, t["cue_far_distance"] - t["cue_near_distance"])
    return min(1.0, max(0.0, (distance - t["cue_near_distance"]) / span))


def variant_for(s, t):
    if s >= t["cue_loud_above"]:
        return "loud"
    if s >= t["cue_mid_above"]:
        return "mid"
    return "faint"


# --------------------------------------------------------------------------
# Sound
# --------------------------------------------------------------------------

_cache = {}


def load(name):
    if name not in _cache:
        path = os.path.join(AUDIO, name)
        with wave.open(path) as w:
            n = w.getnframes()
            _cache[name] = [v / 32768.0 for v in struct.unpack(f"<{n}h", w.readframes(n))]
    return _cache[name]


def at_speed(x, speed):
    """Played faster is also higher, as in the game."""
    if abs(speed - 1.0) < 1e-4:
        return x
    n = int(len(x) / speed)
    out = [0.0] * n
    for i in range(n):
        p = i * speed
        j = int(p)
        f = p - j
        a = x[j] if j < len(x) else 0.0
        b = x[j + 1] if j + 1 < len(x) else 0.0
        out[i] = a + (b - a) * f
    return out


class Event:
    def __init__(self, at, variant, take, speed=1.0, label=""):
        self.at, self.variant, self.take, self.speed, self.label = at, variant, take, speed, label


def render(events, t, m, args):
    """Mix the phrases over the night bed the way `audio.rs` does: each voice
    at its gain on its bus (`mix::gain`), never over full scale."""
    clips = [at_speed(load(f"whistle_{e.variant}_{e.take}.wav"), e.speed) for e in events]
    end = max(int(e.at * SR) + len(c) for e, c in zip(events, clips)) + int(1.5 * SR)
    whistle = [0.0] * end
    busy = [False] * end
    # His whistle follows the master alone.
    on_master = bus_gain(args.volume, 1.0, m)
    for e, c in zip(events, clips):
        i0 = int(e.at * SR)
        g = min(1.0, gain(t, e.variant) * on_master)
        for k, v in enumerate(c):
            whistle[i0 + k] += v * g
            busy[i0 + k] = True
    out = [0.0] * end
    if not args.dry:
        amb, rain = load("ambience_llano.wav"), load("rain_loop.wav")
        level = args.rain
        on_ambience = bus_gain(args.volume, args.ambience, m)
        amb_gain = t["ambience_gain"] * on_ambience * (1.0 - 0.25 * (level - 0.6) / 0.4)
        if args.tense:
            amb_gain *= t["ambience_hush"]
        rain_gain = t["rain_gain"] * on_ambience * level
        # The bed draws back while a whistle sounds, gliding like the game's
        # loops (`mix::glide` at GLIDE_RATE, here per sample).
        duck, k = 1.0, 1.0 - math.exp(-m["GLIDE_RATE"] / SR)
        for i in range(end):
            target = t["whistle_duck"] if busy[i] else 1.0
            duck += (target - duck) * k
            out[i] = (amb[i % len(amb)] * min(1.0, amb_gain * duck) + rain[i % len(rain)] * min(1.0, rain_gain * duck))
    # The device clips whatever the mix sends past full scale.
    return [max(-1.0, min(1.0, b + w)) for b, w in zip(out, whistle)]


def write(path, samples):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(b"".join(struct.pack("<h", int(v * 32767)) for v in samples))


def player():
    for cmd in (["pw-play"], ["paplay"], ["aplay", "-q"], ["ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet"]):
        if shutil.which(cmd[0]):
            return cmd
    return None


def perform(events, t, m, args):
    samples = render(events, t, m, args)
    if args.out:
        write(args.out, samples)
        for e in events:
            print(f"{e.at:6.1f}s  {e.label}")
        print(f"wrote {args.out} ({len(samples) / SR:.1f} s)")
        return
    cmd = player()
    if cmd is None:
        sys.exit("whistle_lab: no audio player found (pw-play, paplay, aplay or ffplay); use --out FILE")
    fd, path = tempfile.mkstemp(suffix=".wav", prefix="whistle_lab_")
    os.close(fd)
    try:
        write(path, samples)
        proc = subprocess.Popen(cmd + [path])
        start = time.monotonic()
        for e in events:
            wait = e.at - (time.monotonic() - start)
            if wait > 0:
                time.sleep(wait)
            print(f"{e.at:6.1f}s  {e.label}", flush=True)
        proc.wait()
    except KeyboardInterrupt:
        proc.terminate()
    finally:
        os.remove(path)


# --------------------------------------------------------------------------
# What to hear
# --------------------------------------------------------------------------

def length(variant, take, speed=1.0):
    return len(load(f"whistle_{variant}_{take}.wav")) / SR / speed


def ladder(t, args):
    chosen = range(takes()) if args.all_takes else [args.take]
    events, at = [], 0.8
    for take in chosen:
        for v in VARIANTS:
            events.append(Event(at, v, take, label=f"take {take}: {SEEMS[v]}  (gain {gain(t, v)})"))
            at += length(v, take) + 1.2
    return events


def ab(t, args):
    rng = random.Random(args.seed)
    events, at = [], 0.8
    order = ["loud", "faint", "loud", "faint", "mid", "faint", "loud"]
    for v in order:
        take = rng.randrange(takes())
        events.append(Event(at, v, take, label=f"{SEEMS[v]}  (take {take})"))
        at += min(length(v, take), 3.6) + 0.6
    return events


def approach(t, args):
    rng = random.Random(args.seed)
    events, at, last = [], 0.8, None
    for d in (60, 45, 36, 28, 22, 17, 13, 10, 7, 5, 3):
        take = rng.randrange(takes() - 1) if last is not None else rng.randrange(takes())
        if last is not None and take >= last:
            take += 1
        last = take
        s = seeming(d, t)
        v = variant_for(s, t)
        events.append(Event(at, v, take, label=f"he is truly {d:>2} m away → seems {SEEMS[v]}"))
        at += 3.2
    return events


def night(t, args):
    """He stalks at a wandering distance, whistling on the game's irregular
    cadence (mirrors CueDirector::stalk_gap at a middling pressure)."""
    rng = random.Random(args.seed)
    sooner = 1.0 - 0.35 * 0.4  # a middling pressure
    events, at, dist, last = [], 1.0, 30.0, None
    lo_speed, hi_speed = t["whistle_speed"]
    while at < args.seconds:
        take = rng.randrange(takes() - 1) if last is not None else rng.randrange(takes())
        if last is not None and take >= last:
            take += 1
        last = take
        v = variant_for(seeming(dist, t), t)
        speed = rng.uniform(lo_speed, hi_speed)
        roll = rng.random()
        if roll < t["stalk_answer_chance"]:
            gap, kind = rng.uniform(*t["stalk_answer"]), "an answer"
        elif roll < t["stalk_answer_chance"] + t["stalk_silence_chance"]:
            gap, kind = rng.uniform(*t["stalk_silence"]) * sooner, "a long quiet"
        else:
            gap, kind = rng.uniform(*t["stalk_phrase_interval"]) * sooner, ""
        events.append(
            Event(at, v, take, speed, label=f"{dist:4.0f} m → {v:<5} take {take} speed {speed:.2f}   then {gap:4.1f} s {kind}")
        )
        at += gap
        # He circles, drifting in and out around fifteen metres.
        dist = min(55.0, max(3.0, dist + rng.uniform(-10.0, 10.0) + (15.0 - dist) * 0.3))
    return events


# --------------------------------------------------------------------------
# Measure: three distances, three different sounds
# --------------------------------------------------------------------------
#
# The files are loudness-matched and the gains are the only level
# difference, so what must tell the distances apart is how each one sounds.
# Every feature is measured on the file alone (no reference to the others),
# with the standard library only:
#
#   air     energy above 3.5 kHz, where no note of either whistle reaches:
#           breath and air (dB of the whole, floored at AIR_FLOOR, which is
#           still some 20 dB above the files' 16-bit noise)
#   bright  spectral centroid over the pitch being whistled (1.0 = a pure
#           tone); pitch-relative, so a lower or higher take, or the game's
#           random playback speed, does not move it
#   slap    how clearly the sound repeats itself a fifth to a third of a
#           second later: the peak of its autocorrelation there, over the
#           autocorrelation just beside it (a single echo is a sharp spike;
#           a held note or a reverb's comb ripple is not)
#   ring    seconds it keeps sounding after its last loud moment (within
#           10 dB of the loudest) until it is 40 dB down: the room ringing
#           on, against the direct sound stopping
#   gaps    share of the phrase, from the first sound to the last loud
#           moment, spent in silences of at least GAP_MIN seconds (below the
#           loudest by GAP_DB): pieces taken away
#   level   dB RMS of the loudest second (what gen_audio.level() matches)
#
# Each feature has to separate the pairs it is meant for (the higher band
# first) by at least its margin from the worst take of one to the worst take
# of the other, and the four takes of each band must lie closer together
# than the two bands' averages lie apart. The rules hold at the game's
# slowest and fastest playback speed too (tuning.whistle_speed, resampled
# the way the game plays them), so no signature leans on the exact rate.
# "hz" is reported only: it follows which take (and speed) is played more
# than the distance. The levels must agree instead: every take within
# LEVEL_TAKES of its band's median, and every band's median within
# LEVEL_BANDS of gen_audio.WHISTLE_RMS and of the other bands', so the
# game's gains stay the only level difference.
#
# Not checked: that the close take is drier than the middling one. Its
# exhale and fade ring on as long as the middling one's room, and no simple
# feature told the two apart on both builds and at both speeds; air,
# brightness and slap are what separate them.

BANDS = ("loud", "mid", "faint")
AIR_FROM, AIR_FLOOR = 3500.0, -70.0
GAP_DB, GAP_MIN = -30.0, 0.15
SLAP_WINDOW = (0.20, 0.32)  # seconds: "a quarter-second slap"
SLAP_AGREE = 0.02  # the mid takes' slaps must land within this of each other (a designed echo)
LEVEL_TAKES, LEVEL_BANDS = 1.5, 2.0  # dB
RULES = [
    # feature, higher band, lower band, least margin, why
    ("air", "loud", "mid", 10.0, "the close one breathes; across the grass the air is gone"),
    ("air", "loud", "faint", 10.0, "the close one breathes; far away it is a bare thread"),
    ("bright", "loud", "mid", 0.05, "breath lifts the close one's spectrum above its notes"),
    ("bright", "loud", "faint", 0.05, "breath lifts the close one's spectrum above its notes"),
    ("slap", "mid", "loud", 0.12, "only across the grass does every note come back"),
    ("slap", "mid", "faint", 0.12, "only across the grass does every note come back"),
    ("ring", "faint", "mid", 0.3, "far away it is mostly the night's room"),
    ("ring", "faint", "loud", 0.3, "far away it is mostly the night's room"),
    ("gaps", "faint", "loud", 0.05, "the wind takes pieces of the far one away"),
    ("gaps", "faint", "mid", 0.05, "the wind takes pieces of the far one away"),
]

_twiddles = {}


def fft(a):
    """In-place iterative radix-2 FFT of a list of complex numbers."""
    n = len(a)
    j = 0
    for i in range(1, n):
        bit = n >> 1
        while j & bit:
            j ^= bit
            bit >>= 1
        j |= bit
        if i < j:
            a[i], a[j] = a[j], a[i]
    size = 2
    while size <= n:
        half = size >> 1
        tw = _twiddles.get(size)
        if tw is None:
            tw = _twiddles[size] = [cmath.exp(-2j * math.pi * k / size) for k in range(half)]
        for start in range(0, n, size):
            for k in range(half):
                p = start + k
                q = p + half
                t = tw[k] * a[q]
                a[q] = a[p] - t
                a[p] = a[p] + t
        size <<= 1
    return a


def read_wav(path):
    with wave.open(path) as w:
        n = w.getnframes()
        return [v / 32768.0 for v in struct.unpack(f"<{n}h", w.readframes(n))]


def frame_energy(x, hop):
    """Mean square of each `hop`-sample frame, and the same in dB below the loudest."""
    e = []
    for i in range(0, len(x) - hop + 1, hop):
        s = 0.0
        for v in x[i:i + hop]:
            s += v * v
        e.append(s / hop)
    top = max(e) or 1e-12
    return e, [10.0 * math.log10(max(v, 1e-14) / top) for v in e]


def spectral(x, n=2048):
    """(centroid Hz, pitch Hz, air dB) over every frame within 40 dB of the loudest."""
    df = SR / n
    win = [0.5 - 0.5 * math.cos(2.0 * math.pi * i / n) for i in range(n)]
    lo, hi, air_k = int(400 / df), int(3500 / df), int(AIR_FROM / df)
    frames = []
    for i in range(0, len(x) - n + 1, n):
        seg = x[i:i + n]
        a = fft([complex(v * w, 0.0) for v, w in zip(seg, win)])
        frames.append((sum(v * v for v in seg), [c.real * c.real + c.imag * c.imag for c in a[: n // 2 + 1]]))
    loudest = max(e for e, _ in frames)
    total = air = moment = pitch = 0.0
    for e, p in frames:
        if e < loudest * 1e-4:
            continue
        s = sum(p[2:])
        total += s
        air += sum(p[air_k:])
        moment += sum(k * df * p[k] for k in range(2, len(p)))
        band = p[lo:hi]
        pitch += (lo + max(range(len(band)), key=band.__getitem__)) * df * s
    total = max(total, 1e-14)
    return moment / total, pitch / total, max(AIR_FLOOR, 10.0 * math.log10(max(air, 1e-14) / total))


def slap(x, fc):
    """(score, lag s): the sharpest self-repeat between SLAP_WINDOW seconds,
    from the autocorrelation of the whistle band mixed down around `fc`."""
    dec = 16
    fsr = SR / dec
    w = -2j * math.pi * fc / SR
    rot = [cmath.exp(w * k) for k in range(dec)]
    step, ph, z = cmath.exp(w * dec), 1 + 0j, []
    for m in range(len(x) // dec):
        seg = x[m * dec:(m + 1) * dec]
        z.append(sum(v * r for v, r in zip(seg, rot)) * ph)
        ph *= step
    maxlag = int((SLAP_WINDOW[1] + 0.05) * fsr)
    n = 1
    while n < len(z) + maxlag:
        n <<= 1
    a = fft(z + [0j] * (n - len(z)))
    r = fft([complex(c.real * c.real + c.imag * c.imag, 0.0) for c in a])  # real power: |forward| = |inverse|
    r = [abs(c) for c in r[:maxlag + 1]]
    r = [v / (r[0] or 1e-12) for v in r]
    near, far = int(0.01 * fsr), int(0.03 * fsr)
    best, at = -1.0, 0
    for k in range(int(SLAP_WINDOW[0] * fsr), int(SLAP_WINDOW[1] * fsr)):
        side = r[k - far:k - near] + r[k + near + 1:k + far + 1]
        score = r[k] - sum(side) / len(side)
        if score > best:
            best, at = score, k
    return best, at / fsr


def features(x):
    hz, pitch, air = spectral(x)
    score, lag = slap(x, pitch)
    _, db = frame_energy(x, 882)  # 20 ms frames, dB below the loudest
    last_loud = max(i for i, d in enumerate(db) if d >= -10.0)
    ring = (max(i for i, d in enumerate(db) if d >= -40.0) - last_loud) * 0.02
    first = next(i for i, d in enumerate(db) if d >= GAP_DB)
    silent = run = 0
    for d in db[first:last_loud + 1] + [0.0]:
        if d < GAP_DB:
            run += 1
        else:
            if run * 0.02 >= GAP_MIN:
                silent += run
            run = 0
    return dict(hz=hz, air=air, bright=hz / max(pitch, 1.0), slap=score, lag=lag, ring=ring,
                gaps=silent / max(1, last_loud + 1 - first), seconds=len(x) / SR)


def active_level(x, window=1.0):
    """dB RMS of the loudest `window` seconds (as gen_audio.level() sees it)."""
    w = int(window * SR)
    acc = best = sum(v * v for v in x[:w])
    for i in range(w, len(x)):
        acc += x[i] * x[i] - x[i - w] * x[i - w]
        best = max(best, acc)
    return 10.0 * math.log10(max(best, 1e-14) / min(w, len(x)))


def median(values):
    s = sorted(values)
    m = len(s) // 2
    return s[m] if len(s) % 2 else (s[m - 1] + s[m]) / 2.0


def measure(directory, label, t, on_master):
    """Print the features of the twelve files in `directory`; return the failures.
    `on_master` is the whistle's gain under the master slider (`bus_gain`),
    or None when the curve could not be read."""
    sys.path.insert(0, HERE)
    import gen_audio

    n = takes()
    speeds = sorted({1.0, *t["whistle_speed"]}) if t else [1.0]
    clips = {(band, k): read_wav(os.path.join(directory, f"whistle_{band}_{k}.wav")) for band in BANDS for k in range(n)}
    rows = {s: {key: features(x if s == 1.0 else at_speed(x, s)) for key, x in clips.items()} for s in speeds}
    levels = {key: active_level(x) for key, x in clips.items()}
    print(f"\n== {label}: {directory}")
    print(f"{'file':<20}{'air dB':>8}{'bright':>8}{'slap':>7}{'at s':>7}{'ring s':>8}{'gaps':>7}{'hz':>7}"
          f"{'level dB':>10}{'played dBFS':>13}")
    for band, k in clips:
        f = rows[1.0][(band, k)]
        g = min(1.0, gain(t, band) * on_master) if t and on_master is not None else None
        played = "-" if g is None else "silent" if g <= 0.0 else f"{levels[(band, k)] + 20.0 * math.log10(g):.1f}"
        print(f"{f'whistle_{band}_{k}.wav':<20}{f['air']:>8.1f}{f['bright']:>8.3f}{f['slap']:>7.3f}{f['lag']:>7.3f}"
              f"{f['ring']:>8.2f}{f['gaps']:>7.2f}{f['hz']:>7.0f}{levels[(band, k)]:>10.1f}{played:>13}")
    failures = []
    print(f"rule (worst take against worst take, at the worst of speeds {', '.join(f'{s:g}' for s in speeds)}; "
          f"spread = the widest band's range of takes)")
    for feat, hi, lo, least, why in RULES:
        worst = None
        for s in speeds:
            a = [rows[s][(hi, k)][feat] for k in range(n)]
            b = [rows[s][(lo, k)][feat] for k in range(n)]
            margin = min(a) - max(b)
            apart = sum(a) / n - sum(b) / n
            spread = max(max(a) - min(a), max(b) - min(b))
            ok = margin >= least and spread < apart
            # The failing speed if any, otherwise the one with the least room.
            key = (ok, min(margin - least, apart - spread))
            if worst is None or key < worst[0]:
                worst = (key, s, margin, apart, spread)
        (ok, _), s, margin, apart, spread = worst
        if not ok:
            failures.append(f"{label}: {feat} {hi} > {lo} (speed {s:g})")
        print(f"  {'ok  ' if ok else 'FAIL'} {feat:<6} {hi:>5} > {lo:<5} by {margin:7.3f} (needs {least:g}); "
              f"apart {apart:7.3f} > spread {spread:6.3f} at speed {s:<4g}  {why}")
    for s in speeds:
        lags = [rows[s][("mid", k)]["lag"] for k in range(n)]
        ok = max(lags) - min(lags) <= SLAP_AGREE
        if not ok:
            failures.append(f"{label}: the mid slaps land at {lags} (speed {s:g})")
        print(f"  {'ok  ' if ok else 'FAIL'} slap   mid takes all repeat {min(lags):.3f}-{max(lags):.3f} s later "
              f"at speed {s:g} (within {SLAP_AGREE} s: one designed echo, not chance)")
    target = 20.0 * math.log10(gen_audio.WHISTLE_RMS)
    medians = {}
    for band in BANDS:
        lv = [levels[(band, k)] for k in range(n)]
        medians[band] = med = median(lv)
        off = max(abs(v - med) for v in lv)
        ok = off <= LEVEL_TAKES and abs(med - target) <= LEVEL_BANDS
        if not ok:
            failures.append(f"{label}: {band} levels {', '.join(f'{v:.1f}' for v in lv)} dB")
        print(f"  {'ok  ' if ok else 'FAIL'} level  {band:>5} median {med:6.1f} dB, {med - target:+.1f} from "
              f"WHISTLE_RMS (needs {LEVEL_BANDS:g}); takes within {off:.1f} of it (needs {LEVEL_TAKES:g})")
    apart = max(medians.values()) - min(medians.values())
    ok = apart <= LEVEL_BANDS
    if not ok:
        failures.append(f"{label}: band levels {apart:.1f} dB apart")
    print(f"  {'ok  ' if ok else 'FAIL'} level  the band medians lie within {apart:.1f} dB of each other "
          f"(needs {LEVEL_BANDS:g}): the game's gains are the only level difference")
    return failures


def run_measure(args):
    # The features need neither; only the played level does.
    try:
        t = tuning()
    except (SystemExit, OSError, ValueError):
        t = None
    try:
        m = mix_numbers()
    except (SystemExit, OSError, ValueError):
        m = None
    on_master = None
    if m:
        master = m["DEFAULT_MASTER"] if args.volume is None else args.volume
        on_master = bus_gain(master, 1.0, m)
    if t:
        print(f"gains the game plays (src/tuning.rs): loud {t['gain_loud']}  mid {t['gain_mid']}  faint {t['gain_faint']}"
              f"  ({20 * math.log10(t['gain_mid'] / t['gain_loud']):.1f} / {20 * math.log10(t['gain_faint'] / t['gain_loud']):.1f} dB)"
              f"; bed ducked to {t['whistle_duck']} under a whistle; speed {t['whistle_speed'][0]}-{t['whistle_speed'][1]}"
              + (f"; played at master {master:g} ({db_text(on_master)}, src/mix.rs)" if m else "; src/mix.rs unread"))
        print("hear them as the game mixes them: python tools/whistle_lab.py ladder --all-takes")
    failures = []
    if args.synth:
        sys.path.insert(0, HERE)
        import gen_audio

        gen_audio.use_synth()
        with tempfile.TemporaryDirectory(prefix="whistle_synth_") as tmp:
            for name, samples in gen_audio.make_whistles().items():
                gen_audio.write_wav(os.path.join(tmp, name), samples)
            failures += measure(tmp, "synthesized fallback", t, on_master)
    else:
        directory = args.dir or AUDIO
        missing = [f"whistle_{v}_{k}.wav" for v in BANDS for k in range(takes())
                   if not os.path.exists(os.path.join(directory, f"whistle_{v}_{k}.wav"))]
        if missing:
            sys.exit(f"whistle_lab: missing {missing} in {directory}")
        failures += measure(directory, "files" if args.dir else "shipped files", t, on_master)
    if failures:
        print("\nmeasure: FAIL — " + "; ".join(failures))
        if args.check:
            sys.exit(1)
    else:
        print("\nmeasure: ok — every pair of distances is told apart")


def main():
    # A Windows pipe or file speaks the ANSI code page, which has no "→":
    # print what it can rather than stop.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(errors="replace")
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("mode", nargs="?", default="ladder", choices=["ladder", "ab", "approach", "night", "measure"])
    parser.add_argument("--regen", action="store_true", help="rebuild the whistle files first")
    parser.add_argument("--take", type=int, default=0, help="ladder: which take (default 0)")
    parser.add_argument("--all-takes", action="store_true", help="ladder: every take")
    parser.add_argument("--dry", action="store_true", help="no rain or insects under it")
    parser.add_argument("--tense", action="store_true", help="the insects hushed, as in a warning or hunt")
    parser.add_argument("--rain", type=float, default=0.8, help="rain level 0.6..1.0 (default 0.8)")
    parser.add_argument("--master", "--volume", dest="volume", type=float, default=None,
                        help="the master slider, 0..1 (default the game's, from src/mix.rs)")
    parser.add_argument("--ambience", type=float, default=1.0,
                        help="the Ambience slider under the bed, 0..1 (default 1); the whistle ignores it")
    parser.add_argument("--seconds", type=float, default=120.0, help="night: how long")
    parser.add_argument("--seed", type=int, default=1997)
    parser.add_argument("--out", help="write the mix to this WAV instead of playing it")
    parser.add_argument("--check", action="store_true", help="measure: exit 1 unless the distances are told apart")
    parser.add_argument("--synth", action="store_true",
                        help="measure: the synthesized fallback whistle, built into a temporary directory")
    parser.add_argument("--dir", help="measure: the twelve whistle files in DIR instead of assets/audio")
    args = parser.parse_args()
    if args.synth and args.dir:
        parser.error("--synth builds its own files into a temporary directory; it takes no --dir")

    if args.regen:
        sys.path.insert(0, HERE)
        import gen_audio

        gen_audio.write_all(AUDIO, gen_audio.make_whistles())
    if args.mode == "measure":
        run_measure(args)
        return
    t = tuning()
    m = mix_numbers()
    if args.volume is None:
        args.volume = m["DEFAULT_MASTER"]
    n = takes()
    if not 0 <= args.take < n:
        sys.exit(f"whistle_lab: --take must be 0..{n - 1}")
    missing = [f"whistle_{v}_{k}.wav" for v in VARIANTS for k in range(n) if not os.path.exists(os.path.join(AUDIO, f"whistle_{v}_{k}.wav"))]
    if missing:
        sys.exit(f"whistle_lab: missing {missing}; run with --regen")
    print(
        f"gains (tuning.rs): loud {t['gain_loud']}  mid {t['gain_mid']}  faint {t['gain_faint']}  "
        f"→ {20 * math.log10(t['gain_loud'] / t['gain_faint']):.1f} dB loud-to-faint; "
        f"bed {'off' if args.dry else f'rain {args.rain}' + (', tense' if args.tense else '')}"
        f"{'' if args.dry or args.ambience == 1.0 else f' (ambience {args.ambience:g})'}; "
        f"master {args.volume:g} puts his whistle at {db_text(bus_gain(args.volume, 1.0, m))} (src/mix.rs)"
    )
    events = {"ladder": ladder, "ab": ab, "approach": approach, "night": night}[args.mode](t, args)
    perform(events, t, m, args)


if __name__ == "__main__":
    main()

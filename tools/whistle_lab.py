#!/usr/bin/env python3
"""Whistle lab: hear the Silbón's whistle exactly as the game mixes it,
without building or playing the game.

Every level and threshold is read from `src/tuning.rs` (the three variant
gains, the rain and ambience beds and the duck under a whistle, the
distance-to-variant inversion, the stalking cadence), and the sounds are the
game's own files in `assets/audio/`. Edit the gains in tuning.rs, or the
sound design in `tools/gen_audio.py` (then pass --regen), and listen again.

    python3 tools/whistle_lab.py                # the three distances, one take
    python3 tools/whistle_lab.py ladder --all-takes
    python3 tools/whistle_lab.py ab             # loud and faint back to back
    python3 tools/whistle_lab.py approach       # he walks in from 60 m to 3 m
    python3 tools/whistle_lab.py night          # two minutes of him stalking
    python3 tools/whistle_lab.py night --seconds 300 --seed 9

Common options: --regen (rebuild the whistle files first, ~3 s), --dry (no
rain or insects), --tense (the insects hushed, as while he warns or hunts),
--volume V (master, default the game's 0.8), --rain R (0.6 drizzle .. 1.0
downpour), --out FILE (write a WAV instead of playing it).

The `approach` and `night` modes mirror `perception.rs` (the inversion and
`CueDirector::stalk_gap`) in a few lines of Python; the Rust tests cover the
game's own behaviour.
"""

import argparse
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


def render(events, t, args):
    """Mix the phrases over the night bed the way `audio.rs` does."""
    clips = [at_speed(load(f"whistle_{e.variant}_{e.take}.wav"), e.speed) for e in events]
    end = max(int(e.at * SR) + len(c) for e, c in zip(events, clips)) + int(1.5 * SR)
    whistle = [0.0] * end
    busy = [False] * end
    for e, c in zip(events, clips):
        i0 = int(e.at * SR)
        g = gain(t, e.variant)
        for k, v in enumerate(c):
            whistle[i0 + k] += v * g
            busy[i0 + k] = True
    out = [0.0] * end
    if not args.dry:
        amb, rain = load("ambience_llano.wav"), load("rain_loop.wav")
        level = args.rain
        amb_gain = t["ambience_gain"] * (1.0 - 0.25 * (level - 0.6) / 0.4)
        if args.tense:
            amb_gain *= t["ambience_hush"]
        rain_gain = t["rain_gain"] * level
        # The bed draws back while a whistle sounds, gliding like the game's
        # loops (a fifth of a second).
        duck, k = 1.0, 1.0 - math.exp(-5.0 / SR)
        for i in range(end):
            target = t["whistle_duck"] if busy[i] else 1.0
            duck += (target - duck) * k
            out[i] = (amb[i % len(amb)] * amb_gain + rain[i % len(rain)] * rain_gain) * duck
    master = args.volume
    return [max(-1.0, min(1.0, (b + w) * master)) for b, w in zip(out, whistle)]


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


def perform(events, t, args):
    samples = render(events, t, args)
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


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("mode", nargs="?", default="ladder", choices=["ladder", "ab", "approach", "night"])
    parser.add_argument("--regen", action="store_true", help="rebuild the whistle files first")
    parser.add_argument("--take", type=int, default=0, help="ladder: which take (default 0)")
    parser.add_argument("--all-takes", action="store_true", help="ladder: every take")
    parser.add_argument("--dry", action="store_true", help="no rain or insects under it")
    parser.add_argument("--tense", action="store_true", help="the insects hushed, as in a warning or hunt")
    parser.add_argument("--rain", type=float, default=0.8, help="rain level 0.6..1.0 (default 0.8)")
    parser.add_argument("--volume", type=float, default=0.8, help="master volume (the game's default 0.8)")
    parser.add_argument("--seconds", type=float, default=120.0, help="night: how long")
    parser.add_argument("--seed", type=int, default=1997)
    parser.add_argument("--out", help="write the mix to this WAV instead of playing it")
    args = parser.parse_args()

    if args.regen:
        sys.path.insert(0, HERE)
        import gen_audio

        gen_audio.write_all(AUDIO, gen_audio.make_whistles())
    t = tuning()
    n = takes()
    if not 0 <= args.take < n:
        sys.exit(f"whistle_lab: --take must be 0..{n - 1}")
    missing = [f"whistle_{v}_{k}.wav" for v in VARIANTS for k in range(n) if not os.path.exists(os.path.join(AUDIO, f"whistle_{v}_{k}.wav"))]
    if missing:
        sys.exit(f"whistle_lab: missing {missing}; run with --regen")
    print(
        f"gains (tuning.rs): loud {t['gain_loud']}  mid {t['gain_mid']}  faint {t['gain_faint']}  "
        f"→ {20 * math.log10(t['gain_loud'] / t['gain_faint']):.1f} dB loud-to-faint; "
        f"bed {'off' if args.dry else f'rain {args.rain}' + (', tense' if args.tense else '')}, master {args.volume}"
    )
    events = {"ladder": ladder, "ab": ab, "approach": approach, "night": night}[args.mode](t, args)
    perform(events, t, args)


if __name__ == "__main__":
    main()

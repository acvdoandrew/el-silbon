#!/usr/bin/env python3
"""Deterministic generator for every original sound in El Silbón.

Pure Python standard library (no numpy). Re-running produces byte-identical
files for the same SEED. Output: 16-bit PCM, mono, 44.1 kHz WAV files in
assets/audio/. The game plays all of them non-spatially (no panning, no
distance attenuation); perceived distance of the whistle is baked into the
three timbre variants below and chosen by the game's perception layer.

Usage:  python3 tools/gen_audio.py            (writes into ../assets/audio)
        python3 tools/gen_audio.py --out DIR
"""

import argparse
import math
import os
import random
import struct
import wave

SR = 44100
SEED = 1997
TAU = 2.0 * math.pi


# --------------------------------------------------------------------------
# Small DSP toolkit
# --------------------------------------------------------------------------

def silence(seconds):
    return [0.0] * int(seconds * SR)


def one_pole_lowpass(x, cutoff):
    a = math.exp(-TAU * cutoff / SR)
    y, out = 0.0, []
    for s in x:
        y = (1.0 - a) * s + a * y
        out.append(y)
    return out


def one_pole_highpass(x, cutoff):
    low = one_pole_lowpass(x, cutoff)
    return [s - l for s, l in zip(x, low)]


def biquad_bandpass(x, center, q):
    w0 = TAU * center / SR
    alpha = math.sin(w0) / (2.0 * q)
    b0, b1, b2 = alpha, 0.0, -alpha
    a0, a1, a2 = 1.0 + alpha, -2.0 * math.cos(w0), 1.0 - alpha
    b0, b1, b2, a1, a2 = b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0
    x1 = x2 = y1 = y2 = 0.0
    out = []
    for s in x:
        y = b0 * s + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
        x2, x1, y2, y1 = x1, s, y1, y
        out.append(y)
    return out


def comb(x, delay, feedback, damp):
    buf = [0.0] * delay
    idx, store, out = 0, 0.0, []
    for s in x:
        y = buf[idx]
        store = y * (1.0 - damp) + store * damp
        buf[idx] = s + store * feedback
        idx = (idx + 1) % delay
        out.append(y)
    return out


def allpass(x, delay, gain):
    buf = [0.0] * delay
    idx, out = 0, []
    for s in x:
        b = buf[idx]
        y = -s + b
        buf[idx] = s + b * gain
        idx = (idx + 1) % delay
        out.append(y)
    return out


def reverb(x, size=1.0, damp=0.35, feedback=0.82):
    """Small Schroeder/Freeverb-style mono reverb (original implementation)."""
    combs = [1116, 1188, 1277, 1356]
    acc = [0.0] * len(x)
    for d in combs:
        c = comb(x, max(1, int(d * size)), feedback, damp)
        for i, v in enumerate(c):
            acc[i] += v * 0.25
    for d in (556, 441):
        acc = allpass(acc, max(1, int(d * size)), 0.5)
    return acc


def mix(dst, src, offset=0, gain=1.0):
    n = len(dst)
    for i, v in enumerate(src):
        j = offset + i
        if j >= n:
            break
        dst[j] += v * gain
    return dst


def normalize(x, peak):
    m = max(1e-9, max(abs(v) for v in x))
    g = peak / m
    return [v * g for v in x]


def fade(x, fade_in, fade_out):
    n = len(x)
    fi, fo = int(fade_in * SR), int(fade_out * SR)
    out = list(x)
    for i in range(min(fi, n)):
        out[i] *= i / fi
    for i in range(min(fo, n)):
        out[n - 1 - i] *= i / fo
    return out


def write_wav(path, samples):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        frames = bytearray()
        for v in samples:
            v = max(-1.0, min(1.0, v))
            frames += struct.pack("<h", int(round(v * 32767.0)))
        w.writeframes(bytes(frames))


def noise(rng, n):
    return [rng.uniform(-1.0, 1.0) for _ in range(n)]


# --------------------------------------------------------------------------
# The whistle: one original ascending phrase, three perceived distances
# --------------------------------------------------------------------------

# An original, slightly "off" ascending seven-step run that ends on a held,
# rising note and a falling tail. Frequencies in Hz, durations in seconds.
PHRASE = [
    (830.0, 0.25),
    (932.0, 0.23),
    (1046.0, 0.25),
    (1108.0, 0.21),
    (1244.0, 0.27),
    (1396.0, 0.29),
    (1480.0, 0.95),
]
GLIDE = 0.045  # portamento between steps


def whistle_dry(rng, breath_amount, wobble):
    """Synthesize the dry phrase; returns (tone, breath) sample lists."""
    total = sum(d for _, d in PHRASE) + 0.35
    n = int(total * SR)
    tone = [0.0] * n
    breath_env = [0.0] * n
    phase = 0.0
    t_cursor = 0.0
    boundaries = []
    for f, d in PHRASE:
        boundaries.append((t_cursor, t_cursor + d, f))
        t_cursor += d
    tail_start = t_cursor
    wob_phase = rng.uniform(0.0, TAU)
    for i in range(n):
        t = i / SR
        # Pitch track with glides between steps.
        f = PHRASE[-1][0]
        amp = 0.0
        for k, (a, b, fk) in enumerate(boundaries):
            if a <= t < b:
                f = fk
                if k > 0 and t - a < GLIDE:
                    prev = boundaries[k - 1][2]
                    u = (t - a) / GLIDE
                    u = u * u * (3.0 - 2.0 * u)
                    f = prev + (fk - prev) * u
                # Tongued articulation: each step swells, small dip between.
                local = (t - a) / (b - a)
                attack = min(1.0, (t - a) / 0.022)
                release = min(1.0, (b - t) / 0.03) if k < len(boundaries) - 1 else 1.0
                amp = attack * (0.62 + 0.38 * release)
                if k == len(boundaries) - 1:
                    # Held final note: slow rise in pitch and intensity.
                    f = fk * (1.0 + 0.085 * local * local)
                    amp *= 0.9 + 0.1 * math.sin(math.pi * min(1.0, local * 1.2))
                break
        if t >= tail_start:
            # Falling breathy tail.
            u = (t - tail_start) / 0.35
            f = PHRASE[-1][0] * 1.085 * (1.0 - 0.22 * u)
            amp = max(0.0, 1.0 - u) ** 1.6
        vib_depth = 0.004 + (0.009 if t > boundaries[-1][0] else 0.0)
        vib = 1.0 + vib_depth * math.sin(TAU * 5.3 * t)
        vib *= 1.0 + wobble * math.sin(TAU * 0.9 * t + wob_phase)
        phase += TAU * f * vib / SR
        s = math.sin(phase) + 0.07 * math.sin(2.0 * phase + 0.4) + 0.015 * math.sin(3.0 * phase)
        tone[i] = s * amp
        breath_env[i] = amp
    breath = one_pole_highpass(noise(rng, n), 1800.0)
    breath = one_pole_lowpass(breath, 7000.0)
    breath = [b * e * breath_amount for b, e in zip(breath, breath_env)]
    return tone, breath


def make_whistles(rng):
    out = {}

    # LOUD: seems right beside you (dry, breathy, full presence). The game
    # plays this when the Silbón is truly FAR away.
    tone, breath = whistle_dry(rng, breath_amount=0.22, wobble=0.0)
    body = [t + b for t, b in zip(tone, breath)]
    room = reverb(body, size=0.55, damp=0.5, feedback=0.6)
    loud = [d * 0.9 + r * 0.12 for d, r in zip(body, room)]
    loud += silence(0.25)
    out["whistle_loud.wav"] = normalize(fade(loud, 0.005, 0.2), 0.82)

    # MIDDLING: somewhere across the grass.
    tone, breath = whistle_dry(rng, breath_amount=0.08, wobble=0.002)
    body = [t + b for t, b in zip(tone, breath)] + silence(1.2)
    body = one_pole_lowpass(body, 5200.0)
    room = reverb(body, size=1.0, damp=0.4, feedback=0.8)
    mid = [d * 0.62 + r * 0.38 for d, r in zip(body, room)]
    out["whistle_mid.wav"] = normalize(fade(mid, 0.01, 0.4), 0.7)

    # FAINT: thin, far, far away. The game plays this when he is truly NEAR.
    tone, breath = whistle_dry(rng, breath_amount=0.0, wobble=0.006)
    body = tone + silence(2.2)
    body = one_pole_highpass(body, 900.0)
    body = one_pole_lowpass(one_pole_lowpass(body, 3300.0), 3600.0)
    room = reverb(body, size=1.35, damp=0.55, feedback=0.86)
    faint = [d * 0.22 + r * 0.78 for d, r in zip(body, room)]
    out["whistle_faint.wav"] = normalize(fade(faint, 0.02, 0.9), 0.55)
    return out


# --------------------------------------------------------------------------
# Night ambience of the llano (seamless loop)
# --------------------------------------------------------------------------

def make_ambience(rng):
    loop = 24.0
    xfade = 2.0
    n = int((loop + xfade) * SR)
    out = [0.0] * n

    # Wind: low-passed noise with slow periodic swells.
    wind = one_pole_lowpass(one_pole_lowpass(noise(rng, n), 520.0), 700.0)
    for i in range(n):
        t = i / SR
        swell = 0.55 + 0.25 * math.sin(TAU * t * 2.0 / loop + 0.7) + 0.2 * math.sin(TAU * t * 5.0 / loop + 2.1)
        out[i] += wind[i] * swell * 1.6

    # Grass hiss: faint high band, modulated with the wind.
    hiss = one_pole_highpass(noise(rng, n), 3000.0)
    for i in range(n):
        t = i / SR
        out[i] += hiss[i] * 0.018 * (0.6 + 0.4 * math.sin(TAU * t * 3.0 / loop))

    # Crickets: three insects, chirps of short pulses.
    for cricket in range(3):
        carrier = rng.uniform(4300.0, 5000.0)
        period = rng.uniform(0.55, 0.9)
        pulses = rng.choice([2, 3, 3, 4])
        level = rng.uniform(0.035, 0.06)
        t = rng.uniform(0.0, period)
        while t < loop + xfade:
            for p in range(pulses):
                start = t + p * 0.034
                length = 0.018
                i0 = int(start * SR)
                for k in range(int(length * SR)):
                    i = i0 + k
                    if i >= n:
                        break
                    e = math.sin(math.pi * k / (length * SR))
                    out[i] += math.sin(TAU * carrier * (i / SR)) * e * level
            t += period * rng.uniform(0.92, 1.08)
            if rng.random() < 0.12:
                t += rng.uniform(0.6, 1.8)  # pauses make it breathe

    # Distant frogs: soft low croak trains.
    for _ in range(5):
        start = rng.uniform(0.5, loop - 1.5)
        f0 = rng.uniform(170.0, 260.0)
        for k in range(rng.randint(3, 6)):
            s = start + k * rng.uniform(0.11, 0.16)
            length = 0.07
            i0 = int(s * SR)
            ph = 0.0
            for j in range(int(length * SR)):
                i = i0 + j
                if i >= n:
                    break
                e = math.sin(math.pi * j / (length * SR)) ** 2
                ph += TAU * f0 * (1.0 + 0.2 * (1.0 - j / (length * SR))) / SR
                v = math.sin(ph) + 0.5 * math.sin(3.0 * ph) + 0.25 * math.sin(5.0 * ph)
                out[i] += v * e * 0.03

    # A far nightbird: two falling notes, twice per loop.
    for start in (6.3, 17.8):
        for k, (f, d) in enumerate(((2150.0, 0.16), (1650.0, 0.34))):
            s = start + k * 0.22
            i0 = int(s * SR)
            ph = 0.0
            for j in range(int(d * SR)):
                i = i0 + j
                if i >= n:
                    break
                u = j / (d * SR)
                e = math.sin(math.pi * u) ** 1.5
                ph += TAU * f * (1.0 - 0.12 * u) / SR
                out[i] += math.sin(ph) * e * 0.02

    # Seamless loop: crossfade the extra tail into the head.
    L = int(loop * SR)
    X = int(xfade * SR)
    looped = out[:L]
    for i in range(X):
        u = i / X
        looped[i] = out[i] * u + out[L + i] * (1.0 - u)
    return normalize(looped, 0.5)


# --------------------------------------------------------------------------
# One-shots
# --------------------------------------------------------------------------

def clack(rng, length=0.09):
    n = int(length * SR)
    partials = [(rng.uniform(1200, 1900), rng.uniform(0.018, 0.035)),
                (rng.uniform(2400, 3600), rng.uniform(0.010, 0.022)),
                (rng.uniform(650, 900), rng.uniform(0.02, 0.05))]
    out = [0.0] * n
    for f, decay in partials:
        ph = rng.uniform(0, TAU)
        for i in range(n):
            t = i / SR
            out[i] += math.sin(ph + TAU * f * t) * math.exp(-t / decay) * 0.4
    burst = one_pole_highpass(noise(rng, n), 2500.0)
    for i in range(n):
        out[i] += burst[i] * math.exp(-(i / SR) / 0.004) * 0.5
    return out


def make_bones(rng):
    out = silence(1.1)
    t = 0.02
    for _ in range(15):
        mix(out, clack(rng), int(t * SR), rng.uniform(0.35, 1.0))
        t += rng.uniform(0.018, 0.09)
        if t > 0.95:
            break
    # Cloth shuffle of the burlap satchel.
    cloth = one_pole_lowpass(one_pole_highpass(noise(rng, len(out)), 400.0), 2500.0)
    for i in range(len(out)):
        u = i / len(out)
        out[i] += cloth[i] * 0.25 * math.sin(math.pi * u)
    return normalize(fade(out, 0.002, 0.08), 0.7)


def make_restitution(rng):
    dur = 4.0
    n = int(dur * SR)
    out = [0.0] * n
    # Hollow knock of the root.
    for f, decay, g in ((96.0, 0.35, 1.0), (192.0, 0.2, 0.4), (301.0, 0.12, 0.25)):
        for i in range(n):
            t = i / SR
            out[i] += math.sin(TAU * f * t) * math.exp(-t / decay) * g * 0.5
    # Soft open chord swell (A2 E3 A3 C#4) with slow beating.
    for f in (110.0, 164.8, 220.0, 277.2):
        det = rng.uniform(-0.4, 0.4)
        for i in range(n):
            t = i / SR
            env = min(1.0, t / 1.2) * max(0.0, 1.0 - max(0.0, t - 2.2) / 1.8)
            out[i] += (math.sin(TAU * (f + det) * t) + 0.3 * math.sin(TAU * 2 * (f + det) * t)) * env * 0.12
    # The tree exhales: breathy band of noise.
    br = biquad_bandpass(noise(rng, n), 700.0, 0.8)
    for i in range(n):
        t = i / SR
        env = math.sin(math.pi * min(1.0, t / dur)) ** 2
        out[i] += br[i] * env * 0.35
    wet = reverb(out, size=1.2, damp=0.5, feedback=0.8)
    out = [d * 0.7 + w * 0.3 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.003, 0.6), 0.6)


def make_caught(rng):
    dur = 3.4
    n = int(dur * SR)
    out = [0.0] * n
    # Rushing wind that swells and collapses.
    rush = noise(rng, n)
    rush = one_pole_lowpass(rush, 1600.0)
    for i in range(n):
        t = i / SR
        env = (min(1.0, t / 0.9) ** 2) * max(0.0, 1.0 - max(0.0, t - 1.4) / 1.8)
        out[i] += rush[i] * env * 1.4
    # Low dissonant cluster.
    for f in (55.0, 58.3, 82.4):
        for i in range(n):
            t = i / SR
            env = min(1.0, t / 0.6) * max(0.0, 1.0 - max(0.0, t - 1.6) / 1.7)
            out[i] += math.sin(TAU * f * t) * env * 0.22
    # A final thin whistle fragment, faint: the fair warning, remembered.
    tone, _ = whistle_dry(rng, breath_amount=0.0, wobble=0.01)
    frag = one_pole_highpass(tone[: int(1.0 * SR)], 1200.0)
    frag = reverb(frag + silence(1.0), size=1.3, damp=0.6, feedback=0.85)
    mix(out, frag, int(0.25 * SR), 0.2)
    return normalize(fade(out, 0.01, 0.4), 0.7)


def make_dawn(rng):
    dur = 5.0
    n = int(dur * SR)
    out = [0.0] * n
    for f, g in ((146.8, 0.2), (220.0, 0.16), (293.7, 0.12), (370.0, 0.1), (440.0, 0.07)):
        det = rng.uniform(-0.3, 0.3)
        for i in range(n):
            t = i / SR
            env = min(1.0, t / 1.6) * max(0.0, 1.0 - max(0.0, t - 2.4) / 2.6)
            out[i] += math.sin(TAU * (f + det) * t) * env * g
    # A few far bell-like partials, like light on the road.
    for k, s in enumerate((0.8, 1.5, 2.3)):
        f = (880.0, 1108.7, 1318.5)[k]
        i0 = int(s * SR)
        for j in range(n - i0):
            t = j / SR
            out[i0 + j] += math.sin(TAU * f * t) * math.exp(-t / 1.1) * 0.05
    wet = reverb(out, size=1.1, damp=0.45, feedback=0.78)
    out = [d * 0.75 + w * 0.25 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.05, 0.8), 0.5)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", default=os.path.join(here, "..", "assets", "audio"))
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)

    rng = random.Random(SEED)
    files = {}
    files.update(make_whistles(rng))
    files["ambience_llano.wav"] = make_ambience(rng)
    files["bones_rattle.wav"] = make_bones(rng)
    files["restitution.wav"] = make_restitution(rng)
    files["caught.wav"] = make_caught(rng)
    files["dawn.wav"] = make_dawn(rng)

    for name, samples in files.items():
        path = os.path.join(args.out, name)
        write_wav(path, samples)
        print(f"wrote {os.path.relpath(path)}  {len(samples) / SR:5.2f}s")


if __name__ == "__main__":
    main()

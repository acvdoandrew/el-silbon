#!/usr/bin/env python3
"""Deterministic generator for every original sound in El Silbón.

Pure Python standard library (no numpy). Re-running produces byte-identical
files for the same SEED. Output: 16-bit PCM, mono, 44.1 kHz WAV files in
assets/audio/. The game plays all of them non-spatially (no panning, no
distance attenuation); perceived distance of the whistle is baked into the
three distances below (each in several takes) and chosen by the game's
perception layer.

Usage:  python3 tools/gen_audio.py                  (writes into ../assets/audio)
        python3 tools/gen_audio.py --out DIR
        python3 tools/gen_audio.py --only-whistles  (just the whistle takes, ~3 s)
        python3 tools/gen_audio.py --only-whistles --synth --out DIR
                                   (the synthesized fallback whistle, as if the
                                    recording were absent; never into assets/)

To hear the whistles as the game mixes them: python3 tools/whistle_lab.py
To check the three distances stay apart:    python3 tools/whistle_lab.py measure --check
"""

import argparse
import collections
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


def echo(x, delay, gain, cutoff):
    """One reflection off the far treeline: later, duller, quieter."""
    out = list(x) + silence(delay)
    return mix(out, one_pole_lowpass(x, cutoff), int(delay * SR), gain)


def skip_old_whistles(rng):
    """The first stream once rendered the three whistles here. Draw the same
    numbers they drew, so every later file of that stream stays identical."""
    n = int((sum(d for _, d in PHRASE) + 0.35) * SR)
    for _ in range(3):
        rng.uniform(0.0, TAU)
        noise(rng, n)


# Four performances of the phrase. Each is rendered at all three perceived
# distances, so the distance is always the timbre and the take is only the
# way he whistles it this time: steps are semitones above the base note, a
# None is a breath of silence. Seven steps up, as the tale has it, but in a
# dark mode, slow and sliding, never quite in tune: a man alone in the night,
# not a tune (the brisk major run of the first build sounded jolly).
WHISTLE_BASE = 700.0
TAKES = [
    # As the tale tells it: seven steps up, the last held and rising.
    dict(shift=0, tempo=1.0, steps=[0, 2, 3, 5, 7, 8, 10], end="rise"),
    # Lower and slower; the last note held, then sinking.
    dict(shift=-2, tempo=1.2, steps=[0, 2, 3, 5, 7, 8, 10], end="fall"),
    # Broken off: five steps, a silence, two higher notes, cut short.
    dict(shift=1, tempo=0.95, steps=[0, 1, 3, 5, 7, None, 10, 11], end="cut"),
    # Hurried, one step stumbled on, ending in a long slide up.
    dict(shift=-1, tempo=0.85, steps=[0, 1, 3, 3, 5, 7, 8, 10], end="glide"),
]
WHISTLE_TAKES = len(TAKES)
STEP = 0.3  # seconds a step, before the take's tempo
SLIDE = 0.11  # seconds sliding from one note into the next


def whistle_take(rng, take):
    """One performance, dry; returns (tone, breath envelope, pitch track,
    sample where the final note begins).

    Legato: each note is reached by a slow slide and scooped into from a
    little below; every note sits a few cents off true; a slow, wide waver
    and a breathy onset. The tone is a whistle's: a pure fundamental with a
    band of air around it (see `pitched_air`)."""
    spec = TAKES[take]
    tempo = spec["tempo"]
    segments = []  # (start, end, freq or None)
    t = 0.0
    steps = spec["steps"]
    for k, step in enumerate(steps):
        last = k == len(steps) - 1
        if step is None:
            d = 0.5 * tempo
        elif last:
            d = (0.28 if spec["end"] == "cut" else 1.05) * tempo
        else:
            d = STEP * tempo * rng.uniform(0.85, 1.2)
        if step is None:
            f = None
        else:
            cents = rng.uniform(-28.0, 18.0)  # never quite in tune, mostly flat
            f = WHISTLE_BASE * 2.0 ** ((step + spec["shift"] + cents / 100.0) / 12.0)
        segments.append((t, t + d, f))
        t += d
    phrase_end = t
    tail = 0.0 if spec["end"] == "cut" else 0.45
    n = int((phrase_end + tail + 0.05) * SR)
    tone = [0.0] * n
    env = [0.0] * n
    freqs = [0.0] * n
    phase = 0.0
    wob_phase = rng.uniform(0.0, TAU)
    seg = 0
    last_f = segments[-1][2]
    prev_f = segments[0][2]
    for i in range(n):
        t = i / SR
        while seg < len(segments) - 1 and t >= segments[seg][1]:
            if segments[seg][2] is not None:
                prev_f = segments[seg][2]
            seg += 1
        a, b, fk = segments[seg]
        if t < phrase_end:
            if fk is None:
                f, amp = prev_f, 0.0
            else:
                local = (t - a) / (b - a)
                after_note = seg > 0 and segments[seg - 1][2] is not None
                if after_note and t - a < SLIDE:
                    u = (t - a) / SLIDE
                    u = u * u * (3.0 - 2.0 * u)
                    f = prev_f + (fk - prev_f) * u
                else:
                    # scooped in from a little below, settling late
                    scoop = max(0.0, 1.0 - (t - a) / (SLIDE * 1.6))
                    f = fk * (1.0 - 0.03 * scoop * scoop)
                # breathy swells: legato between notes, a soft start from a rest
                floor_in = 0.78 if after_note else 0.0
                attack = floor_in + (1.0 - floor_in) * min(1.0, (t - a) / 0.07)
                swell = 0.85 + 0.15 * math.sin(math.pi * min(1.0, local))
                final = seg == len(segments) - 1
                if final:
                    style = spec["end"]
                    if style == "rise":
                        f *= 1.0 + 0.06 * local * local
                    elif style == "fall":
                        f *= 1.0 + 0.02 * local - 0.14 * max(0.0, local - 0.3) ** 1.5
                    elif style == "glide":
                        f *= 1.0 + 0.24 * local * local
                    release = min(1.0, (b - t) / 0.02) if style == "cut" else 1.0
                    amp = attack * swell * release
                else:
                    r = min(1.0, (b - t) / 0.05)
                    before_note = segments[seg + 1][2] is not None
                    amp = attack * swell * ((0.8 + 0.2 * r) if before_note else r)
        else:
            # A long, falling breath of a tail (none after a phrase cut short).
            u = min(1.0, (t - phrase_end) / max(tail, 1e-3))
            end_f = {"rise": 1.06, "fall": 0.88, "glide": 1.24}.get(spec["end"], 1.0)
            f = last_f * end_f * (1.0 - 0.3 * u)
            amp = max(0.0, 1.0 - u) ** 1.4 if tail > 0.0 else 0.0
        # a slow, wide waver, deepening on the held note, and a drift
        held = seg == len(segments) - 1
        vib = 1.0 + (0.006 + (0.012 if held else 0.0)) * math.sin(TAU * 4.1 * t + wob_phase)
        vib *= 1.0 + 0.006 * math.sin(TAU * 0.45 * t + 2.0 * wob_phase)
        f *= vib
        freqs[i] = f
        phase += TAU * f / SR
        tone[i] = (math.sin(phase) + 0.02 * math.sin(2.0 * phase + 0.4)) * amp
        env[i] = amp
    return tone, env, freqs, int(segments[-1][0] * SR)


def pitched_air(rng, freqs, env, bandwidth=120.0):
    """The air of a real whistle: noise through a narrow resonance that
    follows the pitch, so the tone breathes instead of beeping."""
    y1 = y2 = 0.0
    out = [0.0] * len(freqs)
    r = 1.0 - math.pi * bandwidth / SR
    for i, f in enumerate(freqs):
        c = 2.0 * r * math.cos(TAU * max(f, 50.0) / SR)
        y = (1.0 - r) * rng.uniform(-1.0, 1.0) + c * y1 - r * r * y2
        y2, y1 = y1, y
        out[i] = y
    peak = max(1e-9, max(abs(v) for v in out))
    return [v / peak * e for v, e in zip(out, env)]


# The whistle itself is a recording chosen by the user: "El Silbon
# Silbido" (YouTube oNGPZNXmZ1c, uploaded by lisandrolivier, 2017; licence
# unknown, see assets/SOURCES.md), cleaned to 44.1 kHz mono with the video's
# hum and hiss taken out. It holds the phrase twice: a creep of short,
# tongued notes up from about 1.2 to 2.35 kHz and a long held note. Takes 2
# and 3 are the same two performances played a little lower and slower, and
# a little higher and faster. Without the file (or with --synth) the
# synthesized performance below stands in.
SOURCE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "audio", "source",
                      "el_silbon_silbido.wav")
# (start, end, playback rate, where the top note first lands): seconds in the
# recording. From that note on is the held end of the phrase, which the far
# whistle never loses to the wind.
SOURCE_TAKES = [(0.09, 3.99, 1.0, 2.22), (4.0, 7.9, 1.0, 6.13), (0.09, 3.99, 0.93, 2.22), (4.0, 7.9, 1.05, 6.13)]
_source = None
USE_RECORDING = True  # False: build the synthesized fallback (--synth)


def recording():
    """Whether the whistles are built from the recording."""
    return USE_RECORDING and os.path.exists(SOURCE)


def use_synth():
    """Build the whistles from the synthesized fallback from now on."""
    global USE_RECORDING
    USE_RECORDING = False


def source_take(take):
    """One recorded performance, at its playback rate (below 1: lower and
    slower); returns (samples, sample where the held end begins), or None
    without the recording."""
    global _source
    if not recording():
        return None
    if _source is None:
        with wave.open(SOURCE, "rb") as w:
            assert w.getframerate() == SR and w.getnchannels() == 1 and w.getsampwidth() == 2
            raw = w.readframes(w.getnframes())
        _source = [v / 32768.0 for v in struct.unpack("<%dh" % (len(raw) // 2), raw)]
    start, end, rate, held = SOURCE_TAKES[take]
    clip = _source[int(start * SR):int(end * SR)]
    n = int(len(clip) / rate)
    out = [0.0] * n
    for i in range(n):
        pos = i * rate
        k = int(pos)
        f = pos - k
        a = clip[min(k, len(clip) - 1)]
        b = clip[min(k + 1, len(clip) - 1)]
        out[i] = a + (b - a) * f
    return fade(out, 0.004, 0.08), int((held - start) * SR / rate)


def ring_air(rng, x, bandwidth=60.0):
    """The air of a recorded whistle: the tone times slowly wandering noise,
    so a narrow band of breath rides on every note without tracking its
    pitch (the recording was cleaned of its own air)."""
    m = one_pole_lowpass(one_pole_lowpass(noise(rng, len(x)), bandwidth), bandwidth)
    peak = max(1e-9, max(abs(v) for v in m))
    return [v * w / peak for v, w in zip(x, m)]


def whistle_voice(rng, take, air=0.45, breath=0.12):
    """The dry performance with `air` (a band of breath around the tone) and
    `breath` (hiss riding the phrase): the recording when it is there,
    otherwise the synthesized tone. Returns (voice, envelope, sample where
    the final held note begins)."""
    source = source_take(take)
    if source is not None:
        recorded, held_at = source
        env = one_pole_lowpass([abs(v) for v in recorded], 20.0)
        peak = max(1e-9, max(env))
        env = [e / peak for e in env]
        band = ring_air(rng, recorded)
        # The recording sits higher than the synth, so its hiss does too.
        hiss = one_pole_lowpass(one_pole_highpass(noise(rng, len(recorded)), 2000.0), 9000.0)
        voice = [r + bd * air + h * e * breath for r, bd, h, e in zip(recorded, band, hiss, env)]
        return voice, env, held_at
    tone, env, freqs, held_at = whistle_take(rng, take)
    band = pitched_air(rng, freqs, env)
    hiss = one_pole_lowpass(one_pole_highpass(noise(rng, len(tone)), 1500.0), 7000.0)
    return [tn + bd * air + h * e * breath for tn, bd, h, e in zip(tone, band, hiss, env)], env, held_at


def voiced_end(env, floor=0.03):
    """The sample after the last one where the phrase still sounds."""
    for i in range(len(env) - 1, -1, -1):
        if env[i] > floor:
            return i + 1
    return len(env)


def active_rms(x, window=1.0):
    """RMS of the loudest `window` seconds: how loud the phrase itself is,
    whatever its tail of echoes and room."""
    w = int(window * SR)
    if len(x) <= w:
        return math.sqrt(sum(v * v for v in x) / max(1, len(x)))
    acc = sum(v * v for v in x[:w])
    best = acc
    for i in range(w, len(x)):
        acc += x[i] * x[i] - x[i - w] * x[i - w]
        best = max(best, acc)
    return math.sqrt(max(0.0, best) / w)


# Every whistle file has the same phrase loudness; the game's three gains
# (tuning.gain_loud/mid/faint) are the only level difference between them.
# `whistle_lab.py measure --check` holds every file to it.
WHISTLE_RMS = 0.3
WHISTLE_CEILING = 0.97


def limit(x, ceiling=WHISTLE_CEILING, look=0.01, release=0.12):
    """A look-ahead peak limiter: the gain eases down over `look` seconds
    before any sample that would pass `ceiling` and recovers over about
    `release`, so a few stray peaks cost a moment of level instead of the
    whole file's. Deterministic; never lets a sample past the ceiling."""
    n = len(x)
    ahead = max(1, int(look * SR))
    need = [ceiling / abs(v) if abs(v) > ceiling else 1.0 for v in x]
    # The least gain needed anywhere in the next `ahead` samples...
    lowest, window = [1.0] * n, collections.deque()
    for i in range(n - 1, -1, -1):
        while window and need[window[-1]] >= need[i]:
            window.pop()
        window.append(i)
        while window[0] > i + ahead:
            window.popleft()
        lowest[i] = need[window[0]]
    # ...held with a slow recovery...
    rise, held = 1.0 - math.exp(-1.0 / (release * SR)), 1.0
    for i in range(n):
        held = min(lowest[i], held + (1.0 - held) * rise)
        lowest[i] = held
    # ...and averaged over the look-ahead, which ramps it down before each
    # peak and still never exceeds the gain that peak needs.
    out, acc = [0.0] * n, 0.0
    for i in range(n):
        acc += lowest[i]
        if i > ahead:
            acc -= lowest[i - ahead - 1]
        out[i] = max(-ceiling, min(ceiling, x[i] * acc / min(i + 1, ahead + 1)))
    return out


def level(x, rms=WHISTLE_RMS):
    """Bring the phrase (its loudest second) to `rms`, then limit the few
    peaks that would clip. Scaling the whole file down to fit its highest
    peak instead left each take's level to its crest factor: a take whose
    loudest second held a wind gap came out 4 dB quieter than its sisters."""
    g = rms / max(1e-9, active_rms(x))
    return limit([v * g for v in x])


# Each distance is a signature, not a level: the files are loudness-matched
# (WHISTLE_RMS) and the game's gains do the rest, so the signatures must read
# at any volume, over rain, on any headset. `whistle_lab.py measure --check`
# holds them apart.
#   close  (loud):  breath and air on every note, dry, an audible inhale
#                   before and exhale after; hardly any room.
#   across (mid):   the air gone, and one clear slap a quarter-second behind
#                   every note (SLAP), little room.
#   far    (faint): late, thin, shimmering; the wind takes whole pieces of the
#                   phrase away (seeded per take, never the held end) and what
#                   comes through is mostly the open night's room.
NEAR_AIR, NEAR_BREATH, NEAR_ROOM = 0.35, 0.28, 0.08
SLAP, SLAP_GAIN = 0.26, 0.6
# Share the wind takes of the stretch it may touch (from 0.3 s after the
# first note to 0.1 s before the held end): about half of all the phrase
# before its held end.
FAR_GONE = (0.6, 0.7)


def breath_noise(rng, seconds, centre, attack, peak):
    """A soft breath: band-passed noise that swells over `attack` of its
    length and dies away."""
    n = int(seconds * SR)
    b = biquad_bandpass(noise(rng, n), centre, 0.8)
    for i in range(n):
        u = i / n
        b[i] *= (math.sin(0.5 * math.pi * u / attack) if u < attack else ((1.0 - u) / (1.0 - attack)) ** 1.5) * peak
    return b


def whistle_near(rng, take):
    """LOUD: right beside you. An intake of breath first, then the phrase
    close and breathy — every waver and every slip of the air — and the
    breath let go after it; almost no room. Played when he is truly FAR."""
    voice, env, _ = whistle_voice(rng, take, air=NEAR_AIR, breath=NEAR_BREATH)
    # The intake before he whistles, running straight into the first note,
    # and the breath let go after it: only heard this close.
    inhale = breath_noise(rng, 0.55, 1100.0, 0.5, 0.45)
    exhale = breath_noise(rng, 0.45, 850.0, 0.12, 0.45)
    start = len(inhale) - int(0.08 * SR)
    out = inhale[:start] + silence(len(voice) / SR + 0.55)
    mix(out, inhale[start:], start)
    mix(out, voice, start)
    mix(out, exhale, start + max(0, voiced_end(env) - int(0.05 * SR)))
    room = reverb(one_pole_lowpass(out, 3500.0), size=0.5, damp=0.7, feedback=0.6)
    out = [d + r * NEAR_ROOM for d, r in zip(out, room)]
    return level(fade(out, 0.005, 0.3))


def whistle_across(rng, take):
    """MIDDLING: somewhere across the grass. Hardly any breath, the
    brightness gone, and one clear slap a quarter-second behind every note
    off the far fence line; a little room."""
    voice, _, _ = whistle_voice(rng, take, air=0.12, breath=0.04)
    body = voice + silence(1.4)
    top = 3400.0 if recording() else 2400.0  # the recording sits higher than the synth
    body = one_pole_lowpass(one_pole_lowpass(body, top), top)
    body = echo(body, SLAP, SLAP_GAIN, top * 0.8)
    room = reverb(body, size=0.9, damp=0.55, feedback=0.78)
    out = [d * 0.7 + r * 0.3 for d, r in zip(body, room)]
    return level(fade(out, 0.01, 0.5))


def wind_gaps(rng, start, stop):
    """Where the wind takes the phrase away: up to three seeded stretches
    between `start` and `stop` (seconds), each long enough to hear as
    nothing; returns [(from, to)]."""
    span = stop - start
    gone = span * rng.uniform(*FAR_GONE)
    if gone < 0.3:
        return []
    n = max(1, min(3, int(gone / 0.42)))
    weights = [rng.uniform(0.75, 1.25) for _ in range(n)]
    lengths = [gone * w / sum(weights) for w in weights]
    heard = [rng.uniform(0.6, 1.4) for _ in range(n + 1)]
    pieces = [(span - gone) * h / sum(heard) for h in heard]
    gaps, t = [], start
    for k in range(n):
        t += pieces[k]
        gaps.append((t, t + lengths[k]))
        t += lengths[k]
    return gaps


def wind_gate(n, gaps, ramp=0.06, floor=0.015):
    """A gain curve that falls to `floor` over each gap, with soft edges."""
    g = [1.0] * n
    r = int(ramp * SR)
    for a, b in gaps:
        i0, i1 = int(a * SR), int(b * SR)
        for i in range(max(0, i0), min(n, i1)):
            u = min(1.0, (i - i0) / r, (i1 - i) / r)
            u = 0.5 - 0.5 * math.cos(math.pi * u)
            g[i] = 1.0 - (1.0 - floor) * u
    return g


def whistle_far(rng, take):
    """FAINT: far, far away. A thin thread of tone, late to arrive and
    shimmering in the air between; the wind takes whole pieces of the
    phrase away (never its held end), the treeline answers twice, and what
    comes through is mostly the open night. Played when he is truly NEAR."""
    voice, env, held_at = whistle_voice(rng, take, air=0.0, breath=0.0)
    late = 0.15
    body = silence(late) + voice + silence(2.6)
    high = recording()  # the recording sits higher than the synth
    body = one_pole_highpass(one_pole_highpass(body, 900.0 if high else 600.0), 900.0 if high else 600.0)
    for _ in range(4):
        body = one_pole_lowpass(body, 2100.0 if high else 1150.0)
    # The level wanders a little as the air carries it, and shimmers.
    drift = one_pole_lowpass(one_pole_lowpass(noise(rng, len(body)), 0.8), 0.8)
    peak = max(1e-9, max(abs(g) for g in drift))
    rate, depth, ph = rng.uniform(4.0, 7.0), 0.25, rng.uniform(0.0, TAU)
    body = [v * (0.8 + 0.2 * max(-1.0, min(1.0, g / peak))) * (1.0 + depth * math.sin(TAU * rate * i / SR + ph))
            for i, (v, g) in enumerate(zip(body, drift))]
    body = echo(echo(body, 0.46, 0.5, 1900.0 if high else 1200.0), 0.95, 0.3, 1600.0 if high else 1000.0)
    room = reverb(body, size=1.6, damp=0.62, feedback=0.9)
    out = [d * 0.12 + r * 0.88 for d, r in zip(body, room)]
    # The wind takes pieces of it away, room and all: from a little after
    # the first note to a little before the held end.
    first = next((i for i, e in enumerate(env) if e > 0.05), 0)
    gaps = wind_gaps(rng, late + first / SR + 0.3, late + held_at / SR - 0.1)
    out = [v * g for v, g in zip(out, wind_gate(len(out), gaps))]
    # Levelled after the gaps, on what is left: levelled before them, each
    # take would lose a different share of its loudest notes to the wind
    # and the four would land up to 3 dB apart.
    return level(fade(out, 0.03, 1.0))


WHISTLE_DISTANCES = (("loud", whistle_near), ("mid", whistle_across), ("faint", whistle_far))


def make_whistles():
    """whistle_{loud,mid,faint}_{take}.wav; each take on its own seed, the
    same performance at all three distances."""
    out = {}
    for take in range(WHISTLE_TAKES):
        for d, (name, render) in enumerate(WHISTLE_DISTANCES):
            # The same take seed at every distance: the same performance.
            out[f"whistle_{name}_{take}.wav"] = render(random.Random(SEED + 300 + take), take)
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


# --------------------------------------------------------------------------
# Mechanics: weather, footsteps, machines, the herd and the party
# --------------------------------------------------------------------------

def loop_crossfade(out, loop, xfade):
    """Fold the extra tail of `out` into its head so the result repeats seamlessly."""
    L = int(loop * SR)
    X = int(xfade * SR)
    looped = out[:L]
    for i in range(X):
        u = i / X
        looped[i] = out[i] * u + out[L + i] * (1.0 - u)
    return looped


def damped(out, start, freq, decay, gain, phase=0.0, length=None):
    """Add one exponentially decaying sine partial."""
    i0 = int(start * SR)
    length = length if length is not None else decay * 7.0
    for k in range(min(int(length * SR), len(out) - i0)):
        t = k / SR
        out[i0 + k] += math.sin(phase + TAU * freq * t) * math.exp(-t / decay) * gain


def make_rain(rng):
    loop, xfade = 8.0, 1.5
    n = int((loop + xfade) * SR)
    sheet = one_pole_lowpass(one_pole_highpass(noise(rng, n), 700.0), 9000.0)
    sheet = one_pole_lowpass(sheet, 7000.0)
    earth = one_pole_lowpass(noise(rng, n), 260.0)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        swell = 0.85 + 0.15 * math.sin(TAU * t / 3.1 + 0.4) + 0.06 * math.sin(TAU * t / 1.3)
        out[i] = sheet[i] * 0.55 * swell + earth[i] * 1.4
    # Drops on leaves and tin: sparse bright ticks.
    t = 0.0
    while t < loop + xfade:
        t += rng.expovariate(90.0)
        i0 = int(t * SR)
        f = rng.uniform(2500.0, 6500.0)
        g = rng.uniform(0.02, 0.1)
        ph = rng.uniform(0.0, TAU)
        for k in range(int(0.012 * SR)):
            i = i0 + k
            if i >= n:
                break
            out[i] += math.sin(ph + TAU * f * k / SR) * math.exp(-k / (0.003 * SR)) * g
    return normalize(loop_crossfade(out, loop, xfade), 0.55)


def make_thunder(rng, length, rolls):
    n = int(length * SR)
    out = [0.0] * n
    crack = one_pole_highpass(noise(rng, n), 1200.0)
    for i in range(int(0.6 * SR)):
        out[i] += crack[i] * math.exp(-(i / SR) / 0.05)
    raw = noise(rng, n)
    lo1 = one_pole_lowpass(raw, 240.0)
    lo2 = one_pole_lowpass(raw, 90.0)
    # The sky rolls: irregular swells, each a sub-strike further away.
    swells = [(0.0, 1.0, 0.3)]
    for _ in range(rolls):
        swells.append((rng.uniform(0.3, length * 0.55), rng.uniform(0.25, 0.7), rng.uniform(0.25, 0.7)))
    for i in range(n):
        t = i / SR
        env = 0.0
        for t0, g, dec in swells:
            if t >= t0:
                d = t - t0
                env += g * (1.0 - math.exp(-d / 0.06)) * math.exp(-d / (dec + 0.8))
        out[i] += (lo1[i] * 5.0 + lo2[i] * 12.0) * env
        out[i] += math.sin(TAU * 46.0 * t) * math.exp(-t / 2.2) * 0.25
    wet = reverb(out, size=1.6, damp=0.6, feedback=0.85)
    out = [d * 0.7 + w * 0.3 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.002, 1.2), 0.85)


def make_heartbeat(rng):
    n = int(0.9 * SR)
    out = [0.0] * n

    def thump(t0, f0, level):
        i0 = int(t0 * SR)
        ph = 0.0
        click = one_pole_lowpass(noise(rng, int(0.05 * SR)), 500.0)
        for k in range(min(int(0.24 * SR), n - i0)):
            t = k / SR
            f = f0 * (1.0 - 0.35 * min(1.0, t / 0.12))
            ph += TAU * f / SR
            v = math.sin(ph) * math.exp(-t / 0.07) * level
            if k < len(click):
                v += click[k] * math.exp(-t / 0.02) * level * 4.0
            out[i0 + k] += v

    thump(0.0, 62.0, 1.0)
    thump(0.23, 54.0, 0.6)
    return normalize(fade(out, 0.002, 0.05), 0.9)


def make_step(rng, kind):
    n = int(0.34 * SR)
    out = [0.0] * n
    if kind == "dirt":
        body = one_pole_lowpass(noise(rng, n), 900.0)
        grit = biquad_bandpass(noise(rng, n), 2200.0, 0.7)
        for i in range(n):
            t = i / SR
            out[i] += body[i] * math.exp(-t / 0.09) * 6.0 + grit[i] * math.exp(-t / 0.05) * 0.9
        damped(out, 0.0, rng.uniform(80.0, 110.0), 0.05, 0.8)
    elif kind == "grass":
        swish = biquad_bandpass(noise(rng, n), rng.uniform(3300.0, 4400.0), 0.6)
        for i in range(n):
            t = i / SR
            env = min(1.0, t / 0.04) * math.exp(-t / 0.12)
            out[i] += swish[i] * env * 1.6
        damped(out, 0.0, 70.0, 0.03, 0.5)
    elif kind == "wood":
        tick = one_pole_highpass(noise(rng, n), 1800.0)
        damped(out, 0.0, rng.uniform(170.0, 230.0), 0.07, 0.7)
        damped(out, 0.0, rng.uniform(380.0, 520.0), 0.04, 0.5)
        damped(out, 0.0, rng.uniform(900.0, 1300.0), 0.02, 0.2)
        for i in range(n):
            t = i / SR
            out[i] += tick[i] * math.exp(-t / 0.008) * 0.5
        # A little creak as the plank gives.
        ph = 0.0
        f0 = rng.uniform(210.0, 300.0)
        for i in range(int(0.2 * SR)):
            t = i / SR
            ph += TAU * (f0 + 90.0 * t) / SR
            out[i] += math.sin(ph) * math.sin(math.pi * t / 0.2) * 0.12 * (1.0 + 0.6 * math.sin(TAU * 60.0 * t))
    else:  # water
        splash = biquad_bandpass(noise(rng, n), 1300.0, 0.7)
        for i in range(n):
            t = i / SR
            out[i] += splash[i] * math.exp(-t / 0.11) * 2.4
        for _ in range(3):
            t0 = rng.uniform(0.03, 0.2)
            ph = 0.0
            i0 = int(t0 * SR)
            for k in range(int(0.05 * SR)):
                t = k / SR
                if i0 + k >= n:
                    break
                ph += TAU * (400.0 + 14000.0 * t) / SR
                out[i0 + k] += math.sin(ph) * math.sin(math.pi * t / 0.05) * 0.2
    return normalize(fade(out, 0.001, 0.06), 0.7)


def make_cattle(rng):
    dur = 2.7
    n = int(dur * SR)
    src = [0.0] * n
    ph = 0.0
    for i in range(n):
        t = i / SR
        u = t / dur
        f = 96.0 * (0.9 + 0.25 * math.sin(math.pi * min(1.0, u * 1.15)) - 0.12 * u * u)
        f *= 1.0 + 0.012 * math.sin(TAU * 5.5 * t) + 0.02 * math.sin(TAU * 0.8 * t)
        ph += TAU * f / SR
        s = 0.0
        for k in range(1, 22):
            s += math.sin(k * ph) / (k ** 1.05)
        src[i] = s
    a = biquad_bandpass(src, 520.0, 3.0)
    b = biquad_bandpass(src, 980.0, 4.0)
    c = biquad_bandpass(src, 190.0, 1.5)
    breath = one_pole_lowpass(noise(rng, n), 900.0)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        env = min(1.0, t / 0.35) ** 1.5 * min(1.0, (dur - t) / 0.9)
        open_mouth = min(1.0, t / 1.2)  # "oo" opens toward "aw"
        out[i] = (c[i] * 1.0 + a[i] * (0.4 + 0.5 * open_mouth) + b[i] * 0.5 * open_mouth + src[i] * 0.08
                  + breath[i] * 0.9) * env
    out = one_pole_lowpass(out, 2500.0)
    return normalize(fade(out, 0.02, 0.4), 0.75)


def make_engine_loop(rng):
    dur, extra = 2.0, 0.2
    n = int((dur + extra) * SR)
    fire = 22.0  # firings per second: 44 in the loop, so every partial repeats exactly
    clat = one_pole_highpass(one_pole_lowpass(noise(rng, n), 3500.0), 900.0)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        wob = 1.0 + 0.06 * math.sin(TAU * 1.0 * t) + 0.03 * math.sin(TAU * 3.0 * t + 1.0)
        s = 0.0
        for k in range(1, 14):
            s += math.sin(TAU * fire * k * t) * (0.9 / k)
        s += 0.45 * math.sin(TAU * (fire / 2.0) * t + 0.3) + 0.2 * math.sin(TAU * (fire / 4.0) * t)
        gate = max(0.0, math.sin(TAU * fire * t)) ** 6
        out[i] = s * wob + clat[i] * gate * 0.5
    out = [0.7 * a + 0.3 * b for a, b in zip(one_pole_lowpass(out, 600.0), out)]
    return normalize(loop_crossfade(out, dur, extra), 0.6)


def make_engine_start(rng):
    dur = 3.4
    n = int(dur * SR)
    out = [0.0] * n
    cough = one_pole_lowpass(noise(rng, n), 600.0)
    body = one_pole_lowpass(noise(rng, n), 380.0)
    crank_ph = whine_ph = idle_ph = 0.0
    for i in range(n):
        t = i / SR
        if t < 1.6:
            rate = 5.0 + 4.0 * min(1.0, t / 1.5)
            crank_ph += rate / SR
            pulse = max(0.0, math.sin(TAU * crank_ph)) ** 3
            whine_ph += TAU * (180.0 + 90.0 * t) / SR
            out[i] += body[i] * pulse * 5.0 + math.sin(whine_ph) * 0.06 * min(1.0, t / 0.2)
    for t0, g in ((1.55, 1.0), (1.78, 0.7)):
        i0 = int(t0 * SR)
        for k in range(int(0.3 * SR)):
            if i0 + k < n:
                out[i0 + k] += cough[i0 + k] * math.exp(-(k / SR) / 0.09) * 9.0 * g
    for i in range(int(1.7 * SR), n):
        t = i / SR
        rate = 22.0 + 12.0 * math.exp(-(t - 1.9) / 0.35) if t > 1.9 else 30.0
        idle_ph += TAU * rate / SR
        env = min(1.0, (t - 1.7) / 0.25) * min(1.0, (dur - t) / 0.8)
        s = 0.0
        for k in range(1, 9):
            s += math.sin(k * idle_ph) * (0.9 / k)
        out[i] += s * env * 0.5
    out = [0.6 * a + 0.4 * b for a, b in zip(one_pole_lowpass(out, 900.0), out)]
    return normalize(fade(out, 0.01, 0.5), 0.75)


def make_crank(rng):
    dur = 1.2
    n = int(dur * SR)
    out = [0.0] * n
    # Rusty creak: a sweeping, scratchy squeal as the wheel turns.
    ph = 0.0
    for i in range(int(0.62 * SR)):
        t = i / SR
        u = t / 0.62
        ph += TAU * (520.0 + 260.0 * u + 30.0 * math.sin(TAU * 13.0 * t)) / SR
        out[i + int(0.03 * SR)] += (math.sin(ph) + 0.4 * math.sin(2.0 * ph)) * math.sin(math.pi * u) ** 2 * (
            0.10 * (1.0 + 0.7 * math.sin(TAU * 57.0 * t)))
    # The pawl clanks over at the top of each turn.
    for f, decay, g in ((392.0, 0.09, 0.6), (1040.0, 0.05, 0.3), (1730.0, 0.03, 0.15)):
        damped(out, 0.66, f, decay, g)
    tick = one_pole_highpass(noise(rng, int(0.03 * SR)), 2000.0)
    for start in (0.0, 0.66):
        i0 = int(start * SR)
        for k, v in enumerate(tick):
            out[i0 + k] += v * math.exp(-(k / SR) / 0.006) * 0.9
    return normalize(fade(out, 0.001, 0.02), 0.7)


def make_power_on(rng):
    n = int(2.6 * SR)
    out = [0.0] * n
    damped(out, 0.0, 70.0, 0.1, 1.0)
    click = one_pole_highpass(noise(rng, n), 1500.0)
    zap = one_pole_highpass(noise(rng, n), 3000.0)
    for i in range(int(0.05 * SR)):
        out[i] += click[i] * math.exp(-(i / SR) / 0.006) * 0.8
    for t0 in (0.28, 0.44):
        i0 = int(t0 * SR)
        for k in range(int(0.05 * SR)):
            out[i0 + k] += zap[i0 + k] * math.exp(-(k / SR) / 0.01) * 0.6
    for i in range(n):
        t = i / SR
        env = min(1.0, max(0.0, t - 0.3) / 0.6) * math.exp(-max(0.0, t - 1.0) / 0.9)
        hum = math.sin(TAU * 60.0 * t) + 0.6 * math.sin(TAU * 120.0 * t) + 0.3 * math.sin(TAU * 180.0 * t)
        buzz = 0.0
        if 0.35 < t < 1.4:
            buzz = math.sin(TAU * 3600.0 * t) * (0.5 + 0.5 * math.sin(TAU * 120.0 * t)) * 0.03 * math.sin(
                math.pi * (t - 0.35) / 1.05)
        out[i] += hum * env * 0.22 + buzz
    return normalize(fade(out, 0.001, 0.5), 0.7)


def make_susto(rng):
    n = int(1.7 * SR)
    out = [0.0] * n
    gasp = biquad_bandpass(noise(rng, n), 1700.0, 1.2)
    for i in range(int(0.3 * SR)):
        out[i] += gasp[i] * (i / SR / 0.28) ** 2 * 1.6
    for f, g in ((1975.5, 0.3), (2093.0, 0.28), (987.8, 0.18)):
        damped(out, 0.28, f, 0.4, g, length=1.3)
    damped(out, 0.28, 55.0, 0.25, 0.9, length=1.2)
    wet = reverb(out, size=1.2, damp=0.5, feedback=0.8)
    out = [d * 0.7 + w * 0.3 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.002, 0.4), 0.8)


def make_revive(rng):
    n = int(2.6 * SR)
    out = [0.0] * n
    for f, g, t0 in ((220.0, 0.2, 0.0), (329.6, 0.16, 0.1), (440.0, 0.14, 0.2), (554.4, 0.1, 0.3), (659.3, 0.07, 0.45)):
        i0 = int(t0 * SR)
        for k in range(n - i0):
            t = k / SR
            env = min(1.0, t / 0.7) * math.exp(-max(0.0, t - 0.9) / 0.8)
            out[i0 + k] += (math.sin(TAU * f * t) + 0.25 * math.sin(TAU * 2.0 * f * t)) * env * g
    br = biquad_bandpass(noise(rng, n), 1100.0, 0.9)
    for i in range(int(1.4 * SR)):
        out[i] += br[i] * math.sin(math.pi * i / (1.4 * SR)) ** 2 * 0.18
    damped(out, 0.9, 1318.5, 1.0, 0.06)
    wet = reverb(out, size=1.1, damp=0.45, feedback=0.78)
    out = [d * 0.75 + w * 0.25 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.01, 0.9), 0.6)


def make_pray(rng):
    n = int(3.0 * SR)
    src = [0.0] * n
    ph1 = ph2 = 0.0
    for i in range(n):
        t = i / SR
        vib = 1.0 + 0.008 * math.sin(TAU * 5.0 * t)
        ph1 += TAU * 147.0 * vib / SR
        ph2 += TAU * 196.0 * vib / SR
        s = 0.0
        for k in range(1, 11):
            s += (math.sin(k * ph1) + 0.8 * math.sin(k * ph2)) / k
        src[i] = s
    a = biquad_bandpass(src, 420.0, 2.0)
    b = biquad_bandpass(src, 780.0, 3.0)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        env = math.sin(math.pi * min(1.0, t / 2.6)) ** 2
        out[i] = (a[i] + 0.6 * b[i]) * env * 0.35
    for t0, f in ((0.2, 880.0), (1.1, 1174.7)):
        for ratio, decay, g in ((1.0, 1.6, 0.12), (2.76, 0.8, 0.05), (5.4, 0.4, 0.025)):
            damped(out, t0, f * ratio, decay, g)
    wet = reverb(out, size=1.5, damp=0.5, feedback=0.86)
    out = [d * 0.55 + w * 0.45 for d, w in zip(out, wet)]
    return normalize(fade(out, 0.05, 0.9), 0.55)


def make_aji(rng):
    n = int(1.0 * SR)
    out = [0.0] * n
    swish = biquad_bandpass(noise(rng, n), 3200.0, 0.7)
    for i in range(int(0.4 * SR)):
        t = i / SR
        out[i] += swish[i] * min(1.0, t / 0.05) * math.exp(-t / 0.18) * 1.4
    for _ in range(70):
        t0 = rng.uniform(0.05, 0.55)
        damped(out, t0, rng.uniform(2000.0, 5000.0), 0.003, rng.uniform(0.05, 0.3), phase=rng.uniform(0, TAU),
               length=0.02)
    sizzle = one_pole_highpass(noise(rng, n), 5500.0)
    for i in range(int(0.7 * SR)):
        t = i / SR
        out[i + int(0.1 * SR)] += sizzle[i] * math.exp(-t / 0.35) * 0.25
    return normalize(fade(out, 0.001, 0.15), 0.6)


def make_beacon(rng):
    n = int(2.4 * SR)
    out = [0.0] * n
    ph = 0.0
    for i in range(int(0.9 * SR)):
        t = i / SR
        ph += TAU * (55.0 - 20.0 * t) / SR
        out[i] += math.sin(ph) * math.exp(-t / 0.3)
    lo = biquad_bandpass(noise(rng, n), 500.0, 1.0)
    hi = biquad_bandpass(noise(rng, n), 1800.0, 1.0)
    for i in range(n):
        t = i / SR
        env = (1.0 - math.exp(-t / 0.25)) * math.exp(-max(0.0, t - 0.5) / 0.9)
        out[i] += (lo[i] * 1.3 + hi[i] * 0.9) * env
    tick = one_pole_highpass(noise(rng, n), 2500.0)
    t = 0.05
    while t < 2.2:
        t += rng.expovariate(45.0 * math.exp(-t / 1.2) + 4.0)
        i0 = int(t * SR)
        for k in range(int(0.004 * SR)):
            if i0 + k < n:
                out[i0 + k] += tick[i0 + k] * math.exp(-(k / SR) / 0.001) * rng.uniform(0.2, 0.7)
    return normalize(fade(out, 0.002, 0.5), 0.7)


def make_counting(rng):
    out = silence(2.8)
    t = 0.05
    for _ in range(6):
        for k in range(rng.randint(2, 4)):
            mix(out, clack(rng, 0.07), int((t + k * rng.uniform(0.07, 0.11)) * SR), rng.uniform(0.25, 0.6))
        t += rng.uniform(0.32, 0.5)
    return normalize(fade(out, 0.002, 0.1), 0.55)


def make_ping(rng):
    n = int(0.45 * SR)
    out = [0.0] * n
    for f, t0, g in ((988.0, 0.0, 1.0), (1318.5, 0.09, 0.8)):
        i0 = int(t0 * SR)
        for k in range(n - i0):
            t = k / SR
            out[i0 + k] += (math.sin(TAU * f * t) + 0.3 * math.sin(TAU * 2.0 * f * t)) * math.exp(-t / 0.11) * g
    return normalize(fade(out, 0.001, 0.05), 0.5)


def make_bones_set(rng):
    out = silence(0.7)
    t = 0.0
    for k in range(6):
        mix(out, clack(rng, 0.08), int(t * SR), 0.6 * (0.7 ** k))
        t += rng.uniform(0.03, 0.07) * (1.0 + 0.4 * k)
    damped(out, 0.0, 90.0, 0.06, 0.5)
    return normalize(fade(out, 0.001, 0.1), 0.55)


def make_mechanics(rng):
    files = {
        "rain_loop.wav": make_rain(rng),
        "thunder_a.wav": make_thunder(rng, 6.0, 4),
        "thunder_b.wav": make_thunder(rng, 7.2, 6),
        "heartbeat.wav": make_heartbeat(rng),
    }
    for kind in ("dirt", "grass", "wood", "water"):
        for v in range(3):
            files[f"step_{kind}_{v}.wav"] = make_step(rng, kind)
    files["cattle.wav"] = make_cattle(rng)
    files["engine_loop.wav"] = make_engine_loop(rng)
    files["engine_start.wav"] = make_engine_start(rng)
    files["pump_crank.wav"] = make_crank(rng)
    files["power_on.wav"] = make_power_on(rng)
    files["susto.wav"] = make_susto(rng)
    files["revive.wav"] = make_revive(rng)
    files["pray.wav"] = make_pray(rng)
    files["aji_scatter.wav"] = make_aji(rng)
    files["beacon_flare.wav"] = make_beacon(rng)
    files["counting.wav"] = make_counting(rng)
    files["ping.wav"] = make_ping(rng)
    files["bones_set.wav"] = make_bones_set(rng)
    return files


# --------------------------------------------------------------------------
# Skill checks: the rhythm of the work
# --------------------------------------------------------------------------

def make_check_warn(rng):
    """Two quick wooden taps and a small bright ring: the needle is coming."""
    out = silence(0.45)
    for t0, level in ((0.0, 0.8), (0.11, 1.0)):
        i0 = int(t0 * SR)
        click = biquad_bandpass(noise(rng, int(0.03 * SR)), 2400.0, 4.0)
        for k, v in enumerate(click):
            out[i0 + k] += v * math.exp(-(k / SR) / 0.006) * level * 3.0
        damped(out, t0, 1760.0, 0.09, 0.35 * level)
        damped(out, t0, 2640.0, 0.06, 0.15 * level)
    return normalize(fade(out, 0.001, 0.08), 0.6)


def make_check_great(rng):
    """A soft, satisfied knock: the work took."""
    out = silence(0.35)
    damped(out, 0.0, 180.0, 0.07, 1.0)
    damped(out, 0.0, 660.0, 0.05, 0.4)
    tap = one_pole_highpass(noise(rng, int(0.02 * SR)), 1200.0)
    for k, v in enumerate(tap):
        out[k] += v * math.exp(-(k / SR) / 0.004) * 0.6
    return normalize(fade(out, 0.001, 0.1), 0.55)


def make_check_miss(rng):
    """Iron on iron: a clank, then a long grinding screech that carries."""
    n = int(1.6 * SR)
    out = [0.0] * n
    # The clank: a few inharmonic partials struck hard.
    for f, d, g in ((310.0, 0.5, 0.8), (743.0, 0.35, 0.5), (1377.0, 0.25, 0.35), (2210.0, 0.18, 0.2)):
        damped(out, 0.0, f, d, g)
    grit = one_pole_highpass(noise(rng, n), 900.0)
    ph = [0.0, 0.0, 0.0]
    for i in range(n):
        t = i / SR
        env = min(1.0, max(0.0, t - 0.05) / 0.08) * math.exp(-max(0.0, t - 0.25) / 0.45)
        # A bowed, wavering metal squeal with stick-slip roughness.
        wob = 1.0 + 0.04 * math.sin(TAU * 7.0 * t) + 0.02 * math.sin(TAU * 23.0 * t)
        stick = 0.5 + 0.5 * math.sin(TAU * 38.0 * t + 2.0 * math.sin(TAU * 3.0 * t))
        s = 0.0
        for k, f in enumerate((1180.0, 1870.0, 2950.0)):
            ph[k] += TAU * f * wob * (1.0 - 0.18 * t) / SR
            s += math.sin(ph[k]) / (k + 1)
        out[i] += (s * (0.5 + 0.5 * stick) * 0.6 + grit[i] * 0.25 * stick) * env
    out = one_pole_lowpass(out, 6500.0)
    room = reverb(out, size=1.2, damp=0.5, feedback=0.78)
    out = [d * 0.8 + r * 0.3 for d, r in zip(out + silence(0.8), room + silence(0.8))]
    return normalize(fade(out, 0.001, 0.4), 0.85)


def make_radio(rng):
    """A late-night AM broadcast through a storm: hiss that breathes, crackle,
    a man's murmured voice you can almost follow, and now and then a cuatro
    strum from another station bleeding through. A seamless 12 s loop."""
    loop, xfade = 12.0, 1.0
    n = int((loop + xfade) * SR)
    hiss = biquad_bandpass(noise(rng, n), 2200.0, 0.6)
    out = [0.0] * n
    # Voice: a glottal buzz through two moving formants, in phrases.
    ph = 0.0
    buzz = [0.0] * n
    syll = 0.0
    phrase_on = True
    next_switch = rng.uniform(1.5, 3.0)
    for i in range(n):
        t = i / SR
        if t > next_switch:
            phrase_on = not phrase_on
            next_switch = t + (rng.uniform(1.6, 3.2) if phrase_on else rng.uniform(0.3, 0.8))
        pitch = 110.0 + 18.0 * math.sin(TAU * 0.4 * t) + 8.0 * math.sin(TAU * 2.3 * t)
        ph += TAU * pitch / SR
        syll = 0.5 + 0.5 * math.sin(TAU * 4.1 * t + 1.7 * math.sin(TAU * 0.9 * t))
        g = (1.0 if phrase_on else 0.0) * syll
        buzz[i] = (math.sin(ph) + 0.5 * math.sin(2 * ph) + 0.3 * math.sin(3 * ph) + 0.2 * math.sin(4 * ph)) * g
    lo = biquad_bandpass(buzz, 600.0, 2.0)
    mid = biquad_bandpass(buzz, 1500.0, 3.0)
    voice = [a * 0.8 + b * 0.5 for a, b in zip(lo, mid)]
    # The whole broadcast breathes in and out of the static.
    for i in range(n):
        t = i / SR
        fade_sig = 0.55 + 0.45 * math.sin(TAU * t / (loop / 2.0))
        out[i] = voice[i] * 0.5 * fade_sig + hiss[i] * (0.35 + 0.25 * (1.0 - fade_sig))
    # Crackle.
    t = 0.0
    while t < loop + xfade - 0.01:
        t += rng.expovariate(9.0)
        i0 = int(t * SR)
        amp = rng.uniform(0.2, 0.9)
        for k in range(int(0.002 * SR)):
            if i0 + k < n:
                out[i0 + k] += rng.uniform(-1.0, 1.0) * amp * math.exp(-k / (0.0005 * SR))
    # A distant cuatro strum bleeding in from another station.
    for t0 in (3.3, 8.9):
        for f in (294.0, 370.0, 440.0, 587.0):
            damped(out, t0 + rng.uniform(0.0, 0.03), f, 0.5, 0.05)
    out = one_pole_lowpass(one_pole_highpass(out, 300.0), 3400.0)
    return normalize(loop_crossfade(out, loop, xfade), 0.6)


# --------------------------------------------------------------------------
# Fright: stingers, omens and the dread bed
# --------------------------------------------------------------------------

def make_sting_caught(rng):
    """He is in your face: a shrieking cluster over a slammed low hit, the
    whistle's last note torn upward, then a ringing that will not stop."""
    n = int(2.8 * SR)
    out = [0.0] * n
    # Slam: sub drop plus a wide noise burst.
    damped(out, 0.0, 48.0, 0.35, 1.2)
    damped(out, 0.0, 96.0, 0.2, 0.6)
    burst = one_pole_lowpass(noise(rng, n), 5000.0)
    for i in range(int(0.25 * SR)):
        out[i] += burst[i] * math.exp(-(i / SR) / 0.05) * 1.1
    # Shriek: detuned high cluster with fast vibrato, rising.
    phs = [rng.uniform(0, TAU) for _ in range(6)]
    for i in range(n):
        t = i / SR
        env = min(1.0, t / 0.03) * math.exp(-max(0.0, t - 0.5) / 0.6)
        s = 0.0
        for k in range(6):
            f = (1480.0 * (1.0 + 0.012 * (k - 2.5))) * (1.0 + 0.35 * min(1.0, t / 0.6))
            f *= 1.0 + 0.02 * math.sin(TAU * (11.0 + k) * t)
            phs[k] += TAU * f / SR
            s += math.sin(phs[k])
        out[i] += s / 6.0 * env * 0.7
        # Tinnitus: a thin sine that lingers.
        out[i] += math.sin(TAU * 6100.0 * t) * 0.05 * min(1.0, t / 0.3) * math.exp(-max(0.0, t - 1.2) / 0.9)
    room = reverb(out, size=1.1, damp=0.4, feedback=0.75)
    out = [d * 0.85 + r * 0.25 for d, r in zip(out + silence(0.5), room + silence(0.5))]
    return normalize(fade(out, 0.0005, 0.6), 0.95)


def make_sting_reveal(rng):
    """Lightning shows him: a bowed string scrape into a stab of low brass."""
    n = int(2.2 * SR)
    out = [0.0] * n
    grit = biquad_bandpass(noise(rng, n), 900.0, 1.5)
    ph = [0.0, 0.0, 0.0]
    for i in range(n):
        t = i / SR
        env = min(1.0, t / 0.015) * math.exp(-t / 0.7)
        stick = 0.5 + 0.5 * math.sin(TAU * 31.0 * t)
        s = 0.0
        for k, f in enumerate((73.4, 110.0, 146.8)):
            ph[k] += TAU * f / SR
            # A buzzing, brassy tone: a clipped sine.
            s += max(-0.6, min(0.6, 1.8 * math.sin(ph[k]))) / (k + 1)
        out[i] += s * env * 0.6 + grit[i] * env * stick * 0.5
    room = reverb(out, size=1.3, damp=0.5, feedback=0.8)
    out = [d * 0.8 + r * 0.3 for d, r in zip(out + silence(0.6), room + silence(0.6))]
    return normalize(fade(out, 0.0005, 0.5), 0.85)


def make_sting_phantom(rng):
    """Something at the edge of sight: an indrawn breath and a thin high tone."""
    n = int(1.8 * SR)
    out = [0.0] * n
    breath = biquad_bandpass(noise(rng, n), 1600.0, 0.8)
    for i in range(n):
        t = i / SR
        inhale = math.sin(math.pi * min(1.0, t / 0.6)) if t < 0.6 else 0.0
        tone_env = min(1.0, max(0.0, t - 0.2) / 0.15) * math.exp(-max(0.0, t - 0.4) / 0.5)
        out[i] = breath[i] * inhale * 0.7 + math.sin(TAU * 2350.0 * t + 0.3 * math.sin(TAU * 6.0 * t)) * tone_env * 0.25
    room = reverb(out, size=1.4, damp=0.6, feedback=0.82)
    out = [d * 0.6 + r * 0.5 for d, r in zip(out + silence(0.6), room + silence(0.6))]
    return normalize(fade(out, 0.002, 0.5), 0.6)


def make_sting_hunt(rng):
    """He commits: a short, hard double hit and a rising screech of strings."""
    n = int(1.6 * SR)
    out = [0.0] * n
    for t0, g in ((0.0, 1.0), (0.18, 0.8)):
        damped(out, t0, 55.0, 0.25, g)
        i0 = int(t0 * SR)
        hit = one_pole_lowpass(noise(rng, int(0.08 * SR)), 2500.0)
        for k, v in enumerate(hit):
            out[i0 + k] += v * math.exp(-(k / SR) / 0.02) * g * 0.8
    ph = 0.0
    for i in range(n):
        t = i / SR
        env = min(1.0, max(0.0, t - 0.2) / 0.5) * math.exp(-max(0.0, t - 0.9) / 0.3)
        ph += TAU * (600.0 + 700.0 * min(1.0, t / 1.0)) / SR
        out[i] += (math.sin(ph) + 0.4 * math.sin(2.01 * ph)) * env * 0.25
    return normalize(fade(out, 0.0005, 0.3), 0.85)


def make_omen_bones(rng):
    """Bones clattering somewhere off in the dark: far, dull and wet with room."""
    out = silence(3.2)
    t = 0.1
    for _ in range(rng.randint(5, 8)):
        mix(out, clack(rng, 0.06), int(t * SR), rng.uniform(0.2, 0.6))
        t += rng.uniform(0.06, 0.28)
    out = one_pole_lowpass(out, 2200.0)
    room = reverb(out, size=1.5, damp=0.6, feedback=0.85)
    out = [d * 0.35 + r * 0.8 for d, r in zip(out, room)]
    return normalize(fade(out, 0.005, 0.8), 0.5)


def make_omen_lamps(rng):
    """The lamps die: the mains hum sags, fizzes and goes out."""
    n = int(2.4 * SR)
    out = [0.0] * n
    zap = one_pole_highpass(noise(rng, n), 2800.0)
    for i in range(n):
        t = i / SR
        sag = max(0.0, 1.0 - t / 1.8)
        f = 60.0 * (0.8 + 0.2 * sag)
        hum = math.sin(TAU * f * t) + 0.5 * math.sin(TAU * 2 * f * t)
        flick = 1.0 if math.sin(TAU * 7.0 * t + 3.0 * math.sin(TAU * 1.3 * t)) > -0.2 else 0.3
        out[i] = hum * sag * flick * 0.35 + zap[i] * (0.15 if flick < 1.0 else 0.02) * sag
    return normalize(fade(out, 0.01, 0.3), 0.5)


def make_omen_swell(rng):
    """A slow, low swell under everything: something has changed nearby."""
    n = int(4.0 * SR)
    out = [0.0] * n
    rumble = one_pole_lowpass(noise(rng, n), 180.0)
    ph = [0.0, 0.0]
    for i in range(n):
        t = i / SR
        env = math.sin(math.pi * min(1.0, t / 4.0)) ** 2
        for k, f in enumerate((41.2, 61.7)):
            ph[k] += TAU * f * (1.0 + 0.004 * math.sin(TAU * 0.3 * t)) / SR
        out[i] = (math.sin(ph[0]) + 0.6 * math.sin(ph[1]) + rumble[i] * 3.0) * env
    return normalize(fade(out, 0.2, 0.5), 0.55)


def make_dread(rng):
    """The dread bed: a seamless 16 s low drone of beating fifths and air,
    mixed up as the night, the rite and the danger grow."""
    loop, xfade = 16.0, 2.0
    n = int((loop + xfade) * SR)
    air = one_pole_lowpass(noise(rng, n), 400.0)
    out = [0.0] * n
    ph = [0.0, 0.0, 0.0, 0.0]
    freqs = (36.7, 55.0, 55.4, 82.6)
    for i in range(n):
        t = i / SR
        s = 0.0
        for k, f in enumerate(freqs):
            ph[k] += TAU * f * (1.0 + 0.003 * math.sin(TAU * (0.05 + 0.02 * k) * t)) / SR
            s += math.sin(ph[k]) * (0.9 if k == 0 else 0.5)
        swell = 0.75 + 0.25 * math.sin(TAU * t / 8.0)
        out[i] = s * swell * 0.4 + air[i] * 2.0
    return normalize(loop_crossfade(out, loop, xfade), 0.5)


def make_lock_rattle(rng):
    """A padlock yanked against its hasp: quick metal knocks and a shackle ring."""
    out = silence(0.9)
    t = 0.0
    for k in range(rng.randint(3, 5)):
        i0 = int(t * SR)
        tick = one_pole_highpass(noise(rng, int(0.02 * SR)), 2500.0)
        for j, v in enumerate(tick):
            out[i0 + j] += v * math.exp(-(j / SR) / 0.004) * rng.uniform(0.5, 1.0)
        damped(out, t, rng.uniform(2800.0, 3400.0), 0.06, 0.25)
        damped(out, t, rng.uniform(1100.0, 1300.0), 0.08, 0.2)
        t += rng.uniform(0.06, 0.12)
    return normalize(fade(out, 0.001, 0.2), 0.7)


def make_lock_open(rng):
    """The shackle springs free and a ring of keys jingles."""
    out = silence(1.3)
    damped(out, 0.0, 900.0, 0.05, 0.6)
    click = one_pole_highpass(noise(rng, int(0.015 * SR)), 2000.0)
    for j, v in enumerate(click):
        out[j] += v * math.exp(-(j / SR) / 0.003)
    t = 0.12
    for _ in range(9):
        damped(out, t, rng.uniform(3500.0, 6200.0), rng.uniform(0.04, 0.09), rng.uniform(0.1, 0.3))
        t += rng.uniform(0.03, 0.09)
    return normalize(fade(out, 0.001, 0.3), 0.65)


def make_weeping(rng):
    """A grown man weeping, far off across the llano: breathy, broken sobs
    over a low voiced moan, heavy with room."""
    n = int(5.0 * SR)
    out = [0.0] * n
    breath = biquad_bandpass(noise(rng, n), 900.0, 1.2)
    ph = 0.0
    t = 0.2
    sobs = []
    while t < 4.4:
        sobs.append((t, rng.uniform(0.18, 0.4), rng.uniform(0.6, 1.0)))
        t += rng.uniform(0.35, 0.8)
    for i in range(n):
        tt = i / SR
        env = 0.0
        for (s0, d, g) in sobs:
            if s0 <= tt < s0 + d:
                u = (tt - s0) / d
                env = max(env, g * math.sin(math.pi * u) ** 0.6)
        moan_env = 0.35 * math.sin(math.pi * min(1.0, tt / 4.8))
        pitch = 150.0 * (1.0 - 0.12 * min(1.0, tt / 4.5)) * (1.0 + 0.03 * math.sin(TAU * 5.5 * tt))
        ph += TAU * pitch / SR
        voice = math.sin(ph) + 0.5 * math.sin(2 * ph) + 0.25 * math.sin(3 * ph)
        out[i] = voice * (moan_env + 0.6 * env) * 0.5 + breath[i] * env * 0.9
    out = biquad_bandpass(out, 700.0, 0.7)
    room = reverb(out, size=1.5, damp=0.6, feedback=0.86)
    out = [d * 0.3 + r * 0.8 for d, r in zip(out + silence(1.0), room + silence(1.0))]
    return normalize(fade(out, 0.1, 1.0), 0.5)


def make_whip(rng):
    """A bullwhip cracking far away in the dark: a sharp report and echoes."""
    out = silence(2.6)
    crack = one_pole_highpass(noise(rng, int(0.012 * SR)), 1500.0)
    for j, v in enumerate(crack):
        out[j] += v * math.exp(-(j / SR) / 0.002) * 2.0
    swish = biquad_bandpass(noise(rng, int(0.15 * SR)), 3000.0, 1.0)
    for j, v in enumerate(swish):
        k = int(0.0 * SR) + j
        out[k] += v * math.sin(math.pi * j / len(swish)) * 0.3
    out = one_pole_lowpass(out, 5000.0)
    room = reverb(out, size=1.6, damp=0.5, feedback=0.86)
    out = [d * 0.5 + r * 0.9 for d, r in zip(out, room)]
    return normalize(fade(out, 0.0005, 0.6), 0.6)


def make_bottles(rng):
    """Glass on glass out in the grass: two bottles knocking, one rolling."""
    out = silence(2.0)
    t = 0.05
    for _ in range(rng.randint(3, 5)):
        for f, d, g in ((rng.uniform(2200, 2600), 0.25, 0.4), (rng.uniform(3900, 4400), 0.15, 0.25), (rng.uniform(5800, 6400), 0.1, 0.15)):
            damped(out, t, f, d, g)
        t += rng.uniform(0.12, 0.35)
    out = one_pole_lowpass(out, 5500.0)
    room = reverb(out, size=1.3, damp=0.5, feedback=0.8)
    out = [d * 0.5 + r * 0.7 for d, r in zip(out, room)]
    return normalize(fade(out, 0.001, 0.5), 0.45)


def make_banished(rng):
    """He is laid to rest: the whistle's last note unwinding downward into a
    long, warm, settling chord."""
    n = int(6.0 * SR)
    out = [0.0] * n
    ph = 0.0
    for i in range(int(2.2 * SR)):
        t = i / SR
        f = 1480.0 * (1.0 - 0.55 * min(1.0, t / 2.0))
        ph += TAU * f / SR
        out[i] += math.sin(ph) * 0.3 * math.exp(-t / 1.2)
    for f, g in ((110.0, 0.5), (164.8, 0.35), (220.0, 0.3), (277.2, 0.2), (329.6, 0.15)):
        for i in range(int(1.2 * SR), n):
            t = (i - int(1.2 * SR)) / SR
            env = min(1.0, t / 1.5) * math.exp(-max(0.0, t - 2.0) / 1.6)
            out[i] += math.sin(TAU * f * t) * g * env * 0.5
    room = reverb(out, size=1.4, damp=0.5, feedback=0.82)
    out = [d * 0.7 + r * 0.4 for d, r in zip(out, room)]
    return normalize(fade(out, 0.01, 1.0), 0.7)


def make_growl(rng):
    """A dog's low growl: a rough, pulsing rumble through a snarl formant."""
    n = int(2.2 * SR)
    out = [0.0] * n
    ph = 0.0
    rasp = one_pole_lowpass(noise(rng, n), 900.0)
    for i in range(n):
        t = i / SR
        env = min(1.0, t / 0.25) * math.exp(-max(0.0, t - 1.6) / 0.25)
        f = 85.0 + 10.0 * math.sin(TAU * 3.0 * t)
        ph += TAU * f / SR
        pulse = 0.55 + 0.45 * math.sin(TAU * 26.0 * t + math.sin(TAU * 7.0 * t))
        buzz = math.sin(ph) + 0.6 * math.sin(2 * ph) + 0.4 * math.sin(3 * ph) + 0.25 * math.sin(4 * ph)
        out[i] = (buzz * 0.4 + rasp[i] * 3.0) * pulse * env
    out = biquad_bandpass(out, 420.0, 0.8)
    return normalize(fade(out, 0.02, 0.2), 0.65)


def make_bark(rng):
    """Two hard barks, close, with the yard throwing them back."""
    out = silence(1.8)
    for t0 in (0.0, 0.33):
        n = int(0.22 * SR)
        ph = 0.0
        rasp = biquad_bandpass(noise(rng, n), 1200.0, 1.0)
        for j in range(n):
            t = j / SR
            env = min(1.0, t / 0.01) * math.exp(-t / 0.07)
            f = 420.0 * (1.0 + 0.5 * math.exp(-t / 0.03)) * (1.0 - 0.3 * t / 0.22)
            ph += TAU * f / SR
            v = math.sin(ph) + 0.7 * math.sin(2 * ph) + 0.4 * math.sin(3 * ph)
            out[int(t0 * SR) + j] += (v * 0.5 + rasp[j] * 1.5) * env
    room = reverb(out, size=0.9, damp=0.5, feedback=0.7)
    out = [d * 0.85 + r * 0.3 for d, r in zip(out, room)]
    return normalize(fade(out, 0.001, 0.3), 0.8)


def pluck(rng, freq, length, decay=0.996, bright=0.5):
    """One plucked string (Karplus-Strong), the cuatro's voice."""
    period = max(2, int(SR / freq))
    buf = [rng.uniform(-1.0, 1.0) for _ in range(period)]
    # A softer pick: smooth the initial burst a little.
    for i in range(1, period):
        buf[i] = buf[i] * bright + buf[i - 1] * (1.0 - bright)
    out = [0.0] * int(length * SR)
    for i in range(len(out)):
        j = i % period
        nxt = buf[(j + 1) % period]
        v = buf[j]
        out[i] = v
        buf[j] = (v + nxt) * 0.5 * decay
    return out


def make_theme(rng):
    """The title's night: a slow, out-of-tune cuatro strummed alone on a
    porch, over a low drone and wind; a seamless 48 s loop."""
    loop, xfade = 48.0, 4.0
    n = int((loop + xfade) * SR)
    out = [0.0] * n
    # Four strings each, as a cuatro voices them (B minor, G, E minor, F#).
    chords = [
        (185.0, 246.9, 293.7, 370.0),
        (196.0, 246.9, 293.7, 392.0),
        (164.8, 246.9, 329.6, 392.0),
        (185.0, 233.1, 277.2, 370.0),
    ]
    beat = 6.0
    for k in range(int((loop + xfade) / beat)):
        chord = chords[k % len(chords)]
        t0 = k * beat + 0.4
        # A slow downward strum, each string slightly flat and late.
        for s, f in enumerate(chord):
            f *= 1.0 - 0.006 * rng.random()
            string = pluck(rng, f, 5.5, decay=0.9965, bright=0.45)
            mix(out, fade(string, 0.002, 1.5), int((t0 + s * 0.045 + 0.01 * rng.random()) * SR), 0.22)
        # Now and then one high note alone, answered by nothing.
        if k % 2 == 1:
            f = chord[3] * (2.0 if k % 4 == 1 else 1.5)
            note = pluck(rng, f, 3.5, decay=0.995, bright=0.6)
            mix(out, fade(note, 0.002, 1.0), int((t0 + 3.1) * SR), 0.14)
    # A low drone breathing under it, and the wind.
    wind = one_pole_lowpass(noise(rng, n), 260.0)
    ph = 0.0
    for i in range(n):
        t = i / SR
        ph += TAU * 61.7 * (1.0 + 0.002 * math.sin(TAU * 0.07 * t)) / SR
        swell = 0.6 + 0.4 * math.sin(TAU * t / 12.0)
        out[i] += math.sin(ph) * 0.05 * swell + wind[i] * 0.9 * (0.5 + 0.5 * math.sin(TAU * t / 17.0 + 1.0))
    wet = reverb(out, size=1.3, damp=0.45, feedback=0.84)
    out = [d * 0.7 + w * 0.6 for d, w in zip(out, wet)]
    return normalize(loop_crossfade(out, loop, xfade), 0.45)


def make_frogs(rng):
    """A 12 s seamless loop of frogs at the water's edge: a few voices, each
    croaking in its own slow rhythm, close enough to place."""
    loop, xfade = 12.0, 1.5
    n = int((loop + xfade) * SR)
    out = [0.0] * n
    voices = [(420.0, 1.3), (610.0, 0.9), (330.0, 1.7), (780.0, 0.7)]
    for base, every in voices:
        t = rng.uniform(0.0, every)
        while t < loop + xfade - 0.5:
            f = base * rng.uniform(0.94, 1.06)
            # A croak: a few fast pulses of a buzzy, nasal tone.
            pulses = rng.randint(2, 5)
            for k in range(pulses):
                start = int((t + k * 0.055) * SR)
                length = int(0.04 * SR)
                for i in range(length):
                    j = start + i
                    if j >= n:
                        break
                    u = i / length
                    env = math.sin(math.pi * u) ** 1.5
                    ph = TAU * f * i / SR
                    v = math.sin(ph) + 0.5 * math.sin(2.0 * ph) + 0.3 * math.sin(3.0 * ph)
                    out[j] += v * env * 0.25
            t += every * rng.uniform(0.6, 1.6)
    wet = reverb(out, size=0.8, damp=0.5, feedback=0.7)
    out = [d * 0.8 + w * 0.2 for d, w in zip(out, wet)]
    return normalize(loop_crossfade(out, loop, xfade), 0.6)


def make_windmill(rng):
    """A 9 s seamless loop: the windmill's vane creaking round on a dry
    bearing, a tail rattle, the wind in the blades."""
    loop, xfade = 9.0, 1.0
    n = int((loop + xfade) * SR)
    out = [0.0] * n
    wind = one_pole_lowpass(noise(rng, n), 900.0)
    for i in range(n):
        t = i / SR
        out[i] = wind[i] * 0.5 * (0.6 + 0.4 * math.sin(TAU * t / 4.5))
    # Creaks: a squeal of rubbing metal, once or twice a turn.
    t = 0.4
    while t < loop + xfade - 1.0:
        f0 = rng.uniform(380.0, 520.0)
        length = rng.uniform(0.35, 0.7)
        start = int(t * SR)
        ph = 0.0
        for i in range(int(length * SR)):
            j = start + i
            if j >= n:
                break
            u = i / (length * SR)
            f = f0 * (1.0 + 0.25 * math.sin(math.pi * u)) + 30.0 * math.sin(TAU * 23.0 * i / SR)
            ph += TAU * f / SR
            env = math.sin(math.pi * u) ** 0.7
            v = math.sin(ph) + 0.6 * math.sin(2.02 * ph) + 0.35 * math.sin(3.05 * ph)
            out[j] += v * env * 0.22
        t += rng.uniform(2.0, 3.4)
    # The tail rattling in its bracket.
    for k in range(int((loop + xfade) / 0.9)):
        start = int((k * 0.9 + rng.uniform(0.0, 0.3)) * SR)
        mix(out, clack(rng, 0.05), start, 0.25)
    wet = reverb(out, size=1.0, damp=0.45, feedback=0.75)
    out = [d * 0.75 + w * 0.25 for d, w in zip(out, wet)]
    return normalize(loop_crossfade(out, loop, xfade), 0.6)


def make_hum(rng):
    """A 4 s seamless loop: the dynamo turning and the line humming once the
    power is back (mains-like 60 Hz and its harmonics, a whine and a tick)."""
    loop, xfade = 4.0, 0.5
    n = int((loop + xfade) * SR)
    air = one_pole_lowpass(noise(rng, n), 3000.0)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        s = 0.0
        for k, g in ((1, 0.6), (2, 0.35), (3, 0.25), (5, 0.12)):
            s += math.sin(TAU * 60.0 * k * t) * g
        whine = math.sin(TAU * 1240.0 * t + 2.0 * math.sin(TAU * 0.5 * t)) * 0.03
        wobble = 0.85 + 0.15 * math.sin(TAU * 2.0 * t)
        out[i] = s * wobble * 0.5 + whine + air[i] * 0.04
    return normalize(loop_crossfade(out, loop, xfade), 0.5)


def make_ear_whistle(rng):
    """The whistle as it has never been heard all night: right in your ear.
    Two quick breathy notes and the held note tearing upward into a shriek,
    dry, overdriven, cut off dead. Only when he has you."""
    n = int(0.62 * SR)
    out = [0.0] * n
    breath = one_pole_lowpass(one_pole_highpass(noise(rng, n), 1200.0), 9000.0)
    ph = 0.0
    for i in range(n):
        t = i / SR
        if t < 0.1:
            f, amp = 1244.0, min(1.0, t / 0.01)
        elif t < 0.2:
            f, amp = 1396.0, 1.0
        else:
            u = (t - 0.2) / 0.42
            f = 1480.0 * (1.0 + 1.4 * u * u)
            amp = 1.0 + 0.4 * u
        f *= 1.0 + 0.012 * math.sin(TAU * 7.0 * t)
        ph += TAU * f / SR
        tone = math.sin(ph) + 0.25 * math.sin(2.0 * ph) + 0.1 * math.sin(3.0 * ph)
        v = tone * amp * 0.8 + breath[i] * amp * 0.9
        out[i] = math.tanh(2.2 * v)
    # Cut off dead: no tail, no room. Only the shortest click-free release.
    for i in range(int(0.004 * SR)):
        out[n - 1 - i] *= i / (0.004 * SR)
    return normalize(out, 0.95)


def make_ringing(rng):
    """After the blow: the ears ring. A thud, then a high whine that beats
    against itself and fades over about three seconds."""
    n = int(3.2 * SR)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        thud = math.sin(TAU * 48.0 * t) * math.exp(-t / 0.18) * 0.9
        ring = (math.sin(TAU * 3150.0 * t) + 0.6 * math.sin(TAU * 3163.0 * t)) * 0.18
        ring_env = min(1.0, t / 0.25) * math.exp(-max(0.0, t - 0.4) / 1.1)
        out[i] = thud + ring * ring_env
    rumble = one_pole_lowpass(noise(rng, n), 200.0)
    out = [v + r * 0.6 * math.exp(-i / SR / 0.5) for i, (v, r) in enumerate(zip(out, rumble))]
    return normalize(fade(out, 0.002, 0.3), 0.8)


def make_groan(rng):
    """A fallen friend groaning in the grass, close by: a short voiced moan
    that swells and sags, catching in the throat, with breath through the
    teeth; dry, with only a trace of room. The game pitches it per survivor."""
    length = rng.uniform(0.85, 1.35)
    n = int((length + 0.3) * SR)
    out = [0.0] * n
    breath = biquad_bandpass(noise(rng, n), rng.uniform(900.0, 1300.0), 1.1)
    f0 = rng.uniform(125.0, 150.0)
    sag = rng.uniform(0.12, 0.22)
    onset = rng.uniform(0.05, 0.12)
    catch = rng.uniform(24.0, 34.0)
    ph = 0.0
    for i in range(n):
        t = i / SR
        u = min(1.0, t / length)
        env = min(1.0, t / onset) * (1.0 - u) ** 0.8
        pitch = f0 * (1.0 + 0.08 * math.sin(math.pi * u) - sag * u * u) * (1.0 + 0.02 * math.sin(TAU * 6.0 * t))
        ph += TAU * pitch / SR
        voice = math.sin(ph) + 0.55 * math.sin(2 * ph) + 0.3 * math.sin(3 * ph) + 0.15 * math.sin(4 * ph)
        rasp = 0.6 + 0.4 * math.sin(TAU * catch * t + math.sin(TAU * 4.0 * t))
        out[i] = voice * env * rasp * 0.5 + breath[i] * env * 0.35
    out = biquad_bandpass(out, 600.0, 0.6)
    room = reverb(out, size=0.6, damp=0.6, feedback=0.6)
    out = [d * 0.9 + r * 0.15 for d, r in zip(out, room)]
    return normalize(fade(out, 0.01, 0.15), 0.6)


def hoarse_syllable(rng, length, f_start, f_end, formants, hoarse):
    """One hoarse syllable: a buzzing, breathy voice gliding from f_start to
    f_end through its vowel's resonances (centre, Q, gain)."""
    n = int(length * SR)
    src = [0.0] * n
    air = noise(rng, n)
    ph = 0.0
    for i in range(n):
        t = i / SR
        f = (f_start + (f_end - f_start) * t / length) * (1.0 + 0.015 * math.sin(TAU * 7.0 * t))
        ph += TAU * f * (1.0 + rng.uniform(-0.01, 0.01)) / SR
        buzz = sum(math.sin(k * ph) / k for k in range(1, 12))
        env = max(0.0, min(1.0, t / 0.03) * min(1.0, (length - t) / 0.08))
        src[i] = (buzz * (1.0 - hoarse) + air[i] * hoarse * 1.6) * env
    out = [0.0] * n
    for centre, q, gain in formants:
        mix(out, biquad_bandpass(src, centre, q), 0, gain)
    return out


def make_call_help(rng):
    """"¡Auxilio!" from someone lying in the grass: three hoarse syllables,
    the middle one strained high, more a cry than words, with a little room.
    The game pitches it per survivor."""
    out = silence(1.9)
    vowel_a = [(800.0, 4.0, 1.0), (1200.0, 5.0, 0.6)]
    vowel_i = [(320.0, 4.0, 1.0), (2300.0, 6.0, 0.5)]
    vowel_o = [(500.0, 4.0, 1.0), (900.0, 5.0, 0.6)]
    t = 0.05
    mix(out, hoarse_syllable(rng, 0.22, 190.0, 210.0, vowel_a, 0.35), int(t * SR), 0.8)
    t += 0.24
    hiss = biquad_bandpass(noise(rng, int(0.09 * SR)), 3500.0, 1.5)
    mix(out, fade(hiss, 0.01, 0.03), int(t * SR), 0.5)
    t += 0.08
    mix(out, hoarse_syllable(rng, 0.42, 300.0, 270.0, vowel_i, 0.45), int(t * SR), 1.0)
    t += 0.44
    mix(out, hoarse_syllable(rng, 0.38, 220.0, 150.0, vowel_o, 0.55), int(t * SR), 0.7)
    room = reverb(out, size=0.9, damp=0.5, feedback=0.7)
    out = [d * 0.85 + r * 0.25 for d, r in zip(out, room)]
    return normalize(fade(out, 0.005, 0.4), 0.7)


def radio_tone(seconds, freq, rng):
    """A numbers-station tone through a small speaker: a sine with a little
    second harmonic, a touch of hiss, band-limited like AM."""
    n = int(seconds * SR)
    hiss = noise(rng, n)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        out[i] = math.sin(TAU * freq * t) + 0.25 * math.sin(TAU * 2.0 * freq * t) + 0.05 * hiss[i]
    out = one_pole_lowpass(one_pole_highpass(out, 300.0), 3400.0)
    return fade(out, 0.006, 0.02)


def make_pip_short(rng):
    """One short pip of the numbers station (the radio plays one per unit)."""
    return normalize(radio_tone(0.12, 1000.0, rng), 0.5)


def make_pip_long(rng):
    """The long tone that reads a zero."""
    return normalize(radio_tone(1.2, 1000.0, rng), 0.5)


def make_pip_ident(rng):
    """The station's ident that opens each reading: three falling notes."""
    out = silence(1.6)
    for k, f in enumerate((1318.5, 1174.7, 880.0)):
        mix(out, radio_tone(0.42, f, rng), int(k * 0.5 * SR), 0.9)
    return normalize(out, 0.45)


def make_radio_static(rng):
    """An empty stop on the dial: AM hiss that breathes, and crackle. A
    seamless 6 s loop; also the burst when lightning swallows a digit."""
    loop, xfade = 6.0, 0.5
    n = int((loop + xfade) * SR)
    hiss = biquad_bandpass(noise(rng, n), 1800.0, 0.5)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        out[i] = hiss[i] * (0.6 + 0.3 * math.sin(TAU * t / 3.0) + 0.1 * math.sin(TAU * 0.7 * t))
    t = 0.0
    while t < loop + xfade - 0.01:
        t += rng.expovariate(14.0)
        i0 = int(t * SR)
        amp = rng.uniform(0.3, 1.0)
        for k in range(int(0.003 * SR)):
            if i0 + k < n:
                out[i0 + k] += rng.uniform(-1.0, 1.0) * amp * math.exp(-k / (0.0006 * SR))
    out = one_pole_lowpass(one_pole_highpass(out, 250.0), 3600.0)
    return normalize(loop_crossfade(out, loop, xfade), 0.5)


def make_dial_squeal(rng):
    """The dial turned: a heterodyne whine sweeping through the stations,
    with a clunk of the knob."""
    n = int(0.9 * SR)
    out = [0.0] * n
    ph = 0.0
    hiss = noise(rng, n)
    for i in range(n):
        t = i / SR
        f = 2600.0 - 1900.0 * (t / 0.9) + 300.0 * math.sin(TAU * 5.0 * t)
        ph += TAU * f / SR
        env = min(1.0, t / 0.02) * max(0.0, 1.0 - t / 0.9)
        out[i] = (math.sin(ph) * 0.7 + hiss[i] * 0.3) * env
    damped(out, 0.0, 180.0, 0.03, 0.6)
    out = one_pole_lowpass(one_pole_highpass(out, 200.0), 4000.0)
    return normalize(fade(out, 0.002, 0.08), 0.55)


def make_radio_numbers():
    """La Voz del Llano (seeds 9100-9199): the numbers station and the dial."""
    return {
        "radio_pip.wav": make_pip_short(random.Random(9100)),
        "radio_pip_long.wav": make_pip_long(random.Random(9101)),
        "radio_ident.wav": make_pip_ident(random.Random(9102)),
        "radio_static.wav": make_radio_static(random.Random(9103)),
        "radio_squeal.wav": make_dial_squeal(random.Random(9104)),
    }


def make_horror():
    """Sounds added after the first two streams, each on its own seed so
    adding or changing one never changes another."""
    return {
        "check_warn.wav": make_check_warn(random.Random(SEED + 101)),
        "check_great.wav": make_check_great(random.Random(SEED + 102)),
        "check_miss.wav": make_check_miss(random.Random(SEED + 103)),
        "radio_broadcast.wav": make_radio(random.Random(SEED + 104)),
        "sting_caught.wav": make_sting_caught(random.Random(SEED + 105)),
        "sting_reveal.wav": make_sting_reveal(random.Random(SEED + 106)),
        "sting_phantom.wav": make_sting_phantom(random.Random(SEED + 107)),
        "sting_hunt.wav": make_sting_hunt(random.Random(SEED + 108)),
        "omen_bones.wav": make_omen_bones(random.Random(SEED + 109)),
        "omen_lamps.wav": make_omen_lamps(random.Random(SEED + 110)),
        "omen_swell.wav": make_omen_swell(random.Random(SEED + 111)),
        "dread_drone.wav": make_dread(random.Random(SEED + 112)),
        "lock_rattle.wav": make_lock_rattle(random.Random(SEED + 113)),
        "lock_open.wav": make_lock_open(random.Random(SEED + 114)),
        "tell_weeping.wav": make_weeping(random.Random(SEED + 115)),
        "tell_whip.wav": make_whip(random.Random(SEED + 116)),
        "tell_bottles.wav": make_bottles(random.Random(SEED + 117)),
        "banished.wav": make_banished(random.Random(SEED + 118)),
        "dog_growl.wav": make_growl(random.Random(SEED + 119)),
        "dog_bark.wav": make_bark(random.Random(SEED + 120)),
        "title_theme.wav": make_theme(random.Random(SEED + 121)),
        "frogs_loop.wav": make_frogs(random.Random(SEED + 122)),
        "windmill_creak.wav": make_windmill(random.Random(SEED + 123)),
        "dynamo_hum.wav": make_hum(random.Random(SEED + 124)),
        "whistle_ear.wav": make_ear_whistle(random.Random(SEED + 125)),
        "ringing.wav": make_ringing(random.Random(SEED + 126)),
        "downed_groan_0.wav": make_groan(random.Random(SEED + 127)),
        "downed_groan_1.wav": make_groan(random.Random(SEED + 128)),
        "downed_groan_2.wav": make_groan(random.Random(SEED + 129)),
        "call_help.wav": make_call_help(random.Random(SEED + 130)),
    }


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", default=os.path.join(here, "..", "assets", "audio"))
    parser.add_argument("--only-whistles", action="store_true", help="regenerate only the whistle takes")
    parser.add_argument("--synth", action="store_true",
                        help="build the whistles from the synthesized fallback, as if the recording were "
                             "absent (needs --out: never overwrites the shipped whistles)")
    args = parser.parse_args()
    if args.synth:
        shipped = os.path.normcase(os.path.realpath(os.path.join(here, "..", "assets", "audio")))
        if os.path.normcase(os.path.realpath(args.out)) == shipped:
            parser.error("--synth needs --out DIR outside assets/audio (delete assets/audio/source/ to ship the synth)")
        use_synth()
    os.makedirs(args.out, exist_ok=True)
    if args.only_whistles:
        write_all(args.out, make_whistles())
        return

    rng = random.Random(SEED)
    files = {}
    skip_old_whistles(rng)
    files.update(make_whistles())
    files["ambience_llano.wav"] = make_ambience(rng)
    files["bones_rattle.wav"] = make_bones(rng)
    files["restitution.wav"] = make_restitution(rng)
    files["caught.wav"] = make_caught(rng)
    files["dawn.wav"] = make_dawn(rng)
    # A second, independent stream: adding sounds never changes the older files.
    files.update(make_mechanics(random.Random(SEED + 1)))
    files.update(make_horror())
    files.update(make_radio_numbers())

    write_all(args.out, files)


def write_all(out, files):
    for name, samples in files.items():
        path = os.path.join(out, name)
        write_wav(path, samples)
        print(f"wrote {os.path.relpath(path)}  {len(samples) / SR:5.2f}s")


if __name__ == "__main__":
    main()

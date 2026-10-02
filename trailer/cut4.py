#!/usr/bin/env python3
"""Cut one of the three El Silbón trailers of 2026-10-01 (`script_v4.md`).

  --cut radio    A. «La Voz del Llano»: the tale from the shelf radio
  --cut coop     B. «Nadie se queda atrás»: four friends, cards on the beat
  --cut teaser   C. «Si lo oyes lejos»: the rule, shown (add --vertical for 9:16)

  --lang en|es   the narration, cards and subtitles
  --frames DIR   the rendered shots (`el_silbon --trailer --shots X` writes
                 X/trailer/<NN_shot>/; TRAILER_VERTICAL=1 writes
                 X/trailer_vertical/)
  --stems DIR    this cut's generated sound (trailer/out/stems4)
  --old-stems    trailer 3's stems (the hits and the clean far whistles)
  --out FILE     the finished trailer (.mp4)

The game's own sounds come from assets/audio. Machinery: `cutkit.py`.
"""

import argparse
import os
import shutil
import tempfile

import cutkit as K
from cutkit import BONE, DIM, GOLD, ITALIC, PALE, RED, SERIF, spaced

LANG = 0  # 0 English, 1 Spanish


def w(en, es):
    return (en, es)[LANG]


def big(text, size=50, color=BONE, dy=0, delay=0.25):
    return (spaced(text), SERIF, size, color, dy, delay)


def title_card(refrain_2):
    return {"fade_in": 0.2, "fade_out": 1.0, "lines": [
        (spaced("EL SILBÓN"), SERIF, 136, BONE, -90, 1.2),
        (spaced("THE RETURN"), SERIF, 36, GOLD, 30, 1.6),
        ("Si lo oyes lejos, ya está aquí.", ITALIC, 36, "0xD8D0C0", 150, 3.0),
        (refrain_2, ITALIC, 24, DIM, 200, 3.6),
    ]}


def end_card():
    return {"fade_in": 0.6, "fade_out": 1.2, "lines": [
        (spaced(w("SOLO OR WITH UP TO FOUR FRIENDS", "SOLO O CON HASTA CUATRO AMIGOS")), SERIF, 30, PALE, -40),
        (w("ENGLISH  ·  ESPAÑOL", "ESPAÑOL  ·  ENGLISH"), SERIF, 22, DIM, 10),
        (spaced(w("IN DEVELOPMENT", "EN DESARROLLO")), SERIF, 24, GOLD, 60),
    ]}


def refrain():
    return w("If you hear him far away, he is already here.", "Hay silbidos que nunca se deben seguir.")


# Where he stands in 31_wide's lit frame (x, y of a 16:9 frame).
WIDE_HIM = (0.4516, 0.5417)


def shot(d, **kw):
    return dict(dir=d, **kw)


# The catch, at the game's own cues (`omen.rs`): a breath in the silence,
# his whistle in your ear, a blow at each flash, his shriek as he bends
# over you, the slam of the black, the sack of bones hitting the ground.
def catch_sounds(cut, g, catch, black):
    c = lambda o: cut.at(catch, o)
    b = cut.at(black)
    return [
        (g("catch_breath.wav"), c(0.09), -4, {}),
        (g("whistle_ear.wav"), c(0.45), -3, {}),
        (g("sting_caught.wav"), c(1.07), -6, {}),
        (g("catch_hit_0.wav"), c(1.07), -4, {}),
        (g("catch_hit_1.wav"), c(1.27), -3, {}),
        (g("catch_hit_2.wav"), c(1.47), -2, {}),
        (g("catch_hit_3.wav"), c(1.69), -1, {}),
        (g("catch_shriek.wav"), c(1.69), -10, {"until": b}),
        (g("catch_slam.wav"), b, 0, {}),
        (g("catch_bones.wav"), b + 0.2, -6, {}),
        (g("ringing.wav"), b, -10, {}),
    ]


# ===========================================================================
# A. La Voz del Llano
# ===========================================================================

RADIO_VOICE = [
    # (stem, segment, offset, radio?, english, spanish)
    ("r1", "radio", 3.0, True, "La Voz del Llano. Eleven o'clock. Cuentos de camino.",
     "La Voz del Llano. Las once de la noche. Cuentos de camino."),
    ("r2", "capy", 0.6, True, "Tonight a lady writes to us from Calabozo. Her grandfather knew him… when he was a boy.",
     "Esta noche nos escribe una señora desde Calabozo. Su abuelo lo conoció… de muchacho."),
    ("r3", "ceiba", 0.3, True, "A spoiled boy… who killed his own father.",
     "Un muchacho malcriado… que mató a su propio padre."),
    ("r4", "tureco", 0.6, True,
     "The grandfather tied him to a post. Whipped him. Rubbed hot pepper in the wounds… and set the dog on him.",
     "El abuelo lo amarró a un botalón. Lo azotó. Le echó ají en las heridas… y le soltó el perro."),
    ("r5", "face", 0.4, True, "And he cursed him. “You will carry your father's bones… until the end of time.”",
     "Y lo maldijo. «Cargarás los huesos de tu padre… hasta el fin de los tiempos.»"),
    ("r6", "cross", 0.5, True,
     "We interrupt this program. Three workers from the Santa Rosa ranch, in Guárico, have not come home. "
     "Neighbours say they heard whistling… out on the savanna.",
     "Interrumpimos este programa. Tres obreros del Hato Santa Rosa, en el Guárico, no han regresado. "
     "Los vecinos dicen que oyeron silbidos… en la sabana."),
    ("r7", "cano", 0.9, True, "The main road is flooded. The search is suspended. The ranch truck… is the only way out.",
     "La carretera está inundada. Se suspende la búsqueda. La camioneta del hato… es la única salida."),
    ("r8", "survivor", 0.3, True,
     "If you are out there tonight… take his bones back to the ceiba. Stay together. And don't let him see you.",
     "Si están allá afuera esta noche… llévenle los huesos a la ceiba. No se separen. Y que no los vea."),
    ("r9", "radio2", 0.6, False, "And if you hear the whistle, friends… remember. If you hear him close… he is far away.",
     "Y si oyen el silbido, amigos… recuerden. Si lo oyen cerca… está lejos."),
    ("r10", "turn", 0.6, False, "But if you hear him far away…", "Pero si lo oyen lejos…"),
]


def radio_segments():
    return [
        ("cold", "black", 2.5, {}),
        ("radio", "shot", 9.0, shot("34_radio_long", fade_in=1.5)),
        ("capy", "shot", 3.5, shot("35_capybara", fade_in=0.3, fade_out=0.3)),
        ("gate", "shot", 4.5, shot("01_gate", fade_in=0.3)),
        ("ceiba", "shot", 4.5, shot("02_ceiba", fade_in=0.3, fade_out=0.2)),
        ("tureco", "shot", 3.2, shot("05_tureco")),
        ("ward", "shot", 2.4, shot("28_ward", **{"from": 30})),
        ("bark", "shot", 4.0, shot("29_bark")),
        ("face", "shot", 5.0, shot("22_face", fade_in=0.4)),
        ("sack", "shot", 4.0, shot("24_sack", fade_out=0.2)),
        ("cross", "shot", 4.5, shot("10_crossing")),
        ("wide", "shot", 1.55, shot("31_wide")),
        ("hold", "freeze", 3.95, dict(dir="31_wide", frame=46, focus=WIDE_HIM, zoom=2.2, dim=0.1, fade_out=0.3)),
        ("cano", "shot", 3.8, shot("19_cano")),
        ("truck", "shot", 4.0, shot("08_truck")),
        ("survivor", "shot", 2.6, shot("33_survivor", **{"from": 30})),
        ("carry", "shot", 2.2, shot("06_carry", **{"from": 30})),
        ("party", "shot", 2.2, shot("16_party", **{"from": 20})),
        ("hide", "shot", 3.4, shot("12_hide", **{"from": 20})),
        ("lay", "shot", 3.0, shot("17_lay", **{"from": 36})),
        ("crawl", "shot", 5.5, shot("27_crawl")),
        ("chase", "shot", 2.8, shot("13_chase", **{"from": 20})),
        ("reveal", "shot", 2.2, shot("14_reveal")),
        ("radio2", "shot", 3.5, shot("18_radio")),
        ("velo", "shot", 5.0, shot("21_velo")),
        ("turn", "shot", 7.5, shot("32_turn")),
        ("catch", "shot", 2.8, shot("15_catch")),
        ("black", "card", 3.4, {"lines": [big(w("…HE IS ALREADY HERE.", "…YA ESTÁ AQUÍ."), 54, RED, 0, 0.9)]}),
        ("title", "card", 8.5, title_card(refrain())),
        ("end", "card", 5.0, end_card()),
    ]


def radio_sounds(cut, stems, old, voice_len):
    g = lambda n: os.path.join(K.AUDIO, n)
    s = lambda n: os.path.join(stems, n)
    o = lambda n: os.path.join(old, n)
    at = cut.at
    catch, black = at("catch"), at("black")
    fx = {"fx": K.RADIO_FX}
    lay = at("lay", 2.3 - 36 / 30)
    ev = []
    # The voice: through the radio, until the last two lines leave it and
    # are in the room with you.
    for stem, seg, off, radio, *_ in RADIO_VOICE:
        path = os.path.join(stems, ("en", "es")[LANG], f"{stem}.mp3")
        opt = {"fx": K.RADIO_FX} if radio else {"fx": ["aecho=0.8:0.5:40|70:0.18|0.1"]}
        ev.append((path, at(seg, off), 2 if radio else 3, opt))
    r10 = at("turn", 0.6)
    ev += [
        # Beds: the rain until the catch's silence; the llano.
        (g("rain_loop.wav"), 0.0, -22, {"loop": True, "until": catch + 0.05, "fade_in": 2.0, "fade_out": 0.05}),
        (g("rain_loop.wav"), black, -28, {"loop": True, "until": cut.total, "fade_in": 1.5, "fade_out": 3.0}),
        (g("ambience_llano.wav"), at("capy"), -24, {"loop": True, "until": at("survivor"), "fade_in": 2.0,
                                                    "fade_out": 1.0}),
        (g("frogs_loop.wav"), at("capy"), -18, {"loop": True, "until": at("gate", 1.0), "fade_out": 1.0}),
        # The radio: tuned in, a joropo, the ident, the static under the voice.
        (s("sfx_tune.mp3"), 0.3, -10, {"until": at("radio", 1.8), "fade_out": 0.6}),
        (s("a_joropo.mp3"), at("radio", 1.2), 8, {"fx": K.RADIO_FX, "until": at("radio", 3.1), "fade_in": 0.3,
                                                   "fade_out": 0.8}),
        (g("radio_ident.wav"), at("radio", 2.2), -12, fx),
        (g("radio_static.wav"), at("radio", 1.0), -31, {"loop": True, "until": at("radio2", 0.4), "fade_in": 1.0,
                                                         "fade_out": 0.4}),
        (g("radio_static.wav"), at("radio2"), -27, {"loop": True, "until": r10 + voice_len["r10"] + 0.2,
                                                     "fade_out": 0.1}),
        # The bulletin breaks in: a squeal, the pips.
        (g("radio_squeal.wav"), at("cross") - 0.25, -12, fx),
        (g("radio_static.wav"), at("cross") - 0.3, -16, {"until": at("cross", 0.35), "fade_out": 0.1}),
        (g("radio_pip_long.wav"), at("cross", 0.2), -14, fx),
        # The montage ends; the radio crackles back for the rule.
        (g("radio_squeal.wav"), at("radio2") - 0.1, -12, fx),
        # The score: the tale's bed, the drone under the bulletin, the build,
        # the title.
        (s("a_bed.mp3"), at("radio", 4.0), -6, {"until": at("cross"), "fade_in": 2.0, "fade_out": 0.6,
                                                "duck": [(at("radio", 4.0), at("capy", 6.0), -6)]}),
        (g("dread_drone.wav"), at("cross"), -16, {"loop": True, "until": at("survivor", 0.5), "fade_in": 0.5,
                                                  "fade_out": 1.0}),
        (s("a_build.mp3"), at("survivor"), -7, {"trim_from": 11.0, "until": at("radio2") - 0.1, "fade_in": 0.5,
                                                 "fade_out": 0.05, "duck": [(at("survivor"), at("hide", 1.0), -6)]}),
        (g("dread_drone.wav"), at("radio2", 0.5), -20, {"loop": True, "until": catch, "fade_in": 1.5,
                                                        "fade_out": 0.05}),
        (s("end_2.mp3"), at("title") - 0.15, -2, {"until": cut.total, "fade_out": 3.0}),
        # The tale's own sounds.
        (g("dog_growl.wav"), at("tureco", 0.3), -12, {}),
        (g("aji_scatter.wav"), at("ward", 0.6), -14, {}),
        (g("dog_bark.wav"), at("bark", 1.0), -8, {}),
        (g("dog_bark.wav"), at("bark", 1.75), -10, {}),
        (g("thunder_a.wav"), at("bark", 2.6), -12, {}),
        (g("omen_swell.wav"), at("face"), -10, {}),
        (g("whistle_mid_2.wav"), at("sack", 0.3), -12, {}),
        (g("thunder_b.wav"), at("sack", 2.7), -10, {"until": at("cross") - 0.3, "fade_out": 0.4}),
        (g("thunder_a.wav"), at("wide", 1.6), -14, {}),
        (g("step_water_0.wav"), at("cano", 0.4), -13, {}),
        (g("step_water_1.wav"), at("cano", 1.3), -14, {}),
        (g("step_water_2.wav"), at("cano", 2.2), -13, {}),
        (g("engine_start.wav"), at("truck", 0.4), -11, {"until": at("survivor"), "fade_out": 0.4}),
        # The montage: the bones laid and the llano answers, a friend down, the hunt.
        (g("bones_set.wav"), lay, -6, {}),
        (g("sting_rage.wav"), lay + 0.05, -8, {}),
        (g("thunder_b.wav"), at("crawl", 4.0), -10, {"until": at("chase"), "fade_out": 0.3}),
        (g("heartbeat.wav"), at("crawl", 2.0), -12, {"loop": True, "until": at("reveal", 1.5), "fade_in": 1.0,
                                                     "fade_out": 0.3}),
        (g("sting_hunt.wav"), at("chase"), -9, {}),
        (g("sting_reveal.wav"), at("reveal", 0.35), -7, {"until": at("radio2") - 0.1, "fade_out": 0.05}),
        (g("thunder_a.wav"), at("reveal", 0.6), -9, {"until": at("radio2") - 0.1, "fade_out": 0.05}),
        # The rule: loud as he walks the grass (and is gone), far as you turn.
        (g("whistle_loud_3.wav"), at("velo", 0.2), -8, {"until": at("velo", 3.6), "fade_out": 0.8}),
        (o("clean_whistle_faint_0.wav"), at("turn", 2.6), -9, {}),
        # The signal dies as you turn round.
        (s("sfx_dying.mp3"), r10 + voice_len["r10"] - 0.1, -10, {"until": at("catch"), "fade_out": 0.05}),
        (g("heartbeat.wav"), at("turn", 3.0), -11, {"loop": True, "until": at("catch"), "fade_in": 1.5,
                                                    "fade_out": 0.05}),
        (g("sting_reveal.wav"), at("turn", 5.6), -6, {"until": catch, "fade_out": 0.04}),
        (o("sfx_riser.mp3"), catch - 4.0, -12, {"until": catch, "fade_out": 0.03}),
        (o("sfx_boom.mp3"), at("black", 0.85), -8, {}),
    ]
    ev += catch_sounds(cut, g, "catch", "black")
    return ev


def radio_overlays(cut, voice_len):
    out = []
    for stem, seg, off, _, *texts in RADIO_VOICE:
        start = cut.at(seg, off)
        # In the lower bar of the letterbox; a long line on two rows, broken
        # at the sentence nearest its middle.
        text, end = texts[LANG], start + voice_len[stem] + 0.3
        rows = [text]
        if len(text) > 90:
            cuts = [i + 1 for i, ch in enumerate(text) if ch in ".…" and 0 < i < len(text) - 2 and text[i + 1] == " "]
            if cuts:
                mid = min(cuts, key=lambda i: abs(i - len(text) / 2))
                rows = [text[:mid].strip(), text[mid:].strip()]
        for k, row in enumerate(rows):
            dy = 470 + (k - (len(rows) - 1) / 2) * 38
            out.append((row, ITALIC, 30, "0xE6E0D4", int(dy), start + 0.1, end, 0.25))
    return out


# ===========================================================================
# B. Nadie se queda atrás — on b_drive_1 (harp intro, drums ~13 s, a
# breakdown ~40 s, the surge from ~52 s)
# ===========================================================================

def coop_segments():
    return [
        ("party", "shot", 3.5, shot("16_party", fade_in=0.8)),
        ("survivor", "shot", 4.0, shot("33_survivor")),
        ("wide", "shot", 5.5, shot("31_wide")),
        ("sack", "shot", 2.5, shot("24_sack", **{"from": 40})),
        ("carry", "shot", 2.0, shot("06_carry", **{"from": 20})),
        ("cano", "shot", 2.5, shot("19_cano", **{"from": 30})),
        ("lay", "shot", 3.0, shot("17_lay", **{"from": 30})),
        ("count", "shot", 2.5, shot("25_count", **{"from": 30})),
        ("chase1", "shot", 2.5, shot("13_chase")),
        ("ward", "shot", 4.0, shot("28_ward", **{"from": 20})),
        ("bark", "shot", 3.0, shot("29_bark")),
        ("beacon", "shot", 3.0, shot("30_beacon", **{"from": 40})),
        ("power", "shot", 2.0, shot("07_power", **{"from": 30})),
        ("truck", "shot", 2.0, shot("08_truck", **{"from": 50})),
        ("crawl", "shot", 5.5, shot("27_crawl")),
        ("rise", "shot", 4.5, shot("26_rise", **{"from": 15})),
        ("velo", "shot", 3.5, shot("21_velo", **{"from": 45})),
        ("hide", "shot", 3.0, shot("12_hide", **{"from": 30})),
        ("friends", "shot", 3.0, shot("09_friends", **{"from": 45})),
        ("reveal", "shot", 2.2, shot("14_reveal")),
        ("turn", "shot", 3.0, shot("32_turn", **{"from": 135})),
        ("catch", "shot", 2.8, shot("15_catch")),
        ("black", "black", 2.0, {}),
        ("title", "card", 7.0, title_card(refrain())),
        ("end", "card", 4.5, end_card()),
    ]


def coop_overlays(cut):
    at = cut.at
    lower = 330

    def card(text, seg, a, b, size=46, color=BONE, dy=lower):
        return (spaced(text), SERIF, size, color, dy, at(seg, a), at(seg, b), 0.2)

    return [
        card(w("UP TO FOUR FRIENDS", "HASTA CUATRO AMIGOS"), "party", 0.8, 3.7),
        card(w("ONE NIGHT ON THE LLANO", "UNA NOCHE EN EL LLANO"), "wide", 1.6, 5.2),
        card(w("FIND HIS FATHER'S BONES", "ENCUENTRA LOS HUESOS DE SU PADRE"), "sack", 0.15, 2.4, 42),
        card(w("BRING THEM HOME TO THE CEIBA", "LLÉVALOS A LA CEIBA"), "lay", 0.2, 2.9, 42),
        card(w("EVERY BONE MAKES HIM ANGRIER", "CADA HUESO LO ENFURECE MÁS"), "count", 0.1, 2.4, 42),
        card(w("PEPPER MAKES HIM STOP…", "EL AJÍ LO DETIENE…"), "ward", 0.2, 2.0, 42),
        card(w("…AND COUNT", "…A CONTAR"), "ward", 2.0, 3.9, 42),
        card(w("TURECO SMELLS HIM FIRST", "TURECO LO HUELE PRIMERO"), "bark", 0.15, 2.9, 42),
        card(w("LIGHT THE TOWER…", "ENCIENDE LA TORRE…"), "beacon", 0.1, 1.5, 42),
        card(w("…AND HE COMES TO THE FIRE", "…Y VIENE AL FUEGO"), "beacon", 1.5, 2.95, 42),
        card(w("START THE TRUCK", "ARRANCA LA CAMIONETA"), "power", 0.15, 3.9, 42),
        card(w("IF A FRIEND FALLS…", "SI UN AMIGO CAE…"), "crawl", 0.4, 2.3, 42),
        card(w("GO BACK FOR THEM", "VUELVE POR ÉL"), "crawl", 2.4, 3.95, 42),
        card(w("BEFORE HE DOES", "ANTES QUE ÉL"), "crawl", 4.05, 5.45, 42, RED),
        card(w("THE WHISTLE LIES", "EL SILBIDO MIENTE"), "velo", 0.2, 3.4, 64, RED, 0),
    ]


def coop_sounds(cut, stems, old):
    g = lambda n: os.path.join(K.AUDIO, n)
    s = lambda n: os.path.join(stems, n)
    o = lambda n: os.path.join(old, n)
    at = cut.at
    catch, black = at("catch"), at("black")
    lay = at("lay", 2.3 - 1.0)
    return [
        (g("rain_loop.wav"), 0.0, -24, {"loop": True, "until": catch + 0.05, "fade_in": 1.5, "fade_out": 0.05}),
        (g("rain_loop.wav"), black, -30, {"loop": True, "until": cut.total, "fade_in": 1.0, "fade_out": 2.5}),
        # The score: the whole drive, stopped dead on the catch's silence.
        (s("b_drive_1.mp3"), 0.0, -5, {"until": catch, "fade_out": 0.04}),
        (s("end_1.mp3"), at("title") - 0.15, -3, {"until": cut.total, "fade_out": 2.5}),
        # Under the cards, the game.
        (g("thunder_a.wav"), at("wide", 1.5), -12, {}),
        (g("bones_rattle.wav"), at("carry", 0.3), -16, {}),
        (g("step_water_0.wav"), at("cano", 0.3), -15, {}),
        (g("step_water_2.wav"), at("cano", 1.2), -15, {}),
        (g("bones_set.wav"), lay, -8, {}),
        (g("sting_rage.wav"), lay + 0.05, -10, {}),
        (g("counting.wav"), at("count", 0.1), -12, {}),
        (g("sting_hunt.wav"), at("chase1"), -12, {}),
        (g("aji_scatter.wav"), at("ward", 0.1), -10, {}),
        (g("counting.wav"), at("ward", 1.6), -12, {}),
        (g("dog_growl.wav"), at("bark", 0.0), -14, {}),
        (g("dog_bark.wav"), at("bark", 1.0), -7, {}),
        (g("dog_bark.wav"), at("bark", 1.7), -9, {}),
        (g("thunder_b.wav"), at("bark", 2.5), -12, {"until": at("beacon", 0.3), "fade_out": 0.3}),
        (g("beacon_flare.wav"), at("beacon", 0.0), -9, {}),
        (g("pump_crank.wav"), at("power", 0.0), -12, {}),
        (g("power_on.wav"), at("power", 0.5), -10, {}),
        (g("engine_start.wav"), at("truck", 0.0), -9, {"until": at("crawl", 0.5), "fade_out": 0.4}),
        # The breakdown: a friend down calls for help; the strike.
        (g("call_help.wav"), at("crawl", 0.6), -10, {}),
        (g("thunder_a.wav"), at("crawl", 4.0), -8, {}),
        (g("sting_reveal.wav"), at("crawl", 4.0), -12, {}),
        (g("omen_swell.wav"), at("rise", 0.2), -10, {}),
        # The surge: the whistle that lies.
        (g("whistle_loud_3.wav"), at("velo", 0.0), -7, {"until": at("hide", 0.5), "fade_out": 0.5}),
        (g("whistle_mid_1.wav"), at("hide", 0.6), -11, {}),
        (g("thunder_b.wav"), at("friends", 2.0), -10, {"until": at("reveal"), "fade_out": 0.2}),
        (g("sting_reveal.wav"), at("reveal", 0.35), -8, {"until": at("turn"), "fade_out": 0.05}),
        (g("sting_reveal.wav"), at("turn", 1.7), -6, {"until": catch, "fade_out": 0.04}),
        (o("sfx_boom.mp3"), at("title"), -8, {}),
    ] + catch_sounds(cut, g, "catch", "black")


# ===========================================================================
# C. Si lo oyes lejos — the teaser
# ===========================================================================

def teaser_segments():
    return [
        ("close", "black", 4.0, {}),
        ("wide", "shot", 1.55, shot("31_wide")),
        # The strike's instant held, pushing in on the speck that is him.
        ("hold", "freeze", 4.45, dict(dir="31_wide", frame=46, focus=WIDE_HIM, zoom=2.6, dim=0.1, fade_out=0.6)),
        ("far", "black", 3.5, {}),
        ("turn", "shot", 7.5, shot("32_turn", fade_in=0.6)),
        ("catch", "shot", 2.8, shot("15_catch")),
        ("black", "black", 3.6, {}),
        ("title", "card", 7.0, title_card(refrain())),
        ("end", "card", 3.8, end_card()),
    ]


def teaser_overlays(cut):
    at = cut.at
    t = lambda text, seg, a, b, color=BONE, size=50, dy=0: (spaced(text), SERIF, size, color, dy, at(seg, a),
                                                           at(seg, b), 0.45)
    return [
        t(w("IF YOU HEAR HIM CLOSE…", "SI LO OYES CERCA…"), "close", 1.0, 3.8),
        t(w("…HE IS FAR AWAY.", "…ESTÁ LEJOS."), "hold", 1.0, 4.0, BONE, 46, 330),
        t(w("IF YOU HEAR HIM FAR AWAY…", "SI LO OYES LEJOS…"), "far", 1.1, 3.4),
        t(w("…HE IS ALREADY HERE.", "…YA ESTÁ AQUÍ."), "black", 0.9, 3.5, RED, 56),
    ]


def teaser_sounds(cut, stems, old):
    g = lambda n: os.path.join(K.AUDIO, n)
    s = lambda n: os.path.join(stems, n)
    o = lambda n: os.path.join(old, n)
    at = cut.at
    catch, black = at("catch"), at("black")
    return [
        (g("rain_loop.wav"), 0.0, -20, {"loop": True, "until": catch + 0.05, "fade_in": 0.6, "fade_out": 0.05}),
        (g("rain_loop.wav"), black, -28, {"loop": True, "until": cut.total, "fade_in": 1.0, "fade_out": 2.5}),
        # Close: loud, right in your ear (and he is nowhere near).
        (g("whistle_loud_0.wav"), at("close", 0.3), -4, {"until": at("wide", 1.2), "fade_out": 1.0}),
        (g("thunder_a.wav"), at("wide", 1.5), -6, {}),
        (g("ambience_llano.wav"), at("wide"), -22, {"loop": True, "until": at("far"), "fade_out": 0.4}),
        # Far: faint, far off (and he is right behind you).
        (o("clean_whistle_faint_3.wav"), at("far", 0.5), -8, {}),
        (g("dread_drone.wav"), at("far", 1.0), -16, {"loop": True, "until": catch, "fade_in": 2.5,
                                                    "fade_out": 0.05}),
        (g("heartbeat.wav"), at("turn", 1.0), -10, {"loop": True, "until": catch, "fade_in": 2.0,
                                                    "fade_out": 0.05}),
        (g("sting_reveal.wav"), at("turn", 5.5), -5, {"until": catch, "fade_out": 0.04}),
        (o("sfx_riser.mp3"), catch - 4.5, -12, {"until": catch, "fade_out": 0.03}),
        (o("sfx_boom.mp3"), at("black", 0.85), -6, {}),
        (s("end_1.mp3"), at("title") - 0.15, -2, {"until": cut.total, "fade_out": 2.5}),
    ] + catch_sounds(cut, g, "catch", "black")


# ===========================================================================

def main():
    global LANG
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--cut", choices=["radio", "coop", "teaser"], required=True)
    p.add_argument("--lang", choices=["en", "es"], required=True)
    p.add_argument("--frames", required=True)
    p.add_argument("--stems", required=True)
    p.add_argument("--old-stems", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--vertical", action="store_true", help="9:16 (the teaser)")
    p.add_argument("--keep", help="keep the intermediate files in this folder")
    p.add_argument("--sound-only", action="store_true", help="reuse the kept picture (--keep), remix the sound")
    p.add_argument("--plan", action="store_true", help="print the timeline, render nothing")
    a = p.parse_args()
    LANG = ("en", "es").index(a.lang)
    frames = os.path.join(a.frames, "trailer_vertical" if a.vertical else "trailer")

    voice_len = {}
    if a.cut == "radio":
        cut = K.Cut(radio_segments(), a.vertical)
        for stem, *_ in RADIO_VOICE:
            voice_len[stem] = K.duration(os.path.join(a.stems, a.lang, f"{stem}.mp3"))
        lines = sorted((cut.at(seg, off), stem) for stem, seg, off, *_ in RADIO_VOICE)
        for (t0, s0), (t1, s1) in zip(lines, lines[1:]):
            if t0 + voice_len[s0] > t1 - 0.3:
                print(f"warning: {s0} ends at {t0 + voice_len[s0]:.2f} s, {s1} starts at {t1:.2f} s")
        overlays = radio_overlays(cut, voice_len)
        sounds = lambda: radio_sounds(cut, a.stems, a.old_stems, voice_len)
        # A burst of static on the picture where the radio breaks up.
        static = [(cut.at("cross") - 0.3, cut.at("cross", 0.25)), (cut.at("radio2") - 0.15, cut.at("radio2", 0.15)),
                  (cut.at("catch") - 0.45, cut.at("catch") - 0.05)]
        letterbox = 140
    elif a.cut == "coop":
        cut = K.Cut(coop_segments(), a.vertical)
        overlays, static, letterbox = coop_overlays(cut), [], 0
        sounds = lambda: coop_sounds(cut, a.stems, a.old_stems)
    else:
        cut = K.Cut(teaser_segments(), a.vertical)
        overlays, static, letterbox = teaser_overlays(cut), [], 0
        sounds = lambda: teaser_sounds(cut, a.stems, a.old_stems)

    cut.plan(frames) if a.plan else print(f"timeline {cut.total:.1f} s")
    if a.plan:
        return
    tmp = a.keep or tempfile.mkdtemp(prefix=f"silbon_{a.cut}_")
    os.makedirs(tmp, exist_ok=True)
    graded = os.path.join(tmp, "graded.mp4")
    if a.sound_only and os.path.exists(graded):
        video = graded
    else:
        video = cut.build_video(frames, tmp, overlays, letterbox, static)
    print("picture done")
    audio = cut.build_audio(sounds(), tmp)
    print("sound done")
    cut.mux(video, audio, a.out)
    if not a.keep:
        shutil.rmtree(tmp, ignore_errors=True)
    print(f"wrote {a.out}")


if __name__ == "__main__":
    main()

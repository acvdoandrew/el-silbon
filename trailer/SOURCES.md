# Trailer sources

Everything heard or seen in the trailers that is not the game itself.
Raw generations, frames and renders stay out of git (`trailer/out/`,
`~/Videos/`).

## Trailers 4–6: A «La Voz del Llano», B «Nadie se queda atrás», C «Si lo oyes lejos» (EN / ES), 2026-10-01

All ElevenLabs material from the Creator account, flow `7c4ZCUEoOYwOuTi3KbRF`
("El Silbón — trailers A/B/C"). Script: `trailer/script_v4.md`; edit:
`trailer/cut4.py` (machinery `trailer/cutkit.py`); stems in
`trailer/out/stems4/` (session ids in `sessions.txt`).

- **Narration (A only)** (`eleven_v4`): a designed voice, **El Locutor del
  Llano** (`mtrTjKREKkB2FTiZn6LC`, `eleven_ttv_v3`, an older llanero radio
  host of the 1960s), one take per line, r1–r10 in both languages. Both
  transcribed back with `eleven_scribe_v1`: they match the script.
- **Score** (`eleven_music_v2_5`, instrumental; the cues transcribe to no
  words): `a_bed` 50 s (sub drone, distant harp, cello), `a_joropo` 20 s (a
  1960s joropo, heard through the radio filter), `a_build` 40 s (string
  ostinato, frame drum, rising harp, toms), `b_drive_1` 80 s (horror joropo:
  harp, cuatro, maracas, war drums; `b_drive_2` unused), `end_1` / `end_2`
  15 s (impact, harp, low choir; C/B and A).
- **Effects** (`eleven_text_to_sound_v2`): `sfx_tune` (a valve radio tuned
  across the dial), `sfx_dying` (the signal breaking up into static). The
  hits `sfx_boom` and `sfx_riser` and the two clean far whistles are trailer
  3's (`trailer/out/stems3/`).
- **From the game:** every other sound is `assets/audio` (the radio's
  static, squeal, ident and pips; Tureco's ElevenLabs growl and bark; the
  catch and `catch_bones`; whistles built from the uncleared third-party
  recording — see below).
- **Edit-side effects:** an AM radio filter on the narration (band 320 Hz –
  3.2 kHz, compression, soft clip, a small box echo); static bursts on the
  picture where the radio breaks up; a held strike frame pushed in on him
  (`31_wide`, frame 46) — real footage held, nothing added to it.

## Trailer 3 (EN / ES, the Madrina), 2026-10-01

All ElevenLabs material from the Creator account, flow `95POp49C16hm4McLmAT8`
("El Silbón — trailer 3 (the Madrina)"). Script: `trailer/script_v3.md`;
edit: `trailer/edit3.py`; stems in `trailer/out/stems3/` (session ids in
`sessions.txt`).

- **Narration** (`eleven_v4`): library voice **Graciela - Wise and
  Grounded** (`Tc7dePDnjW90vlgpLnhZ`, mature Latin American female) in both
  languages, `[whispers]` only on n1, n9 and n11. Both languages
  transcribed back with `eleven_scribe_v1`: they match the script.
- **Score** (`eleven_music_v2_5`, instrumental, built round the arpa
  llanera): `m1` 42 s (the tale: a lone harp over a sub drone), `m2` 36 s
  (a minor joropo ostinato with maracas and frame drum, rising), `m3` 18 s
  (struck harp strings and pounding drums), `m4` 15 s (an impact, the harp,
  a distant wordless voice).
- **Hits:** `sfx_*` from trailer 2 (same Creator account).
- **From the game:** every other sound is `assets/audio`, including the
  new catch sounds and the whistles built from the uncleared third-party
  recording (see below). The two far whistles (opening and end) are
  trailer 1's clean takes from commit `f75473b`, not the game's current
  wind-torn ones.

## Trailer 2 (EN / ES), 2026-10-01

All ElevenLabs material in the cut was generated on the user's **Creator**
account, flow `Eaqc16H3Dz4RT01b0ayL` ("El Silbón — trailer 2 narration
(EN/ES)"). An earlier score made on the un-upgraded account (flow
`S5XnPulPYleYArAHVfRh`) was not used in the final cut.

### Narration (`eleven_v4`)

One narrator for both languages: library voice **Jose Rea - Narración**
(`sqYpTimImojg8h2yjzQz`, es-venezuelan, middle-aged male), one take per
line with `[slowly] [deep voice]` (vo6 `[softly]`, vo9 `[whispers]`).
Text: `trailer/script.md`. The session ids per line are in
`trailer/out/stems/sessions.txt`. Both languages were transcribed back with
`eleven_scribe_v1` and match the script word for word.

### Score (`eleven_music_v2_5`, instrumental)

| stem | length | prompt (abridged) |
|---|---|---|
| `m1.mp3` | 33 s | dark folk-horror ambient: lone detuned cuatro, breathing drone, swelling low strings |
| `m2.mp3` | 44 s | build: 70 BPM pulse, frame drum and maracas, cuatro ostinato in B minor, rising strings |
| `m3.mp3` | 19 s | chase: pounding drums, staccato dissonant strings, rising clusters |
| `m4.mp3` | 16 s | end title: impact, lonely cuatro, sub drone, cellos, distant choir hum |

### Hits (`eleven_text_to_sound_v2`)

`sfx_braam` (4 s low distorted brass hit), `sfx_boom` (4 s sub drop and
crash), `sfx_riser` (5 s dissonant swell), `sfx_whoosh` (1.5 s).

### From the game

Every other sound is the game's own `assets/audio` (see
`assets/SOURCES.md`). **The whistles are built from a third-party
recording** ("El Silbon Silbido", YouTube `oNGPZNXmZ1c`, licence unknown,
not cleared for release). The user chose to keep that whistle in the
trailer (2026-10-01). Clear it or swap in the synthesized whistle
(`python3 tools/gen_audio.py --synth --out DIR`) before any store or
commercial use of the trailer.

Fonts: Noto Serif (OFL), bundled with the game.

## Trailer 1 (teaser), 2026-09-29

Narration `eleven_v3`, premade voice "Callum" (English); four
`eleven_music` cues; hits `eleven_text_to_sound`; all on flow
`t3VPUz9ndVLhNvptnZKb` (the earlier account). Edit: `tools/trailer/edit.py`.

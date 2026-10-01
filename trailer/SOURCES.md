# Trailer sources

Everything heard or seen in the trailers that is not the game itself.
Raw generations, frames and renders stay out of git (`trailer/out/`,
`~/Videos/`).

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

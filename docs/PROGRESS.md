# Progress

## Current handoff — 2026-10-01 (test.6, the crawl, comic sounds)

- **test.6** (`v0.1.0-test.6`, local tag; fingerprint `0x595f9863bfc94fc4`): main at `df7d2b0` (Tripo models, wildlife, the heavier catch) verified on Windows: gate green (clippy clean here; the `src/icon.rs:80` lint seen on Linux does not fire on Windows), sweeps Normal 150 / 146, Gentle 150 / 150, Hard 150 / 150, headless net smoke PASS. Zip 70.4 MB.
- **The crawl** (`5e5b657`, playtest: "they're just laying on the ground and floating forward"): a downed friend's crawl was sized against a walk (0.9 m/s is a quarter of 3.6) on a walking stride. `avatar::stride_and_size` now sizes it against `Tuning::crawl_speed` on a 0.8 m `CRAWL_STRIDE`; the prone pose reaches with one arm, pulls with the other, swings the opposite knee out to push, rolls hips and shoulders, bobs the head. Presentation only (fingerprint unchanged). Checked in the trailer's `20_downed` frames (grass hides most of it); judge in play.
- **Comic sounds (user, from trailer 3):**
  - Removed from `trailer/edit3.py`: the three `sfx_whoosh` hits before `hide` / `chase` / `reveal` (heard at about 1:20 and 1:25), and the `bones_rattle` after the catch's slam (about 1:35). Re-render trailer 3 on Linux.
  - **Done (ElevenLabs, user's choice):** Tureco's `dog_growl.wav` and `dog_bark.wav` and a new `catch_bones.wav` (played at the catch's cut instead of `bones_rattle`) are ElevenLabs effects (`eleven_text_to_sound_v2`, flow `zh6P9MUzoGajOtxnsK8q`, the user's picks: growl 3, bark 2, bones 3). Sources in `assets/audio/source/elevenlabs/`; `gen_audio.py` (`recorded_or`) trims and levels them to the old files' loudness and falls back to the synthesized ones without the source, so a regeneration never overwrites them (verified: only those three files change). `bones_rattle.wav` itself (taking a bundle, the haul) is unchanged. Trailer 3's `edit3.py` puts `catch_bones` back after the slam; re-render on Linux. Unverified: the levels by ear in play.

## Current handoff — 2026-10-01 (trailer 3: the Madrina)

Uncommitted. The user asked for a scarier catch (done, below) and then a
new, dramatic trailer with a female narrator telling the tale, in both
languages, no mention of AI.

- **Output** (`~/Videos/`): `el_silbon_trailer3_en.mp4`,
  `el_silbon_trailer3_es.mp4` (111.8 s, 1080p30, -14.0 LUFS, true peak
  -1.7/-1.8 dBFS), `*_1080p_share.mp4` (~120 MB, ~9 Mbps) and
  `*_preview_720p.mp4`; the catch alone: `el_silbon_catch_preview.mp4`.
- **The cut** (`trailer/script_v3.md`, `trailer/edit3.py --lang en|es`):
  the Madrina (the tale's teller in `lore.rs`) to the children: cold open
  in the dark, the legend card, the murder, the post and the dog, the curse
  over a push in on his face, his walk; the game (bones, La Rabia, his
  counting, the truck, dawn, the caño, four friends, a friend down); the
  rule; the new catch; the title.
- **New shots** in `src/trailer.rs`: `22_face`, `23_caiman`, `24_sack`,
  `25_count` (with `Shot::bones_home`); every shot re-rendered with the
  Tripo models.
- **The far whistle** (opening, end) is trailer 1's clean take from
  `f75473b`: the user heard the game's current wind-torn far whistle as
  broken audio in the cold open. Remixed both cuts and their copies.
- **After the user's review** (2026-10-01, stills from the cut):
  - Cattle, horse, capybara, egret: the legs below the belly line now
    belong to the body (`legs()` force boxes in `tools/models/tripo.py`);
    they had followed the neck and tail bones and swung as the head
    grazed. The herd now stands on the surface height, not y = 0.
  - Tureco stands at his post (the old sit sank and tilted the root and
    buried a standing model's hind legs), wears a collar, and the rope
    sags in eight lengths from the post to the collar's side.
  - El Silbón's arms hang plumb (shoulders undo the torso's lean); the
    count and the hunt reach forward, not backward. The catch is untouched.
  - Survivors: arms brought in from the model's A-pose, a bigger arm swing
    and stride (the foot test still holds). Trailer walks lengthened to a
    walking pace (a short path sized the stride to a shuffle).
  - Shots: Tureco seen across his rope, the sack followed from his
    shoulder (not from the ground), the hide pass reversed so the sack
    rides away from the camera, the party three-quarters on.
  - Sound: no synthesized groan or cry over the downed friend; the loud
    whistle ends before the whispered rule, the next one moves into the
    hide, the music ducks 9 dB under the rule and the whisper is +4 dB.
- **Verified:** both transcripts match the script; frames sampled across
  the English cut; gate green (old `icon.rs` lint aside).
- **Unverified:** nobody has watched or listened yet — the harp score, the
  narration over it, and the catch's mix in the edit are judged by numbers
  only.

## Current handoff — 2026-10-01 (Tripo models, wildlife, a heavier catch)

Uncommitted. The user generated 14 models in Tripo and asked for them in
the game, then for the jump scare to be scarier (sound and camera).

- **Models** (`assets/models/*.glb`, recorded in `assets/SOURCES.md`):
  El Silbón, the four survivors, Tureco, cow/bull/calf (the "goat" export
  is a zebu calf), horse, capybara, caiman, egret, truck. Prepared by
  `tools/models/tripo.py` (`-- NAME`, `--measure` for the grid renders):
  turn, real size, decimate, 2K JPEG texture, and skin to the game's joint
  names with identity rest rotations. Heat weighting fails on Tripo meshes
  (20–100 overlapping pieces), so weights are geometric per piece: arm
  bones only for pieces that reach the upper-arm tube, a soft inner side
  for loose sleeves, a tight tube for El Silbón's bare arms through his
  coat, `zones` for his sack, `force` boxes for Tureco's jaw. Every model
  was checked in a test pose render.
- **El Silbón's proportions:** hips at the code's 1.66 m (2.59 m with the
  hat). The catch now aims at his face from the loaded body's own Head
  joint (`Fright::head`, set by `models.rs`; the stand-in's
  `STAND_IN_HEAD` otherwise); the catch test runs both bodies.
- **Wildlife:** `District::fauna` (scenery only: no blockers, sight or
  sound), `world::fauna` (the herd's idle poses scaled per animal), a
  horse and an egret with the herd, three chigüires on the caño's north
  bank, a baba in the channel, an egret in the shallows.
- **Truck:** faces +X like the old one; lamp-lens discs carry the
  `truck_lamp_*` materials the engine lights.
- **Bevy `jpeg` feature** enabled (the Tripo textures are JPEG); one small
  decoder crate added to `Cargo.lock`.
- **The catch, heavier:** new WAVs (`catch_breath`, `catch_hit_0..3`,
  `catch_shriek`, `catch_slam`; 89 in all) on new stings (`Breath`,
  `Flash(k)`, `Shriek`; the slam with the bones and the ringing at the
  cut), cued at `omen::BREATH_AT`/`FLASH_AT` (the one source of the flash
  times). The view: a growing tremble in the silence, a flinch at each
  flash (snap up and aside, knocked back), shake nearly doubled, the lens
  narrowing in the silence, punching harder at the flashes and bulging
  wide as he bends over you. Preview: `~/Videos/el_silbon_catch_preview.mp4`
  (its mix is hand-set, not the game's).
- **Photos:** new `64_model_horse`, `65_model_capybara`, `66_model_caiman`.
- **Verified:** gate (fmt, clippy `-D warnings` except the old
  `src/icon.rs:80` lint, test: lib 135, district 9, session 66 + 3
  ignored); the full `--photos` set at 2560×1440 (`screenshots/photos/`);
  the catch frames in-engine.
- **Unverified:** the in-game catch mix by ear (gains `0.9`, `0.75–1.2`,
  `0.55`, `1.3` of sfx); the horse seen only from behind; the truck's tail
  lenses; play with the new survivors in two windows. The gameplay
  fingerprint moved (`district.rs`, `Cargo.lock`): this build won't pair
  with test.5.

## Current handoff — 2026-10-01 (trailer 2, EN / ES)

Uncommitted. The user asked for a new trailer showing the new features, in
English and Spanish, with no mention of the tools it was made with (it
may go on a Steam page; not decided, so no store call to action).

- **Output** (`~/Videos/`): `el_silbon_trailer2_en.mp4` and
  `el_silbon_trailer2_es.mp4` (masters, 1080p30, 113.4 s, about 350 MB),
  `*_1080p_share.mp4` (about 56 MB) and `*_preview_720p.mp4`.
- **Script:** `trailer/script.md` (timeline, both narrations, cards). One
  llanero narrator; lines from the Madrina's chapters and the whistle rule.
- **New shots** in `src/trailer.rs` (presentation only, as before):
  `17_lay` (the last bundle laid beside the other four, the lamps'
  rage stutter via `RelicDelivered`), `18_radio`, `19_cano` (two survivors
  wading past the boat, from the bridge), `20_downed` (crawling, torch
  dropped, a friend creeping in), `21_velo` (he stares over the grass and
  is gone at 2.6 s). New staging hooks: `Him::vanish_at`, `Shot::lay`.
  No dawn sky (the game has none: a card and `dawn.wav`); no spectator
  shot (no distinct view to film).
- **Edit:** `trailer/edit.py --lang en|es` (`--plan` prints the timeline
  and narration overlaps; `--keep DIR --sound-only` remixes without
  re-rendering the picture). The v1 `tools/trailer/edit.py` is untouched.
  Drops v1's "made with" and "generated in code" cards. Two-pass linear
  loudnorm plus a limiter (the single-pass one overshot to +1.5 dBTP).
- **Sound:** narration, score and hits all from the user's new Creator
  ElevenLabs account (flow `Eaqc16H3Dz4RT01b0ayL`), details in
  `trailer/SOURCES.md`. Narrator: Jose Rea (`sqYpTimImojg8h2yjzQz`,
  Venezuelan) in both languages. The game's whistles stay the
  YouTube-derived ones by the user's choice (not cleared for release).
- **Verified:** every shot rendered and the new ones reviewed on contact
  sheets; both cuts' frames sampled (cards, subtitles with accents, title,
  end card); narration transcribed back with Scribe, word for word in
  both languages; -14.4 LUFS, true peak -1.0 (EN) / -1.4 (ES) dBFS; the
  catch's silence measures -91 dB; video and sound timelines agree
  (113.4 s; a frame-count guard now stops a cut that asks a shot for more
  frames than were rendered). `cargo fmt --check` and `cargo test` green
  (lib 135, district 9, session 66 + 3 ignored). **Clippy fails on this
  machine only in `src/icon.rs:80`** (`chunks_exact_to_as_chunks`, new in
  Rust 1.98; not touched here); the trailer code is clippy-clean.
- **Unverified:** nobody has listened yet. The music, the hits and the
  narration-over-music balance were judged only by loudness numbers, and
  the narrator's Venezuelan-accented English was judged only by the
  transcript.
- **Next:** the user's watch; then nudge gains or offsets in
  `trailer/edit.py` and remix with `--sound-only`.

## Start here — 2026-09-30 (end of day: test.5 on main)

- **State:** `main` = `v0.1.0-test.5` (fingerprint `0x8e42d8fa763d35cc`), published as a GitHub pre-release. Everything from the M1b batch (sections below) and test.5 (harder Silbón, El Velo, menu arrows, Language / Idioma, Journal columns) is merged. No other branch holds unmerged work; the temporary lane worktrees are gone.
- **Windows dev setup:** checkout `C:\Users\andre\source\repos\el-silbon`, MSVC toolchain via `rustup override`; Smart App Control must stay off. Gate as in AGENTS.md; sweeps per night with `$env:ROUTE_NIGHT='normal|gentle|hard'; cargo test --locked --test session the_routes_hold_over_many_storms -- --ignored --nocapture`; package with `powershell -File tools\package.ps1 0.1.0-test.N` (zip in `target\dist\`, shared over Discord or a GitHub pre-release).
- **Sweep now (solo / shared):** Normal 150 / 146 (on the bar: shared 38, 72, 119, 145, a harder Silbón catching player 2 on lit open ground), Gentle 150 / 150, Hard 150 / 150. Any harder tuning needs the route driver (`src/script.rs`) to get smarter first, or a decision on the bar.
- **Waiting on the test.5 playtest:** do the veils (25–70 s, whistle gaps capped at 10 s) feel right; is he aggressive enough now (grace 60 s, Relax 40–70 s, 3 hunts per 10 min on Normal); the ‹ › arrows by mouse; Spanish texts on screen (page 19 and long Journal titles).
- **User decisions so far:** Outcome::Dawn at 1.5 × night_length; `anima_sight` on; `bleed_after_sack` 30 s; a hauled friend's map dot follows them live; the name stays "El Silbón — The Return"; display mode in Video settings, no fullscreen key; repo public, test builds as GitHub pre-releases or zips over Discord.
- **Next candidates (roadmap v3, M2):** La Cuenta (vigil while he counts), the rest of the call wheel (Gritos y silbos), El Compadre and the house omens, Ánimas haunts for spectators, El desmayo, El desamparado, Mandados and the copla at dawn, El paso del caño, the tale station on the radio.
- **Known small issues:** at 1280 px the 860 px pause Journal covers part of the objectives list; the exe's ProductName reads `el_silbon` (winresource default).

## Current handoff — M1b (branch m1b)

**test.5 (merged, 2026-09-30).** `t5-silbon` (031cf19: he hunts harder, El Velo) and `t5-lang` (c6141ff, 475222b: menu arrows by mouse, Language / Idioma, the Journal fits) merged onto test.4. Gate on the merged tree: fmt, clippy `-D warnings`, test (lib 135, district 9, session 66 + 3 ignored) clean. Sweeps (solo / shared): Normal 150 / 146, Gentle 150 / 150, Hard 150 / 150. Headless two-process net smoke: NET SMOKE PASS on host and client; fingerprint `0x8e42d8fa763d35cc` (does not pair with test.4). Packaged `el_silbon-0.1.0-test.5-windows.zip` (109.1 MB) with `tools/package.ps1`. Unverified by hand: the veils and the harder pacing in a human night, the ‹ › arrows by mouse, the Spanish texts on screen. User decision (test.4 playtest): a friend's map dot keeps following them live while hauled.

**test.4 (merged): verified and tagged `v0.1.0-test.4` (local, not pushed).**
- Tree: m1b 524a765, clean, Windows/MSVC. The tests and the net smoke ran on the debug build, the zip on the release build, all from this one tree. This note's commit changes only docs, which the fingerprint does not hash.
- Gate: fmt, check, test (lib 124, district 9, session 64 + 3 ignored), clippy -D warnings, all clean.
- Route sweeps (150 seeds each, `target\t4\sweep-{normal,gentle,hard}.log`):
  - Normal: solo 148/150, shared 150/150 (bar 146 met). Failing seeds are solo 76 and 141, the same two recorded below.
  - Gentle: solo 150/150, shared 150/150 (bar 146 met).
  - Hard (informational): solo 149/150, shared 145/150 (river baseline 150/141). Failing seeds are solo 147 and shared 13, 19, 61, 94, 127, identical to the wiring-time sweep. The test's own 97% assert fires on the shared 145, as it does at the river baseline; it is not the bar.
- Net smoke (headless, two processes from this tree's debug exe, 127.0.0.1:5361, about 12.8 minutes): `NET SMOKE PASS host` and `NET SMOKE PASS client`. Fingerprint `NET fingerprint 0x7447a278f501d3ce` on both sides. The exit-code watcher recorded nothing, so the PASS lines and both processes having exited are the evidence (logs `target\t4\host.err`, `client.err`).
- Package: `tools\package.ps1 0.1.0-test.4` built release and wrote `target\dist\el_silbon-0.1.0-test.4-windows.zip` (109.1 MB, 117 entries). It holds `el_silbon.exe` (byte-identical to `target\release`), `PLAYING.txt`, `assets\branding\whistle-cover.png`, all 82 WAVs (`sting_rage.wav` and the five radio clips, pips included) and no `assets\audio\source`. The exe embeds one icon group with seven images; the extracted 32x32 icon is the hat-and-face silhouette.
- Still unverified (rendered, user-led): the splash, the SHARED DAWN line, the single strip at a laying, the dots for a watcher at the radio. The net smoke does not reach the 1800 s Dawn; physical-LAN and internet play are untested.

**Integration: the four lanes, the branding, and their wiring (merge commits plus one wiring commit).**
- Merged into m1b with `git merge --no-ff`, in this order: m1b-rage b8ef64d (4b8de7d), m1b-respiro 47c0a33 (70cfd81), m1b-radio ebc3b3d (ac4c1ca), m1b-spec 4167cb1 (e793303), main c5dd566 (6b768c5). After each merge `cargo check --locked` and `cargo test --locked --lib` were green (lib 102, 108, 115, 115, 124), and after the radio merge every test binary built.
- Resolutions that carry numbers:
  - `Event::ALL` is 53 (`Dawn`, then `RadioTuned`).
  - `GAMEPLAY` is 17 (`pacing.rs`, then `radio.rs`, both after `Cargo.lock`).
  - `assets/SOURCES.md` counts 82 WAVs: 76, plus `sting_rage`, plus the five radio clips. Both new rows are kept.
  - `audio.rs` plays `Escaped | Dawn` as the dawn and `RadioTuned` as the squeal.
  - The AGENTS.md pure list gains both `pacing` and `radio`.
  - This file keeps every block: the route follow-up, then the spectator follow-up (item 9's), La Rabia, El Respiro and La Voz del Llano, plus main's branding section.
  - `tuning.rs`, `sim.rs` (`base.at_rage(stage).at_pressure(p)` in `update_threat`, plus the leash), `script.rs`, `tests/session.rs` (radio test beside the respiro tests) and `gen_audio.py` (both tables) merged as described. main merged without conflict (`Cargo.toml`, `Cargo.lock`, `build.rs`, `src/icon.rs`, `src/ui/splash.rs`, `tools/package.ps1` as main has them).
- Audio: `python tools/gen_audio.py` rewrote all 82 clips and the tree stayed clean. Every tracked WAV's blob equals the one each branch shipped (03a0b1a 77, rage 78, respiro 77, radio 82, spec 77, main 73; none differ).
- User decisions, as merged: `Outcome::Dawn` on (1.5 x `night_length` 1200 = 1800 s); `anima_sight` true; `bleed_after_sack` 30 s (read as a floor); the name "El Silbón — The Return"; no fullscreen key.
- Wiring (the wiring commit):
  - **Dawn.**
    - `net::watched` returns None once the outcome is over. Any ending, Dawn included, gives the fallen their own eye, heart and HUD back: the camera, crosshair, controls line and the omens' eye all follow it.
    - The session's snapshot stops sending the friend's `watched` vitals once the night is over; `Watch` was already refused then.
    - The shared status line gains a SHARED DAWN line, the render smoke names the stage `dawn`, and the route driver's guard names the outcome ("the run ended Dawn before the route meant it to").
    - Already handled by the lanes: the outcome card, the Journal tally (`Ending::Dawn`), the awards (deeds go out on any ending), the dawn sound and the hint.
    - The route tests still accept only Won then Failed. A Dawn in a route is a failure, by design; at 1800 s it lies past every route await.
  - **One strip per laying.**
    - The separate `RelicDelivered` hint ("One more bundle at rest…") is gone. The Madrina's chapter is the laying's only strip, and its last line is the hint for the stage of anger that laying brings. The English texts now break before that line, as the Spanish did.
    - The wording matches the rage lane's levers:
      - 1: he whistles more often (the stalk interval).
      - 2: a lit torch from farther (the lure, +4 m per stage; the stolen torch from 2).
      - 3: tall grass from farther (`grass_sight` 7.4 m).
      - 4: every bone angers him more.
      - 5: the engine calls him.
    - The stinger and the lamps' stutter stay the telegraph (layings within 8 s are one). The anger pips stay on the bones line.
    - At the fifth laying the `AllBonesHome` objective hint (bring the power back, then the truck) still shows beside the strip. It is the objective, not a telegraph, so it stays.
  - **Pacing and La Rabia** already compose: rage scales his abilities (`update_threat`, `light_lure`), and `pace()` sets the warning floor and the leash after that, so the director decides when he may warn or hunt. A new test pins it (below). `Mood.rage` exists, and the director's tests build it by hand.
  - **Spectators.** `RelicDelivered` (the rage telegraph), `RadioTuned` and `Dawn` are all sent to everyone (`None`), so they already reach the fallen, and `shared_with_watcher` needs nothing new. `hud::radio_dots` now measures `radio_reach` from the watched friend, so the watcher reads the pips where the friend hears them (the squeal and the pips were already placed at the radio and heard from the shoulder camera).
  - **Splash.** No lane added a flag or a driver. `Launch::driven()` and the splash test already cover every driver, so they are unchanged.
- Tests:
  - `dawn_ends_the_watch_of_the_fallen_with_the_night`. It was red on the merged tree without the wiring: "no friend's body once the night is over".
  - `his_anger_reaches_a_torch_farther_but_the_grace_still_holds_him_off`, written against the merged tree:
    - At stage 0, a torch at the midpoint of the base and stage-4 lure ranges does not draw him; at stage 4 it does.
    - For the rest of the grace he never warns and never comes inside the 30 m leash.
- Gate on the wired tree: fmt, check, test (lib 124, district 9, session 64 + 3 ignored), and clippy -D warnings are clean.
- Route sweeps (`target\sweep-merge-{normal,gentle,hard}.log`):
  - Normal: solo 148/150, shared 150/150, with the same two failure lines as the merged tree before wiring (bar 146 met). The torch-off fix to the route driver was not needed.
  - Gentle: solo 150/150, shared 150/150 (bar 146 met).
  - Hard (informational): solo 149/150, shared 145/150 (river baseline 150/141). The failing seeds are the same as before wiring:
    - Solo 147: the run ended Won during the ignition step, the respiro lane's route-timing artefact (its solo 108).
    - Shared 13 and 61: player 2 went down on the walk to the truck (Go (44, 30)) with all five laid and stamina 0.03. This is the rage lane's stage-5 lure signature (its Normal shared 26 and 133).
    - Shared 19, 94 and 127: player 1 went down on a bundle or batteries leg in run 1.
  - The merged tree before wiring measured normal 148/150, gentle 150/150, hard 149/145.
  - Normal solo 76 and 141 are new against the river baseline (150/150). Both fail the same way, and 76 was traced (`target\trace-merge-normal-solo-76.log`):
    - Run 2 (the scripted failure run), step `Await Recovered`.
    - The first warning comes at night 191 s: the Peak after the 120 s grace and the Build.
    - He downs the evading walker about 10 s later by the corral approach (63.4, 6.5), with stamina 0.12 and nothing laid.
    - This is the respiro lane's documented solo 84 signature (which now passes). It is not the stage-5 lure.
- Fingerprint: it moves with the merge (hashed files from every lane, `Cargo.lock` from main) and once more with the wiring's one-line condition in `session.rs`. It is not recomputed here; test.3 must be built from this tree.
- Unverified (rendered, user-led): the SHARED DAWN line; the fallen's view cutting back at an ending; the single strip at a laying; the dots for a watcher at the radio; the splash with the merged build.
- Not run: the two-process net smoke (the relay and `WorldView.radio` are new to it) and the rendered game.
- Bounds moved: none. Invariant tests are untouched. Committed on m1b; not pushed.

**Fingerprint ignores line endings.**
- `src/net/protocol.rs`: `fingerprint()` now calls the private `fingerprint_of<'a>(texts: impl IntoIterator<Item = &'a str>) -> u64`, FNV-1a over `text.bytes().filter(|&b| b != b'\r')`.
- This deliberately changes the fingerprint: test.3 will NOT pair with test.1 (old CRLF/test.1 value 0x93dcc314afdc3f17). From now on a Windows CRLF build and a Linux LF build of the same commit DO pair.
- Only `\r` bytes are dropped, so an LF checkout hashes exactly as before (unchanged HEAD on Linux: 0x656b174ca193ce45); only the Windows CRLF value moves.
- This tree with only this item: 0xa7c36c4384a95020 on both platforms. From the Python model of the hash, which reproduces the real Rust test.1 value from HEAD re-CRLF'd. A checkpoint, not the test.3 value: items 6-11 edit hashed files again.
- New hashed files later (pacing.rs, errand.rs, remedy.rs, radio.rs) just join the `GAMEPLAY` list in `protocol.rs`.
- Test `a_windows_and_a_linux_checkout_of_one_commit_share_a_fingerprint_but_edits_do_not`: CRLF and LF texts agree; a changed or removed line still refuses.
- Verified: fmt, check, test (lib 87, district 7, session 41 + 3 ignored), clippy -D warnings clean. Route sweep: normal solo 150/150, shared 150/150; gentle 150/150, 149/150 (shared seed 124, = baseline); hard 136/150, 134/150 (= baseline; the test's own 97% assert fires, not the bar), failing seed set identical to HEAD's. No newly failing seeds.
- Net smoke (headless, two processes, 127.0.0.1:5311): run 1 host `NET SMOKE FAIL: the player went down before the route meant it to` (run 1, night 155 s; handshake, delivery and marks had succeeded), client then failed on the closed session. Run 2: both `NET SMOKE PASS`, exit 0. Gameplay is unchanged (seed sets identical), so run 1 reads as a pre-existing real-time smoke flake.
- Unverified: a real Linux LF build pairing with a Windows CRLF build.
- Bounds moved: none. Committed on m1b; not pushed. Sweep logs `target\sweep-{normal,gentle,hard,hard-head}.log` are gitignored.

**Skill-check miss (item 6, roadmap fix 2).**
- Pure `skill::Rhythm`: a miss (the needle sweeps past, or a press outside the zone) now throws the work back `check_setback` 2.5 s of that task's work and stalls the hands for `check_stall` 1.8 s of holding on (new `Pulse::Stalled`: no progress, and the wait for the next check does not count down). A refused claim never stalls.
- The stall survives `rest()`. It is only won back by holding a task (altar, pump or ignition), so letting go and grabbing again cannot skip it. A player who misses at the pump and goes to the altar starts there stalled (shaken hands).
- `Rhythm::allow(progress, step)` returns 0 while stalled and never goes past `skill::LAST_TURN` (0.99) while a check is in flight. `resolve_holds` gates the altar, pump and ignition work on it, so finishing a task can no longer drop a pending check. The altar's great-press cap uses `LAST_TURN`.
- Miss noise is `max(noise_miss 40, the task's work noise × noise_miss_over 1.6)`: altar 40 m, pump 54.4 m, truck 120 m (a backfire).
- `tuning`: `check_loss` removed; `check_setback`, `check_stall` and `noise_miss_over` added.
  - Deliberate deviation from the investigation (fixPlan 2e): these are not scaled by Night. Gentle ×0.7 would put the setback at 1.75 s, under the 2.05 s a check lasts (warn + sweep + latency), which breaks the new invariant that a miss always ends behind where its warning found the work. Hard ×1.2/×1.25 would add cost where the sweep has the least margin. Tune per night after a playtest.
- Wire: `Vitals.stall` (seconds left, `#[serde(default)]`). HUD: while stalled the hold label becomes "The bones slip… / The crank kicks back… / The engine floods…". The first-check hint now says the work slips back. No hidden AI state is added.
- Tests:
  - Pure: `a_miss_throws_the_hands_off_and_only_work_wins_them_back`, `a_press_outside_the_zone_stalls_but_a_refused_claim_does_not`, `a_pending_check_holds_the_last_turn`, plus a tuning invariant: for every Night, setback > warn + sweep + latency, stall > 0 and the miss is louder than the work.
  - Session, written first against the old API; each failed for its root cause:
    - the rewritten miss half of `long_tasks_ask_…`: "a miss costs more than the check let him win: 0.5533328 after, 0.42666632 at the warning". Now the pump must end below the warning's mark, hold flat through the stall, then rise.
    - `a_task_cannot_be_finished_under_a_pending_check` (pump and truck): "finished under a pending check (truck: false)".
    - `a_miss_carries_beyond_the_noise_of_the_work_it_spoils` (pump and truck): "he heard the miss over the work (truck: false)"; the 40 m miss did not reach 1.25 × the crank's 34 m. It zeroes `rain_mask` and `thunder_mask` (masking 1), pins him 1.25 × the work noise's reach away every tick, and asserts `focus` stays None under the work's own pulses and becomes Some on the miss tick.
  - Also `ignoring_the_rhythm_finishes_well_behind_keeping_it`: pump 18.9 s vs 9.6 s, 2 misses; truck 10.7 s vs 5.8 s, 1 miss; the bound is setback + stall = 4.3 s.
  - The co-op pump test now answers checks for both players (`Rig::work_all`) and subtracts any great-press bonus from the solo sum; the 0.05 tolerance is unchanged (this seed: 0 great presses, solo 0.498). It measures summation.
- Fingerprint checkpoint for this tree: 0xc8f0af5d5e118e17 (Python model, recomputed at commit; it reproduces the previous item's 0xa7c36c4384a95020). Items 7-11 move it again.
- Verified:
  - fmt, check, test (lib 90, district 7, session 44 + 3 ignored), clippy -D warnings.
  - Route sweep: normal solo 150/150, shared 150/150; gentle 150/150, 149/150 (shared seed 124, = baseline); hard 136/150, 134/150 (= baseline; the test's own 97% assert fires, not the bar).
  - The failing seed sets are identical to HEAD's on every night. On Hard, 27 of 30 failure reports are identical to `sweep-hard-head.log`; shared 20, 38 and 46 fail the same way at the same step, with positions moved slightly after the pump. No newly failing seeds.
  - Net smoke (headless, two processes from this tree, 127.0.0.1:5311), run twice: `NET SMOKE PASS host` and `NET SMOKE PASS client` both times; exit codes captured on the second run, host 0 and client 0 (the first run's host exited 0; its client's code was not captured).
  - Invariant tests untouched: hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden. The `tests/session.rs` diff touches only the `Rig` helper, the skill-check tests and the co-op pump test; the tuning test only gains the new invariant.
- Unverified: the stall label on screen; the feel of about 4.3 s per miss (setback + stall) in a real night.
- Open:
  - The cap is per player: a teammate with no check pending can finish the pump or engine while yours is pending, and yours then drops silently.
  - `input_age > 0.3` over a jittery link still rests and drops a check (unchanged).
- Bounds moved: none. Relocated assertion (not a loosening): the old "he heard the screech" check in `long_tasks_ask_…` put him 25 m from the pump, inside the crank's own 34 m pulse, so it passed without the miss noise. `a_miss_carries_beyond_the_noise_of_the_work_it_spoils` replaces it and is strictly harder.
- The Phase 2 miss rule below is marked superseded. README line 120 ("costs work") is still accurate. Committed on m1b; not pushed.

**River (item 7, roadmap fix 1, Phase 1).**
- **Decision:** the caño is waded. This reverses "clearly bounded water … no swimming" (annotated in the district-pass notes below) for the caño only. The marsh, the pond and the creek keep their banks, and there is still no swimming. Do not restore the caño's bank.
- `District::channels` (the caño rect, bound once as `cano`) is carved out of the bank blockers. The boat (`District::boat`, a Furniture rect) and the two mooring stumps (`District::stumps`, Post circles r 0.12) are now layout blockers; `world::district::boat` reads them.
- Pure `geometry::Wade { Dry, Shallow, Deep }` from `Layout::wade(p)`:
  - Rect tests come first, then surfaces (deck and pier are Dry).
  - Authored shallows outside a channel are Shallow.
  - Inside a channel, the terrain depth under `WATER_LEVEL` (-0.13) decides: Dry at or below `WADE_WET` 0.02, Deep above `WADE_DEEP` 0.3.
  - A ford inside a channel is never Deep, and its dry ends at the banks are Dry, so wading starts exactly at the drawn waterline. (Deviation from the investigation's "shallow_at → Shallow": the new grid test caught the ford's dry ends being waded.)
- `wading()` is `wade != Dry`. `water_line()` and `rest_height()` (the ground, or afloat 0.2 under the water) are new.
- Tuning: `deep_wade_factor` 0.35 and `noise_deep` 14 (×0.4 crouched). Nobody sprints waist-deep, and crawling is slowed too. Neither is night-scaled.
- Afloat:
  - The session's `ground_height` is `rest_height` (at spawn, on every move, and in the sack), so a dropped or released bundle floats at WATER_LEVEL + 0.15.
  - `Pose::eye` never goes below WATER_LEVEL + 0.15, and the body target is `rest_height + 0.3`.
  - `net::avatars` draws the downed at `rest_height` and the standing at `surface_height` (thigh-deep).
- The ford: terrain lifts shallows inside a channel toward `FORD_BED` -0.3 (smoothed over 1.5 m from its sides), so the caño sheet is paler and clearer over it. The second shallows sheet is no longer drawn inside a basin, and no shore reeds grow on the ford.
- No ground grass grows over water, and the map shades channels paler than the marsh.
- He wades unslowed and silent (sim unchanged). Deep steps, yours and teammates', play the water clips at ×1.4 gain and ×0.85 pitch, never at his position.
- Route driver (`script.rs`: not a rule, not hashed):
  - A new A* class `DEEP` costs 6. This deliberately deviates from the roadmap's "TIGHT" (3). At 3 the pier → lookout leg is a coin flip (about 44 vs 45 m), and default gate seed 4 (`the_routes_hold_through_other_storms_and_missed_frames`) waded into a hunt and failed.
  - The string-pull no longer cuts through deep water that the A* path stayed out of. It did so at the ford's east side (Normal shared 60 and Hard shared 7, which both pass now).
- Tests. Each new one failed first on the old code for its root cause:
  - district `the_cano_is_wadeable_anywhere_and_the_ford_reads_shallower`: red, blocked wading across at x = -6. It also checks depth tiers; a 0.5 m grid over the caño that is free except within 1 m of rails, bridge, ramp, boat and stumps; Dry exactly where the bed is above the water; Deep exactly past `WADE_DEEP`; the ford bed more than 0.2 above the channel's; the boat solid; the marsh still a wall.
  - district `a_body_in_the_channel_floats_where_a_friend_on_the_bank_can_reach_it`: red, "drowned view" at y -0.31.
  - session `wading_the_deep_channel_is_slow_and_heard_farther_than_the_ford`: red, "waded 8.14 m in 3 s, not 3.78" (pushed out of the bank).
  - session `what_falls_in_the_channel_floats`: red, "the dropped bundle sank" at y -0.30.
  - body `deeper_water_is_slower_and_louder_and_waist_deep_there_is_no_sprint`: red, speeds [3.6, 1.98, 1.98].
  - Inverted on purpose (the reversed decision, not a loosening): `tests/district.rs` :85-87 "deep water is a wall" became "you wade into it", and :105 `!wading(10,-65)` became `wade == Deep`.
  - `one_solo_seed` and `one_shared_seed` now honour `ROUTE_NIGHT` (they were Normal only).
- Verified: fmt, check, test (lib 91, district 9, session 46 + 3 ignored), clippy -D warnings clean; the same when rerun at commit.
- Route sweep on the final tree, rerun and identical (and again at commit): normal solo 150/150, shared 148/150; gentle 150/150, 150/150; hard 150/150, 138/150 (the test's own 97% assert fires on hard, not the bar).
- Newly failing against the branch baseline (`sweep6-*`), each traced with ROUTE_LOG and a temporary position trace (since removed; at commit each failure line matched the traced run exactly):
  - Normal shared 35 and Hard shared 4:
    - Bundle 4 lay at its ground site by the base shed (52.5,-87.5). From the ramp foot, the local A* box holds neither the bridge nor the bank road, so the driver now wades east of the ramp (x ≈ 45.5). Before, `local()` failed there and the driver walked about 100 m of bank road along the patrol network.
    - Coming back, it was warned waist-deep, turned back, and was hunted down on the far bank (Hard 4: at the south waterline, (41.5,-69.9)). That is deep water doing its job on the fiction's shortcut.
  - Normal shared 11:
    - The whole run was dry (bridge, ramp, deck). Coming down the ramp, player 1 was warned from the far bank (28.9,-77.3).
    - The hunt went straight across the channel, waist-deep and unslowed (the Phase 2A decision), where the bank used to send him round by the bridge.
  - Hard shared 126:
    - No water at the end. Player 1 was caught on the ramp from the far bank west of it (39.5,-74.2, him about 36,-74).
    - That is the signature of baseline Hard shared 5, 30, 43, 70 and 93, reached through a timeline that shifted earlier in the night. The cause of the shift was not traced (neither player 1 nor he waded deep that night).
  - Hard shared 123:
    - Player 1 took the base-shed wade above.
    - Player 2 then died on the walk to the truck (Go (44,30)), caught near (9,14.6) fleeing west on 0.03 stamina. That is the pre-existing pattern of baseline Hard shared 38.
  - No longer failing: Gentle shared 124. Hard solo 17, 38, 44, 51, 72, 75, 81, 87, 95, 97, 101, 102, 117, 118. Hard shared 28, 34, 63, 70, 73, 75, 82.
  - Hard solo 17 was traced: its night diverges early, so the Hard solo gain (136 → 150) is observed, not attributed. (Since attributed: the carved bank walls. See "The route keeps to the bridge" below.)
- Fingerprint checkpoint for this tree: 0x078a48644d371cd7, from the same Python model, recomputed at commit (it reproduces the previous 0xc8f0af5d5e118e17 from HEAD). Items 8-11 move it again. No protocol message or wire field changed, and `Wade` is not on the wire. `src/body.rs` briefly had LF endings during the work and is back to CRLF; the fingerprint ignores `\r` either way.
- Unverified (user-led, rendered):
  - whether the waterline and the wading start match by eye
  - the ford's paler bar
  - whether the boat and stump footprints match their meshes
  - thigh-deep standing avatars and floating downed ones
  - the deep-step sound
  - the map shade
- Net smoke (headless, two processes from this tree, 127.0.0.1:5311), run twice at commit, about 9 min each: `NET SMOKE PASS host` and `NET SMOKE PASS client` both times. Run 1: client exit 0; the host outlived the 540 s wait and its code was not captured. Run 2: host 0, client 0.
- Open:
  - The marsh and the pond keep the dry-shore invisible wall along their rectangles (Phase 2D).
  - The reflex's `rate()` still counts deep water as only +3 wet, so a hide spot waist-deep is a trap.
  - Tureco crosses the caño unslowed, like him.
  - The catch cinematic's fallen eye (`lunge_frame` at `player.rs` :613 and :694, and `world/silbon.rs` :1031) still reads `surface_height`, so a player caught waist-deep watches the catch from about -0.31, under the water sheet. This is presentation only, outside this item's three sites. (Done: see "The route keeps to the bridge" below.)
  - Phase 2 (soaked torch, drifting bundle) has not started.
  - Lore page 5 is untouched.
- Bounds moved: none. Invariant tests are untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden). Committed on m1b; not pushed.

**Downed allies and the help call (item 8, roadmap fix 7, investigation Tier 1 + the call's first kind + `bleed_after_sack`).**
- One gate for every cue that leads a friend to the fallen: `PlayerView::findable()` = downed and not `hauled` (their own row only; in his sack their position is his). The torch, groans, HUD marker, roster distance, map ring and the client crosshair's bodies all read it, so none of them shows while he carries them.
- `Snapshot::scene_data` now counts only findable bodies (it counted the sack as a revivable body, which the session then ignored).
- **Protocol hygiene.** The fingerprint change is on purpose; test.3 will not pair with test.1. `Action::Call` is added at the end of `Action`. `Snapshot.calls` has `#[serde(default)]`. No hidden AI state goes on the wire.
- **The help call** (the call wheel's first kind, no separate help flag):
  - Wire: `Action::Call { kind: CallKind }` appended after `DriveOff`, accepted before the `is_active` guard like `Ping`; `CallKind { Help }`; `Snapshot.calls: Vec<CallView { kind, pos, left, by }>` with `#[serde(default)]`. No new `Event` codes, no hidden AI state.
  - `Session::call`: refused for the captive, the dead and (for `Help`) anyone on their feet; per-player `call_wait` (`call_cooldown` 8 s). It pushes `noise_call` 18 m from the caller's own body (masked by rain like any noise; he hears it only Stalking and Present) and a call at `(pos.x, rest height, pos.y)` whatever the client claims, living `call_life` 3 s.
  - Client: `Intent.call`; `read_devices` turns V into `Call { Help }` while down (and into nothing while hauled). A downed player's V is no longer a mark. The session still accepts a `Ping` from the downed or the captive as before (unchanged rule; the game's client no longer sends one).
- **`bleed_after_sack` 30 s** (user decision): `release_captive` on both release paths (the ward's `SackDropped` in `threat_step`, Tureco's bark in `dog_step`). **Read as a floor** (`bleed.max(30)`), not a reset: a fall dropped at once keeps its longer bleed. Confirm this reading with the user.
- Presentation:
  - Ground torch: `world::avatar::GroundTorch` (top-level, one per teammate, hidden unless findable with `light`), placed `GROUND_TORCH` (0.4, 0, -1.8) from the body (past the reaching hand, kept out of walls with `move_circle`), beam low along the ground turned by their yaw and clamped pitch, so it sweeps as they look around. Lights carry `GroundTorchLight`; the render smoke census excludes them and checks one ground torch per teammate. The hand beam stays hidden for any non-zero status (the prone pose aims it at the sky). No battery gutter (charge is not on the wire for others).
  - Groans: `audio::party_cries`, placed at the head, every `Tuning::groan_gap` (3.5 s at no bleed left to 8 s at full, ±20% from `Rng::fork(seed ^ id, count)`, clamped to `groan_every`), first one at once, pitched by `Survivor::voice()` (Llanero 0.9, Coplera 1.2, Encargado 0.82, Muchacho 1.08). Never a session noise.
  - Calls: `audio::play_calls`, placed at the caller, their voice, lifted like a mark (`mark_lift`); a hint for a friend's fresh call.
  - HUD: `hud::downed_markers` (shared play only; a diamond in the slot colour and "who · N m", "· Ns" under 20 s; pinned to the left or right edge with ‹ › when out of view; pulses faster as they bleed; a fresh call flashes it). Roster: "DOWN 42s · 23 m", or "IN HIS SACK 42s" with no distance. Downed panel second line (crawl, V, F); banner, controls line and briefing mention the cry. SackDropped hint rewritten; DogBark hint down to priority 2 so it no longer overwrites it in the same batch.
  - Map: a pulsing red ring round a findable friend's dot (border only, so the dot recolouring never paints it). The hauled dot still blinks red and tracks him (decision D1, left open).
- Audio: `downed_groan_{0,1,2}.wav` and `call_help.wav` from `make_groan` / `make_call_help` on seeds `SEED+127..130`. Regenerated into `assets/audio`: `git status` shows exactly the four new files, all 72 existing ones byte-identical (checked before too: a fresh regeneration reproduced all 72). A second run reproduces the four new ones byte for byte. `assets/SOURCES.md`: 76 WAVs, two rows.
- Tuning added (none night-scaled): `bleed_after_sack` 30, `call_cooldown` 8, `call_life` 3, `noise_call` 18, `groan_every` (3.5, 8), `groan_gain` 0.55.
- Tests:
  - Red first on the old code, each for its root cause: `a_body_in_his_sack_is_no_crosshair_target_until_he_drops_it` ("the sack is no body to kneel beside"), `a_dropped_sack_leaves_time_to_find_them` ("5.0 s left to find them (bark: false)"; ward and bark paths, plus a 50 s bleed kept).
  - Written with the new API: `a_cry_for_help_comes_from_the_body_and_draws_him_within_earshot` (still, dry night; a Stalking, Present threat at 0.8 × the call's reach hears it at the body, at 1.25 × does not; the shared call lies at the caller's body at rest height), `a_cry_for_help_waits_for_breath_and_fades`, `nobody_cries_for_help_from_his_sack_the_grave_or_their_feet` (captive, dead and standing refused; `findable` false in the sack and for the dead, true once dropped).
  - Lib: `tuning::the_fallen_groan_more_often_as_they_bleed`, `survivor::each_friend_is_known_by_their_voice`.
- Fingerprint checkpoint for this tree: 0x8d808c5460a8c897 (Python model of `fingerprint_of`; it reproduces HEAD's 0x078a48644d371cd7). Items 9-11 move it again.
- Verified: fmt, check, test (lib 93, district 9, session 51 + 3 ignored), clippy -D warnings clean; the same when rerun at commit.
- Route sweep on the final tree, rerun at commit and identical: normal solo 150/150, shared 148/150 (11, 35); gentle 150/150, 150/150; hard 150/150, 138/150 (the test's own 97% assert fires on hard, not the bar). Every failure report on every night is identical, character for character, to HEAD's `sweep7v-*` (only the assert's line number moved). No newly failing seeds (routes never call and never outlive a sack drop). Normal shared 11 and 35 and hard shared 4, 123 and 126 fail against 3288232 but came with item 7. With `ROUTE_LOG`, 11 and 35 fail at the first down, before any sack or call. Logs `target\sweep8f-*.log`, and at commit `sweep8v-*`.
- Net smoke (headless, two processes, 127.0.0.1:5311):
  - Implementer, release build, run once: `NET SMOKE PASS host` and `NET SMOKE PASS client`. Exit codes were not captured. Logs `target\net8-*.{out,err}`.
  - At commit, debug build, about 9.5 min: both PASS, host exit 0, client exit 0. Logs `target\net8v-*`.
  - Every snapshot carried the new `calls` field over real UDP, and it was always empty. The route never calls, so `Action::Call` itself was not sent there.
- Audio at commit: `tools/gen_audio.py --out` into a scratch folder reproduced all 76 WAVs, byte for byte, the four new ones included.
- **Line endings.** `src/net/mod.rs` and `tests/session.rs` were LF in the working copy and were converted back to CRLF to match the checkout; every changed file is CRLF. Git stores LF, and the fingerprint ignores `\r`.
- README and `docs/PLAYING.txt` controls now say V is a cry for help while down.
- Unverified (user-led, rendered): the lying torch's look and brightness in tall grass (the trailer's `91_survivors_down` mates are `down`, `light` and not hauled, so they now show one); groan and cry levels, pitch and whether the cry reads as "¡Auxilio!"; the marker's placement, edge pins and the ‹ › glyphs; the map ring; the two-window find-them check (`--host 127.0.0.1:5000` / `--join 127.0.0.1:5000`: get one player hauled, pepper the path, then find them by groan, torch and marker); the rendered net smoke (new ground-torch census).
- Open:
  - `Deeds.calls` and the "Called for mamá" / "Silbó de noche" awards are not built (roadmap section 3 item 2, M2).
  - D1 (the hauled dot and `PlayerView.position` track him), D2 (flare), D3 (a cry from nobody), D4 (whether the lying torch lures him; it does not: `light_lure` ignores the non-active).
  - No 3D help marker over the caller (the HUD marker flashes instead).
- Hint priority: the DogBark hint moving from 3 to 2 is presentation only (it no longer overwrites the SackDropped hint shown at the same moment). It is not a rule change.
- Bounds moved: none. Invariant tests are untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden; `tests/session.rs` only gains tests and one import). Committed on m1b; not pushed. Next: spectating (item 9) moves the fingerprint again.

**Spectating (item 9, roadmap fix 8, investigation "spectator" Phase 1).**
- **Fairness.** A player gone for the night (Dead) sees him only when the friend they watch could, from that friend's pose, by the unchanged rules. They hear only the whistle that friend hears. They never get their own dead eye, and never an all-seeing view.
- Session (`net::session`):
  - `Participant::watch: Option<PlayerId>`. `Action::Watch { id }` is appended after `Call` and is accepted before the `is_active` guard, like `Ping` and `Call`.
  - `Session::watch` refuses a run that is not active, a requester who is not Dead (the downed and the living keep their own eyes), themselves, and anyone not on their feet.
  - `watchable` means on their feet (`Status::Active`). **Deviation** from the investigation, which also allowed a downed friend out of the sack: "auto-advance when the watched friend falls" makes the downed no target, or a chosen downed friend would be dropped at once.
  - `watch_step()` runs between `outcome_step` and `flush_events`, so the cues that follow go to the right watcher. A Dead player whose friend is no longer on their feet (fell, sacked, died, left) moves to the standing friend nearest where they lie (ties to the lowest id). Everyone else watches nobody. BTreeMap order, no randomness.
  - `snapshot()`: the viewer is the recipient; for a Dead recipient it is the watched friend, only with `anima_sight` and only while that friend is watchable (looked up with `.get()`, so for the one tick after a friend leaves the dead get no view at all). `threat`, `danger` and `exposure` come from the viewer with the unchanged rules: the 0.55 cone, 75 m (130 m from a deck), line of sight, and danger only when `target` is the viewer and they stand. `me` stays the recipient's own vitals. `PlayerView.watching` carries whom each player watches.
  - `cues()`: each Cue pushed for a listener is cloned, with the same serial, to every Dead player watching them, in the same pass. The outbox is FIFO per recipient and the serial is global, so the client's `serial > last_serial` rule keeps it. The dead's own `CueDirector` never ticks and nothing startles them. Omens and other `Some(id)` events (warning, hunt, susto, Tureco's growl) are not forwarded.
  - `Participant::die()` (bleed-out and Taken): Dead, revive 0, stun 0, not sprinting. The Taken no longer keep a stale "Frozen with fright" (stun only thaws in `Body::advance`, which captives and the dead skip).
- `Tuning::anima_sight` (default on, user decision). Off, the dead never get threat, danger or exposure; they still ride their friend's shoulder and hear their friend's whistle.
- **Protocol hygiene.** The fingerprint change is on purpose; test.3 will not pair with test.1. `Action::Watch` is appended at the end of `Action`, `PlayerView.watching` has `#[serde(default)]`, and no hidden AI state goes on the wire.
  - **Deviation:** `PROTOCOL` (renet's `protocol_id`) is not bumped. A mismatch there fails the UDP handshake silently, where the fingerprint refusal is readable. Item 8 appended `Call` the same way.
  - **Deviation:** no `Snapshot.watched` vitals in Phase 1 (see Open).
- Client:
  - Pure `net::watched(snapshot, me)` (only for status 2, only while that row stands) and `net::cycle_watch(snapshot, me, step)` (friends on their feet in roster order, wrapping). `Network::watched`, `spectating` and `watch_label`.
  - `spectate_keys` (Control, after `net_keys`; not in the smoke, net smoke or photo drivers; only Playing, running and status 2): A, ← or left click watch the previous friend; D, → or right click the next. Each sends `NetControl::Action(Action::Watch)`. The dead's input packet still carries nothing.
  - Camera: `net::update` (Simulate) writes `player::shoulder_transform` from the watched row (position, yaw, pitch, crouch), eased at 18/s like their avatar and cut on a new friend, a new run or a jump over 4 m. So Ears, omens, pings and lamps all read the spectator eye in Present.
  - `shoulder_transform` (presentation only): 2.2 m back, 0.45 m right and 0.45 m up from their eye, kept 0.25 m off blockers with `move_circle`, and pulled in (×1, 0.75, 0.5, 0.3, 0.15, else over their head) until `line_of_sight(friend, camera)` holds. It looks at their eye + look direction × 6 m. Every coordinate comes from `Layout`.
  - The hand torch is hidden and dark while spectating (the friend's own beam lights the way), and the head bob is off.
  - Audio: whistles play for the dead while they watch (`play_whistles` and `mix` both muted status 2 outright before).
  - HUD: "YOU DIED" holds 3 s (`DIED_HOLD`), then a light cold wash (0.14). A new `WatchLine` (bottom centre, spawned once at startup) reads "Watching La Coplera · A / D another friend", with the keys only when there is another. The status panel speaks of the friend ("He has seen …", "He is coming for …!", "… is out of his sight… for now"). The dead get "A friend is down." and "The sack falls!" without "hold E". The banner shows the watch line instead of "You bled out. Watch over your teammates.", which was also wrong for the Taken.
  - README and `docs/PLAYING.txt` controls: A / D while gone for the night.
- Tests:
  - Red first on the old code, each for its root cause:
    - `a_taken_player_is_not_left_frozen_with_fright`: "the dead are not frozen with fright (1.98 s)".
    - `the_fallen_see_him_only_through_their_friends_eyes`: "the fallen see him through their friend's eyes" (no threat for the dead). It also checks equal danger and exposure; the friend turned away hides him; and a wall between friend and him hides him even where the dead body itself faces him with a clear line of sight. After the fix, `anima_sight = false` was added: no threat, danger 0, exposure 0.
    - `the_fallen_hear_the_whistle_their_friend_hears`: the dead host got no Cue (left `[]`, right `[(1, 2, 1.0495794, false, 1)]`). The friend hears it faint near him; from where the host lies it would have been loud.
  - Written with the new API: `the_fallen_watch_a_friend_on_their_feet` (four players). The nearest standing friend is chosen; `watched` and `cycle_watch` step round; refusals for a downed requester, themselves, an unknown id and the living. When the watched friend leaves, the snapshot still builds without him and the next tick moves on. When the watched friend is sacked, the watch moves on and the sack is refused. A restart clears every watch.
  - Lib: `player::a_friend_is_watched_from_their_own_side_of_every_wall`: a 30 × 30 m grid around the house × 8 headings. The friend always has line of sight to the camera, which looks where they look; both pulled-in and full-distance views occur.
- Fingerprint checkpoint for this tree: 0x44387ca6b9e1452e (Python model of `fingerprint_of`, recomputed at commit; it reproduces HEAD's 0x8d808c5460a8c897). Items 10-11 move it again.
- Verified: fmt, check, test (lib 94, district 9, session 55 + 3 ignored), clippy -D warnings clean; the same when rerun at commit (clippy forced to recheck every target; `target\gate9i-*.log`).
- Route sweep on the final tree, rerun at commit and identical: normal solo 150/150, shared 148/150 (11, 35); gentle 150/150, 150/150; hard 150/150, 138/150 (the test's own 97% assert fires on hard, not the bar). Every failure report on every night is identical, character for character, to item 8's `sweep8v-*`. No newly failing seeds. Normal shared 11 and 35 fail against 3288232 but came with item 7; both fail at a first down (downs 1, revives 0, status 1), so no player was Dead yet. Solo never has a Dead player; in the shared route the dead partner's new view changes only its script's `seen`, which never moved an outcome. Logs `target\sweep9-*.log`, and at commit `sweep9i-*`.
- Net smoke (headless, two processes, 127.0.0.1:5311, debug build of this tree):
  - Run 1: the host failed with `NET SMOKE FAIL: the player went down before the route meant it to`, in run 1 at 250 s (Go (43,-59), delivered 3/5, at (44.8,-81.9)). The client then failed on the closed session; both exited 1. Nobody had died yet, so none of this item's code had run (no watcher, no forwarded cue; `die()` untouched). This is the known real-time flake (seen in the fingerprint item), with the signature of the base-shed wade that item 7 brought to Normal shared 35.
  - Run 2, about 9.5 min: `NET SMOKE PASS host` and `NET SMOKE PASS client`; host exit 0, client exit 0. Every snapshot carried the new `watching` field over real UDP. The route never sends `Watch`.
  - At commit, debug build, about 9.5 min: both PASS, host exit 0, client exit 0.
  - Logs `target\net9-*`, `target\net9b-*` and, at commit, `target\net9i-*` (gitignored).
- Unverified (user-led, rendered, two windows `--host 127.0.0.1:5000` / `--join 127.0.0.1:5000`; let one player die, the sack and Taken being the quickest): the shoulder camera's feel, and whether it is pulled in well against walls, fences and the deck; the 3 s hold and the cold wash; the watch line and the status panel's third-person lines; whistles heard at the right level while watching; A / D and the mouse buttons switching; the auto-advance when the friend is sacked.
- Reasoned, not observed: a real client applying a forwarded Cue (its serial equals the friend's) and then the Events that follow it. `flush_events` runs before `cues()` in each step, so a step's Events carry lower serials than its Cues, and the next step's are higher; the route pilots never read cues, so no test or smoke plays this path.
- Open:
  - The dead's own frozen vitals still drive their vitals panel, the vignette's cold edge and the heartbeat while they watch (pre-existing). Either send the friend's (`Snapshot.watched`, the investigation's 3g) or quiet them at death.
  - Events only the friend is sent (the hunt sting, warnings, susto, omens, Tureco's growl) are not forwarded, so the watcher sees "He is coming for …!" but hears no hunt sting.
  - Not built: the roster's "watching Pn", a ring on the watched dot on the map, a mention of A / D in the in-game controls line.
  - Any left or right click while watching switches friend, a reflexive click to take the cursor back included. Drop the mouse buttons if that annoys in play.
  - The crosshair dot still draws over the friend's back while watching.
  - The routes never send `Watch`; the sweep and the net smoke exercise the automatic watch only.
  - `AGENTS.md` (perception boundary) could note that a Dead recipient's snapshot uses the watched friend's eye with `anima_sight`; not edited here.
  - Haunts (Phase 2, Ánimas) are M2.
- Bounds moved: none. Invariant tests are untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden; `tests/session.rs` only gains tests, a helper and two imports). Committed on m1b; not pushed.

**The fingerprint covers the seeded generator and the world noise (after test.3).**
- Where pairing stands:
  - `v0.1.0-test.3` is tagged at 89f1ede. Its fingerprint is 0x44387ca6b9e1452e, from a real headless host run of 89f1ede's hashed sources (only the unhashed `transport.rs` print added). It equals the item 9 checkpoint from the Python model, recomputed again at commit from 89f1ede's git objects.
  - test.3 does not pair with test.1 or test.2 (both 0x93dcc314afdc3f17 on Windows).
  - From 18b47f8 on, a Windows (CRLF) and a Linux (LF) build of one commit share a fingerprint.
  - This tree: **0xd335073e06e8cde2**, printed by both processes of a real net smoke (`target\fp11\net-{host,client}.err`, and at commit `target\fp12\net-{host,client}.err`). The Python model `target\fp.py` (now with `src/rng.rs` and `src/noise.rs`) gives the same value. It does not pair with test.3: `rng.rs` and `noise.rs` join the hash, and `protocol.rs` gains a test. A checkpoint: items 10-11 edit hashed files again.
  - Superseded checkpoint: 0x34bb1c0c537e2997 (`target\fp10`) was this item before `noise.rs` joined the list.
- `src/net/protocol.rs`:
  - The hashed sources are now the `GAMEPLAY` const (15 texts), and `fingerprint()` is `fingerprint_of(GAMEPLAY)`, still without `\r`.
  - `src/rng.rs` is hashed. `geometry/district.rs`, `net/session.rs`, `sim.rs`, `storm.rs`, `perception.rs`, `skill.rs` and `director.rs` all draw from `rng::Rng`. A change to the generator deals another district, storm and night from one seed, and before this change it still paired.
  - `src/noise.rs` is hashed after it, before `Cargo.lock`. `district.rs` imports `fbm2`, and it decides two things:
    - Which tree trunks are kept (:1104). Kept trunks become host-authoritative `Blocker { Shape::Circle, sight: Sight::Blocks, kind: Trunk }` (:1368-1373), which block movement and line of sight.
    - The terrain height (:1234, :1246, :1253). `Layout::wade` reads that height to choose Dry, Shallow or Deep, which sets wading speed and noise.
    - So a change to the noise dealt other cover and other wading from one seed, and before this change it still paired.
- `src/net/transport.rs`: `Endpoint::new` prints `NET fingerprint 0x…` to stderr when hosting or joining (`!mode.is_solo()`). It prints once the socket is set up, so a refused or unbindable address fails without printing a value. `transport.rs` is not hashed, so the print does not move the value. Every networked run and the net smoke log now show it.
- Test `a_build_whose_seed_deals_another_night_does_not_pair`:
  - For `rng.rs` and `noise.rs`, it appends a comment to that source, substitutes it into `GAMEPLAY`, and requires a fingerprint different from this build's. Appending, instead of editing a constant, keeps the test independent of how either file is written.
  - Red first, with `rng.rs` hashed and `noise.rs` not: "a build with another noise.rs paired" (both sides 6649486625843381237). The `rng.rs` case passed. Earlier in this item, with neither file hashed, the `rng.rs` case was red: "a build with another generator paired". Green after each file joined the list.
- **Protocol hygiene.** No message, wire field or enum changed. `PROTOCOL` is not bumped, the same deviation as items 8 and 9. No hidden AI state is on the wire.
- Verified:
  - fmt, check, test (lib 95, district 9, session 55 + 3 ignored), clippy -D warnings clean; the same when rerun at commit. Logs `target\fp11\gate-*.log`, and at commit `target\fp12\gate-*.log`.
  - Route sweep on the final tree (`noise.rs` hashed), at commit: normal solo 150/150, shared 148/150 (11, 35); gentle 150/150, 150/150; hard 150/150, 138/150 (the test's own 97% assert fires on hard, not the bar).
    - Every failure report on every night is identical, character for character, to the previous commit's `sweep9i-*`. No newly failing seeds: no gameplay source changed, and `rng.rs` and `noise.rs` themselves are unchanged. Logs `target\sweep12v-*.log` (and `target\sweep10fp-*.log`, the same, from before `noise.rs` joined).
  - Net smoke (headless, two processes, 127.0.0.1:5311, debug build of this tree):
    - Implementer, run once: `NET SMOKE PASS host` and `NET SMOKE PASS client`; host exit 0, client exit 0. Logs `target\fp11\net-*`.
    - At commit, run once, about 9.3 min: both PASS, host exit 0, client exit 0. Both first lines read `NET fingerprint 0xd335073e06e8cde2`, and the host accepted the client. Logs `target\fp12\net-*`.
- Unverified: a real Linux LF build pairing with a Windows CRLF build (the unit test and the Python model agree that they do).
- Open: other unhashed sources that the hashed ones use:
  - `src/survivor.rs`: `Survivor::assign` (who gets which survivor) runs on the host. `code`/`from_code` map the wire byte to a survivor. This is identity, not layout or rules.
  - `src/awards.rs`: `Deeds` is on the wire (`Snapshot.deeds`, `#[serde(default)]` on the struct). The awards are computed on each client (`ui/mod.rs` :1015), so this is presentation only.
  - Done: `src/noise.rs` (above).
- Bounds moved: none. The only test change is a new assertion in `protocol.rs`; invariant tests are untouched.
- The commit holds exactly three files: `docs/PROGRESS.md`, `src/net/protocol.rs` and `src/net/transport.rs`. No asset touched. `target\fp.py`, `target\fp11\*` and `target\fp12\*` are gitignored evidence. Committed on m1b; not pushed.

**The route keeps to the bridge, and the caño hides no one (follow-up to item 7, 91bdfd7).**
- **Decision (route driver only):** the scripted route wades waist-deep only where no dry way exists anywhere. `script.rs` is debug tooling, not a rule, and is not hashed.
- The problem: from the lookout's ramp foot (43,-54) the base-shed bundle (52.5,-87.5) lies straight across the caño, and `route()`'s local box holds neither the bridge nor the bank road. Since item 7 the box's A* therefore waded east of the ramp. The trail network is only consulted when the box fails, and it would not have saved the walk either: its off-leg from node (43,-54) may wade too, and legs are compared by plain length, so that wading leg would win. Hence the dry pass keeps the network's legs dry as well. This wade was Normal shared 35 and Hard shared 4, and the net smoke's run-1 flake had its signature.
- `script.rs`:
  - New `Water { Wade, Dry }`. `Nav::local` takes one. `Dry` makes waist-deep cells impassable through a single `passable` test, used for the cell itself and for both corners of a diagonal. The start and goal cells get no exemption, so a goal afloat mid-channel has no dry way.
  - `route()` is now `route_by(Dry) || route_by(Wade)`:
    - The dry pass is the old search (local box, then the trail network) with dry legs. The network's own edges are all dry: the bridge deck, the ford (shallow) and the bank road.
    - The wading pass is exactly the old search, with DEEP costing 6.
    - The two passes differ only in the cells the old search could wade, so a walk that stayed clear of deep water keeps its path. Only an exact tie between equal-cost paths could still resolve differently.
  - Kept in `Wade`: the reflex's `way()` (its hide paths) and the ramp foot ↔ deck legs in `plan()`.
  - Why neither of the task's other options:
    - At DEEP 6, a wider box would still wade: about 9 m waist-deep plus the bank is roughly 80 cost against about 89 m dry.
    - A bridge or ford waypoint is what the dry trail network already gives.
  - The dry way from the ramp foot to the shed runs over the bridge and along the bank road south of the deck: 88.9 m out and 90.3 m back, against 34.8 m straight.
- Test, red first: lib `script::tests::the_walk_keeps_out_of_the_cano_while_a_dry_way_exists`.
  - Red: "waded from [43, -54] to [52.5, -87.5]: [(46.875, -62.125), (46.875, -77.125), (52.5, -87.5)]", straight across east of the ramp.
  - It checks both ways, and that a goal afloat at (34,-65.5) is still reached.
- Normal shared 35 and Hard shared 4 now pass. Their traces (`target\cano\trace-fix-*`) have no position in the deep channel. Item 7's traces waded east of the ramp at 201 s and 194 s.
- **The caño hides no one.** The session rule is unchanged: the new test passed the first time it ran.
  - `tests/session.rs` `the_cano_hides_no_one`, with a `fate` helper, sets up this scene:
    - a lone player keeping still waist-deep at (34,-65.5)
    - him 10 m off on the north bank (inside the crouched notice of 13.2 m, beyond `grass_sight` 5 m), Stalking, Present, Still, cooldown 0
  - The steps of WarningBegan, HuntBegan and Downed must equal those of the same stand on open dry ground, standing and crouched.
  - The dry spot is found by search: dry all the way in, clear for his 0.45 m circle, unlit, not tall grass, far from Tureco. It is (-60,-50).
  - Standing: warned at step 0, hunted at 204, down at 417, in both places. He wades in after them unslowed.
  - The test has teeth. Two throwaway mutations, each reverted:
    - Concealing a deep wader in `threat_step`'s `Prey`: red, "warned, hunted and caught waist-deep: [None, None, None]".
    - Slowing him ×0.35 in deep water in `step_toward`: red, down at step 455, not 417.
  - Noise is already covered by `wading_the_deep_channel_is_slow_and_heard_farther_than_the_ford` and the body test, so it is not duplicated.
  - No rule and no hashed file changed. The fingerprint stays **0xd335073e06e8cde2**, printed by both processes of every net smoke below, so this pairs with fb6c1ca.
- **Hard solo 136 → 150 after item 7, explained (traced).**
  - The pre-wade failure signature (`sweep6-hard`, 12 of 14 solo seeds): the walker downed at (22.6,-70.3), him last seen at y = -60.58. The other two, 38 and 75, are the same trap with him pinned against the bridge rail (x = 19.66).
  - Hard solo 17 was traced on a real 7690b09 build (a scratch export with its own target dir). A shared target dir silently reused the current library, because the scratch sources had older mtimes; see the note at the end of this item.
    - At 167 s he begins the hunt from the north bank by the bridge (18.2,-54.6). The route is walking from the stilt hut to the ramp foot, along the south bank toward the bridge.
    - The caño's Bank wall, which does not block sight, stops him at y = -60.58 (his 0.42 m radius) but not his gaze.
    - The walker slides west along the south wall at y = -70.30 (the player radius), in plain view 10 m across the water.
    - Exposure takes him: 0.13 → 0.89 in 2.6 s, down at 170 s.
  - On HEAD the same seed matches pre exactly until 138.7 s. There it diverges first, at the ford's south end (the ford's dry ends are now Dry, so the walker leaves the ford 0.67 m sooner), and the seed passes.
  - Controlled experiments on HEAD, each reverted:
    - x1, the old ford speed only: matches pre until 149.5 s, where the walker takes the carved bank at the pier corner. Passes.
    - x2, the old ford and the bank wall restored: the trace never diverges from pre. Fails the same way.
    - x3, the bank wall alone restored: seed 17 fails the same way.
      - The whole Hard sweep drops to solo 132/150, shared 133/150.
      - 16 of the 18 solo failures carry the exact pre-wade signature. The other two, 37 and 75, are him pinned against the bridge rail (x = 19.66) across the water.
      - 13 of the 14 pre-wade failing solo seeds are among them.
  - **Conclusion:** the jump comes from carving the caño's bank walls.
    - The walls pinned both him and the walker at the waterline, so a hunt across the channel became a catch by exposure through the water. Wading removed that trap.
    - It was never the base-shed wade: these runs fail on bundle 3's leg, not bundle 4's.
    - Hard solo stays 150/150 with this fix.
  - Evidence: `target\cano\trace-{pre,head,x1-oldford,x2-oldford-walls,x3-walls}-hard-solo-17.log`, `target\cano\sweep-x3-walls-hard.log` and `target\cano\diverge.py`.
- **The catch's fallen eye** (presentation only):
  - New `world::silbon::catch_frame(s, catch, layout, tuning)`, used by all four callers:
    - `player.rs`: `torch_light` and `head_bob`
    - `world/omen.rs`: the lunge light
    - `world/silbon.rs`: `animate_silbon`
  - `lunge_frame` takes a second closure, `rest`. As the eye falls it goes from `surface + standing` to `rest_height + the downed eye`. This is the same as before on dry land, where the two heights agree. His feet stay on `surface_height` (the bed).
  - Test, red first: `world::silbon::tests::a_catch_in_the_channel_is_watched_from_above_the_water`.
    - Red: "s 0.38: the caught eye is under the water at -0.0335". The message was reworded after the fix.
    - Over the whole catch, for every catch way, the eye stays more than 0.1 above `WATER_LEVEL` and his feet stay on the bed.
  - The existing synthetic-ground catch test passes the same closure twice.
- Verified (the integrator reran the gate, the sweep and the smoke on this tree; the implementer's own logs are `target\cano\*`):
  - Gate: fmt, fmt --check, check, test (lib 97, district 9, session 56 + 3 ignored), clippy -D warnings, all clean. Logs `target\integ\{check,test,clippy}.log`.
  - Bounds audit of `git diff HEAD -- tests src/script.rs src/sim.rs src/tuning.rs`: `sim.rs` and `tuning.rs` are untouched, `script.rs` changes no number, and `tests/session.rs` only adds lines (a test, a helper, `Wade` on one import). The invariant tests are not touched.
  - Route sweep (`target\integ\sweep-*.log`, same counts as the implementer's): normal solo 150/150, shared 150/150; gentle 150/150, 150/150; hard 150/150, 141/150. On hard the test's own 97% assert fires, which is not the bar. The bars were normal 146, gentle 148, hard solo 147 and shared 135.
  - Against the previous commit's `sweep12v-*` (normal 150/148, gentle 150/150, hard 150/138), no seed newly fails on any night.
    - No longer failing: Normal shared 11 and 35; Hard shared 4, 61 and 126.
    - Still failing: Hard shared 5, 20, 30, 38, 43, 46, 93, 123 and 124.
      - 5, 30, 38, 43, 93 and 124 are identical character for character.
      - 20 and 46 (player 2 waited 300 s for Carried(1), the baseline pattern) fail at the same step on a shifted timeline.
      - So does 123 (player 2 caught at (9.2,14.5) on the walk to the truck, the pattern of baseline Hard shared 38): at 326 s, was 289 s. Player 1 now walks round to the shed instead of wading.
  - Net smoke (headless, two processes, 127.0.0.1:5311, one debug build of this tree), **three runs in a row**, run by the integrator and again by the implementer:
    - Integrator: every run printed `NET SMOKE PASS host` and `NET SMOKE PASS client`, with host exit 0 and client exit 0. They took 10.7, 10.2 and 11.6 min. The smoke's own limit is 2400 s, so no cap had to move.
    - Every process printed `NET fingerprint 0xd335073e06e8cde2`, so this pairs with fb6c1ca.
    - Logs `target\integ\net{1,2,3}-{host,client}.err`, `net-codes.txt` and the runner `net.ps1`. The implementer's three runs (10.6, 11.3, 11.5 min, all passing) are in `target\cano\`.
    - The machine was not idle: an unrelated `cargo test` compile in another checkout was still running as run 1 started, and the implementer's own runs overlapped a scratch compile. CPU load only; every run passed.
    - The run-1 flake that item 9 recorded (down at (44.8,-81.9) coming back from the shed) had this wade's signature. It did not recur in these six runs, which does not prove it gone.
- Unverified (user-led, rendered): the look of a catch waist-deep.
- Open:
  - The reflex's own hide paths and hide spots may still wade (DEEP 6; `rate()` counts deep water only +3 wet), so a hide spot waist-deep is still possible (open since item 7).
  - Normal shared 11 now passes, although it neither waded nor went to the shed. Traced against HEAD:
    - The traces match until 140.4 s, where player 1, evading at the pier's north-west corner, walks about 0.5 m farther from the channel. After that the night runs on a new timeline.
    - `target\cano\trace-{head,fix}-normal-shared-11.log`.
    - Hard shared 61 and 126 also pass now. They were not traced.
- Tooling note: a scratch build of another commit must use its own `CARGO_TARGET_DIR`. Cargo hashes path packages relative to their root, so a second checkout sharing `target\` reuses and overwrites this tree's artifacts. After such a mix-up, run `cargo clean -p el_silbon` (dependencies are kept).
- Bounds moved: none (the route bounds, the quick-hands bound and the smoke and route caps are all as they were). Invariant tests are untouched. `tests/session.rs` only gains a test, a helper and one import. No asset changed. Committed on m1b; not pushed.

**Spectator follow-up: the watcher lives the friend's night (branch m1b-spec, from fb6c1ca; follow-up to 89f1ede).**
- Closes three of item 9's Open points: the dead's frozen vitals, the unforwarded friend's events, and the mouse / crosshair / roster / controls-line leftovers. `anima_sight` stays on (user decision).
- **Protocol hygiene.** `Snapshot.watched: Option<Vitals>` is appended at the end of `Snapshot` with `#[serde(default)]`. No new `Event` or `Action`, no hidden AI state on the wire (only the friend's own body, which their own snapshot already carries as `me`). `protocol.rs` and `session.rs` change, so the fingerprint moves on purpose; no checkpoint recorded (the parallel lanes move it again at merge). `PROTOCOL` not bumped (same deviation as items 8 and 9).
- Session (`net::session`):
  - `Participant::vitals()` builds `Vitals` (the old inline `me` block, unchanged). `snapshot()` fills `watched` for a Dead recipient whose `watch` is `watchable`, from that friend's `vitals()`. **Not gated on `anima_sight`**: it is the friend's own fear, breath, peppers and torch, not a view of him.
  - `shared_with_watcher(Event)` (pure, public) is the rule for what reaches the watcher: `WarningBegan`, `HuntBegan`, `Susto`, `DogGrowl` and the nine `Omen*`. `flush_events` sends a `Some(friend)` event to each Dead player whose `watch` is that friend when the rule allows it, in the same `Events` message as their own. Never forwarded: what the friend's hands do (pickups, `Prayed`, `SkillCheck`, `SkillGreat`, `BatteriesTaken`), nor `WarningAverted`, `LostTrack`, `ThreatReturned` (their hints say "you"; the status panel already follows the friend's `danger`). `DogBark` was already broadcast. The whistle path (`cues()`) is unchanged; its whistle-susto goes through `flush_events` and is forwarded like any other.
  - `watch_step` still runs before `flush_events`, so a friend who falls that tick no longer forwards to the watcher (the watch has moved on).
- Client:
  - Pure `net::felt(snapshot, me)`: `watched` while `net::watched` holds, else `me`; `Network::felt()`. It drives the heartbeat level and speed and `dazed` in `audio::mix`, the vitals panel (fear, breath, peppers, torch) and the vignette's cold edge.
  - Omens (`world::omen::frights`) are placed from the watched friend's row (position and yaw), not the shoulder camera 2.2 m behind; the lunge light still reads the camera. A false mark is never in the watched friend's colour either.
  - Hints while Dead: "He has seen your friend.", "He is coming for your friend!", "Susto — fright freezes your friend." (static strings; the status panel names the friend). The hunt sting, growl and susto sounds play as for the friend.
  - `spectate_keys`: keyboard only (A / D, ← / →); the mouse buttons no longer switch.
  - `Crosshair` and `ControlsLine` markers (the literal controls string is untouched, merge-friendly). `hud::watch_line` hides the crosshair while spectating and swaps the controls line to "A / D or ← / →  watch another friend · M map · Esc", restoring the spawned text after.
  - Roster: a Dead row reads "P2 La Coplera  lost · watching P1".
  - README controls row: "A / D or arrows" (the click is gone).
- Tests (`tests/session.rs`, new functions and one import):
  - `the_fallen_live_the_hunt_on_the_friend_they_watch_and_no_other`: three players; the dead host lies beside player 3 but watches player 2. The real AI stalks and warns, then hunts (stepped tick by tick until `HuntBegan`). Hunting 3: 3 gets both, the host neither. Hunting 2: both reach the host. Red first with the forwarding disabled: "the fallen live their friend's WarningBegan: []".
  - `the_fallen_feel_the_fright_of_the_friend_they_watch`: the dead host's own fear 0.95; `felt` follows friend 2's fear and stamina, then friend 3's after `Watch`; the living get `watched: None` and their own; when the watched friend is sacked, the watch moves on and `felt` follows the new friend.
- Verified (lane integrator, on the final tree): `cargo fmt --check`, `cargo check --locked`, `cargo test --locked` (lib 95, district 9, session 57 + 3 ignored), `cargo clippy --locked --all-targets -- -D warnings` all clean. No asset or `tools/gen_audio.py` change; `git status` shows nothing under `assets/`, so all 76 WAVs are the base's bytes.
- Not proven by the gate: `hud::watch_line` gained a crosshair `&mut Visibility` query (`Without<WatchLine>`) and a `ControlsLine` children query. They read as disjoint, but Bevy checks that when the schedule initialises, and no test builds the HUD app; the first rendered launch proves it.
- Route sweep, normal (rerun by the lane integrator on the final tree; log `target\gate-sweep-normal.log`): solo 150/150, shared 148/150 (11, 35), the base's. The log `target\sweep-spec-normal.log` equals the base `target\sweep-normal.log` line for line (only the shell wrapper differs). No newly failing seeds: nothing in the simulation changed, only who is sent which event and a new snapshot field; the pilots never read events. Gentle and hard sweeps and the net smoke were not run (integrator's, after the merge).
- Unverified (user-led, rendered, two or three windows): the heartbeat and cold edge following the friend; hunt sting, growl and susto heard while watching; omens appearing ahead of the friend from the shoulder view; crosshair gone and the controls line while watching and back after a restart; the roster's "watching Pn".
- Open:
  - A Dead player with nobody standing to watch still feels their own frozen vitals (heartbeat, vignette); the night is usually lost or ending by then.
  - A watcher's omen is not the same one the friend sees (each client places its own from its own turn counter); both are from the friend's eye.
  - Not built: a ring on the watched friend's dot on the map.
- Bounds moved: none. Bounds audit (`git diff HEAD -- tests src/script.rs src/sim.rs src/tuning.rs`): `script.rs`, `sim.rs` and `tuning.rs` unchanged; `tests/` holds only the two new tests and the `felt` import, no removed or loosened assertion. Invariant tests untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden).
- Commit and housekeeping: committed on m1b-spec by the lane integrator after the gate; not pushed, and the merge with the other lanes is the main integrator's. Every changed file is CRLF like the checkout (rustfmt had turned `tests/session.rs` into LF; converted back and rechecked every file). `.cargo/config.toml` is untouched and still untracked. No asset or `tools/gen_audio.py` change; `git status` shows nothing under `assets/`, so all 76 WAVs are the base's bytes. The fingerprint moves on purpose (`protocol.rs` and `session.rs` changed); no checkpoint recorded because the merge moves it again. `PROTOCOL` is not bumped, as in items 8 and 9. No hidden AI state on the wire.
- Merge notes (places this branch could collide with the other lanes):
  - `src/net/protocol.rs`: `Snapshot.watched` is the last field, `#[serde(default)]`.
  - `src/net/session.rs`: `Participant::vitals()` is a new method right after `die()`, and `snapshot()`'s `me:` calls it (the old inline block is removed). The pure `shared_with_watcher` sits between `struct Call` and `struct Session`. The filter inside `flush_events` is rewritten in place. `snapshot()` gains a `watched:` initializer after `calls:`.
  - `src/ui/mod.rs`: `Crosshair` and `ControlsLine` structs follow `WatchLine`. A `ControlsLine,` line sits two unchanged lines above the controls-string literal (not edited), so a radio-lane edit to that literal should merge cleanly. A `Crosshair,` line follows the crosshair's `BackgroundColor`.
  - `src/ui/hud.rs`: three guarded `status() == 2` arms go just before the existing `Event::WarningBegan` arm in `hints_and_captions`. `watch_line` gains queries. The roster's `2 =>` arm is changed. The `vitals` and `vignette` fear reads are changed.
  - `src/audio.rs`: the heartbeat and `dazed` in `mix` read `felt`.
  - `src/world/omen.rs`: `frights` works out its eye and forward direction from the watched friend, and the false-mark filter is changed.
  - `src/net/mod.rs`: new `felt` function and `Network::felt`; `spectate_keys` no longer takes the mouse.
  - `tests/session.rs`: `felt` is added to the `net::{...}` import; the new tests follow `the_fallen_hear_the_whistle_their_friend_hears`.
  - `README.md`: the watch row of the controls table.
  - No new enum variants and no GAMEPLAY list change.

**La Rabia and the slower night (rage lane, roadmap v3 section 2: the tuning table and "La Rabia, bones drive the night").** Not El Respiro, the grace, the carry cap or Dawn (other lanes).
- Tuning table (`src/tuning.rs`): `night_length` 840 → 1200 s; `pressure_base` 0.12 → 0.10; `pressure_night` 0.5 → 0.2; `pressure_carry` 0.07 → 0.04; `pressure_rite` 0.08 → 0.12; `with_night` now scales `pressure_carry` by the same 1.3 / 0.5 as the rite, so a laying nets +0.08 Normal, +0.04 Gentle, +0.104 Hard (× 1.25 for the Hijo) and never lowers pressure; `stalk_phrase_interval` (5, 11) → (6, 13); `omen_quiet` (35, 70) → (45, 85); `pump_hold` 10 → 12 s. `perception.rs` `stalk_gap` pressure factor 0.35 → 0.2.
- **La Rabia.** `tuning::MAX_RAGE` = 5. Pure `Tuning::at_rage(stage)` beside `at_pressure`: stalk interval × (1 − 0.05·s) (4.5–9.8 s at 5); `stalk_silence_chance` 0.2 − 0.04·s (floored at 0); `stalk_answer_chance` 0.15 + 0.03·s; `omen_quiet` × (1 − 0.08·s); `grass_sight` 5 + 0.8·s (7.4 m at 3); `light_lure_range` + 4·s m (53 m at 2 on Normal; added in metres to the night-scaled base, so Hard 54 + 4·s, Gentle 36 + 4·s). It never touches his speeds, warning, exposure or hearing.
  - Stage = `Progress::rage()` = bundles delivered, capped at 5. Public already as `WorldView.delivered`: no protocol change, no new wire field; continuous pressure stays hidden.
  - Readers: `Encounter::update_threat` (`base.at_rage(stage).at_pressure(p)`, so the grass sight), `CueDirector::stalk_gap` (from `enc.progress.rage()`), `Director` through the new `Mood.rage` (the omen quiet and the stolen torch), `Session::light_lure` (range), and the route driver's evader in `script.rs` (unhashed; it judges grass cover by the stage from `snap.world.delivered`, else it would crouch 6 m from him at stage 3 believing itself hidden).
  - **Rule change:** the stolen-torch omen unlocks at `Tuning::stolen_light_rage` 2 (bones laid), replacing `stolen_light_pressure` 0.45 (field renamed in place). The existing director test helper gained a rage argument: the calm cases pass 0, the "late night" cases 5, the same-seed case 2; its assertions are unchanged.
  - Stage 5 "the engine calls him" is today's rule, unchanged. The `Omen::StolenLight` and `director.rs` module comments now say the torch comes with his anger, not late in the night. Nothing unlocks because a player fell: a capture's `release_all` leaves the stage where it was (tested).
- **Telegraph** (client presentation, unhashed): `world::omen::RageTelegraph` on `Fright`. On `Event::RelicDelivered`, unless another laying came less than 8 s before (chained: a burst laid 3.5 s apart is one telegraph), it plays `Sting::Rage` (appended at the end of `Sting`; unplaced, `VoiceKind::Sting`, music bus, g × 0.9) and every lamp and lantern stutters for 1.2 s (a multiplier in the `dynamic.rs` lamp rig and `world/mod.rs` `flicker_lights`, without the omen's 35 m gate). The HUD bones line ends "his anger ••···". **Deviation:** the bundled Noto Sans has no ■ / □ (U+25A0 / U+25A1, checked in its cmap), so • (U+2022) marks anger and · (U+00B7) calm. The dread drone now steps explicitly by stage / 5 (it already stepped by delivered / total); no new voice. The Madrina chapter is the radio lane's.
- Audio: `sting_rage.wav` (4.8 s: a gust through the grass, the ceiba's roots groaning as a bent low creak, far thunder; never a whistle) from `make_sting_rage` on `random.Random(9200)` (a literal seed in this lane's 9200-9299 range, not `SEED + 9200`), appended at the end of `make_horror`'s table. A fresh `--out` generation into a scratch folder reproduced all 76 existing WAVs byte for byte (`Get-FileHash`); only the new file was copied into `assets/audio`. The integrator regenerated again on the final tree: all 77 shipped WAVs match byte for byte. `assets/SOURCES.md`: 77 WAVs, one row.
- Tests:
  - `sim::every_bundle_laid_is_a_step_up_on_every_night_and_variant` replaces `pressure_rises_with_the_night_the_load_and_every_bundle_laid_to_rest` (the one allowed replacement). Every Night × each variant (first seed of each in 0..60): the night and the load still raise pressure; each of the five layings from dusk raises pressure over before the pickup and over the carried state (unless at `pressure_max`, which Hard × Hijo reaches at the fifth), and the stage steps up by exactly one; a fall leaves the stage at 0.
  - `tuning::rage_only_tightens_and_never_outruns_a_walker`: stages 0-5 × pressure 0-1 (21 steps) × Night. Hunt and stalk stay under walking, warnings over 1.5 s; speeds, warn time and distance equal stage 0's; each lever only tightens stage by stage; silence + answer < 1; grass sight under the crouched notice distance; stage > 5 clamps; `at_rage(0)` is the identity; 53 m and 7.4 m on Normal.
  - `sim::a_crouched_player_seven_metres_off_in_grass_is_found_only_once_he_is_angry`: `has_sight` false at stages 0-2 and true at 3-5 (the stage from delivered relics); standing in the open he sees them at any stage.
  - `director::the_stolen_torch_waits_for_his_anger_and_every_laying_crowds_the_quiet`: no stolen torch below stage 2 even at fear and pressure 0.9; one at stage 2 at pressure 0.1; omen counts over 3000 s never fall as the stage rises and stage 5 has more than stage 0.
  - `perception::his_anger_crowds_the_stalking_whistle_but_keeps_no_steady_rhythm`: 30 min of stalking at stage 5 has a mean gap under 0.9 × stage 0's, still with quick answers and a spread of more than 2.5 ×.
  - `world::omen::tests::layings_in_a_burst_telegraph_once_and_the_lamps_stutter_briefly`.
  - Invariant tests untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden).
- Verified: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked` (lib 100, district 9, session 55 + 3 ignored), `cargo clippy --locked --all-targets -- -D warnings` clean (`target\gate-rage-*.log`, re-run on the final tree by the integrator after `cargo fmt`, `target\gate-final-*.log`; fmt changed nothing). The scripted solo route stays inside 3-14 min.
- Route sweep, normal: on the gameplay-complete tree, on the final tree, and once more by the integrator on the committed tree, the same five seeds failing every time: solo 149/150, shared 146/150 (bar ≥ 146; base 150/148). Logs `target\sweep-rage.log`, `target\sweep-rage-final.log`, `target\sweep-rage-gate.log`. Shared 11 and 35 (base failures) now pass. Newly failing, attributed by two diagnostic sweeps with the rage levers switched off by a temporary edit (reverted; logs `target\sweep-norage.log`, `target\sweep-nolure.log`, both solo 149, shared 148):
  - Solo 87: not a loss. The outcome is Won while the driver is still in its ignition step: it hid in the truck's shadow through the whole warm-up and the run ended first. From the tuning table alone (the night timeline shifts), not the stage.
  - Shared 10 (player 1 down at the altar step, 3/5 laid, 12 failed hiding places) and shared 136 (player 1 caught at (42.1, -68.9) near the ramp, the far-bank signature of base shared 35): from the tuning table alone; the night's timeline moved.
  - Shared 26 and 133: player 2 down in the house yard on the walk to the truck (Go (44, 30)), all five laid, stamina 0.04 / 0.06. Caused by the stage-5 lure (65 m): with the lure alone unraged both pass. The route driver walks with its torch lit; the lure is the rule working as designed.
- Fingerprint moves on purpose (`session.rs`, `tuning.rs`, `sim.rs`, `perception.rs` and `director.rs` are hashed and edited); no new hashed file. Value not computed (no `target\fp.py` in this worktree; it moves again at merge).
- Bounds moved: none. The scripted solo route bound (3-14 min), "quick hands" (< 8 min), the `script.rs` route timeouts (600 s) and the net-smoke host cap were not needed and are unchanged.
- Not run here (integrator): the two-process net smoke, the gentle and hard sweeps, the rendered game.
- Unverified (user-led, rendered): the stinger's level and feel; the lamp stutter; the anger pips' look; the drone step; the pacing of a 20-minute night by a human.
- Open: the route driver could go dark in the open from stage 4 so the 65 m lure costs the sweep less (route driver, not a rule); "quick hands" is still < 8 min although the night is longer; `stolen_light_rage` and the stage levers are not night-scaled beyond the lure's base.
- Process incident (no effect on the repo): at about 14:14-14:16 some PowerShell file writes used relative paths, which resolved against the process's own directory (`C:\Users\andre\source\repos\el-silbon`, the main repo) instead of this worktree, and changed `src/tuning.rs`, `src/sim.rs`, `src/perception.rs` and `src/director.rs` there. `git diff` there showed only this lane's changes; it was saved to the lane's scratchpad as `accidental.patch` and the four files were restored with `git checkout --`. `git status` in el-silbon is clean. Every later write used absolute paths into esb-c.
- Merge notes (likely conflicts with the other three lanes):
  - `src/tuning.rs` defaults: `night_length`, the four pressure values, `pump_hold`, `stalk_phrase_interval`, `omen_quiet` changed and `stolen_light_pressure` renamed to `stolen_light_rage` (u8) in place; El Respiro may touch the same lines (`first_warn_delay` nearby, Dawn's use of `night_length`). `with_night` gained a `pressure_carry` line beside `pressure_rite`; keep it next to any grace scaling.
  - Additions at the end: `MAX_RAGE` after `DEFAULT_SEED`; `at_rage` after `at_pressure`; `Mood.rage` last field of `Mood`; `Sting::Rage` last variant of `Sting`; `sting_rage.wav` last entry of `make_horror`'s table in `tools/gen_audio.py` (keep the radio lane's lines too).
  - `assets/SOURCES.md` line 12 says "All 77 WAVs"; the radio lane bumps it too, so add the counts. My row follows `call_help.wav`.
  - `src/sim.rs` adds `Progress::rage` and changes one line in `update_threat`; if El Respiro adds a leash there, keep `base.at_rage(stage).at_pressure(p)` as the scaling line. `session.rs` changes are small (a `rage` local in the omens, the range line in `light_lure`).
  - Fingerprint: hashed files are edited, so it moves; no new file in `GAMEPLAY`, no protocol change.
  - Re-check after merging: the stolen-torch omen needs 2 bundles laid, not pressure. Any lane that tests `Omen::StolenLight` or builds a `Mood` by hand (a pacing test, say) needs the new `rage` field.

**El Respiro, the pacing director, plus `carry_max` and the Dawn outcome (branch m1b-respiro, roadmap v3 section 2).**
- Parallel lane: built from fb6c1ca alongside La Rabia, the radio + Madrina chapters and spectator polish. The retune table (`night_length`, `pressure_*`, `stalk_*`, `omen_quiet`, `pump_hold`, `grass_sight`, `light_lure_range`) and `with_night` are untouched; `tuning.rs` is not edited at all. Every director number lives in `src/pacing.rs`. Bones laid are read from `Progress::delivered()`, not from rage code.
- New pure `src/pacing.rs` (in `lib.rs`, the AGENTS.md pure list, and appended at the END of `GAMEPLAY` in `protocol.rs`, after `Cargo.lock`). The director is `Respiro` (so named to avoid clashing with the omen `director::Director`):
  - Phases `Dormant → Grace → Build → Peak → Fade → Relax → Build …`. `begin()` is called only at a real manifest (the first pickup in `interact`, or `rouse` waking him), never because a test pinned him, so the existing AI tests keep their warnings.
  - Grace: Gentle 240, Normal 120, Hard 45 s. Floor `cooldown.max(grace_left)`, leash 30 m. The first laying does not end it (roadmap decision).
  - Menace 0-100 per active player, the worst counts: +12/s seen (he has sight and they are his target), +8/s inside 16 m, +5/s torch lure (`pulse[4]` running), +4/s Tureco growling beside them, +3/s fear >= 0.7; decays 4/s.
  - Build 60-120 s. Peak: warnings allowed while hunts begun in the rolling 600 s are under the budget (Gentle 1, Normal 2, Hard 3). A Peak ends when a hunt resolves (`LostTrack`, or him calm again after a hunt: a down, pepper, Tureco), after menace >= 80 for 45 s, or at 120 s. The last two wait until he is Stalking, so a Peak never ends mid-warning or mid-hunt.
  - Fade waits until he is Stalking (it never withdraws a haul, which would drop the sack without `SackDropped`), then calls `Encounter::withdraw()` if he stands or rises in the world, and Relax begins.
  - Relax: `max(40, U(75,120) × (1 − 0.1 × laid) × night)`, Hard ×0.7, Gentle ×1.3; leash 45 m. A push ends it only after 40 s (counted as an early exit).
  - Pushes (`push_forward`): a bundle laid, holding the pump or the ignition, the beacon lit, any session noise >= 34 m (raw radius, before rain masking), anyone standing within 30 m of him while he is present. Lifting or carrying bones is not a push.
  - **Decision (unspecified in the roadmap):** a push in Build ends it once its 60 s minimum has passed (symmetric with Relax), so the engine or the pump can bring the Peak on.
  - Outside a Peak (Grace, Build, Fade, Relax, or a spent budget) the floor is `cooldown.max(1.0)` every tick: more than a tick, because `update_threat` counts the cooldown down before it looks. The session applies the orders inside `threat_step`, after noises and `choose_prey` and before `update_threat`, so `rouse()`'s `min(2.0)` and the wrong naming's `cooldown = 0` earlier in the step are floored too.
  - Hidden AI state: `Session.pacing` is never on the wire. No protocol field, message or event carries the phase or the menace.
  - **Deviation:** whenever else his warning is held (a Build, a Fade, a Peak with the budget spent) he is leashed at `LEASH_HELD` 30 m. Without it he loitered at his 13 m standoff in plain view through the Build, unable to warn, and the Peak's first warning came from point-blank: the first sweep of this item lost Normal solo 84, 105, 143, Gentle solo 26 seeds and Hard solo 26 seeds that way (downed about 9 s after the warning, fear 0.9). With it he approaches from past his notice range when the Peak opens, as the old first warning did.
- `sim`: `Threat.leash: Option<f32>` (appended). While leashed, `stalk` (now given the watchers) swaps a target node closer than the leash to anyone for the allowed node nearest it (else `farthest_from`), stops circling, and searches a noise from the node instead of walking off-node to a spot inside the leash. He still walks, whistles honestly and turns toward noise. The leash steers; it is not a wall.
- `carry_max` (`pacing::carry_max`): 2, Gentle 3, enforced in `Session::interact` ("Your arms are full: lay a bundle down first."). The scripted routes never carry more than 2.
- **Dawn** (user decision; 1.5 × `night_length`, read from `Tuning`, so the rage lane's 1200 s gives 30 min):
  - `Outcome::Dawn` (appended; wire code 3, decoded before the `_ => Failed` arm; the protocol round-trip test covers it) and `Event::Dawn` (appended to `Event` and `Event::ALL`, now 52).
  - `Session::outcome_step`: at `pacing::dawn(night_length)`, `Encounter::dawn()` sinks him for good (`Sinking { relocate: false }`; a haul drops the sack and `release_captive` runs) and sets `Encounter.dawn`. With bones still out the night ends as Dawn. With every bone home the night goes on without him: `rouse` and `withdraw` are no-ops after dawn, and the director stops.
  - Wired: the outcome card ("You lived, but he will be back.", no win marks), `profile::Ending::Dawn` and `Tally.dawns` (`#[serde(default)]`), the Journal line ("Lived till dawn"), a HUD caption for `Event::Dawn`, and `Event::Dawn` plays the existing `dawn.wav` (no new sound; captions only, per the user). `net::mod`'s outcome hold is for Failed only, so Dawn goes straight to the card. The script and net-smoke drivers compare outcomes generically, so a Dawn where a route expects Won fails loudly.
- Stats: `pacing::Beats` (seconds in Grace/Build/Peak/Fade/Relax, peaks, hunts, early Relax exits), kept off `sim::Stats` because that goes on the wire. `smoke_exit` logs `SMOKE PACING (this run): …` from `Endpoint::debug_session()` (DEBUG ONLY; the smoke restarts, so it is the last run's beats).
- Tests (new functions only):
  - Pure (`pacing.rs`), all three nights, many seeds, a stand-in threat that warns whenever allowed and random pushes: `a_relax_of_at_least_forty_seconds_always_follows_a_peak`, `hunts_in_any_rolling_ten_minutes_stay_within_the_budget`, `a_push_ends_a_relax_only_after_its_floor`, `the_same_seed_gives_the_same_beats`, `grace_forbids_warnings_and_keeps_him_away_until_it_ends`, `menace_is_the_worst_of_the_party_and_decays`.
  - Session `a_player_in_plain_view_ten_metres_from_him_is_never_warned_during_grace_or_relax`: a real pickup; pinned present 10 m away in line of sight through the grace and the Build; the same stand in the Peak is warned within 3 s; hidden behind the house wall until the Peak fades; then 39 s of Relax in plain view. Checked red by mutation: with no Grace floor it fails "warned in the Grace at 9.0 s"; with no Relax floor, "warned in a Relax at 235.0 s".
  - Session `dawn_ends_a_night_with_bones_out_at_one_and_a_half_night_lengths` (a 100 s night: Running at 149.5 s, Dawn at 150 s, the event and the snapshot say so; with every bone home he sinks to Hidden, the night runs on and `rouse` cannot bring him back) and `nobody_carries_more_bones_than_the_night_allows` (2, 2, 3; drop one and the next lifts).
  - The `profile` round trip records a Dawn; the `protocol` round trip includes Dawn.
  - Invariant tests untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden).
- Verified (this tree, Windows): `cargo fmt --check`, `cargo check --locked`, `cargo test --locked` (lib 101, district 9, session 58 + 3 ignored), `cargo clippy --locked --all-targets -- -D warnings` clean. Logs `target\respiro\gate-*.log`; again after the await move, `target\respiro\fix-*.log`.
- Route sweeps (final tree; logs `target\respiro\sweep2-*.log`, `sweep-normal-final2.log`, and for Gentle with the moved awaits `sweep-gentle-exp1500.log`):
  - **Normal: solo 149/150, shared 150/150** (base 150/148). Shared 11 and 35 no longer fail. Newly failing: solo 84 only. Run 2 (the scripted failure run), step `Await Recovered`: the player is downed about 9 s after the first warning at night 200 s near the corral approach (56.6, 5.3), fear 0.92, stamina 0.34. Traced with `ROUTE_LOG` (before the held leash, the same signature): the failure run stands in the open from night 13 s, the grace and the Build hold his warning for about 3 min while her fear climbs to 0.9, and the hunt downs her before the route's evasion breaks sight. The win run of seed 84 met no warning at all (at the truck by night ~300 s). A route-script artefact of the long wait, not a rule regression.
  - **Gentle: solo 150/150, shared 150/150** (base 150/150), with the six 900 s awaits in `script.rs` raised to 1500 s (see Bounds moved). With them at 900 s the shared sweep was 0/150 (`sweep2-gentle.log`): every seed failed run 2 (the shared failure) `waited 900s for Outcome(Failed)`, one down and one hunt, the partner still standing at night ~913 s. That is the design, not a regression: the Gentle hunt budget is 1 per rolling 600 s and a shared failure needs a second down, so the second hunt cannot start before ~600 s after the first (~night 370 s). The 1500 s sweep (`sweep-gentle-exp1500.log`) ran on a tree identical to this one: every other source file predates it, and the only later write to `script.rs` was the revert this fix undoes. It was not re-run after re-applying the change (the lane's rules leave the sweeps to the integrator). Normal and Hard need no re-run: a longer timeout changes a result only where the old one fired, and no Normal or Hard failure in `sweep2-*` was a timeout.
  - Dawn and the moved awaits: on this tree Dawn lands at 1260 s (1.5 × 840), inside the new 1500 s awaits, and a Dawn where a route expects Failed fails loudly. In the 1500 s Gentle sweep both downs landed before 1260 s on all 150 seeds. After the rage lane's 1200 s night, Dawn moves to 1800 s, past every route await, so the margin only grows.
  - Hard: solo 149/150, shared 135/150 (base 150/138; the test's own 97% assert fires on Hard, as at base). Solo 108: the run ended Won while the route was still in its ignition step (the engine warmed with the player in the truck zone and no warning to interrupt it), a route-timing artefact. Shared 6, 7, 16, 22, 29, 42, 57, 73, 76, 94, 104, 126, 129, 148, 149 fail (base: 4, 5, 20, 30, 38, 43, 46, 61, 93, 123, 124, 126); most are player 2 downed on the walk to the truck (`Go (44,30)`, near (16, 24)), the pattern of base Hard shared 38. Not traced one by one.
  - The two-process net smoke and the rendered smoke were not run (the integrator runs them after merging). Expect run 2 of the net smoke to take several minutes longer: its first warning now waits through the grace and a Build. `net::smoke`'s own whole-route `TIMEOUT` is 2400 s, and no 600 s host cap exists in the code (the 540-600 s waits in earlier notes were the integrator's shell waits).
- Bounds moved, under the roadmap's allowance "`script.rs` route timeouts → 1500 s":
  - `script.rs` `fail_run`: `Await Outcome(Failed)` 900 s → 1500 s.
  - `script.rs` `net_host`: `Await Party(2)` 900 s → 1500 s, and run 2 `Await Outcome(Failed)` 900 s → 1500 s.
  - `script.rs` `net_client`: `Await Started` 900 s → 1500 s, run 1 `Await Outcome(Won)` 900 s → 1500 s, and run 2 `Await Outcome(Failed)` 900 s → 1500 s.
  - The two shared `Outcome(Failed)` awaits are the ones the Gentle sweep needed. The other four go with them because that is exactly the tree the 1500 s sweep measured.
  - Unchanged: the 600 s awaits (`fail_run` `Warned`, `net_host` run 3 `Party(1)`), the solo route bound (3-14 min), "quick hands" (< 8 min) and the net-smoke host cap. `net::smoke`'s whole-route `TIMEOUT` stays at 2400 s.
- Fingerprint: moves on purpose (`pacing.rs` joins `GAMEPLAY`; `sim.rs`, `session.rs` and `protocol.rs` change). Not recomputed here; it moves again at the merge.
- No asset and no `tools/gen_audio.py` change, so every generated file is unchanged.
- Unverified: everything in a rendered night (the caption, the card, the Journal line, the rooster's timing); human-timed nights (the beats log is there for them); how the Peak's first warning feels after a Build spent in his view.
- Open:
  - The roadmap's second session test (after a sack resolves he stays >= 45 m from every standing player until a push) is not written. The leash steers his patrol target; it does not stop a player walking up to him, or an edge passing near someone.
  - No placed fake-outs in Build (roadmap A1) and no "one real scare every 4-6 min" rule.
  - Menace only ends a Peak; nothing else reads it yet.
  - Integrator: re-run the Gentle sweep once after the merge to confirm shared 150/150 with the 1500 s awaits (this lane did not re-run it; see the sweep note above). The Gentle hunt budget stays 1, as the roadmap set it.
  - `Event` wire codes are declaration order (`emit` sends `*e as u8`; clients decode through `Event::ALL`). Three lanes append after `OmenFalseMark`: the merge must keep the enum and `ALL` in the same order (`sim::tests::event_codes_round_trip` catches a mismatch).
- Merge notes:
  - `script.rs`: six one-line edits, `timeout: 900.0` → `1500.0` (the `fail_run` Failed await, `net_host` `Party(2)` and run-2 Failed, `net_client` `Started`, run-1 Won and run-2 Failed; lines 1695, 1817, 1875, 1919, 1964 and 1984 on this branch). If another lane also edits these lines, keep the larger value: Gentle shared needs about 1260 s or more for its second down. Each await must also stay below Dawn (1.5 × `night_length`), or a Dawn arrives where the route expects Failed and the route fails loudly. With the rage lane's `night_length` 1200, Dawn is at 1800 s, so 1500 s is safe.
  - `PROGRESS.md`: every change is inside this subsection, at the end of the M1b handoff.
- Integrator check at commit (this tree): `cargo fmt --check`, `cargo check --locked`, `cargo test --locked` (lib 101, district 9, session 58 + 3 ignored) and `cargo clippy --locked --all-targets -- -D warnings` all clean; Normal route sweep re-run, solo 149/150, shared 150/150 (only solo 84 fails, as above). Logs `target\respiro\int-*.log`. Bounds audit of `tests/`, `script.rs`, `sim.rs` and `tuning.rs`: `tests/session.rs` has additions only, `tuning.rs` is untouched, `script.rs` changes only the six timeouts, and `sim.rs` changes no test; the one changed assertion elsewhere is the `profile` round trip (`nights` 3 → 4, because it now also records a Dawn). The Gentle and Hard sweeps were not re-run at the commit.
- Committed on m1b-respiro; not pushed.

**La Voz del Llano and La Madrina (item 11, roadmap section 4; branch m1b-radio from fb6c1ca).**
- Pure `src/radio.rs` (added to the AGENTS.md pure list, and hashed as the LAST entry of `GAMEPLAY`, after `Cargo.lock`, so parallel lanes append cleanly):
  - `STOPS` 540, 620, 710, 880, 1010, 1270 kc; the dial reads 0 off, `i + 1` for `STOPS[i]`; `turned` goes off → each stop → off.
  - `stations(seed)` shuffles one Numbers, one Tale (placeholder), one Joropo and three Static (Fisher-Yates on `Rng::fork(seed, 0x4AD1_0000)`); `numbers_stop`, `station`, `frequency`.
  - `numbers_hour(code)`: the ident (1.6 s) at 0, then each digit in a FIXED 3.6 s window (d pips 0.4 s apart; a zero is one 1.2 s tone), 2 s between ident and digits, a 5 s rest. `CYCLE` = 23.4 s, constant. Deviation: fixed windows rather than "2 s after the last pip", so a digit swallowed by static cannot be read off its length, and the phase is a pure function of the run clock. The gap between digits is therefore at least 2 s.
  - `dropped(seed, cycle)`: one cycle in four, lightning swallows digit `(offset + cycle) % 3` (a seeded per-cycle coin, not tied to `storm` bolts; see Open). Consecutive strikes always differ, so any two cycles running carry every digit.
  - `beats_between(seed, code, t0, t1)`: every beat starting in `[t0, t1)` of the run, absolute times from one expression, so any split into frames hears each beat once. A swallowed window is one `Static` beat.
  - `Listener`: counts pips per window from what it hears; a digit counts only if its whole window was heard without a gap (tuning in mid-window or a gap loses that window; static leaves it for another round).
  - `dots(seed, code, now)`: this cycle so far, ident as a ring, pips as dots, a zero as a dash, static as waves.
  - It never takes his position: the radio never reacts to where he is.
- Truth and wire (appended, merge-friendly): `Progress::radio` (reset by a restart through `Progress::new`), `Event::RadioTuned` (last; `Event::ALL` is 52), `Encounter::turn_dial`, `TargetKind::Radio` (last) through `Action::Interact` (any active player; it changes for everyone), a `noise_dial` 8 m squeal at the radio, `WorldView.radio` (last, `#[serde(default)]`). Tuning (appended at the end of `Tuning`): `noise_dial` 8, `radio_reach` 6. No hidden AI state on the wire: the dial is public, the code is already derivable from the shared seed, and nothing carries his distance.
- Layout: `District::radio` (-4.74, 1.82, -5.4), and page 10's `NoteSite` now takes that one value (id kept). `evaluate_target` skips the note at the radio and offers the Radio instead; turning the dial adds page 10 to the Journal on that client (`encounter::update_target`), so saved Journals and "Keeper of the tale" still reach 20 until the M2 tale station takes the page over.
- Client:
  - `audio::play_radio`: on the numbers stop, `beats_between(last elapsed, snapshot.elapsed)` each frame (reset on a new run, a backwards clock or a jump over 1 s), each beat a placed one-shot at the radio (the static burst played at 6/3.6 speed to fill its window). The broadcast loop plays on the Tale stop and, at ×1.12, on the Joropo stop (a placeholder, no joropo clip yet); a new static loop (`VoiceKind::RadioStatic`) on the three empty stops. The squeal is placed at the radio on `RadioTuned`.
  - `hud::radio_dots` (new `RadioDots` line above the caption): within `radio_reach` of the radio on the numbers stop, whatever the captions setting.
  - Prompts: the radio shows its kc (or off); the key box's prompt shows the padlock tag, «880 kc» for tonight's numbers stop (from the shared seed, no wire field).
  - Objective, first-time help, How to play, the briefing and README now point at the radio instead of the pages.
- Pages 3, 4 and 16 rewritten (Spanish and English) to point at the radio and its tag; `lore::fill` and `lore::keep` are deleted; the note card and the Journal show each page as written.
  - Batch-1 reconciliation: `the_journal_never_tells_a_nights_code` asserted that every digit sits on some page's site, the opposite of this design. It is rewritten, not loosened: no page or chapter keeps a blank for a digit, so what the Journal keeps is the same every night.
- La Madrina: `lore::chapter(1..=6)` in Spanish and English (the entrails; the father killed; the post, whip, ají and dog; the curse, the sack and the whistle; Santa Rosa's storm night; all home: "nómbralo bien" / "name him rightly"). Each last line hints at that stage's trick, text only (1 he whistles more often, 2 a lit torch calls him from farther, 3 he finds you in tall grass from farther, 4 every bone angers him more, 5 the engine calls him). `lore::chapters_told(delivered, total)`: one per bundle laid, and the sixth with the fifth. `hud::madrina_strip` (top centre, its own entities, 14 s per chapter, queued, separate from `Hint`), driven by the shared `WorldView.delivered`, so everyone reads the same chapter. `Profile::chapters` (`#[serde(default)]`) keeps them; the Journal lists them (`Page::Chapter`, `Act::Chapter`, appended).
- Route driver (`script.rs`, not hashed): `Want::Radio` and `Cond::Digits` (appended). At the radio it clicks the dial every 0.5 s until the snapshot shows the stop on the padlock's tag, then feeds `beats_between(previous elapsed, elapsed)` to its own `Listener`; the key box is tried with `Listener::code()` only, never `lock_code`.
  - Solo (`RouteScript::full`): the radio comes before the table bundle, while he still sleeps (about 20 to 45 s).
  - Shared (`net_host`/`net_client`): the co-op relay. The host no longer listens. After acknowledging the marks, the partner walks in by the back door (new `BACK_DOOR_IN`), counts the pips, goes out the back door to the key box and opens it, then goes to the lit porch as before; the host finds the box open.
  - While awaiting both marks, a route marks the altar again if its own mark faded (the partner's mark could expire while the host was hiding).
  - Why the relay: with the host listening at the start, normal shared fell to 141/150; with the partner listening at the start the host waited at the altar and the default-seed gate test failed. Placements measured (shared, normal): host at the start 141; partner after the marks via the porch 145 (145 with the re-mark); partner watching from the porch 145; partner listening at the start 146, 149 with the re-mark but the default-seed gate test failed; partner in the house throughout 134. This one (the partner by the back door) passes the gate and the sweep. Saved variants: `target\radio\script-variant-*.rs`.
- Audio: `radio_pip`, `radio_pip_long`, `radio_ident`, `radio_static`, `radio_squeal` from `make_radio_numbers()` on seeds 9100-9104. A full regeneration into a scratch folder reproduced all 76 existing WAVs byte for byte (SHA-256); only the five new files were copied in. `assets/SOURCES.md`: 81 WAVs, five rows.
- Tests (written with the new API in the same pass, not red-first):
  - radio: `one_numbers_station_per_night_and_it_moves_between_seeds` (all six stops occur over 200 seeds), `the_pips_reproduce_the_code_for_every_night` (200 seeds, a listener tuning in mid-cycle at ragged frame rates, through `beats_between` and `Listener`, within 4 cycles; mid-window tune-in and a gap never count a digit short), `each_beat_is_heard_exactly_once_at_any_frame_rate` (1/144 s to 7.3 s and a random dt), `two_cycles_running_carry_every_digit` (200 seeds × 200 cycles), `digits_are_two_seconds_apart_and_the_cycle_rests`, `the_dots_show_what_was_said_this_cycle`.
  - lore: `the_journal_never_tells_a_nights_code` (rewritten, above), `each_bundle_laid_tells_the_next_chapter_and_all_home_the_last`.
  - session: `the_radio_dial_is_shared_and_its_squeal_draws_him_until_a_restart_turns_it_off` (both players can aim it; a Stalking, Present threat at 1.25 × the squeal's reach does not hear it, at 0.8 × does; each turn shows in both snapshots and both hear `RadioTuned`; a restart turns it off).
- Fingerprint checkpoint for this tree: 0xc218379caf056ba9 (Python FNV-1a model of `fingerprint_of`, `target\radio\fp.py [REV]`, which reproduces HEAD's 0xd335073e06e8cde2). It does not pair with test.3; merging the other lanes moves it again.
- Verified: fmt, check, test (lib 102, district 9, session 56 + 3 ignored), clippy -D warnings clean. Route sweep, normal: solo 150/150, shared 148/150 (22, 48). Base fb6c1ca: 150/148 (11, 35).
  - Newly failing: shared 22 (the host is caught laying bundle 3 at the ceiba at about 130 s while the partner is on the key-box leg) and shared 48 (the host goes down at the base-shed wade, the item-7 signature). No longer failing: shared 11 and 35. The relay changes where the partner stands from about 60 s to 150 s, and with it where he walks.
  - Logs: `target\radio\sweep-final-normal.log` and the measured alternatives `sweep-normal*.log` (gitignored).
- Not run here (integrator): the two-process net smoke (its routes now carry the relay and `WorldView.radio`), the gentle and hard sweeps, the rendered game. The net smoke is the main thing to watch after the merge.
  - Net smoke changes: the partner does the relay (`BACK_DOOR_IN`, the radio, `BACK_DOOR`, then `Do(Lockbox)`); the host's `Do(Lockbox)` waits for the key (300 s timeout) if the partner is late or down; snapshots carry `WorldView.radio`.
  - Solo `--smoke` and `tour` gain a 20-45 s radio step before the table bundle; check any wall-clock budget in `debug.rs` (none changed here; its only fixed budgets are `READY_TIMEOUT` 180 s, `SHOT_TIMEOUT` 45 s and `PHOTOS_TIMEOUT` 20 min).
- Unverified (rendered, user-led): the pips' level and timing at the radio, whether 3 falling notes read as an ident, the static and squeal, the dots line (the glyphs • — ~ in Noto Sans), the strip's placement over other HUD, the joropo placeholder, and whether the tag in the key box prompt is found.
- Open:
  - Lightning's swallowed digits are a seeded coin, not the storm's own bolts (`storm`); tie them together if a playtest wants the static to follow the thunder.
  - The tale stop plays the old broadcast; the M2 tale station replaces it and page 10's dial shortcut.
  - The strip does not merge layings within 8 s or merge with La Rabia's telegraph; the rage lane owns that.
  - No hint teaches the dial; the prompt, the tag and pages 3, 4 and 16 do.
  - The model radio's dial does not visibly turn.
- Other hashed files touched: `protocol`, `session`, `district`, `tuning`, `control` and `sim`.
- Merge notes (for the integrator of the four lanes):
  1. **HUD systems in `src/ui/mod.rs`:** the Present `.chain()` tuple hit Bevy's 20-element limit. `(hud::radio_dots, hud::madrina_strip)` is nested as one entry; other lanes should nest their systems the same way, not append.
  2. **Array lengths:** `Event::ALL` goes from 51 to 52 (`RadioTuned` last) and `GAMEPLAY` from 15 to 16. Where lanes collide, set the length to the sum of their additions. `radio.rs` sits after `Cargo.lock` (the task said "END"); the handoff above had put `noise.rs` before `Cargo.lock`. Choose one order; the fingerprint moves either way.
  3. **Same-spot appends (keep both sides):** `Tuning` tail (`noise_dial`, `radio_reach`, and their Default entries), `Progress` tail (`radio`), `WorldView` tail (`radio`), `TargetKind::Radio`, `Want::Radio`, `Cond::Digits`, `VoiceKind::RadioStatic`, `Page::Chapter` / `Act::Chapter`, `Profile::chapters`.
  4. **La Rabia overlap in `lore.rs`:** the chapter texts hardcode the rage-stage hints as prose (torch farther, tall grass farther, engine calls him). If that lane ships different levers, reconcile the wording. The strip does not merge layings within 8 s; the telegraph is theirs.
  5. **`script.rs`:** `win_to_altar` now takes a `radio: bool`, and the net client route changed after the crouch. `Step::Await` gains the re-mark for `Cond::Marks { mine: true }`.
  6. **`docs/PROGRESS.md`:** each lane appends its block at the end of the M1b section; keep all blocks.
  7. **`gen_audio.py`:** `make_radio_numbers()` is added to `main` after `make_horror()`.
- Gate re-run by the lane integrator on this tree: fmt, check, test (lib 102, district 9, session 56 + 3 ignored), clippy -D warnings clean; normal sweep again solo 150/150, shared 148/150 (22, 48), above the 146 bar. Bounds audit: the diff of `tests`, `script.rs`, `sim.rs` and `tuning.rs` only adds; the one rewritten test is the lore one above. The fingerprint checkpoint was reproduced by `python target\radio\fp.py`.
- Bounds moved: none (route timeouts and the 3-14 min bound untouched). Invariant tests untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden). Committed on m1b-radio; not pushed. The untracked `.cargo/` (a local build setting) is never added.

**He presses harder, and El Velo (branch t5-silbon from `v0.1.0-test.4` 5c24329; playtest test.4: "not aggressive enough", "make him disappear and reappear").**
- Written without a compiler; the integrator compiled and verified it on this tree (below). No rendered run, no audio run, no human night yet.
- **Verified by the integrator.** It compiled as written; `cargo fmt` reflowed `debug.rs`, `perception.rs` and `tests/session.rs`, and nothing else needed fixing. Gate on the final tree: fmt, check, test (lib 128, district 9, session 66 + 3 ignored), clippy `-D warnings` all clean.
  - Route sweeps (150 storms each, solo / shared) on the final tree: Normal 150 / 146 (shared 38, 72, 119, 145), Gentle 150 / 150, Hard 150 / 150. For comparison, test.4 had Normal 148 / 150, Gentle 150 / 150 and Hard 149 / 145. The bar (>= 146 for Normal and Gentle) holds; Normal shared sits exactly on it.
  - Before the driver fix, the author's code alone gave Normal 150 / 146 (the same four seeds), Gentle **138** / 150 and Hard 150 / 150. The 12 Gentle solo failures were one timeline in run 2. The driver waits alone in the dark for his warning, so its fear is at 0.95 when the warning comes. It hid well (he had been out of sight for 3.9 s), but it hit 1.0 and a susto stood it up screaming just before the new 4.0 s `lose_track_time`. It then wrote off the hiding place that had worked and walked 28 m of open ground in his sight until exposure downed it.
  - **Driver change (`src/script.rs`, `Reflex`, a new `SHAKEN` = 0.8 s).** While our own susto holds us (`snap.me.stun`), and for 0.8 s after, being seen is blamed on the scream, not on the place. The place is not written off, and the driver crouches back into it. It reads only its own stun from the snapshot, never his state or position. He is no easier to pass; the driver just no longer runs from a hiding place that works.
  - The four remaining Normal shared failures (player 2, run 1, `Go (44, 30)`): he warns from 15 m on ground lit by the lamps. The player has no stamina and no pepper, and walks about 34 m in his sight to the nearest grass. That is the harder Silbón (he notices from farther away and hunts longer), not a driver bug; left as is.
  - Headless two-process net smoke (`--host 127.0.0.1:5371` / `--join`, `--net-smoke --headless`, debug build of this tree): both print NET SMOKE PASS (host: shared pickup and delivery, marks, the shared win, two restarts, the shared failure, the dropped load recovered; client: likewise, leaving while carrying). About 12 minutes, same machine (not LAN or internet evidence).
- **Task A, the numbers (old -> new).** Director numbers stay in `src/pacing.rs`, threat numbers in `src/tuning.rs`:

  | What | Where | Old | New |
  |---|---|---|---|
  | Grace after the first pickup (Gentle / Normal / Hard) | `pacing::grace` | 240 / 120 / 45 s | 120 / 60 / 25 s |
  | Relax length before bones, night and floor | `pacing::RELAX` | U(75, 120) s | U(40, 70) s |
  | Relax floor (nothing ends it sooner; no Relax shorter) | `pacing::RELAX_FLOOR` | 40 s | 25 s |
  | Hunts per rolling 10 min (Gentle / Normal / Hard) | `pacing::hunt_budget` | 1 / 2 / 3 | 2 / 3 / 4 |
  | Relax leash | `pacing::LEASH_RELAX` | 45 m | 35 m |
  | Out of sight in a hunt before he loses track | `Tuning::lose_track_time` | 2.5 s | 4.0 s |
  | Averts before he withdraws, Normal (Gentle / Hard) | `Tuning::averts_to_withdraw`, `with_night` | 3 (2 / 4) | 4 (3 / 5) |
  | How far he notices a standing player (base; Gentle x0.85, Hard x1.12, pressure up to x1.25, all as before) | `Tuning::warn_distance` | 22 m | 24 m |
  | Veiled: long-silence chance multiplier (new) | `Tuning::veil_silence` | - | 0.25 |
  | Veiled: longest stalking gap (new) | `Tuning::veil_gap_max` | - | 10 s |
  | Veil length / seen gap between veils (new) | `pacing::VEIL`, `pacing::VEIL_GAP` | - | U(25, 70) s / U(12, 35) s |

  - Unchanged: the Relax night scale (Gentle x1.3, Hard x0.7), the grace leash (30 m), `LEASH_HELD` (30 m), Build (60-120 s), his hearing and every speed.
  - `with_night` now derives the averts from the base (`+1` Hard, `-1` Gentle, at least 1) instead of hardcoding 4 / 2, so Normal no longer equals Hard.
- **Task B, El Velo** (pure logic in `pacing.rs`, already hashed and already on the AGENTS.md pure list; no new module, no new `GAMEPLAY` entry):
  - `Respiro` alternates seen and veiled stretches only in a Build or a Relax and only while the session says he is calm (Stalking): a seen gap `VEIL_GAP`, then a veil `VEIL`, and so on. Drawn from its own stream `Rng::fork(seed, 0x5E5B_0E10)`, so the veils never move the beats' own draws; deterministic per seed. Every `enter()` lifts the veil (a push that brings the Peak on lifts it before any tick), and any tick that is not calm lifts it. Never in the Grace, a Peak or a Fade; never while warning, hunting, counting, hauling or after dawn. The first veil of a Build always falls before the Build's shortest length (35 < 60 s).
  - `Session::veiled()` = `pacing.veiled() && threat.state == Stalking && !encounter.dawn`. The live state matters: the director reads `calm` before `update_threat`, so a warning that begins this tick shows in that tick's snapshot.
  - **The wire.** His pose already travels only as `Snapshot.threat: Option<VisibleThreat>`, sent when he is in the viewer's cone and line of sight. While veiled `snapshot()` sends `None` to everyone, from anywhere (the check is inside the visibility closure). No new field, no "hidden" flag, no protocol change; `danger` is 0 then because he only stalks. The veil itself is hidden AI state, like the phase.
  - **The client.** Every presentation of him (`world::silbon`, the lightning reveal, the HUD) reads `net::mirror` of the snapshot, so a `None` draws nothing and places nothing. Solo goes through the same session snapshot.
  - **The whistle as the guide.** The session sets `CueDirector::veil(bool)` each tick before `tick`. Veiled, the long-silence chance is x0.25 and no stalking gap outlasts 10 s; as a veil falls, a silence already under way is cut to 10 s. Still inverted, still categorical, never spatialized. The fallen hear the same forwarded cue as before.
  - **He still senses.** `sim` is untouched: he moves, sees and hears as ever. **Decision:** Tureco still growls truthfully up close, and `DogView.facing` still turns toward him inside `growl_range` (22 m), as it always has. That is the dog's truthful cue, not the veil leaking; Tureco stays the only truthful proximity cue.
  - **Reappearance.** A warning can only start in a Peak, so the veil always lifts first. New client-only `world::omen::Materialize` (a pure struct, unit-tested): when he comes into view after at least 8 s unseen and, within 3 s of appearing, comes for this eye (`danger` 1-3), it plays the existing `Sting::Reveal` (`sting_reveal.wav`) and shares the lightning reveal's 45 s cooldown. It reads only the snapshot (shown or not, the danger), so it never knows why he was unseen, and it also fires after a long out-of-view stretch. No new Event, sound or wire field. The warning still lasts `warn_time` (over 1.5 s, invariant untouched).
  - **Lightning. Decision: nothing.** A strike shows nothing of a veiled man: the reveal in `omen.rs` already reads `snapshot.threat`, which is `None`. Why: a silhouette would need his position on the wire while veiled (forbidden), and the point of the veil is that the whistle (and Tureco) are the only guides.
  - **Spectators.** `snapshot()` builds `threat` and `danger` from the viewer and the veil is session-wide, so a fallen watcher sees exactly what the friend sees, veils included; `Materialize` works for them from the same fields.
  - **Stats.** `pacing::Beats` gains `veiled` (seconds) and `veils` (count); the `SMOKE PACING` line in `smoke_exit` logs both.
- Tests (all pass on the final tree):
  - `pacing`: `a_relax_of_at_least_forty_seconds_always_follows_a_peak` is renamed `a_relax_of_at_least_its_floor_always_follows_a_peak` (it reads `RELAX_FLOOR`; same assertions). `the_same_seed_gives_the_same_beats` also records every veil change and its tick, checks that another seed differs and that he was veiled. New: `the_veil_falls_only_in_a_build_or_a_relax_and_never_while_he_warns_or_hunts` (3 nights x 12 seeds x 3600 s with the stand-in threat: veiled only in Build/Relax, only after a calm input, never with a warning or hunt on, no veil longer than `VEIL.1`, some veil runs its course) and `anything_but_stalking_and_the_peak_lift_the_veil_at_once`.
  - `perception`: `veiled_he_is_never_silent_for_long_and_still_keeps_no_rhythm` (30 min each way: longest veiled gap <= 10 s, mean shorter than seen, quick answers and spread kept, the same inverted variant; a long silence under way is cut short as the veil falls).
  - `tuning`: the night test also checks averts Gentle < Normal < Hard (at least 1) and that the veil whistle numbers only shorten gaps.
  - `sim`: `averted_again_and_again_…` asserted exactly 3 averts; it now asserts `t.averts_to_withdraw` (the number's purpose, not a loosening).
  - `world::omen`: `he_materializes_only_after_a_long_while_unseen_and_only_coming_for_you`.
  - Session: `veiled_he_is_in_no_snapshot_and_the_warning_that_follows_still_reads` (6 director seeds, host pinned 10 m off in plain view with a dead watcher: veiled only in Build/Relax and only Stalking; no threat and danger 0 in the snapshot, nothing on the wire says "veil"; host and watcher always agree; the warning shows him to both and lasts over 1.5 s; at least one Build runs out with him veiled) and `tureco_still_growls_at_him_when_he_walks_veiled`.
  - Invariant tests untouched (hunt and stalk slower than walking, warnings over 1.5 s, continuous pressure hidden, whistle inverted and unplaced).
- Risks:
  - The default-seed route tests (`the_smoke_route_…`, `the_tour_…`, `the_network_smoke_routes_…`) pass. The sweeps are re-measured above, and Normal shared has no margin left over the bar.
  - Fingerprint moves (`pacing.rs`, `tuning.rs`, `perception.rs`, `session.rs`, `sim.rs` edited; `script.rs` is not hashed). No protocol change. Builds from before this change will not pair with it.
- Files: `src/pacing.rs`, `src/tuning.rs`, `src/perception.rs`, `src/net/session.rs`, `src/sim.rs` (one test), `src/world/omen.rs`, `src/debug.rs`, `src/script.rs` (driver, by the integrator), `tests/session.rs`, this file.
- Next: a rendered solo smoke and a human night to judge whether 25-70 s veils and a 10 s whistle cap feel right, whether the stinger at a materialization reads as a scare, and how soon the first Peak now lands (about 2-3 min after the first pickup on Normal).

**Menu arrows and the Language setting (branch t5-lang, from v0.1.0-test.4 = 5c24329). Gate green; the rendered look in Spanish and the arrows by mouse stay user-led.**
- Playtest: "the arrows in the menu are not working properly, we have to lower the settings using the keys", and "implement language … so we're not showing both texts in English and Spanish".
- **The arrow bug** (`src/ui/menu.rs`, traced on the title Settings hub, Video / Audio / Controls / Calibrate and the pause card, which share one `navigate`):
  - The `‹ value ›` of a chooser was one `Text` inside the row's `Button`; the arrows were only glyphs. Every mouse press on a row, wherever it landed, became `(act, +1)`, so a click on ‹ raised the value, and at the top of its travel (100 %, FOV 100, brightness max) did nothing. Only the Left key lowered. The Left/Right keys themselves worked both ways.
  - A second fault on the keys: the filter dropped only `dir < 0` on non-choosers, so Right (or D) pressed a focused button (Back, Leave, Quit, Resume…), against its own comment.
  - Not the cause (checked): no overlay blocks clicks (every full-screen node is `FocusPolicy::Pass`, the splash despawns); `calibrate.rs` only shows the hats.
  - Fix: chooser values are bare, and each chooser row spawns two nested `Button`s (`RowArrow { row, dir }`), which block the row under them (Bevy 0.19 `ui_focus_system`: `Button` requires `FocusPolicy::Block`). One pure rule, `choose(rows, focus, Input) -> (focus, Option<(Act, dir)>)`: a click on ‹ is the Left key, on › the Right key, on the row itself Enter; Left/Right (keys or arrows) turn only choosers, both ways; a disabled row takes neither hover nor click. The arrow glyphs follow the row's focus colour.
  - Tests (`menu.rs`): `a_click_on_an_arrow_is_the_key_that_way_and_a_click_on_the_row_is_enter`, `left_and_right_never_press_a_button_and_a_dead_row_takes_nothing`, `the_arrows_lower_and_raise_every_setting_from_either_end` (every slider from its top and bottom, every two-way choice both ways, through `choose` then `adjust`).
- **Language** (`Settings.lang: lang::Lang`, `#[serde(default)]`, English by default, no locale detection):
  - New pure module `src/lang.rs`: `Lang { En, Es }`, `Words { en, es }`, `Lang::say / pick / toggled / name`, and the tables (`menu`, `hud`, `hint`, `panel`, `outcome`, `net`). Added to the AGENTS.md pure list and the lib.rs docs.
  - Local only: saved in `profile.settings`, never on the wire, and no hashed file was touched (`GAMEPLAY` unchanged; `perception.rs`, `radio.rs`, `tuning.rs` keep their English strings and the UI translates around them), so the fingerprint is still test.4's `0x7447a278f501d3ce` (checked, below) and this build pairs with test.4 builds. Two friends in different languages read the same chapter at the same moment, each in their own.
  - The row: Settings hub, "Language / Idioma" (read in both, so it can be found from either), value English / Español; toggling rebuilds the page at once. Static texts spawned once carry `ui::Tr(Words)` and a `translate` system; the controls reminder is set each frame by `watch_line` (its cached copy is gone).
  - Shown in one language only now (were both at once): every lore page (title, medium, body, signature; `lore::Page.title/by` are `Words`, `Page::text(lang)`), the Madrina's chapter titles and texts (Journal rows, the chapter page, the HUD strip: first line only, the second left empty), the note panel, the padlock and naming headers, the naming choices, the title tagline.
  - Translated (Stage 2): every menu page (title, solo, friends, host, join, journal, settings pages, calibrate and its hat labels, how to play, credits, pause, outcome choices, confirmations, the address error), the briefing card and its buttons, the objectives and guidance, the threat line, the roster, vitals, prompts, hold and stall labels, the downed and died panel, the controls reminder, every hint, the whistle captions, the outcome card (titles, tales, stats, marks, awards, who walked), the map legend, the watching line and the session banner's own lines.
  - Still English only: the transport's status and error strings inside the session banner (`net::transport`), the F12 debug overlay and every debug/smoke driver log, the brand "EL SILBÓN — The Return". Proper names (El Silbón, Tureco, La Madrina, El Borracho / El Hijo / El Arriero, place names, the survivors' names) are the same in both. Spanish-only in-world boards (`lore::sign`) stay as painted.
  - Tests: `lang` (default English, saved as a word, flips both ways; each language shows only its half), `lore::each_language_reads_a_whole_page_of_its_own`, the profile round trip carries `lang = es` and a test.1 profile loads as English. `tests/district.rs` reads the page title's two halves.
- Docs: `docs/PLAYING.txt` names the Language setting and the ‹ › arrows.
- Verified (lane integrator, this tree): `cargo fmt`, then `cargo fmt --check` clean. `cargo check --locked --all-targets` needed one fix: the briefing's night label shadowed the `night` text query in `ui/mod.rs` (renamed `kind`); nothing else. `cargo test --locked`: lib 130, district 9, session 64 + 3 ignored, all pass (the three menu tests, the two `lang` tests and the lore one among them). `cargo clippy --locked --all-targets -- -D warnings` clean.
- Fingerprint: `git diff 5c24329` over the 17 `GAMEPLAY` files is empty, and FNV-1a over them (without `\r`) recomputes to `0x7447a278f501d3ce`, test.4's; this build pairs with test.4 builds.
- Normal sweep (`ROUTE_NIGHT=normal`, `the_routes_hold_over_many_storms`): solo 148/150 (seeds 76 and 141, downed before the route meant it), shared 150/150, as at test.4.
- Rendered `--menu-shots` (debug build, 1600×900): `MENU SHOTS OK`, 21 captures. The Settings hub (title and pause card) shows the "Language / Idioma" row with `‹ English ›` at the row's right; page 0 and the briefing read English only. The driver runs on `Profile::default()` (it never reads a saved profile) and never opens Audio, so it shows neither Spanish nor the slider arrows. Seen in passing, not from this change: the Journal (20 pages + 6 chapters + Back, the same rows as test.4) overflows the 900 px window, its heading and Back clipped.
- Still unverified, a rendered look (user-led): click ‹ and › on Audio at 100 % and 0 %, Video brightness/contrast, the pause card's Settings, Right on Back does nothing; switch to Español on the title and in a paused night, read a page, lay a bundle for a chapter strip, and open the outcome card. Spanish line lengths are untested on screen (the note paper clips at 92 % height; the menu body is 470 px wide). The note body is now one italic 21 px text in either language (English used to be sans 15 under the Spanish): check page 19's English, the longest, for clipping.
- **Journal fits** (`ui/menu.rs`, `debug.rs`): its 20 pages and 6 chapters sit in two columns each (`columns`, filled top to bottom, so Up/Down read down one and on into the next; Left/Right cross to the nearest found row, `across`, tested), Back full width, card and rail 860 px; `--menu-shots` gains `18c_pause_journal`, and at 1600×900 and `--size 1280x720` the title and pause Journal show heading and Back inside the screen (gate green). Unverified: Spanish titles in a 350-px column, focus by keys and mouse on found pages (the driver's profile has none).

## Current handoff — 2026-09-30 (M1a landed: fingerprint-neutral, pairs with test.1)

**Branding (main).** The user's branding kit as cover and icon. The name stays
El Silbón — The Return everywhere (window title, menus, package, exe, saves);
only the cover's painted title reads "WHISTLE".
- `assets/branding/`: cover, icon art, 256 px icon, 7-size ICO (plain Git;
  provenance in `assets/SOURCES.md` "Branding": user-supplied, author and
  terms not stated, confirm before a public release).
- Exe icon: root `build.rs` compiles `whistle.ico` with `winresource` 0.1.31
  (build-dependency, `toml` feature off) only when `CARGO_CFG_TARGET_OS` is
  `windows`; Linux builds skip it. MSVC needs `rc.exe` (Windows SDK).
  FileDescription/ProductName stay winresource's default `el_silbon`.
- Live window and taskbar icon: `src/icon.rs` decodes the embedded 256 px PNG
  (Bevy's `png` feature, now listed explicitly; it was already compiled in) and
  sets it through `bevy::winit::WINIT_WINDOWS` on the main thread
  (`NonSendMarker`): `set_window_icon`, plus `set_taskbar_icon` on Windows.
  Once per native window id, retried until the window exists. A direct
  `winit = "=0.30.13"` dependency (the locked version) supplies the `Icon` type.
- Launch splash (`src/ui/splash.rs`), plain title launch only
  (`Launch::splash`): never `--play`, `--host`/`--join`, the debug drivers or
  headless. Black; the cover fades in (0.5 s), holds at least 1.6 s, stays
  while the title's models and fonts load (cap 6 s wall), fades out (0.7 s):
  about 2.8 s when the title is ready. Any key or click skips it (0.25 s) and
  is swallowed, so the menus never see it (`FocusPolicy::Block` stops hover and
  clicks). Cover-fit with a bounded crop (5 % sideways, 18 % vertically, so the
  painted title always shows), black bars beyond; 16:9, 16:10 and 3:2 fill.
  Loaded from `assets/branding/whistle-cover.png`; `tools/package.sh` now
  ships that one file. A missing file ends the splash at once.
- README header image.
- **Fingerprint changed.** `Cargo.lock` is a fingerprint input and gained the
  `winresource` package and the direct `winit` entry, so this tree no longer
  pairs with test.1 in multiplayer (the heading above describes M1a, not this
  change). M1b's planned `\r` strip changes the fingerprint again.
- Verified: gate green (fmt, check, clippy `-D warnings`; lib 95, district 7,
  session 41 + 3 ignored). New tests: splash timing (quick start, waits for the
  title up to the cap, a late cover keeps its hold, skip and missing cover, a
  stalled frame), cover fit (never stretched, title never cut on ten screen
  shapes, common screens filled), the icon decode (256 px RGBA8 with clear and
  solid pixels, accepted by winit), `Launch::splash`. The debug `el_silbon.exe`
  holds one icon group with all seven sizes (read as a data file, not run).
- Integrator (Windows, same tree): gate re-run green (lib 95, district 7,
  session 41 + 3 ignored). `cargo build --release --locked` green; the release
  exe holds one icon group, and its 256 px icon extracted with System.Drawing
  is pixel-identical to `whistle.ico`'s 256 px entry (16 and 32 px read back
  too). That entry and `whistle-icon-256.png` (the live window's) are the same
  art rasterised slightly differently (about 14 % of pixels differ, at most
  21/255): the kit's, not the code's. Package copy step reproduced without
  `zip` (absent from Git Bash here, so `tools/package.sh` stops at its last
  line on this machine) into `target/dist/el_silbon-branding-check`: it holds
  `assets/branding/whistle-cover.png`; the icons need no file. Headless
  two-process net smoke from the debug exe (127.0.0.1:5341): NET SMOKE PASS on
  host and client. The exe's version info (ProductName, FileDescription) still
  reads `el_silbon`, winresource's default: a possible follow-up, not changed.
- Unverified (user-led): the splash on screen and its skip; Explorer, title
  bar, Alt-Tab, running and pinned taskbar icons; the icon across
  fullscreen/window switches; a packaged build run from outside the tree
  (`asset_root()` prefers the compile-time `CARGO_MANIFEST_DIR/assets` while it
  exists, so on this machine set `BEVY_ASSET_ROOT` to the package folder, or
  use another machine); the X11 window icon on Linux (Wayland ignores window
  icons).

**Audio mix (fix 3).**
- New pure `src/mix.rs`: slider even in dB (0% silent, 5% = 2 dB), `MIX_HEADROOM_DB -4`, `DEFAULT_MASTER 0.8` (= -12 dB), and a glide that reaches its target at any frame rate.
- `Settings`: `master` / `music` / `ambience` / `effects` replace `volume`. The old key is ignored, so every tester's saved level resets to 80%.
- Buses: Music = theme, dread and omen stingers. Ambience = bed, rain, frogs, windmill, thunder. Master only = the whistle, Tureco's growl and bark, the hunt sting and the catch (a new `VoiceKind::Cue` for the dog and the hunt: no sub-slider may hide a truth about him). Effects = the rest.
- Volume previews. The catch is trimmed: ear whistle ×0.8, caught ×1.2.
- Verified: tests reproduce the old glide stall and pass on the fix.
- Unverified: every level by ear. Duck, hush and omen silences now reach their designed depth for the first time.

**Whistle distances (fix 4, tools).**
- `gen_audio.level()` sets each take's loudest second to WHISTLE_RMS, and a look-ahead limiter holds the peaks. Far takes 0/1 are no longer 4 dB quieter.
- `measure --check` now also gates levels and runs every rule at speeds 0.92, 1 and 1.06. Only the 12 whistle WAVs changed.
- Verified: check exit 0 on the recording and on the synth.
- Unverified: listening. The limiter takes up to 6 dB on one fragment of far takes 0 and 1.

**Whistle lab mirror (this pass).**
- `tools/whistle_lab.py` reads `SLIDER_FLOOR_DB`, `MIX_HEADROOM_DB`, `DEFAULT_MASTER` and `GLIDE_RATE` from `src/mix.rs` (not `SLIDER_STEP`), and mixes each sound on its bus with a per-sound cap at full scale.
- New flags: `--master` (`--volume` is an alias) and `--ambience` (bed only).
- The "played" column uses the curve: loud about -23 dBFS at the default.
- The lab no longer crashes when its output goes to a pipe or file on Windows.
- Verified: rendered whistle at master 100% / Ambience 0 is exactly -4.00 dB with the bed silent; default about -12 dB; master 0 silent. Measure exit 0 on the shipped files and on the synth.
- Renders are about 10 dB quieter than before, on purpose.

**Whistle captions (fix 4, captions).**
- `caption_look` depends only on the band: loud is 24 px, warm and upright; middling is unchanged; faint is 15 px, cold, alpha 0.8, and fades over its last 1.5 s; a phantom is grey.
- Cross-check: faint takes' last loud moment is at 2.9-3.2 s (3.1-3.5 s at speed 0.92), and the faint caption holds to 3.0 s, then fades while the sound dies away. Loud and middling files end at 2.7-3.4 s; their captions stay to 4.5 s.
- Unverified: how it reads on screen.

**Journal code leak (fix 11).**
- The Journal shows pages through the new `lore::keep`, which puts "·" where the digits were. The in-world page still shows tonight's digits.
- Verified: a test covers every page in both languages.
- Unverified: the rendered menu.

**Display mode (fix 5, user override).**
- `display_mode { Fullscreen (default), Window }` replaces `fullscreen`. The old key is ignored, so every test.1 profile opens fullscreen once.
- Settings is a hub: Video / Audio / Controls / Calibrate brightness.
- `--windowed` greys out the display-mode row. Leaving fullscreen gives a window fitted to 85% of the screen.
- No F11 or Alt+Enter (user decision).
- Verified: headless World test of the display switch.
- Unverified: every real-window behaviour on Windows.

**Brightness, contrast, calibration (fix 6).**
- Pure `src/display.rs`: log-contrast about the fog level, then a shadow gamma that keeps white fixed.
- At the defaults the picture is exactly the old one (exposure 0.3, gamma 1.0).
- New calibration page (`ui/calibrate.rs`) with three hats, offered once per profile (`Profile.calibrated`).
- `--menu-shots` now takes 21 captures.
- Unverified: on screen, the HATS levels, and a one-time flash of a few frames when the picture first leaves the defaults.

**Integration checks (this pass).**
- New test `profile::a_testers_saved_profile_loads_into_this_build`: a full test.1 profile keeps its pages, tally, join address, survivor and choices; master, display mode and contrast start at defaults; calibration is offered; the old keys are not saved again.
- No Rust code still uses the removed `volume` or `fullscreen` settings.
- Fingerprint unchanged: 0x93dcc314afdc3f17 from a real run, equal to v0.1.0-test.1 checked out with CRLF. (History: this is the test.2 value, `v0.1.0-test.2` at 93dfb95. M1b changed it; test.3 does not pair with test.1 or test.2.)
- The real test.1 Windows exe (built from CRLF sources) and this tree paired in both directions over headless loopback ("NET accepted").
- Gate green: lib 86, district 7, session 41 + 3 ignored, clippy clean. Headless two-process net smoke: NET SMOKE PASS on host and client, exit 0.

**Also in this pass.**
- The title screen's whistles no longer spend the loud and faint lessons
  (`hints_and_captions` teaches them only in `Flow::Playing`; the title's
  faint whistle used to mark "so he is NEAR" as taught before any night).
- Line endings enter the fingerprint (`include_str!` hashes raw bytes):
  a CRLF checkout on Windows pairs with test.1-windows, an LF Linux build
  does not. Strip `\r` in `fingerprint()` in M1b (it changes the
  fingerprint). **Done** in 18b47f8. Do not bump the crate version for test.2: `Cargo.lock` is
  a fingerprint input; tag the release instead.
- Open for the user's eye: middling and faint captions are both pale blue
  italics and may read alike; the calibration card can cover the hat
  labels below about 690 logical px of window height.

## Earlier handoff — 2026-09-29 (roadmap v3; M1 starting)

Research and design only so far; no gameplay code has changed except the
clippy fix below. A multi-agent pass read the code behind every playtest
note (below), researched co-op party and co-op horror games, and produced
"Roadmap v3" (next section): root causes and fixes for the playtest notes,
a slower night (El Respiro, La Rabia), fifteen voice-chat co-op features,
the radio numbers station for the truck key, and a build order (M1a, M1b,
M2, M3). The per-note investigations, with file and line evidence and fix
plans, are kept in `docs/research/2026-09-29-playtest-investigations.json`,
and a dense map of the current game in `docs/research/2026-09-29-game-map.md`.

- Fixed: clippy 1.97 `manual_range_contains` at `world/district.rs:1330`,
  which failed the gate on Windows with the newer toolchain.
- Windows development: the repository now also builds on Windows with the
  MSVC toolchain (`rustup override` for the checkout; Smart App Control must
  be off, since it blocks Cargo's unsigned build scripts). Gate green there
  at `3288232` apart from the clippy fix above. `python` on that machine is
  3.11 (the audio tools need only the standard library); `python3` is the
  Microsoft Store stub.
- Route sweep baseline on Windows at `3288232` (150 seeds, solo / shared):
  Normal 150/150, 150/150; Gentle 150/150, 149/150; Hard 136/150,
  134/150 (below the 97 % bar before any change; Hard is not the gate).
  Compare [b]-tier items against these, not against 150/150.
- User decisions (2026-09-29), from the roadmap's list: the dawn floor at
  1.5 × `night_length` (30 min on Normal) as a new `Outcome::Dawn`, yes;
  `anima_sight` on (the dead see what the friend they watch sees);
  `bleed_after_sack` 30 s. Local commits per item, no push until asked.
- **Fullscreen (user, overrides roadmap fix 5):** a display-mode option in
  a Video section of Settings, not a key. No F11 or Alt+Enter: on Hyprland
  (and Windows) a game-side fullscreen key fights the window manager. The
  full game launches fullscreen by default; test and debug runs (the
  `--smoke`, `--net-smoke`, `--menu-shots`, photo and trailer drivers, and
  a `--windowed` flag for hands-on testing) stay windowed.
- Next: M1a (fingerprint-neutral: audio mix, whistle distances, Journal
  code leak, fullscreen, brightness and contrast), then M1b.

## Roadmap v3: playtest fixes, a slower night, co-op comedy (proposal, 2026-09-29)

Nothing here is built. This roadmap answers the first remote playtest (the notes at the top of this file) and the user's request for a slower night with mechanics that are funny between friends on voice chat. It draws on three kinds of input:

- seven read-only investigations of the code at `3288232`
- six research passes over co-op party games, co-op horror and llanero folklore
- three competing design proposals

Where the inputs disagreed, the call is marked **Decided**, with the reason. File references are to `3288232`.

Every item keeps these rules:

- The whistle stays inverted, categorical and unplaced.
- Tureco is the only truthful proximity cue.
- Omens and phantoms are placed from the listener's own eye.
- `net::session` decides every outcome.
- All randomness comes from the seed.
- He stays slower than a walking player, and a warning lasts over 1.5 s.

Voice runs over Discord, so any voice comedy has to come from situations the game creates.

Cost is S (small), M (medium) or L (large). Each item also has a tier:

- **[a]**: presentation or an omen only. No sweep impact.
- **[b]**: a session rule the route driver may ignore. The fingerprint changes, so re-run the 150-seed `the_routes_hold_over_many_storms` sweep at Gentle, Normal and Hard.
- **[c]**: a mandatory objective. Everything in [b], plus a new `script.rs` Want.

### 1. Playtest fixes

1. **River** (M, [b]).
   - *Cause*: the caño is one deep `water` rect. All of it becomes Bank blockers except the bridge deck and a 6 m ford (`geometry/district.rs:664`, `:1340-1360`). The drawn waterline recedes up to 2.8 m inside the rect (`:1224`), so players hit an invisible wall on dry, reedy bank. The ford is drawn as dark as deep water (`:1230`), so nobody finds it.
   - *Fix* (the investigation's Phase 1):
     - `District::channels` carves the caño out of the banks.
     - A pure `Wade { Dry, Shallow, Deep }` from `Layout::wade` checks surfaces first and runs the rect tests before any terrain sampling.
     - Deep water: tuning `deep_wade_factor` 0.35, no sprint, `noise_deep` 14 m.
     - `Layout::rest_height` makes downed bodies and dropped bundles float (`control.rs:72`, `:349`, `net/mod.rs:698`).
     - The ford gets a raised, lighter bed.
     - The boat and the mooring stumps become Layout blockers.
     - The route A* treats deep water as TIGHT, and the map shades the channel.
     - He wades unslowed and silent (Phase 2A). A splash would place him.
     - This reverses PROGRESS.md:1724 for the caño only. The marsh, the pond and the creek stay banks.
     - Invert the check at `tests/district.rs:85-87`.
2. **Skill checks** (M, [b]).
   - *Cause*: a miss is applied (`net/session.rs:566-583`), but it only costs a fixed 8 %: 0.8 s of pump work, 0.48 s of truck work. That is less than the 2.05 s of full-rate work the task earns while the check runs, because `resolve_holds` never gates work on the rhythm (`:770`, `:805`, `:814`). Finishing a task also drops a pending check without penalty (`control.rs:319`, `skill.rs:132`). And the 40 m miss is no louder than the crank (34 m) and quieter than the engine (75 m).
   - *Fix*, in the pure `skill::Rhythm`:
     - A miss throws back `check_setback` (2.5 s of work) and stalls the hands for `check_stall` (1.8 s). This is a new `Pulse::Stalled`, and the stall survives `rest()`.
     - `allow()` holds the last 1 % of a task while a check is in flight (`LAST_TURN`).
     - The miss noise is `max(noise_miss, work noise × 1.6)`: about 54 m at the pump and a 120 m backfire at the truck.
     - `Vitals.stall` carries the stall to the HUD, which shows a label.
     - Rewrite the test that only checks the dip (`tests/session.rs:482-533`), and make the co-op pump test answer its checks (`:885`).
3. **Audio level** (M, [a]).
   - *Cause*: the master slider is linear amplitude with a default near maximum (0.8 is only −1.9 dB; `app.rs:39`, `ui/menu.rs:1125`), and the mix has no headroom. The loop glide in `audio.rs:976-983` also stops a frame-rate-dependent distance short of its target (±0.058 at 144 Hz). So 0 % is not silent, the title theme plays under the whole night, and the whistle duck, the insect hush and the omen silences never reach their designed depth.
   - *Fix*:
     - A dB slider with a −40 dB floor and 5 % steps.
     - Master, music, ambience and effects buses, with −4 dB of headroom. `Settings.master` replaces `volume`, so stale saved volumes reset.
     - The whistle and the catch stay on master only, so no sub-slider can hide him.
     - A frame-rate-independent `glide()` that snaps to its target.
     - An audible preview when a slider moves, including in the pause menu.
     - Catch trims: ear whistle ×1.7 → ×0.8, caught sting ×1.6 → ×1.2.
     - `whistle_lab.py` mirrors the new curve.
     - Nothing touches `tuning.rs`, so the fingerprint is unchanged.
4. **Whistle distance** (M, [a]).
   - *Cause*: with the recording, the three bands differ only in level. `gen_audio.py:410-414` drops the breath and air the design relied on. Filtering a near-pure tone only scales it, and `level()` normalises that away: the middling and faint spectra correlate 0.963. Random pitch and take variation within a band is also larger than the difference between bands.
   - *Fix*:
     - Bake three signatures into the 12 WAVs:
       - near: breath and air, dry, an inhale and an exhale
       - middling: a clear quarter-second slap
       - far: seeded fragments that always keep the final held note, a shimmer, mostly room
     - Gate them with `python tools/whistle_lab.py measure --check`, on both the recording build and the `--synth` build. On this machine the interpreter is `python`; `python3` is the Store stub.
     - Style the captions per band: loud is large and warm, faint is small and fading.
     - Gains, thresholds and the inversion are untouched.
     - Re-listen only after the audio-level glide fix, because the duck never reached its depth before.
     - Before the next remote test: Windows Sound → Communications → "Do nothing", and Discord attenuation at 0 %.
5. **Fullscreen** (S, [a]).
   - *Cause*: the toggle exists, but it is the last Settings row, labelled "Display: Window" with no chooser marks (`ui/menu.rs:453`). There is no F11 or Alt+Enter. The saved mode is applied a frame late, by a one-shot in `remember` (`:1168`). It was never checked on Windows (PROGRESS.md:726).
   - *Fix*:
     - `Settings.fullscreen` is the single source of truth.
     - `app.rs` creates the window in the saved mode (`MonitorSelection::Primary`), and a single `apply_display` system mirrors later changes, restoring a centred 1600×900 window when leaving fullscreen.
     - F11 and Alt+Enter work in PreUpdate. Alt+Enter consumes Enter, so it cannot start the host's night or name him by accident.
     - The row moves to the top of Settings and is labelled "Fullscreen".
     - The controls text in How to play, the briefing and PLAYING.txt mentions F11.
6. **Brightness and contrast** (M, [a]).
   - *Cause*: brightness is an exposure offset worth at most ×1.6 (`player.rs:529`), which cannot lift him out of the near-black band on panels that crush blacks. Bevy's sectional contrast pivots at linear 0.5 and wrecks night HDR (`player.rs:184`). Nothing shows players what "correct" looks like.
   - *Fix*:
     - A new pure `display.rs`: brightness becomes a shadow gamma that holds white, and contrast becomes a log-contrast about the fog level. Both go through sectional CDL gamma plus exposure. At the defaults the picture is bit-identical to today.
     - A calibration page with three unlit hats (should vanish / barely visible / plainly seen), offered once on first launch. This also fixes discoverability.
     - The trailer keeps its look through `EXPOSURE_LIFT`.
7. **Finding downed allies** (M, [b]).
   - *Cause*: a downed friend is usually sacked and dropped somewhere else with the bleed still running (`session.rs:1077-1105`). Once dropped, they lie prone and dark, because the beam is hidden for any non-zero status (`net/mod.rs:735`). They are also silent (`audio.rs:811`). There is no marker and no distance.
   - *Fix* (the investigation's Tier 1):
     - The downed friend's own torch lies lit in the grass and sweeps as they look around.
     - Placed groans every 3.5–8 s, more often as they bleed, in their survivor's pitch.
     - A truth-only HUD marker, and the distance on the roster.
     - A ring on the map.
     - `scene_data` stops counting hauled players as bodies.
     - None of these show while the friend is in his sack.
     - Also the help call, the first kind of the call wheel (section 3), and `bleed_after_sack` 30 s (recommended; confirm in play).
8. **Spectating** (M–L, [b]).
   - *Cause*: snapshots and cues are built only from the recipient's own pose (`session.rs:1386-1454`), so the dead stare at frozen grass. A Taken player also keeps stale stun (`body.rs:156`).
   - *Fix* (the investigation's Phase 1):
     - `Action::Watch`, auto-advancing when the watched friend falls.
     - An over-the-shoulder camera authored in `net::update` (Simulate).
     - The watched player's categorical cue is copied with the same serial.
     - `die()` clears stun.
     - An `anima_sight` tunable (see decisions in section 5).
     - Haunts come in M2 (Ánimas).
9. **Rising difficulty** (L, [b]).
   - *Cause*: laying a bundle nets only +0.01 pressure on Normal (rite 0.08 minus the 0.07 carry it replaces), and lowers pressure on Gentle, because `with_night` scales rite but not carry (`tuning.rs:604`). The clock's 0.5 drowns the bones. Every lever is a smooth lerp. Nothing says he is angrier, and the caption at `ui/hud.rs:616` is calming.
   - *Fix*: La Rabia, in section 2.
10. **Telling the tale** (S, then M, [a]).
    - *Cause*: the legend is 20 unordered pages, and nothing ties it to progress.
    - *Fix*: section 4. The Madrina's chapter per bundle laid comes in M1, the radio's tale station in M2, and the four survivor versions in M3.
11. **Truck key code** (L, [c]).
    - *Cause*: the digits sit in three pages that look like the other seventeen (`lore.rs:116`, `:127`, `:280-289`), and the game rewards reading all twenty. The pause Journal also fills pages read on earlier nights with tonight's code (`ui/menu.rs:404`), so returning players can skip the puzzle.
    - *Fix*: the radio's numbers hour (section 4). Close the Journal leak at once in M1a.

### 2. Slower pacing

**Target.** A Normal co-op night runs 15–20 min; solo 12–18 min, Gentle about 20–25 min, Hard 12–16 min. Today the scripted route wins inside 10 min, and no human night has been timed. The slower pace comes from beats that make people stop, listen and talk. Walk, crouch and sprint speeds do not change.

**Decided:**

- 15–20 min rather than 20–28 min: the user asked for "a bit" slower, and doubling today's night keeps "one more night" alive.
- The module is a new pure `pacing.rs`, and the feature is called **El Respiro**.

**Tuning** (`src/tuning.rs` unless noted):

| Value | Now | v3 | Why |
|---|---|---|---|
| `night_length` | 840 s | 1200 s | the clock spans a 20-minute night |
| `pressure_base` | 0.12 | 0.10 | a calmer opening |
| `pressure_night` | 0.5 | 0.2 | the clock stops drowning the bones |
| `pressure_carry` | 0.07 | 0.04 | carrying stays risky, and laying becomes a step |
| `pressure_rite` | 0.08 | 0.12 | a laying nets +0.08 on Normal (was +0.01) |
| `with_night` | scales rite only | scales carry and rite by the same 1.3 / 0.5 | a laying can never lower pressure |
| `first_warn_delay` | 9 s | grace: Gentle 240, Normal 120, Hard 45 s | a real setup phase after he wakes |
| `stalk_phrase_interval` | 5–11 s | 6–13 s × (1 − 0.05·stage) | sparser early, denser than today by stage 5 (4.5–9.8 s) |
| `stalk_silence_chance` | 0.2 | 0.2 − 0.04·stage | he fills the silence as he angers |
| `stalk_answer_chance` | 0.15 | 0.15 + 0.03·stage | as above |
| stalk_gap pressure factor (`perception.rs:166`) | 0.35 | 0.2 | the stage is the change players feel |
| `omen_quiet` | 35–70 s | 45–85 s × (1 − 0.08·stage) | fewer omens early |
| `pump_hold` | 10 s | 12 s | the pump is a longer stand |
| `deliver_hold` | 3 s | 3 s | the new miss rule already adds cost |
| `truck_warmup` | 26 s | 26 s | at pressure 1.0 (hunt 3.1 m/s) a longer warm-up is only more lethal |
| `check_loss` | 8 % of the task | `check_setback` 2.5 s + `check_stall` 1.8 s | see fix 2 |
| carry cap | none | new `carry_max` 2 (Gentle 3) | at least three trips; the scripted route already carries at most 2 |
| deep caño | a wall | 0.35 × walk, no sprint, +14 m | see fix 1 |
| `grass_sight` | 5 m | 5 + 0.8·stage (7.4 m at stage 3) | a rage lever, gated on the sweep |
| `light_lure_range` | 45 m | 45 + 4·stage (53 m at stage 2) | a rage lever, gated on the sweep |

**El Respiro, the pacing director** (L, [b]). A new pure module, `src/pacing.rs`. Add it to `lib.rs`, the AGENTS.md pure list and `protocol::fingerprint()`.

- **Grace.** After the first pickup he rises as today, at least 30 m from everyone.
  - For the grace period he cannot warn anyone: `th.cooldown = th.cooldown.max(grace_left)`.
  - He is leashed at least 30 m from every standing player.
  - He still walks, whistles honestly and investigates noise, so the calm teaches the inverted whistle.
  - **Decided:** the grace does not end at the first laying. The house-table bundle is always near the start, so the grace would often be under a minute.
- **Menace.** The director keeps a menace value M from 0 to 100, the worst over all active players. It is read from session truth:
  - +12/s while he sees someone
  - +8/s while someone is inside 16 m
  - +5/s while a torch lures him
  - +4/s while Tureco growls
  - +3/s while someone's fear is at 0.7 or more
  - M decays 4/s.
- **Cycle: Build → Peak → Fade → Relax.**
  - **Build**, 60–120 s: normal stalking. This is also where roadmap A1's placed fake-outs play: a cow lows beside you, a door bangs, Tureco knocks a bucket over. At most one real scare every 4–6 min.
  - **Peak**: warnings and hunts are allowed. It ends when a hunt resolves (a down, lost track, or three averts into a withdraw), when menace stays at 80 or more for 45 s, or after 120 s.
  - **Fade**: the existing `Encounter::withdraw()`. He sinks and rises at the patrol node farthest from everyone.
  - **Relax**: 75–120 s × (1 − 0.1 × bones laid), never under 40 s. Hard ×0.7, Gentle ×1.3.
    - He is leashed at least 45 m from every standing player, so the ordinary whistle honestly reads loud and Tureco settles on his own.
    - Nothing in the first 40 s can end it. After that it ends on a push: laying a bundle, cranking, turning the ignition, lighting the beacon, any noise of 34 m or more, or someone walking within 30 m of him.
    - Lifting and carrying bones is not a push.
- **Hunt budget.** Committed hunts per rolling 10 min: Gentle 1, Normal 2, Hard 3. Once the budget is spent, his warning floor holds until the next slot opens. He still stalks, so the night stays tense.
- **Hidden state.** The phase is hidden AI state and never goes on the wire.
  - **Decided:** no phase-keyed "the llano sings" event. It would broadcast the director's state; the honest loud whistle already says he is far.
- **Dawn floor** (*Decision*). The rooster crows at 1.5 × `night_length` (30 min on Normal) and he sinks for the night. With bones still out, the night ends as a new `Outcome::Dawn`, "you lived, but he will be back", tallied in the Journal. It gives slow teams a floor instead of a death spiral.
- **Build.**
  - `sim`: `Threat.leash: Option<f32>`, honoured by the patrol step through the existing `farthest_from`.
  - `session`: builds the menace inputs, applies the director's orders before `threat_step`, and calls `push_forward()` from the push sites.
  - `Stats` gains seconds per phase, peaks, hunts and early exits from Relax, logged in `smoke_exit`, so the next remote playtest gives the first human-timed night.
- **Tests.**
  - Pure: a Relax of at least 40 s always follows a Peak; hunts per rolling 600 s never exceed the budget; a push ends Relax only after the floor; the same seed gives the same beats.
  - Session: a player in plain view 10 m from him is never warned during the grace or a Relax; after a sack resolves, he stays at least 45 m from every standing player until a push.

**La Rabia, bones drive the night** (M, [b]). The rage stage is the number of bones laid (0–5). It is public, because it is already `WorldView.delivered`, so no protocol change is needed. Continuous pressure stays hidden (`tests/session.rs:1325` must stay green). A pure `Tuning::at_rage(stage)` sits beside `at_pressure`, and `Progress::rage()`, `CueDirector` and `Mood.rage` read it. Following Andy Bray's rule for Alien: Isolation, nothing ever unlocks because a player died.

| Stage | He learns |
|---|---|
| 1 | whistles and omens start to crowd in, and do more at every later stage |
| 2 | a lit torch draws him from 53 m; the stolen-torch omen unlocks; La Cuenta becomes possible (M2) |
| 3 | he finds you in tall grass from 7.4 m (was 5 m) |
| 4 | at a spent ají ward he waits at the edge (M2, a Tier 2 lever); a second Cuenta |
| 5 | the engine calls him (as today); Relax is at its floor |

- **Telegraph.** Each laying brings:
  - an unplaced stinger (a gust, roots groaning, far thunder; never a whistle; `sting_rage` from `gen_audio.py` on a new seed)
  - every lamp on the llano stutters for 1.2 s
  - anger pips on the bones line ("his anger ■■□□□")
  - a stepped dread drone
  - the Madrina's chapter, whose last line hints at the new trick (section 4)
  - Layings within 8 s of each other merge, because players tend to lay in bursts of 1, 2 and 2.
- **Decided:** stage 5 does not raise the chance of silence (one proposal). That would fight the rule that silence falls with each stage.
- **Later levers.** Habit unlocks, fed only by escapes (for example, three grass escapes bring the grass sight a stage early), and the ají edge-wait land in M2, one at a time, each behind the sweep. Lanterns going out for good waits until brightness is verified by eye.
- **Tests:**
  - `every_bundle_laid_is_a_step_up_on_every_night_and_variant` replaces `sim.rs:1743`.
  - `rage_only_tightens_and_never_outruns_a_walker` (stages × pressure × Night).
  - A crouched player 7 m away in grass is unseen at stage 0 and seen at stage 3.
  - Stalking gaps are denser at stage 5, but still with no steady rhythm.

**Other slowers.**

- The skill-check stall (fix 2).
- Deep wading (fix 1).
- The carry cap.
- The radio relay: the radio and the key box are 38 m apart (section 4).
- La Cuenta vigils (M2).

**Timeline** (Normal co-op, after M1):

| Time | Beat |
|---|---|
| 0–2 min | Dusk: untie Tureco, read the tag on the padlock, plan |
| ~2 min | First pickup, then 2 min of grace |
| 4–9 min | Bundles 1–2, one Peak, one Relax |
| 9–14 min | Bundles 3–4, one or two Peaks (M2 adds a vigil at each stage) |
| 14–16 min | Bundle 5 and the pump (12 s plus checks) |
| 16–18 min | Radio relay and the key |
| 18–20 min | The truck, or naming him at the ceiba |

**Tests and caps that move on purpose.** Record each move in the handoff.

- The scripted solo route bound: 3–14 min → 5–22 min.
- "Quick hands": under 8 min → under 12 min.
- The net-smoke host cap: 600 s → 1500 s.
- The `script.rs` route timeouts: 600 s → 1500 s.
- The sweep stays at ≥97 % solo and shared, at all three difficulties.

The tuning invariants (hunt and stalk slower than walking, warnings over 1.5 s) gain a grid over stage × pressure × Night.

### 3. Voice-chat comedy and co-op

**Principles.**

- Voice comedy comes from information asymmetry: one player hears, sees or holds what the others cannot.
- Every in-game way to talk costs noise he hears.
- Fakes need a fair tell.
- The dead get jobs that cannot reveal where he is.
- Every co-op rule has a solo form.

1. **La Cuenta: someone has to hear him count** (M, [b], M2).
   - *What*:
     - After the 2nd and 4th bundle, the director turns the next Relax into a vigil.
     - A caption names one enclosed building, chosen by the seed (the rancho, the stilt hut or the lookout cabin). After a 20 s lead-in he counts his father's bones for 30–40 s.
     - He is Hidden (sunk) throughout, and the unplaced bone clacks are heard only inside that building.
     - The rule: at least one listener (two when three or four players stand) stays inside to the last clack, with the torch off, not sprinting, and making no noise louder than a walk. Breaking the rule restarts the count once, faster.
     - Heard: he rises far away, the listeners' fear drops 0.2, and Tureco's courage refills.
     - Unheard: the standing player farthest from the building is *señalado* until the next laying. He prefers them as if they were 15 m closer, their omens come sooner, and their torch drains ×1.5. An ají thrown at their own feet lifts the mark.
     - The rhythm is naming evidence: the Borracho loses count and starts over, the Hijo sobs before the last bone, and the Arriero clacks in pairs.
   - *On voice*: "EVERYBODY SHUT UP, HE'S COUNTING." Then "seven or eight?", then "pairs, it's the Drover".
   - *From*: the legend's own rule (if nobody hears the count, someone does not wake); Buckshot Roulette's public count; Don't Scream.
   - **Decided:** he is Hidden, not standing at the door. A building he is known to stand at would be a new truthful location of him. Failure gives a readable, reversible mark rather than a death at dawn.
   - *Build*:
     - `sim`: `Encounter::hide()` and `rise()`.
     - A pure `count.rs`: `beats(variant, rng)` and `listeners_needed(standing)`.
     - The vigil sites are the existing enclosed `district.sheds`, tested with `Layout::inside`.
     - Session: `Vigil { site, t, restarts }` and `Participant.marked`, which feeds `choose_prey`, the omen interval and battery drain.
     - Protocol: `WorldView.vigil` (serde default); `Event::{CountBegan, CountHeard, CountUnheard, Marked}` appended.
     - Audio times the clacks from `snapshot.elapsed`.
     - The vigil is optional, so the driver gets no new Want; expect scripted routes to be marked in the sweep.
   - *Tests*: no WarningBegan during a vigil; staying through clears it; leaving restarts it once, then fails it; an unheard count marks the farthest standing player; ají clears the mark; the patterns differ by variant; deterministic per seed.
2. **Gritos y silbos: the call wheel** (M, [b]; the help call in M1, the rest in M2).
   - *What*: hold Q for the wheel.

     | Call | Effect | Noise he hears |
     |---|---|---|
     | ¡Aquí! | a placed call | 18 m |
     | ¡Corran! | flashes everyone's HUD | 30 m |
     | ¡Shh! | a silent gesture | none |
     | your survivor's silbo | two or three finger-whistle notes at 600–1000 Hz, never his rising steps; Tureco trots to his friend's silbo | 26 m |
     | ¡Ave María Purísima! | asks who is real | a shout |
     | ¡Sin pecado concebida! | the reply; a real one flashes a true 2 s marker over whoever answered | a shout |

     - Every call is placed at the caller, so the group learns that anything with a direction is a friend; his whistle never has one.
     - Downed, V becomes a hoarse ¡Auxilio! at your own body: 18 m, 8 s cooldown. The captive and the dead cannot call.
   - *On voice*: every call is an argument ("stop whistling, he'll come back!"). Groups invent codes, and whoever spams the silbo becomes the night's villain.
   - *From*: roadmap B2; the Lethal Company walkie-talkie's cost; Sea of Thieves' shout wheel; the superstition against whistling at night; the doorstep call and response.
   - **Decided:** one call system, whose first kind is the downed help call. There is no separate `PingView.help` flag.
   - *Build*:
     - Protocol: `Action::Call { kind }`, accepted before the `is_active` guard like `Ping`; `Snapshot.calls: Vec<CallView>` (serde default).
     - Session: `call()` with a cooldown and radius per kind (tuning `noise_call`, `noise_corran`, `noise_silbo`, `call_cooldown`).
     - A pure `Survivor::voice()` gives each survivor's pitch.
     - Clips from `gen_audio.py` on new seeds.
     - `Deeds.calls`, and the awards "Silbó de noche" and "Called for mamá".
     - Check in `whistle_lab.py` that the silbo band misses his 1.2–2.35 kHz phrase.
   - *Tests*: a call draws a present, stalking threat inside its radius and not beyond; rate limits hold; the captive and the dead are refused; a help call lands on the caller's own body; a reply marker only answers a live Ave María.
3. **El Compadre, the borrowed call and the house that is not safe** (S–M, [a], M2).
   - *What*: new director omens, all placed from the listener's eye.
     - **El Compadre** (co-op, fear 0.55 or more): a living teammate's own survivor model, torch lit, waves slowly from a treeline 25–40 m away. It is always a teammate the viewer cannot see. When a beam or an Ave María reaches it, it stretches to three metres under a sombrero for 0.4 s and is gone. It never answers.
     - **The borrowed call**: once a friend's call has been heard three times, another listener at fear 0.7 or more hears it from 15–25 m, at least 10 m from the real friend. A fake silbo has one note too many.
     - **The house** (roadmap A5), while you are inside: knocks on the wall you are not facing, a door bangs, the local radio sound bursts into static, the porch lamp dies for a moment.
   - *On voice*: "Is that you waving at the ceiba?" "I'm in the TRUCK." It hits hardest with two players, because there is only one friend it can be.
   - *From*: Lethal Company's Masked; the Skinwalkers and Mirage mods; MIMESIS; Unfortunate Spacemen.
   - *Build*:
     - `director.rs`: new omens, with `Mood.indoors` and `Mood.learned`; events appended.
     - `world/omen.rs`: an `AvatarKit` stand-in wearing the teammate's survivor and colour.
   - *Tests* (director): Compadre only in co-op, over the fear threshold and never under pursuit; house omens only indoors; deterministic.
4. **Ánimas: haunt the friend you watch** (M, [b], M2; Phase 1 spectating is in M1).
   - *What*:
     - Every 45 s an ánima can haunt the friend it watches: flicker lamps, knock, footsteps, a false mark, a phantom whistle, a nudge of the radio dial, or a candle over a downed friend's body.
     - The candle is the one act signed with the dead player's colour. Every other haunt except the dial nudge looks exactly like the night's own omens.
     - The dead also see dropped bundles glow, and downed friends burn like candles (never while hauled).
     - Haunts are refused while the target is warned, hunted or mid check, and they never change fear, noise or rules.
   - *On voice*: "Was that you?" "...Was it?" The dead backseat-drive the rescue ("left, LEFT, past the trough").
   - *From*: roadmap B4; Blood on the Clocktower's ghost vote; Mysterium; PEAK's ghosts; Among Us ghosts; DeadAndBored.
   - **Decided:** no herd-spooking haunt and no Tureco haunt. A 48 m bellow would change the rules, and Tureco must stay truthful.
   - *Build*:
     - `Action::Haunt { kind }`; `Participant.haunt_wait` and `rest`; the ánima's own `Rng::fork(seed ^ id, 0xA417)`.
     - `director::hush()`; `perception::conjure()`.
     - Tuning `anima_every` 45, `anima_rest` 20.
     - The candle is a `Ping` by the dead player's id. Award "Restless soul".
   - *Tests*: cooldowns and refusals; noises, threat state, fear, stun and progress unchanged across a haunt; a phantom whistle reaches only its target.
5. **El desmayo: faint, then a slap or your name** (M, [b], M2).
   - *What*:
     - In co-op, a susto while he is not pursuing you becomes a faint: you drop your load, the screen goes black and muffled, and the camera drifts up out of your body for up to 8 s.
     - A friend can slap you awake (a press, instant, a 14 m noise he hears) or call your name (a 2 s hold, 4 m). Agua florida also works.
     - Unhelped, you wake at fear 0.55. After that, 60 s of immunity.
     - Under pursuit a susto stays today's 1.1 s freeze. Solo never faints.
   - *On voice*: "Don't SLAP her, he'll hear!" Award "Cachetada de oro".
   - *From*: the folk illness susto and the cure of calling the soul back; Gang Beasts (short knockouts, no chains); PEAK's 8 s ragdoll.
   - **Decided:** waking unhelped leaves you at fear 0.55, not downed. Helplessness should last seconds.
   - *Build*:
     - `body.rs`: `faint` and `faint_immunity`; a pure `startle(company, pursued)`.
     - `control.rs`: `TargetKind::Fainted(id)`.
     - Protocol: `PlayerView.faint`, `Vitals.faint` (serde default).
     - The soul camera is written in `net::update`.
   - *Tests*: faints only in co-op and never under pursuit; a slap wakes you and makes a noise; carried bones drop; the immunity holds; he can still catch a fainted player.
6. **El desamparado: whoever leaves a friend is his** (S, [b], M2).
   - *What*:
     - In co-op, stay more than 70 m from every standing teammate for 25 s while someone is downed, fainted or in the sack, and you become desamparado: grain on the screen, and Tureco howls where he stands.
     - His prey choice treats you as 15 m closer, and omens crowd you.
     - Coming back within 30 m of a teammate clears it. Speeds and warnings are untouched.
   - *On voice*: aimed at the friend who waits alone at the truck. "Told you."
   - *From*: PEAK's Scoutmaster (Rule Zero); roadmap B3.
   - *Build*:
     - Session: `Participant.alone`, `forsaken`.
     - `choose_prey` bias, shared with the mark and the drunk.
     - `Vitals.forsaken`; award "Dejó al compañero".
   - *Tests*: never solo; only under the conditions and after 25 s; clears on regroup; he prefers the forsaken over a closer teammate.
7. **Mandados and the copla at dawn** (M, [b], M2).
   - *What*:
     - Each survivor privately draws one errand from the hato's old people, dealt from the seed and their id, from a pool of about 24. Examples: pet Tureco three times; leave an ají at the ceiba roots for the ánimas; never light your torch in the corral; be the last aboard; carry a bundle across the caño without dropping it; keep two spare batteries until dawn; answer every Ave María; hear a whole count.
     - There are no traitors.
     - At dawn the errands are revealed next to the awards, and a four-line copla of the night names the survivors ("Y el Encargado, que juraba no tenerle miedo, soltó el saco en el caño"). The copla is saved to the Journal and opens the next title screen.
   - *On voice*: "Why are you hoarding batteries?" "No reason." The reveal is the screenshot.
   - *From*: Dead of Winter's secret objectives; Dale & Dawson's red tasks; Wildermyth.
   - *Build*:
     - A pure `errand.rs`: `deal(seed, id)`, distinct within a party, and `done(&Errand, &Deeds)`. Add it to the fingerprint.
     - The needed `Deeds` counters (serde default).
     - A pure `awards::copla()`.
     - Each client computes its own errand from the seed; that is acceptable among friends.
   - *Tests*: errands are deterministic and distinct; `done` depends only on deeds; no errand changes the Outcome; the copla is four lines and names only present survivors.
8. **El paso del caño** (M, [b], M2, after fix 1).
   - *What*:
     - A bundle dropped in the deep channel drifts east at 0.3 m/s until it lodges at the bridge piles or the fence.
     - Crouching in deep water with the torch on douses it for 25 s, so you hide low in the reeds, in the dark.
     - Sprinting onto the bank in heavy rain can slip you in: a 1.2 s stun, one bundle into the current, an 18 m splash. A friend on the bank holds E to pull you out (roadmap B6).
     - The pond is treated like the channel, which removes its invisible wall too.
   - *On voice*: "You dropped the tibia in the RIVER." "Chase it!"
   - *From*: the river investigation's Phase 2; R.E.P.O.'s fragile loot.
   - *Build*:
     - `sim`: drift of `Relic::Ground` where `wade == Deep`.
     - Session: `Participant.soaked`; `Event::{TorchSoaked, Slipped}` appended.
     - A pure slip rule in `body.rs` (sprint, rain above 0.85, within 1 m of the edge, seeded per player and tick).
     - Optional: he walks on the water (presentation only).
   - *Tests*: drift is deterministic and stays in the caño; a soaked torch cannot light; slips need rain and a sprint; the pond edges are free.
9. **La repisa: aguardiente and tonight's remedios** (M, [b], M3).
   - *What*:
     - The aguardiente always works the same way: fear clears, no susto or faint for 90 s, and the skill zone widens ×1.3.
     - The price: you hiccup (a 6 m noise every 15–30 s that friends hear and you barely do), your avatar sways, and he prefers you (as if 12 m closer, 18 m on a Borracho night).
     - The first time he catches a drinker, he draws the drink out through the navel instead of downing or sacking them: they are left sober at fear 1.0 with a 2 s stun, and he sinks away. Once per player per night.
     - Beside the bottle, four unlabelled remedios (guarapo, café negro, chimó, agua florida), with effects dealt per night: second wind, steady pulse, hiccups, heavy lids, loose tongue (your calls come out twice), cat's eyes (a local brightness lift). Agua florida always wakes a fainted friend.
     - Only the taker sees what theirs did.
   - *On voice*: "Stop drinking, he's coming for YOU." The volunteer decoy narrates their stagger, and "red is bad tonight" might be a prank.
   - *From*: the legend (he hunts drunkards); PEAK's shroomberries; Phasmophobia's cursed items.
   - **Decided:** the drinker keeps the inverted whistle. Un-inverting it per player breaks a pinned, tested rule.
   - *Build*:
     - A pure `remedy.rs`, `tonight(seed)`, added to the fingerprint.
     - Shelf points in the Layout; `TargetKind::Shelf(i)`.
     - Session: effects; the drunk catch path (`Event::Sobered`).
     - `PlayerView.drunk`, `Vitals.effects`.
   - *Tests*: deterministic per seed; the first catch of a drinker never sacks them; a `CueDirector` gives identical phrases with and without every effect.
10. **A cuestas y en hombros** (M–L, [b], M3).
    - *What*: one mount system with two verbs.
      - **Piggyback**: hold G on a downed friend. Their bleed pauses, and you lose your bones and your torch hand. They ride facing backward with the torch and the ají.
      - **Shoulders**: a friend climbs your crouched back. Their eye rises 0.9 m over the grass and can reach one or two seeded high bundle spots. You steer at crouch speed, seeing only stalks, and he sees the rider from ×1.3 farther.
      - The pair topple (2 s stun, a 14 m clatter) on a sprint, a susto, deep water or his warning.
    - *On voice*: "Left... your OTHER left." "Shine it behind us!" "No, the light draws him!"
    - *From*: PEAK; Human: Fall Flat; R.E.P.O.
    - **Decided:** the carrier moves at walk × `carry_floor` (2.23 m/s), not 0.72 × walk (2.6 m/s). A carried friend should be as dangerous to haul as a full load of bones.
    - *Build*:
      - The rider's pose is pinned to the carrier each tick, the same pattern as the captive at `session.rs:1124`.
      - `Action::Dismount`; `PlayerView.mount`.
      - High relic sites in the Layout.
    - *Tests*: a carried friend does not bleed; mounted speed never exceeds walk × `carry_floor`; high sites are reachable only from shoulders; he can still sack the carrier.
11. **La Vigía: binoculars at the Mirador** (S, [a], M3).
    - *What*:
      - Binoculars lashed to the lookout rail: hold to zoom ×3–4 while standing still.
      - The deck already sees 130 m, so you see friends' torches, and him only in lightning or lamplight.
      - Twenty painted posts and named spots ("poste 12", "el tanque") appear on the ground and on the map.
      - The deck is open to his eyes, and a lit torch up there is a beacon.
    - *On voice*: "He's by the cow thing!" "THE CORRAL?" "NO, THE OTHER COW THING."
    - *From*: Lethal Company's radar operator; Iron Lung; Sea of Thieves' crow's nest.
    - **Decided:** the lookout gets no wider sight rule; zoom only.
    - *Build*: `District::scope` and `posts` in the Layout; the map reads them; a local FOV zoom in `player.rs`.
    - *Tests*: post numbers are unique and stand on free ground.
12. **Contener el aliento, and don't look at him** (M, [b], M3).
    - *What*:
      - A6: crouched in grass or within 1.5 m of a wall, hold C. Your step noise drops to ×0.3 and he notices you at ×0.6. Stamina drains, and letting go (or running out) makes a gasp he hears: 10 m, or 16 m if you ran dry.
      - A7: during a warning, keeping him within 20° of your view fills fear 0.06/s faster.
    - *On voice*: four friends whispering "don't breathe" while one runs out of air.
    - *From*: roadmap A6 and A7; The Outlast Trials; Don't Scream.
    - *Build*: `body.rs`; an `Input` bit (serde default); `Layout::near_wall`; A7 in `fear_step` from session truth.
    - *Tests*: holding lowers noise and drains stamina; a release always gasps and the gasp draws focus; refused in the open.
13. **Agüeros and the night of the week** (S–M, [b], M3).
    - *What*: about 60 % of seeds deal an omen card at the briefing:
      - **Luna llena**: little rain, and he notices you farther away, as you see him farther away.
      - **Sin Tureco**: he ran off after a cow; find him first.
      - **Pilas chinas**: batteries last 60 %, but there are 8 spares.
      - **Invierno**: the ford runs deep.
      - **Velorio next door**: candles count as lamplight, but omens crowd you.

      A "Noche de la semana" seed comes from the ISO week, with best times kept in the Journal.
    - *On voice*: comparing the weekly night between friend groups.
    - *From*: roadmap B8; Phasmophobia's weekly challenges; The Outlast Trials' Variators.
    - *Build*:
      - `Twist::of(seed)` and `with_twist` after `with_night`. The twist comes from the handshaken seed, so there is no protocol field.
      - `ROUTE_TWIST` for the sweep.
    - *Tests*: the invariants hold for every twist; Sin Tureco's dog is reachable; the sweep stays at ≥97 % per twist.
14. **La Kodak: flash photos developed at dawn** (M, [b], M3).
    - *What*:
      - A 12-exposure camera sits in the truck's glovebox. The flash lights about 20 m for 0.25 s and draws him like a beam.
      - At dawn the roll plays as a slideshow captioned from the night, and is saved as PNGs.
      - If he was truly in the frame but unseen, the print shows his hat brim. The host records this, and sends it only with the outcome.
    - *On voice*: watching your friends' worst moments back together.
    - *From*: Content Warning; Phasmophobia's photo camera.
    - *Build*:
      - `Action::Photo`.
      - A host-only `Shot { t, in_frame }`; the outcome snapshot carries `shots` (serde default).
      - Client capture through Bevy's `Screenshot` (the F12 path).
      - Each player sees only their own roll in version 1.
    - *Tests*: film runs out; the flash draws focus; `in_frame` never appears in a snapshot before the outcome.
15. **El mandador: three cracks in a cross** (M, [b], M3).
    - *What*:
      - One drover's whip hangs in the corral each night.
      - With him warning or hunting you within 15 m, crack it left, right and forward, each on the skill needle.
      - Three hits: he flinches away like Tureco's bark and drops the sack.
      - Any miss: the crack only tells him where you are (a 60 m noise), and the whip is spent.
      - On an Arriero night it is his own whip: he does not flinch.
    - *On voice*: "Don't use the whip, I think it's the Drover!"
    - *From*: the legend's three wards (whip, dog, ají); Pacify's sacrificial doll.
    - *Build*: a `skill` Rhythm kind that chains three checks; `encounter.flinch()` unless `Variant::Arriero`; `Event::WhipCracked`.
    - *Tests*: three hits make a non-Arriero flinch; the Arriero never does; a miss focuses him on the cracker; one whip per night.

### 4. Telling the tale and the key-code puzzle

**The key code: La Voz del Llano** (L, [c], M1b).

- **The dial.** The shelf radio gets a shared dial with six stops plus off (540, 620, 710, 880, 1010 and 1270 kc).
  - The seed shuffles them: one numbers station, one tale station, one joropo station, three static.
  - A tag hanging on the padlock at the windmill gives tonight's numbers frequency.
- **The numbers station.** It repeats the code forever, in a cycle of at most 25 s:
  - an ident
  - each digit d as d short pips (a zero is one long tone)
  - 2 s between digits, and a 5 s rest.
- **Static.** Lightning can swallow one digit in a cycle, but never the same digit two cycles running, so any two cycles give every digit.
- **Relay.** The radio and the key box are 38 m apart. In co-op one player counts pips aloud while another turns the dials; solo, you walk between them.
- **Shared.** Anyone can turn the dial, it changes for everyone, and it squeals (an 8 m noise).
- **Captions.** A dots caption shows within reach of the radio whatever the captions setting, as the accessibility path.
- **Boundary.** The radio never reacts to where he is. Static near him would be a new truthful cue, like A8.
- **Build.**
  - Pure `radio.rs`: `stations`, `numbers_hour`, `beats_between`, `dots`, `dropped`. Add it to the fingerprint.
  - `Progress::radio` and `Event::RadioTuned` (appended).
  - `District::radio`. The page-10 note site moves there; the page id stays so saved Journals keep it.
  - `TargetKind::Radio` through `Action::Interact`; `WorldView.radio`.
  - Pips are timed from `snapshot.elapsed`, so every client hears the same beat.
  - Pages 3, 4 and 16 are rewritten to point at the radio, and `lore::fill` is deleted, which closes the Journal leak for good.
  - The driver gets a `Want::Radio`.
- **Tests.**
  - One numbers station per night, and it moves between seeds.
  - The pips reproduce `lock_code` for 200 seeds.
  - Each beat is heard exactly once at any frame rate.
  - Two consecutive cycles always carry every digit.
  - The dial is shared, its squeal draws his focus, and a restart turns it off.
- **Decided:**
  - A continuous cycle, not airings every 3 min: the 38 m relay is the slowdown, and scheduled waits are dead time solo.
  - Pips, not voiced digits: voice needs TTS or recordings, which is the user's call under AGENTS.md.

**The tale.**

1. **La Madrina, desde las raíces** (S, [a], M1b, with the radio).
   - Each bundle laid tells the next chapter in its own strip, in Spanish and English:
     1. the deer's entrails
     2. the father killed
     3. the grandfather's post, whip, ají and dog
     4. the curse, the sack and the whistle
     5. Santa Rosa's storm night
     6. when all bones are home: "name him rightly".
   - The count is shared, so everyone reads the same chapter at the same moment.
   - Its last line hints at his new trick, which merges the rage caption into it.
   - The strip is separate from `Hint`, so the Hijo's weeping tell is never pushed out.
   - Chapters are kept in the Journal (`profile.chapters`, serde default).
2. **The tale station** (M, [a], M2). The radio's tale stop plays the legend as a serial, captioned, one chapter per rage stage, continued across nights. Standing through a full chapter adds page 10 to the Journal.
3. **Cuatro versiones** (M, [a], M3). The pages that carry evidence about tonight's version of him read differently per survivor:
   - La Coplera: a copla
   - El Encargado: ledger sums
   - El Llanero: brands and tracks
   - El Muchacho: what his abuela told him

   Each version holds a different sign. Any two narrow it down, and all four settle it. With fewer players the absent versions are dealt to those present; solo holds all four. Pure `lore::version` and `deal_versions`; client-local.
4. **The count's rhythm** (La Cuenta) adds naming evidence in M2. Naming him stays a one-in-three choice at the ceiba for now.
5. The twenty pages stay as optional collectibles. "Pages found n/20" leaves the in-play panel, and "Keeper of the tale" remains a between-nights reward.

### 5. Build order

**Decisions for the user** (the default in brackets):

- the dawn floor, a new outcome [yes, at 30 min]
- `anima_sight`, whether the dead see him through the friend they watch [on: the playtest asked to watch teammates, and they see only that friend's screen]
- `bleed_after_sack` [30 s]
- the velorio revive [not yet]
- an opt-in microphone [no]
- A8, A9, B10 and B11 [unchanged]
- voiced digits or narration [no]
- fullscreen by default on first launch [yes]

**M1a, fingerprint-neutral** (ships as `0.1.0-test.2` and still pairs with `test.1`).

Writer A owns `app.rs`, `audio.rs`, `ui/menu.rs`, `profile.rs`, `player.rs`, `display.rs`, `ui/calibrate.rs`, `ui/title.rs`, `trailer.rs` and `debug.rs`. Writer B owns `tools/gen_audio.py`, `tools/whistle_lab.py`, the whistle WAVs and `assets/SOURCES.md`, then the caption styles in `ui/hud.rs` and `ui/mod.rs`. B's volume-curve change to the lab waits for A's constants in `app.rs`.

| # | Item | Cost | Now? |
|---|---|---|---|
| 1 | Audio mix, buses and glide (A) | M | Yes; the user checks levels by ear |
| 2 | Whistle distance signatures and captions (B) | M | Yes; gated by `measure --check`; the user listens after 1 |
| 3 | Journal leak: `menu.rs:404` shows "·" for digits (A) | S | Yes |
| 4 | Fullscreen (A) | S | Yes; the user checks on Windows |
| 5 | Brightness, contrast and calibration (A) | M | Yes; the user checks the three hats |

**M1b, one fingerprint batch** (ships as `test.3`).

`net/session.rs`, `protocol.rs`, `tuning.rs`, `sim.rs`, `ui/hud.rs` and `audio.rs` are shared, so a single rules writer takes items 6–11 in order. A parallel art writer may own only disjoint files: `world/avatar.rs` (ground torch), `world/wet.rs` and `world/flora.rs` (river visuals), and new clips in `gen_audio.py` once B has finished. The integrator runs the gate and the sweeps after each stable step.

| # | Item | Cost | Now? |
|---|---|---|---|
| 6 | Skill-check miss | M | Yes |
| 7 | River, Phase 1 | M | Yes; full sweep |
| 8 | Downed allies Tier 1, the help call, `bleed_after_sack` | M | Yes; the bleed default needs confirming |
| 9 | Spectating Phase 1 and the `die()` fix | M–L | Yes; the `anima_sight` default is a decision |
| 10 | El Respiro, La Rabia Tier 1, the retune, grass and lure levers, the moved tests | L | Yes; sweep at three nights; the dawn floor waits for the user |
| 11 | La Voz del Llano and the Madrina's chapters | L | Yes, last; if time runs short it opens M2 |

After `test.3`, time a real two-player night from the `Stats` log, and tune the grace, the Relax length and `carry_max` from that number.

**M2, the first comedy wave.**

| Item | Cost | Now? |
|---|---|---|
| La Cuenta | M | After the `test.3` timing |
| Gritos y silbos, the rest of the wheel | M | After the whistle bands are confirmed by ear |
| El Compadre and the house (A5); the borrowed call after the wheel | S–M | Yes if M1 lands |
| Ánimas haunts | M | After spectating is played |
| El desmayo | M | After M1 |
| El desamparado | S | Yes if M1 lands |
| Mandados and the copla | M | Yes if M1 lands |
| El paso del caño | M | After the river is played |
| La Rabia Tier 2 levers, one per sweep | M | After M1 |
| The tale station | M | After the radio |

**M3.** La repisa, A cuestas y en hombros, La Vigía, Contener el aliento, Agüeros, La Kodak, El mandador, Cuatro versiones. All wait on the M2 playtest; La Vigía is small enough to slot in earlier.

**M4, bigger bets and decisions.**

- **El sombrero / velorio**: carry a taken friend's hat to the ceiba to raise them. Reverses "taken means dead".
- **El Novenario**: nine nights, a friend's own bones among the next night's bundles, a hidden omen card, entierros on Thursday nights.
- **Él te escucha**: an opt-in, loudness-only microphone. Sits next to B11 and needs a new `cpal` dependency.
- **El lazo**: a rope tether at the caño, for two players first.
- B10 (accomplice) and B11 (proximity voice).

### 6. Rejected ideas

- **Perception boundary.**
  - A drinker who hears the whistle un-inverted.
  - El Elegido (only one player hears the whistle).
  - A count placed at his true position.
  - Radio static, or a frog chorus, that reacts to his distance (A8 in disguise).
  - A phase-keyed "the llano sings" event.
  - A lamp-bulb dispatcher board (it would share one player's private omen lies, or leak true lamp deaths).
  - Ánimas that make Tureco bark or whine.
- **Voice and microphone.**
  - Speech recognition (Vosk: saying "Silbón", naming him aloud).
  - Cloning friends' voices, as in Skinwalkers or MIMESIS.
  - Voiced digits and a live narrator (generated-audio rule).
- **Two players and hidden roles.**
  - A permanent traitor (el tocado, B10's accomplice).
  - The secretly drunk rascao.
  - El Caporal (one player holds the only ledger).
- **Scope.**
  - La Trocha, a driven escape.
  - Dudo, liar's dice at the kitchen table.
  - The contrapunteo duel.
  - Obra Dinn-style memory dioramas.
  - The Libro de Hierros brand ledger.
  - Possessing a cow.
  - Arranque en frío, a three-station truck start (the finale should be a sprint).
  - The rabipelado thief.
  - Inventory slots with a porch chest.
- **Pacing.**
  - A 45–55 min night clock.
  - A longer truck warm-up.
  - El fogón and the porch rest (a player-made safe zone; the director already guarantees release).
  - Tall-tale voting at the fire.
  - Scheduled radio airings.
  - Unlocks triggered by deaths.
  - Player whistles that raise his anger.

### 7. Sources

Some wiki pages (Phasmophobia, Outlast, Dead by Daylight, Left 4 Dead) and the Demonologist threads were read only as search snippets.

**Co-op party and friendslop games**
- https://lethal-company.fandom.com/wiki/Walkie-Talkie
- https://lethal-company.fandom.com/wiki/Guide:Camera_duty
- https://lethal-company.fandom.com/wiki/Signal_translator
- https://lethal-company.fandom.com/wiki/Masked
- https://lethal-company.fandom.com/wiki/Hoarding_Bug
- https://lethal-company.fandom.com/wiki/Time
- https://lethal.miraheze.org/wiki/Spectator
- https://lethal.miraheze.org/wiki/Masks_(Enemy)
- https://lethal.miraheze.org/wiki/Signal_Translator
- https://lethal.miraheze.org/wiki/Walkie-talkie
- https://primagames.com/tips/how-to-play-the-ship-duty-role-in-lethal-company
- https://steamcommunity.com/sharedfiles/filedetails/?id=3143870501
- https://www.thegamer.com/lethal-company-workplace-horror-comedy-politics-social-impact-quota/
- https://www.pushtotalk.gg/p/how-lethal-company-sold-10-million-copies
- https://www.pcgamer.com/lethal-companys-co-op-horror-comedy-is-like-phasmophobia-but-immediately-more-chaotic-and-deadly/
- https://www.destructoid.com/lethal-companys-skinwalkers-mod-mimics-your-friends-voices/
- https://win.gg/news/lethal-company-skinwalkers-mod-is-spooky-and-hilarious/
- https://thunderstore.io/c/lethal-company/p/RugbugRedfern/Skinwalkers/
- https://thunderstore.io/c/lethal-company/p/qwbarch/Mirage/
- https://thunderstore.io/c/lethal-company/p/Quixler/DeadAndBored/
- https://kotaku.com/content-warning-impressions-1851407637
- https://content-warning.fandom.com/wiki/Sp%C3%B6%C3%B6ktube
- https://contentwarning.wiki.gg/wiki/Camera
- https://www.pcgamer.com/games/horror/content-warning-is-giving-you-a-chance-to-actually-go-viral-by-sending-your-best-clips-to-the-lost-footage-project/
- https://en.wikipedia.org/wiki/Content_Warning
- https://www.thegamer.com/repo-how-to-revive-teammates/
- https://www.gamesradar.com/games/horror/repo-revive/
- https://store.steampowered.com/app/3241660/REPO/
- https://steamcommunity.com/app/3241660/discussions/0/689743495770948023/
- https://hardcoregamer.com/repo-looting-guide/
- https://gamerant.com/repo-mistakes-everyone-makes-how-to-avoid/
- https://dotesports.com/indies/news/r-e-p-o-valuables-list-price-and-all-effects
- https://www.gamedeveloper.com/business/peak-co-developer-aggro-crab-shares-lessons-in-friendslop
- https://80.lv/articles/climbing-sim-peak-was-meant-to-be-friendslop-game-from-the-start
- https://en.wikipedia.org/wiki/Peak_(video_game)
- https://peak.wiki.gg/wiki/How_to_play
- https://peak.wiki.gg/wiki/Campfire
- https://peak.wiki.gg/wiki/Shroomberry
- https://peak.wiki.gg/wiki/Scoutmaster_Myres
- https://www.pcgamesn.com/peak/scoutmaster
- https://gamerant.com/peak-revive-mechanics-how-to-revive-downed-koed-dead-teammates/
- https://gamerant.com/peak-which-berries-mushrooms-safe-differences-poisoned-safe-foods/
- https://deltiasgaming.com/peak-how-to-identify-poisonous-foods-and-mushrooms/
- https://steamcommunity.com/app/3527290/discussions/0/592900638661148880/
- https://www.thegamer.com/proximity-chat-is-incredible-peak-repo-lethal-company-phasmophobia-among-us/
- https://appmagic.rocks/blog/friendslop-steam-games-2025
- https://en.wikipedia.org/wiki/Friendslop
- https://en.wikipedia.org/wiki/Chained_Together
- https://indiegame.com/en/archives/28817
- https://store.steampowered.com/app/2522520/Only_Up_With_Friends/
- https://www.shacknews.com/article/135585/bread-and-fred-review-score
- https://www.thegamer.com/human-fall-flat-interview-tomas-sakalauskas/
- https://steamcommunity.com/app/285900/discussions/0/343785574531948394/
- https://gamerant.com/grounded-guide-defeat-spiders/
- https://medium.com/@calvinflowers7/mage-arena-review-real-voice-spells-real-chaos-and-real-fun-f8be8757269a
- https://store.steampowered.com/app/3716600/Mage_Arena/
- https://steamcommunity.com/app/3716600/discussions/0/595155668457012140/
- https://store.steampowered.com/app/3097560/Liars_Bar/
- https://www.thegamer.com/liars-bar-liars-dice-explained-guide/
- https://thegameofnerds.com/2024/10/08/liars-bar-review-a-hilarious-russian-roulette-tabletop-game/
- https://gameworldobserver.com/2024/10/21/liars-bar-100k-ccu-steam-curve-animation-turkey-success
- https://www.thegamer.com/buckshot-roulette-items-explained-guide/
- https://noisypixel.net/buckshot-roulette-review-perfect-for-streamers-2024/
- https://80.lv/articles/buckshot-roulette-developer-on-making-the-game-solo-feedback-success
- https://criticalvideogamestudies.com/buckshot-roulette-and-liars-bar-a-mix-of-casual-fun-uncertainties-and-psychological-warfare/

**Horror design and pacing**
- https://www.gamedeveloper.com/design/the-perfect-organism-the-ai-of-alien-isolation
- https://www.gamedeveloper.com/design/revisiting-the-ai-of-alien-isolation
- https://steamcdn-a.akamaihd.net/apps/valve/2009/ai_systems_of_l4d_mike_booth.pdf
- https://left4dead.fandom.com/wiki/The_Director
- https://en.wikipedia.org/wiki/Mike_Booth
- https://www.aiandgames.com/p/how-the-beast-works-in-amnesia-the
- https://www.dualshockers.com/amnesia-the-bunker-terrifying-generator-mechanic/
- https://kineticgames.co.uk/news/phasmophobia-voice-recognition-update
- https://phasmophobia.fandom.com/wiki/Voice_chat
- https://phasmophobia.fandom.com/wiki/Spirit_Box
- https://phasmophobia.fandom.com/wiki/Cursed_Possession
- https://phasmophobia.fandom.com/wiki/Ouija_Board
- https://phasmophobia.fandom.com/wiki/Setup_Phase
- https://phasmophobia.fandom.com/wiki/Challenge_Mode
- https://phasmophobia.fandom.com/wiki/Difficulty
- https://screenrant.com/phasmophobia-ghost-voice-recognition-phrases-talking-microphone-chat/
- https://www.exitlag.com/blog/monkey-paw-wishes-in-phasmophobia/
- https://www.gamespot.com/articles/all-phasmophobia-tarot-cards-and-effects/
- https://steamcommunity.com/app/1929610/discussions/0/6169410450154846224/
- https://demonologist.fandom.com/wiki/Spirit_Box
- https://sirusgaming.com/demonologist-voice-commands/
- https://en.wikipedia.org/wiki/Devour_(video_game)
- https://www.devourgame.com/
- https://store.steampowered.com/app/967050/Pacify/
- https://outlast.fandom.com/wiki/Escalation
- https://outlast.fandom.com/wiki/Variators
- https://store.steampowered.com/app/1850740/Ghost_Watchers/
- https://deadbydaylight.wiki.gg/wiki/Hooks
- https://deadbydaylight.wiki.gg/wiki/Totems
- https://www.dexerto.com/gaming/new-horror-game-uses-ai-to-copy-you-and-your-teammates-movements-and-voices-3200757/
- https://store.steampowered.com/app/2827200/MIMESIS/
- https://game8.co/articles/reviews/mimesis-review-early-access
- https://virus.hr/en/reviews/mimesis-early-access
- https://boingboing.net/2025/05/29/new-horror-game-lets-ai-clone-your-voice-to-terrorize-friends.html
- https://en.wikipedia.org/wiki/Don%27t_Scream
- https://www.pcgamer.com/if-you-scream-while-playing-dont-scream-you-have-to-restart-the-game/
- https://dredge.wiki.gg/wiki/Panic
- https://www.thegamer.com/pacific-drive-developers-interview/
- https://www.dreadcentral.com/editorials/493266/signalis-sows-terror-through-clever-save-point-design/
- https://www.gametruth.com/editorials/limited-inventory-in-signalis-is-paradoxical-yet-intentional/
- https://gamerant.com/indie-horror-game-iron-lung-scary-limitations-player-fov-perspective/
- https://www.thejimquisition.com/post/mouthwashing-hurts-so-good-review
- https://sonsoftheforest.fandom.com/wiki/Cannibals
- https://sonsoftheforest.fandom.com/wiki/Kelvin
- https://www.gamepressure.com/newsroom/golden-boy-kelvin-from-sons-of-the-forest-loved-by-players/z35176
- https://www.relyonhorror.com/reviews/review-yuppie-psycho/

**Communication, deduction and puzzles**
- https://media.gdcvault.com/gdc2016/Presentations/Kane_Ben_Designing_Asymmetric_Gameplay.pdf
- https://gdcvault.com/play/1023471/Designing-Asymmetric-Gameplay-For-Keep
- https://www.gamedeveloper.com/design/finding-the-fun-in-bomb-defusal-with-i-keep-talking-and-nobody-explodes-i-
- https://www.gamedeveloper.com/design/road-to-the-igf-steel-crate-games-i-keep-talking-and-nobody-explodes-i-
- https://www.bombmanual.com/print/KeepTalkingAndNobodyExplodes-BombDefusalManual-v1.pdf
- https://ktane.fandom.com/wiki/Who's_on_First
- https://blog.playstation.com/2020/06/15/co-op-spy-thriller-operation-tango-explores-innovations-in-asymmetrical-co-op/
- https://www.gamepressure.com/editorials/reviews/operation-tango-review-a-stylish-spy-co-op/zb41f
- https://news.xbox.com/en-us/2023/01/31/co-op-puzzle-design-for-we-were-here-forever/
- https://www.gamespew.com/2017/03/communication-key-puzzle-game/
- https://news.xbox.com/en-us/2019/10/04/me-you-walkie-talkies-and-a-whole-lot-of-puzzles-in-we-were-here-too/
- https://en.wikipedia.org/wiki/We_Were_Here_(series)
- https://mcvuk.com/development-news/when-we-made-sea-of-thieves/
- https://www.seaofthieves.com/community/forums/topic/68049/what-is-your-ship-role
- https://www.gamedeveloper.com/design/road-to-the-igf-ghost-town-games-i-overcooked-i-
- https://theescaperoomer.com/pine-studio-tomislav-interview/
- https://www.co-optimus.com/interview/2510/page/1/unrailed-2-back-on-track-developer-interview.html
- https://wiki.bloodontheclocktower.com/Drunk
- https://en.wikipedia.org/wiki/Blood_on_the_Clocktower
- https://draughtslondon.com/mastering-blood-on-the-clocktower/
- https://www.meeplemountain.com/reviews/blood-on-the-clocktower/
- https://steamcommunity.com/sharedfiles/filedetails?id=1174654924
- https://steamcommunity.com/sharedfiles/filedetails/?id=3563090520
- https://steamcommunity.com/sharedfiles/filedetails/?id=2950696718
- https://en.wikipedia.org/wiki/Dale_&_Dawson_Stationery_Supplies
- https://steamcommunity.com/sharedfiles/filedetails/?id=3321911072
- https://www.vice.com/en_ca/article/kzmjkn/project-winter-is-the-best-game-about-betrayal-this-year
- https://projectwinter.co/about.html
- https://town-of-salem.fandom.com/wiki/Medium_(ToS)
- https://town-of-salem.fandom.com/wiki/Last_Will
- https://town-of-salem.fandom.com/wiki/Forger
- https://mechanicsofmagic.com/2024/04/09/among-us-diego-valdez-duran/
- https://among-us.fandom.com/wiki/Submit_Scan
- https://among-us.fandom.com/wiki/Ghost
- https://dread-hunger.fandom.com/wiki/Beginner_Thrall_Guide_by_Raven
- https://www.youtube.com/watch?v=NZ3oyCEKZIw
- https://boardgamegeek.com/thread/1795201/secret-and-betrayer-objectives
- https://therewillbe.games/articles-boardgame-reviews/4805-dead-of-winter-review
- https://en.wikipedia.org/wiki/Betrayal_at_House_on_the_Hill
- https://mechanicsofmagic.com/2022/04/15/critical-play-betrayal-at-house-on-the-hill/
- https://en.wikipedia.org/wiki/Mysterium_(board_game)

**Story, radio and narration**
- https://oxenfree.fandom.com/wiki/Radio
- https://oxenfree-archive.fandom.com/wiki/Anomalies
- https://www.gamedeveloper.com/design/road-to-the-igf-night-school-studio-s-i-oxenfree-i-
- https://www.gamedesigngazette.com/2018/01/the-immersive-greatness-of-oxenfrees.html
- https://kentucky-route-zero.fandom.com/wiki/WEVP-TV
- https://kentucky-route-zero.fandom.com/wiki/Un_Pueblo_de_Nada
- https://tvtropes.org/pmwiki/pmwiki.php/Main/NumbersStations
- https://signalis.wiki.gg/wiki/Magpie_Box_Puzzle
- https://en.wikipedia.org/wiki/Stories_Untold_(video_game)
- https://www.pcgamer.com/play-a-radio-host-guiding-callers-to-safety-in-horror-adventure-killer-frequency/
- https://en.wikipedia.org/wiki/The_War_of_the_Worlds_(1938_radio_drama)
- https://www.eluniversal.com.mx/opinion/mochilazo-en-el-tiempo/el-terror-tambien-entra-por-los-oidos-radiodramas-de-suspenso-y-miedo/
- https://es.wikipedia.org/wiki/El_siniestro_Doctor_Mortis
- https://headstuff.org/entertainment/gaming/sagebrush-is-a-dark-and-bold-dive-into-survivors-guilt/
- https://filmstories.co.uk/features/exploring-return-of-the-obra-dinns-rule-of-three/
- https://en.wikipedia.org/wiki/Return_of_the_Obra_Dinn
- https://adventuregamehotspot.com/review/2529/nobody-wants-to-die
- https://www.pcgamesn.com/nobody-wants-to-die/review
- https://en.wikipedia.org/wiki/Mouthwashing_(video_game)
- https://store.steampowered.com/app/2506450/Guayota/
- https://waytoomany.games/2024/10/06/review-guayota/
- https://store.steampowered.com/app/1411900/Mictlan_An_Ancient_Mythical_Tale/
- https://store.steampowered.com/app/854570/Pamali_Indonesian_Folklore_Horror/
- https://jrfm.eu/index.php/ojs_jrfm/article/view/218
- https://www.gamedeveloper.com/design/pentiment-director-explains-how-going-all-in-on-fonts-helped-elevate-the-medieval-detective-rpg-
- https://www.pcgamer.com/how-pentiments-hand-crafted-fonts-give-pen-and-ink-a-voice/
- https://www.gamedeveloper.com/design/interview-storytelling-through-narration-in-i-bastion-i-
- https://darkestdungeon.fandom.com/wiki/Narrator_(Darkest_Dungeon)
- http://press.konagame.com/
- https://godisageek.com/reviews/candle-the-power-of-the-flame-review/
- https://www.vice.com/en/article/wildermyth-review/
- https://en.wikipedia.org/wiki/Inscryption
- https://www.gamedeveloper.com/design/video-the-dialog-systems-and-tools-of-i-firewatch-i-
- https://venturebeat.com/games/crafting-relationships-through-radio-in-firewatch/
- https://gamesbeat.com/crafting-relationships-through-radio-in-firewatch/
- https://www.gamesradar.com/making-of-monkey-island-insult-sword-fighting/

**Folklore and culture**
- https://en.wikipedia.org/wiki/El_Silb%C3%B3n
- https://www.culturagenial.com/es/el-silbon/
- https://www.pacarinadelsur.com/mitos-leyendas/el-silbon/
- https://www.todacolombia.com/folclor-colombia/mitos-y-leyendas/silbon.html
- https://steemit.com/spanish/@relatos/la-leyenda-del-silbon
- https://www.musicallanera.net/leyendas-llaneras/la-leyenda-del-silbon/
- https://www.welovevillavo.com/post/el-silb%C3%B3n-mitos-y-leyendas-del-llano-1
- https://wildhunt.org/2019/11/columna-el-silbon-una-leyenda-venezolana-sobre-los-ancestros.html
- https://wildhunt.org/2019/11/column-el-silbon-a-venezuelan-legend-about-the-ancestors.html
- https://www.scaryforkids.com/el-silbon/
- https://lanoticia.com/primerafila/entretenimiento/el-silbon-la-atemorizante-leyenda-de-venezuela-y-colombia/
- https://www.eldestapeweb.com/atr/virales/por-que-la-tradicion-no-aconseja-silbar-de-noche-y-que-puede-pasar--2024121317313
- https://notiapure.com.ve/cultura/entierros-de-morocotas-en-apure-leyendas-de-tesoros-y-espantos/
- https://steemit.com/spanish/@juan170/leyenda-los-entierros
- https://www.redalyc.org/pdf/712/71206804.pdf
- https://es.wikipedia.org/wiki/Chim%C3%B3_llanero
- https://www.radionacional.co/cultura/tradiciones/el-chimo-costumbre-y-tradicion-de-la-cultura-llanera
- https://es.wikipedia.org/wiki/Didelphis_marsupialis
- https://ultimasnoticias.com.ve/noticias/tu-mascota/mundo-animal/mira-hay-un-rabipelao-en-el-arbol/
- https://es.wikipedia.org/wiki/1_Broadcasting_Caracas
- https://www.eltiempo.com/archivo/documento/MAM-206324
- https://en.wikipedia.org/wiki/Radio_Sutatenza
- https://www.insai.gob.ve/hierros
- https://ich.unesco.org/en/USL/colombian-venezuelan-llano-work-songs-01285
- https://www.cancilleria.gov.co/newsroom/news/cantos-trabajo-llano-colombo-venezolanos-fueron-reconocidos-unesco-patrimonio
- https://www.radionacional.co/cultura/contrapunteo-llanero-la-tradicion-musical-de-la-orinoquia
- https://es.wikipedia.org/wiki/Florentino_y_El_Diablo
- https://babel.banrepcultural.org/digital/collection/p17054coll10/id/2797
- https://en.wikipedia.org/wiki/Susto
- https://es.wikipedia.org/wiki/Truco_venezolano
- https://www.diocesisdecordoba.es/carta-semanal-obispo/ave-maria-purisima-sin-pecado-concebida
- https://forum.wordreference.com/threads/ave-maria-pur%C3%ADsima-sin-pecado-concebida.1321371/
- https://www.spanishdict.com/answers/171100/beh-de-burro-o-veh-de-vaca
- https://lotoven.com/animalitos/
- https://dialnet.unirioja.es/descarga/articulo/5043782.pdf

**Tools**
- https://alphacephei.com/vosk/models
- https://github.com/alphacep/vosk-api

## Earlier handoff — 2026-09-29 (first remote playtest notes)

Notes only; nothing below is implemented or triaged yet. From the first
two-player test of the Windows build (`0.1.0-test.1`) over Tailscale.

### Bugs

- **River**: players cannot walk through the river.
- **Skill checks**: a missed skill check did not slow task progress.
- **Audio level**: the overall mix is too loud, and the in-game volume
  slider does not control it properly.
- **Whistle distance**: the whistle sounded the same far and near. By
  design it is inverted (faint when he is close, loud when far), so check
  whether the three distances are distinguishable at all with the
  recorded whistle, not just whether the inversion reads.

### Missing features

- **Fullscreen**: no fullscreen mode.
- **Spectating**: a dead player has nothing to watch; add a spectator view
  of the surviving teammates.
- **Finding downed allies**: hard to find a downed teammate; they need a
  clearer marker or sound.
- **Brightness and contrast**: on some monitors El Silbón is hard to see;
  add brightness and contrast sliders to the settings.

### Design ideas

- **Rising difficulty**: get harder as more bones are laid to rest.
- **Telling the tale**: find a better way to tell the legend in game than
  the twenty pages.
- **The truck key code**: its code is spread over three of the twenty
  pages, so players end up hunting for all twenty. Put the code in a few
  dedicated notes instead, or make it a puzzle, such as one told through
  the radio broadcasts.

## Earlier handoff — 2026-09-29 (the whistle from a recording)

Uncommitted. The user asked for the whistle of a YouTube clip ("El Silbon
Silbido", oNGPZNXmZ1c) in place of ours, after being told its licence is
unknown and it is not original work; they chose to use it. It is kept as
`assets/audio/source/el_silbon_silbido.wav` (44.1 kHz mono, hum and hiss
removed) and recorded in `assets/SOURCES.md` as third-party, **not cleared
for release**. `tools/gen_audio.py` builds the twelve whistle files from it
(two performances, and each played at 0.93 and 1.05 speed for four takes)
through the same close / across / far processing, with the filters raised
for its higher register (1.2 to 2.35 kHz); without the file it falls back
to the synthesized whistle below. The catch's whistle in your ear
(`whistle_ear.wav`) is still synthesized. Unverified: the user's ear
(`python3 tools/whistle_lab.py`; renders in `~/Music/el_silbon_whistles/`).

## Earlier handoff — 2026-09-29 (a spookier whistle)

Uncommitted. The user found the game's whistle funny and the trailer's
opening one (the game's own faint take) spooky. The close and middling
takes were a bright, brisk, clean major run (with overdrive on the close
one): a tune. `tools/gen_audio.py` now performs the same seven rising
steps in a dark mode, slow and legato (each note slid and scooped into),
a few cents off true, with a slow, wide waver; the tone is a pure
fundamental plus noise through a narrow resonance that follows the pitch
(`pitched_air`), so it breathes like a real whistle; the close take lost
its overdrive and gained a small dark room. The three distances and the
inversion are unchanged; only the 12 whistle files changed. Phrases run
3.6–5.0 s at the close distance (3.4 s before).

Four ElevenLabs sound-effect whistles (flow `t3VPUz9ndVLhNvptnZKb`) were
made for comparison only; none plays the tale's seven rising steps, and
game audio stays generated in code unless the user chooses otherwise.
Unverified: the user has not listened yet (`python3 tools/whistle_lab.py`,
`ab`, `approach`, `night`).

## Earlier handoff — 2026-09-29 (the trailer, redone)

The teaser was re-rendered with the v2 models (`--trailer`, 2094 frames)
and re-cut with the same narration, score and sound: 89.4 s, -13.5 LUFS,
peak -1.3 dB, the catch's 0.45 s silence still silent. New shot
`16_party` (after the "UP TO FOUR FRIENDS" card): all four survivors walk
out of the dark toward the camera under the gate lamps, torches on.
Output: `~/Videos/el_silbon_trailer.mp4` (and a 720p preview); the first
cut is kept as `el_silbon_trailer_v1.mp4`. Unverified: the user has not
watched it yet.

## Earlier handoff — 2026-09-29 (models v2 and their animation)

Uncommitted, on top of the survivors pass below. The user asked for every
model to look like its concept sheet and to be animated well, working in
Blender through the MCP.

### What changed

- **Survivors v2** (`tools/models/survivors.py`, rewritten): sculpted heads
  (brow, sockets, cheekbones, jaw, nose, ears), larger per the sheet;
  sloped shoulders, fuller limbs, hands with four fingers and a thumb (the
  right one a fist round the torch); turned-down collars, pockets, belts,
  boots, sneakers with laces, a ruffled blouse and a gathered skirt with a
  ruffled hem, combed hair. One baked 2048 texture per survivor: per-facet
  colour times numpy patterns chosen per face (`pat` attribute: flowers,
  plaid, denim, straw, linen weave, jute, canvas), a painted face decal
  (eyes, lids, brows, nostrils, lips, age lines) and contact shadow (AO);
  normals are recalculated outward so the AO only sees real occluders.
  17-joint rig: Pelvis, Torso, Chest, Neck, Head, Hip/Knee/Foot and
  Shoulder/Elbow/Hand L/R.
- **Teammate animation** (`world::avatar`): a pure `pose(bone, &Gait)`
  gives the walk (stride sized to speed: pelvis bob and turn, knees folding
  through the swing, feet kept level, counter-swinging arms, head steadied),
  the sprint (lean, bent elbows, high heel kick), the crouch (pelvis down
  0.3 m, hips and knees folded, lean) and the crawl when down (arms reaching
  ahead in turn, head up), breathing and idle glances; the torch arm keeps
  the beam where they look. The beam and the carried bundle ride on the
  hand and chest joints once the model is in (`Rigged`); the stand-in keeps
  the old squash. Tests: forward kinematics keeps feet on the ground
  standing, crouching, walking and sprinting, the leading foot is ahead and
  its arm swings back; the beam follows the look while sprinting; a
  downed survivor reaches past the head.
- **El Silbón** (`tools/silbon_model.py`): a woven llanero sombrero after
  the sheet — flat-topped, slightly tapered crown, a drooping cone of a
  brim, a frayed fringe of hanging straws; darker straw and rags. Face,
  eyes and joints untouched (the user's stare is kept; the catch frames
  were re-shot).
- **Tureco** (`tools/models/dog.py`): fuller body and neck, thicker legs
  and paws, bigger amber eyes with pupils, taller ears, a brush of a tail,
  a deeper coat; new wrist/hock joints (`PawFL/FR/BL/BR`) that `dog.rs`
  folds through the trot (`DogPaw`), and folds the hocks when he sits tied.
- **The herd** (`tools/models/cattle.py`): heavier bodies and legs with
  knobbly knees, a broader head, a big dark muzzle with nostrils, droopy
  ears in coat colour (pinkish inside); a rig per variant (Neck, Head,
  EarL/R, Tail). `herd.rs` `cow_pose`: each beast grazes for most of a
  minute-long cycle, lifts its head to look round, chews, flicks its ears
  and swishes its tail, out of step with the others; spooked, heads come up
  and tails lash.
- **The truck** was already closest to its sheet and is unchanged.
- Trailer driver: review-only shots `90_survivors_walk` (sprint, crouch,
  carrying, walk, side on) and `91_survivors_down` (a crawl, a crouch
  beside it); not in the cut (`TRAILER_ONLY=90`, `TRAILER_ONLY=91`; a
  full `--trailer` render now includes them, and `edit.py` ignores them).
- `tools/models/review.py`: review renders of any exported `.glb` (the
  Silbón's sheet).

### Verified

- Gate: fmt, clippy `-D warnings`, 65 unit + 7 district + 41 session tests.
- Cycles sheets (`assets/models/src/renders/`): survivors, Tureco, herd.
- In game (release): `63_model_survivors`, `55_tureco`, `62_model_herd`
  (a cow grazing), the catch frames 53/54/58/59 (his stare as before, under
  the new hat), and the trailer driver's walk/crawl shots (stride, sprint
  kick, crouched creep, crawl reading correctly).
- Rendered solo `--smoke`: `SMOKE PASS` (589.5 s simulated, two restarts,
  census unchanged). Rendered two-process UDP `--net-smoke`: host and client
  `NET SMOKE PASS` and `NET RENDER SMOKE PASS` (census kept through run 3).

### Unverified / next

- Hands-on: a teammate walking, sprinting, crouching, going down and
  aiming the torch, seen live in two windows; the herd's cycle over a
  minute; Tureco trotting.
- The survivor `.glb`s are 9–12 MB each (PNG textures, flat-shaded split
  vertices); JPEG colour or 1024 textures would shrink them if needed.

## Earlier handoff — 2026-09-29 (the four survivors)

Uncommitted. The user sent a "Survivors" concept sheet (El Llanero, La
Coplera, El Encargado, El Muchacho) and asked for playable-character models
built in Blender through the MCP, and a way to choose them with friends.

### The models (`tools/models/survivors.py`)

One shared low-poly body dressed four ways in the sheet's faceted style
(flat per-facet colours, like Tureco): the llanero's cream liquiliqui with
stand collar, pockets and buttons and a black flat-brimmed hat; the
coplera's ruffled white blouse, flowered red skirt with a ruffled hem and a
low bun with a red ribbon; the encargado's khaki shirt with rolled sleeves,
belt, dark trousers in rubber boots, striped towel, grey hair, moustache
and straw hat; the muchacho's open plaid overshirt over a white tee, jeans,
sneakers and red bracelet. The sheet's alternative trousers for the coplera
were not made. Each holds a torch in the right hand where the game hangs the
beam, and is skinned to Body, Torso, Head, Shoulder/Elbow L/R and Hip/Knee
L/R (8–19k triangles). Iterated live through the Blender MCP in its own
scene (nothing saved over the open file); the final `.glb`s, `survivors.blend`,
the review sheet and the menu portraits come from a headless run:
`PATH=/usr/bin:$PATH blender -b --factory-startup --python tools/models/survivors.py -- --renders DIR --portraits assets/ui/survivors --blend assets/models/src/survivors.blend`.

### In the game

- `survivor` (pure): the four, their names and roles, and `assign` (your
  wish unless taken, else the first free). `net::session` keeps who each
  player is across restarts, gives out a free one on joining, and
  `choose_survivor` changes it only in the lobby; `PlayerView.survivor` and
  `Hello.survivor` carry it (`#[serde(default)]`), `Action::Become` changes it
  from the lobby. `Endpoint::with_survivor` sets the wish (solo/host at
  once, client in the handshake).
- Choosing: "Who you are ‹ … ›" on the **With friends** page with the chosen
  survivor's portrait beside the rail (saved as `Profile.survivor`);
  `--survivor NAME` on the command line; **F7** in the lobby steps to the
  next free one. The party list shows `P2 La Coplera (you)`.
- Teammates (`world::avatar`): the survivor model loads over the old
  procedural llanero (which stays if it cannot load); a changed survivor is
  respawned. `avatar::animate` walks them from their speed over the ground
  (hips, knees and arms, same signs as the Silbón's rig), leans into a
  sprint, breathes at rest, and turns the head and the torch arm, with its
  beam, to their look pitch. A teammate who leaves mid-load no longer keeps
  `ModelsPending` waiting.
- Photo `63_model_survivors`: the four in a line at the gate
  (`PHOTOS_ONLY=63`). Staged photo/trailer teammates are each someone else.

### Verified

- Gate green: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D
  warnings`, `cargo test --locked` (63 unit + 7 district + 41 session).
- New tests: a party of four wishing for the same one gets four different
  people, free wishes are granted, codes round-trip (`survivor`); in a
  session, a clash gives someone free, a leaver frees theirs, only the lobby
  can change it, and it survives a restart (`tests/session.rs`).
- Rendered `--photos` frame `63_model_survivors`: all four load and read
  clearly at night at the ranch gate.
- Two-process UDP `--net-smoke`, both processes `--survivor llanero`:
  headless, host and client `NET SMOKE PASS`; rendered, both `NET SMOKE PASS`
  and `NET RENDER SMOKE PASS` (census kept through run 3). The host stays El
  Llanero and the client is given La Coplera; both rosters show it, through
  both restarts. (A first rendered attempt failed when the host's scripted
  route was downed by him at 158 s mid-route; the identical rerun passed,
  so it reads as the real-time route's timing, not this change.)
- `--menu-shots` (`06_multiplayer`): the "Who you are" row and the
  portrait with name and role beside the rail.

### Unverified / next

- The walk in motion was only seen from behind in tall grass (the trailer's
  friends shot); a user look at a teammate walking, sprinting, crouching and
  aiming the torch is still wanted (two windows on one machine is enough:
  `--host 127.0.0.1:5000` and `--join 127.0.0.1:5000 --survivor coplera`).
- F7 in the lobby and ‹ › on the menu row by hand.
- The night's awards still name players as P1/P2; they could use the
  survivors' names.

## Earlier handoff — 2026-09-29 (models in the game, and the teaser)

### The Blender models (`130ebf7`, local, not pushed)

`world::models` puts each glTF from `assets/models/` (see
`assets/SOURCES.md`) under the entity it replaces: the Silbón, Tureco, the
herd (a bull, a calf and cows), the bone bundles (placed and carried), the
shelf radio and the truck. When a scene is ready the procedural stand-in
goes and the game's markers move onto the model's named nodes (his joints
and body, the dog's head, jaw, tail and legs, the truck's lamp materials),
so the existing animation drives the real rigs. A model that fails to load
(for example a clone without Git LFS) leaves the procedural one in place.
The drivers wait for `ModelsPending::settled()`. Verified: gate green;
photos of every model (`60_model_bundle`, `61_model_radio`,
`62_model_herd`, the catch frames); rendered `--smoke` PASS with a steady
census.

### The teaser (committed locally, not pushed)

- `--trailer` (`src/trailer.rs`) stages fifteen shots in the real game
  (camera paths, his poses, lightning on a pinned storm clock, staged
  teammates, the hat, the real catch) and captures 1920x1080 frames on a
  fixed 1/30 s clock. Everything staged is presentation only.
- `tools/trailer/edit.py` cuts the frames with cards, subtitles, grain,
  narration, score and sound (ffmpeg); the mix is normalised to about
  -14 LUFS, true peak -1.5 dB.
- Narration: ElevenLabs `eleven_v3`, premade voice "Callum" in English (the
  Spanish library voices need a paid tier); the Spanish refrain is on the
  title card. Score: four `eleven_music` cues; hits: `eleven_text_to_sound`.
  All on the ElevenLabs flow `t3VPUz9ndVLhNvptnZKb`. The narration was
  transcribed back and matches the script. Stems and frames stay out of git.
- Output: `~/Videos/el_silbon_trailer.mp4` (85.9 s, 1080p30, -13.3 LUFS) and a
  720p preview next to it.
- Verified: gate green (fmt, clippy `-D warnings`, 62 + 7 + 40 tests); every
  shot reviewed on contact sheets; the catch's silence measured silent.
  Unverified: the user has not yet watched it.

## Earlier handoff — 2026-09-29 (the catch, rebuilt)

Uncommitted, with the quick wins below (all on `c154ec4`). The user found
being caught underwhelming and buggy ("as if el silbon is inside the
ground") and asked for it fixed and made very scary.

### Why it looked buried

The lunge placed him relative to the camera: root = eye − 2.75 m. Caught,
the eye drops to 0.34 m, so his root stood about 2.4 m underground with
only the hat and face above the grass (visible in the old frame 53).

### The catch now (`silbon::lunge_frame`, pure; `omen::Catch`)

Frozen at the moment of the catch (where the eye stood and looked, and one
of three ways: ahead, left or right, which hand leads), so the view and he
never chase each other. One pure function gives his root, bend and joints,
the caught eye and what it looks at, the torch, the light on his face, the
black and the shake; the model, the camera, the torch, the face light and
the veil all read it.

1. **Silence** (0.45 s): every sound stops (except the catch's own), the
   world dims, the torch gutters out.
2. **His whistle right in your ear** (`whistle_ear.wav`, 0.62 s), in the
   dark: the one time it is loud because he is close (it cannot mislead
   now; presentation only, a Sting on your own status, never a cue).
3. **Strobe**: the torch bursts back four times, each flash showing him
   closer (5 m, 3 m, 1.7 m, on you), the view flaring and punching in,
   while you are knocked onto your back.
4. **Over you**: standing on the real ground, crouched and bent nearly flat
   over you, his face lit, twisted upright to stare straight into your
   view, head snapping side to side, claws closing.
5. **Black** at 1.75 s: the bones rattle, the ears ring (`ringing.wav`),
   the world murmurs back; the view is handed back under the black.

The HUD is hidden for the whole catch; a solo run's outcome waits for it to
end (`fright.lunge.is_none()`, not a copied constant). The old
standing-camera poses are gone; photo frames 53, 54, 58 and 59 now hold
the catch at four instants (`PHOTOS_ONLY=lunge`). The smoke route's
`08_caught` is retimed to land while he is over you, and it restarts only
after the black lifts.

### Verified

- New pure test: over the whole catch, on uneven ground, from every side,
  his feet are on the ground, his hips never in the earth, the eye above
  it, and when shown his face is 0.3–6 m away, turned to the eye and in
  the middle of the view.
- Gate green (62 unit + 7 district + 40 session tests); photos of the four
  instants reviewed (the user approved the stare as it is).
- Rendered `--smoke` (release, user-approved): `SMOKE PASS` (589.5 s
  simulated, two restarts, census steady: the veil is spawned once).
  `08_caught` is a real catch: the view on its back, he stands on the
  ground bent over it, his eyes lit under the brim, claws out, no HUD.

### Unverified (user-led)

- The whole thing in motion and with sound (`--play` and get caught), and
  the shared case (hauled into the sack: the catch keeps the view steady
  while the body is carried off underneath, until the black).

## Current handoff — 2026-09-29 (quick wins: scares and co-op)

Uncommitted, on top of `c154ec4` (whistle, whistle lab, placed sound). The
user asked for the roadmap v2 quick wins (below) while they decide A8, A9,
B10 and B11.

### What changed

- **A2 Better lunges** (presentation): being caught now starts with 0.4 s
  of total silence (every sink, loops included, muted at once; the
  ordinary downed sound skipped for the one caught), then he comes with
  the sting. Three poses (arms wide, the photographed one; rising from
  below with a claw at your eyes; looming over you, head cocked) and,
  sometimes, from the edge of your sight with the head snapping toward
  him. A camera recoil and shake (laid on in `head_bob`'s own
  write-and-restore), a field-of-view punch-in, a white flash and a black
  blink (`view_settings`, never in photos). Pose and side come from the
  omen counter, never from where he is. The outcome waits for the gap
  too. Photos gain `58_caught_lunge_from_below` and `59_caught_lunge_looming`.
- **A3 Footsteps that are nobody's** (director → session event → omen,
  placed): at high fear, 5–7 steps from 9–12 m behind you, walking toward
  where you stood, on the ground's real surface; turn to face them and
  there is nothing.
- **A4 False marks** (director, in company only): at high fear, a mark in
  a living teammate's colour where nobody marked, with its tick at the
  same loudness a real one has. Local only: never in the snapshot.
- **B3 The truck leaves** (`Action::DriveOff`, key X): anyone aboard the
  ready truck can go; whoever is not aboard (down, in his sack, or simply
  elsewhere) is left behind (`Snapshot::left_behind`). Refused alone,
  before the truck is ready, from outside the zone, while down, and after
  the night is over; a restart clears it. The left behind get their own
  ending ("They left without you.") and count as caught in the tally; the
  ones who left get a mark. The route script never presses X.
- **B1 Night awards** (`awards.rs`, pure): the session counts each
  player's deeds (sustos, drops, bundles laid, downs, revives, peppers,
  stampedes, warnings, times in the sack, first fall) and sends them once
  the night is over; the outcome card lists up to two awards each (left
  behind, rode in his sack, first to fall, screamed the most,
  butterfingers, his favourite, guardian angel, bone bearer, ají in every
  pocket, started a stampede, Tureco's friend, untouched). Floors mean
  nothing is given for nothing; comparing awards need company; ties go to
  the earliest in the party.
- Also fixed on the way: an event raised by an action after the night is
  over was never sent (the step returns early); `drive_off` flushes it.

### Verified

- Gate green: fmt, clippy `-D warnings`, 61 unit (new: four award rules;
  footsteps and false marks need fear, false marks need company, omens
  still never repeat back to back) + 7 district + 40 session tests (new:
  who may drive off and who is left; deeds told only at the end, cleared
  by a restart).
- Sweep (`the_routes_hold`): solo 150/150, shared 149/150 (unchanged; the
  same seed 125).
- Headless UDP pair (the protocol gained `DriveOff`, `left_behind`,
  `deeds` and two omen codes): first run **failed** (the host, evading
  on the way to bundle 3 at 156 s with fear 0.97, went down before the
  route meant it to; the client then failed because the host left); the
  rerun passed on both sides. Nothing in this batch changes the rules the
  route plays against (deeds only count, nobody presses X, omens are
  presentation) and the deterministic shared sweep is unchanged, so this
  reads as the real-time pair's timing variance, but it is one failure in
  two runs: rerun it before relying on it.

- Rendered `--photos` (release, user-approved): 54 frames, exit 0. The two
  new lunge poses were wrong at first (on this rig a negative tilt leans
  toward the viewer, and the face only shows when the head tips back far
  enough to lift the brim); fixed by placing each pose's head from the rig
  (from below: 0.35 m under the eye, 0.9 m out; looming: 0.3 m above,
  1.4 m out) and re-shot with `PHOTOS_ONLY=lunge` (new debug filter):
  all four lunge frames show his lit face under the brim.
- Rendered `--smoke` (release, user-approved): first run **failed** the
  restart census (UI nodes 121 → 131): a closed menu kept its last page's
  rows (the outcome's) until it reopened, since the menu work
  (`8fa6fd7`, never smoke-run). Fixed (closing the menu despawns its
  rows); rerun `SMOKE PASS` (586.4 s simulated, two restarts, census
  steady). Reviewed: `08_caught` is a real catch with the new lunge
  (red, his lit face at arm's length); `10_escaped` shows the outcome card
  with tonight's awards ("His favourite", "Started a stampede").
- Fixed after that review: a whistle caption from play overlapped the
  outcome menu's last row; hints and captions now show only during play
  (not re-smoked: a two-line visibility change).

### Unverified (user-led, or needs a rendered run)

- The lunge's silence, side approach, kick, punch and flash in motion
  (stills cannot show them).
- The phantom steps and false marks in play (they need high fear, and
  the false mark needs a friend).
- Driving off and the awards on the outcome card in a real shared night.

## Current handoff — 2026-09-29 (placed sound; scare and co-op proposal)

Uncommitted, on top of the uncommitted whistle work below (both on
`8fa6fd7`). The user asked for more spatial, realistic audio, better jump
scares, a scarier game and funnier, more repeatable play with friends; the
scare and co-op part is a proposal (next section), the audio is built.

### What changed (audio)

- **Placed sounds** (`audio.rs`): a `SpatialListener` on the camera (its
  ears deliberately swapped: rodio 0.22's pan gives *more* to the farther
  ear, which only its inverse-square term normally hides, and placed
  emitters pin that term at one; documented at `attach_listener`); each
  placed voice carries an `Anchor` (its true point). Its emitter stands
  2 m out in that direction at a spatial scale that keeps rodio's own
  (inverse-square, far too steep) distance gain at one, so rodio only pans
  and `Tuning::heard` does distance (full within 3 m, inverse beyond,
  faded out by 90 m) and occlusion (0.45 behind the layout's sight
  blockers). `mix`, pause/resume and restart cover both sink kinds.
- **Placed**: the truck engine, the radio, the pump crank; new loops of
  frogs at the caño and the marsh, the windmill's creak, the dynamo's hum
  once powered (`gen_audio.py`, own seeds; older files byte-identical);
  power, truck start, the altar (bones laid, all home, banishment), the key
  box, the beacon, the cattle, Tureco's growl/bark/freeing (at the dog),
  marks (at the mark, lifted up to threefold so they carry), thunder (from
  the bolt everyone saw; `storm::bolt_ground` now shared with the bolt
  mesh, which is unchanged), the bones and the silence's click omens (from
  a spot behind the listener), and **teammates' footsteps** (new: surface,
  gait and load from the snapshot; none while down, stunned, hauled or
  jumping more than 2 m in a frame, since a hauled player stands at his
  true position).
- **Never placed**: the whistle (the perception boundary: its apparent
  distance lies and nothing may say where he is), every stinger, the hunt,
  his signs (weeping, whip, bottles), counting; own actions whose actor is
  unknown to the listener stay centred.

### Verified

- Gate green: fmt, clippy `-D warnings`, 57 unit (new: placed sounds
  fade monotonically, vanish at `sound_far`, and a wall always dulls them)
  + 7 district + 38 session tests.
- Rendered `--menu-shots` (release): `MENU SHOTS OK`, no panic and no
  audio/listener warnings, so every audio system's parameters validate
  and the placed loops play on the title and in a solo night.
- Headless UDP pair: not rerun for this change (the protocol is unchanged
  and the headless mode has no audio); the whistle build's pair passed.

### Unverified (user-led)

- By ear: the panning (mild by rodio's design: a sound fully to one side
  is full in that speaker and half in the other; check it is not
  reversed, e.g. the windmill on your right when it stands on your right), the
  falloff distances, whether the frogs, windmill and hum sit right, thunder
  from the bolt's side, the clatter behind you.
- Teammates' footsteps need a shared night (two applications on one
  machine or real friends): hear a friend walk up behind you; nothing
  while they are downed or in his sack.

## Scare and co-op roadmap v2 (proposal, 2026-09-29)

Nothing here is built; it is for the user to choose from. Comparisons come
from knowledge of the games plus a few quick searches this session
(Game Developer on jump-scare timing and Alien: Isolation's fake-outs; PC
Gamer, GamesRadar and Game Rant on R.E.P.O. and Lethal Company).

### Already in the game (so not proposed again)

The inverted whistle (four takes, no steady rhythm), omens (lamps die,
silence, bones, drag marks, his hat, phantoms, the stolen torch), phantom
whistles at high fear, the caught lunge and its stinger, the lightning
reveal, the hunt sting, the dread drone and heartbeat, the dying torch and
the beam that draws him, escalation per bundle, three variants with tells,
the sack, Tureco, skill checks, the padlock, power routing, naming him,
difficulty, distinctions, the journal and tally, now placed sound.

### What the references teach

| Source | Lesson | For El Silbón |
|---|---|---|
| Alien: Isolation | Fear lives in the minutes before an encounter; directional sound is information and threat at once; fake-outs are a breather after tension | A scare *budget* with build-up and release; placed sound as bait |
| Amnesia, P.T. | Imagination beats the model; show him late and briefly; safe spaces must fail sometimes | The house stops being safe; fewer, better glimpses |
| Outlast (Trials) | Active hiding (hold your breath, peek) keeps the player doing something while terrified | Hold breath in the grass |
| Phasmophobia | Dead players stay in the game; investigation arguments | The dead become ánimas who haunt the living |
| R.E.P.O. | Clumsy shared carrying of something fragile is the funniest objective in horror | A two-person carry that rattles |
| Lethal Company | Leaving someone behind; the voice of the dead going quiet; proximity voice | The truck can leave without you; voice is postponed here |
| Among Us, Dread Hunger | A secret traitor multiplies replays | An optional accomplice mode |

### A. Scarier, and better jump scares

Cost: S small, M medium, L large. Layer and any constraint in brackets.

1. **A scare budget** (M; `director`, pure): build-up (insects and frogs
   fall silent, rain eases, heartbeat) → peak (a real scare) → release,
   and sometimes the peak is a *fake-out* (a cow lows next to you, a door
   bangs in the wind, Tureco knocks a bucket) placed near the listener.
   At most one real scare every 4–6 minutes, so each lands.
2. **Better lunges** (S–M; presentation): 0.3–0.5 s of total silence
   before the hit (every loop cut), a camera kick, a field-of-view punch
   and a vignette flash; three lunge poses instead of one; sometimes the
   lunge comes from the side you are not facing (a placed rush of wind).
3. **Footsteps that are nobody's** (S; `perception`/omen, placed): at high
   fear, steps in the grass behind you, placed like a teammate's. Solo it
   is plainly wrong; in co-op you cannot tell. Never at his true position.
4. **False marks** (S; omen, co-op): at high fear a teammate's mark tick
   sounds from somewhere nobody marked ("I didn't ping that").
5. **The house is not safe** (S–M; omen, placed): the ranch lamp dies
   while you are inside; a door bangs; knocks on the wall you are not
   facing; the radio turns itself on to static.
6. **Hold your breath** (M; `body`/`sim`, session-validated): crouched in
   grass or behind a wall, hold a key: noise falls, stamina drains,
   heartbeat pounds; letting go gasps (a noise he hears). Hiding becomes
   something you do.
7. **Don't look at him** (S; `sim`): watching him while he warns fills
   your fear faster (the legend's advice), so players look away and
   imagine instead.
8. *Decision* **Nature goes quiet near him** (S; `perception`): insects
   and frogs hush when he is truly within ~20 m. A truthful cue like
   Tureco's, competing with the lying whistle; the user decides.
9. *Decision* **A whistle with a lying direction** (S; `perception`):
   the whistle is pinned unplaced by `AGENTS.md`. Option: pan it from a
   direction chosen to mislead (behind you, or mirrored), never the true
   one. Changes a core rule, so only if the user wants it.

### B. Funny and repeatable with friends

1. **Night awards** (S; session stats → outcome screen): per player,
   "Screamed the most" (sustos), "Dropped the bones 4 times", "Left
   behind", "Tureco's favourite", "Scared by a cow". The screenshot moment
   at the end of every night.
2. **Shouts and gestures** (S–M; new action, avatars): point, wave,
   "shh", "follow me", and a shout of "¡corran!" that warns everyone and
   is also a noise he hears. Honest warnings and funny betrayals without
   voice chat.
3. **The truck leaves** (S; session): the driver can go when they choose;
   those left behind watch the lights leave and are caught at dawn.
   Scored, remembered, argued over.
4. **Ánimas** (M; session actions for the dead, presentation): the dead
   and the hauled become restless souls who, once a minute, can flicker a
   lamp, knock a bottle, rattle the key box or send one friend a phantom
   whistle. They cannot see him or mark him (no truth leak). Death stops
   being time out.
5. **A two-person carry** (M–L; `sim` + session): the father's skull in
   a wooden box: two carriers within 2 m, slow, and every bump or drop
   rattles loudly. R.E.P.O.'s piano on the llano.
6. **Mud and water** (M; `sim`, presentation): sprinting downhill in the
   rain can slip you; falling in the caño splashes loudly and a friend can
   pull you out.
7. **Struggle in the sack** (S; session, presentation): the hauled player
   mashes to make the sack rattle louder so friends can find them.
8. **Nights with a twist** (S–M; `tuning` presets like `Night`): full
   moon (no rain, seen farther), flood (more water), Tureco ran off, old
   batteries, wake night (more omens); a daily night number friends share,
   with best times.
9. **Things to earn** (M; `profile`): hats, ruanas and torch colours
   unlocked by the tally and awards.
10. *Decision* **An accomplice** (L; hidden role, protocol): one player
    secretly serves him (pockets a bundle, sends phantoms). Big replay
    hook, and it touches the protocol's hidden information.
11. *Decision* **Proximity voice**: the heart of Lethal Company's and
    R.E.P.O.'s comedy, and postponed in `AGENTS.md`. B2 and B4 are the
    non-voice substitutes until the user brings voice back.

### Suggested order

1. Quick wins: A2 better lunges, A3 phantom footsteps, A4 false marks,
   B1 night awards, B3 the truck leaves. **Built** (see the quick-wins
   handoff at the top).
2. Then: A1 the scare budget, A5 the house, B2 shouts, B4 ánimas.
3. Then: A6 hold your breath, B5 the two-person carry, B8 twists, B9
   unlocks.
4. Decisions first: A8, A9, B10, B11.

## Current handoff — 2026-09-29 (the whistle)

Uncommitted on top of `8fa6fd7` (title screen and menus). The user found
the loud and faint whistles hard to tell apart, wanted to test the whistle
alone without building the game or hunting for him, and found its steady
rhythm unscary.

### What changed

- **Distances that read** (`tools/gen_audio.py`): the three distances now
  move every cue the same way. Measured before, the middling file was
  duller than the faint one (so timbre contradicted level). Now: *loud* =
  an intake of breath, then dry, breathy, full harmonics, slightly
  overdriven, no room; *middling* = no breath, harmonics mostly gone, one
  slap echo, half room; *faint* = a bare, nearly pure thread arriving late,
  gusting with the wind, two treeline echoes, almost all room. The files
  are loudness-matched, so `gain_loud/mid/faint` (now 1.0 / 0.3 / 0.1,
  20 dB loud-to-faint; were 0.8 / 0.4 / 0.16) are the only level knob.
  Also fixed: every note boundary clicked (the envelope jumped 0.62 → 0);
  the old whistle had it too.
- **Four takes** of the phrase, each at all three distances
  (`whistle_{loud,mid,faint}_{0..3}.wav`, replacing the three old files):
  the phrase as told; lower and slower, sinking; broken off after a
  silence; hurried, stumbling into a slide. `perception` picks one per
  phrase, never the same twice running; the take travels in the network
  cue (`take`, serde default) and says nothing about where he is.
- **No steady rhythm**: stalking gaps are usually 5–11 s, but 15 % of the
  time he answers himself in 1.8–3.4 s and 20 % of the time the llano goes
  quiet for 15–26 s; warning and hunt intervals vary ±35 %; playback speed
  (and so pitch) spreads 0.92–1.06. All in `tuning.rs`, as are the
  variant thresholds (`cue_loud_above`, `cue_mid_above`).
- **Whistle lab** (`tools/whistle_lab.py`, Python stdlib, no build):
  `ladder`, `ab`, `approach`, `night`; mixes the game's files over its rain
  and insect loops with the game's gains and duck, all read from
  `tuning.rs`; `--regen` rebuilds the whistle files first (~3 s).

### Verified

- Gate green: fmt, clippy `-D warnings`, 56 unit (new: the stalking
  whistle keeps no steady rhythm, uses every take and never repeats one,
  spreads its speed, and its variant still depends on distance alone) +
  7 district + 38 session tests.
- Sweep (`the_routes_hold`): solo 150/150, shared 149/150 (was 146/150;
  the cadence feeds fear through the faint and middling phrases).
- Audio: every non-whistle file byte-identical after regeneration;
  spectrograms show the loud/middling/faint ordering and no note clicks.
  In the lab's mix at the default rain, the faint phrase sits about 6 dB
  over the ducked bed and the loud one about 17 dB over the bed.
- All four lab modes render (`--out`); playback itself was not listened to.
- Headless two-process UDP `--net-smoke` (the cue gained `take`): host and
  client `NET SMOKE PASS`. The host needed about 650 s of wall time; the
  previous build finished inside a 600 s cap, so the new cadence can
  lengthen the scripted shared night a little (more warnings and frights
  along the way); every step completed.

### Unverified (user-led)

- Whether loud, middling and faint are now unmistakable by ear, and
  whether faint is too quiet on your speakers or headphones (try
  `python3 tools/whistle_lab.py ab`, then `approach`; adjust
  `gain_faint` in `tuning.rs` and rerun; no build needed).
- Whether the new cadence and takes feel less predictable and scarier in
  play; the rendered smoke was not rerun.

## Current handoff — 2026-09-29 (title screen, menus, profile)

Main is at `5b8b971` (the overnight work, merged as PR #1). This section
describes the uncommitted working tree on top of it: the start screen and
a full menu system, asked for "as a finished game in alpha/beta", with a
live llano scene behind the title. Nothing is committed.

### What changed

- **Title screen** (`ui/title.rs`): a plain launch opens on `Flow::Title`.
  The camera drifts slowly past seven landmarks from the layout (ranch,
  ceiba, corral, fields, caño, lookout, gate), 17 s each with fades through
  black; faint whistles every 22–38 s; on a lightning strike (at most every
  70 s) the phantom omen may show his shape. The torch and head bob are off;
  a new cuatro loop, `title_theme.wav`, plays only here.
- **Menus** (`ui/menu.rs`), keyboard (arrows/WASD, Enter, Esc) and mouse:
  Main (Play, Play with friends, Journal, Settings, How to play, Credits,
  Quit), Play (difficulty, night number or a new night), Host (this
  machine's LAN address), Join (address, remembered), Journal (the 20 pages
  found over every night, readable again, and the tally), Settings (volume,
  sensitivity, invert Y, FOV, brightness, head bob, captions, fullscreen),
  pause menu (resume, settings, journal, how to play, restart, leave to the
  title), confirmations, and the outcome's choices. The old pause panel and
  outcome buttons are gone.
- **Profile** (`profile.rs`): settings, pages read and a tally (nights,
  escapes, banishments, caught, fastest) saved as JSON under
  `$XDG_DATA_HOME/el-silbon/` (else `~/.local/share/el-silbon/`); saving is
  best effort. The debug drivers never read or write it.
- **Nights begin at runtime** (`net::StartRun` / `net::LeaveRun`): the
  tuning, layout and endpoint are replaced without a relaunch. The bundle
  perches follow the night's layout (one perch per hiding place, shown
  under tonight's); the briefing's night line updates.
- **Joiners adopt the host's night**: a seed or difficulty mismatch is no
  longer a refusal; the host answers `ServerMessage::Tonight`, the joiner
  switches to that night and knocks again (once). Build fingerprint
  mismatches are still refused.
- The party roster always exists and shows only in a shared night.
- `--play` skips the title; `--menu-shots` is a new labelled debug driver.
- Fixed while verifying: a solo night started from the menu skipped its
  briefing, and "Leave to the title" from the pause menu stuck on Paused
  (both were a later system in the same frame overriding the pending state);
  the `◂ ▸` and `▏` glyphs are not in the bundled Noto fonts (now `‹ ›`
  and `|`, also in the naming panel); a `Res<Settings>`/`ResMut<Settings>`
  conflict in the menu's input system panicked at startup.

### Verified

- Gate green: fmt, check, 55 unit + 7 district + 38 session tests (new:
  the profile round trip; a loopback UDP test in which a joiner with
  another night is told the host's and then admitted with it), clippy
  `-D warnings`.
- Rendered `--menu-shots` (release): `MENU SHOTS OK`, 19 captures reviewed
  (title at four stops, every page, the briefing, play, pause, pause
  settings and the return to the title), in a 1600×900 window and a
  portrait tile.
- Headless two-process UDP `--net-smoke` on the final build: host and
  client `NET SMOKE PASS` (the protocol gained `Tonight`).
- Audio regenerated: every older file byte-identical; the theme loop's
  level and seam checked numerically.

### Unverified (user-led)

- The title in motion: the drift and fades, the faint whistles, the
  lightning apparition (not captured), the theme's mood and level.
- Mouse use of every page; typing seeds and addresses; fullscreen toggle;
  settings persisting across launches; the journal filling as pages are
  found; the tally after real nights.
- Hosting and joining from the menus on two machines, including a joiner
  on another night adopting the host's.
- The sweep, rendered `--smoke`/`--tour` and the rendered UDP pair were not
  re-run for this change (the rules are untouched; smoke and photos skip
  the title).

### Try it

```sh
cargo run --release --locked                  # title screen
cargo run --release --locked -- --play        # straight into a night
cargo run --release --locked -- --menu-shots --shots /tmp/menu   # captures
```

Checklist: move through every page by mouse and by keys; change each
setting, quit, relaunch and see it kept; start a night from Play, press
Esc, leave to the title, start another; host from one machine and join from
another with a different difficulty chosen. Useful F12 views: the title
at the ranch and at the ceiba, the journal after finding a page, the pause
menu.

## Current handoff — 2026-09-29 overnight (gameplay, fear, the tale)

This section describes the current working tree: the Tier 1 look pass (the
section after this one) plus the whole gameplay-and-fear roadmap the user
approved, built overnight in nine gated phases (the build log below has the
details, the reasons and the numbers for each). Nothing is committed.

### Status

- **Gate green** on the final tree: `cargo fmt --check`, `cargo check
  --locked`, `cargo test --locked` (53 unit + 7 district + 38 session tests
  pass; 3 opt-in ignored), `cargo clippy --locked --all-targets -- -D
  warnings`.
- **Sweep** (`cargo test --locked --release --test session -- --ignored
  --nocapture the_routes_hold`): solo 150/150, shared 146/150 (≥ 97 %).
- **Rendered `--smoke` and `--tour` pass** on the final build, and the
  headless two-process UDP `--net-smoke` passes (see the evidence table).

### What the game has now

- **Tension**: a torch that runs down (spare batteries around the
  landmarks) and whose lit beam draws him from 45 m; every bundle laid to
  rest angers him more and he whistles more often; after three averted
  warnings in a row he tires of waiting over you.
- **Skill checks** (Space) while laying bones, cranking the pump and turning
  the engine over; a miss screeches across the llano.
- **The tale** in twenty pages of the old llano (letters, ledgers, a copla,
  the parish register, a telegram, an almanac, a clipping, a prayer card, a
  photograph, the shelf radio): the son, the deer, the father, the curse,
  Tureco, the torn sack, and the Hacienda Santa Rosa's attempt to lay the
  father down. Pages found are counted across nights.
- **Fear**: a per-player director sends omens after long quiet (lamps die,
  silence, bones in the dark, drag marks, his hat on the trail, phantoms at
  high fear, a stolen torch late at night); phantom whistles at high fear;
  jump scares (he lunges into your face when you are caught, a stinger when
  lightning shows him close, a sting when a hunt begins); a dread drone.
- **Puzzles**: the padlocked truck key (three digits, new every night,
  written in three pages); the dynamo's lamp lines (two of three, switched
  at the windmill panel; the bridge starts dark); naming him at the ceiba.
- **Replay**: bundles hide in different places every night; three returns
  of the Silbón (the Drunkard's, the Son, the Drover) with their own signs,
  temper and trade-offs, revealed after the run; banishment as a second way
  to win; a new night every solo launch (`Night #n`); distinctions on the
  outcome screen.
- **Friends**: caught with a teammate standing, you go into his sack and
  are carried off; ají in his path (or Tureco's bark) drops you, or you are
  gone in 35 s. **Tureco** the dog, untied behind the house, follows you,
  growls truthfully when he is near and barks him off (90 s courage).

### Evidence (final build)

All under `screenshots/overnight-*/` (git-ignored). No desktop input, focus
or window manipulation was automated; the drivers are in-game.

| Run | Result |
|---|---|
| Gate | fmt, check, 53 unit + 7 district + 38 session tests, clippy: green |
| Sweep, normal nights | solo 150/150, shared 146/150 (seeds 60, 103, 115, 149: downed at the altar at ~110 s) |
| Sweep, `ROUTE_NIGHT=hard` / `gentle` | 136/150 + 133/150 / 150/150 + 149/150 (information only) |
| Rendered `--photos` (`overnight-final/`, `overnight-pages/`) | exit 0; new frames `53_caught_lunge_down`, `54_caught_lunge_standing`, `55_tureco`, `56_ui_page_radio`, `57_ui_page_copla` reviewed |
| Rendered `--smoke` (`overnight-final/smoke-run/`) | `SMOKE PASS`, 586.4 s simulated, two restarts, census unchanged (1 camera, 1 directional, 5 spots, 33 points, 1 ambience, 1 threat, 306 meshes, 144 UI nodes). Captures reviewed: lamp-line objective, key-box guidance, skill bar at the ignition, the Drover's cattle, the outcome with the night's reveal and distinctions, the caught lunge ("He has you.") |
| Rendered `--tour` (`overnight-final/tour-run/`) | `SMOKE PASS`, 756.1 s simulated, three restarts |
| Headless UDP pair, before lamp lines (`overnight-net/`, first run) | host and client `NET SMOKE PASS` |
| Headless UDP pair, final build (`overnight-net/`) | host and client `NET SMOKE PASS` (shared win, shared failure, both restarts, the leaving carrier's load recovered) |
| Rendered UDP pair, final build (`overnight-net-rendered/`) | both `NET SMOKE PASS` and `NET RENDER SMOKE PASS` (scene census preserved through run 3) |

The smoke and tour ran on the build before `--night` (difficulty only adds
a launch option, a tuning preset that defaults to the unchanged normal, and
the handshake field); the final UDP pair covers the handshake.

### Unverified (user-led)

- Everything above in hands-on play: difficulty and pacing with the new
  pressures (battery, lure, escalation, variants), whether the skill checks
  feel fair at real frame rates and network latency, how often omens come,
  whether the jump scares land and the phantoms read, the radio and
  weeping sounds, Tureco's look and following, the padlock and naming
  panels' usability, the sack with real friends.
- Physical LAN, three- or four-human sessions.
- The whistle mix changes and the Tier 1 lighting (earlier handoff).

### How to try it

```sh
cargo run --release --locked                    # a new night every launch
cargo run --release --locked -- --seed 42       # a chosen night (share it)
cargo run --release --locked -- --night hard    # gentle | normal | hard
cargo run --release --locked -- --host 127.0.0.1:5000 --seed 7   # host (friends join with the same --seed)
cargo run --release --locked -- --join 127.0.0.1:5000 --seed 7
```

Checklist: read the note on the table and the shelf radio; untie Tureco
behind the house; lay a bundle (answer the Space check); let the torch run
low; open the key box with the three numbers (Madrina at the ceiba, the
stilt hut ledger, the lookout cabin); switch the lamp lines at the panel;
on another night, watch for the signs and name him at the ceiba (N); with a
friend, get caught and have them pepper his path.

### Next steps

1. User playtest (try `--night hard` if normal does not scare you) and
   tuning (numbers are in `tuning.rs`: `battery_*`,
   `light_lure_*`, `pressure_rite`, `check_*`, `omen_quiet`,
   `phantom_*`, `haul_*`, `dog_*`, `tell_every`, `averts_to_withdraw`).
2. Roadmap items not built: two-person mechanisms, decoy bones, loadouts
   and unlocks, difficulty tiers, a multi-night campaign.
3. Visual Tiers 2–5 (CC0 textures, Blender glTF, CC0 audio) as planned
   below; update `AGENTS.md`/`assets/SOURCES.md` with the first CC0 asset.

## Previous handoff — 2026-09-29 (Tier 1 look pass)

Tier 1 of the visual upgrade roadmap (below), plus three user requests made
during it: head bob while walking, a darker night, and a more distinct
whistle. Uncommitted; builds on commit 6812742.

### What changed, and why

- **Night levels** (`world/land.rs`): one set of constants — `MOON_LUX`
  1,150 → 200, `NIGHT_AMBIENT` 90 → 10, horizon/zenith sky and fog colour
  roughly halved — now read by the spawn (`app.rs`, `land.rs`) and by the
  lightning flash (`world/weather.rs`), which only adds to them. The
  flashlight and lamps keep their power, so they now carry the frame.
- **Grading/post** (`player.rs`): `post_saturation` 1.14 → 0.9,
  temperature −0.04 → −0.03, bloom 0.12 (shared `BLOOM` constant with the
  flash), camera `Vignette` (0.55) and a faint `ChromaticAberration`
  (0.006). The UI vignette in `ui/hud.rs` is untouched: it is fear and
  exposure feedback.
- **Flashlight**: a handheld torch model (lathe mesh: knurled grip, flared
  head, glowing lens) held low right, nearly parallel to the view, with look
  sway and a hand bob. Two spots from its head: the hot core (same 380 k
  lumens, cone 0.1/0.27 rad, shadows, `VolumetricLight`) and a wide dim spill
  (70 k, 0.22/0.62 rad, shadows) for the soft ring. `Flashlight` now sits on
  the torch; `torch_light` switches both beams and the lens from its state.
- **Volumetric beam**: `VolumetricFog` on the camera (64 steps, no jitter:
  without TAA jitter reads as crawling grain; no ambient in the fog) and a
  60 m `FogVolume` that follows the eye. First pass at density 0.035 was a
  solid white cone; now 0.012 with light intensity 0.25.
- **Lamp shadows** (`world/dynamic.rs`): the three nearest pooled lamps
  within 28 m cast shadows (fixtures were already `NotShadowCaster`).
- **Head bob** (`player.rs::head_bob`): rise, side sway and a slight roll
  from a shared stride phase (`Gait`, also driving the torch bob), scaled
  by pace. Presentation only: targeting, sight and aim read the `Pose`, not
  the camera. It lays its offset on whoever placed the camera that frame
  (session, photo or F12 view) and removes it next frame, and a jump over
  1 m (teleport, restart) is not a stride.
- **Whistle distinctness** (user: "not accurate or distinctive enough when
  far and close"). The rules are unchanged and still inverted, non-spatial
  (see `AGENTS.md`); the network cue stays categorical on purpose (no
  invertible continuous distance), so the fix is in the categories:
  - Gains 0.5/0.34/0.22 → 0.8/0.4/0.16 (`tuning.rs`).
  - Timbres (`tools/gen_audio.py`): the loud one is closer and breathier;
    the middling one gets one slap echo and half reverb; the faint one is
    band-limited like sound across open air, with two late treeline echoes
    and almost all reverb. The rng draws are unchanged, so only the three
    whistle WAVs changed (verified).
  - Measured active loudness as played: loud −15.0 → −9.8 dB, mid −23.5 →
    −22.3 dB, faint −31.1 → −34.5 dB (a spread of 16 → 25 dB).
  - The ambience and rain duck to 0.4× while any whistle sounds
    (`tuning.whistle_duck`, `audio.rs::mix`), so the faint one is heard as
    faint rather than lost.
  If "not accurate" meant the loud-means-far inversion itself, that is
  the design; changing it is a rules decision, not a mix fix.

### Evidence

- Gate green: `cargo fmt --check`, `cargo check --locked`,
  `cargo test --locked` (78 passed, 1 opt-in ignored), `cargo clippy
  --locked --all-targets -- -D warnings`.
- Photos (`screenshots/tier1-look/`, git-ignored), reviewed at full
  resolution: much darker frames; lanterns, the lit ranch window and porch
  bulb read as the light sources; the Silbón reads as a silhouette against
  the lit house; the torch head sits low right.
- The rendered `--smoke` on the first Tier 1 build (before head bob, the
  darker levels and the audio change) passed: `SMOKE PASS`, 508.0 s
  simulated, two restarts with the census unchanged (4 spots now: core,
  spill and the truck's two headlamps).
- Final build (with head bob, darker levels and the new whistles):
  `--photos` exit 0 (47 frames, reviewed), and the rendered `--smoke`
  passes: `SMOKE PASS`, 508.0 s simulated (30,819 frames), win and failure
  routes, two restarts with the census unchanged (1 camera, 1 directional,
  4 spots, 32 points, 1 ambience, 1 threat, 276 meshes, 127 UI nodes).
  The `--tour` and UDP pairs were not rerun for this pass.

### Unverified (user-led)

- How it feels in hands-on play: darkness vs readability, head bob comfort,
  the torch against walls (a viewmodel can poke into geometry at contact),
  the frame rate with volumetrics and three shadowed lamps.
- Listening: the three whistles over rain, and the duck.

### Next steps

1. User-led check (commands below): walk the ranch with the torch on and
   off; stand in rain near a lantern; turn quickly; sprint; press F to see
   the lens and both beams switch. Listen for whistles at different
   distances from him.
2. If too dark: raise `NIGHT_AMBIENT` or `MOON_LUX` in `world/land.rs`.
   If the head bob is too strong: `BOB_*` in `player.rs`.
3. Tier 2 (HUD on demand, film grain), then CC0 textures (Tier 3) once
   the asset policy files are updated.

### Resume commands

```sh
cargo build --release --locked
./target/release/el_silbon                                   # play solo
./target/release/el_silbon --photos --shots screenshots/tier1-look
./target/release/el_silbon --smoke  --shots screenshots/tier1-look/smoke-run
python3 tools/gen_audio.py                                   # deterministic
```

## Overnight build log — 2026-09-29 (gameplay and fear roadmap)

The user approved the whole gameplay and fear roadmap below, plus telling the
tale through period documents and adding jump scares alongside the
psychological horror, and left this running overnight. Work goes in phases;
each ends with the gate green (fmt, check, test, clippy) and the opt-in
300-route sweep at or above 97 %. Uncommitted.

### Phase 1 — tension core (done, gated)

- **Flashlight battery** (`net::session`, `tuning.battery_*`): 300 s of
  light when full, draining only while switched on; at 0 the torch is dead
  whatever the switch says (`Participant.light` is now switch AND charge).
  Six pairs of spare batteries (`District::batteries`, beside existing
  finds) restore 60 % and cannot be taken while the torch is fresh
  (≥ 95 %). Wire: `Vitals.battery`, `Snapshot.batteries`,
  `TargetKind::Batteries`, `Event::BatteriesTaken` (finder only). Protocol
  id bumped to …0003. Presentation: below 18 % the beam gutters and drops
  out (`player::torch_light`); HUD shows "Torch n%" under 50 % and "Torch:
  dead"; the battery pair is a small lathe model on a perch.
- **Beams draw him** (`Session::light_lure`): a lit torch he can see within
  45 m makes him come and look (a noise at the holder, not masked by rain),
  every 2 s. He notices a lit player at 28.6 m, so the torch reveals you
  from much farther than your body does.
- **Escalation**: `pressure_relief` is now `pressure_rite` (+0.08 per
  bundle laid to rest, more than carrying one): the night gets harder as the
  bones go home. Stalking whistles come up to a third sooner with pressure.
- **Driver**: the route keeps its torch off while it hides and while he has
  been seen in the last 45 s (`ScriptFrame.dark`, honoured by the test
  pilot, `--smoke` and both network smokes through the same toggle key a
  player uses); it picks spare batteries up as optional steps. Without the
  torch discipline the lure trapped the solo route in a warn/hide loop at
  the truck (136/150).
- Tests: `a_torch_runs_down_and_dies_and_spare_batteries_bring_it_back`,
  `a_lit_torch_he_can_see_draws_him_from_beyond_his_notice`; the pressure
  test now requires escalation. Sweep: solo 150/150, shared 148/150.
  New diagnostic: `ROUTE_LOG=1 ROUTE_SEED=n cargo test --release --test
  session one_solo_seed -- --ignored --nocapture`.

### Phase 2 — skill checks (done, gated)

- New pure module `skill` (unit-tested): a per-player `Rhythm` seeded from
  the run seed and player id. While a long task is worked — laying bones at
  the altar (first check 0.5 s into each bundle; the hold is now 3.0 s),
  cranking the pump, turning the engine over — a check comes every 2.5–5 s
  of work: a 0.6 s warning (chime, pulsing frame), then a needle sweeps a
  track in 1.1 s. Zone: 14 % of the track starting at 45–82 %; its first
  4 % is great.
- The host owns every check. `Action::Skill { id, needle }` claims where the
  client saw the needle; `Rhythm::press` refuses another check, a claim
  ahead of the host's needle (+6 % slack), older than the latency window
  (0.35 s) or before the needle moved. Letting go mid-check costs nothing.
- Great: +4 % of the task (the altar hold +12 %). Miss (outside the zone or
  no press): −8 % of the pump or engine, or the bundle's laying starts over;
  a 40 m noise at the site (the llano hears iron screech), a fright for the
  worker, `Event::SkillMissed` for everyone. Two cranking together finish
  sooner but each gets checks. (The miss rule is superseded by M1b item 6:
  a setback in seconds, a stall, the last turn waits, a louder miss.)
- Wire: `Vitals.check: Option<CheckView>` (id, host needle, zone),
  `Intent.skill`, `net::needle` (snapshot needle carried on by its age).
  HUD: a SPACE bar under the hold progress (amber zone, pale great sliver,
  red needle). Sounds: `check_warn`, `check_great`, `check_miss` on their
  own seeds (no older file changed). First check teaches the key.
- Driver: `RouteScript::skill_press` watches the needle between snapshots
  and presses just inside the great sliver. Tests: `skill` unit tests and
  `long_tasks_ask_for_skill_checks_a_miss_screeches_and_a_great_press_speeds_the_work`;
  the rig's `work` answers checks. Sweep: solo 149/150, shared 146/150.

### Phase 3 — the tale in the llano's own papers (done, gated)

- `lore.rs` rewritten: every page has a `Medium` (note, letter, ranch
  ledger, newspaper clipping, the back of a photograph, almanac margins, a
  copla, a telegram, the parish register, a watch log, a prayer card, the
  radio) and a title; `PAGES` = 19 (ids 0–18). The seven old notes are kept;
  twelve new ones tell the legend in order and set up the run: the son and
  the deer's entrails, the father he killed, the grandfather's chaparro
  switch, the ají and the dog Tureco, the curse of the sack; the storm night
  the sack tore on the caño fence, the five bundles hidden in the landmarks,
  the parish book's instruction to lay the father at the Santa Rosa ceiba,
  the telegram that says the truck is the only way out, the foreman's
  ledger (escalation: "every sack we laid made him angrier"), and hints for
  batteries, the light lure, skill-check rhythm and the stolen torch.
- Placement (`District::notes`, all reachable per `tests/district.rs`): the
  gate crates, the ranch cart, the house shelf (a real radio model with a
  glowing dial, and a framed photograph), the coop, the windmill barrel,
  the corral hay, the fields barrel, the tool shed, the stilt hut crates,
  the tower's foot and the truck crates.
- Presentation: the page panel shows the medium and title, paper colour by
  medium (the radio card is dark with light ink), and "pages found n/19"
  (`encounter::PagesRead`, kept across restarts so the tale builds over
  several nights). Reading the radio plays `radio_broadcast.wav` (static,
  crackle, a murmured announcer, a stray cuatro). The outcome screen counts
  pages and closes the tale when all are read.
- Rendered smoke on this build: `SMOKE PASS` (503 s simulated); the skill
  bar, its lesson caption and the HUD reviewed in the captures.

### Phase 4 — fear: omens, phantoms and jump scares (done, gated)

- New pure module `director` (unit-tested): per player, seeded. Left alone
  (not warned or hunted) for 35–70 s (shorter with pressure; longer and
  only whispers before he wakes), the night sends an omen, never the same
  twice running: lamps die, silence, bones clatter, drag marks, his hat on
  the trail, a phantom (fear ≥ 0.55), a stolen torch (pressure ≥ 0.45).
  Events `Omen*` go to that player only; none says where he is or changes
  a rule.
- Phantom whistles (`CueDirector::phantom`): at fear ≥ 0.7 a rare whistle
  that is not there (any timbre, slurred), captioned "A whistle…? Or only
  the blood in your ears." They do not add fear (an earlier version did and
  fed a susto spiral that cost the sweep). `ServerMessage::Cue.phantom`.
- `world::omen` presents it all near the listener's own eye: lamps and
  lanterns within 35 m gutter out for 7 s; ambience and rain hush for 7 s
  and end on one bone clack; two furrows of drag marks; his straw hat on the
  trail (gone when you come within 5 m); an unlit tall hatted silhouette at
  the edge of view for 0.7 s; a sweeping torch 30–42 m off that drifts and
  winks out when approached.
- Jump scares: caught, he lunges into the camera (arms up, shaking) with
  `sting_caught`, and the outcome screen waits 1.6 s for it; a lightning
  strike with him in view within 32 m plays `sting_reveal` (45 s cooldown);
  a hunt starting plays `sting_hunt`. A `dread_drone` loop swells with the
  night, the bundles laid and pursuit.
- Rule change, anti-camping: after three averted warnings in a row he tires
  of waiting over his prey, sinks and rises far away (`averts_to_withdraw`,
  test `averted_again_and_again_he_tires_of_waiting_and_rises_elsewhere`).
  Escalation had made him camp the ignition and the stilt hut; this fixed
  the driver's warn/hide loops and is kinder to humans too.
- New sounds (own seeds; no older file changed): `sting_caught`,
  `sting_reveal`, `sting_phantom`, `sting_hunt`, `omen_bones`,
  `omen_lamps`, `omen_swell`, `dread_drone`.
- Sweep: solo 150/150, shared 150/150 (partner's truck wait raised to 900 s).
- Unverified: how the omens and scares look and sound in hands-on play; the
  rendered smoke checks they don't break the run.

### Phase 5a — the padlocked truck key (done, gated)

- The ignition now also needs the key (`Blocked::NeedKey`), padlocked in a
  small steel box on the crates by the windmill (`District::lockbox`). Its
  three digits come from the seed (`sim::lock_code`: new every night, first
  digit 1–9) and are written into three pages with `{D1}{D2}{D3}`
  (`lore::fill`): the Madrina's note at the ceiba altar (first), the
  foreman's ledger in the stilt hut (second, and where the box is), the
  guard's note in the lookout cabin (third). All three lie on the route
  you walk anyway.
- `E` at the box opens a padlock panel (client-local, like pages): 1/2/3
  turn the dials (Shift turns back), Enter tries (`Action::TryCode`). The
  host checks reach and the code; a wrong try rattles (`Event::LockRattle`
  to the one trying, a 12 m noise), the right one clunks open
  (`Event::KeyFound` to all; the lid opens and the padlock is gone).
  HUD objective and guidance point to the box and the pages; sounds
  `lock_rattle`, `lock_open`.
- First placed by the truck, where the route ran along the road fence and
  he could see (and expose) the driver through it with no cover or way to
  reach him: 141/150 solo. At the windmill, right after the pump: 150/150.
- Driver: opens the box after the pump with the code (as a player who read
  the three pages). Tests: `every_seed_locks_the_key_with_its_own_three_digits`,
  `the_truck_key_is_padlocked_and_only_the_pages_numbers_open_it_at_the_box`;
  the whole-map playthrough opens the box; the truck test asks for the key.
- Jump-scare framing reviewed with two new labelled photo frames
  (`53_caught_lunge_down`, `54_caught_lunge_standing`, held by the photo
  driver): he stops at 1.7 m, head snapped back so the brim lifts, two pale
  eyes (new, tiny, only lit by light) in the dark skull, hooked hands thrown
  wide, a hard light catching him. Also found: the photo mirror is fine for
  the torch; an earlier version put him inside the camera (arms and coat
  over the lens, pitch black).
- Sweep: solo 150/150, shared 150/150.

### Phase 5b — hiding places that change (done, gated)

- `District::relic_sites`: each bundle but the table one has 2–3 hiding
  places in its landmark (corral: shed, gate trough, east pens; fields:
  cart, grass north, grass south; caño: two spots in the stilt hut;
  lookout: the cabin deck, the shed at the tower's foot).
  `Layout::with_seed(seed)` picks one per bundle; the app, the headless
  network smoke and the sweeps use it (`Layout::new()` keeps the authored
  spots for the fixed tests). Every candidate is proven reachable, and 40
  seeds give at least 12 different nights.
- Rejected after measurement: the deck's open front (shared 124/150: seen
  from the whole llano with nowhere to break his sight).

### Phase 6 — which of him walks tonight (done, gated)

- `sim::Variant`, seeded per night: **El Borracho** (the drunkard's
  return: hears 40 % farther and whistles slurred, but stalks 8 % slower;
  bottles clink), **El Hijo** (the son himself: weeps each time a bundle is
  laid, each laid bundle angers him 25 % more, but pepper keeps him counting
  30 % longer), **El Arriero** (the drover: stalks 6 % faster, but the
  herd bellows when he passes within 18 m and a whip cracks in the dark).
  Hunt speed is never changed. First versions without trade-offs (the Son
  ×1.6 anger, the Drover ×1.12 pace) cost the sweep (138/150 shared);
  with trade-offs 150/146.
- Signs: `Event::Weeping` (at each laid bundle, the Son), `WhipCrack` /
  `BottleClink` every 60–110 s while he walks (session `tells`, own rng),
  the Drover's cattle, the Drunkard's slurred phrase speed (perception).
  Captions and sounds `tell_weeping`, `tell_whip`, `tell_bottles`.
- A new page, "Las tres vueltas" (id 19, the tool shed; `PAGES` = 20),
  teaches the three returns and the rite.
- **Banishment, a second way to win**: with every bone at the ceiba, `N`
  at the altar opens "Which of him walks tonight?", 1/2/3 and Enter
  (`Action::Name`). Right: `Event::Banished`, he sinks for good, the run is
  won (`banished` in the snapshot; its own outcome text and `banished.wav`).
  Wrong: `Event::NameWrong`, he is roused at once, and the ceiba will not
  hear another name for 60 s. The truck escape still works as before; the
  driver keeps taking the truck.
- Tests: `every_variant_walks_some_nights_and_only_the_right_name_lays_him_to_rest`,
  `naming_him_rightly_at_the_ceiba_lays_him_to_rest_and_wrongly_enrages_him`,
  `each_return_leaves_its_own_signs`. Sweep: solo 150/150, shared 146/150.

### Phase 7 — bones in the sack (done, gated)

- With a teammate still standing, a caught player is no longer left where
  they fell: he puts them in his sack (`ThreatState::Hauling`,
  `Encounter::haul`, `Event::Hauled`) and walks the patrol toward the node
  farthest from everyone standing at 1.7 m/s (slower than a walking
  teammate), blind to everything else. The captive's pose rides with him
  (`Session.captive`); they cannot be revived while in the sack.
- Pepper in his path stops him to count and he drops them
  (`Event::SackDropped`): then the usual revive. Reaching the far node, or
  35 s without being stopped, and they are gone (`Event::Taken`, dead).
  Solo play is unchanged: caught alone, the night is over.
- Wire: `PlayerView.hauled`, threat state code 5. Presentation: the
  captive's avatar hides (inside the sack); the captive's own screen goes
  dark brown, "IN HIS SACK — only ají in his path will make him drop you";
  teammates get "He stuffed them into his sack and walks off! Get ají in
  his path…", then "He drops the sack…" or "He is gone into the grass. And
  so are they."
- Tests: the revive test now plays haul → pepper → drop → revive;
  `nobody_stops_him_and_the_one_in_his_sack_is_gone`. Sweep: solo 150/150,
  shared 146/150.
- Rendered smoke on the variants build: `SMOKE PASS` (561.7 s simulated).

### Phase 8 — Tureco (done, gated)

- The dog from the notes, tied to a post behind the house by the back door
  (`District::dog_post`). Hold `E` to untie him (1.5 s, hold kind 7,
  `Event::DogFreed`); he follows whoever freed him (or the nearest friend
  standing within 12 m), trailing 1.8 m behind at up to 5.5 m/s. If his
  friend is in the sack, he runs after the sack.
- He senses the truth, up close only (`perception::dog_senses`): within
  22 m of the Silbón he growls (`Event::DogGrowl` to his friend: "He is near
  — whatever the whistle says"), the one cue that does not lie. Within
  7 m, if he has the courage, he barks (`Event::DogBark`, a 30 m noise):
  whatever the Silbón was doing (stalking, warning, hunting, hauling), he
  flinches and sinks away to rise far off (`Encounter::flinch`); anyone in
  his sack falls out. Courage returns after 90 s. Tied, he still growls and
  barks at the house.
- Wire: `Snapshot.dog: DogView` (pos, facing, mood 0 tied / 1 following /
  2 growling / 3 barking, owner), `Scene.dog_tied`. Presentation
  (`world::dog`): a lean brown dog (body, head with snout and pricked ears,
  four swinging legs, tail), sitting on a rope at his post while tied,
  head low to growl, thrown up to bark. Sounds `dog_growl`, `dog_bark`;
  Doña Rosa's note now says where he is and that he fears dogs. New photo
  frame `55_tureco` (reviewed).
- The route driver leaves him tied, so the sweep measures the game without
  him. Tests: `tureco_follows_whoever_unties_him_growls_when_he_is_near_and_barks_him_off`,
  `tureco_barks_the_sack_open`. Sweep: solo 150/150, shared 146/150.

### Phase 9 — power routing, a new night each launch, distinctions (done, gated)

- **Lamp lines**: every powered pole lamp belongs to a line
  (`Lamp::circuit`: 0 the hacienda and the western road, 1 the middle road
  and the corral, 2 the eastern road and the bridge). The old dynamo feeds
  two at most (`Progress::circuits`, `FIRST_CIRCUITS` = hacienda + corral:
  the bridge starts dark). A panel on a post beside the pump
  (`District::panel`, `TargetKind::Panel`, a press) steps through the three
  pairs (`CIRCUIT_SETTINGS`), with a clunk (`Event::LinesSwitched`).
  `Layout::is_lit(p, circuits)` now takes the live mask; fear, the driver's
  "lie low only in lamplight" and the lamp pool all read it
  (`WorldView.circuits`). The panel's three levers stand up for the live
  lines; each line's bulbs glow on their own material. The objective line
  names the lit lines. Since lamplight calms fear, choosing where the light
  goes (the truck's wait at the bridge, or the road home) matters.
- The driver switches the bridge line on after opening the key box. Test:
  `the_dynamo_carries_two_lines_and_the_panel_chooses_which`.
- **A new night every launch**: solo play without `--seed` rolls a seed
  from the clock (bundle spots, padlock code, which of him walks, storm);
  the debug routes keep 1997 and shared sessions still need the same
  `--seed` on every machine. The briefing shows "Night #n", now describes
  the key box, the rite, the rhythm, the battery, the sack and Tureco, and
  lists Space and N.
- **Outcome**: the night's variant is revealed ("Tonight it was the son
  himself (El Hijo). Night #n.") and distinctions are listed: silent as the
  grass (never seen), unbroken (nobody fell), nobody left behind (someone
  got up), the one who named him, quick hands (under 8 minutes), keeper of
  the tale (every page).
- Sweep: solo 150/150, shared 146/150.

### Phase 10 — how hard the night is (done, gated)

- `--night gentle|normal|hard` (`tuning::Night`, `Tuning::with_night`;
  normal is the tuned game, unchanged). Hard: he notices you 12 % farther,
  his gaze downs you 15 % sooner, stalks 8 % faster, the torch lasts 70 %,
  every rite angers him 30 % more, the beam lures from 20 % farther, omens
  come 30 % sooner, the skill zone is 25 % narrower, and he needs four
  averted warnings to tire. Gentle the reverse (and a wider zone that still
  fits on the track). Hunt speed and the warning stay within the same caps
  (test `every_night_keeps_him_slower_than_a_walker_and_his_warning_readable`).
- Shared sessions: `ClientMessage::Hello.night`; a mismatch is refused with
  "Night mismatch: restart with --night …". The briefing shows it.
- The sweep measures other nights with `ROUTE_NIGHT=gentle|hard` (the gate
  stays on normal): hard solo 136/150, shared 133/150; gentle 150/150,
  149/150 — hard is genuinely harder for the scripted player.

## Visual upgrade roadmap (2026-09-29)

Source: a frame-by-frame review of a reference horror trailer ("The Widow",
Godot + Blender MCP + CC0 textures and sounds) against our
`screenshots/handoff-review/` captures. The gap is mostly **lighting and
value structure**, then material detail, geometry, presentation.

What the reference does: the frame is mostly near-black and light comes only
from a few warm sources (candles, one lit window, the flashlight); the torch
has a hot core and a soft ring and is always in hand; surfaces are CC0 PBR
sets (brick, parquet, rust, wood) with normal and roughness maps; props are
bevelled and trimmed; there is no HUD, only serif italic captions, a vignette
and film grain; its best horror beat is a silhouette in a lit doorway.

What ours does: a blue ambient of 90 plus 1,150 lux of moonlight lit every
frame to one even mid-blue, with the sky brighter than the ground;
`post_saturation` 1.14 pushed that blue; most lamps cast no shadow; props
are sharp-edged boxes without contact darkening; the objective list and
debug strip are in every frame; the Silbón portrait reads as a dark
scarecrow.

**Asset policy decision (user, 2026-09-29):** the user intends to adopt CC0
textures, Blender-made glTF meshes (via Blender MCP) and CC0 audio. Until the
first such asset lands, `AGENTS.md` and `assets/SOURCES.md` still say
everything is original and generated; update both, and record every file's
source URL and licence in `SOURCES.md`, in the same change that adds it.

### Tier 1 — lighting pass (no new assets)

- Night levels: ambient fill and moonlight cut hard, fog/sky darkened so the
  practical lights (lanterns, ranch window, beacon, flashlight) carry the
  frame. One set of constants, shared by the spawn and the lightning flash.
- Grading: saturation below 1, practicals stay warm.
- Flashlight: tight hot core plus a wide dim spill (Bevy 0.19's
  `SpotLightTexture` projects orthographically, a cylinder, so it cannot
  draw a cone cookie; two spots do it); a handheld torch model with lens
  glow, walking bob and look sway.
- Shadows on the nearest pooled lamps only (the fixtures are already
  `NotShadowCaster`).
- `VolumetricFog` in a volume that follows the camera, flashlight as a
  `VolumetricLight`: beams in the rain.
- Camera `Vignette` and a faint `ChromaticAberration` (Bevy's effect stack);
  the fear/exposure UI vignette stays as gameplay feedback.
- Deferred with reason: SSAO/contact shadows (with the ambient fill cut there
  is little ambient left to occlude; contact shadows want TAA, which would
  smear the vertex-animated grass and rain); film grain (needs a custom
  post-process node; see the `post_processing.rs`/`custom_post_processing`
  examples).

### Tier 2 — presentation

- HUD: objectives on a key press, or fading a few seconds after they change;
  the debug strip only under F12.
- Encounters staged so the Silbón is seen as a silhouette against light: a
  lamp, a doorway, a lightning flash.
- Caught screen: red/black treatment instead of the grey fog wash.
- Film grain and, for trailers, letterbox framing.

### Tier 3 — materials (CC0)

- CC0 PBR sets (ambientCG, Poly Haven) at 1K–2K with colour, normal,
  roughness and AO: rusted corrugated zinc, weathered planks, plaster, mud,
  bark, stone, burlap. Keep procedural textures where they already read
  well (grass, water ripples, notes, signs).
- Grime darkening toward the ground, edge wear, roughness that follows the
  rain (`wet.rs`), and decals (`clustered_decals.rs`) for mud, rust streaks
  and puddles.

### Tier 4 — geometry (Blender MCP glTF)

- A bevelled-box builder in `world::mesh`, used wherever `cuboid` is now;
  edges that catch highlights are the cheapest big win.
- Hero assets modelled in Blender via Blender MCP and exported as `.glb`:
  the Silbón, the truck, the ranch house, the ceiba, lanterns. Placement,
  collision, sight and aim stay in `geometry::Layout`; glTF supplies meshes
  only.
- A readable, specific Silbón silhouette (over-long limbs, the bone sack, the
  hat brim) for close encounters.

### Tier 5 — audio (CC0)

- CC0 foley for footsteps (dirt, grass, wood, water), wood creaks, rain on
  zinc and thunder, replacing the synthesized ones where they sound thin.
  The whistle phrases stay original and the perception rules (inverted,
  non-spatial) are unchanged.

## Gameplay and fear roadmap (proposal, 2026-09-29)

User request: better tasks (skill checks like Dead by Daylight's
generators), puzzles, more fear, and replayability with friends. Nothing
here is built yet; it is a proposal for the user to choose from. The
comparisons come from knowledge of the games, not from fresh research.

### What the hit co-op horror games do, and what to take

| Game | What makes it work | Take for El Silbón |
|---|---|---|
| Dead by Daylight | Long hold tasks split attention with random skill checks; a miss bangs loudly and tells the killer; teaming up is faster but riskier | Skill checks on the pump crank, truck repair and altar rite; a miss makes a noise he hears and loses progress |
| Phasmophobia | Every contract is an investigation: which ghost is it? Evidence, tools, a journal, then a correct call; clear hunt tells | "Which Silbón tonight": seeded variants with tells and counters, and a notebook for the evidence |
| Devour | Each objective completed makes the monster faster and angrier; the finale is a frenzy | Invert today's curve: each bundle laid to rest should *raise* his pressure (now `pressure_relief` lowers it) |
| Left 4 Dead (AI Director) | Build-up, peak, relax pacing tuned to the team's stress; placements vary per run | A director that spaces omens, hunts and quiet stretches, and seeds the placements |
| Lethal Company, R.E.P.O., Content Warning | Separation, proximity voice, comedy that turns to terror, spectating the dead | Tasks that force splitting up; the dead keep watching; proximity voice later (postponed in `AGENTS.md`) |
| The Outlast Trials | Puzzles under pressure, two-person mechanisms, variants of each trial, unlocks | Two-player mechanisms (winch and light, gate and crank); seeded puzzle answers |
| Alien: Isolation | The monster adapts: hide in lockers often and it checks lockers | He learns the party's habits: favourite grass, favourite lamp |

### A. Tasks with skill checks (DBD-style)

- **Pump crank** (exists as a hold): at random seeded moments a ring
  appears; press in the zone. Good gives normal progress, great gives a
  bonus, and a miss makes the windmill screech: a noise event at the tower,
  lost progress and a fear spike. Two cranking together is faster but gets
  more checks.
- **Truck**: turn ignition into a short repair chain (fuel, spark plug,
  choke) spread over the ranch and the extraction area, each a skill-check
  hold, then a timed ignition.
- **Altar rite**: laying the bones becomes a rhythm check.
- **Architecture**: the check schedule comes from `rng`, inside `sim`. The
  client sends the press with its sequence number; `net::session` judges it
  inside a latency window on its own clock. Tests cover a miss making noise,
  a great giving the bonus, determinism, and a late press being refused.

### B. Puzzles (answers seeded per run, so they never repeat)

- **Truck key padlock**: the code comes from clues (a date circled on the
  calendar, the brand on the cattle, the count of crosses at the shrine).
- **Power routing** at the windmill: limited capacity, so the team chooses
  which lamps to light (a safe path to the ceiba, or the caño).
- **The father's bones**: some bundles hold animal bones; notes say how to
  tell (a ring on a finger bone, a cracked skull). The wrong bones at the
  altar anger him.
- **Two-person mechanisms**: the caño bridge winch while a partner holds
  the beacon; the corral gate held while another drives the cattle.
- **Tureco the dog** (already named in note 1): find and free him. His
  growl points truthfully at the Silbón, against the inverted whistle, but
  his barking draws him. The rule belongs in `perception`.

### C. Fear

- **Flashlight battery**: limited charge, flicker when low, batteries to
  find, and the Silbón drawn to beams. The torch becomes a risk, which is
  exactly what makes darkness frightening.
- **Escalation**: every bundle laid to rest raises his pressure and how
  often he whistles; the truck warm-up is the frenzy.
- **Director and omens**: long quiet stretches; lamps dying one by one
  along a path; the cattle all turning to face one way; the lit window
  going dark; drag marks from the sack in the mud; the rain stopping
  before a hunt.
- **Stolen torch**: he sometimes carries a light at the edge of the fog,
  and in co-op you cannot tell it from a teammate's. It is a
  threat-behaviour state owned by the session.
- **Bones in the sack**: a caught player is carried off in his sack, not
  just downed; the team tracks the rattling and ambushes him with ají to
  free them.
- **Lightning reveals**: a strike shows him standing closer than you
  thought (a silhouette, not a jump scare), used rarely.
- **Hallucinations** at high fear, per player and cosmetic only: false
  whistles, phantom silhouettes. They are produced in `perception` and
  never change the truth.
- **Counting you can hear**: bones clicking one by one nearby when an ají
  stops him.

### D. Replayability and mastery

- Seeded placements: bundle sites, batteries, puzzle answers, which lamps
  work, weather.
- **Silbón variants** with evidence (examples: the Drunkard, drawn to noise
  and bottles; the Son, calmed by prayer; the Drover, who fears dogs and the
  whip). Each has tells and counters; naming him correctly opens a
  banishment ending at dawn.
- Several endings: the truck, the boat on the caño (needs fuel), or
  banishment.
- Loadouts and roles: lantern-bearer (steady light, slow), whip, radio,
  extra ají. Unlocked through play, with a grade per run (stealth,
  teamwork, speed) like DBD's emblems.
- Difficulty tiers and modifiers (moonless, flood, no dog); a daily seed;
  a campaign of several nights with the map changing between them.

### Suggested order

1. Flashlight battery and beams that draw him (small; gives Tier 1's
   darkness its point).
2. Skill checks on the pump, truck and altar, with noise on a miss.
3. Escalation per bundle, plus director pacing and omens.
4. Seeded placements and two puzzles (padlock code, power routing).
5. Bones in the sack (capture and rescue).
6. Silbón variants and evidence: the biggest replay hook, and the biggest
   job.

Every rule change lands in the pure layers (`sim`, `perception`,
`script`), validated by `net::session`, with behaviour tests and the
scripted route driver updated to play it.

## Previous handoff — 2026-09-29 (verification pass, committed as 6812742)

This section describes the tree as of commit 6812742. It supersedes the
earlier 2026-09-29 handoff (compile blocker, unverified driver) and the
historical milestones below, none of which prove the current build.

### Status

- **Gate green:** `cargo fmt --check`, `cargo check --locked`,
  `cargo test --locked` (78 passed, 1 opt-in ignored) and
  `cargo clippy --locked --all-targets -- -D warnings`.
- **Rendered solo `--smoke` and `--tour` pass** on the real renderer, and the
  **two-process headless UDP `--net-smoke` passes on both processes**.
  The **rendered two-process UDP pair passes** too (story and scene census
  on both peers).
- **Photos rebuilt:** the widened-fog overviews and the powered
  watchtower/beacon view now render as intended; ground-level frames keep
  play fog and the restrained rain.
- Since committed as 6812742 (the `src/ui.rs` rename was resolved there;
  the replacement is `src/ui/{mod,hud,map}.rs`). Do not restore the
  obsolete UI.

### What changed in this pass, and why

Compile blocker: `src/debug.rs` borrowed `frame.log` instead of moving it
(E0382). Formatting applied to the stable files.

Route driver (`src/script.rs`). Traces of the real session showed three
failure families, all in the driver, none in the rules:

1. **Planner:** routes onto the trail network considered only the four
   nearest patrol nodes, so from the lookout ramp foot a node across the caño
   crowded out the bridge foot and the altar walk went east round the whole
   map. It now considers seven.
2. **Grass deadlock:** a hiding place was chosen with a 0.6 m grass margin,
   but the walk stopped 0.3 m short, where the margin test failed, so the
   player stood uncrouched in the open. Where we stand is now judged by the
   rules' own test (body centre in grass, small allowance for the trailing
   body), the walk goes all the way, and a place picked as cover that does
   not hide us on arrival is given up. The "hunt in sight while in grass"
   timer only runs while actually in grass.
3. **Strategy:** after an averted warning he stays 13–30 m away with a
   3–6 s cooldown, so resuming the route at once re-warned endlessly (75
   warnings in one run; hundreds of seconds stuck by the corral and the
   altar). Measured over seeded sweeps, what works is:
   - **Commit, then vanish:** warned with cover (shadow or grass) within
     2.5 m of walk and him at least 9 m off, stay in view until the hunt
     begins, then step into cover. A hunt that loses sight for 2.5 s makes
     him sink and rise at the node farthest from every player; this one rule
     took the sweeps from roughly 50–80 % to 98–100 %.
   - **Cover round corners:** hiding places reachable by a short planned
     path (grid A*, within 18 m, at most six planned per choice), not only in
     a straight line (the stilt hut, the house, the corral).
   - **Lie low only in lamplight:** waiting out his patience in the dark
     just waits for a susto (it releases the crouch in grass); lanterns calm
     fear, so the driver lies low only where lit (sweeping the horizon for
     him, bounded to 30 s).
   - **Shadowed stand spots:** when he was seen lately, interaction spots in
     the shadow of cover from where he was come first.
   - **Truck warm-up:** the rules only need everyone standing in the zone
     once the engine is warm, so during warm-up the driver may hide up to
     12 m beyond the zone and walks back in when calm (the old leash kept it
     standing in the open inside the zone).
   Rejected after measurement: resuming at once without the tactics above,
   lying low everywhere, a torch-off stealth walk, and taking cover whenever
   he is in view within 10 m (network 139/150 vs 148/150 without it).

Client adapter (`src/net/mod.rs`), found by the rendered smoke, not the tests:

- **Solo restart trap (real gameplay bug):** the unfinished cutover only
  re-entered `Playing` on a new run when already playing, so restarting from
  the solo outcome screen (R or "Play again") or the pause menu left the game
  on the old screen with controls dead. A new run now enters `Playing` except
  from solo's opening briefing (its lobby).
- **Determinism:** in solo the body now eases on the simulated clock, as the
  session steps, so a fixed-step `--smoke` replays exactly what
  `cargo test` plays (the rendered log matches the test timeline to 0.1 s).
- R on the outcome screen was handled twice for hosts (the second, stale
  restart was refused and flashed "Action belongs to an old run."); only
  `app::pause_keys` handles it now.

Tests (`tests/session.rs`): the `Pilot` can miss seeded frames the way a
stalled client does (the session steps on the last input; the next frame's
`dt` includes the stall). `the_routes_hold_through_other_storms_and_missed_frames`
plays the solo and shared routes over four storms and stall patterns with
the original assertions. The opt-in
`the_routes_hold_over_many_storms` (`-- --ignored`) plays 150 of each and
requires 97 %.

Docs: `README.md` and `assets/SOURCES.md` rewritten for the current game
(five bundles, pump, truck, up to four players, all 37 WAVs, shaders,
material maps, meshes); `src/net/smoke.rs` story updated for the warm-up.

### Fresh evidence (this working tree, release build)

All under `screenshots/handoff-review/` (git-ignored). No desktop input,
focus or window manipulation was automated; the drivers are in-game.

| Run | Result |
|---|---|
| `--smoke` (`smoke-run/`) | `SMOKE PASS`, 508.0 s simulated, 31,185 frames: all five bundles, pump, truck, warm-up, escape (win); restart; warning, recovery, caught (failure); restart. Census unchanged after both restarts (1 camera, 1 directional, 3 spots, 32 points, 1 ambience, 1 threat, 274 meshes, 127 UI nodes). 11 captures. |
| `--tour` (`tour-run/`) | `SMOKE PASS`, 677.1 s simulated: all ten places at eye height, lookout ramp and vantage, overview, restart, then the full win and failure route; three restarts, census unchanged. 23 captures. |
| Headless UDP, two processes, 127.0.0.1:5197 (`udp-headless/`) | Both `NET SMOKE PASS`, exit 0, about 9 minutes of wall clock: delivery seen by the partner, marks exchanged and acknowledged, shared win with all bones, power and truck, restart, shared failure seen by both, restart, partner leaves carrying, host recovers and delivers the dropped bundle. |
| Rendered UDP, two windows, 127.0.0.1:5198 (`udp-rendered/`) | Both `NET SMOKE PASS` and `NET RENDER SMOKE PASS`, exit 0, about 10 minutes: the same story as headless on the real renderer. World census identical on both peers across runs 1–3 (1 camera, 3 world spots, 32 points, 1 moon, 1 ambience, 1 threat, 1 carried-bundle model) and the teammate avatar/torch roster matched the party throughout. Game-owned captures: 13 host, 10 client (`*/network/`), including both won and failed screens and the host's downed view. |
| `--photos` (`photos/`) | 47 frames, 0 captured before pipelines settled, exit 0, `MANIFEST.tsv` written. Reviewed: `00_overview` and `00_overview_late` (widened fog, labelled on image and in the manifest) now show the whole district's topology, the late one with lamps, lookout glow and truck lights; `17_watchtower_b` (powered mirror) shows the beacon fire lighting the cabin, roof and deck; `11_ranch_a` keeps play fog (105 m) and thin, sparse rain; `21_interior_rain` has no rain indoors; its bright lines are specular crests on the corrugated zinc, also present in the previous run and more prominent here because the tiling compositor gave the window a tall 1261×1390 shape (manifest records it), which widens the vertical field of view. No window was resized or moved by the driver. |
| `cargo test` seeded sweep (150 storms × solo and shared, missed frames) | solo 150/150, shared 148/150. The two misses: a susto freeze during a hunt near the fields' grass (seed 24) and a 300 s altar stall under repeated warnings (seed 143). |

The first rendered `--smoke` of this pass won run 1 and then stuck at the
spawn: that is how the restart trap above was found; the pass listed is the
rerun after the fix. The headless UDP pair ran on the release build made
just before the two `net::update`/`net_keys` fixes, which only touch the
rendered app's flow and keys (the headless driver never enters them); every
rendered run and the photos used the final build. Test-only changes came
after the release build.

### Implemented gameplay and client systems

- One connected authored district: the nine reference landmarks plus the
  extraction truck/bridge. The old encounter map and `--map` selector are gone.
- Solo and up-to-four-player sessions share the authoritative
  `net::session` rules; there is no separate offline truth path.
- Main loop: collect five bone bundles from the ranch, corral, fields, caño
  and lookout; hold interact to return each at the ceiba altar; crank the
  windmill pump; start the truck; survive its warm-up and gather all standing
  players in its boarding zone.
- Implemented crouch/grass concealment, sprint/stamina, noise/hearing, fear
  and susto, finite ají wards/counting, downed/crawl/revive/bleed, cattle
  disturbance, tower beacon distraction, pings, seven notes and seven pepper
  sites. These are implemented rules, not a claim of human balance testing.
- Host validates movement, targeting, ownership and shared outcomes.
  Whistle cues remain inverted and non-spatial, without leaking hidden
  threat distance through audio or captions.
- HUD has objectives, vitals, interaction/hold prompts, notes, downed view
  and a live map. Controls: WASD/mouse, E or left click interact/hold, F torch,
  Shift sprint, Ctrl/C crouch, G drop, Q ají, V/middle-click ping, M map,
  Esc menu, F12 screenshot. Host controls and lobby admission remain.
- Audio generator now produces 37 original WAVs, including rain, thunder,
  heartbeat, material footsteps, cattle, pump, engine, wards, counting,
  prayer, revival and pings. Subjective listening/comfort is still user-led.

### Implemented and visually reviewed art

The result is a stylized, original procedural interpretation, not a
photorealistic reproduction of the supplied references.

| Location | Current defining assets |
|---|---|
| Main Entry | Timber portal, skull, chained bell, open braced gates, signs and fencing |
| Ranch Cluster | Playable house/porch, distinct shed dressing, well/windlass, pickup, water tank, coop/hens and farm props |
| Water Tower | Plateau-grounded braced tower, tank, windmill, pump standpipe/handle, shed and trough |
| Shrine / Ceiba | Buttress-root tree, grounded offering table, cross/icons, candles, bottles, flowers, hanging cloth/rosaries and sign |
| Corral | Pens/gates, reinforced shelter, cattle, hay/water troughs and rope |
| Tall Grass Fields | Dense animated grass around clear trails, shelter/cart, supplies and broken wire fencing |
| Marsh Edge | Reed/cattail banks, lily pads, soft low mist, partly submerged fence remains and firm-bank route |
| Caño / Flooded Lowland | Channel, bridge/boardwalk with bed-reaching piles, stilt hut, moored boat and warning signs |
| Old Watch Tower | Walkable guarded ramp, braced tower/deck, cabin roof/back wall, banner, base shed and beacon |
| Extraction | Original canvas-bed truck, working engine/light presentation and bridge |

- Rain clarity was corrected at the shader: the old additive output ignored
  the intended fade and produced bright bars. It now uses premultiplied
  colour/alpha, 1,800 rather than 5,200 streaks, shorter streaks, strong
  near/far fading and masks for house, porch, sheds and lookout roof.
- Storm clock/shared-menu pause handling and lightning strike seeding were
  corrected. Ground-level fog/colour grading remain restrained.
- Wet water/puddles have procedural ripple shading; grass/reeds use
  root-pinned vertex wind. Side-by-side time-separated captures showed motion.
  Shader sources: `assets/shaders/{rain,grass,wet}.wgsl`.
- Normal/roughness maps cover wood, zinc, mud, bark, cloth and burlap.
  `world::mesh` now supplies a finite tangent basis so those normal maps
  actually affect rendering.
- El Silbón has a tall gaunt silhouette, broad frayed hat, hidden face,
  ragged coat, long hooked bony fingers and a burlap femur sack.
  Teammates have distinguishable clothing, hats, ruanas, belts, boots,
  torches and carried bags.
- Fixed the nearest-lamp pool retaining old lights, the beacon child's
  doubled world offset, missing note meshes, unsupported props and tower
  terrain contact. Decorative ruin placements now belong to the shared
  layout. A shrine note moved onto the table initially intercepted the altar
  target; its lateral placement is corrected and a targeting regression
  test passed before the latest driver changes.

### Remaining limits and unverified

- The driver is a scripted player, not a balance claim. About 1 % of seeded
  shared runs still fail (see the sweep row); the rendered and UDP shared
  runs are not deterministic, so a rerun can differ.
- Physical LAN, internet, Windows, three- or four-human sessions, human
  pacing and comfort, and subjective audio remain unverified.
- The solo downed caption still says "someone may reach you"
  (`src/ui/hud.rs`), which only applies in shared sessions; the outcome
  overlay is correct.
- Photo frames stage state (camera, powered lamps, photo-only teammates,
  downed view) and are presentation review only.
- Presentation notes from the captures: in shared sessions the connection
  banner overlaps the objective checklist (top-left), and on the win screen
  the truck objective still reads unchecked.

### Next steps

1. Review the whole working tree (it is large and uncommitted) and stage it
   deliberately, including the `src/ui.rs` rename; commit and push only on
   request, keeping the repository private.
2. User-led hands-on play: solo win and loss with R restarts, and a two- to
   four-player LAN session, with the checklist in `README.md`.
3. Optional driver work: the two residual sweep misses above; measure any
   change with the opt-in sweep, not a single seed.

### Resume commands

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --test session -- --ignored --nocapture   # 300-route sweep, ~30 s
cargo build --release --locked
./target/release/el_silbon --smoke  --shots screenshots/handoff-review/smoke-run
./target/release/el_silbon --tour   --shots screenshots/handoff-review/tour-run
./target/release/el_silbon --photos --shots screenshots/handoff-review
```

In separate terminals (omit `--headless` and give each its own `--shots`
for the rendered pair):

```sh
./target/release/el_silbon --host 127.0.0.1:5197 --net-smoke --headless
./target/release/el_silbon --join 127.0.0.1:5197 --net-smoke --headless
```


---

## Historical checkpoint: connected nine-landmark map


### Inspected references

`docs/art_references/` and other clearly named project reference directories
were absent. All eleven supplied files were opened directly from their explicit
attachment paths in `/home/senku/Downloads/`; no unrelated personal directory
was searched. They were not copied into runtime assets.

| Location / role | Inspected filename | Composition to translate, not reproduce literally |
|---|---|---|
| Overview | `El Silbon Map.png` | Entry southwest, ranch inland, water tower west, shrine central-north, corral/fields east, flooded north and distant lookout; interconnected trails |
| Ceiba style / Shrine | `el silbon ceiba tree.png` | Buttress roots, broad umbrella crown, rustic gate/sign kit, open grassland and restrained warm lamps |
| Main Entry | `el silbon main entry.png` | Heavy timber gateway, opened gates, signs, fence and roadside work props |
| Ranch Cluster | `el silbon ranch cluster.png` | Inhabited-looking porch house, shared yard, well, sheds and farm clutter |
| Water Tower | `el Silbon Water Tower.png` | Braced metal tank/windmill silhouette and small pump shed |
| Corral | `El Silbon Corral.png` | Broad pen aisles, gate frames, troughs and roofed livestock shelter |
| Shrine | `el silbon shrine.png` | Root-side offering clearing, cloth, candles and a fenced approach; ritual treatment remains fictional |
| Tall Grass Fields | `el silbon tall grass fields.png` | Worn track between varied-height grass, open shelter, distant windmill cue |
| Caño / Flooded Lowland | `el silbon cano flooded.png` | Connected water channels, planked crossing, stilt hut and static boat |
| Old Watch Tower | `El Silbon Old Watch Tower.png` | Braced timber lookout with roofed platform and base shed; steep reference ladder needs a walkable substitute |
| Marsh Edge | `el Silbon marsh edge.png` | Muddy edge path, broken fence and reed islands separating firm ground from water |

### Layout and integration decisions

- The existing house/interior/table remain at their authored origin. Normal
  launch selects `district`; `--map encounter` preserves the original scene.
  Named spawn, satchel, offering, escape, landmarks and threat anchors share
  `Layout`; there is no map-management framework or procedural rearrangement.
- Early loop: Entry → Ranch → Shrine → Corral → Entry. Outer alternatives:
  Ranch → Water Tower → Marsh Edge → Watch Tower → Caño → Shrine, and
  Corral → Tall Grass Fields → Entry, with a fields-to-caño cross-connection.
  The ceiba is now over 45 m from the house centre. Extraction remains at the
  Entry gateway, not the overview reference's separate kilometre-scale exit.
  The overview was compacted into roughly 162 × 142 m of playable bounds:
  Entry stays on the southern road to preserve the original spawn/house
  relationship, rather than copying the reference's southwest gate literally.
- Preserve chunky original geometry, cool moonlight, warm practical accents
  and open Llanos silhouettes. Replace the reference's dangerous deep-water
  travel with clearly bounded water and safe crossings; no swimming/climbing.
  (Reversed for the caño only in M1b item 7: it is waded, slow and loud.
  The marsh, the pond and the creek stay banks; still no swimming.)
- Movement remains planar in X/Z, with a shared authored surface-height query
  for the 6.4 m lookout deck, 26 m guarded ramp, bridge and stilt-hut platform.
  Camera/aim, host movement, remote avatars, threat presentation and dropped
  satchels read the same surface data. No jumping, climbing or stacked floors.
- Reused `src/world/{house,ceiba,land,props,mesh,texture}.rs`, `SpawnCtx`,
  merged `MeshBuilder` geometry and the shared `Palette`. District vegetation
  and new kits live in `world/district.rs`; the original scene's vegetation
  remains in `land.rs`. The Silbón mesh/model was not replaced.
- The original encounter retains its eight-anchor ring. The district uses a
  nine-anchor dry-land ring routed around new obstacles. Threat movement now
  uses the existing collision sweep rather than crossing physical blockers.
  This is not pathfinding: the threat can still stall against obstacles while
  creeping/returning off the ring; no new navigation director is claimed.
- Both peers explicitly exchange map ID in addition to seed and source
  fingerprint. The new district source is included in that fingerprint.
- Actual tuning remains **3.6 m/s walking**, **2.7 m/s carrying**, radius **0.3 m**.
  Pass 1 physical route/anchor/water/ramp checks passed (3 tests) before art.
  Tests walk every route both directions, not just a connectivity graph.
  First check caught a patrol crossing the ranch rail; its anchors were moved.

### Built locations

- Main Entry: timber portal, carved crest, open gates, roadside rail boundaries.
- Ranch: original interior/table, two sheds, yard rails, stone well, handcart,
  crates and barrels.
- Water Tower: tapered braced frame, lathed tank, windmill sails, service pipe,
  pump installation and shed.
- Corral: two wide pens around a central aisle, gate frames, troughs, hay and
  roofed shelter; clear routes on both sides.
- Shrine: original ceiba relocated to a more distant clearing, candles, cloth
  and stone offerings, eastern and western approaches.
- Fields: deliberately taller grass patches around open paths, broken rails,
  field shelter and abandoned cart.
- Caño: bounded channel, planked bridge, reeds, stilt hut and original static boat.
- Watchtower: timber lookout with guarded walking ramp, open deck, roof and
  base shed; not a decorative inaccessible ladder.
- Marsh: broad flooded boundary, broken fences, reed islands and firm-bank road.
- One shared palette and merged geometry; grass is batched spatially rather
  than an entity/material per blade. Six added unshadowed practical lights.
  No new dependencies, downloaded runtime assets or external generation services.

### Runtime review and verified visual revision

- First release tour passed all nine ground-level approaches, the lookout
  ascent/descent, victory/failure and three restart censuses. Captures retained
  in `screenshots/district-review/smoke/` as before-revision evidence.
- Actual review found a cropped shrine clearing, a weak head-on lookout
  silhouette, overly straight water edges, a distant/dark overview and low
  outdoor material readability. Revisions: lowered shrine framing; raised
  lookout deck 4.2 → 6.4 m and added a walked side viewpoint; submerged mud
  lobes/water colour variation; closer overview; normal district moonlight
  650 → 1150 lux, ambient 55 → 90 and fog visibility 82 → 105 m. The original
  map's lighting remains unchanged. Ground shots use normal game lighting,
  not an artificially bright capture mode.
- Reviewed final overview and **all nine location views**, plus ramp/vantage,
  in `screenshots/district-final/smoke/`. The root clearing/offerings and timber
  tower structure now read in the captures. The style remains deliberately
  simplified; rectangular lowland topology and flat dry ground are still visible
  from the debug overview, not claimed to match photographic reference detail.
- Final release tour passed in **332.3 simulated seconds**, including the
  objective smoke; its exploration portion reached restart at **162.6 s**.
  The district objective-only smoke passed in **169.8 s** (win at **109.8 s**).
  The original regression passed in **142.4 s** (win at **86.8 s**).
  These are scripted, accelerated measurements including intentional waits,
  not human completion-time estimates.
- Final district restarts retained `[1 camera, 1 moon, 1 spot, 8 points,
  1 ambience, 1 threat, 160 mesh entities, 74 UI nodes]`. The original retained
  its 3463 mesh entities and 2 point lights. No new panic/missing-asset errors.
- Final stable gate after the revision: formatting check, locked check,
  **35 tests passed**, strict all-target Clippy and release build passed.
  New behavior checks cover bidirectional physical routes at both movement
  speeds, map/anchor determinism, guarded water/ramp movement, the full district
  tour/objective/restart sequence, and host-authoritative elevated-carrier
  disconnect/drop/recovery.

### Measured route travel times

Fixed 60 Hz collision sweeps at the real 3.6 / 2.7 m/s movement speeds; both
directions verified. These exclude looking, interaction and human hesitation.
The ranch-to-shrine row starts at the house's back-door approach.

| Authored journey | Walking | Carrying |
|---|---:|---:|
| Entry → ranch | 4.72 s | 6.30 s |
| Ranch → corral, exposed | 10.02 s | 13.35 s |
| Corral → entry, fence track | 12.02 s | 16.00 s |
| Ranch back approach → shrine | 9.82 s | 13.08 s |
| Corral → shrine | 22.97 s | 30.60 s |
| Ranch → water tower | 14.10 s | 18.78 s |
| Water tower → marsh | 23.37 s | 31.15 s |
| Marsh → lookout, long outer bank road | 58.93 s | 78.55 s |
| Caño bridge | 6.67 s | 8.88 s |
| Shrine → caño | 11.45 s | 15.25 s |
| Shrine western escape | 18.63 s | 24.83 s |
| Corral → fields | 5.95 s | 7.92 s |
| Fields → entry | 15.08 s | 20.10 s |
| Fields → lookout, narrow grass/reed track | 25.02 s | 33.35 s |
| Corral → caño | 12.52 s | 16.68 s |
| Lookout ramp approach → deck | 9.43 s | 12.58 s |

The long bank road is optional; the caño crossing is the shorter connection.
Fences and grass are visual screening, not AI sight cover; walls and trunks
provide actual line-of-sight breaks. The long bank road's interest versus
commuting time remains a useful human playtest question.

### Comparable performance observations

Same release executable, seed 1997, Vulkan/RTX 4070 (610.57.04), Ryzen 7 9800X3D,
30.4 GiB RAM. Sequential, no-vsync objective smoke runs requested 1600×900;
the compositor produced 1261×688 captures. Warm-up/capture waits are excluded
from recorded wall-clock Update intervals. Map routes differ where the shrine
moved; these are not controlled GPU timings or a certified FPS benchmark.

| Scene | Samples | Mean interval | Median | P95 | Mesh entities |
|---|---:|---:|---:|---:|---:|
| Original regression | 8543 | 2.196 ms | 2.185 ms | 2.433 ms | 3463 |
| District objective route | 10186 | 2.199 ms | 2.199 ms | 2.437 ms | 160 |
| District full tour | 19943 | 2.094 ms | 2.142 ms | 2.385 ms | 160 |

The observed objective-run intervals are essentially unchanged; merged grass
patches reduce entity count despite the larger scene. No Windows/hardware
portability performance measurement was made.

### Final multiplayer integration

- Two separate release headless UDP processes on loopback, `--map district`,
  passed shared win/failure, two host restarts, carrier disconnect and item
  recovery. Host/client wall times were 59.97 / 57.46 s, not network latency.
- Two separate **rendered** release processes on loopback also printed
  `NET RENDER SMOKE PASS`: same shared outcomes/restarts/disconnect/recovery,
  with scene census `[1 camera, 1 spot, 8 points, 1 moon, 1 ambience, 1 threat,
  1 table item, 1 returned item, 1 carried item]` preserved. Remote avatar
  census matched connected peers. Both applications exited successfully.
- Real UDP mismatch probes rejected `district` versus `encounter` and seed
  1997 versus 42 before admission. Evidence:
  `screenshots/district-final/network-mismatch.log`.
- Genuine peer captures: `screenshots/district-network/host/network/` and
  `screenshots/district-network/client/network/`. Reviewed connection, carrying,
  and victory views. The compositor tiled the client to 621×688: transient
  toast/HUD text overlaps at that narrow size, although outcome buttons remain
  usable. No UI redesign was made for this map task.
- This is same-machine UDP/render evidence, not a physical-LAN or internet
  test. Elevated-carrier recovery is covered by the host behavior test; no
  claim of a two-human lookout playtest or subjective audio review.

### Original-map baseline verification (before implementation)

- Initial Git status: clean `main`, tracking `origin/main`.
- Passed: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`
  (**30 passed**), `cargo clippy --locked --all-targets -- -D warnings`,
  and `cargo build --release --locked`.
- Release offline runtime passed with
  `./target/release/el_silbon --smoke --seed 1997 --size 1600x900 --shots screenshots/map-preflight-baseline`.
  It exercised satchel pickup, warning → hunt → lost track, restitution,
  escape, failure and two restarts. Both restart censuses matched: 1 camera,
  1 directional light, 1 spotlight, 2 point lights, 1 ambience, 1 threat,
  3463 mesh entities and 74 UI nodes. No panic or missing-asset error appeared.
- Fresh genuine captures: `screenshots/map-preflight-baseline/smoke/`
  (`01_roadside.png` through `09_caught.png`, using the existing shot names).
  Reviewed roadside, approach, interior, hollow, ceiba-landmark and win views.
  The requested 1600×900 window produced **1261×688** captures under the
  compositor. No bright debug lighting was used.
- Visual baseline: warm interior materials and satchel are readable; the
  roadside is sparse and the ceiba canopy is very dark against the sky.
  Preserve the house's material treatment; improve landmark silhouette and
  approach cues in the map pass rather than interpreting this as completed art.
- Script timing: pickup at 12.1 s, restitution at 70.0 s, escape at 86.8 s;
  full win/failure/restart route **142.5 simulated seconds**, reported 9511
  frames. The process took **23.56 wall seconds** including warm-up/captures.
  These are accelerated debug-route measurements, not human travel times,
  steady-state FPS or expanded-map performance.
- Renderer/hardware: Vulkan, NVIDIA GeForce RTX 4070, driver 610.57.04,
  12282 MiB VRAM; AMD Ryzen 7 9800X3D, 30.4 GiB system RAM.
  No FPS/frame-time benchmark is claimed. `/usr/bin/time` is not installed;
  the initial timing-wrapper command exited 127 without starting the game.
  The direct retry above passed without installing anything.
- Two separate release processes using real loopback UDP also passed:
  `./target/release/el_silbon --host 127.0.0.1:5167 --net-smoke --headless`
  and the matching `--join 127.0.0.1:5167 --net-smoke --headless`.
  Host reported shared win/failure, two restarts, carrier disconnect and item
  recovery; client reported shared win/failure/restarts and leaving while
  carrying. Process wall times were 51.61 s host / 49.09 s client, not latency
  or throughput measurements. No physical-LAN/internet or fresh rendered
  multiplayer test was performed in this preflight.

### Launch and current checkpoint

```sh
cargo run --release --locked                           # expanded district
cargo run --release --locked -- --map encounter        # original regression
cargo run --release --locked -- --tour --shots screenshots/district-final
```

`--tour` walks all nine locations at normal eye height before exercising the
full objective/failure/restart smoke. One explicitly debug-only overview uses
a raised camera and longer fog visibility; it is not ground-readability proof.
The build, offline rendered tour/regression, and both headless and rendered
real-UDP multiplayer gates passed. Evidence and remaining manual limits are above.
No commit, push, publication, package installation or desktop input automation.

Manual checklist (no desktop automation): explore both loops; ascend/descend
the lookout; approach water away from crossings; take/return the satchel and
escape through Entry; pause/resume, adjust volume, get caught and restart.
F12 views: corral's two exits, shrine's western approach, lookout facing ranch,
and field-path forks. Audio comfort, human movement feel, Windows runtime,
physical-LAN and internet play remain unverified.
Highest-value playtest: can two people navigate between named landmarks at
night without HUD directions, and does the outer bank road earn its travel time?

### Changed source/document files

- `AGENTS.md`, `README.md`, `docs/PROGRESS.md`, `assets/SOURCES.md`.
- `src/geometry.rs`, new `src/geometry/district.rs`.
- `src/app.rs`, `src/control.rs`, `src/player.rs`, `src/debug.rs`,
  `src/script.rs`, `src/sim.rs`.
- `src/net/{mod,protocol,session,smoke,transport}.rs`.
- `src/world/{mod,land,texture,silbon}.rs`, new `src/world/district.rs`.
- New `tests/district.rs`; generated evidence under the named screenshot folders.

## Previous milestone: two-player shared encounter

- Implemented the shared encounter without changing the map or adding voice.
- Standalone Renet 2.0 + renet_netcode 2.0; Bevy remains exactly 0.19.1.
  Direct loopback/private-LAN hosting only, development authentication, no relay.
- Host owns input-driven movement/collision, stable player IDs, the singleton
  satchel, all interactions/objectives and one threat. Snapshots are 20 Hz;
  host simulation is 60 Hz. Remote players are temporary amber/blue meshes.
- Lobby-only admission with gameplay/configuration fingerprint and seed check.
  Host Enter starts, F6/R restarts, F10 ends; client F10 leaves. Esc is a local
  menu, not a global pause. No reconnect or host migration.
- Dropped/captured/disconnected carriers leave a recoverable satchel. Captured
  players wait for restart; the survivor continues. All remaining players
  caught means failure. After restitution, an active player must return to
  the road from outside its goal to produce shared victory.
- Run epochs and input/action sequences reject old-run/replayed requests.
  Disconnect removes the avatar, ownership, pending effects and input.
- Whistles are listener-specific categorical cues generated host-side.
  Hidden transforms are omitted; a present enemy is sent only inside a broad
  view cone with unblocked authored sight. The host is trusted; this does not
  hide previously seen positions or prevent inference from cues.
- After verification, the user explicitly authorized committing this checkpoint
  and pushing it to the existing private `acvdoandrew/el-silbon` repository.
  Future pushes still require an explicit request.

### Verification

- Original baseline: 19 tests, formatting, check and Clippy passed.
- Final build gate: `cargo fmt --check`, `cargo check --locked`,
  `cargo test --locked` (**30 passed**), Clippy `--all-targets -- -D warnings`,
  and `cargo build --release --locked` passed.
- Added authority tests cover conflicting ownership, invalid reach/occlusion,
  drop/transfer, restitution/escape, capture, disconnect, stale movement,
  restart epochs and absence of hidden enemy transforms.
  Distinct near/far listeners receive different categorical whistle variants;
  manifestation considers both players, and a teammate already waiting on
  the road cannot skip the carrier's return journey.
- Two **real UDP processes on this machine**, headless: complete win,
  shared failure, two restarts, carrier leave and host recovery passed.
  Separate real UDP probes rejected late joining and a mismatched seed.
  The final release repeated the complete headless two-process route successfully.
  An abrupt host-termination probe produced an explicit snapshot timeout.
  The smoke driver was corrected to wait on stale snapshots rather than
  misreport connection loss as a walking/path failure.
- Two **rendered processes on this machine**, Vulkan/RTX 4070, dev and release builds:
  both reported `NET RENDER SMOKE PASS`. Restarts retained the census
  `[1 camera, 1 spot, 2 points, 1 moon, 1 ambience, 1 threat, 1 table item,
  1 returned item, 1 first-person item]`; remote avatar counts matched peers.
  Latest reviewed images in `screenshots/network-checkpoint/network/` show both players, carrying,
  listener warning, individual capture and shared win/failure.
  The final menu fix was exercised in the dev build: session status no longer
  covers outcome buttons. Both processes again passed the full route and census.
  The final release executable was also rebuilt successfully from those sources.
- Final release offline `--smoke` passed win/failure/two restarts; camera,
  lights, ambience, threat, mesh and UI census stayed unchanged. Its nine
  genuine captures are in `screenshots/offline-regression/smoke/`.
- No Omarchy input/window automation. No two-physical-machine LAN test and
  no separate-internet-connection test. Human movement feel, simultaneous
  input and audio comfort remain user-led checks.

The sections below preserve the initial offline encounter's history.

## Decisions

- **Engine**: Bevy `=0.19.1`, Rust 2024; features `3d`, `ui`, `audio`, `wav`,
  `keyboard`, `mouse` (no physics crate).
- **Truth vs presentation**: `sim` (objectives, threat states
  Dormant/Stalking/Warning/Hunting/Resolved, exposure, timers) is pure and
  deterministic; the ECS feeds it one `TickInput` per frame.
- **Shared geometry**: player circle vs axis-aligned wall/fence/furniture
  rectangles and circles, sub-stepped (≤ 5 cm) so nothing tunnels. Line of
  sight uses the same shapes: solid wall pieces and the ceiba trunk block;
  windows, doorways, fences and furniture do not.
- **Threat**: authored eight-anchor ring, no navmesh. Manifests at the anchor
  farthest from the player (≥ 30 m, tested for the whole paddock). Warns only
  with line of sight within 22 m; hunts only while seeing the player (never
  moves toward a hidden player). Losing track → sinks, rises far away,
  cooldown. Hunt speed < carry speed.
- **Primary escape**: break line of sight with the house's solid walls (the
  ceiba trunk also works). Taught by the note, briefing, legend and hints.
- **Perception inversion**: `perception::seeming_closeness` maps true
  distance 5→42 m onto seeming 0→1 (truly near seems faint). Three recorded
  timbres (loud/middling/faint), gentle gains, mono, non-spatial, scheduled
  per state. Captions describe the seeming impression only.
- **Visuals**: procedural meshes and textures from the fixed seed; moonlight
  (cascaded shadows) + two warm kerosene lights (shadowed) + flashlight
  (shadowed); exponential-squared fog matched to the sky horizon; light bloom.
- **Fonts**: Noto Sans/Serif (OFL) because the Bevy default font is ASCII-only
  and the game uses Spanish text.
- **Testing preference**: user now owns hands-on desktop tests. No further
  Omarchy keyboard/pointer/window automation without renewed permission.

## Completed (written)

- Full loop: briefing → play → pause/settings → win or caught → restart.
- FPS look/move with collision, flashlight, crosshair targeting with reach and
  occlusion, satchel carry with speed penalty, hold-to-return restitution with
  pause-on-interrupt, road escape.
- Threat stalking/warning/hunting/recovery/resolution with rising/sinking
  presence and a procedurally animated model.
- Audio: ambience loop, three whistle timbres, bones, restitution, caught and
  dawn stingers (all generated by `tools/gen_audio.py`).
- HUD, captions, teaching hints, exposure vignette, note overlay, menus.
- Debug: F12 screenshots; `--smoke` scripted route with screenshots and a
  restart entity census.
- Tests: geometry (walls/door/sills/fence, sight, aim), sim (progression,
  invalid transitions, restitution pause, safe manifestation, warn→hunt→catch,
  warning averted, lose track and withdraw, no approach through walls,
  restart reset), perception (inversion, scheduling), control (targeting,
  walking), CLI parsing, and the full route replayed headlessly.

## Verified

- Audio generator ran; outputs are deterministic across runs (hash match) and
  are 16-bit mono 44.1 kHz WAV (ffprobe). Loudness checked with ffmpeg
  `volumedetect`.
- Integrator, first candidate (after `cargo fmt` and one `f32` annotation in
  `world/house.rs`): `cargo fmt --check`, `cargo check --locked`,
  `cargo test --locked` (19 tests), `cargo clippy --locked --all-targets -- -D
  warnings` passed. Release `--smoke` passed: win in 85.4 s simulated, full
  win/restart/caught/restart 141.0 s, both restart censuses unchanged, eight
  screenshots written. The compositor resized the 1600×900 request to 1261×688.
- Screenshot review of that run: the house exterior and interior read well
  (warm lantern vs moonlit grass, detailed textures). Problems found: the
  roadside capture was blank (taken before pipelines compiled), stars were
  large white squares, the ceiba was a thin dark column hidden behind the HUD,
  and the Silbón was a black stick at 20 m.
- Release runtime used Vulkan on NVIDIA RTX 4070 / driver 610.57.04,
  Ryzen 7 9800X3D, 30.4 GiB RAM, Linux 7.2.3. No FPS benchmark is claimed.
  The smoke used accelerated fixed-step simulation, not a timed human playtest.
- Real game-only PipeWire output was captured: 10 seconds, stereo output from
  the mono source mix, nonzero samples, peak 5272/32768. This proves output,
  not subjective sound quality or headphone comfort.
- Before the user's testing-preference change, native input exercised
  mouse look, forward motion, flashlight toggle, pause/resume, volume
  80→70→80%, sensitivity 1.0→1.1→1.0, and captions On→Off→On.
  Genuine screenshots: `screenshots/verification/smoke/` (first candidate),
  `screenshots/manual/` (native-input checks). The initial blank roadside shot
  is retained as evidence of the capture-readiness defect, not a successful scene.

## Revision 2 checkpoint

- Smoke waits for the renderer: a render-world probe reports pipelines still
  compiling; the route starts after 30 idle frames and 2 s real warm-up, and
  every capture freezes simulated time and holds the script until the view is
  compiled and the file is saved.
- Moon moved high into the south-south-west (behind the road): the house
  front, the ceiba's trunk and crown and a figure in the yard are moonlit from
  the player's side against the dark north. Moonlight 520 → 650 lux.
- Sky: mostly overcast with moon-silvered cloud edges; about 400 faint,
  pixel-sized stars only in cloud gaps.
- Ceiba: height 25 m, crown radius 17 m, seven longer limbs, eighteen fill
  clumps; brighter foliage texture.
- Silbón: pale grime-streaked shirt (new texture), paler straw hat with a
  0.72 m brim, lighter skin; silhouette unchanged.
- HUD: compact objective panel (370 px, more transparent); controls moved to a
  bottom-right line.
- Screenshot framing: roadside and approach aim between house and ceiba; a new
  `07_ceiba_landmark` looks back at the whole tree on the way out (captures now
  run 01…09).
- Built successfully with `cargo build --release --locked`; formatting, locked
  checking, 19 headless tests and Clippy with `-D warnings` passed.
  The headless route still covers successful escape, getting caught and restart.
- The open game process predates this revision. Close it and relaunch with
  `cargo run --release --locked` to load the updated presentation.

## Original-encounter historical risks

- The revised scene has now rendered in the two-player process checks above.
  Human visual/pacing assessment remains user-led; no new art was introduced
  for this networking milestone beyond the explicitly temporary player avatars.
- Encounter length (target 3–5 minutes) is unmeasured with human players; the
  optimized scripted win takes 85 s.
- Invisible walls at the road ends (x = ±44 m) are hidden only by fog.
- Hammock, door leaf, gate and shutters are visual; the hammock footprint
  collides, the others are placed where they cannot be walked through.

## Next

1. User-led district playtest: navigate by landmark names without HUD guidance,
   compare the short caño crossing with the long bank road, and test meeting
   at the lookout with a real teammate.
2. Evaluate only observed map readability/collision/pacing issues from that playtest.
3. This map milestone is the stopping point. Do not automatically start voice,
   new missions, a navigation rewrite or another multiplayer milestone.

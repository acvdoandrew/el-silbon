# Working on El Silbón

## Ownership and workflow

- Current milestone: one connected rural horror district with the nine reference
  landmarks plus extraction, playable solo or with up to four players.
  Proximity voice and unrelated networking work are postponed.
- Read the current handoff at the top of `docs/PROGRESS.md` before editing.
  Preserve the unfinished verification work; do not treat older historical
  success reports as proof of the current working tree.
- Preserve the existing art, offline mode and working multiplayer. Commit and
  push only when explicitly requested; keep the GitHub repository private.

- One writer per explicitly assigned file set; disjoint coding and art slices
  may run concurrently. One integrator owns builds, runtime checks and screenshots.
  Do not edit files another writer is changing.
- Mid-change, do not run formatters or project-wide builds; the integrator
  runs the gate once a change set is stable.
- Record decisions, verified vs unverified status and next steps in
  `docs/PROGRESS.md`; record every asset in `assets/SOURCES.md`.
- User preference: hands-on desktop verification is user-led. Do not automate
  Omarchy cursor, keyboard, focus, or window manipulation without renewed permission.
  Supply exact run commands, a short checklist, and requested F12 views instead.
  Exception: trailer work has standing permission (see **Trailer** below).

## Build and test gate

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo run --release --locked -- --smoke # optional real-renderer route; coordinate with user
```

`Cargo.lock` is included. Bevy is pinned `=0.19.1`; read version-specific
sources/examples (`~/.cargo/registry/src/*/bevy-0.19.1/examples`) before using
an API — do not mix older Bevy idioms.

## Architecture conventions

- **Pure truth, no ECS**: `tuning`, `geometry`, `sim`, `perception`, `control`,
  `script`, `rng`, `noise`, `body`, `storm`, `skill` (skill checks),
  `director` (omens), `pacing` (El Respiro, the pacing director), `mix`
  (volume curve, buses, glide), `radio` (the dial and the numbers hour),
  `display`
  (brightness and contrast as the camera's grade) and `lang` (the English /
  Spanish tables; a local choice, never on the wire or in the fingerprint)
  stay headless and unit-tested.
  ECS modules (`app`, `player`, `encounter`, `audio`, `ui`,
  `world`, `debug`) adapt them.
- **One layout**: every coordinate (walls, openings, fences, ceiba, props,
  route anchors, trails) comes from `geometry::Layout`. Collision, line of
  sight, aiming and visuals all read it. Never duplicate a number in a builder.
- **Perception boundary**: only `perception` turns the Silbón's true distance
  into a cue: the whistle, inverted; Tureco's growl and bark, truthful but
  only up close (`dog_senses`). Audio and captions consume `WhistlePhrase`
  and events only; never read threat position/distance there, never
  spatialize the whistle. Omens and phantoms are placed near the listener's
  own eye and never say where he is.
- **Truth validates claims**: `net::session` is authoritative in both solo and
  multiplayer for movement, aim/occlusion, ownership, one threat and outcomes.
  `net::protocol` carries stable player IDs, run epochs and sequence numbers,
  never Bevy entity IDs or hidden AI state. `net::transport` is independent of
  rendering; `net::mod` adapts snapshots to local presentation.
- **Shared sessions**: keep Renet standalone and Bevy pinned. Admit only in the
  initial lobby; require matching gameplay fingerprint/seed. Never make network
  menus freeze host simulation or let a client locally reset shared state.
- **Network verification**: run separate host/client executables with
  `--net-smoke --headless` for real UDP, or `--net-smoke` for rendered census
  and screenshots. These are in-game drivers, not Omarchy input automation.
  Keep unit, same-machine, physical-LAN and internet evidence distinct.
- **Determinism**: all randomness is `rng::Rng` from the seed. The solo smoke
  uses fixed simulated time; two-process network smoke uses real-time endpoints.
- **Tests** cover behaviour: objective progression and invalid transitions,
  whistle inversion, warning/hunt/line-of-sight recovery, restart reset,
  collision/sight/aim, and the full scripted route. Do not add tests that pin
  wording or wiring.
- Debug features stay clearly labelled (`--smoke`, F12) and never set the
  outcome directly.
- Meshes, textures and audio are original; bundled Noto fonts are third-party OFL.
  (This governs what ships in the game, not trailer media — see **Trailer**.)
  Audio: regenerate with `python3 tools/gen_audio.py` (deterministic).
  Textures and meshes are generated in Rust at startup.

## Trailer (standing permission, granted 2026-09-29)

A trailer/teaser session may own the whole job end to end without asking:
script and narration, voice, gameplay capture, edit, music/SFX mix and render.

- **ElevenLabs is approved** for narration, voice design and trailer-only SFX
  or music. Pick or design voices freely and spend credits as needed. Only the
  narration script and trailer text go out; never upload source code, `.env`
  or secrets.
- **Capture**: the session may build and launch the game (release or debug),
  record the screen or game window (e.g. `wf-recorder`, `gpu-screen-recorder`,
  `ffmpeg`), and position/focus/fullscreen the game window for recording. It may
  send keyboard/mouse input to the game window only, to play or stage shots.
  Prefer in-game drivers (`--smoke`, `--net-smoke`, a new scripted `--trailer`
  camera/route mode) for repeatable shots. Don't touch other apps or windows.
- **It is its own integrator**: it may run builds and the gate while working;
  just don't edit files another writer currently owns. A `--trailer` mode is a
  debug feature: clearly labelled, off by default, never sets outcomes.
- **Game rules don't bind the edit**: perception/whistle and "original assets"
  rules apply to game code. In the trailer, cut, re-mix, layer narration and
  add effects however works best, while keeping the game's footage and lore honest.
- **Files**: work in `trailer/` (scripts, narration text, edit project, notes);
  keep raw captures and renders out of git (`trailer/out/`, ignored), and list
  ElevenLabs voices/generations in `trailer/SOURCES.md`. Commit only when asked.

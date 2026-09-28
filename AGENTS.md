# Working on El Silbón

## Ownership and workflow

- Current multiplayer milestone: Astra implements directly, with no subagents,
  advisors, model routing changes or billing/authentication changes. The user
  explicitly superseded the earlier Opus-only implementation ownership.
  Preserve the existing art and offline mode. Commit and push only when the
  user explicitly requests it; keep the GitHub repository private.

- One writer at a time edits gameplay code, geometry, materials, audio and
  asset scripts; an integrator owns builds, runtime verification and
  screenshots. Do not edit files another agent is changing.
- Mid-change, do not run formatters or project-wide builds; the integrator
  runs the gate once a change set is stable.
- Record decisions, verified vs unverified status and next steps in
  `docs/PROGRESS.md`; record every asset in `assets/SOURCES.md`.
- User preference: hands-on desktop verification is user-led. Do not automate
  Omarchy cursor, keyboard, focus, or window manipulation without renewed permission.
  Supply exact run commands, a short checklist, and requested F12 views instead.

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

- **Pure truth, no ECS**: `tuning`, `geometry`, `sim`, `perception`,
  `control`, `script`, `rng` depend only on `bevy::math`. Keep them headless
  and unit-tested. ECS modules (`app`, `player`, `encounter`, `audio`, `ui`,
  `world`, `debug`) adapt them.
- **One layout**: every coordinate (walls, openings, fences, ceiba, props,
  route anchors, trails) comes from `geometry::Layout`. Collision, line of
  sight, aiming and visuals all read it. Never duplicate a number in a builder.
- **Perception boundary**: only `perception` turns the Silbón's true distance
  into a cue, inverted. Audio and captions consume `WhistlePhrase` only; never
  read threat position/distance there, never spatialize the whistle.
- **Truth validates claims**: offline `sim` checks objective/reach; multiplayer
  `net::session` owns movement, aim/occlusion, ownership, one threat and outcomes.
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
- **Determinism**: all randomness is `rng::Rng` from the seed. The smoke route
  uses fixed simulated time.
- **Tests** cover behaviour: objective progression and invalid transitions,
  whistle inversion, warning/hunt/line-of-sight recovery, restart reset,
  collision/sight/aim, and the full scripted route. Do not add tests that pin
  wording or wiring.
- Debug features stay clearly labelled (`--smoke`, F12) and never set the
  outcome directly.
- Meshes, textures and audio are original; bundled Noto fonts are third-party OFL.
  Audio: regenerate with `python3 tools/gen_audio.py` (deterministic).
  Textures and meshes are generated in Rust at startup.

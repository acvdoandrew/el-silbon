# El Silbón — The Return

A compact first-person folk-horror encounter set on a fictional stretch of the
Colombian–Venezuelan Llanos, late 1990s. This repository is the **first local
playable test** of a planned cooperative game (4–8 players, reference six); it
is single-player only for now and makes no multiplayer claims.

Your truck died on the road. Beyond a sagging fence stand an abandoned house
with a lamp still burning and an enormous ceiba. Take the bone satchel from the
house, carry it to the ceiba's roots and hold there until the bones are home,
then walk back to the road. Something tall, under a broad hat, whistles out on
the llano — and the whistle lies: **when it sounds loud and close he is far
away; when it sounds thin and far away he is near.** Solid walls (and the
ceiba's trunk) break his line of sight. There is no combat.

The folklore here is fiction inspired by the legend; nothing depicts real
ritual practice.

## Requirements

- Rust ≥ 1.95 (edition 2024). Bevy is pinned to [`0.19.1`](https://github.com/bevyengine/bevy/releases/tag/v0.19.1); its [tagged manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml) specifies the MSRV.
- Linux: a Vulkan GPU, ALSA/PipeWire, `libudev`, Wayland or X11 headers
  (the usual Bevy Linux dependencies).
- Python 3 (standard library only) — only to regenerate the audio.

## Run

```sh
cargo run --release --locked                 # normal game
cargo run --release --locked -- --help       # launch options
cargo test --locked                          # headless gameplay tests
```

Launch options:

| Option | Meaning |
|---|---|
| `--seed N` | world-scatter and whistle-jitter seed (default `1997`) |
| `--size WxH` | window size (default `1600x900`) |
| `--shots DIR` | screenshot folder (default `./screenshots`) |
| `--smoke` | **debug**: scripted deterministic route, see below |

Assets load from this crate's `assets/` even when the binary is started from
`target/`; set `BEVY_ASSET_ROOT` to override.

## Controls

| Input | Action |
|---|---|
| Mouse | look (after clicking **Begin**, the cursor is captured) |
| W A S D / arrows | move |
| E or left click | interact; **hold** at the ceiba's hollow |
| F | flashlight on/off |
| Esc | pause (frees the cursor) / resume |
| F12 | screenshot of the game window |
| R | restart (on the win/lose screen) |

The pause menu adjusts master volume, mouse sensitivity and whistle captions,
and can restart from the road. Losing window focus pauses automatically.
There is no camera bob or shake.

## The encounter

1. Roadside: walk through the gate, across the yard and into the house.
2. Take the bone satchel from the table (reach and line of sight are
   enforced; the note beside it explains the whistle). Carrying slows you.
3. He rises out of the grass at the anchor farthest from you — never close.
   He walks an authored route around the property, lurks, and creeps in when
   he can see you.
4. If he sees you within range he **warns** (the whistle turns faint, the
   screen edge darkens). Break line of sight — solid walls, not windows or
   doorways — to avert it. Otherwise he **hunts**: exposure builds while he
   sees you. Staying out of sight long enough makes him lose track, sink and
   rise again far away.
5. Hold E at the hollow in the ceiba's roots for three seconds (progress
   pauses if you let go or look away). He leaves.
6. Walk back out to the road: win. Being caught ends the run; both outcomes
   offer restart.

Intended length is 3–5 minutes; **this is not yet measured with human
players** (see `docs/PROGRESS.md`).

## Tuning

Every gameplay number lives in [`src/tuning.rs`](src/tuning.rs): speeds, reach,
warning and hunt timings, exposure, perception distances and gains, audio
levels. The authored layout (house, openings, fences, ceiba, props, route
anchors) is in [`src/geometry.rs`](src/geometry.rs) and drives both collision
and visuals.

## Debug smoke route (`--smoke`)

`cargo run --release --locked -- --smoke` plays the real game with a deterministic
scripted player (fixed 60 Hz simulated time, fixed seed, cursor never
grabbed): roadside → house → satchel → waits exposed for the warning and hunt →
hides inside → he loses track → ceiba restitution → road (win) → restart →
satchel → stands exposed until caught → restart. It drives the same intent →
look/collision → crosshair targeting → truth path as a player and never writes
the outcome. The route only starts once the renderer reports no pipelines left
to compile (plus a two-second real warm-up), and each screenshot waits — with
simulated time frozen — until what is on screen is fully compiled and saved.
Screenshots land in `screenshots/smoke/`: `01_roadside`, `02_approach`,
`03_interior`, `04_warning`, `05_hunting`, `06_ceiba_hollow`,
`07_ceiba_landmark` (the whole tree, looking back on the way out), `08_win`,
`09_caught`. After each restart it compares entity counts (cameras, lights,
ambience loop, the Silbón, meshes, UI nodes) with the first run. It logs
`SMOKE PASS` and exits 0, or `SMOKE FAIL: …` and exits 1. `cargo test --locked`
replays the same route headlessly.

## Scope of this build

In: one authored night encounter, full win/lose/restart loop, pause/settings,
captions, procedural world and textures, original synthesized audio.
Not in: networking or co-op, voice, save data, controller support, content
beyond this encounter. Next milestone: two real clients (see
`docs/PROGRESS.md`).

## Hands-on checks

Desktop testing is user-led; no automated Omarchy pointer/keyboard control.
After rebuilding, close the old game window and relaunch to see the changes.

- Walk the whole route; note confusing landmarks, collision snags, and total time.
- Take the satchel, wait for a warning in the yard, then hide behind a solid wall.
  Check that the warning and recovery feel fair.
- Check whistle audibility/comfort and whether the distance inversion is understandable.
- Complete restitution and escape; on a second run remain exposed, then restart with R.
- Check pause/resume, mouse capture, volume, sensitivity, and caption controls.
- F12 saves game-only images in `screenshots/` (or the `--shots` folder).
  Useful views: roadside, house table, whole ceiba, first threat sighting.

Verified results and remaining visual/playtest checks: [`docs/PROGRESS.md`](docs/PROGRESS.md).

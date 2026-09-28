# El Silbón — The Return

A compact first-person folk-horror encounter set on a fictional stretch of the
Colombian–Venezuelan Llanos, late 1990s. The current checkpoint supports an
offline encounter and a two-player, player-hosted development session.
The eventual cooperative target remains 4–8 players (reference six).

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

## Two-player session

Build once: `cargo build --release --locked`. Start two separate applications:

```sh
# Host: plays as the amber player.
./target/release/el_silbon --host 127.0.0.1:5000 --shots screenshots/host
# Client: plays as the blue player.
./target/release/el_silbon --join 127.0.0.1:5000 --shots screenshots/client
```

Wait until the banner reports **2/2 connected**, then the **host presses Enter**.
Both retain WASD, mouse look, E/hold E and F. **G** puts the satchel down.
**F6/R** restarts everyone (host only); **F10** ends the session for the host
or leaves for the client. Closing the application also disconnects.
Esc/focus loss opens a **local menu only**: the shared world continues.

For a trusted LAN, substitute the host's explicit private LAN IP in **both**
commands. Wildcard/public host addresses are rejected. No firewall/router
changes, port forwarding, discovery, relay, Steam login or internet hosting
are provided. Both peers must use matching gameplay sources, Cargo.lock and
`--seed` (default 1997); mismatches are rejected before admission.

**Policy:** join before the first start only. Restart retains connected
players and does not reopen admission. After a disconnection, the host may
continue alone; launch a new host session to admit a replacement player.

The host validates movement/collision, aim, interaction reach, item ownership,
objective progression and one threat simulation. Exactly one player carries
the satchel. Dropping, capture or disconnection leaves it recoverable.
A caught player is inactive until restart; the other may continue. All
remaining players caught means shared failure. After restitution, an active
survivor returning from outside the road goal produces shared victory.

### Networking choice and boundaries

[Renet 2.0](https://docs.rs/renet/2.0.0/renet/) and
[renet_netcode 2.0](https://docs.rs/renet_netcode/2.0.0/renet_netcode/)
are standalone Rust libraries, independent of the installed Bevy 0.19.1.
This avoids an engine upgrade or replication framework. Renet's
[separate Steam transport](https://github.com/lucaspoffo/renet/tree/master/renet_steam)
is a future integration option, **not implemented here**.

Host simulation runs at 60 Hz; snapshots/input send at 20 Hz. Remote positions
are smoothed; there is no prediction/rollback or production anti-cheat.
Actions use reliable ordered messages, run epochs and sequence numbers.
The direct-address netcode setup is **unauthenticated development mode**:
trusted loopback/LAN only, not a secure public service.

Whistles are generated host-side per listener and sent as categorical timbre
and pitch variation, not true distance or an invertible continuous distance.
Enemy transforms are included only when present, within a broad view cone
and with unblocked authored line of sight. The host necessarily knows the
truth; clients can remember previously visible positions or infer information
from cues. This is not a claim of comprehensive hidden-information security.

## Controls

| Input | Action |
|---|---|
| Mouse | look; offline Begin or multiplayer entry captures cursor |
| W A S D / arrows | move |
| E or left click | interact; **hold** at the ceiba's hollow |
| F | flashlight on/off |
| Esc | offline pause / multiplayer local menu; releases cursor |
| F12 | screenshot of the game window |
| R / F6 | multiplayer host restart; offline R on the outcome screen |
| G | multiplayer: put the satchel down |
| F10 | multiplayer: host ends session / client leaves |

The menu adjusts master volume, mouse sensitivity and whistle captions.
Offline pause freezes the encounter; multiplayer menus/focus loss stop only
local input while the shared encounter continues. There is no bob or shake.

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
6. Walk back out to the road: win. Offline capture ends the run; in multiplayer
   a captured player waits for restart while their teammate may continue.

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

In: one authored encounter, offline and two-player direct-address play,
shared satchel/objectives/threat/outcomes, restart and disconnect recovery,
pause/settings, captions, existing procedural visuals and synthesized audio.
Not in: voice, assisted carrying, rescue/revival, Steam authentication,
matchmaking, reconnect/host migration, map expansion or internet relay.
Next milestone: **basic proximity voice**, after the two-player checklist.

## Hands-on checks

Desktop testing is user-led; no automated Omarchy pointer/keyboard control.
After rebuilding, close the old game window and relaunch to see the changes.

### Two-player checklist

1. Connect both applications before host Enter. Confirm amber/blue remote
   players move and collide with the existing walls/doors; each controls only
   its own camera. Try joining a third application after start: expect rejection.
2. Both aim at the satchel and press E together: exactly one should carry it.
   Carrier presses G; the other aims down and picks it up. Try E from too far
   away or through a wall: ownership must not change.
3. Compare whistle impressions from separated positions. Hide during a warning
   and confirm the warning clears without an unavoidable capture.
4. Return the satchel at the ceiba, then have an active player return to the
   road. Both must receive victory; host F6/R resets both.
5. Let the carrier get caught. Confirm they stop moving, the satchel drops and
   the survivor can recover it. Both caught must produce shared failure.
6. Restart, let the client carry, then client F10/close. Confirm its avatar
   disappears and the host can recover/finish. End/close the host separately:
   the client must report disconnection rather than continue a private AI run.
7. Repeat restart and menus: no extra players, lights, satchels or audio loops.

### Reproducible two-process smoke

These commands use **real UDP endpoints in separate processes** and scripted
game input, not desktop automation or a mocked network. Run in two terminals:

```sh
./target/release/el_silbon --host 127.0.0.1:5000 --net-smoke --headless
./target/release/el_silbon --join 127.0.0.1:5000 --net-smoke --headless
```

The headless route uses accelerated fixed steps and claims no rendering proof.
It performs a shared win, a both-caught run, two host restarts, and a carrier
disconnect followed by host pickup. Each process prints `NET SMOKE PASS`
and exits; failures return a nonzero exit code.

Omit `--headless` to exercise the real renderer, scene census and game-owned
screenshots in `screenshots/network/`. This opens two windows but never grabs
the pointer or injects Omarchy input. Use separate `--shots` folders if desired.

### Offline checklist

- Walk the whole route; note confusing landmarks, collision snags, and total time.
- Take the satchel, wait for a warning in the yard, then hide behind a solid wall.
  Check that the warning and recovery feel fair.
- Check whistle audibility/comfort and whether the distance inversion is understandable.
- Complete restitution and escape; on a second run remain exposed, then restart with R.
- Check pause/resume, mouse capture, volume, sensitivity, and caption controls.
- F12 saves game-only images in `screenshots/` (or the `--shots` folder).
  Useful views: roadside, house table, whole ceiba, first threat sighting.

Verified results and remaining visual/playtest checks: [`docs/PROGRESS.md`](docs/PROGRESS.md).

# El Silbón — The Return

A first-person folk-horror game set in one connected, authored district of
the fictional Colombian–Venezuelan Llanos, 1998: nine landmarks plus the
extraction road. Play alone, or with up to four players in a player-hosted
session on a trusted LAN. There is no combat.

Your truck died at the river bridge. Ahead lie a hacienda with a lamp still
burning, a windmill that could bring the power back, and five bundles of
bones taken from El Silbón's sack. He is out in the rain, he wants them
back, and he listens to everything. The whistle lies: **loud means he is
far, thin means he is near.**

The folklore here is fiction inspired by the legend; nothing depicts real
ritual practice.

> Development status, verification evidence and known limits live in
> [`docs/PROGRESS.md`](docs/PROGRESS.md) (read its current handoff first).

## Requirements

- Rust ≥ 1.95 (edition 2024). Bevy is pinned to
  [`0.19.1`](https://github.com/bevyengine/bevy/releases/tag/v0.19.1); its
  [tagged manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml)
  specifies the MSRV.
- Linux: a Vulkan GPU, ALSA/PipeWire, `libudev`, Wayland or X11 headers (the
  usual Bevy Linux dependencies).
- Python 3 (standard library only), only to regenerate the audio.

## Run

```sh
cargo run --release --locked                 # solo
cargo run --release --locked -- --help       # launch options
cargo test --locked                          # headless rules and route tests
```

| Option | Meaning |
|---|---|
| `--seed N` | world-scatter, storm and whistle-jitter seed (default `1997`) |
| `--size WxH` | window size (default `1600x900`) |
| `--shots DIR` | folder for F12 screenshots and the debug drivers (default `./screenshots`) |
| `--host ADDR` | host a shared session, e.g. `127.0.0.1:5000` (loopback or private LAN only) |
| `--join ADDR` | join a host before its run starts |
| `--smoke` | **debug**: deterministic scripted run (win, restart, caught, restart), then exit |
| `--tour` | **debug**: walk to every landmark and up the lookout, then the full `--smoke` route |
| `--photos` | **debug**: presentation review captures from searched viewpoints, then exit |
| `--net-smoke` | **debug**: scripted two-process shared-run route (with `--host`/`--join`) |
| `--headless` | with `--net-smoke`: real networking without graphics |

Assets load from this crate's `assets/` even when the binary is started from
`target/`; set `BEVY_ASSET_ROOT` to override.

## The run

1. **Five bone bundles** lie at the ranch house table, the corral, the tall
   grass fields, the caño's stilt hut and the old watchtower's deck. Taking
   the first one wakes him; he rises far from everyone.
2. Carry bundles to the **altar in the ceiba's roots** and hold interact to
   lay each down. Carrying slows you, makes you louder and raises the night's
   pressure; every bundle home eases it.
3. With all five home, **crank the windmill pump** (the hold adds up across
   players) to restore power: the lamps come on.
4. **Start the truck** at the extraction road. The engine's roar draws him
   while it warms up; once it is warm, every standing player must be in its
   boarding zone to escape. If everyone is down or dead, the run fails.

Surviving him:

- He warns when he sees you within range (the screen edge darkens, the
  whistle thins). Break line of sight — walls, trunks, the truck — or crouch
  in tall grass beyond a few metres to avert it. Otherwise he **hunts**:
  exposure builds while he sees you. Stay out of his sight long enough and
  he loses track, sinks and rises again far away.
- Everything makes a sound: crouching sneaks, running is heard far away,
  wading and planks are loud; rain and thunder mask footsteps. He
  investigates what he hears.
- **Fear** grows in the dark and alone and with every warning; lamplight,
  company and prayer at the altar calm it. At its peak a **susto** freezes
  you for a moment and you cry out.
- **Ají** peppers (seven sites, up to three carried) scatter into a ward he
  will not cross; the first touch makes him stop and count bones.
- Caught players go **down**: they crawl and bleed out unless a teammate
  holds interact beside them to revive them. Solo, going down ends the run.
- Spooked cattle bellow; the lit watchtower beacon draws him away for a
  while. Seven notes around the district explain the rules in fiction.

## Controls

| Input | Action |
|---|---|
| Mouse | look (Begin, or joining, captures the cursor) |
| W A S D / arrows | move |
| Shift | run (stamina) |
| Ctrl / C | crouch |
| E or left click | use; **hold** at the altar, pump, ignition, beacon or a downed teammate |
| F | flashlight on/off |
| G | put a bundle down |
| Q | scatter ají |
| V or middle click | mark a spot for the party |
| M | map |
| Esc | pause (solo) / local menu (shared); releases the cursor |
| F12 | screenshot of the game window |
| R | outcome screen: play again (host) |
| Enter / F6 / F10 | shared session: host starts / host restarts / host ends or client leaves |

The menu adjusts master volume, mouse sensitivity and whistle captions. Solo
pause freezes the run; in a shared session menus and focus loss stop only
local input while the world continues.

## Shared sessions (up to four players)

Build once (`cargo build --release --locked`), then start separate
applications:

```sh
./target/release/el_silbon --host 127.0.0.1:5000 --shots screenshots/host
./target/release/el_silbon --join 127.0.0.1:5000 --shots screenshots/p2
```

Players join in the lobby before the first start; the host presses **Enter**
to start. Restarts keep the connected players and do not reopen admission.
A player who leaves drops what they carry where they stood; the rest can
continue. For a trusted LAN use the host's explicit private address in every
command. Wildcard and public addresses are rejected; there is no discovery,
relay, port forwarding, Steam login or internet hosting. Every peer must run
matching gameplay sources, `Cargo.lock` and `--seed`; mismatches are refused
before admission.

The host's session is the single authority, in solo too: it validates
movement and collision, aim, reach and occlusion, item ownership, objective
progress and the one threat, and decides every outcome. It steps at 60 Hz;
snapshots and input travel at 20 Hz. Remote players are smoothed; there is no
prediction or rollback. Actions carry stable player IDs, run epochs and
sequence numbers, never engine entity IDs or hidden AI state.

Networking uses [Renet 2.0](https://docs.rs/renet/2.0.0/renet/) and
[renet_netcode 2.0](https://docs.rs/renet_netcode/2.0.0/renet_netcode/),
standalone of the pinned Bevy. The direct-address setup is
**unauthenticated development mode**: trusted loopback/LAN only.

Whistles are chosen on the host per listener and sent as categorical cues,
never a distance. His transform reaches a client only while he is present,
within a broad view cone and with unblocked line of sight. The host
necessarily knows the truth; clients may remember or infer. This is not a
claim of hidden-information security.

## Layout and tuning

Every gameplay number lives in [`src/tuning.rs`](src/tuning.rs). Every
coordinate — walls, openings, fences, the ceiba, props, sites, trails and
his patrol graph — comes from one `geometry::Layout`
([`src/geometry.rs`](src/geometry.rs),
[`src/geometry/district.rs`](src/geometry/district.rs)); collision, line of
sight, aiming and visuals all read it.
`cargo run --release --locked --example layout_svg -- target/layout.svg`
draws it top-down.

He walks a patrol graph over the trails and steps straight toward what he
pursues, sliding along blockers; it is not a navmesh. Sight is 2D line of
sight against the layout's blockers; the lookout deck sees the whole llano,
and is not a safe place.

## Debug drivers

All drivers play through the same input → session → snapshot path a player
uses, never teleport the player, never write truth or outcomes, and never
touch the desktop cursor, keyboard, focus or windows. They are labelled
debug features.

- **`--smoke`** plays a scripted player on fixed 60 Hz simulated time:
  every bundle to the altar, the pump, the truck, the escape (win); restart;
  then a second night of warning, hunt and recovery, then standing in the
  open until caught (failure); restart. Screenshots wait, with simulated time
  frozen, until the renderer has compiled what is on screen. After each
  restart it compares entity counts (cameras, lights, ambience loop, the
  Silbón, meshes, UI nodes) with the first run. It prints `SMOKE PASS` and
  exits 0, or `SMOKE FAIL: …` and exits 1. Captures:
  `<shots>/smoke/` (`01_roadside` … `10_escaped`, `07_warning`,
  `08_caught`).
- **`--tour`** first walks to every landmark at eye height and up the
  lookout ramp, then restarts and plays the full smoke route. Its single
  `00_overview` frame raises the camera and widens the fog range for
  topology only.
- **`--photos`** places the camera at viewpoints searched from the layout and
  writes labelled frames and a `MANIFEST.tsv` to `<shots>/photos/`. Staged
  state (camera, powered lamps, photo-only teammates, the downed view) is a
  presentation mirror, written on each image and in the manifest; it is
  never gameplay proof. Only the `00_overview*` frames widen the fog range.
- **`--net-smoke`** runs the shared route in two real processes over UDP:
  a delivery and exchanged marks, a shared win with all bones, power and
  the truck, a restart, a shared failure, a second restart, then a carrier
  who disconnects while carrying and a host who recovers and delivers the
  dropped bundle. Both processes run on the wall clock at 60 Hz, so a full
  run takes real minutes:

  ```sh
  ./target/release/el_silbon --host 127.0.0.1:5197 --net-smoke --headless
  ./target/release/el_silbon --join 127.0.0.1:5197 --net-smoke --headless
  ```

  Each prints `NET SMOKE PASS …` and exits 0, or exits non-zero with the
  failing step and state. Omit `--headless` (and give each process its own
  `--shots`) to exercise the real renderer, the scene census and game-owned
  screenshots in `<shots>/network/`.

`cargo test --locked` replays the solo, tour and shared routes headlessly
against the real session, with each client's snapshots at the rate its
endpoint delivers them.

## Scope

In: the connected district, solo and up-to-four-player direct-address
sessions, shared objectives, threat and outcomes, stealth, noise, fear,
ají, downing and revival, pings, restart and disconnect recovery,
pause/settings, captions, procedural visuals and synthesized audio.

Not in: proximity voice (postponed), swimming, climbing, boat physics,
Steam authentication, matchmaking, reconnection, host migration or internet
relay.

## Hands-on checks

Desktop testing is user-led; nothing automates the desktop pointer or
keyboard. After rebuilding, close the old window and relaunch.

- Walk the whole route solo; note confusing landmarks, collision snags and
  total time. Try a warning in the open, then hide behind a wall or crouch in
  tall grass; check that recovery feels fair.
- Listen to the whistle, rain and thunder for comfort and whether the
  inverted distance reads.
- Win once and lose once; restart with R from the outcome screen.
- With two to four applications: join before Enter, check each controls only
  its own view, share a bundle hand-off (G then E), revive a downed teammate,
  and leave while carrying — the bundle must stay recoverable.
- F12 saves game-only images in `screenshots/` (or the `--shots` folder).

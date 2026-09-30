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
cargo run --release --locked                 # the title screen and its menus
cargo run --release --locked -- --play       # straight into a solo night
cargo run --release --locked -- --help       # launch options
cargo test --locked                          # headless rules and route tests
```

| Option | Meaning |
|---|---|
| `--seed N` | the night: bundle hiding places, padlock code, which of him walks, storm (solo without it: a new night every launch; debug routes and shared sessions: `1997`) |
| `--night N` | `gentle`, `normal` (default) or `hard`; a joiner takes the host's night automatically |
| `--play` | skip the title screen and go straight into a solo night |
| `--size WxH` | window size (default `1600x900`) |
| `--windowed` | play in a window this launch and leave the saved choice alone (a plain launch follows Settings > Video > Display mode, fullscreen at first; the debug drivers always use a window) |
| `--shots DIR` | folder for F12 screenshots and the debug drivers (default `./screenshots`) |
| `--host ADDR` | host a shared session, e.g. `127.0.0.1:5000` (loopback or private LAN only) |
| `--join ADDR` | join a host before its run starts |
| `--survivor S` | who the others see you as: `llanero`, `coplera`, `encargado` or `muchacho` (default: the menu's saved choice) |
| `--smoke` | **debug**: deterministic scripted run (win, restart, caught, restart), then exit |
| `--tour` | **debug**: walk to every landmark and up the lookout, then the full `--smoke` route |
| `--photos` | **debug**: presentation review captures from searched viewpoints, then exit |
| `--menu-shots` | **debug**: the title screen and every menu page, one capture each, then exit |
| `--trailer` | **debug**: render the teaser's staged shots frame by frame at 1920x1080, then exit |
| `--net-smoke` | **debug**: scripted two-process shared-run route (with `--host`/`--join`) |
| `--headless` | with `--net-smoke`: real networking without graphics |

Assets load from this crate's `assets/` even when the binary is started from
`target/`; set `BEVY_ASSET_ROOT` to override.

A plain launch opens on the **title screen**: the camera drifts past the
hacienda's landmarks in the rain while a cuatro plays alone; now and then a
whistle, and sometimes, when lightning strikes, a tall shape. Its menus
(keyboard or mouse): **Play** (difficulty and night number), **Play with
friends** (host on this machine's LAN address or join one), **Journal** (the
pages of the tale found over every night, readable again, and the tally of
nights), **Settings** (**Video**: fullscreen or a window, field of view,
brightness and contrast with a calibration page of three hats (offered
once, on the first title screen), head bob; **Audio**: master, music,
ambience and effects volume, whistle captions; **Controls**: sensitivity,
invert Y), **How to
play**, **Credits** and **Quit**. The game opens fullscreen until the
player picks a window. Esc in a night opens the pause menu (resume,
settings, journal, restart, leave to the title). Settings, pages and the
tally are saved to `$XDG_DATA_HOME/el-silbon/profile.json` (else
`~/.local/share/el-silbon/profile.json`); the debug drivers never touch it.

## The run

1. **Five bone bundles** lie at the ranch house table, the corral, the tall
   grass fields, the caño's stilt hut and the old watchtower (all but the
   table one change hiding place from night to night). Taking the first
   one wakes him; he rises far from everyone.
2. Carry bundles to the **altar in the ceiba's roots** and hold interact to
   lay each down, keeping the rhythm (below). Carrying slows you and makes
   you louder, and **every bundle laid to rest angers him more**.
3. With all five home, **crank the windmill pump** (the hold adds up across
   players) to restore power. The old dynamo carries only two of three lamp
   lines; the **panel beside the pump** switches which (the bridge starts
   dark).
4. The truck key is padlocked in a **key box** at the windmill. Its three
   numbers change every night. A tag on the padlock names a frequency; the
   **shelf radio** in the house (press E to turn its dial, shared by
   everyone, and it squeals) reads the numbers out on that stop, forever:
   each digit as that many short pips, a zero as one long tone. Lightning
   can swallow a digit; the next round carries it. Near the radio the pips
   also show as dots.
5. **Start the truck** at the extraction road. The engine's roar draws him
   while it warms up; once it is warm, every standing player must be in its
   boarding zone to escape.
6. **Or name him.** He comes back in one of three ways each night (the
   drunkard's return, the son himself, the drover), each with its own signs
   and temper. With every bone at the ceiba, press **N** at the altar and
   name the right one: he is laid to rest. Name the wrong one and he comes,
   furious.

If everyone is down or dead, the run fails.

Surviving him:

- He warns when he sees you within range. Break line of sight (walls,
  trunks, the truck) or crouch in tall grass beyond a few metres to avert
  it. Otherwise he **hunts**: exposure builds while he sees you. Lose him
  and he sinks and rises far away; avert him three times running and he
  tires of waiting over you.
- Everything makes a sound: crouching sneaks, running is heard far away,
  wading and planks are loud; rain and thunder mask footsteps.
- Your **torch** runs down (spare batteries lie around the landmarks) and
  a lit beam he can see draws him from far off. The dark is safer, and
  worse.
- **Skill checks**: while laying bones, cranking or turning the engine
  over, a chime warns and a needle sweeps; press **Space** in the marked
  zone. A miss screeches across the llano, costs work and frightens you.
- **Fear** grows in the dark and alone; lamplight, company and prayer calm
  it. At its peak a **susto** freezes you. Badly frightened, you may hear
  whistles that are not there and glimpse him where he is not.
- **Ají** peppers scatter into a ward he will not cross; the first touch
  makes him stop and count bones.
- **Tureco**, the ranch dog, is tied behind the house. Untie him and he
  follows you; he growls when the Silbón is truly near and, when he has the
  courage, barks him off.
- Caught with friends still standing, you go **into his sack**: he carries
  you off, and only ají in his path (or Tureco's bark) makes him drop you
  before you are gone. Once out, a teammate can help you up. Solo, being
  caught ends the run.
- With friends, whoever is aboard the ready truck can **drive off without
  the others** (X); those left behind get their own ending. At the end of
  every night the outcome card hands out **awards** (screamed the most,
  butterfingers, first to fall, guardian angel, rode in his sack…).
- When he catches you there is a breath of silence first; he comes from
  one of three ways, sometimes from the edge of your sight. Badly
  frightened, you may hear footsteps behind you that are nobody's, and in
  company see a friend's mark where nobody marked.
- Twenty pages lie around the district (letters, ledgers, a copla, the
  parish register, a telegram, the radio on the shelf…): read them all
  over your nights to piece the tale together.

## Controls

| Input | Action |
|---|---|
| Mouse | look (Begin, or joining, captures the cursor) |
| W A S D / arrows | move |
| Shift | run (stamina) |
| Ctrl / C | crouch |
| E or left click | use; **hold** at the altar, pump, ignition, beacon, a downed teammate or Tureco |
| Space | skill check (press as the needle crosses the zone) |
| 1 / 2 / 3, Enter | at the key box: turn the dials (Shift turns back), try; at the ceiba: choose a name, name him |
| N | at the ceiba with every bone home: name which of him walks tonight |
| F | flashlight on/off |
| G | put a bundle down |
| Q | scatter ají |
| X | with friends, aboard the ready truck: drive off now, leaving whoever is not aboard |
| V or middle click | mark a spot for the party; down, a hoarse cry for help from where you lie (he may hear it too) |
| A / D, arrows or left / right click | gone for the night with friends: watch the previous / next friend on their feet (you see and hear only what they do) |
| M | map |
| Esc | pause (solo) / local menu (shared); releases the cursor |
| F12 | screenshot of the game window |
| R | outcome screen: play again (host) |
| Enter / F6 / F10 | shared session: host starts / host restarts / host ends or client leaves |
| F7 | shared session lobby: be the next free survivor |

The menu adjusts the master, music, ambience and effects volume (even in
decibels, with a moment of that sound as a slider moves; the whistle and the
catch follow the master alone), mouse sensitivity and whistle captions. Solo
pause freezes the run; in a shared session menus and focus loss stop only
local input while the world continues.

## Shared sessions (up to four players)

Build once (`cargo build --release --locked`), then start separate
applications:

```sh
./target/release/el_silbon --host 127.0.0.1:5000 --shots screenshots/host
./target/release/el_silbon --join 127.0.0.1:5000 --shots screenshots/p2
```

Each player is one of four survivors — El Llanero, La Coplera, El Encargado
or El Muchacho — chosen on the **With friends** page (saved in the profile)
or with `--survivor`. The host grants each wish unless someone already has
that survivor, then gives the first one free; in the lobby **F7** steps to the
next free one. It only changes how the others see you and the name in the
party list.

Players join in the lobby before the first start; the host presses **Enter**
to start. Restarts keep the connected players and do not reopen admission.
A player who leaves drops what they carry where they stood; the rest can
continue. For a trusted LAN use the host's explicit private address in every
command. Friends elsewhere can join over a private VPN such as Tailscale
(its 100.64.x.x addresses are accepted; the host gives its Tailscale
address). Wildcard and public addresses are rejected; there is no discovery,
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

### Sound

The llano's own things are placed: machines, frogs at the water, the
windmill, the dynamo once powered, Tureco, teammates' footsteps, marks,
the altar, the key box, the thunder (from the bolt you saw) and an omen's
clatter behind you pan to their direction and fade with distance and
behind walls (`sound_near`, `sound_far`, `sound_occluded` in
`tuning.rs`). The whistle, the stingers and his signs are never placed:
where he is must not leak through sound, and the whistle lies about
distance by design.

### The whistle

`python3 tools/whistle_lab.py` plays the whistle as the game mixes it, with
no build: the gains, the rain and insect beds, the duck under a whistle and
the distance inversion are read from `src/tuning.rs`, the sounds are the
game's files. Modes: `ladder` (loud, middling, faint; `--all-takes`), `ab`
(loud and faint back to back), `approach` (he walks in from 60 m to 3 m and
you hear which one each distance gives) and `night` (minutes of him
stalking, on the game's irregular cadence). `--regen` rebuilds the whistle
files from `tools/gen_audio.py` first; `--dry`, `--tense`, `--rain`,
`--master` (the master slider; `--volume` is the same), `--ambience` (the
bed's slider only) and `--out FILE` shape or save the mix. Levels are the
`gain_loud/mid/faint` tuning values (the files are loudness-matched) under
the volume curve read from `src/mix.rs`, so the default master plays the
mix 12 dB below full scale, as the game does.

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
  `PHOTOS_ONLY=lunge` (any part of a frame name) shoots just those frames.
- **`--menu-shots`** opens on the title screen, shows it at four landmark
  stops and then every menu page, begins a solo night (briefing, play,
  pause, pause settings) and leaves it for the title again, saving one
  capture per step to `<shots>/menu/`. It prints `MENU SHOTS OK` and exits
  0. Presentation review only.
- **`--trailer`** renders the teaser's fifteen staged shots (`src/trailer.rs`)
  at 30 fps on a fixed 1/30 s clock into `<shots>/trailer/<NN_shot>/%05d.png`
  (about 1,800 frames, 3.3 GB). `TRAILER_ONLY=catch` renders only matching
  shots; `TRAILER_STILLS=1` writes one still per shot to
  `<shots>/trailer_stills/` for framing. Everything staged (camera, his
  pose, lightning, teammates) is presentation only. The cut, with
  narration, score, sound, cards and subtitles, is assembled by
  `python3 tools/trailer/edit.py --frames <shots>/trailer --stems DIR --out FILE`
  (ffmpeg); the stems (ElevenLabs narration, score and hits) are not in git.
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
the title screen and menus, a saved profile (settings, journal, tally),
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
  inverted distance reads (`python3 tools/whistle_lab.py ab` and `approach`
  first, then in play).
- Win once and lose once; restart with R from the outcome screen.
- With two to four applications: join before Enter, check each controls only
  its own view, share a bundle hand-off (G then E), revive a downed teammate,
  and leave while carrying — the bundle must stay recoverable.
- F12 saves game-only images in `screenshots/` (or the `--shots` folder).

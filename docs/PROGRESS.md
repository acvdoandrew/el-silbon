# Progress

## Current handoff — 2026-09-29 (verification pass)

This section describes the current working tree. It supersedes the earlier
2026-09-29 handoff (compile blocker, unverified driver) and the historical
milestones below, none of which prove the current build.

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
- Nothing was committed or pushed. The index still records
  `src/ui.rs -> src/ui/legacy_ui.rs` while the legacy file is deleted in the
  working tree and the replacement is `src/ui/{mod,hud,map}.rs`; review that
  deliberately when staging. Do not restore the obsolete UI.

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

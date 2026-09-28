# Progress

## Current milestone: two-player shared encounter

- Implemented directly by Astra under the user's ownership override; no
  subagents, model/config/auth changes, new map content or voice.
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
- **Model ownership verified**: implementation worker `anthropic/claude-opus-5-5`,
  xhigh, no fallback; read-only API helper `openai-codex/gpt-6-luna`, max.
  No global routing/effort/auth changes. A session fallback-disable request was
  unanswered, so settings stayed unchanged; the writer was instructed to stop
  on any model substitution.
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

## Unresolved / risks

- The revised scene has now rendered in the two-player process checks above.
  Human visual/pacing assessment remains user-led; no new art was introduced
  for this networking milestone beyond the explicitly temporary player avatars.
- Encounter length (target 3–5 minutes) is unmeasured with human players; the
  optimized scripted win takes 85 s.
- Invisible walls at the road ends (x = ±44 m) are hidden only by fog.
- Hammock, door leaf, gate and shutters are visual; the hammock footprint
  collides, the others are placed where they cannot be walked through.

## Next

1. User-led two-player checklist in README: movement, competing pickup,
   transfer, restitution/escape, capture, carrier disconnect and host departure.
2. Fix only observed foundation issues before additional systems or content.
3. **Next milestone: basic proximity voice feasibility and implementation.**
   Not started here. The eventual target remains 4–8 players.

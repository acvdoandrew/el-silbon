# Progress

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
  sooner but each gets checks.
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

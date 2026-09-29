use bevy::math::{Vec2, Vec3};
use el_silbon::{
    body::Status,
    control::{Intent, Pose, Target},
    geometry::{Layout, ground},
    net::{
        Wire, controls_live, follow_body, mirror,
        protocol::{Action, HOST, Input, SEND_INTERVAL, STEP, ServerMessage, Snapshot},
        session::Session,
        status_of,
    },
    script::{Observation, RouteScript, crosshair},
    sim::{Encounter, Event, Outcome, Presence, Relic, ThreatState},
    tuning::Tuning,
};
use std::collections::BTreeMap;

/// A scripted table of players around one session: real inputs, real steps.
struct Rig {
    s: Session,
    l: Layout,
    t: Tuning,
    input_seq: BTreeMap<u64, u64>,
    action_seq: BTreeMap<u64, u64>,
    /// Keep the threat from ever warning (map/route tests, not AI tests).
    calm: bool,
}

impl Rig {
    fn new(players: u64) -> Self {
        let l = Layout::new();
        let t = Tuning::default();
        let mut s = Session::new(&l, &t);
        for id in 2..=players {
            s.add_player(id, &l, &t).unwrap();
        }
        let mut rig = Self {
            s,
            l,
            t,
            input_seq: BTreeMap::new(),
            action_seq: BTreeMap::new(),
            calm: false,
        };
        rig.act(HOST, Action::Start).unwrap();
        rig
    }

    fn act(&mut self, id: u64, action: Action) -> Result<(), String> {
        let seq = self.action_seq.entry(id).or_insert(0);
        *seq += 1;
        let run = self.s.run;
        let seq = *seq;
        self.s.command(id, run, seq, action, &self.l, &self.t)
    }

    fn pose(&self, id: u64) -> Pose {
        self.s.players[&id].pose
    }

    fn put(&mut self, id: u64, pos: Vec2) {
        self.s.players.get_mut(&id).unwrap().pose.pos = pos;
    }

    fn look(&mut self, id: u64, at: Vec3) {
        let (l, t) = (&self.l, &self.t);
        self.s.players.get_mut(&id).unwrap().pose.look_at(t, l, at);
    }

    /// Stand at `pos` looking at `at`.
    fn stand(&mut self, id: u64, pos: Vec2, at: Vec3) {
        self.put(id, pos);
        self.s.players.get_mut(&id).unwrap().pose.lower = 0.0;
        self.look(id, at);
    }

    fn send(&mut self, id: u64, axis: [f32; 2], hold: bool, crouch: bool, sprint: bool) {
        let seq = self.input_seq.entry(id).or_insert(0);
        *seq += 1;
        let p = self.s.players[&id].pose;
        let input = Input {
            run: self.s.run,
            sequence: *seq,
            axis,
            yaw: p.yaw,
            pitch: p.pitch,
            hold,
            crouch,
            sprint,
            light: false,
        };
        self.s.input(id, input, &self.t).unwrap();
    }

    fn tick(&mut self) {
        if self.calm && self.s.encounter.threat.state != ThreatState::Dormant {
            self.s.encounter.threat.cooldown = 1.0e9;
        }
        self.s.step(&self.l, &self.t, STEP);
    }

    fn idle(&mut self, secs: f32) {
        for _ in 0..(secs / STEP) as usize {
            let ids: Vec<u64> = self.s.players.keys().copied().collect();
            for id in ids {
                self.send(id, [0.0; 2], false, false, false);
            }
            self.tick();
        }
    }

    fn hold(&mut self, id: u64, secs: f32) {
        for _ in 0..(secs / STEP) as usize {
            self.send(id, [0.0; 2], true, false, false);
            self.tick();
        }
    }

    /// Walk a path with the real controller. `mode` = (crouch, sprint).
    fn walk(&mut self, id: u64, path: &[Vec2], mode: (bool, bool)) {
        for &goal in path {
            for step in 0..(90.0 / STEP) as usize {
                let pos = self.pose(id).pos;
                let d = goal - pos;
                if d.length() < 0.2 {
                    break;
                }
                assert!(step < (89.0 / STEP) as usize, "stuck walking to {goal:?} from {pos:?}");
                self.s.players.get_mut(&id).unwrap().pose.yaw = Pose::yaw_toward(pos, goal);
                self.send(id, [0.0, 1.0], false, mode.0, mode.1);
                self.tick();
            }
        }
    }

    /// Walk along the patrol graph between the nodes nearest two points.
    fn route(&self, from: Vec2, to: Vec2) -> Vec<Vec2> {
        let patrol = &self.l.patrol;
        let (mut at, goal) = (patrol.nearest(from), patrol.nearest(to));
        let mut out = vec![patrol.nodes[at]];
        while at != goal {
            at = patrol.next_hop(at, goal);
            out.push(patrol.nodes[at]);
        }
        out
    }

    fn events(&mut self, id: u64) -> Vec<Event> {
        let mut out = Vec::new();
        self.s.outbox.retain(|(to, m)| {
            if *to == id
                && let ServerMessage::Events { events, .. } = m
            {
                out.extend(events.iter().filter_map(|c| Event::from_code(*c)));
                return false;
            }
            true
        });
        out
    }

    fn take(&mut self, id: u64, index: usize) {
        let at = self.l.district.relics[index];
        let stand = self.stand_for(at);
        self.stand(id, stand, at);
        self.act(id, Action::Interact).unwrap();
        assert_eq!(self.s.encounter.progress.relics[index], Relic::Carried(id));
    }

    /// A spot 1.7 m from `at` on the side facing the road (open ground).
    fn stand_for(&self, at: Vec3) -> Vec2 {
        ground(at) + Vec2::new(0.0, 1.7)
    }
}

const TABLE: Vec2 = Vec2::new(2.9, -3.3);

#[test]
fn pickups_have_one_owner_drop_transfers_and_the_first_wakes_him_far_away() {
    let mut r = Rig::new(2);
    let bundle = r.l.district.relics[0];
    assert_eq!(r.s.encounter.threat.state, ThreatState::Dormant);
    r.stand(1, TABLE, bundle);
    r.stand(2, Vec2::new(2.5, -3.6), bundle);
    r.act(1, Action::Interact).unwrap();
    assert!(r.act(2, Action::Interact).is_err(), "second taker must be refused");
    assert_eq!(r.s.encounter.progress.relics[0], Relic::Carried(1));
    // The first pickup wakes him, rising at least 30 m from everyone.
    assert_eq!(r.s.encounter.threat.state, ThreatState::Stalking);
    assert!(matches!(r.s.encounter.threat.presence, Presence::Rising { .. }));
    for p in r.s.players.values() {
        assert!(r.s.encounter.threat.pos.distance(p.pose.pos) >= r.t.manifest_min_distance);
    }
    // Only the carrier can put it down; someone else can pick it up.
    assert!(r.act(2, Action::Drop).is_err());
    r.act(1, Action::Drop).unwrap();
    let Relic::Ground(at) = r.s.encounter.progress.relics[0] else {
        panic!("dropped bundle must lie on the ground")
    };
    r.stand(2, ground(at) + Vec2::new(-1.0, 0.0), at);
    r.act(2, Action::Interact).unwrap();
    assert_eq!(r.s.encounter.progress.relics[0], Relic::Carried(2));
    assert!(r.act(1, Action::Interact).is_err());
}

#[test]
fn reach_occlusion_epoch_permissions_and_state_are_validated() {
    let mut r = Rig::new(2);
    let bundle = r.l.district.relics[0];
    // Too far.
    r.stand(2, Vec2::new(2.0, 2.0), bundle);
    assert!(r.act(2, Action::Interact).is_err());
    // Behind the east wall.
    r.stand(2, Vec2::new(5.6, -5.2), bundle);
    assert!(r.act(2, Action::Interact).is_err());
    // Looking away.
    r.stand(2, TABLE, bundle);
    r.s.players.get_mut(&2).unwrap().pose.yaw += 3.0;
    assert!(r.act(2, Action::Interact).is_err());
    assert_eq!(r.s.encounter.progress.carried_total(), 0);
    // Stale epoch, unknown player, non-host restart and start twice.
    assert!(r.s.command(2, 0, 99, Action::Interact, &r.l, &r.t).is_err());
    assert!(r.s.command(9, 1, 1, Action::Interact, &r.l, &r.t).is_err());
    assert!(r.act(2, Action::Restart).is_err());
    assert!(r.act(HOST, Action::Start).is_err());
    assert_eq!(r.s.run, 1);
    // Replayed sequence numbers are refused.
    let run = r.s.run;
    assert!(r.s.command(2, run, 1, Action::Drop, &r.l, &r.t).is_err());
    // Nothing to drop or scatter.
    assert!(r.act(1, Action::Drop).is_err());
    assert!(r.act(1, Action::UseAji).is_err());
}

#[test]
fn a_leaving_carrier_drops_the_load_and_the_survivor_continues() {
    let mut r = Rig::new(2);
    r.take(2, 0);
    r.stand(2, Vec2::new(30.0, 20.0), r.l.district.relics[0]);
    r.s.remove_player(2);
    assert_eq!(r.s.players.len(), 1);
    assert!(!r.s.encounter.outcome.is_over());
    let Relic::Ground(at) = r.s.encounter.progress.relics[0] else {
        panic!("item lost after disconnect")
    };
    assert!((at.x - 30.0).abs() < 0.1 && (at.z - 20.0).abs() < 0.1);
    r.stand(1, ground(at) + Vec2::new(-1.0, 0.0), at);
    r.act(1, Action::Interact).unwrap();
    assert_eq!(r.s.encounter.progress.relics[0], Relic::Carried(1));
    assert!(r.s.outbox.iter().all(|(id, _)| *id != 2));
}

/// Put a hunting threat point-blank on `id` so the next step downs them.
fn doom(r: &mut Rig, id: u64, at: Vec2) {
    r.put(id, at);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Hunting;
    th.presence = Presence::Present;
    th.pos = at + Vec2::new(0.0, 0.6);
    th.cooldown = 0.0;
}

#[test]
fn the_hunted_player_goes_down_drops_the_load_and_a_teammate_revives_them() {
    let mut r = Rig::new(2);
    r.take(2, 0);
    r.put(1, Vec2::new(-30.0, 20.0));
    doom(&mut r, 2, Vec2::new(0.0, 8.0));
    r.s.players.get_mut(&1).unwrap().pose.pos = Vec2::new(-30.0, 20.0);
    r.idle(0.05);
    assert!(r.s.players[&2].status.is_downed());
    assert!(r.s.players[&1].status.is_active());
    assert!(
        matches!(r.s.encounter.progress.relics[0], Relic::Ground(_)),
        "the load is dropped"
    );
    assert!(!r.s.encounter.outcome.is_over(), "a teammate still stands");
    assert_eq!(r.s.encounter.stats.downs, 1);
    // He backs off instead of standing over the body.
    assert_eq!(r.s.encounter.threat.state, ThreatState::Stalking);
    assert!(r.act(2, Action::Interact).is_err(), "a downed player cannot act");
    // The teammate kneels beside them and holds.
    let body = r.pose(2).pos;
    let above = Vec3::new(body.x, r.l.surface_height(body) + 0.3, body.y);
    r.stand(1, body + Vec2::new(0.0, 1.6), above);
    r.calm = true;
    r.hold(1, r.t.revive_hold * 0.5);
    let half = r.s.players[&2].revive;
    assert!(half > 0.3 && half < 0.7, "revive is a progress bar: {half}");
    // Letting go loses progress slowly, it does not reset.
    r.idle(1.0);
    let after = r.s.players[&2].revive;
    assert!(after < half && after > 0.0);
    r.hold(1, r.t.revive_hold + 0.5);
    assert!(r.s.players[&2].status.is_active());
    assert_eq!(r.s.encounter.stats.revives, 1);
    assert!(r.s.players[&2].body.fear <= 0.5 + 1e-3, "revived shaken, not terrified");
    let ev = r.events(2);
    assert!(ev.contains(&Event::Revived) && ev.contains(&Event::Downed));
}

#[test]
fn bleeding_out_is_final_and_nobody_standing_is_failure() {
    let mut r = Rig::new(2);
    r.put(1, Vec2::new(-30.0, 20.0));
    doom(&mut r, 2, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    let Status::Downed { bleed } = r.s.players[&2].status else {
        panic!("not downed")
    };
    assert!((bleed - r.t.bleed_out).abs() < 1.0);
    r.calm = true;
    r.idle(r.t.bleed_out + 1.0);
    assert!(matches!(r.s.players[&2].status, Status::Dead));
    assert!(!r.s.encounter.outcome.is_over());
    // A dead teammate cannot be revived: no body target is offered.
    let body = r.pose(2).pos;
    let above = Vec3::new(body.x, r.l.surface_height(body) + 0.3, body.y);
    r.stand(1, body + Vec2::new(0.0, 1.6), above);
    r.hold(1, r.t.revive_hold + 1.0);
    assert!(matches!(r.s.players[&2].status, Status::Dead));
    // The last one standing goes down: the run is lost.
    r.calm = false;
    doom(&mut r, 1, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.outcome, Outcome::Failed);
    // Nothing is accepted after the end.
    assert!(r.act(1, Action::Drop).is_err());
}

#[test]
fn going_down_alone_ends_a_solo_run_at_once() {
    let mut r = Rig::new(1);
    doom(&mut r, HOST, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.outcome, Outcome::Failed);
    assert_eq!(r.s.snapshot(HOST, &r.l, &r.t).outcome, 2);
}

fn stand_at_altar(r: &mut Rig, id: u64) {
    let altar = r.l.ceiba.offering;
    let out = (ground(altar) - r.l.ceiba.center).normalize();
    r.stand(id, ground(altar) + out * 1.3, altar);
}

#[test]
fn only_the_carrier_lays_bones_down_one_bundle_at_a_time() {
    let mut r = Rig::new(2);
    r.calm = true;
    r.take(2, 0);
    r.take(2, 1);
    stand_at_altar(&mut r, 1);
    stand_at_altar(&mut r, 2);
    // Player 1 carries nothing: holding there is prayer, not delivery.
    r.hold(1, r.t.deliver_hold * 3.0);
    assert_eq!(r.s.encounter.progress.delivered(), 0);
    // Player 2 lays one down per deliver_hold seconds.
    r.hold(2, r.t.deliver_hold + 0.1);
    assert_eq!(r.s.encounter.progress.delivered(), 1);
    assert_eq!(r.s.encounter.progress.carried_by(2), 1);
    r.hold(2, r.t.deliver_hold + 0.1);
    assert_eq!(r.s.encounter.progress.delivered(), 2);
    assert_eq!(r.s.encounter.progress.carried_by(2), 0);
    // Interrupted progress does not carry over to the next bundle.
    r.take(2, 2);
    stand_at_altar(&mut r, 2);
    r.hold(2, r.t.deliver_hold * 0.6);
    r.idle(0.5);
    r.hold(2, r.t.deliver_hold * 0.6);
    assert_eq!(r.s.encounter.progress.delivered(), 2, "a broken hold starts over");
    let snap = r.s.snapshot(2, &r.l, &r.t);
    assert_eq!(snap.world.delivered, 2);
    assert_eq!(snap.world.total as usize, r.l.district.relics.len());
}

#[test]
fn praying_calms_fear_but_makes_a_noise_that_draws_him() {
    let mut r = Rig::new(1);
    r.calm = true;
    r.s.players.get_mut(&HOST).unwrap().body.fear = 0.9;
    stand_at_altar(&mut r, HOST);
    // A calm, present Silbón within earshot of the altar.
    let node = r.l.patrol.nodes[r.l.patrol.nearest(ground(r.l.ceiba.offering) + Vec2::new(10.0, 0.0))];
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = node;
    th.movement = el_silbon::sim::Movement::AtAnchor(r.l.patrol.nearest(node));
    let mut drawn = false;
    for _ in 0..(r.t.pray_hold as usize + 2) * 60 {
        r.send(HOST, [0.0; 2], true, false, false);
        r.tick();
        drawn |= r.s.encounter.threat.focus.is_some();
    }
    assert!(r.s.players[&HOST].body.fear < 0.05, "prayer clears fear");
    assert!(drawn, "prayer is heard: aqui tambien se escucha");
    assert!(r.events(HOST).contains(&Event::Prayed));
}

#[test]
fn cranking_the_pump_adds_up_across_players_and_lights_the_lamps() {
    let mut r = Rig::new(2);
    r.calm = true;
    let pump = r.l.district.pump;
    let stand = ground(pump) + Vec2::new(0.0, 1.45);
    r.stand(1, stand, pump);
    r.stand(2, stand + Vec2::new(0.9, 0.0), pump);
    let d = &r.l.district;
    let powered = d.lamps.iter().filter(|l| l.powered).count();
    assert!(powered >= 8, "the power poles are lamps that need the pump");
    let far = Vec2::new(-30.0, 31.0);
    assert!(!r.l.is_lit(far, false) || r.l.is_lit(far, true));
    // One player alone takes the full pump_hold; two take half.
    r.hold(1, r.t.pump_hold * 0.5);
    let solo = r.s.encounter.progress.power;
    assert!((solo - 0.5).abs() < 0.05, "{solo}");
    for _ in 0..(r.t.pump_hold * 0.3 * 60.0) as usize {
        r.send(1, [0.0; 2], true, false, false);
        r.send(2, [0.0; 2], true, false, false);
        r.tick();
    }
    assert!(
        r.s.encounter.progress.power_on(),
        "two crankers finish in well under pump_hold"
    );
    assert!(r.events(2).contains(&Event::PowerRestored));
    assert!(
        r.l.is_lit(Vec2::new(-24.0, 27.0), true),
        "powered pole lights the road once on"
    );
    assert!(!r.l.is_lit(Vec2::new(-24.0, 27.0), false));
}

#[test]
fn the_truck_waits_for_bones_and_power_then_its_engine_is_the_finale() {
    let mut r = Rig::new(2);
    let ign = r.l.district.ignition;
    let stand = ground(ign) + Vec2::new(0.0, -1.7);
    r.stand(1, stand, ign);
    r.stand(2, stand + Vec2::new(1.0, 0.0), ign);
    r.hold(1, r.t.truck_hold + 1.0);
    assert_eq!(r.s.encounter.progress.truck, 0.0, "no bones, no power");
    for x in r.s.encounter.progress.relics.iter_mut() {
        *x = Relic::Delivered;
    }
    r.hold(1, 2.0);
    assert_eq!(r.s.encounter.progress.truck, 0.0, "power still missing");
    r.s.encounter.progress.power = 1.0;
    r.hold(1, r.t.truck_hold + 1.0);
    assert!(r.s.encounter.progress.truck_running());
    assert!(r.events(1).contains(&Event::TruckStarted));
    // The engine wakes and rouses him and maxes the night.
    assert_ne!(r.s.encounter.threat.state, ThreatState::Dormant);
    r.idle(0.05);
    assert_eq!(r.s.encounter.pressure, 1.0);
    // Winning needs everyone standing in the boarding zone.
    let zone = r.l.district.truck;
    r.calm = true;
    r.put(1, zone.zone_center);
    r.put(2, zone.zone_center + Vec2::new(zone.zone_radius + 3.0, 0.0));
    r.idle(0.2);
    assert_eq!(r.s.encounter.outcome, Outcome::Running, "one teammate is still outside");
    r.put(2, zone.zone_center + Vec2::new(1.0, 0.0));
    r.s.players.get_mut(&1).unwrap().status = Status::Downed { bleed: 30.0 };
    r.idle(0.2);
    assert_eq!(r.s.encounter.outcome, Outcome::Running, "nobody is left behind");
    r.s.players.get_mut(&1).unwrap().status = Status::Active;
    r.idle(0.2);
    assert_eq!(
        r.s.encounter.outcome,
        Outcome::Running,
        "the engine is still warming up"
    );
    let snap = r.s.snapshot(1, &r.l, &r.t);
    assert!(snap.world.warm > 0.0 && snap.world.warm < 1.0);
    r.idle(r.t.truck_warmup);
    assert_eq!(r.s.encounter.outcome, Outcome::Won);
    assert!(r.events(1).contains(&Event::Escaped));
    assert_eq!(r.s.snapshot(1, &r.l, &r.t).outcome, r.s.snapshot(2, &r.l, &r.t).outcome);
}

/// A stalking threat 14 m away with sight but a huge warning cooldown.
fn lurker(r: &mut Rig, player: Vec2) {
    let node = Vec2::new(35.0, 2.0);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = node;
    th.movement = el_silbon::sim::Movement::AtAnchor(r.l.patrol.nearest(node));
    th.cooldown = 1.0e9;
    r.put(player_id(), player);
}
fn player_id() -> u64 {
    HOST
}

fn shuffle(r: &mut Rig, mode: (bool, bool)) -> bool {
    let mut heard = false;
    for i in 0..(4 * 60) {
        let dx = if (i / 60) % 2 == 0 { 1.0 } else { -1.0 };
        let p = r.pose(HOST).pos;
        r.s.players.get_mut(&HOST).unwrap().pose.yaw = Pose::yaw_toward(p, p + Vec2::new(dx, 0.0));
        r.send(HOST, [0.0, 1.0], false, mode.0, mode.1);
        r.tick();
        heard |= r.s.encounter.threat.focus.is_some();
    }
    heard
}

#[test]
fn sprinting_is_loud_walking_and_crouching_are_not() {
    for (mode, loud) in [((false, true), true), ((false, false), false), ((true, false), false)] {
        let mut r = Rig::new(1);
        lurker(&mut r, Vec2::new(34.0, 16.0));
        assert_eq!(shuffle(&mut r, mode), loud, "mode (crouch, sprint) = {mode:?}");
    }
}

#[test]
fn a_crouched_player_in_tall_grass_walks_past_a_watching_threat() {
    // A standing player 10 m from him is warned at once; a crouched one in
    // the fields is not even seen. He waits on a real patrol node.
    let grass = Vec2::new(58.0, -2.0);
    let node = Vec2::new(68.0, 0.0);
    assert!(Layout::new().district.tall_grass_at(grass));
    let spot = |r: &mut Rig| {
        let th = &mut r.s.encounter.threat;
        th.state = ThreatState::Stalking;
        th.presence = Presence::Present;
        th.pos = node;
        th.movement = el_silbon::sim::Movement::AtAnchor(r.l.patrol.nearest(node));
        th.cooldown = 0.0;
    };
    let mut standing = Rig::new(1);
    standing.put(HOST, grass);
    spot(&mut standing);
    standing.idle(0.3);
    assert_eq!(standing.s.encounter.threat.state, ThreatState::Warning);
    let mut hidden = Rig::new(1);
    hidden.put(HOST, grass);
    spot(&mut hidden);
    for _ in 0..(4 * 60) {
        hidden.send(HOST, [0.0; 2], false, true, false);
        hidden.tick();
    }
    assert!(hidden.s.players[&HOST].body.crouching);
    assert_ne!(hidden.s.encounter.threat.state, ThreatState::Warning);
    assert!(!hidden.s.encounter.threat.has_sight);
}

#[test]
fn peppers_are_finite_capped_and_make_a_ward_he_cannot_cross() {
    let mut r = Rig::new(1);
    r.calm = true;
    let at = r.l.district.aji[0];
    r.stand(HOST, ground(at) + Vec2::new(0.0, 1.8), at);
    r.act(HOST, Action::Interact).unwrap();
    assert_eq!(r.s.players[&HOST].aji, 1);
    assert!(r.s.encounter.progress.aji_taken[0]);
    assert!(r.act(HOST, Action::Interact).is_err(), "already gone");
    assert!(r.events(HOST).contains(&Event::AjiTaken));
    // Full hands leave the pepper where it is.
    r.s.players.get_mut(&HOST).unwrap().aji = r.t.aji_max;
    let other = r.l.district.aji[1];
    r.stand(HOST, ground(other) + Vec2::new(0.0, 1.8), other);
    assert!(r.act(HOST, Action::Interact).is_err());
    assert!(!r.s.encounter.progress.aji_taken[1]);
    // Scatter one: a ward appears ahead of the player and a pepper is spent.
    r.s.players.get_mut(&HOST).unwrap().aji = 1;
    r.stand(HOST, Vec2::new(0.0, 12.0), Vec3::new(0.0, 1.6, 0.0));
    r.act(HOST, Action::UseAji).unwrap();
    assert_eq!(r.s.players[&HOST].aji, 0);
    assert_eq!(r.s.encounter.zones.len(), 1);
    let z = r.s.encounter.zones[0];
    assert!(
        z.pos.distance(Vec2::new(0.0, 12.0)) < 2.5,
        "the ward lands at the player's feet"
    );
    assert!(r.act(HOST, Action::UseAji).is_err());
    let snap = r.s.snapshot(HOST, &r.l, &r.t);
    assert_eq!(snap.zones.len(), 1);
    assert_eq!(snap.me.aji, 0);
}

#[test]
fn pings_are_rate_limited_range_checked_and_shared() {
    let mut r = Rig::new(2);
    r.put(1, Vec2::new(0.0, 12.0));
    let here = [3.0, 0.5, 8.0];
    r.act(1, Action::Ping { at: here }).unwrap();
    assert!(r.act(1, Action::Ping { at: here }).is_err(), "cooldown");
    assert!(
        r.act(2, Action::Ping { at: [-5.0, 0.0, 30.0] }).is_ok(),
        "cooldown is per player"
    );
    r.idle(r.t.ping_cooldown + 0.1);
    assert!(r.act(1, Action::Ping { at: [500.0, 0.0, 0.0] }).is_err(), "too far");
    assert!(
        r.act(
            1,
            Action::Ping {
                at: [f32::NAN, 0.0, 0.0]
            }
        )
        .is_err()
    );
    r.act(1, Action::Ping { at: [4.0, 0.5, 8.0] }).unwrap();
    let snap = r.s.snapshot(2, &r.l, &r.t);
    let mine: Vec<_> = snap.pings.iter().filter(|p| p.by == 1).collect();
    assert_eq!(mine.len(), 1, "a new mark replaces the old one");
    assert_eq!(mine[0].pos, [4.0, 0.5, 8.0]);
    assert!(snap.pings.iter().any(|p| p.by == 2));
    r.idle(r.t.ping_life + 1.0);
    assert!(r.s.snapshot(1, &r.l, &r.t).pings.is_empty(), "marks fade");
}

#[test]
fn fear_climbs_alone_in_the_dark_and_company_and_lamplight_calm_it() {
    let dark = Vec2::new(-50.0, -55.0);
    let mut alone = Rig::new(2);
    alone.calm = true;
    alone.put(1, dark);
    alone.put(2, Vec2::new(60.0, -10.0));
    assert!(!alone.l.is_lit(dark, false));
    alone.idle(20.0);
    let scared = alone.s.players[&1].body.fear;
    assert!(scared > 0.3, "{scared}");
    // Company: the same dark, a teammate beside you.
    let mut together = Rig::new(2);
    together.calm = true;
    together.put(1, dark);
    together.put(2, dark + Vec2::new(2.0, 0.0));
    together.s.players.get_mut(&1).unwrap().body.fear = 0.5;
    together.idle(10.0);
    assert!(together.s.players[&1].body.fear < 0.3);
    // Lamplight: alone but in the glow of the gate lantern.
    let mut lit = Rig::new(2);
    lit.calm = true;
    lit.put(1, Vec2::new(-4.0, 24.0));
    lit.put(2, Vec2::new(60.0, -10.0));
    lit.s.players.get_mut(&1).unwrap().body.fear = 0.5;
    assert!(lit.l.is_lit(Vec2::new(-4.0, 24.0), false));
    lit.idle(10.0);
    assert!(lit.s.players[&1].body.fear < 0.2);
    // The snapshot tells the local player their own fear and nobody else's.
    let snap = alone.s.snapshot(1, &alone.l, &alone.t);
    assert!((snap.me.fear - scared).abs() < 1e-6);
}

#[test]
fn a_susto_freezes_the_player_and_screams_where_he_can_hear() {
    let mut r = Rig::new(1);
    r.calm = true;
    r.put(HOST, Vec2::new(-50.0, -55.0));
    r.s.players.get_mut(&HOST).unwrap().body.fear = 0.999;
    // A calm Silbón 25 m away hears the scream.
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = Vec2::new(-50.0, -30.0);
    th.movement = el_silbon::sim::Movement::AtAnchor(r.l.patrol.nearest(th.pos));
    r.idle(1.0);
    assert!(r.events(HOST).contains(&Event::Susto));
    assert!(r.s.players[&HOST].body.stun > 0.0 || r.s.players[&HOST].body.fear < 0.6);
    assert!(r.s.encounter.threat.focus.is_some(), "the scream carries");
    // While frozen, walking goes nowhere.
    let p = r.s.players.get_mut(&HOST).unwrap();
    p.body.stun = 1.0;
    let before = p.pose.pos;
    for _ in 0..30 {
        r.send(HOST, [0.0, 1.0], false, false, false);
        r.tick();
    }
    assert!(r.pose(HOST).pos.distance(before) < 1e-4);
    assert!(r.act(HOST, Action::Interact).is_err());
}

#[test]
fn hidden_threat_transform_is_not_in_listener_snapshots() {
    let mut r = Rig::new(2);
    let th = &mut r.s.encounter.threat;
    th.presence = Presence::Present;
    th.state = ThreatState::Stalking;
    th.pos = Vec2::new(0.0, -4.0);
    // Outside the solid side wall, looking through it.
    r.stand(2, Vec2::new(8.0, -4.0), Vec3::new(0.0, 2.0, -4.0));
    assert!(r.s.snapshot(2, &r.l, &r.t).threat.is_none());
    r.s.encounter.threat.pos = Vec2::new(0.0, 15.0);
    r.stand(2, Vec2::new(0.0, 8.0), Vec3::new(0.0, 2.0, 15.0));
    assert!(r.s.snapshot(2, &r.l, &r.t).threat.is_some());
    r.s.players.get_mut(&2).unwrap().pose.yaw += std::f32::consts::PI;
    assert!(r.s.snapshot(2, &r.l, &r.t).threat.is_none(), "behind the listener");
    r.s.encounter.threat.presence = Presence::Hidden;
    assert!(r.s.snapshot(1, &r.l, &r.t).threat.is_none());
    // The snapshot never carries where he is heading.
    let json = serde_json::to_string(&r.s.snapshot(1, &r.l, &r.t)).unwrap();
    assert!(!json.contains("focus") && !json.contains("pressure"));
}

#[test]
fn from_the_tower_deck_the_whole_llano_is_in_view() {
    let mut r = Rig::new(2);
    let d = &r.l.district;
    let deck = d.watch_deck.center();
    let far = deck + Vec2::new(-4.0, 100.0);
    let th = &mut r.s.encounter.threat;
    th.presence = Presence::Present;
    th.state = ThreatState::Stalking;
    th.pos = far;
    // Same distance, same look: invisible from the ground, visible from the deck.
    r.stand(1, Vec2::new(70.0, 10.0), Vec3::new(far.x, 1.0, far.y));
    let ground_view = {
        let p = Vec2::new(70.0, 10.0);
        r.s.encounter.threat.pos = p + Vec2::new(0.0, -100.0);
        r.stand(1, p, Vec3::new(p.x, 1.0, p.y - 100.0));
        r.idle(0.05);
        r.s.snapshot(1, &r.l, &r.t).threat.is_some()
    };
    assert!(!ground_view, "100 m is beyond ground-level sight");
    let deck_pos = deck;
    r.s.encounter.threat.pos = deck_pos + Vec2::new(0.0, -100.0);
    r.stand(1, deck_pos, Vec3::new(deck_pos.x, 8.0, deck_pos.y - 100.0));
    r.idle(0.05);
    assert!(r.s.players[&1].pose.pos.distance(deck_pos) < 0.5);
    assert!(r.s.snapshot(1, &r.l, &r.t).threat.is_some(), "the deck sees 130 m");
}

#[test]
fn one_hidden_threat_produces_distinct_categorical_listener_cues() {
    let mut r = Rig::new(2);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Warning;
    th.presence = Presence::Present;
    th.pos = Vec2::new(0.0, 10.0);
    r.put(1, Vec2::new(-35.0, 20.0));
    r.put(2, Vec2::new(0.0, 8.0));
    r.tick();
    let cue = |r: &Rig, id| {
        r.s.outbox.iter().find_map(|(recipient, message)| match message {
            ServerMessage::Cue { variant, .. } if *recipient == id => Some(*variant),
            _ => None,
        })
    };
    assert_eq!(cue(&r, 1), Some(0)); // actually far: perceived loud/near
    assert_eq!(cue(&r, 2), Some(2)); // actually near: perceived faint/far
}

#[test]
fn a_faint_whistle_frightens_the_listener_and_a_loud_one_does_not() {
    let mut r = Rig::new(2);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Warning;
    th.presence = Presence::Present;
    th.pos = Vec2::new(0.0, 10.0);
    r.put(1, Vec2::new(-35.0, 20.0));
    r.put(2, Vec2::new(0.0, 8.0));
    let base = r.s.players[&2].body.fear;
    r.tick();
    assert!(
        r.s.players[&2].body.fear > base + 0.03,
        "a thin, faraway whistle means he is near"
    );
    assert!(r.s.players[&1].body.fear < 0.01);
}

#[test]
fn restart_resets_everything_and_invalidates_old_messages() {
    let mut r = Rig::new(2);
    r.take(2, 0);
    r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 20.0 };
    r.s.players.get_mut(&1).unwrap().aji = 2;
    r.act(1, Action::Ping { at: [1.0, 0.0, 30.0] }).unwrap();
    r.act(HOST, Action::Restart).unwrap();
    assert_eq!(r.s.run, 2);
    assert_eq!(r.s.players.len(), 2);
    assert_eq!(r.s.encounter, el_silbon::sim::Encounter::new(&r.l));
    assert!(r.s.outbox.is_empty() && r.s.pings.is_empty());
    assert!(
        r.s.players
            .values()
            .all(|p| p.status.is_active() && p.aji == 0 && p.body.fear == 0.0)
    );
    // Old-run actions and inputs are refused.
    let stale = r.s.command(2, 1, 999, Action::Interact, &r.l, &r.t);
    assert!(stale.is_err());
    assert!(
        r.s.input(
            2,
            Input {
                run: 1,
                sequence: 999,
                axis: [1.0, 1.0],
                hold: true,
                ..Default::default()
            },
            &r.t
        )
        .is_err()
    );
    let before = r.pose(2);
    r.s.step(&r.l, &r.t, STEP);
    for _ in 0..60 {
        r.s.step(&r.l, &r.t, STEP);
    }
    assert_eq!(r.pose(2), before);
    // Late joiners are still refused after a restart.
    assert!(r.s.add_player(3, &r.l, &r.t).is_err());
    // The four-player cap and duplicate identities hold before the start.
    let l = Layout::new();
    let t = Tuning::default();
    let mut lobby = Session::new(&l, &t);
    for id in 2..=4 {
        lobby.add_player(id, &l, &t).unwrap();
    }
    assert!(lobby.add_player(5, &l, &t).is_err());
    assert!(lobby.add_player(3, &l, &t).is_err());
    let xs: Vec<f32> = lobby.players.values().map(|p| p.pose.pos.x).collect();
    let mut sorted = xs.clone();
    sorted.sort_by(f32::total_cmp);
    sorted.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    assert_eq!(sorted.len(), 4, "four players spawn apart: {xs:?}");
}

#[test]
fn stale_motion_and_invalid_input_never_keep_players_walking() {
    let mut r = Rig::new(2);
    let initial = r.pose(2).pos;
    let mk = |sequence, axis, yaw| Input {
        run: 1,
        sequence,
        axis,
        yaw,
        ..Default::default()
    };
    r.s.input(2, mk(2, [1.0, 0.0], 0.0), &r.t).unwrap();
    r.s.input(2, mk(1, [-1.0, 0.0], 0.0), &r.t).unwrap();
    for _ in 0..60 {
        r.tick();
    }
    let stopped = r.pose(2).pos;
    assert!(stopped.x > initial.x && stopped.distance(initial) < r.t.walk_speed * 0.31);
    for _ in 0..60 {
        r.tick();
    }
    assert_eq!(r.pose(2).pos, stopped);
    assert!(r.s.input(2, mk(3, [0.0; 2], f32::NAN), &r.t).is_err());
    assert!(r.s.input(2, mk(4, [f32::INFINITY, 0.0], 0.0), &r.t).is_err());
}

/// One bundle's errand: out from the altar to the bundle, and back.
struct Errand {
    index: usize,
    out: Vec<Vec2>,
    back: Vec<Vec2>,
}

/// The whole map, end to end, through the real session and controller:
/// every bundle at its landmark, the altar, the pump, the truck. The threat
/// is muzzled (never warns) so this proves the map, not the AI.
#[test]
fn the_whole_map_is_playable_end_to_end() {
    let mut r = Rig::new(2);
    r.calm = true;
    let d = r.l.district.clone();
    let altar_stand = {
        let altar = r.l.ceiba.offering;
        ground(altar) + (ground(altar) - r.l.ceiba.center).normalize() * 1.3
    };
    let shrine = Vec2::new(-18.0, -40.0);
    let walk = (false, false);

    // Player 1: through the front door to the table bundle.
    r.walk(
        1,
        &[
            Vec2::new(0.0, 27.0),
            Vec2::new(0.0, 18.0),
            Vec2::new(0.0, 4.0),
            Vec2::new(0.0, 0.0),
        ],
        walk,
    );
    r.walk(1, &[Vec2::new(0.6, -1.5), TABLE], walk);
    r.look(1, d.relics[0]);
    r.act(1, Action::Interact).unwrap();
    assert_eq!(r.s.encounter.progress.relics[0], Relic::Carried(1));
    assert_eq!(r.s.encounter.threat.state, ThreatState::Stalking);
    // Out the back door and along the western track to the ceiba.
    let mut to_altar = vec![
        Vec2::new(-3.0, -5.0),
        Vec2::new(-3.0, -7.4),
        Vec2::new(-3.0, -8.0),
        Vec2::new(-10.0, -23.0),
        shrine,
    ];
    to_altar.push(altar_stand);
    r.walk(1, &to_altar, walk);
    r.look(1, d_offering(&r));
    r.hold(1, r.t.deliver_hold + 0.2);
    assert_eq!(r.s.encounter.progress.delivered(), 1);

    // The remaining bundles, each at its landmark, walked along the graph.
    let corral_gate = Vec2::new(35.0, -18.0);
    let fields = Vec2::new(53.0, 13.0);
    let bridge_south = Vec2::new(18.0, -65.0);
    let tower_base = Vec2::new(43.0, -54.0);
    let cat = |a: Vec<Vec2>, b: Vec<Vec2>, c: Vec<Vec2>| a.into_iter().chain(b).chain(c).collect::<Vec<_>>();
    let errands = [
        Errand {
            index: 1,
            out: cat(r.route(altar_stand, corral_gate), vec![Vec2::new(35.0, -24.8)], vec![]),
            back: cat(vec![corral_gate], r.route(corral_gate, shrine), vec![altar_stand]),
        },
        Errand {
            index: 2,
            out: cat(
                r.route(altar_stand, fields),
                vec![Vec2::new(59.7, 9.0), Vec2::new(59.7, 2.3)],
                vec![Vec2::new(62.0, 2.3)],
            ),
            back: cat(
                vec![Vec2::new(59.7, 2.3), Vec2::new(59.7, 9.0)],
                r.route(fields, shrine),
                vec![altar_stand],
            ),
        },
        Errand {
            index: 3,
            out: cat(
                r.route(altar_stand, bridge_south),
                vec![
                    Vec2::new(18.0, -73.0),
                    Vec2::new(21.0, -73.0),
                    Vec2::new(21.0, -70.6),
                    Vec2::new(26.0, -70.6),
                ],
                vec![Vec2::new(26.0, -73.8)],
            ),
            back: cat(
                vec![
                    Vec2::new(26.0, -70.6),
                    Vec2::new(21.0, -70.6),
                    Vec2::new(21.0, -73.0),
                    Vec2::new(18.0, -73.0),
                ],
                r.route(bridge_south, shrine),
                vec![altar_stand],
            ),
        },
        Errand {
            index: 4,
            out: cat(
                r.route(altar_stand, tower_base),
                vec![Vec2::new(43.0, -59.0), Vec2::new(43.0, -83.0)],
                vec![Vec2::new(43.0, -87.8)],
            ),
            back: cat(
                vec![Vec2::new(43.0, -83.0), Vec2::new(43.0, -59.0)],
                r.route(tower_base, shrine),
                vec![altar_stand],
            ),
        },
    ];
    for e in errands {
        r.walk(1, &e.out, walk);
        r.look(1, d.relics[e.index]);
        r.act(1, Action::Interact)
            .unwrap_or_else(|err| panic!("bundle {}: {err}", e.index));
        assert_eq!(
            r.s.encounter.progress.relics[e.index],
            Relic::Carried(1),
            "bundle {}",
            e.index
        );
        r.walk(1, &e.back, walk);
        r.look(1, d_offering(&r));
        r.hold(1, r.t.deliver_hold + 0.2);
        assert_eq!(r.s.encounter.progress.delivered(), e.index + 1, "delivered {}", e.index);
    }
    assert!(r.s.encounter.progress.bones_home());
    assert!(r.events(1).contains(&Event::AllBonesHome));

    // The pump at the windmill, on its rise.
    let tower_approach = Vec2::new(-39.0, -14.0);
    let mut to_pump = r.route(altar_stand, tower_approach);
    to_pump.extend([Vec2::new(-42.0, -18.0), ground(d.pump) + Vec2::new(0.0, 1.45)]);
    r.walk(1, &to_pump, walk);
    r.look(1, d.pump);
    r.hold(1, r.t.pump_hold + 0.5);
    assert!(r.s.encounter.progress.power_on());
    assert!(r.s.players[&1].pose.pos.distance(ground(d.pump)) < 2.0);

    // Both players to the truck: player 2 comes down the road from the spawn.
    let mut to_truck = r.route(tower_approach, Vec2::new(55.0, 30.0));
    to_truck.push(Vec2::new(55.7, 29.2));
    r.walk(1, &to_truck, walk);
    r.look(1, d.ignition);
    r.hold(1, r.t.truck_hold + 0.5);
    assert!(r.s.encounter.progress.truck_running(), "engine must start");
    r.idle(0.05);
    assert_eq!(r.s.encounter.pressure, 1.0);
    assert_eq!(r.s.encounter.outcome, Outcome::Running, "player 2 is still at the gate");
    r.walk(2, &[Vec2::new(30.0, 31.0), Vec2::new(50.0, 30.0)], walk);
    r.idle(0.3);
    assert_eq!(
        r.s.encounter.outcome,
        Outcome::Running,
        "the engine is still warming up"
    );
    r.idle(r.t.truck_warmup);
    assert_eq!(r.s.encounter.outcome, Outcome::Won);
    let minutes = r.s.encounter.elapsed / 60.0;
    assert!(
        (3.0..14.0).contains(&minutes),
        "a full run should take minutes, not {minutes:.1}"
    );
}

fn d_offering(r: &Rig) -> Vec3 {
    r.l.ceiba.offering
}

/// A scripted player at a session: one frame is a real client's frame, in the
/// game's order and by its rules (`el_silbon::net`), against the real session
/// with no shortcut into its state. The client only knows what its endpoint
/// hands it: a snapshot every frame when its session runs in the same
/// process (solo, host) but one per send interval as a joiner. Its body
/// trails its own row of that snapshot, its crosshair is the HUD's (read
/// after the head turned, from the snapshot in hand), its controls carry
/// nothing while it is frozen or dead or the run is over, a joiner's input
/// goes out at the send interval and ahead of each command, and every command
/// is stamped with the run of the snapshot last seen. With `stalls` set, the
/// client now and then misses frames, as a real one does in a render stall:
/// the session steps on with its last input, and the next frame's time
/// includes the stall.
struct Pilot {
    id: u64,
    script: RouteScript,
    /// Frames between the snapshots this client is handed.
    every: u64,
    frames: u64,
    /// The newest snapshot received, and the presentation's mirror of it.
    view: Snapshot,
    encounter: Encounter,
    pose: Pose,
    /// What the crosshair showed as the last frame ended.
    hud: Option<Target>,
    /// The newest input packet, as an endpoint keeps it between sends.
    latest: Input,
    input_seq: u64,
    action_seq: u64,
    finished: bool,
    left: bool,
    /// The outcome standing at each restart the route asked for.
    ended: Vec<Outcome>,
    /// Each (run, ended outcome) this player's own snapshot showed.
    observed: Vec<(u64, Outcome)>,
    /// Seed of the frames this client misses (0: none).
    stalls: u64,
    /// Session steps so far, and frames missed since the last one ran.
    steps: u64,
    missed: u32,
}

impl Pilot {
    /// `joiner`: this client reaches the session over the wire.
    fn new(id: u64, script: RouteScript, joiner: bool, s: &Session, l: &Layout, t: &Tuning) -> Self {
        let view = s.snapshot(id, l, t);
        let mut pose = Pose::spawn(l);
        let row = view.player(id).expect("the player is in the session");
        follow_body(&mut pose, row, true, STEP, t);
        let mut encounter = Encounter::new(l);
        mirror(&mut encounter, &view);
        let every = if joiner {
            (SEND_INTERVAL / STEP).round() as u64
        } else {
            1
        };
        Self {
            id,
            script,
            every,
            frames: 0,
            view,
            encounter,
            pose,
            hud: None,
            latest: Input::default(),
            input_seq: 0,
            action_seq: 0,
            finished: false,
            left: false,
            ended: Vec::new(),
            observed: Vec::new(),
            stalls: 0,
            steps: 0,
            missed: 0,
        }
    }

    /// About one frame in eighty is lost to a stall, by the seed.
    fn stalled(&self) -> bool {
        if self.stalls == 0 {
            return false;
        }
        let mut h = self.stalls ^ self.steps.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 31;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 29;
        h.is_multiple_of(80)
    }

    /// Seconds this frame covers: one step, plus any stall before it.
    fn dt(&self) -> f32 {
        STEP * (1 + self.missed) as f32
    }

    /// A command as an endpoint stamps it: with the run of the snapshot last seen.
    fn act(&mut self, s: &mut Session, l: &Layout, t: &Tuning, action: Action) -> Result<(), String> {
        self.action_seq += 1;
        s.command(self.id, self.view.run, self.action_seq, action, l, t)
    }

    fn idle(&self) -> bool {
        self.finished || self.left
    }

    /// The client's frame: the route decides from what was last seen, the
    /// head turns, the crosshair is read, the input goes out with its presses
    /// and the route's own command follows.
    fn frame(&mut self, s: &mut Session, l: &Layout, t: &Tuning) {
        self.steps += 1;
        if self.idle() {
            return;
        }
        if self.stalled() {
            self.missed += 1;
            return;
        }
        self.frames += 1;
        let dt = self.dt();
        let step = self.script.tick(&Observation {
            layout: l,
            tuning: t,
            me: self.id,
            pose: self.pose,
            encounter: &self.encounter,
            snapshot: Some(&self.view),
            target: self.hud,
            dt,
        });
        if let Some(done) = step.finished {
            done.unwrap_or_else(|e| panic!("player {} route failed: {e}", self.id));
            self.finished = true;
            return;
        }
        if step.leave {
            s.remove_player(self.id);
            self.left = true;
            return;
        }
        let me = Some(self.id);
        // The frozen and the downed can still turn their heads; the dead only watch.
        if status_of(Some(&self.view), me) != 2 {
            self.pose.look(step.intent.look_delta, t);
        }
        self.hud = crosshair(l, t, &self.view, self.id, &self.pose);
        let live = controls_live(Some(&self.view), me, true);
        let wire = Wire::new(&step.intent, live, &self.pose, true, self.hud, l, t);
        self.input_seq += 1;
        self.latest = Input {
            run: self.view.run,
            sequence: self.input_seq,
            ..wire.input
        };
        let actions: Vec<Action> = wire.commands().chain(step.command()).collect();
        if !actions.is_empty() || self.frames.is_multiple_of(self.every) {
            // Refused when the run has moved on; the next packet catches up.
            let _ = s.input(self.id, self.latest, t);
        }
        for action in actions {
            if matches!(action, Action::Restart) {
                self.ended.push(s.encounter.outcome);
            }
            let sent = self.act(s, l, t, action);
            // The host's start and restart are never refused. A mark can be
            // (cooldown) and a press can lose a race to another taker, or
            // arrive under a run that has just ended: the route repeats those.
            if matches!(action, Action::Start | Action::Restart) {
                sent.unwrap_or_else(|e| panic!("player {} {action:?} refused: {e}", self.id));
            }
        }
    }

    /// The endpoint hands over the newest snapshot: a new run resets what it
    /// counts. True when the run is new.
    fn arrive(&mut self, s: &Session, l: &Layout, t: &Tuning) -> bool {
        let snap = s.snapshot(self.id, l, t);
        let fresh = snap.run != self.view.run;
        if fresh {
            self.input_seq = 0;
            self.action_seq = 0;
            self.latest = Input::default();
        }
        mirror(&mut self.encounter, &snap);
        if snap.outcome().is_over() {
            let seen = (snap.run, snap.outcome());
            if self.observed.last() != Some(&seen) {
                self.observed.push(seen);
            }
        }
        self.view = snap;
        fresh
    }

    /// After the session stepped: a snapshot arrives (each frame, or each
    /// send interval for a joiner) and the body follows its row of the newest.
    fn receive(&mut self, s: &Session, l: &Layout, t: &Tuning) {
        if self.idle() || self.stalled() {
            return;
        }
        let fresh = self.frames.is_multiple_of(self.every) && self.arrive(s, l, t);
        let dt = self.dt();
        if let Some(local) = self.view.player(self.id) {
            follow_body(&mut self.pose, local, fresh, dt, t);
        }
        self.missed = 0;
    }
}

/// Frame the pilots against one session until every route is done; the
/// host's events are returned.
fn play(s: &mut Session, l: &Layout, t: &Tuning, pilots: &mut [&mut Pilot], minutes: f32) -> Vec<Event> {
    let mut seen = Vec::new();
    for _ in 0..(minutes * 60.0 / STEP) as usize {
        for p in pilots.iter_mut() {
            p.frame(s, l, t);
        }
        if pilots.iter().all(|p| p.idle()) {
            return seen;
        }
        s.step(l, t, STEP);
        for (to, message) in s.outbox.drain(..) {
            if to == HOST
                && let ServerMessage::Events { events, .. } = message
            {
                seen.extend(events.iter().filter_map(|c| Event::from_code(*c)));
            }
        }
        for p in pilots.iter_mut() {
            p.receive(s, l, t);
        }
    }
    let at: Vec<String> = pilots
        .iter()
        .map(|p| format!("player {} at {}", p.id, p.script.step_name()))
        .collect();
    panic!("the routes did not finish in {minutes} minutes: {}", at.join("; "));
}

/// The solo smoke route, played through the real session: every bundle to
/// the altar, the pump, the truck and a won escape; a restart; a second night
/// of warning, recovery and a caught failure; a restart. `t` picks the storm;
/// `stalls` the frames the client misses (0: none).
fn solo_route(t: Tuning, stalls: u64, route: fn(&Layout, &Tuning) -> RouteScript, outcomes: &[Outcome]) {
    let l = Layout::new();
    let t = &t;
    let mut s = Session::new(&l, t);
    let mut pilot = Pilot::new(HOST, route(&l, t), false, &s, &l, t);
    pilot.stalls = stalls;
    // A solo endpoint starts its run before the first snapshot it hands over.
    pilot.act(&mut s, &l, t, Action::Start).unwrap();
    pilot.arrive(&s, &l, t);
    let seen = play(&mut s, &l, t, &mut [&mut pilot], 40.0);
    assert_eq!(pilot.ended, outcomes, "the outcome each restart ended");
    assert_eq!(s.run as usize, outcomes.len() + 1);
    let count = |e: Event| seen.iter().filter(|x| **x == e).count();
    assert_eq!(
        count(Event::RelicDelivered),
        5,
        "all five bundles reach the altar, once each"
    );
    for e in [
        Event::RelicTaken,
        Event::AllBonesHome,
        Event::PowerRestored,
        Event::TruckStarted,
        Event::WarningBegan,
        Event::Downed,
    ] {
        assert!(count(e) > 0, "the route never produced {e:?}");
    }
    let escaped = seen.iter().position(|e| *e == Event::Escaped).expect("the escape");
    let downed = seen.iter().position(|e| *e == Event::Downed).expect("the fall");
    assert!(escaped < downed, "the escape comes first, the caught failure second");
    assert!(
        count(Event::WarningAverted) + count(Event::LostTrack) + count(Event::CountingEnded) > 0,
        "he must lose us at least once before he catches us"
    );
}

#[test]
fn the_smoke_route_wins_restarts_recovers_fails_and_restarts() {
    solo_route(
        Tuning::default(),
        0,
        RouteScript::full,
        &[Outcome::Won, Outcome::Failed],
    );
}

#[test]
fn the_tour_walks_every_landmark_then_plays_the_whole_route() {
    solo_route(
        Tuning::default(),
        0,
        RouteScript::tour,
        &[Outcome::Running, Outcome::Won, Outcome::Failed],
    );
}

/// The two-process network route, without sockets: host and partner routes
/// against one session, the partner a joiner (snapshots and input at the send
/// interval). Run 1: delivery and marks seen by both, then all five
/// bundles, the pump, the truck and both standing in its zone: a shared WIN
/// both endpoints saw before the host restarts. Run 2: a shared FAILURE both
/// saw. Run 3: the carrier's load falls when she leaves and the host recovers
/// and delivers it.
#[test]
fn the_network_smoke_routes_share_outcomes_restarts_and_a_dropped_load() {
    network_route(Tuning::default(), 0);
}

/// The two-process story against one session; `t` picks the storm, `stalls`
/// the frames the clients miss (0: none).
fn network_route(t: Tuning, stalls: u64) {
    let (l, t) = (Layout::new(), &t);
    let mut s = Session::new(&l, t);
    s.add_player(2, &l, t).unwrap();
    let mut host = Pilot::new(HOST, RouteScript::net_host(&l, t), false, &s, &l, t);
    let mut partner = Pilot::new(2, RouteScript::net_client(&l, t), true, &s, &l, t);
    host.stalls = stalls;
    partner.stalls = stalls.wrapping_mul(7919);
    let seen = play(&mut s, &l, t, &mut [&mut host, &mut partner], 60.0);
    assert!(host.finished && partner.left);
    assert_eq!(s.run, 3, "two clean restarts");
    assert_eq!(
        host.ended,
        [Outcome::Won, Outcome::Failed],
        "the outcome standing at each restart"
    );
    let shared = [(1, Outcome::Won), (2, Outcome::Failed)];
    assert_eq!(
        host.observed, shared,
        "the host saw the shared win, then the shared failure"
    );
    assert_eq!(
        partner.observed, shared,
        "the partner saw the shared win, then the shared failure"
    );
    assert_eq!(s.players.len(), 1);
    assert_eq!(
        s.encounter.progress.relics[0],
        Relic::Delivered,
        "the dropped load was recovered and laid down"
    );
    let count = |e: Event| seen.iter().filter(|x| **x == e).count();
    assert_eq!(
        count(Event::RelicDelivered),
        6,
        "five bundles in the win run, the recovered one in run three"
    );
    for e in [
        Event::AllBonesHome,
        Event::PowerRestored,
        Event::TruckStarted,
        Event::Escaped,
    ] {
        assert_eq!(count(e), 1, "{e:?} exactly once, in the win run");
    }
    let escaped = seen.iter().position(|e| *e == Event::Escaped).unwrap();
    let downed = seen
        .iter()
        .position(|e| *e == Event::Downed)
        .expect("the shared failure");
    assert!(escaped < downed, "the shared win comes before the shared failure");
    assert!(
        count(Event::Downed) >= 2,
        "both players are caught before the shared failure"
    );
}

/// Real clients miss frames now and then, and a different seed brings a
/// different storm (rain and thunder mask footsteps differently), so he
/// meets the scripted players elsewhere and at other moments. The routes
/// hold to the same outcomes under each.
#[test]
fn the_routes_hold_through_other_storms_and_missed_frames() {
    for seed in 1..=4 {
        solo_route(
            Tuning::with_seed(seed),
            seed,
            RouteScript::full,
            &[Outcome::Won, Outcome::Failed],
        );
        network_route(Tuning::with_seed(seed), seed);
    }
}

/// What a frame that holds every control down puts on the wire for `id`, by
/// the game's rules, from the snapshot that player was last handed.
fn every_control(r: &Rig, id: u64) -> (Input, Vec<Action>) {
    let snap = r.s.snapshot(id, &r.l, &r.t);
    let intent = Intent {
        move_axis: Vec2::Y,
        interact_pressed: true,
        interact_held: true,
        crouch: true,
        sprint: true,
        drop: true,
        use_aji: true,
        ..Default::default()
    };
    let live = controls_live(Some(&snap), Some(id), true);
    let wire = Wire::new(&intent, live, &Pose::spawn(&r.l), true, None, &r.l, &r.t);
    (wire.input, wire.commands().collect())
}

#[test]
fn controls_carry_nothing_while_frozen_dead_or_the_run_is_over() {
    let mut r = Rig::new(2);
    let (input, commands) = every_control(&r, HOST);
    assert_eq!(input.axis, [0.0, 1.0]);
    assert!(input.hold && input.crouch && input.sprint && input.light);
    assert!(
        matches!(commands.as_slice(), [Action::Drop, Action::UseAji]),
        "a live player's presses go out in order: {commands:?}"
    );
    let silent = |r: &Rig, id: u64, why: &str| {
        let (input, commands) = every_control(r, id);
        assert_eq!(input.axis, [0.0; 2], "{why}: no movement");
        assert!(!(input.hold || input.crouch || input.sprint), "{why}: no held control");
        assert!(commands.is_empty(), "{why}: no commands, got {commands:?}");
    };
    // A susto holds the player frozen.
    r.s.players.get_mut(&HOST).unwrap().body.stun = 2.0;
    silent(&r, HOST, "frozen");
    r.s.players.get_mut(&HOST).unwrap().body.stun = 0.0;
    // The dead only watch; the downed still crawl.
    r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 20.0 };
    let (input, _) = every_control(&r, 2);
    assert_eq!(input.axis, [0.0, 1.0], "a downed player still crawls");
    r.s.players.get_mut(&2).unwrap().status = Status::Dead;
    silent(&r, 2, "dead");
    // A finished run takes no more input from anyone.
    r.s.encounter.outcome = Outcome::Won;
    silent(&r, HOST, "won");
}

/// Opt-in measurement for work on the route script: the solo and shared
/// routes over many storms, each with its own pattern of missed frames. The
/// driver plays a chaotic game, so one seed passing proves little; this
/// counts. `cargo test --locked --test session -- --ignored --nocapture`
#[test]
#[ignore = "slow: 300 full routes"]
fn the_routes_hold_over_many_storms() {
    let n = 150;
    std::panic::set_hook(Box::new(|_| {}));
    let why = |e: Box<dyn std::any::Any + Send>| e.downcast_ref::<String>().cloned().unwrap_or_default();
    let (mut solo, mut shared) = (0, 0);
    for seed in 1..=n {
        match std::panic::catch_unwind(|| {
            solo_route(
                Tuning::with_seed(seed),
                seed,
                RouteScript::full,
                &[Outcome::Won, Outcome::Failed],
            )
        }) {
            Ok(()) => solo += 1,
            Err(e) => eprintln!("solo seed {seed}: {}", why(e)),
        }
        match std::panic::catch_unwind(|| network_route(Tuning::with_seed(seed), seed)) {
            Ok(()) => shared += 1,
            Err(e) => eprintln!("shared seed {seed}: {}", why(e)),
        }
    }
    let _ = std::panic::take_hook();
    eprintln!("solo {solo}/{n}, shared {shared}/{n}");
    assert!(
        solo * 100 >= n * 97 && shared * 100 >= n * 97,
        "fewer than 97% of routes held"
    );
}

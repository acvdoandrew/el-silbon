use bevy::math::{Vec2, Vec3};
use el_silbon::{
    body::Status,
    control::{Intent, Pose, Target},
    geometry::{Layout, Wade, ground},
    net::{
        Wire, controls_live, cycle_watch, follow_body, mirror,
        protocol::{Action, CallKind, HOST, Input, SEND_INTERVAL, STEP, ServerMessage, Snapshot},
        session::Session,
        status_of, watched,
    },
    script::{Observation, RouteScript, crosshair},
    sim::{Encounter, Event, Outcome, Presence, Relic, ThreatState},
    survivor::Survivor,
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
    /// Every player's torch is switched on.
    lit: bool,
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
            lit: false,
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
            light: self.lit,
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

    /// Hold interact for `secs`, answering every skill check just inside
    /// the start of its zone, as a practised hand does.
    fn work(&mut self, id: u64, secs: f32) {
        self.work_all(&[id], secs);
    }

    /// `work` for several players at once, all holding on together.
    fn work_all(&mut self, ids: &[u64], secs: f32) {
        for _ in 0..(secs / STEP) as usize {
            for &id in ids {
                self.send(id, [0.0; 2], true, false, false);
                let t = &self.t;
                let press = self.s.players[&id]
                    .rhythm
                    .check
                    .filter(|c| c.needle(t) >= c.zone + t.check_great * 0.5)
                    .map(|c| (c.id, c.needle(t)));
                if let Some((check, needle)) = press {
                    self.act(id, Action::Skill { id: check, needle }).unwrap();
                }
            }
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
fn each_player_is_someone_different_chosen_in_the_lobby_and_kept_across_restarts() {
    let l = Layout::new();
    let t = Tuning::default();
    let mut s = Session::new(&l, &t);
    // The host wants the coplera and gets her; a joiner who wants her too is
    // given someone free, and another's free wish is granted.
    assert_eq!(s.choose_survivor(HOST, Survivor::Coplera), Ok(Survivor::Coplera));
    s.add_player(2, &l, &t).unwrap();
    assert_eq!(s.choose_survivor(2, Survivor::Coplera), Ok(Survivor::Llanero));
    s.add_player(3, &l, &t).unwrap();
    assert_eq!(s.choose_survivor(3, Survivor::Muchacho), Ok(Survivor::Muchacho));
    // A player may change their mind to someone free, never to someone taken.
    assert_eq!(s.choose_survivor(2, Survivor::Muchacho), Ok(Survivor::Llanero));
    assert_eq!(s.choose_survivor(2, Survivor::Encargado), Ok(Survivor::Encargado));
    // A leaver's person is free again.
    s.remove_player(3);
    assert_eq!(s.choose_survivor(2, Survivor::Muchacho), Ok(Survivor::Muchacho));
    // Everyone sees who everyone is, and it survives a restart.
    let seen = |s: &Session| -> Vec<(u64, u8)> {
        s.snapshot(HOST, &l, &t)
            .players
            .iter()
            .map(|p| (p.id, p.survivor))
            .collect()
    };
    let before = seen(&s);
    assert_eq!(
        before,
        vec![(HOST, Survivor::Coplera.code()), (2, Survivor::Muchacho.code())]
    );
    s.command(HOST, s.run, 1, Action::Start, &l, &t).unwrap();
    assert!(s.choose_survivor(2, Survivor::Llanero).is_err(), "only in the lobby");
    s.restart(&l, &t);
    assert_eq!(seen(&s), before);
}

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
    assert!(r.act(2, Action::Interact).is_err(), "a downed player cannot act");
    // With a teammate still standing, he puts the fallen in his sack and
    // walks off with them, slower than a walking player.
    assert_eq!(r.s.encounter.threat.state, ThreatState::Hauling);
    assert!(r.events(1).contains(&Event::Hauled));
    let snap = r.s.snapshot(1, &r.l, &r.t);
    assert!(snap.player(2).unwrap().hauled, "the teammate sees them carried off");
    r.idle(2.0);
    assert!(
        r.pose(2).pos.distance(r.s.encounter.threat.pos) < 1.0,
        "in the sack, with him"
    );
    assert!(r.pose(2).pos.distance(Vec2::new(0.0, 8.0)) > 1.0, "carried off");
    // Pepper in his path: he stops to count, and drops them.
    let th = r.s.encounter.threat.pos;
    let ahead = th + r.s.encounter.threat.facing * 2.5;
    r.s.encounter.place_aji(&r.t, ahead, &mut Vec::new());
    let mut dropped = false;
    for _ in 0..(10.0 / STEP) as usize {
        r.tick();
        if r.s.encounter.threat.state == ThreatState::Counting {
            dropped = true;
            break;
        }
    }
    assert!(dropped, "the ward stopped him");
    assert!(r.events(1).contains(&Event::SackDropped));
    // The teammate kneels beside them and holds.
    let body = r.pose(2).pos;
    let above = Vec3::new(body.x, r.l.surface_height(body) + 0.3, body.y);
    let side = (body - r.s.encounter.threat.pos).normalize_or(Vec2::Y);
    r.stand(1, body + side * 1.4, above);
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
fn nobody_stops_him_and_the_one_in_his_sack_is_gone() {
    let mut r = Rig::new(2);
    r.put(1, Vec2::new(-30.0, 20.0));
    doom(&mut r, 2, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.threat.state, ThreatState::Hauling);
    // A body in his sack is no body to kneel beside.
    let body = r.pose(2).pos;
    let above = Vec3::new(body.x, r.l.surface_height(body) + 0.3, body.y);
    r.stand(1, body + Vec2::new(0.0, 1.4), above);
    r.hold(1, 1.0);
    assert_eq!(r.s.players[&2].revive, 0.0);
    r.calm = true;
    r.put(1, Vec2::new(-30.0, 20.0));
    r.idle(r.t.haul_time + 1.0);
    assert!(matches!(r.s.players[&2].status, Status::Dead), "taken");
    assert!(r.events(1).contains(&Event::Taken));
    assert!(!r.s.encounter.outcome.is_over(), "one still stands");
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
    r.work(2, r.t.deliver_hold + 0.1);
    assert_eq!(r.s.encounter.progress.delivered(), 1);
    assert_eq!(r.s.encounter.progress.carried_by(2), 1);
    r.work(2, r.t.deliver_hold + 0.1);
    assert_eq!(r.s.encounter.progress.delivered(), 2);
    assert_eq!(r.s.encounter.progress.carried_by(2), 0);
    // Interrupted progress does not carry over to the next bundle.
    r.take(2, 2);
    stand_at_altar(&mut r, 2);
    r.work(2, r.t.deliver_hold * 0.6);
    r.idle(0.5);
    r.work(2, r.t.deliver_hold * 0.6);
    assert_eq!(r.s.encounter.progress.delivered(), 2, "a broken hold starts over");
    let snap = r.s.snapshot(2, &r.l, &r.t);
    assert_eq!(snap.world.delivered, 2);
    assert_eq!(snap.world.total as usize, r.l.district.relics.len());
}

#[test]
fn long_tasks_ask_for_skill_checks_a_miss_screeches_and_a_great_press_speeds_the_work() {
    let rig = |id: u64| {
        let mut r = Rig::new(1);
        r.calm = true;
        let pump = r.l.district.pump;
        r.stand(id, ground(pump) + Vec2::new(0.0, 1.45), pump);
        r
    };
    // Crank until the first check's needle is sweeping.
    let until_sweep = |r: &mut Rig| {
        for _ in 0..(8.0 / STEP) as usize {
            r.send(HOST, [0.0; 2], true, false, false);
            r.tick();
            if r.s.players[&HOST].rhythm.check.is_some_and(|c| c.t > 0.0) {
                return;
            }
        }
        panic!(
            "no skill check in 8 s of cranking (hold kind {}, power {})",
            r.s.players[&HOST].hold_kind, r.s.encounter.progress.power
        );
    };
    // Let it sweep past: the pump ends up behind where the warning found it,
    // and the hands are thrown off for a moment.
    let mut r = rig(HOST);
    let mut warned = None;
    for _ in 0..(8.0 / STEP) as usize {
        r.send(HOST, [0.0; 2], true, false, false);
        r.tick();
        if r.s.players[&HOST].rhythm.check.is_some() {
            warned = Some(r.s.encounter.progress.power);
            break;
        }
    }
    let warned = warned.expect("a skill check within 8 s of cranking");
    assert!(r.events(HOST).contains(&Event::SkillCheck), "the worker is warned");
    until_sweep(&mut r);
    let snap = r.s.snapshot(HOST, &r.l, &r.t);
    let c = snap.me.check.expect("the check is in the worker's snapshot");
    assert!(c.needle > 0.0 && c.zone >= r.t.check_zone_at.0);
    let fear = r.s.players[&HOST].body.fear;
    let mut missed = false;
    for _ in 0..(2.0 / STEP) as usize {
        r.send(HOST, [0.0; 2], true, false, false);
        r.tick();
        if r.s.players[&HOST].rhythm.check.is_none() {
            missed = true;
            break;
        }
    }
    assert!(missed, "the unanswered check swept past");
    let after = r.s.encounter.progress.power;
    assert!(
        after < warned,
        "a miss costs more than the check let him win: {after} after, {warned} at the warning"
    );
    assert!(r.s.players[&HOST].body.fear > fear, "the worker startles");
    assert!(r.events(HOST).contains(&Event::SkillMissed));
    // The crank kicks back: holding on does nothing for a moment...
    for _ in 0..(r.t.check_stall * 0.9 / STEP) as usize {
        r.send(HOST, [0.0; 2], true, false, false);
        r.tick();
        assert_eq!(
            r.s.encounter.progress.power, after,
            "no work while the hands are thrown off"
        );
    }
    // ...then the work goes on.
    r.hold(HOST, r.t.check_stall * 0.2 + 0.5);
    assert!(r.s.encounter.progress.power > after, "the work resumes");
    // A press at the start of the zone is great and speeds the work.
    let mut r = rig(HOST);
    until_sweep(&mut r);
    let c = r.s.players[&HOST].rhythm.check.unwrap();
    assert!(
        r.act(
            HOST,
            Action::Skill {
                id: c.id,
                needle: c.zone + 0.01
            }
        )
        .is_err(),
        "a needle from the future is refused"
    );
    while r.s.players[&HOST].rhythm.check.unwrap().needle(&r.t) < c.zone + 0.01 {
        r.send(HOST, [0.0; 2], true, false, false);
        r.tick();
    }
    let at = r.s.players[&HOST].rhythm.check.unwrap().needle(&r.t);
    let before = r.s.encounter.progress.power;
    r.act(HOST, Action::Skill { id: c.id, needle: at }).unwrap();
    assert!(r.s.encounter.progress.power >= before + r.t.check_bonus * 0.99);
    assert!(r.events(HOST).contains(&Event::SkillGreat));
    assert!(r.s.players[&HOST].rhythm.check.is_none());
    assert!(
        r.act(HOST, Action::Skill { id: c.id, needle: at }).is_err(),
        "one press per check"
    );
    // Letting go mid-check costs nothing.
    r.s.encounter.progress.power = 0.0;
    until_sweep(&mut r);
    let power = r.s.encounter.progress.power;
    r.idle(3.0);
    assert!(r.s.players[&HOST].rhythm.check.is_none());
    assert_eq!(r.s.encounter.progress.power, power);
}

/// Stand at a long task: the pump, or the ignition with everything it needs.
fn at_task(truck: bool) -> Rig {
    let mut r = Rig::new(1);
    r.calm = true;
    let site = if truck {
        for x in r.s.encounter.progress.relics.iter_mut() {
            *x = Relic::Delivered;
        }
        r.s.encounter.progress.power = 1.0;
        r.s.encounter.progress.key = true;
        r.l.district.ignition
    } else {
        r.l.district.pump
    };
    let side = if truck {
        Vec2::new(0.0, -1.7)
    } else {
        Vec2::new(0.0, 1.45)
    };
    r.stand(HOST, ground(site) + side, site);
    r
}

fn task_progress(r: &Rig, truck: bool) -> f32 {
    let p = &r.s.encounter.progress;
    if truck { p.truck } else { p.power }
}

#[test]
fn a_task_cannot_be_finished_under_a_pending_check() {
    for truck in [false, true] {
        let mut r = at_task(truck);
        for _ in 0..(8.0 / STEP) as usize {
            r.send(HOST, [0.0; 2], true, false, false);
            r.tick();
            if r.s.players[&HOST].rhythm.check.is_some() {
                break;
            }
        }
        assert!(r.s.players[&HOST].rhythm.check.is_some(), "a check came");
        // Nearly done: the last turn still waits on the rhythm.
        if truck {
            r.s.encounter.progress.truck = 0.97;
        } else {
            r.s.encounter.progress.power = 0.97;
        }
        let mut missed = false;
        for _ in 0..(3.0 / STEP) as usize {
            r.send(HOST, [0.0; 2], true, false, false);
            r.tick();
            if r.s.players[&HOST].rhythm.check.is_none() {
                missed = true;
                break;
            }
            assert!(
                task_progress(&r, truck) < 1.0,
                "finished under a pending check (truck: {truck})"
            );
        }
        assert!(missed, "the check swept past (truck: {truck})");
        assert!(r.events(HOST).contains(&Event::SkillMissed));
        assert!(
            task_progress(&r, truck) < 0.97,
            "the miss threw the work back (truck: {truck})"
        );
        let p = &r.s.encounter.progress;
        assert!(if truck { !p.truck_running() } else { !p.power_on() });
    }
}

#[test]
fn a_miss_carries_beyond_the_noise_of_the_work_it_spoils() {
    use el_silbon::sim::Variant;
    for truck in [false, true] {
        let mut r = at_task(truck);
        // A still, dry night: nothing masks either noise.
        r.t.rain_mask = 0.0;
        r.t.thunder_mask = 0.0;
        r.idle(0.1);
        let (site, work) = if truck {
            (ground(r.l.district.ignition), r.t.noise_truck)
        } else {
            (ground(r.l.district.pump), r.t.noise_pump)
        };
        let mut heard = false;
        for _ in 0..(12.0 / STEP) as usize {
            // He stands calm a quarter beyond where the work itself carries.
            let reach = work * r.t.hearing_gain(r.s.encounter.pressure) * Variant::of(r.t.seed).hearing();
            let th = &mut r.s.encounter.threat;
            th.state = ThreatState::Stalking;
            th.presence = Presence::Present;
            th.pos = site + Vec2::new(reach * 1.25, 0.0);
            th.focus = None;
            r.send(HOST, [0.0; 2], true, false, false);
            r.tick();
            if r.events(HOST).contains(&Event::SkillMissed) {
                assert!(
                    r.s.encounter.threat.focus.is_some(),
                    "he heard the miss over the work (truck: {truck})"
                );
                heard = true;
                break;
            }
            assert!(
                r.s.encounter.threat.focus.is_none(),
                "the work alone does not carry that far (truck: {truck})"
            );
        }
        assert!(heard, "the unanswered check was missed (truck: {truck})");
    }
}

#[test]
fn ignoring_the_rhythm_finishes_well_behind_keeping_it() {
    for truck in [false, true] {
        // Seconds of holding on until the task is done, and the misses.
        let finish = |answer: bool| {
            let mut r = at_task(truck);
            let mut misses = 0;
            for i in 1..=(60.0 / STEP) as usize {
                if answer {
                    r.work(HOST, STEP);
                } else {
                    r.hold(HOST, STEP);
                }
                misses += r.events(HOST).iter().filter(|e| **e == Event::SkillMissed).count();
                if task_progress(&r, truck) >= 1.0 {
                    return (i as f32 * STEP, misses);
                }
            }
            panic!("never finished (truck: {truck}, answering: {answer})");
        };
        let (clean, slips) = finish(true);
        let (sloppy, misses) = finish(false);
        assert_eq!(slips, 0, "a practised hand never misses");
        assert!(misses >= 1, "nobody ignores every check for free (truck: {truck})");
        let t = Tuning::default();
        assert!(
            sloppy >= clean + t.check_setback + t.check_stall,
            "{sloppy} s ignoring every check against {clean} s keeping the rhythm, {misses} misses (truck: {truck})"
        );
    }
}

#[test]
fn the_truck_key_is_padlocked_and_only_the_pages_numbers_open_it_at_the_box() {
    let mut r = Rig::new(1);
    r.calm = true;
    let code = el_silbon::sim::lock_code(r.t.seed);
    let wrong = [code[0] % 9 + 1, code[1], code[2]];
    // From across the yard, nothing can be tried.
    r.stand(HOST, Vec2::new(0.0, 20.0), r.l.district.lockbox);
    assert!(r.act(HOST, Action::TryCode { code }).is_err(), "out of reach");
    // At the box: a wrong combination rattles, loud enough for him.
    let at = r.l.district.lockbox;
    let spot = [
        Vec2::new(0.0, 1.5),
        Vec2::new(-1.5, 0.0),
        Vec2::new(0.0, -1.5),
        Vec2::new(1.5, 0.0),
    ]
    .into_iter()
    .map(|o| ground(at) + o)
    .find(|&p| {
        r.l.is_free(p, r.t.player_radius) && {
            let mut pose = Pose::at(p, 0.0);
            pose.look_at(&r.t, &r.l, at);
            matches!(
                r.l.aim(pose.eye(&r.t, &r.l), pose.look_dir(), at, 0.2, r.t.site_reach),
                el_silbon::geometry::AimStatus::Ready { .. }
            )
        }
    })
    .expect("somewhere to stand at the box");
    r.stand(HOST, spot, at);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = ground(at) + Vec2::new(0.0, -9.0);
    th.focus = None;
    r.act(HOST, Action::TryCode { code: wrong }).unwrap();
    assert!(!r.s.encounter.progress.key);
    assert!(r.events(HOST).contains(&Event::LockRattle));
    r.tick();
    assert!(r.s.encounter.threat.focus.is_some(), "he heard the rattle");
    assert!(
        r.act(HOST, Action::TryCode { code: [10, 0, 0] }).is_err(),
        "no such dial"
    );
    // The right one opens it, once, and everyone hears the key come free.
    r.act(HOST, Action::TryCode { code }).unwrap();
    assert!(r.s.encounter.progress.key);
    assert!(r.events(HOST).contains(&Event::KeyFound));
    assert!(r.s.snapshot(HOST, &r.l, &r.t).world.key);
    assert!(
        r.act(HOST, Action::TryCode { code }).is_err(),
        "the box is already open"
    );
}

/// A rig on a night when `variant` walks.
fn rig_for(variant: el_silbon::sim::Variant, players: u64) -> Rig {
    let seed = (1..500)
        .find(|&s| el_silbon::sim::Variant::of(s) == variant)
        .expect("every return has its nights");
    let mut r = Rig::new(players);
    r.t = Tuning::with_seed(seed);
    r.s = Session::new(&r.l, &r.t);
    for id in 2..=players {
        r.s.add_player(id, &r.l, &r.t).unwrap();
    }
    r.action_seq.clear();
    r.input_seq.clear();
    r.act(HOST, Action::Start).unwrap();
    r
}

#[test]
fn naming_him_rightly_at_the_ceiba_lays_him_to_rest_and_wrongly_enrages_him() {
    use el_silbon::sim::Variant;
    for tonight in Variant::ALL {
        let mut r = rig_for(tonight, 1);
        r.calm = true;
        stand_at_altar(&mut r, HOST);
        assert!(
            r.act(
                HOST,
                Action::Name {
                    variant: tonight.code()
                }
            )
            .is_err(),
            "not before every bone is home"
        );
        r.s.encounter.progress.relics.fill(Relic::Delivered);
        let wrong = Variant::ALL.into_iter().find(|v| *v != tonight).unwrap();
        r.act(HOST, Action::Name { variant: wrong.code() }).unwrap();
        assert!(r.events(HOST).contains(&Event::NameWrong));
        assert_ne!(r.s.encounter.threat.state, ThreatState::Dormant, "he comes, furious");
        assert!(
            r.act(
                HOST,
                Action::Name {
                    variant: tonight.code()
                }
            )
            .is_err(),
            "the ceiba is not listening yet"
        );
        assert_eq!(r.s.encounter.outcome, Outcome::Running);
        r.s.encounter.progress.naming_cooldown = 0.0;
        // From across the clearing, no name reaches the roots.
        r.put(HOST, ground(r.l.ceiba.offering) + Vec2::new(0.0, 14.0));
        assert!(
            r.act(
                HOST,
                Action::Name {
                    variant: tonight.code()
                }
            )
            .is_err()
        );
        stand_at_altar(&mut r, HOST);
        r.act(
            HOST,
            Action::Name {
                variant: tonight.code(),
            },
        )
        .unwrap();
        assert!(r.events(HOST).contains(&Event::Banished));
        assert_eq!(r.s.encounter.outcome, Outcome::Won);
        let snap = r.s.snapshot(HOST, &r.l, &r.t);
        assert!(snap.world.banished && snap.outcome == 1);
    }
}

#[test]
fn each_return_leaves_its_own_signs() {
    use el_silbon::sim::Variant;
    // The Son weeps when a bundle of his father's bones is laid down.
    for (v, weeps) in [(Variant::Hijo, true), (Variant::Arriero, false)] {
        let mut r = rig_for(v, 1);
        r.calm = true;
        r.take(HOST, 0);
        stand_at_altar(&mut r, HOST);
        r.work(HOST, r.t.deliver_hold + 0.3);
        assert_eq!(r.s.encounter.progress.delivered(), 1);
        assert_eq!(r.events(HOST).contains(&Event::Weeping), weeps, "{v:?}");
    }
    // While he walks the llano: a whip for the Drover, glass for the Drunkard.
    for (v, sign) in [
        (Variant::Arriero, Event::WhipCrack),
        (Variant::Borracho, Event::BottleClink),
    ] {
        let mut r = rig_for(v, 1);
        lurker(&mut r, Vec2::new(-30.0, 0.0));
        r.idle(r.t.tell_every.1 + 1.0);
        let seen = r.events(HOST);
        assert!(seen.contains(&sign), "{v:?} never gave its sign");
        let other = if sign == Event::WhipCrack {
            Event::BottleClink
        } else {
            Event::WhipCrack
        };
        assert!(!seen.contains(&other));
    }
    // The herd knows the Drover: it bellows when he passes, nobody near it.
    let mut r = rig_for(Variant::Arriero, 1);
    lurker(&mut r, Vec2::new(-30.0, 0.0));
    r.s.encounter.threat.pos = Vec2::new(35.0, -12.0);
    r.idle(1.0);
    assert!(r.events(HOST).contains(&Event::CattleSpooked));
}

#[test]
fn tureco_follows_whoever_unties_him_growls_when_he_is_near_and_barks_him_off() {
    let mut r = Rig::new(1);
    r.calm = true;
    let post = r.l.district.dog_post;
    let dog_at = Vec3::new(post.x, r.l.surface_height(post) + 0.45, post.y);
    r.stand(HOST, post + Vec2::new(0.0, 1.5), dog_at);
    r.hold(HOST, r.t.untie_hold + 0.2);
    assert!(
        r.s.dog.free && r.s.dog.owner == Some(HOST),
        "untied, and he knows whose he is"
    );
    assert!(r.events(HOST).contains(&Event::DogFreed));
    assert_eq!(r.s.snapshot(HOST, &r.l, &r.t).dog.mood, 1);
    // He keeps at your heels.
    r.walk(HOST, &[Vec2::new(-3.0, -8.0), Vec2::new(-10.0, -23.0)], (false, false));
    r.idle(1.5);
    assert!(r.s.dog.pos.distance(r.pose(HOST).pos) < 3.5, "Tureco follows");
    // He growls when he is near, true whatever the whistle says...
    let me = r.pose(HOST).pos;
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = r.s.dog.pos + Vec2::new(15.0, 0.0);
    th.movement = el_silbon::sim::Movement::Still;
    th.cooldown = 1.0e9;
    r.tick();
    assert!(r.events(HOST).contains(&Event::DogGrowl));
    assert_eq!(r.s.snapshot(HOST, &r.l, &r.t).dog.mood, 2);
    // ...and barks him off when he comes close: he flinches away.
    r.calm = false;
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Hunting;
    th.pos = r.s.dog.pos + Vec2::new(4.0, 0.0);
    r.tick();
    assert!(r.events(HOST).contains(&Event::DogBark));
    assert!(matches!(
        r.s.encounter.threat.presence,
        Presence::Sinking { relocate: true, .. }
    ));
    assert!(r.s.dog.courage > 0.0);
    // His courage needs time: close again at once, no second bark.
    r.idle(r.t.sink_time + r.t.rise_time + 0.2);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = r.s.dog.pos + Vec2::new(4.0, 0.0);
    r.tick();
    assert!(!r.events(HOST).contains(&Event::DogBark));
    assert!(matches!(r.s.encounter.threat.presence, Presence::Present));
    let _ = me;
}

#[test]
fn tureco_barks_the_sack_open() {
    let mut r = Rig::new(2);
    r.put(1, Vec2::new(-30.0, 20.0));
    r.s.dog.free = true;
    r.s.dog.owner = Some(1);
    doom(&mut r, 2, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.threat.state, ThreatState::Hauling);
    // Tureco catches up with him.
    r.s.dog.pos = r.s.encounter.threat.pos + Vec2::new(3.0, 0.0);
    r.tick();
    let ev = r.events(1);
    assert!(ev.contains(&Event::DogBark) && ev.contains(&Event::SackDropped));
    assert_ne!(r.s.encounter.threat.state, ThreatState::Hauling);
    assert!(
        !r.s.snapshot(1, &r.l, &r.t).player(2).unwrap().hauled,
        "out of the sack"
    );
    assert!(r.s.players[&2].status.is_downed(), "down, and can be helped up");
}

/// Pepper in his path while he hauls: he stops to count, and drops the sack.
fn ward_off(r: &mut Rig) {
    let th = r.s.encounter.threat.pos;
    let ahead = th + r.s.encounter.threat.facing * 2.5;
    r.s.encounter.place_aji(&r.t, ahead, &mut Vec::new());
    for _ in 0..(10.0 / STEP) as usize {
        r.tick();
        if r.s.encounter.threat.state == ThreatState::Counting {
            return;
        }
    }
    panic!("the ward never stopped him");
}

/// Player 2 falls with player 1 far off and standing: into his sack.
fn sacked(r: &mut Rig) {
    r.put(1, Vec2::new(-30.0, 20.0));
    doom(r, 2, Vec2::new(0.0, 8.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.threat.state, ThreatState::Hauling);
}

#[test]
fn a_body_in_his_sack_is_no_crosshair_target_until_he_drops_it() {
    let mut r = Rig::new(2);
    sacked(&mut r);
    // What a friend's own client offers the crosshair, from its snapshot.
    let bodies = |r: &Rig| -> Vec<u64> {
        let snap = r.s.snapshot(1, &r.l, &r.t);
        snap.scene_data(1, false).bodies.iter().map(|b| b.0).collect()
    };
    assert!(!bodies(&r).contains(&2), "the sack is no body to kneel beside");
    r.idle(1.0);
    ward_off(&mut r);
    assert!(bodies(&r).contains(&2), "dropped, they are a body to help up");
}

#[test]
fn a_dropped_sack_leaves_time_to_find_them() {
    // Late in the haul, dropped for pepper or for Tureco's bark: the fallen
    // lie where nobody saw them fall, and get time to be found.
    for bark in [false, true] {
        let mut r = Rig::new(2);
        if bark {
            r.s.dog.free = true;
            r.s.dog.owner = Some(1);
        }
        sacked(&mut r);
        r.idle(1.0);
        r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 5.0 };
        if bark {
            r.s.dog.pos = r.s.encounter.threat.pos + Vec2::new(3.0, 0.0);
            r.tick();
            assert!(r.events(1).contains(&Event::SackDropped));
        } else {
            ward_off(&mut r);
        }
        let Status::Downed { bleed } = r.s.players[&2].status else {
            panic!("still down (bark: {bark})")
        };
        assert!(
            bleed >= r.t.bleed_after_sack - 1e-3,
            "{bleed:.1} s left to find them (bark: {bark})"
        );
    }
    // It is a floor: a fall caught at once keeps its own, longer bleed.
    let mut r = Rig::new(2);
    sacked(&mut r);
    r.idle(1.0);
    r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 50.0 };
    ward_off(&mut r);
    let Status::Downed { bleed } = r.s.players[&2].status else {
        panic!("still down")
    };
    assert!(bleed > 45.0, "{bleed:.1} s: the floor never takes time away");
}

const HELP: Action = Action::Call { kind: CallKind::Help };

/// Player 2 lies downed on open ground (not in his sack), player 1 stands
/// far off out of the way, and the night is still and dry.
fn fallen() -> (Rig, Vec2) {
    let mut r = Rig::new(2);
    r.t.rain_mask = 0.0;
    r.t.thunder_mask = 0.0;
    let body = Vec2::new(0.0, 12.0);
    r.put(1, Vec2::new(-30.0, 20.0));
    r.put(2, body);
    r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 40.0 };
    r.idle(0.1);
    (r, body)
}

/// He stands calm and listening `dist` metres from `at`, going nowhere.
fn listening(r: &mut Rig, at: Vec2, dist: f32) {
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = at + Vec2::new(dist, 0.0);
    th.movement = el_silbon::sim::Movement::AtAnchor(r.l.patrol.nearest(th.pos));
    th.cooldown = 1.0e9;
    th.focus = None;
}

#[test]
fn a_cry_for_help_comes_from_the_body_and_draws_him_within_earshot() {
    use el_silbon::sim::Variant;
    for (inside, heard) in [(0.8, true), (1.25, false)] {
        let (mut r, body) = fallen();
        let reach = r.t.noise_call * r.t.hearing_gain(r.s.encounter.pressure) * Variant::of(r.t.seed).hearing();
        listening(&mut r, body, reach * inside);
        r.tick();
        assert!(r.s.encounter.threat.focus.is_none(), "nothing else he heard");
        r.act(2, HELP).expect("the fallen may call");
        r.tick();
        let focus = r.s.encounter.threat.focus;
        assert_eq!(focus.is_some(), heard, "a cry {inside} × its reach away");
        if let Some(f) = focus {
            assert!(f.pos.distance(body) < 0.01, "he heard it where they lie");
        }
        // Everyone is told of it, placed at the caller's own body.
        let snap = r.s.snapshot(1, &r.l, &r.t);
        let call = snap.calls.iter().find(|c| c.by == 2).expect("the call is shared");
        assert_eq!(call.kind, CallKind::Help);
        let at = Vec3::from_array(call.pos);
        assert!(ground(at).distance(body) < 0.01);
        assert!(
            (at.y - r.l.rest_height(body)).abs() < 0.01,
            "on the ground, not in the air"
        );
    }
}

#[test]
fn a_cry_for_help_waits_for_breath_and_fades() {
    let (mut r, body) = fallen();
    r.act(2, HELP).unwrap();
    assert!(r.act(2, HELP).is_err(), "the voice needs breath");
    r.idle(r.t.call_cooldown - 1.0);
    assert!(r.act(2, HELP).is_err(), "still spent");
    assert!(r.s.snapshot(1, &r.l, &r.t).calls.is_empty(), "the call has faded");
    r.idle(1.1);
    r.act(2, HELP).expect("breath enough to call again");
    let calls = r.s.snapshot(1, &r.l, &r.t).calls;
    assert_eq!(calls.len(), 1);
    assert!(ground(Vec3::from_array(calls[0].pos)).distance(body) < 0.01);
}

#[test]
fn nobody_cries_for_help_from_his_sack_the_grave_or_their_feet() {
    let mut r = Rig::new(2);
    sacked(&mut r);
    assert!(r.act(2, HELP).is_err(), "in his sack, nobody calls");
    // A friend's view: in his sack, nothing leads to the fallen.
    let snap = r.s.snapshot(1, &r.l, &r.t);
    assert!(snap.calls.is_empty());
    assert!(!snap.player(2).unwrap().findable(), "no cue while he carries them");
    r.idle(1.0);
    ward_off(&mut r);
    assert!(
        r.s.snapshot(1, &r.l, &r.t).player(2).unwrap().findable(),
        "dropped, they can be found"
    );
    r.act(2, HELP).expect("out of the sack, they may call");
    // On your feet, the cry is not yours to give.
    assert!(r.act(1, HELP).is_err(), "standing, nobody cries for help");
    // The dead are past calling.
    r.calm = true;
    r.s.players.get_mut(&2).unwrap().status = Status::Dead;
    r.idle(r.t.call_cooldown + 0.1);
    assert!(!r.s.encounter.outcome.is_over(), "one still stands");
    assert!(r.act(2, HELP).is_err(), "the dead are past calling");
    assert!(!r.s.snapshot(1, &r.l, &r.t).player(2).unwrap().findable());
}

#[test]
fn a_taken_player_is_not_left_frozen_with_fright() {
    let mut r = Rig::new(2);
    // A susto froze them just before he caught them; in his sack the body
    // never moves, so nothing thaws it there.
    r.s.players.get_mut(&2).unwrap().body.stun = 2.0;
    sacked(&mut r);
    r.calm = true;
    r.idle(r.t.haul_time + 1.0);
    assert!(matches!(r.s.players[&2].status, Status::Dead), "taken");
    let stun = r.s.snapshot(2, &r.l, &r.t).me.stun;
    assert_eq!(stun, 0.0, "the dead are not frozen with fright ({stun:.2} s)");
}

/// Whom `id` watches, as their own snapshot says (0 for nobody).
fn watching(r: &Rig, id: u64) -> u64 {
    r.s.snapshot(id, &r.l, &r.t).player(id).unwrap().watching
}

#[test]
fn the_fallen_watch_a_friend_on_their_feet() {
    let mut r = Rig::new(4);
    r.put(1, Vec2::new(-20.0, 20.0));
    r.put(2, Vec2::new(30.0, 20.0));
    r.put(3, Vec2::new(60.0, 20.0));
    r.put(4, Vec2::new(-24.0, 20.0));
    r.s.players.get_mut(&4).unwrap().status = Status::Downed { bleed: 0.5 };
    assert!(
        r.act(4, Action::Watch { id: 1 }).is_err(),
        "the downed still have their own eyes"
    );
    r.idle(1.0);
    assert!(matches!(r.s.players[&4].status, Status::Dead));
    assert_eq!(watching(&r, 4), 1, "gone, they watch the friend standing nearest");
    let snap = r.s.snapshot(4, &r.l, &r.t);
    assert_eq!(
        watched(&snap, Some(4)).map(|p| p.id),
        Some(1),
        "their eye is that friend's"
    );
    assert!(
        watched(&r.s.snapshot(1, &r.l, &r.t), Some(1)).is_none(),
        "the living use their own"
    );
    // They choose whom, stepping round the friends on their feet (never
    // themselves).
    assert_eq!(cycle_watch(&snap, 4, -1), Some(3));
    assert_eq!(cycle_watch(&snap, 4, 1), Some(2));
    r.act(4, Action::Watch { id: 2 }).expect("a friend on their feet");
    assert_eq!(watching(&r, 4), 2);
    assert!(r.act(4, Action::Watch { id: 4 }).is_err(), "not themselves");
    assert!(r.act(4, Action::Watch { id: 9 }).is_err(), "nobody who is not here");
    assert!(
        r.act(1, Action::Watch { id: 2 }).is_err(),
        "the living have their own eyes"
    );
    r.idle(0.5);
    assert_eq!(watching(&r, 4), 2, "the choice holds while that friend stands");
    // The watched friend leaves: nothing breaks, nothing of him shows, and
    // the next moment the watch moves on.
    r.s.remove_player(2);
    assert!(r.s.snapshot(4, &r.l, &r.t).threat.is_none());
    r.tick();
    assert_eq!(watching(&r, 4), 1);
    // The watched friend falls into his sack: the watch moves on to one
    // still standing, and the sack is not a friend to watch.
    r.act(4, Action::Watch { id: 3 }).unwrap();
    doom(&mut r, 3, Vec2::new(60.0, 20.0));
    r.idle(0.05);
    assert_eq!(r.s.encounter.threat.state, ThreatState::Hauling);
    assert_eq!(watching(&r, 4), 1, "their friend fell: the watch moved on");
    assert!(r.act(4, Action::Watch { id: 3 }).is_err(), "not the one in his sack");
    // A new night: everyone has their own eyes again.
    r.act(HOST, Action::Restart).unwrap();
    assert!(r.s.players.values().all(|p| p.watch.is_none()));
    assert_eq!(watching(&r, 4), 0);
}

/// He stands present and warning at `at`, going nowhere.
fn warning_at(r: &mut Rig, at: Vec2) {
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Warning;
    th.presence = Presence::Present;
    th.pos = at;
}

#[test]
fn the_fallen_see_him_only_through_their_friends_eyes() {
    let mut r = Rig::new(2);
    // Player 2 is gone for the night and lies far off; player 1 stands and
    // faces him as he warns them.
    r.put(2, Vec2::new(-30.0, 20.0));
    r.s.players.get_mut(&2).unwrap().status = Status::Dead;
    let him = Vec2::new(0.0, 15.0);
    warning_at(&mut r, him);
    r.stand(1, Vec2::new(0.0, 8.0), Vec3::new(him.x, 2.0, him.y));
    r.tick();
    r.s.encounter.threat.pos = him;
    let friend = r.s.snapshot(1, &r.l, &r.t);
    let fallen = r.s.snapshot(2, &r.l, &r.t);
    assert!(
        friend.threat.is_some() && matches!(friend.danger, 1..=3),
        "the friend sees him come"
    );
    let seen = fallen.threat.expect("the fallen see him through their friend's eyes");
    assert_eq!(seen.position, friend.threat.unwrap().position);
    assert_eq!(fallen.danger, friend.danger, "and feel the danger the friend is in");
    assert_eq!(fallen.exposure, friend.exposure);
    // Without `anima_sight` the fallen watch their friend panic at nothing.
    r.t.anima_sight = false;
    let blind = r.s.snapshot(2, &r.l, &r.t);
    assert!(blind.threat.is_none() && blind.danger == 0 && blind.exposure == 0.0);
    r.t.anima_sight = true;
    // The friend looks away: he is behind the fallen's eyes too.
    r.s.players.get_mut(&1).unwrap().pose.yaw += std::f32::consts::PI;
    assert!(r.s.snapshot(1, &r.l, &r.t).threat.is_none());
    assert!(r.s.snapshot(2, &r.l, &r.t).threat.is_none(), "behind the friend");
    // A wall between the friend and him hides him from both, even where the
    // fallen body itself faces him with nothing in the way.
    let him = Vec2::new(0.0, -4.0);
    r.s.encounter.threat.pos = him;
    r.stand(1, Vec2::new(8.0, -4.0), Vec3::new(him.x, 2.0, him.y));
    r.stand(2, Vec2::new(0.0, -1.0), Vec3::new(him.x, 2.0, him.y));
    assert!(r.l.line_of_sight(r.pose(2).pos, him), "the body itself has him in view");
    assert!(r.s.snapshot(1, &r.l, &r.t).threat.is_none());
    assert!(
        r.s.snapshot(2, &r.l, &r.t).threat.is_none(),
        "never what the fallen's own eye would see"
    );
}

#[test]
fn the_fallen_hear_the_whistle_their_friend_hears() {
    let mut r = Rig::new(2);
    // The host is gone for the night, far from him; player 2 stands near him.
    r.s.players.get_mut(&HOST).unwrap().status = Status::Dead;
    warning_at(&mut r, Vec2::new(0.0, 10.0));
    r.put(HOST, Vec2::new(-35.0, 20.0));
    r.put(2, Vec2::new(0.0, 8.0));
    r.tick();
    let cues = |r: &Rig, id: u64| -> Vec<(u64, u8, f32, bool, u8)> {
        r.s.outbox
            .iter()
            .filter_map(|(to, m)| match m {
                ServerMessage::Cue {
                    serial,
                    variant,
                    speed,
                    phantom,
                    take,
                    ..
                } if *to == id => Some((*serial, *variant, *speed, *phantom, *take)),
                _ => None,
            })
            .collect()
    };
    let friend = cues(&r, 2);
    assert_eq!(friend.len(), 1, "the friend hears him");
    // Near him the friend hears it faint; from where the host lies it would
    // have been loud. The fallen hear the friend's, whistle for whistle.
    assert_eq!(
        cues(&r, HOST),
        friend,
        "the fallen hear exactly what their friend hears"
    );
}

#[test]
fn the_dynamo_carries_two_lines_and_the_panel_chooses_which() {
    use el_silbon::geometry::district::{ALL_CIRCUITS, FIRST_CIRCUITS};
    let mut r = Rig::new(1);
    r.calm = true;
    let panel = r.l.district.panel;
    let spot = ground(panel) + Vec2::new(0.0, 1.3);
    r.stand(HOST, spot, panel);
    assert!(r.act(HOST, Action::Interact).is_err(), "no power, nothing to switch");
    // The bridge's lamps: dark until their line is switched on.
    let bridge = Vec2::new(42.0, 26.6);
    assert!(r.l.is_lit(bridge, ALL_CIRCUITS) && !r.l.is_lit(bridge, FIRST_CIRCUITS));
    r.s.encounter.progress.power = 1.0;
    r.idle(0.1);
    assert_eq!(r.s.snapshot(HOST, &r.l, &r.t).world.circuits, FIRST_CIRCUITS);
    r.stand(HOST, spot, panel);
    r.act(HOST, Action::Interact).expect("the panel is in reach");
    let now = r.s.encounter.progress.circuits;
    assert!(now & 0b100 != 0, "the bridge line is live");
    assert_eq!(now.count_ones(), 2, "never all three");
    assert!(r.events(HOST).contains(&Event::LinesSwitched));
    // Every setting is two lines, and the switch comes round again.
    for _ in 0..3 {
        r.act(HOST, Action::Interact).unwrap();
        assert_eq!(r.s.encounter.progress.circuits.count_ones(), 2);
    }
    assert_eq!(r.s.encounter.progress.circuits, now);
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
    assert!(!r.l.is_lit(far, 0) || r.l.is_lit(far, el_silbon::geometry::district::ALL_CIRCUITS));
    // One player alone takes the full pump_hold; two take half. Both keep
    // the rhythm; each great press adds its own bonus on top of the work.
    r.work(1, r.t.pump_hold * 0.5);
    let greats = r.events(1).iter().filter(|e| **e == Event::SkillGreat).count();
    let solo = r.s.encounter.progress.power - greats as f32 * r.t.check_bonus;
    assert!((solo - 0.5).abs() < 0.05, "{solo} after {greats} great presses");
    r.work_all(&[1, 2], r.t.pump_hold * 0.3);
    assert!(
        r.s.encounter.progress.power_on(),
        "two crankers finish in well under pump_hold"
    );
    assert!(r.events(2).contains(&Event::PowerRestored));
    assert!(
        r.l.is_lit(Vec2::new(-24.0, 27.0), el_silbon::geometry::district::ALL_CIRCUITS),
        "powered pole lights the road once on"
    );
    assert!(!r.l.is_lit(Vec2::new(-24.0, 27.0), 0));
}

#[test]
fn the_truck_waits_for_bones_and_power_then_its_engine_is_the_finale() {
    let mut r = Rig::new(2);
    let ign = r.l.district.ignition;
    let stand = ground(ign) + Vec2::new(0.0, -1.7);
    r.stand(1, stand, ign);
    r.stand(2, stand + Vec2::new(1.0, 0.0), ign);
    r.work(1, r.t.truck_hold + 1.0);
    assert_eq!(r.s.encounter.progress.truck, 0.0, "no bones, no power");
    for x in r.s.encounter.progress.relics.iter_mut() {
        *x = Relic::Delivered;
    }
    r.hold(1, 2.0);
    assert_eq!(r.s.encounter.progress.truck, 0.0, "power still missing");
    r.s.encounter.progress.power = 1.0;
    r.hold(1, 2.0);
    assert_eq!(r.s.encounter.progress.truck, 0.0, "the key is still padlocked away");
    r.s.encounter.progress.key = true;
    r.work(1, r.t.truck_hold + 1.0);
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

/// The engine warm and ready, the threat kept off.
fn ready_truck(r: &mut Rig) {
    r.calm = true;
    let progress = &mut r.s.encounter.progress;
    for x in progress.relics.iter_mut() {
        *x = Relic::Delivered;
    }
    progress.power = 1.0;
    progress.key = true;
    progress.truck = 1.0;
    progress.warm = r.t.truck_warmup;
}

#[test]
fn whoever_is_aboard_the_ready_truck_can_drive_off_without_the_others() {
    // Alone there is nobody to leave: the ordinary boarding rule stands.
    let mut solo = Rig::new(1);
    ready_truck(&mut solo);
    let zone = solo.l.district.truck;
    solo.put(1, zone.zone_center + Vec2::new(zone.zone_radius + 5.0, 0.0));
    assert!(solo.act(1, Action::DriveOff).is_err());

    let mut r = Rig::new(3);
    let zone = r.l.district.truck;
    let outside = zone.zone_center + Vec2::new(zone.zone_radius + 5.0, 0.0);
    r.put(1, zone.zone_center);
    r.put(2, zone.zone_center + Vec2::new(0.8, 0.0));
    r.put(3, outside);
    assert!(r.act(1, Action::DriveOff).is_err(), "not before the truck runs");
    ready_truck(&mut r);
    r.put(1, outside);
    assert!(r.act(1, Action::DriveOff).is_err(), "only from aboard");
    r.put(1, zone.zone_center);
    r.s.players.get_mut(&2).unwrap().status = Status::Downed { bleed: 30.0 };
    assert!(r.act(2, Action::DriveOff).is_err(), "not by the fallen");
    assert_eq!(r.s.encounter.outcome, Outcome::Running);
    r.act(1, Action::DriveOff).expect("aboard and ready");
    assert_eq!(r.s.encounter.outcome, Outcome::Won);
    // The fallen one in the zone and the one outside are both left.
    let snap = r.s.snapshot(3, &r.l, &r.t);
    assert_eq!(snap.left_behind, vec![2, 3]);
    r.idle(0.05);
    assert!(r.events(1).contains(&Event::Escaped));
    assert!(r.act(1, Action::DriveOff).is_err(), "the night is over");
    // A new night leaves nobody behind.
    r.act(HOST, Action::Restart).unwrap();
    assert!(r.s.snapshot(3, &r.l, &r.t).left_behind.is_empty());
    assert_eq!(r.s.encounter.outcome, Outcome::Running);
}

#[test]
fn the_night_remembers_what_each_player_did_and_tells_it_once_it_is_over() {
    let mut r = Rig::new(2);
    r.calm = true;
    for _ in 0..2 {
        r.s.encounter.progress.relics[1] = Relic::Carried(2);
        r.act(2, Action::Drop).expect("carrying");
    }
    assert!(
        r.s.snapshot(2, &r.l, &r.t).deeds.is_empty(),
        "nothing is told mid-night"
    );
    ready_truck(&mut r);
    let zone = r.l.district.truck;
    r.put(1, zone.zone_center);
    r.put(2, zone.zone_center + Vec2::new(zone.zone_radius + 5.0, 0.0));
    r.act(1, Action::DriveOff).expect("aboard and ready");
    let snap = r.s.snapshot(2, &r.l, &r.t);
    let deeds = |id: u64| snap.deeds.iter().find(|(p, _)| *p == id).map(|(_, d)| *d).unwrap();
    assert_eq!(deeds(2).drops, 2);
    assert_eq!(deeds(1).drops, 0);
    // A new night starts with a clean slate.
    r.act(HOST, Action::Restart).unwrap();
    assert!(r.s.players.values().all(|p| p.deeds == Default::default()));
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
fn a_torch_runs_down_and_dies_and_spare_batteries_bring_it_back() {
    let mut r = Rig::new(1);
    r.calm = true;
    r.lit = true;
    r.idle(1.0);
    let fresh = r.s.players[&HOST].battery;
    assert!(fresh < 1.0 && r.s.players[&HOST].light, "a lit torch drains");
    // Switched off, it keeps its charge.
    r.lit = false;
    r.idle(1.0);
    assert_eq!(r.s.players[&HOST].battery, fresh);
    // Nearly flat: it dies, and a dead torch gives no light whatever the switch says.
    r.lit = true;
    r.s.players.get_mut(&HOST).unwrap().battery = 0.002;
    r.idle(2.0);
    assert_eq!(r.s.players[&HOST].battery, 0.0);
    assert!(!r.s.players[&HOST].light);
    assert_eq!(r.s.snapshot(HOST, &r.l, &r.t).me.battery, 0.0);
    // Spare batteries revive it, once.
    let at = r.l.district.batteries[2];
    r.stand(HOST, ground(at) + Vec2::new(0.0, 1.8), at);
    r.act(HOST, Action::Interact).expect("the spare batteries are in reach");
    assert!((r.s.players[&HOST].battery - r.t.battery_pickup).abs() < 1e-5);
    assert!(r.s.encounter.progress.batteries_taken[2]);
    assert!(r.events(HOST).contains(&Event::BatteriesTaken));
    r.idle(0.1);
    assert!(r.s.players[&HOST].light);
    assert!(r.act(HOST, Action::Interact).is_err(), "already taken");
    // A fresh torch leaves spares where they lie.
    let other = r.l.district.batteries[3];
    r.s.players.get_mut(&HOST).unwrap().battery = 1.0;
    r.stand(HOST, ground(other) + Vec2::new(0.0, 1.8), other);
    assert!(r.act(HOST, Action::Interact).is_err());
    assert!(!r.s.encounter.progress.batteries_taken[3]);
}

#[test]
fn a_lit_torch_he_can_see_draws_him_from_beyond_his_notice() {
    let node = Vec2::new(35.0, 2.0);
    let l = Layout::new();
    let t = Tuning::default();
    // Somewhere in the open he can see, farther than he would notice a
    // lit player but within the torch's lure.
    let notice = t.warn_distance * t.sight_light;
    let spot = (0..200)
        .flat_map(|i| (0..60).map(move |k| (i, k)))
        .map(|(i, k)| {
            let a = k as f32 / 60.0 * std::f32::consts::TAU;
            node + Vec2::new(a.cos(), a.sin()) * (notice + 3.0 + i as f32 * 0.05)
        })
        .find(|&p| {
            p.distance(node) < t.light_lure_range - 2.0
                && l.bounds.contains(p)
                && l.is_free(p, t.player_radius)
                && l.line_of_sight(p, node)
        })
        .expect("an open spot in his view");
    for (lit, drawn) in [(true, true), (false, false)] {
        let mut r = Rig::new(1);
        lurker(&mut r, spot);
        r.lit = lit;
        r.idle(0.5);
        assert_eq!(
            r.s.encounter.threat.focus.is_some(),
            drawn,
            "torch {} at {:.0} m",
            if lit { "on" } else { "off" },
            spot.distance(node)
        );
        assert_ne!(
            r.s.encounter.threat.state,
            ThreatState::Warning,
            "too far to be noticed"
        );
    }
}

#[test]
fn sprinting_is_loud_walking_and_crouching_are_not() {
    for (mode, loud) in [((false, true), true), ((false, false), false), ((true, false), false)] {
        let mut r = Rig::new(1);
        lurker(&mut r, Vec2::new(34.0, 16.0));
        assert_eq!(shuffle(&mut r, mode), loud, "mode (crouch, sprint) = {mode:?}");
    }
}

/// Walk from `from` toward `to` for three seconds on a still, dry night with
/// him calm and listening a fixed way off: how far the walker got, and
/// whether he heard a step.
fn wade(from: Vec2, to: Vec2, listen: f32) -> (f32, bool) {
    use el_silbon::sim::Variant;
    let mut r = Rig::new(1);
    r.calm = true;
    r.t.rain_mask = 0.0;
    r.t.thunder_mask = 0.0;
    r.put(HOST, from);
    let mut heard = false;
    for _ in 0..(3.0 / STEP) as usize {
        let pos = r.pose(HOST).pos;
        let reach = listen * r.t.hearing_gain(r.s.encounter.pressure) * Variant::of(r.t.seed).hearing();
        let th = &mut r.s.encounter.threat;
        th.state = ThreatState::Stalking;
        th.presence = Presence::Present;
        th.pos = pos + Vec2::new(0.0, reach);
        th.focus = None;
        r.s.players.get_mut(&HOST).unwrap().pose.yaw = Pose::yaw_toward(pos, to);
        r.send(HOST, [0.0, 1.0], false, false, false);
        r.tick();
        heard |= r.s.encounter.threat.focus.is_some();
    }
    (r.pose(HOST).pos.distance(from), heard)
}

#[test]
fn wading_the_deep_channel_is_slow_and_heard_farther_than_the_ford() {
    let t = Tuning::default();
    // Between a ford step and a deep one.
    let listen = t.noise_walk + 0.5 * (t.noise_wade + t.noise_deep);
    let (deep, splash) = wade(Vec2::new(30.0, -65.5), Vec2::new(38.0, -65.5), listen);
    let expected = t.walk_speed * t.deep_wade_factor * 3.0;
    assert!(
        (deep - expected).abs() < expected * 0.05,
        "waded {deep:.2} m in 3 s, not {expected:.2}"
    );
    assert!(splash, "he did not hear the channel waded");
    let (ford, wet) = wade(Vec2::new(0.0, -62.0), Vec2::new(0.0, -69.0), listen);
    assert!(ford > deep * 1.4, "the ford ({ford:.2} m) is quicker than the channel");
    assert!(!wet, "the ford carries no farther than a wading step");
}

#[test]
fn what_falls_in_the_channel_floats() {
    let afloat = el_silbon::geometry::district::WATER_LEVEL - 0.2 + 0.35 - 1e-3;
    let deep = Vec2::new(34.0, -65.5);
    // Put down by hand.
    let mut r = Rig::new(1);
    r.calm = true;
    r.take(HOST, 0);
    r.put(HOST, deep);
    r.idle(0.1);
    r.act(HOST, Action::Drop).unwrap();
    let Relic::Ground(at) = r.s.encounter.progress.relics[0] else {
        panic!("not dropped")
    };
    assert!(at.y >= afloat, "the dropped bundle sank: {at:?}");
    // Let go by someone struck down in the water.
    let mut r = Rig::new(2);
    r.take(2, 0);
    r.put(1, Vec2::new(-30.0, 20.0));
    doom(&mut r, 2, deep);
    r.idle(0.05);
    assert!(r.s.players[&2].status.is_downed());
    let Relic::Ground(at) = r.s.encounter.progress.relics[0] else {
        panic!("not released")
    };
    assert!(at.y >= afloat, "the released bundle sank: {at:?}");
}

/// The session steps at which he warned, hunted and felled a lone player
/// keeping still at `at` (crouched or not), from `off` away. He starts calm,
/// still and ready to warn.
fn fate(at: Vec2, off: Vec2, crouch: bool) -> [Option<usize>; 3] {
    let mut r = Rig::new(1);
    r.put(HOST, at);
    let th = &mut r.s.encounter.threat;
    th.state = ThreatState::Stalking;
    th.presence = Presence::Present;
    th.pos = at + off;
    th.movement = el_silbon::sim::Movement::Still;
    th.cooldown = 0.0;
    let mut when = [None; 3];
    for step in 0..(30.0 / STEP) as usize {
        r.send(HOST, [0.0; 2], false, crouch, false);
        r.tick();
        for e in r.events(HOST) {
            let k = match e {
                Event::WarningBegan => 0,
                Event::HuntBegan => 1,
                Event::Downed => 2,
                _ => continue,
            };
            when[k].get_or_insert(step);
        }
        if r.s.encounter.outcome.is_over() {
            break;
        }
    }
    when
}

/// Waist-deep in the caño is no refuge. In his view a player there is warned,
/// hunted and caught on the very steps he would be on dry ground, standing or
/// crouched: the water conceals no one the way tall grass does, and he wades
/// in after them unslowed. (Their own steps there carry farther: see
/// `wading_the_deep_channel_is_slow_and_heard_farther_than_the_ford`.)
#[test]
fn the_cano_hides_no_one() {
    let (l, t) = (Layout::new(), Tuning::default());
    // Near enough to be noticed even crouched, farther than tall grass
    // would let him see a crouched player.
    let off = Vec2::new(0.0, 10.0);
    assert!(off.length() < t.warn_distance * t.sight_crouch && off.length() > t.grass_sight);
    // In plain view, clear for his whole walk in, unlit, and far from Tureco.
    let open = |at: Vec2| {
        l.line_of_sight(at, at + off)
            && (0..=40).all(|k| l.is_free(at + off * (k as f32 / 40.0), 0.45))
            && !l.is_lit(at, 0)
            && !l.district.tall_grass_at(at)
            && at.distance(l.district.dog_post) > t.growl_range + off.length() + 5.0
    };
    let wet = Vec2::new(34.0, -65.5);
    assert_eq!(l.wade(wet), Wade::Deep);
    assert!(open(wet), "him on the north bank, in plain view of the channel");
    let dry = (-60..=60)
        .flat_map(|x| (-50..=40).map(move |y| Vec2::new(x as f32, y as f32)))
        .find(|&p| {
            l.bounds.contains(p + off) && (0..=40).all(|k| l.wade(p + off * (k as f32 / 40.0)) == Wade::Dry) && open(p)
        })
        .expect("open dry ground");
    for crouch in [false, true] {
        let (w, d) = (fate(wet, off, crouch), fate(dry, off, crouch));
        assert!(
            w.iter().all(Option::is_some),
            "crouched {crouch}: warned, hunted and caught waist-deep: {w:?}"
        );
        assert_eq!(
            w, d,
            "crouched {crouch}: waist-deep at {wet} and on dry ground at {dry}"
        );
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
    assert!(!alone.l.is_lit(dark, 0));
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
    assert!(lit.l.is_lit(Vec2::new(-4.0, 24.0), 0));
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
    r.work(1, r.t.deliver_hold + 0.2);
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
        r.work(1, r.t.deliver_hold + 0.2);
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
    r.work(1, r.t.pump_hold + 0.5);
    assert!(r.s.encounter.progress.power_on());
    assert!(r.s.players[&1].pose.pos.distance(ground(d.pump)) < 2.0);

    // Both players to the truck: player 2 comes down the road from the spawn.
    // The key box on the windmill's crates, with the numbers from the pages.
    r.walk(1, &[Vec2::new(-37.0, -23.3)], walk);
    r.look(1, d.lockbox);
    let code = el_silbon::sim::lock_code(r.t.seed);
    r.act(1, Action::TryCode { code })
        .expect("the box opens with the pages' numbers");
    assert!(r.s.encounter.progress.key);
    // Back round the trough the way we came.
    r.walk(1, &[Vec2::new(-42.0, -18.0)], walk);
    let mut to_truck = r.route(tower_approach, Vec2::new(55.0, 30.0));
    to_truck.push(Vec2::new(55.7, 29.2));
    r.walk(1, &to_truck, walk);
    r.look(1, d.ignition);
    r.work(1, r.t.truck_hold + 0.5);
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
        if std::env::var_os("ROUTE_LOG").is_some()
            && let Some(msg) = &step.log
        {
            eprintln!(
                "[{:.1}s p{}] {msg} (battery {:.2}, danger {}, pressure-ish night {:.0}s)",
                self.script.elapsed(),
                self.id,
                self.view.me.battery,
                self.view.danger,
                self.view.elapsed
            );
        }
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
        let wire = Wire::new(&step.intent, live, &self.pose, !step.dark, self.hud, l, t);
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
    let l = Layout::with_seed(t.seed);
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
    let (l, t) = (Layout::with_seed(t.seed), &t);
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

/// The night a diagnostic or the sweep measures: `ROUTE_NIGHT=gentle|hard`
/// (normal by default).
fn route_night() -> el_silbon::tuning::Night {
    std::env::var("ROUTE_NIGHT")
        .ok()
        .and_then(|v| el_silbon::tuning::Night::parse(&v))
        .unwrap_or_default()
}

/// Opt-in measurement for work on the route script: the solo and shared
/// routes over many storms, each with its own pattern of missed frames. The
/// driver plays a chaotic game, so one seed passing proves little; this
/// counts. `cargo test --locked --test session -- --ignored --nocapture`
/// One seed of the solo route with its log: `ROUTE_LOG=1 ROUTE_SEED=n`
/// (and `ROUTE_NIGHT`).
#[test]
#[ignore = "diagnostic"]
fn one_solo_seed() {
    let seed: u64 = std::env::var("ROUTE_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    solo_route(
        Tuning::with_seed(seed).with_night(route_night()),
        seed,
        RouteScript::full,
        &[Outcome::Won, Outcome::Failed],
    );
}

/// One seed of the shared route with its log: `ROUTE_LOG=1 ROUTE_SEED=n`
/// (and `ROUTE_NIGHT`).
#[test]
#[ignore = "diagnostic"]
fn one_shared_seed() {
    let seed: u64 = std::env::var("ROUTE_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    network_route(Tuning::with_seed(seed).with_night(route_night()), seed);
}

#[test]
#[ignore = "slow: 300 full routes"]
fn the_routes_hold_over_many_storms() {
    let n = 150;
    // `ROUTE_NIGHT=gentle|hard` measures another night (the gate is normal).
    let night = route_night();
    std::panic::set_hook(Box::new(|_| {}));
    let why = |e: Box<dyn std::any::Any + Send>| e.downcast_ref::<String>().cloned().unwrap_or_default();
    let (mut solo, mut shared) = (0, 0);
    for seed in 1..=n {
        match std::panic::catch_unwind(|| {
            solo_route(
                Tuning::with_seed(seed).with_night(night),
                seed,
                RouteScript::full,
                &[Outcome::Won, Outcome::Failed],
            )
        }) {
            Ok(()) => solo += 1,
            Err(e) => eprintln!("solo seed {seed}: {}", why(e)),
        }
        match std::panic::catch_unwind(|| network_route(Tuning::with_seed(seed).with_night(night), seed)) {
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

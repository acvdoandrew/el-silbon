use bevy::math::{Vec2, Vec3};
use el_silbon::{
    control::Pose,
    geometry::Layout,
    net::{
        protocol::{Action, HOST, Input, STEP, Satchel},
        session::Session,
    },
    sim::{Objective, Presence, ThreatState},
    tuning::Tuning,
};

fn session() -> (Session, Layout, Tuning) {
    let l = Layout::authored();
    let t = Tuning::default();
    let mut s = Session::new(&l, &t);
    s.add_player(2, &l, &t).unwrap();
    s.command(HOST, 1, 1, Action::Start, &l, &t).unwrap();
    (s, l, t)
}
fn aim(s: &mut Session, id: u64, pos: Vec2, at: Vec3, t: &Tuning) {
    let p = &mut s.players.get_mut(&id).unwrap().pose;
    p.pos = pos;
    p.yaw = Pose::yaw_toward(pos, Vec2::new(at.x, at.z));
    p.pitch = (at.y - t.eye_height).atan2(pos.distance(Vec2::new(at.x, at.z)));
}
fn hold(s: &mut Session, id: u64, sequence: u64, t: &Tuning) {
    let p = s.players[&id].pose;
    s.input(
        id,
        Input {
            run: s.run,
            sequence,
            yaw: p.yaw,
            pitch: p.pitch,
            hold: true,
            ..Default::default()
        },
        t,
    )
    .unwrap();
}
#[test]
fn conflicting_pickups_have_one_owner_then_drop_allows_transfer() {
    let (mut s, l, t) = session();
    for id in [1, 2] {
        aim(&mut s, id, Vec2::new(2.15, -3.5), l.satchel, &t);
    }
    s.command(1, 1, 2, Action::Take, &l, &t).unwrap();
    assert!(s.command(2, 1, 1, Action::Take, &l, &t).is_err());
    assert!(s.command(1, 1, 2, Action::Take, &l, &t).is_err());
    assert_eq!(s.satchel, Satchel::Carried(1));
    assert!(s.command(2, 1, 2, Action::Drop, &l, &t).is_err());
    s.command(1, 1, 3, Action::Drop, &l, &t).unwrap();
    let Satchel::Ground(at) = s.satchel else {
        panic!("no dropped satchel")
    };
    aim(&mut s, 2, Vec2::new(at[0] - 1.0, at[2]), Vec3::from_array(at), &t);
    s.command(2, 1, 3, Action::Take, &l, &t).unwrap();
    assert_eq!(s.satchel, Satchel::Carried(2));
    assert_eq!(s.encounter.objective, Objective::ReturnBones);
    assert!(s.command(1, 1, 4, Action::Take, &l, &t).is_err());
}
#[test]
fn range_occlusion_ownership_and_epoch_are_validated() {
    let (mut s, l, t) = session();
    assert!(s.command(2, 1, 1, Action::Take, &l, &t).is_err());
    assert_eq!(s.encounter.objective, Objective::FindSatchel);
    s.satchel = Satchel::Ground([2.0, 0.4, 0.0]);
    aim(&mut s, 2, Vec2::new(2.0, 2.0), Vec3::new(2.0, 0.4, 0.0), &t);
    assert!(s.command(2, 1, 2, Action::Take, &l, &t).is_err());
    assert!(s.command(2, 0, 3, Action::Take, &l, &t).is_err());
    assert!(s.command(99, 1, 1, Action::Take, &l, &t).is_err());
    assert!(s.command(2, 1, 3, Action::Restart, &l, &t).is_err());
    assert_eq!(s.run, 1);
}
#[test]
fn disconnected_carrier_releases_item_and_survivor_can_continue() {
    let (mut s, l, t) = session();
    aim(&mut s, 2, Vec2::new(2.15, -3.5), l.satchel, &t);
    s.command(2, 1, 1, Action::Take, &l, &t).unwrap();
    s.remove_player(2);
    assert_eq!(s.players.len(), 1);
    assert!(!s.encounter.objective.is_over());
    let Satchel::Ground(at) = s.satchel else {
        panic!("item lost after disconnect")
    };
    aim(&mut s, 1, Vec2::new(at[0] - 1.0, at[2]), Vec3::from_array(at), &t);
    s.command(1, 1, 2, Action::Take, &l, &t).unwrap();
    assert_eq!(s.satchel, Satchel::Carried(1));
    assert!(s.outbox.iter().all(|(id, _)| *id != 2));
}
#[test]
fn capture_releases_carrier_but_only_all_caught_is_failure() {
    let (mut s, l, t) = session();
    aim(&mut s, 2, Vec2::new(0.0, 8.0), Vec3::ZERO, &t);
    s.satchel = Satchel::Carried(2);
    s.encounter.objective = Objective::ReturnBones;
    s.encounter.threat.state = ThreatState::Hunting;
    s.encounter.threat.presence = Presence::Present;
    s.encounter.threat.pos = Vec2::new(0.0, 8.5);
    s.step(&l, &t, STEP);
    assert!(s.players[&2].caught);
    assert!(!s.players[&1].caught);
    assert!(matches!(s.satchel, Satchel::Ground(_)));
    assert_eq!(s.encounter.objective, Objective::ReturnBones);
    assert!(s.command(2, 1, 1, Action::Take, &l, &t).is_err());
    s.remove_player(1);
    assert_eq!(s.encounter.objective, Objective::Failed);
}
#[test]
fn only_carrier_can_restitute_and_escape_is_shared_once() {
    let (mut s, l, t) = session();
    s.satchel = Satchel::Carried(2);
    s.encounter.objective = Objective::ReturnBones;
    for id in [1, 2] {
        aim(&mut s, id, Vec2::new(-16.2, -20.9), l.ceiba.offering, &t);
    }
    for seq in 1..240 {
        hold(&mut s, 1, seq, &t);
        s.step(&l, &t, STEP);
    }
    assert_eq!(s.encounter.restitution, 0.0);
    for seq in 1..240 {
        hold(&mut s, 2, seq, &t);
        s.step(&l, &t, STEP);
    }
    assert_eq!(s.satchel, Satchel::Returned);
    assert_eq!(s.encounter.objective, Objective::Escape);
    assert_eq!(s.encounter.restitution, 1.0);
    assert!(s.command(2, 1, 1, Action::Drop, &l, &t).is_err());
    s.players.get_mut(&2).unwrap().pose.pos = l.spawn;
    s.step(&l, &t, STEP);
    assert_eq!(s.encounter.objective, Objective::Won);
    let before = s.outbox.len();
    for _ in 0..60 {
        s.step(&l, &t, STEP);
    }
    assert_eq!(s.outbox.len(), before);
    assert_eq!(s.snapshot(1, &l, &t).objective, s.snapshot(2, &l, &t).objective);
}
#[test]
fn restart_invalidates_old_actions_inputs_and_pending_effects() {
    let (mut s, l, t) = session();
    aim(&mut s, 2, Vec2::new(2.15, -3.5), l.satchel, &t);
    s.command(2, 1, 1, Action::Take, &l, &t).unwrap();
    s.players.get_mut(&2).unwrap().caught = true;
    s.command(1, 1, 2, Action::Restart, &l, &t).unwrap();
    assert_eq!(s.run, 2);
    assert_eq!(s.players.len(), 2);
    assert_eq!(s.satchel, Satchel::Ground(l.satchel.to_array()));
    assert_eq!(s.encounter, el_silbon::sim::Encounter::new(&l));
    assert!(s.outbox.is_empty());
    assert!(s.players.values().all(|p| !p.caught));
    assert!(s.command(2, 1, 999, Action::Take, &l, &t).is_err());
    assert!(
        s.input(
            2,
            Input {
                run: 1,
                sequence: 999,
                axis: [1.0, 1.0],
                hold: true,
                ..Default::default()
            },
            &t
        )
        .is_err()
    );
    let before = s.players[&2].pose;
    for _ in 0..60 {
        s.step(&l, &t, STEP);
    }
    assert_eq!(s.players[&2].pose, before);
    assert!(s.add_player(3, &l, &t).is_err());
}
#[test]
fn hidden_threat_transform_is_not_in_listener_snapshot() {
    let (mut s, l, t) = session();
    s.encounter.threat.presence = Presence::Present;
    s.encounter.threat.state = ThreatState::Stalking;
    s.encounter.threat.pos = Vec2::new(0.0, -4.0);
    // Outside the solid side wall, looking through it.
    aim(&mut s, 2, Vec2::new(8.0, -4.0), Vec3::new(0.0, 2.0, -4.0), &t);
    assert!(s.snapshot(2, &l, &t).threat.is_none());
    s.encounter.threat.pos = Vec2::new(0.0, 15.0);
    aim(&mut s, 2, Vec2::new(0.0, 8.0), Vec3::new(0.0, 2.0, 15.0), &t);
    assert!(s.snapshot(2, &l, &t).threat.is_some());
    s.players.get_mut(&2).unwrap().pose.yaw += std::f32::consts::PI;
    assert!(s.snapshot(2, &l, &t).threat.is_none());
    s.encounter.threat.presence = Presence::Hidden;
    assert!(s.snapshot(1, &l, &t).threat.is_none());
}
#[test]
fn stale_motion_and_missing_packets_do_not_keep_players_walking() {
    let (mut s, l, t) = session();
    let initial = s.players[&2].pose.pos;
    s.input(
        2,
        Input {
            run: 1,
            sequence: 2,
            axis: [1.0, 0.0],
            ..Default::default()
        },
        &t,
    )
    .unwrap();
    s.input(
        2,
        Input {
            run: 1,
            sequence: 1,
            axis: [-1.0, 0.0],
            ..Default::default()
        },
        &t,
    )
    .unwrap();
    for _ in 0..60 {
        s.step(&l, &t, STEP);
    }
    let stopped = s.players[&2].pose.pos;
    assert!(stopped.x > initial.x && stopped.distance(initial) < t.walk_speed * 0.31);
    for _ in 0..60 {
        s.step(&l, &t, STEP);
    }
    assert_eq!(s.players[&2].pose.pos, stopped);
    assert!(
        s.input(
            2,
            Input {
                run: 1,
                sequence: 3,
                yaw: f32::NAN,
                ..Default::default()
            },
            &t
        )
        .is_err()
    );
}

#[test]
fn one_hidden_threat_produces_distinct_categorical_listener_cues() {
    use el_silbon::net::protocol::ServerMessage;
    let (mut s, l, t) = session();
    s.encounter.objective = Objective::ReturnBones;
    s.satchel = Satchel::Carried(2);
    s.encounter.threat.state = ThreatState::Warning;
    s.encounter.threat.presence = Presence::Present;
    s.encounter.threat.pos = Vec2::new(0.0, 10.0);
    s.players.get_mut(&1).unwrap().pose.pos = Vec2::new(-35.0, 20.0);
    s.players.get_mut(&2).unwrap().pose.pos = Vec2::new(0.0, 8.0);
    s.step(&l, &t, STEP);
    let cue = |id| {
        s.outbox.iter().find_map(|(recipient, message)| match message {
            ServerMessage::Cue { variant, .. } if *recipient == id => Some(*variant),
            _ => None,
        })
    };
    assert_eq!(cue(1), Some(0)); // actually far: perceived loud/near
    assert_eq!(cue(2), Some(2)); // actually near: perceived faint/far
}

#[test]
fn manifestation_chooses_clearance_from_both_players() {
    let (mut s, l, t) = session();
    let carrier = Vec2::new(2.15, -3.5);
    let (offline_anchor, _) = l.farthest_anchor(carrier);
    s.players.get_mut(&1).unwrap().pose.pos = l.ring[offline_anchor];
    aim(&mut s, 2, carrier, l.satchel, &t);
    s.command(2, 1, 1, Action::Take, &l, &t).unwrap();
    for p in s.players.values() {
        assert!(s.encounter.threat.pos.distance(p.pose.pos) > t.warn_distance);
    }
}

#[test]
fn teammate_waiting_on_road_cannot_skip_the_return_journey() {
    let (mut s, l, t) = session();
    s.encounter.objective = Objective::ReturnBones;
    s.satchel = Satchel::Carried(2);
    aim(&mut s, 2, Vec2::new(-16.2, -20.9), l.ceiba.offering, &t);
    for seq in 1..240 {
        hold(&mut s, 2, seq, &t);
        s.step(&l, &t, STEP);
    }
    assert!(l.in_road_goal(s.players[&1].pose.pos));
    assert_eq!(s.encounter.objective, Objective::Escape);
    s.players.get_mut(&2).unwrap().pose.pos = l.spawn;
    s.step(&l, &t, STEP);
    assert_eq!(s.encounter.objective, Objective::Won);
}

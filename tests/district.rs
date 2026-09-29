use bevy::math::{Vec2, Vec3};
use el_silbon::{
    control::Pose,
    geometry::{AimStatus, Layout, district::LandmarkId, ground},
    tuning::Tuning,
};

fn walk(l: &Layout, from: Vec2, to: Vec2, radius: f32, speed: f32) -> (Vec2, f32) {
    let mut p = from;
    let dt = 1.0 / 60.0;
    let mut seconds = 0.;
    for _ in 0..180 * 60 {
        let d = to - p;
        if d.length() < 0.03 {
            return (p, seconds);
        }
        let next = l.move_circle(p, d.normalize() * f32::min(speed * dt, d.length()), radius);
        assert!(
            next.distance(p) > 0.00001,
            "blocked from {from:?} toward {to:?} at {p:?}"
        );
        p = next;
        seconds += dt;
    }
    panic!("unreachable {to:?} from {from:?} at {p:?}");
}

#[test]
fn district_routes_are_physically_walkable_in_both_directions() {
    let l = Layout::new();
    let t = Tuning::default();
    let d = &l.district;
    let slowest = t.walk_speed * t.carry_floor * t.wade_factor;
    for route in &d.routes {
        for speed in [t.walk_speed, slowest] {
            let mut elapsed = 0.;
            for pair in route.points.windows(2) {
                assert!(
                    l.is_free(pair[0], t.player_radius),
                    "{} starts blocked at {:?}",
                    route.name,
                    pair[0]
                );
                elapsed += walk(&l, pair[0], pair[1], t.player_radius, speed).1;
                walk(&l, pair[1], pair[0], t.player_radius, speed);
            }
            println!("ROUTE {} speed={speed:.1} time={elapsed:.2}s", route.name);
        }
    }
    for landmark in &d.landmarks {
        assert!(l.is_free(landmark.approach, t.player_radius), "{:?}", landmark.id);
    }
}

#[test]
fn the_silbon_can_walk_every_patrol_edge_at_his_own_radius() {
    let l = Layout::new();
    let t = Tuning::default();
    assert!(l.patrol.connected());
    for &(a, b) in &l.patrol.edges {
        let (pa, pb) = (l.patrol.nodes[a], l.patrol.nodes[b]);
        assert!(l.is_free(pa, 0.42) && l.is_free(pb, 0.42), "{pa:?} {pb:?}");
        walk(&l, pa, pb, 0.42, t.stalk_speed);
        walk(&l, pb, pa, 0.42, t.stalk_speed);
    }
    // No patrol node is a dead end that would leave him stranded from the
    // rest: every node has a neighbour, and the graph spans the whole map.
    assert!((0..l.patrol.len()).all(|i| !l.patrol.neighbors(i).is_empty()));
    let d = &l.district;
    for m in &d.landmarks {
        let node = l.patrol.nearest(m.approach);
        assert!(
            l.patrol.nodes[node].distance(m.approach) < 20.0,
            "{:?} is far from any patrol node",
            m.id
        );
    }
}

#[test]
fn district_boundaries_and_tower_guardrails_prevent_stranding() {
    let l = Layout::new();
    let d = &l.district;
    let canal = d.landmark(LandmarkId::Cano).center;
    // Deep water south of the bridge is a wall.
    let blocked = l.move_circle(canal + Vec2::new(-7., 10.), Vec2::new(0., -12.), 0.3);
    assert!(blocked.y > -61.);
    let (deck, _) = walk(
        &l,
        d.landmark(LandmarkId::Watchtower).approach,
        d.watch_deck.center(),
        0.3,
        3.6,
    );
    assert!((l.surface_height(deck) - d.watch_height).abs() < 0.01);
    let guarded = l.move_circle(deck, Vec2::new(12., 0.), 0.3);
    assert!(guarded.x < d.watch_deck.max.x);
    walk(&l, deck, d.landmark(LandmarkId::Watchtower).approach, 0.3, 3.6);
    // The plank bridge crosses; the deck stays above the water.
    let crossed = walk(&l, canal + Vec2::new(0., 11.), canal - Vec2::new(0., 13.), 0.3, 3.6).0;
    assert!(crossed.y < -77.9);
    // The ford wades across the same channel, elsewhere.
    let ford = walk(&l, Vec2::new(0., -55.), Vec2::new(0., -75.), 0.3, 2.0).0;
    assert!(ford.y < -74.9);
    assert!(l.wading(Vec2::new(0., -65.)) && !l.wading(Vec2::new(10., -65.)));
    // The extraction bridge spans its creek along the road; the creek is a wall beside it.
    let over = walk(&l, Vec2::new(50., 31.), Vec2::new(75., 31.), 0.3, 3.6).0;
    assert!(over.x > 74.9, "the road must cross the creek on the bridge");
    let creek = l.move_circle(Vec2::new(64.5, 25.0), Vec2::new(0., -8.), 0.3);
    assert!(creek.y > 22.0 && creek.y < 30.0);
    let edge = l.move_circle(Vec2::new(30., 34.), Vec2::new(0., 20.), 0.3);
    assert!(edge.y <= l.bounds.max.y);
}

#[test]
fn authored_district_has_ten_places_safe_anchors_and_stable_geometry() {
    let l = Layout::new();
    assert_eq!(l, Layout::new());
    let d = &l.district;
    let ids: std::collections::HashSet<_> = d.landmarks.iter().map(|v| v.id).collect();
    assert_eq!(ids.len(), 10);
    assert!(ids.contains(&LandmarkId::Extraction));
    assert!(l.is_free(l.spawn, 0.3));
    assert!(l.ceiba.center.distance(l.house.footprint.center()) > 45.);
    let stand = ground(l.ceiba.offering) + (ground(l.ceiba.offering) - l.ceiba.center).normalize() * 1.3;
    assert!(l.is_free(stand, 0.3));
    // The truck is a solid body; its ignition is reachable from the road.
    assert!(!l.is_free(d.truck.center, 0.3));
    assert!(d.truck.zone_center.distance(Vec2::new(55.7, 29.2)) < d.truck.zone_radius);
    // The bridge deck is dead level with the road it carries.
    let deck = d.surface_height(Vec2::new(64., 31.));
    assert!((deck - 0.22).abs() < 0.01);
    // Landmarks keep clear of one another's clearing radius.
    for a in &d.landmarks {
        for b in &d.landmarks {
            if a.id != b.id {
                assert!(
                    a.center.distance(b.center) > (a.clearing.min(b.clearing)),
                    "{:?} vs {:?}",
                    a.id,
                    b.id
                );
            }
        }
    }
}

/// Some open spot within reach and sight of `at`, on the same level.
fn interactable(l: &Layout, t: &Tuning, at: Vec3, radius: f32, reach: f32) -> Option<Vec2> {
    for r in [1.2_f32, 1.6, 2.0, 2.3] {
        for k in 0..24 {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            let p = ground(at) + Vec2::new(a.cos(), a.sin()) * r;
            if !l.is_free(p, t.player_radius) || !l.bounds.contains(p) {
                continue;
            }
            if (l.surface_height(p) - (at.y - 0.8)).abs() > 1.4 && (l.surface_height(p) - at.y).abs() > 1.4 {
                continue; // a different floor
            }
            let mut pose = Pose::at(p, 0.0);
            pose.look_at(t, l, at);
            let eye = pose.eye(t, l);
            if matches!(l.aim(eye, pose.look_dir(), at, radius, reach), AimStatus::Ready { .. }) {
                return Some(p);
            }
        }
    }
    None
}

#[test]
fn every_bundle_pepper_note_and_site_can_actually_be_reached_and_seen() {
    let l = Layout::new();
    let t = Tuning::default();
    let d = &l.district;
    for (i, p) in d.relics.iter().enumerate() {
        assert!(
            interactable(&l, &t, *p, el_silbon::geometry::district::RELIC_RADIUS, t.relic_reach).is_some(),
            "bundle {i} at {p:?} cannot be picked up"
        );
    }
    for (i, p) in d.aji.iter().enumerate() {
        assert!(
            interactable(&l, &t, *p, 0.22, t.relic_reach).is_some(),
            "pepper {i} at {p:?} cannot be picked up"
        );
    }
    for n in &d.notes {
        assert!(
            interactable(&l, &t, n.pos, 0.24, t.note_reach).is_some(),
            "note {} at {:?} cannot be read",
            n.id,
            n.pos
        );
    }
    for (name, p, r) in [
        ("pump", d.pump, 0.65),
        ("ignition", d.ignition, 0.6),
        ("beacon", d.beacon, 0.6),
    ] {
        assert!(
            interactable(&l, &t, p, r, t.site_reach).is_some(),
            "{name} at {p:?} cannot be used"
        );
    }
    assert!(interactable(&l, &t, l.ceiba.offering, l.ceiba.offering_radius, t.altar_reach).is_some());
    // Note ids are unique and the lore has text for every one.
    let mut ids: Vec<u8> = d.notes.iter().map(|n| n.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), d.notes.len());
}

#[test]
fn every_landmark_has_light_and_the_terrain_is_walkable_everywhere() {
    let l = Layout::new();
    let d = &l.district;
    // Fear needs somewhere to recover: landmarks with lamps are lit without
    // power, so the dark between them is a real cost.
    for id in [
        LandmarkId::Entry,
        LandmarkId::Ranch,
        LandmarkId::WaterTower,
        LandmarkId::Corral,
        LandmarkId::Shrine,
        LandmarkId::Fields,
        LandmarkId::Cano,
        LandmarkId::Watchtower,
    ] {
        let m = d.landmark(id);
        let lit = l
            .light_sources
            .iter()
            .filter(|s| !s.powered)
            .any(|s| ground(s.pos).distance(m.center) < s.radius + m.clearing);
        assert!(lit, "{id:?} has no lamp of its own");
    }
    // No cliffs: terrain changes gently between metres, and is dead flat
    // wherever people start and finish.
    let mut worst = 0.0_f32;
    let mut x = l.bounds.min.x;
    while x < l.bounds.max.x {
        let mut z = l.bounds.min.y;
        while z < l.bounds.max.y {
            let a = Vec2::new(x, z);
            if !d
                .water
                .iter()
                .chain(&d.shallows)
                .any(|w| w.contains(a) || w.contains(a + Vec2::X))
            {
                let slope = (l.terrain(a + Vec2::X) - l.terrain(a)).abs();
                worst = worst.max(slope);
            }
            z += 1.0;
        }
        x += 1.0;
    }
    assert!(worst < 0.55, "terrain gradient {worst} per metre is a cliff");
    assert!(l.terrain(l.spawn).abs() < 0.01);
    assert!(l.terrain(d.truck.zone_center).abs() < 0.01);
    assert!(
        (l.terrain(Vec2::new(-46.0, -25.0)) - d.tower_ground).abs() < 0.01,
        "the windmill stands on high ground"
    );
    assert!(
        (l.terrain(Vec2::new(0.0, -2.5))).abs() < 0.05,
        "the house floor is level"
    );
    // Water dips below the shore.
    assert!(l.terrain(Vec2::new(30.0, -65.0)) < -0.5);
}

#[test]
fn a_player_who_leaves_on_the_deck_drops_the_bundle_where_a_teammate_can_take_it() {
    use el_silbon::net::{
        protocol::{Action, Input, STEP},
        session::Session,
    };
    use el_silbon::sim::Relic;
    let l = Layout::new();
    let t = Tuning::default();
    let d = &l.district;
    let mut session = Session::new(&l, &t);
    session.add_player(2, &l, &t).unwrap();
    session.command(1, 1, 1, Action::Start, &l, &t).unwrap();
    session.players.get_mut(&2).unwrap().pose.pos = d.landmark(LandmarkId::Watchtower).approach;
    session.encounter.progress.relics[0] = Relic::Carried(2);
    // Walk the ramp with the real controller until on the deck.
    for sequence in 1..2000 {
        let pose = session.players[&2].pose;
        let remaining = d.watch_deck.center() - pose.pos;
        if remaining.length() < 0.12 {
            break;
        }
        let dir = remaining.normalize();
        session
            .input(
                2,
                Input {
                    run: 1,
                    sequence,
                    axis: [dir.dot(pose.right2()), dir.dot(pose.forward2())],
                    yaw: pose.yaw,
                    ..Default::default()
                },
                &t,
            )
            .unwrap();
        session.step(&l, &t, STEP);
        session.encounter.threat.cooldown = 1.0e9;
    }
    let pos = session.players[&2].pose.pos;
    assert!(
        pos.distance(d.watch_deck.center()) < 0.15,
        "the ramp must lead onto the deck"
    );
    session.remove_player(2);
    let Relic::Ground(item) = session.encounter.progress.relics[0] else {
        panic!("lost carrier item")
    };
    assert!(
        (item.y - (d.watch_height + 0.35)).abs() < 0.01,
        "dropped at deck height: {}",
        item.y
    );
    let mut pose = Pose::at(pos + Vec2::new(0., 0.8), 0.0);
    pose.look_at(&t, &l, item);
    session.players.get_mut(&1).unwrap().pose = pose;
    session
        .command(1, 1, 2, Action::Interact, &l, &t)
        .expect("the bundle must be takeable on the deck");
    assert_eq!(session.encounter.progress.relics[0], Relic::Carried(1));
}

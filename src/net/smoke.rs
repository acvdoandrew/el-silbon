//! Explicit development automation, using real endpoints and ordinary inputs.
//! No position teleports, objective setters, fake transport, or desktop input.
//!
//! Both processes run the shared route engine (`script::RouteScript`) with a
//! role each: walking with look/move deltas, aiming the real crosshair, and
//! sending the same commands a human's frames would. The story, seen from
//! either side:
//!
//! run 1 — the host lays the table bundle at the ceiba while the partner
//! watches it land in her own snapshot, marks the altar, and only when one
//! snapshot holds both her mark and the host's does she acknowledge (by
//! crouching, which the host sees); she holds the crouch until the host has
//! taken the next bundle. The host then carries the other four bundles home,
//! cranks the pump and starts the truck while the partner waits in the lit
//! porch, and once the lamps are on she walks to the truck's boarding zone.
//! Both wait there through the engine's warm-up (reflexes play the threat,
//! hiding near the truck if he comes) and stand in the zone once it is warm:
//! the shared WIN. Once the outcome is decided the authority takes no more
//! body input, so there is no acknowledgement: the host lingers a bounded
//! 2.5 s so the partner's snapshot shows the win, then restarts, and the
//! partner waits (bounded) for the new epoch.
//! run 2 — the bundle wakes him; both stand in the open until he has caught
//! them both. Both must see the shared failure before the host restarts.
//! run 3 — the partner takes the table bundle and disconnects carrying it;
//! the host sees the party shrink and the load fall to the ground, walks to it,
//! picks it up and lays it at the altar.
//!
//! Both processes run on the wall clock at 60 Hz, as the game does, and build
//! every frame by the game's own rules (`super::Wire`, `controls_live`,
//! `follow_body`): the body trails its snapshot row, the head turns unless
//! dead, the crosshair is the HUD's own (a frame old), and the controls carry
//! nothing while the player is frozen or dead or the run is over. A full
//! route therefore takes real minutes.
//!
//! An unexpected run epoch, a restart that does not reset everything, being
//! caught where the route does not intend it, or a timeout fails the smoke
//! with a diagnostic naming the step and the state; nothing here treats a
//! surprise as success.
use super::{
    Wire, controls_live, follow_body, mirror,
    protocol::*,
    status_of,
    transport::{Endpoint, Mode},
};
use crate::{
    app::Launch,
    control::{Intent, Pose, Target},
    geometry::Layout,
    script::{Observation, RouteScript, crosshair},
    sim::Encounter,
    tuning::Tuning,
};
use std::time::{Duration, Instant};

/// Seconds the whole route may take (the clock is the wall clock).
const TIMEOUT: f32 = 2400.0;

/// One frame of a 60 Hz client.
const FRAME: Duration = Duration::from_micros(16_667);

pub struct Driver {
    /// Chosen on the first snapshot, once the role (host or partner) is known.
    script: Option<RouteScript>,
    /// The listener's mirror of the run, filled from snapshots as the client
    /// adapter does, for the route's diagnostics.
    mirror: Encounter,
    elapsed: f32,
    seen_run: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub struct Frame {
    pub intent: Intent,
    /// Start, restart or mark. Presses (pick up, pepper) travel in the
    /// intent, as for a human; see `super::Wire`.
    pub action: Option<Action>,
    pub leave: bool,
    /// The route wants the torch off (it is hiding).
    pub dark: bool,
}

impl Driver {
    pub fn new(layout: &Layout, _tuning: &Tuning) -> Self {
        Self {
            script: None,
            mirror: Encounter::new(layout),
            elapsed: 0.0,
            seen_run: 0,
            done: false,
            error: None,
        }
    }

    fn step(&self) -> String {
        self.script
            .as_ref()
            .map_or_else(|| "connecting".into(), RouteScript::step_name)
    }

    /// One route frame. `target` is what the HUD's crosshair showed as the
    /// last frame ended, as a human sees it: the route is never given a
    /// fresher view than the game gives.
    pub fn tick(
        &mut self,
        endpoint: &Endpoint,
        pose: Pose,
        target: Option<Target>,
        dt: f32,
        layout: &Layout,
        tuning: &Tuning,
    ) -> Frame {
        let mut frame = Frame {
            intent: Intent::default(),
            action: None,
            leave: false,
            dark: false,
        };
        self.elapsed += dt;
        if self.error.is_some() || self.done {
            return frame;
        }
        if self.elapsed > TIMEOUT {
            self.error = Some(format!("smoke timed out after {TIMEOUT:.0}s at {}", self.step()));
            return frame;
        }
        if endpoint.closed {
            self.error = Some(format!("{} (at {})", endpoint.status, self.step()));
            return frame;
        }
        // Let the endpoint diagnose connection loss; do not misreport stale
        // snapshots as a pathfinding failure when the host has departed.
        if endpoint.snapshot.is_some() && endpoint.snapshot_age() > 0.25 {
            return frame;
        }
        let (Some(snap), Some(id)) = (&endpoint.snapshot, endpoint.id) else {
            return frame;
        };
        if snap.run != self.seen_run {
            self.seen_run = snap.run;
            eprintln!("NET SMOKE player={id} observing run={}", snap.run);
        }
        mirror(&mut self.mirror, snap);
        let host = id == HOST;
        let script = self.script.get_or_insert_with(|| {
            if host {
                RouteScript::net_host(layout, tuning)
            } else {
                RouteScript::net_client(layout, tuning)
            }
        });
        let step = script.tick(&Observation {
            layout,
            tuning,
            me: id,
            pose,
            encounter: &self.mirror,
            snapshot: Some(snap),
            target,
            dt,
        });
        if let Some(msg) = &step.log {
            eprintln!("NET SMOKE player={id}: {msg}");
        }
        match &step.finished {
            Some(Ok(())) => {
                self.done = true;
                eprintln!(
                    "NET SMOKE PASS host: shared pickup and delivery, both marks acknowledged, the shared win with \
                     all bones, power and truck, two restarts, the shared failure, and the disconnected carrier's \
                     dropped load recovered and delivered"
                );
            }
            Some(Err(e)) => self.error = Some(e.clone()),
            None => {}
        }
        if step.leave {
            self.done = true;
            eprintln!(
                "NET SMOKE PASS client: watched the host's delivery, exchanged marks, acknowledged, boarded the \
                 truck for the shared win, saw the shared failure and both restarts; leaving while carrying a bundle"
            );
        }
        frame.action = step.command();
        frame.leave = step.leave;
        frame.intent = step.intent;
        frame.dark = step.dark;
        frame
    }
}

/// Two independent processes still use real UDP, handshakes, input packets,
/// host validation and per-listener snapshots, and each runs on the wall clock
/// at 60 Hz as the game does, so their clocks cannot drift apart. No rendering
/// is claimed here. A frame is the game's, in the game's order: the update
/// brings the newest snapshot and the body follows its row, the route decides
/// from what the client last saw, the head turns unless dead, the crosshair is
/// read, the input goes out with its presses and the route's own command
/// follows.
pub fn run_headless(launch: Launch) -> Result<(), String> {
    let layout = Layout::with_seed(launch.seed);
    let tuning = Tuning::with_seed(launch.seed).with_night(launch.night);
    if launch.network == Mode::Solo {
        return Err("headless requires --host/--join".into());
    }
    let mut endpoint =
        Endpoint::new(launch.network.clone(), &layout, &tuning)?.with_survivor(launch.survivor.unwrap_or_default());
    let mut driver = Driver::new(&layout, &tuning);
    let mut pose = Pose::spawn(&layout);
    let mut run = 0;
    // What the crosshair showed as the last frame ended: the HUD's view.
    let mut hud: Option<Target> = None;
    let mut last = Instant::now();
    eprintln!("NET SMOKE headless {}", endpoint.status);
    loop {
        let frame_start = Instant::now();
        let dt = frame_start.duration_since(last).as_secs_f32().min(0.1);
        last = frame_start;
        endpoint.update(dt, &layout, &tuning);
        if let Some(s) = &endpoint.snapshot
            && let Some(local) = endpoint.id.and_then(|id| s.player(id))
        {
            follow_body(&mut pose, local, s.run != run, dt, &tuning);
            run = s.run;
        }
        let frame = driver.tick(&endpoint, pose, hud, dt, &layout, &tuning);
        if let Some(e) = driver.error {
            return Err(e);
        }
        // The frozen and the downed can still turn their heads; the dead only watch.
        if status_of(endpoint.snapshot.as_ref(), endpoint.id) != 2 {
            pose.look(frame.intent.look_delta, &tuning);
        }
        hud = endpoint
            .snapshot
            .as_ref()
            .zip(endpoint.id)
            .and_then(|(s, id)| crosshair(&layout, &tuning, s, id, &pose));
        // The torch is on, as the game starts and the route never switches it.
        let live = controls_live(endpoint.snapshot.as_ref(), endpoint.id, true);
        let wire = Wire::new(&frame.intent, live, &pose, !frame.dark, hud, &layout, &tuning);
        wire.send(&mut endpoint, &layout, &tuning);
        if let Some(action) = frame.action {
            endpoint.command(action, &layout, &tuning);
        }
        if frame.leave {
            endpoint.close("Client left the smoke session carrying a bundle.");
        }
        endpoint.notices.clear();
        if driver.done {
            return Ok(());
        }
        if let Some(rest) = FRAME.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}

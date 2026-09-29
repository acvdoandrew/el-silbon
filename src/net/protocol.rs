//! Small explicit wire contract. No ECS IDs, hidden AI state or continuous
//! whistle-distance values cross this boundary.
use crate::control::SceneData;
use crate::sim::{Event, Relic};
use bevy::math::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

pub use crate::sim::PlayerId;
pub const HOST: PlayerId = 1;
pub const PROTOCOL: u64 = 0x5349_4c42_4f4e_0003;
pub const MAX_PLAYERS: usize = 4;
pub const STEP: f32 = 1.0 / 60.0;
pub const SEND_INTERVAL: f32 = 1.0 / 20.0;
pub const MAX_MESSAGE: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Input {
    pub run: u64,
    pub sequence: u64,
    pub axis: [f32; 2],
    pub yaw: f32,
    pub pitch: f32,
    /// Interact is held.
    pub hold: bool,
    pub crouch: bool,
    pub sprint: bool,
    /// The flashlight is on (it makes you easier to see).
    pub light: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Action {
    /// Press interact: take the bundle, pepper or item in the crosshair.
    Interact,
    /// Put a carried bundle down.
    Drop,
    /// Scatter a pepper.
    UseAji,
    /// Mark a spot for the whole party.
    Ping {
        at: [f32; 3],
    },
    /// Press for skill check `id`, claiming the needle stood at `needle`.
    Skill {
        id: u32,
        needle: f32,
    },
    /// Try a combination on the key box's padlock.
    TryCode {
        code: [u8; 3],
    },
    /// Name which of him walks tonight, at the ceiba (`sim::Variant` code).
    Name {
        variant: u8,
    },
    Start,
    Restart,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello {
        fingerprint: u64,
        seed: u64,
        /// The night's difficulty (`tuning::Night` code): every peer must agree.
        #[serde(default = "normal_night")]
        night: u8,
    },
    Input(Input),
    Action {
        run: u64,
        sequence: u64,
        action: Action,
    },
    Leave,
}

/// A bone bundle as the wire sees it: 0 on the ground, 1 carried, 2 laid
/// at the altar.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelicView {
    pub state: u8,
    pub pos: [f32; 3],
    pub owner: PlayerId,
}

impl RelicView {
    pub fn from_relic(r: &Relic) -> Self {
        match *r {
            Relic::Ground(p) => Self {
                state: 0,
                pos: p.to_array(),
                owner: 0,
            },
            Relic::Carried(id) => Self {
                state: 1,
                pos: [0.0; 3],
                owner: id,
            },
            Relic::Delivered => Self {
                state: 2,
                pos: [0.0; 3],
                owner: 0,
            },
        }
    }

    pub fn to_relic(self) -> Relic {
        match self.state {
            0 => Relic::Ground(Vec3::from_array(self.pos)),
            1 => Relic::Carried(self.owner),
            _ => Relic::Delivered,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: PlayerId,
    pub position: [f32; 2],
    pub yaw: f32,
    pub pitch: f32,
    /// 0 active, 1 downed, 2 dead.
    pub status: u8,
    pub crouch: bool,
    pub sprint: bool,
    pub light: bool,
    /// Bundles carried.
    pub carrying: u8,
    /// Revive progress while downed, 0..1.
    pub revive: f32,
    /// Seconds a downed player has left.
    pub bleed: f32,
    /// In his sack, being carried off.
    #[serde(default)]
    pub hauled: bool,
}

/// Only sent when physically present, in the listener's view cone and with
/// unobstructed sight. This is legitimate visible presentation, not an AI dump.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VisibleThreat {
    pub position: [f32; 2],
    pub facing: [f32; 2],
    pub speed: f32,
    pub visibility: f32,
    /// 0 dormant, 1 stalking, 2 warning, 3 hunting, 4 counting, 5 hauling.
    pub state: u8,
}

/// A skill check in flight for the listener: its id, where the host's
/// needle is (negative during the warning) and where the zone starts. Widths
/// and timing come from the shared tuning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckView {
    pub id: u32,
    pub needle: f32,
    pub zone: f32,
}

/// The listener's own body.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Vitals {
    pub fear: f32,
    pub stamina: f32,
    pub aji: u8,
    /// Torch charge, 0..1 (at 0 the torch is dead whatever the switch says).
    pub battery: f32,
    /// Seconds still frozen by a susto.
    pub stun: f32,
    /// What the local player is holding on, and how far along it is (0..1):
    /// 0 nothing, 1 laying bones down, 2 praying, 3 cranking the pump,
    /// 4 the ignition, 5 the beacon, 6 reviving a teammate.
    pub hold_kind: u8,
    pub hold: f32,
    /// A skill check the listener is being asked for.
    pub check: Option<CheckView>,
}

/// Shared progress of the run.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct WorldView {
    pub delivered: u8,
    pub total: u8,
    pub power: f32,
    pub truck: f32,
    /// 0..1: how warm the running engine is; at 1 the truck can leave.
    pub warm: f32,
    /// Seconds the beacon still burns; whether it can be lit now.
    pub beacon: f32,
    pub beacon_ready: bool,
    /// The truck key is out of its box.
    pub key: bool,
    /// The lamp lines lit right now (bit per circuit; 0 without power).
    #[serde(default)]
    pub circuits: u8,
    /// He was named rightly and laid to rest.
    pub banished: bool,
    /// Seconds before the ceiba will hear another name.
    pub naming: f32,
    /// Seconds of bellowing left.
    pub cattle: f32,
    /// 0..1 through the night.
    pub night: f32,
}

/// Tureco as everyone sees him: where he is and faces, and what he does
/// (0 tied, 1 following, 2 growling, 3 barking).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DogView {
    pub pos: [f32; 2],
    pub facing: [f32; 2],
    pub mood: u8,
    pub owner: PlayerId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PingView {
    pub pos: [f32; 3],
    pub left: f32,
    pub by: PlayerId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub run: u64,
    pub tick: u64,
    pub started: bool,
    pub players: Vec<PlayerView>,
    pub relics: Vec<RelicView>,
    /// Which peppers are gone.
    pub aji: Vec<bool>,
    /// Which spare batteries are gone.
    pub batteries: Vec<bool>,
    /// 0 running, 1 won, 2 failed.
    pub outcome: u8,
    pub world: WorldView,
    pub elapsed: f32,
    pub stats: [u32; 5],
    pub threat: Option<VisibleThreat>,
    /// Local warning/hunting feedback: 0 quiet, 1 warning, 2 hunt in sight,
    /// 3 hunt out of sight, 4 he is counting bones. Never contains a hidden
    /// transform or route target.
    pub danger: u8,
    pub exposure: f32,
    pub me: Vitals,
    /// Pepper wards: x, z, seconds left.
    pub zones: Vec<[f32; 3]>,
    pub pings: Vec<PingView>,
    #[serde(default)]
    pub dog: DogView,
}

impl Snapshot {
    pub fn player(&self, id: PlayerId) -> Option<&PlayerView> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn outcome(&self) -> crate::sim::Outcome {
        outcome(self.outcome)
    }

    /// The crosshair's view of the world for player `me`.
    pub fn scene_data(&self, me: PlayerId, stunned: bool) -> SceneData {
        let mine = self.player(me);
        SceneData {
            relics: self.relics.iter().map(|r| r.to_relic()).collect(),
            aji_taken: self.aji.clone(),
            batteries_taken: self.batteries.clone(),
            power_on: self.world.power >= 1.0,
            truck_running: self.world.truck >= 1.0,
            bones_home: self.world.delivered == self.world.total,
            beacon_ready: self.world.beacon_ready,
            key: self.world.key,
            dog_tied: (self.dog.mood == 0).then(|| Vec2::from_array(self.dog.pos)),
            carrying: mine.map_or(0, |p| p.carrying as usize),
            aji_held: self.me.aji,
            battery: self.me.battery,
            bodies: self
                .players
                .iter()
                .filter(|p| p.status == 1)
                .map(|p| (p.id, Vec2::from_array(p.position)))
                .collect(),
            me,
            acting: mine.is_some_and(|p| p.status == 0) && !stunned && self.started && !self.outcome().is_over(),
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "Snapshots dominate traffic and are serialized immediately; boxing adds an allocation to every snapshot"
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        id: PlayerId,
    },
    Rejected(String),
    /// The host's night differs from the one the joiner knocked with: the
    /// joiner takes this seed and difficulty and knocks again.
    Tonight {
        seed: u64,
        night: u8,
    },
    Snapshot(Snapshot),
    Cue {
        run: u64,
        serial: u64,
        variant: u8,
        speed: f32,
        /// Heard only in this listener's fear: there was no whistle.
        phantom: bool,
    },
    Events {
        run: u64,
        serial: u64,
        events: Vec<u8>,
    },
    ActionResult {
        run: u64,
        sequence: u64,
        error: Option<String>,
    },
    Ended(String),
}

/// Exact gameplay build + seed handshake, rather than assuming layouts/config
/// match because both applications happened to start successfully.
pub fn fingerprint() -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for text in [
        include_str!("protocol.rs"),
        include_str!("session.rs"),
        include_str!("../geometry.rs"),
        include_str!("../geometry/district.rs"),
        include_str!("../tuning.rs"),
        include_str!("../control.rs"),
        include_str!("../sim.rs"),
        include_str!("../body.rs"),
        include_str!("../storm.rs"),
        include_str!("../perception.rs"),
        include_str!("../skill.rs"),
        include_str!("../director.rs"),
        include_str!("../../Cargo.lock"),
    ] {
        for byte in text.bytes() {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn normal_night() -> u8 {
    crate::tuning::Night::Normal.code()
}

pub fn encode<T: Serialize>(message: &T) -> Vec<u8> {
    serde_json::to_vec(message).expect("wire values are finite and serializable")
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() > MAX_MESSAGE {
        return Err("message exceeds limit".into());
    }
    serde_json::from_slice(bytes).map_err(|e| format!("invalid session message: {e}"))
}

pub fn outcome(code: u8) -> crate::sim::Outcome {
    use crate::sim::Outcome;
    match code {
        0 => Outcome::Running,
        1 => Outcome::Won,
        _ => Outcome::Failed,
    }
}

pub fn outcome_code(o: crate::sim::Outcome) -> u8 {
    use crate::sim::Outcome;
    match o {
        Outcome::Running => 0,
        Outcome::Won => 1,
        Outcome::Failed => 2,
    }
}

pub fn threat_state(code: u8) -> crate::sim::ThreatState {
    use crate::sim::ThreatState;
    match code {
        1 => ThreatState::Stalking,
        2 => ThreatState::Warning,
        3 => ThreatState::Hunting,
        4 => ThreatState::Counting,
        5 => ThreatState::Hauling,
        _ => ThreatState::Dormant,
    }
}

pub fn threat_code(s: crate::sim::ThreatState) -> u8 {
    use crate::sim::ThreatState;
    match s {
        ThreatState::Dormant => 0,
        ThreatState::Stalking => 1,
        ThreatState::Warning => 2,
        ThreatState::Hunting => 3,
        ThreatState::Counting => 4,
        ThreatState::Hauling => 5,
    }
}

pub fn event(code: u8) -> Option<Event> {
    Event::from_code(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relic_views_round_trip_and_state_codes_are_stable() {
        for r in [
            Relic::Ground(Vec3::new(1.0, 2.0, 3.0)),
            Relic::Carried(4),
            Relic::Delivered,
        ] {
            assert_eq!(RelicView::from_relic(&r).to_relic(), r);
        }
        for s in [
            crate::sim::ThreatState::Dormant,
            crate::sim::ThreatState::Stalking,
            crate::sim::ThreatState::Warning,
            crate::sim::ThreatState::Hunting,
            crate::sim::ThreatState::Counting,
            crate::sim::ThreatState::Hauling,
        ] {
            assert_eq!(threat_state(threat_code(s)), s);
        }
        for o in [
            crate::sim::Outcome::Running,
            crate::sim::Outcome::Won,
            crate::sim::Outcome::Failed,
        ] {
            assert_eq!(outcome(outcome_code(o)), o);
        }
    }
}

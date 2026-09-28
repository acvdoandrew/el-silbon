//! Small explicit wire contract. No ECS IDs, hidden AI state or continuous
//! whistle-distance values cross this boundary.
use serde::{Deserialize, Serialize};

pub type PlayerId = u64;
pub const HOST: PlayerId = 1;
pub const PROTOCOL: u64 = 0x5349_4c42_4f4e_0001;
pub const MAX_PLAYERS: usize = 2;
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
    pub hold: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Action {
    Take,
    Drop,
    Start,
    Restart,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { fingerprint: u64, seed: u64 },
    Input(Input),
    Action { run: u64, sequence: u64, action: Action },
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Satchel {
    Ground([f32; 3]),
    Carried(PlayerId),
    Returned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: PlayerId,
    pub position: [f32; 2],
    pub yaw: f32,
    pub pitch: f32,
    pub caught: bool,
}

/// Only sent when physically present, in the listener's view cone and with
/// unobstructed sight. This is legitimate visible presentation, not an AI dump.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VisibleThreat {
    pub position: [f32; 2],
    pub facing: [f32; 2],
    pub speed: f32,
    pub visibility: f32,
    pub state: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub run: u64,
    pub tick: u64,
    pub started: bool,
    pub players: Vec<PlayerView>,
    pub satchel: Satchel,
    pub objective: u8,
    pub restitution: f32,
    pub restituting: bool,
    pub elapsed: f32,
    pub stats: [u32; 3],
    pub threat: Option<VisibleThreat>,
    /// Local warning/hunting feedback: 0 quiet, 1 warning, 2 hunt in sight,
    /// 3 hunt out of sight. Never contains a hidden transform or route target.
    pub danger: u8,
    pub exposure: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        id: PlayerId,
    },
    Rejected(String),
    Snapshot(Snapshot),
    Cue {
        run: u64,
        serial: u64,
        variant: u8,
        speed: f32,
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
        include_str!("../tuning.rs"),
        include_str!("../control.rs"),
        include_str!("../sim.rs"),
        include_str!("../perception.rs"),
        include_str!("../../Cargo.lock"),
    ] {
        for byte in text.bytes() {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    hash
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

pub fn objective(code: u8) -> crate::sim::Objective {
    use crate::sim::Objective;
    match code {
        0 => Objective::FindSatchel,
        1 => Objective::ReturnBones,
        2 => Objective::Escape,
        3 => Objective::Won,
        _ => Objective::Failed,
    }
}
pub fn threat_state(code: u8) -> crate::sim::ThreatState {
    use crate::sim::ThreatState;
    match code {
        1 => ThreatState::Stalking,
        2 => ThreatState::Warning,
        3 => ThreatState::Hunting,
        4 => ThreatState::Resolved,
        _ => ThreatState::Dormant,
    }
}
pub fn event(code: u8) -> Option<crate::sim::Event> {
    use crate::sim::Event::*;
    [
        SatchelTaken,
        ThreatManifested,
        WarningBegan,
        HuntBegan,
        WarningAverted,
        LostTrack,
        ThreatReturned,
        RestitutionComplete,
        ThreatResolved,
        Escaped,
        Caught,
    ]
    .get(code as usize)
    .copied()
}

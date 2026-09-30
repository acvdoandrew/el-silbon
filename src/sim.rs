//! Encounter truth: the Silbón's hidden state, the world's shared progress and
//! the rules that connect them.
//!
//! Headless and deterministic. `net::session` owns the players and feeds this
//! module positions, noise and interactions; nothing in here knows about
//! rendering, audio or input devices. Solo play and hosted play run the very
//! same rules.

use bevy::math::{Vec2, Vec3};

use crate::geometry::Layout;
use crate::rng::Rng;
use crate::tuning::Tuning;

pub type PlayerId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    Running,
    /// Everyone still standing reached the running truck.
    Won,
    /// Nobody is left on their feet.
    Failed,
}

impl Outcome {
    pub fn is_over(self) -> bool {
        !matches!(self, Outcome::Running)
    }
}

/// The Silbón's true behavioural state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreatState {
    /// Not yet here: the first bone bundle is still where it lay.
    Dormant,
    /// Walking his patrol, lurking, investigating noise, creeping toward a
    /// visible player.
    Stalking,
    /// He has seen you. Break line of sight before the hunt begins.
    Warning,
    /// Closing in while he can see you; exposure builds.
    Hunting,
    /// Stopped by pepper: he squats to count his bones.
    Counting,
    /// Carrying a fallen player off in his sack, toward the far end of the
    /// llano. Pepper in his path makes him drop them; if he gets away with
    /// them, they are gone.
    Hauling,
}

/// Whether he is physically present in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Presence {
    Hidden,
    /// Rising out of the grass, `t` seconds in.
    Rising {
        t: f32,
    },
    Present,
    /// Sinking away; `relocate` = re-manifest far from everyone afterwards.
    Sinking {
        t: f32,
        relocate: bool,
    },
}

/// How he is moving along the patrol graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Movement {
    /// Standing on node `i`.
    AtAnchor(usize),
    /// Walking the graph between two adjacent nodes.
    Walk { from: usize, to: usize },
    /// Left node `home` and walks straight toward a visible player.
    Creep { home: usize },
    /// Walking straight back to node `home`.
    Return { home: usize },
    /// Left node `home` and walks straight toward the spot of a noise.
    Investigate { home: usize },
    /// Standing where he heard something, listening.
    Search,
    /// Not moving (warning, hunting without sight, rising, sinking, counting).
    Still,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Event {
    RelicTaken = 0,
    ThreatManifested,
    WarningBegan,
    HuntBegan,
    /// Sight broken during a warning: the hunt never starts.
    WarningAverted,
    /// Sight broken long enough during a hunt: he withdraws.
    LostTrack,
    /// He rose again at a distant node after withdrawing.
    ThreatReturned,
    RelicDelivered,
    Escaped,
    /// The hunted player went down.
    Downed,
    RelicDropped,
    AllBonesHome,
    PowerRestored,
    TruckStarted,
    AjiTaken,
    AjiUsed,
    CountingBegan,
    CountingEnded,
    Susto,
    Revived,
    Died,
    CattleSpooked,
    BeaconLit,
    Prayed,
    /// Spare torch batteries taken (only the finder hears it).
    BatteriesTaken,
    /// A skill check begins (its warning; only the worker hears it).
    SkillCheck,
    /// A great press (only the worker hears it).
    SkillGreat,
    /// A missed check: everyone hears the screech.
    SkillMissed,
    /// Tells of which of him walks tonight (see `Variant`): a man weeping
    /// when a bundle is laid (the Son), a whip cracking (the Drover), glass
    /// clinking in the grass (the Drunkard).
    Weeping,
    WhipCrack,
    BottleClink,
    /// Tureco is untied and follows whoever freed him.
    DogFreed,
    /// Tureco growls low at the dark: he is near (to the dog's friend).
    DogGrowl,
    /// Tureco barks him off: he flinches away into the grass.
    DogBark,
    /// He put the fallen player in his sack and is carrying them off.
    Hauled,
    /// Pepper stopped him: he dropped the one he was carrying.
    SackDropped,
    /// He got away with the one in his sack. They are gone.
    Taken,
    /// Named rightly at the ceiba: he is laid to rest (a way to win).
    Banished,
    /// Named wrongly: the ceiba shudders and he comes, furious.
    NameWrong,
    /// The panel at the windmill switched which lines the dynamo feeds.
    LinesSwitched,
    /// The padlock opened: the truck key is ours.
    KeyFound,
    /// A wrong combination: the padlock rattles (heard by the one trying,
    /// and by him if he is near).
    LockRattle,
    /// Omens (see `director`), each to one player only.
    OmenLampsDie,
    OmenSilence,
    OmenBones,
    OmenDrag,
    OmenHat,
    OmenPhantom,
    OmenStolenLight,
    OmenFootsteps,
    OmenFalseMark,
}

impl Event {
    pub const ALL: [Event; 51] = [
        Event::RelicTaken,
        Event::ThreatManifested,
        Event::WarningBegan,
        Event::HuntBegan,
        Event::WarningAverted,
        Event::LostTrack,
        Event::ThreatReturned,
        Event::RelicDelivered,
        Event::Escaped,
        Event::Downed,
        Event::RelicDropped,
        Event::AllBonesHome,
        Event::PowerRestored,
        Event::TruckStarted,
        Event::AjiTaken,
        Event::AjiUsed,
        Event::CountingBegan,
        Event::CountingEnded,
        Event::Susto,
        Event::Revived,
        Event::Died,
        Event::CattleSpooked,
        Event::BeaconLit,
        Event::Prayed,
        Event::BatteriesTaken,
        Event::SkillCheck,
        Event::SkillGreat,
        Event::SkillMissed,
        Event::Weeping,
        Event::WhipCrack,
        Event::BottleClink,
        Event::DogFreed,
        Event::DogGrowl,
        Event::DogBark,
        Event::Hauled,
        Event::SackDropped,
        Event::Taken,
        Event::Banished,
        Event::NameWrong,
        Event::LinesSwitched,
        Event::KeyFound,
        Event::LockRattle,
        Event::OmenLampsDie,
        Event::OmenSilence,
        Event::OmenBones,
        Event::OmenDrag,
        Event::OmenHat,
        Event::OmenPhantom,
        Event::OmenStolenLight,
        Event::OmenFootsteps,
        Event::OmenFalseMark,
    ];

    pub fn from_code(code: u8) -> Option<Event> {
        Self::ALL.get(code as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub warnings: u32,
    pub hunts: u32,
    /// Averted warnings, lost-track withdrawals and counted-bones escapes.
    pub recoveries: u32,
    pub downs: u32,
    pub revives: u32,
}

/// A heard noise he cannot stop thinking about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    pub pos: Vec2,
    pub ttl: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Threat {
    pub state: ThreatState,
    pub presence: Presence,
    pub pos: Vec2,
    /// Unit direction he faces on the ground plane.
    pub facing: Vec2,
    /// Ground speed this tick (for the walk animation).
    pub speed: f32,
    pub movement: Movement,
    /// Seconds in the current state.
    pub state_time: f32,
    /// While > 0 he cannot begin a warning.
    pub cooldown: f32,
    /// 0..1; reaching 1 during a hunt downs the hunted player.
    pub exposure: f32,
    /// Seconds without sight during a warning or hunt.
    pub unseen: f32,
    /// Seconds lurking at the right node without sight.
    pub lurk_time: f32,
    /// He currently has line of sight to his prey.
    pub has_sight: bool,
    /// Out of patience: roaming the whole patrol instead of waiting.
    pub circling: bool,
    /// The last noise that caught his attention.
    pub focus: Option<Focus>,
    /// Seconds left searching the spot of a noise.
    pub search: f32,
    /// Seconds left counting bones.
    pub counting: f32,
    /// Node he came from (roaming does not double back).
    pub prev: usize,
    /// Roaming choice counter: deterministic variety.
    pub turns: u32,
    /// Seconds spent walking without progress.
    pub stall: f32,
    /// Warnings averted since he last hunted or rose anew.
    pub averts: u32,
    /// Hauling: the node he is making for, and seconds since he took them.
    pub haul_to: usize,
    pub hauled: f32,
}

impl Threat {
    fn dormant(layout: &Layout) -> Self {
        Self {
            state: ThreatState::Dormant,
            presence: Presence::Hidden,
            pos: layout.patrol.nodes[0],
            facing: Vec2::new(0.0, 1.0),
            speed: 0.0,
            movement: Movement::Still,
            state_time: 0.0,
            cooldown: 0.0,
            exposure: 0.0,
            unseen: 0.0,
            lurk_time: 0.0,
            has_sight: false,
            circling: false,
            focus: None,
            search: 0.0,
            counting: 0.0,
            prev: 0,
            turns: 0,
            stall: 0.0,
            averts: 0,
            haul_to: 0,
            hauled: 0.0,
        }
    }

    /// 0 = hidden underground, 1 = fully standing.
    pub fn visibility(&self, tuning: &Tuning) -> f32 {
        match self.presence {
            Presence::Hidden => 0.0,
            Presence::Rising { t } => (t / tuning.rise_time).clamp(0.0, 1.0),
            Presence::Present => 1.0,
            Presence::Sinking { t, .. } => 1.0 - (t / tuning.sink_time).clamp(0.0, 1.0),
        }
    }

    fn set_state(&mut self, state: ThreatState) {
        self.state = state;
        self.state_time = 0.0;
        self.unseen = 0.0;
    }
}

/// Where a bone bundle is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Relic {
    Ground(Vec3),
    Carried(PlayerId),
    Delivered,
}

/// A pepper ward on the ground: he cannot cross it, and touching it makes
/// him stop to count his bones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AjiZone {
    pub pos: Vec2,
    pub radius: f32,
    pub life: f32,
    /// He has already stopped to count here.
    pub spent: bool,
}

/// The shared, non-player progress of the run.
#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub relics: Vec<Relic>,
    pub aji_taken: Vec<bool>,
    pub batteries_taken: Vec<bool>,
    /// 0..1: 1 = the windmill runs and the lamps are lit.
    pub power: f32,
    /// Which lines the dynamo feeds once it runs (bit per lamp circuit):
    /// two of the three at most.
    pub circuits: u8,
    /// The truck key is out of its padlocked box.
    pub key: bool,
    /// Seconds before he may be named again after a wrong name.
    pub naming_cooldown: f32,
    /// He was named rightly and laid to rest.
    pub banished: bool,
    /// 0..1: 1 = the engine is running.
    pub truck: f32,
    /// Seconds the engine has been running.
    pub warm: f32,
    /// Seconds the beacon still burns, and until it can be lit again.
    pub beacon: f32,
    pub beacon_cooldown: f32,
    /// Seconds of bellowing left, cooldown, and the unsettled-herd meter.
    pub cattle_alarm: f32,
    pub cattle_cooldown: f32,
    pub cattle_spook: f32,
}

impl Progress {
    fn new(layout: &Layout) -> Self {
        Self {
            relics: layout.district.relics.iter().map(|&p| Relic::Ground(p)).collect(),
            aji_taken: vec![false; layout.district.aji.len()],
            batteries_taken: vec![false; layout.district.batteries.len()],
            power: 0.0,
            circuits: crate::geometry::district::FIRST_CIRCUITS,
            key: false,
            naming_cooldown: 0.0,
            banished: false,
            truck: 0.0,
            warm: 0.0,
            beacon: 0.0,
            beacon_cooldown: 0.0,
            cattle_alarm: 0.0,
            cattle_cooldown: 0.0,
            cattle_spook: 0.0,
        }
    }

    pub fn delivered(&self) -> usize {
        self.relics.iter().filter(|r| matches!(r, Relic::Delivered)).count()
    }
    /// La Rabia's stage: the bundles laid to rest (0..=5). Public as the
    /// bones line; only a laying raises it, never a fall.
    pub fn rage(&self) -> u8 {
        self.delivered().min(usize::from(crate::tuning::MAX_RAGE)) as u8
    }
    pub fn carried_by(&self, id: PlayerId) -> usize {
        self.relics.iter().filter(|r| **r == Relic::Carried(id)).count()
    }
    pub fn carried_total(&self) -> usize {
        self.relics.iter().filter(|r| matches!(r, Relic::Carried(_))).count()
    }
    pub fn bones_home(&self) -> bool {
        self.delivered() == self.relics.len()
    }
    pub fn power_on(&self) -> bool {
        self.power >= 1.0
    }
    /// The lines actually lit right now (none until the pump runs).
    pub fn live_circuits(&self) -> u8 {
        if self.power_on() { self.circuits } else { 0 }
    }
    pub fn truck_running(&self) -> bool {
        self.truck >= 1.0
    }
    /// The engine has run long enough to drive away.
    pub fn truck_ready(&self, tuning: &Tuning) -> bool {
        self.truck_running() && self.warm >= tuning.truck_warmup
    }
    pub fn beacon_ready(&self) -> bool {
        self.beacon <= 0.0 && self.beacon_cooldown <= 0.0
    }
}

/// Which of him walks tonight. The legend tells of three returns, each with
/// its signs and its temper (see the page "Las tres vueltas").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// The Drunkard's: he hears everything and whistles slurred, but
    /// stumbles slower; glass clinks in the grass where he has been.
    Borracho,
    /// The Son himself: he weeps when his father's bones are laid down, each
    /// one laid angers him more, and he lingers longer counting them.
    Hijo,
    /// The Drover: a little faster across the llano, but the cattle bellow
    /// as he passes and a whip cracks in the dark.
    Arriero,
}

impl Variant {
    pub const ALL: [Variant; 3] = [Variant::Borracho, Variant::Hijo, Variant::Arriero];

    /// Tonight's, from the seed.
    pub fn of(seed: u64) -> Self {
        Self::ALL[Rng::fork(seed, 0x7A12).below(3)]
    }

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(code as usize).copied()
    }

    /// How much farther he hears.
    pub fn hearing(self) -> f32 {
        if self == Variant::Borracho { 1.4 } else { 1.0 }
    }

    /// How much each bundle laid to rest angers him.
    pub fn rite(self) -> f32 {
        if self == Variant::Hijo { 1.25 } else { 1.0 }
    }

    /// How much faster he walks and creeps (never his hunt).
    pub fn pace(self) -> f32 {
        match self {
            Variant::Arriero => 1.06,
            Variant::Borracho => 0.92,
            Variant::Hijo => 1.0,
        }
    }

    /// How much longer pepper keeps him counting bones.
    pub fn counting(self) -> f32 {
        if self == Variant::Hijo { 1.3 } else { 1.0 }
    }
}

/// The combination of the padlock on the truck key's box: three digits, a
/// fresh one every seed, written into three of the hacienda's pages (see
/// `lore::fill`).
pub fn lock_code(seed: u64) -> [u8; 3] {
    let mut rng = Rng::fork(seed, 0x10CC);
    [1 + rng.below(9) as u8, rng.below(10) as u8, rng.below(10) as u8]
}

/// The player he is currently interested in, as he perceives them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prey {
    pub pos: Vec2,
    /// Multiplier on how far away he notices them (crouch, light).
    pub sight: f32,
    /// Crouched in tall grass: only visible up close.
    pub concealed: bool,
}

impl Prey {
    pub fn plain(pos: Vec2) -> Self {
        Self {
            pos,
            sight: 1.0,
            concealed: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub outcome: Outcome,
    pub threat: Threat,
    pub progress: Progress,
    pub zones: Vec<AjiZone>,
    pub elapsed: f32,
    pub stats: Stats,
    /// 0..1, recomputed every tick from the night and the run's progress.
    pub pressure: f32,
}

impl Encounter {
    pub fn new(layout: &Layout) -> Self {
        Self {
            outcome: Outcome::Running,
            threat: Threat::dormant(layout),
            progress: Progress::new(layout),
            zones: Vec::new(),
            elapsed: 0.0,
            stats: Stats::default(),
            pressure: 0.0,
        }
    }

    /// Restart: every truth value and timer back to its initial state.
    pub fn reset(&mut self, layout: &Layout) {
        *self = Self::new(layout);
    }

    // ------------------------------------------------------------ the world

    /// Advance the world's own clocks and the night's pressure.
    pub fn tick_world(&mut self, tuning: &Tuning, dt: f32) {
        self.elapsed += dt;
        let p = &mut self.progress;
        p.beacon = (p.beacon - dt).max(0.0);
        if p.beacon <= 0.0 {
            p.beacon_cooldown = (p.beacon_cooldown - dt).max(0.0);
        }
        p.cattle_alarm = (p.cattle_alarm - dt).max(0.0);
        p.naming_cooldown = (p.naming_cooldown - dt).max(0.0);
        p.cattle_cooldown = (p.cattle_cooldown - dt).max(0.0);
        p.cattle_spook = (p.cattle_spook - 0.3 * dt).max(0.0);
        for z in &mut self.zones {
            z.life -= dt;
        }
        self.zones.retain(|z| z.life > 0.0);
        if p.truck_running() {
            p.warm += dt;
        }
        self.pressure = if p.truck_running() {
            1.0
        } else {
            let night = (self.elapsed / tuning.night_length).clamp(0.0, 1.0);
            (tuning.pressure_base
                + tuning.pressure_night * night
                + tuning.pressure_carry * p.carried_total() as f32
                + tuning.pressure_rite * Variant::of(tuning.seed).rite() * p.delivered() as f32)
                .clamp(0.05, tuning.pressure_max)
        };
    }

    /// A player picks up bundle `index`. The first ever pickup wakes him.
    pub fn take_relic(&mut self, index: usize, id: PlayerId, events: &mut Vec<Event>) -> bool {
        match self.progress.relics.get_mut(index) {
            Some(r @ Relic::Ground(_)) => {
                *r = Relic::Carried(id);
                events.push(Event::RelicTaken);
                true
            }
            _ => false,
        }
    }

    /// Put down the first bundle `id` carries at `at`.
    pub fn drop_relic(&mut self, id: PlayerId, at: Vec3, events: &mut Vec<Event>) -> bool {
        if let Some(r) = self.progress.relics.iter_mut().find(|r| **r == Relic::Carried(id)) {
            *r = Relic::Ground(Vec3::new(at.x, at.y + 0.35, at.z));
            events.push(Event::RelicDropped);
            true
        } else {
            false
        }
    }

    /// Everything `id` carries falls around `at` (captured or gone).
    pub fn release_all(&mut self, id: PlayerId, at: Vec3) {
        let mut n = 0.0_f32;
        for r in &mut self.progress.relics {
            if *r == Relic::Carried(id) {
                let a = n * 1.9;
                let spread = if n == 0.0 { 0.0 } else { 0.45 };
                *r = Relic::Ground(Vec3::new(at.x + a.cos() * spread, at.y + 0.35, at.z + a.sin() * spread));
                n += 1.0;
            }
        }
    }

    /// Lay one carried bundle at the altar.
    pub fn deliver_relic(&mut self, id: PlayerId, events: &mut Vec<Event>) -> bool {
        let Some(r) = self.progress.relics.iter_mut().find(|r| **r == Relic::Carried(id)) else {
            return false;
        };
        *r = Relic::Delivered;
        events.push(Event::RelicDelivered);
        if self.progress.bones_home() {
            events.push(Event::AllBonesHome);
        }
        true
    }

    pub fn take_aji(&mut self, index: usize, events: &mut Vec<Event>) -> bool {
        match self.progress.aji_taken.get_mut(index) {
            Some(taken) if !*taken => {
                *taken = true;
                events.push(Event::AjiTaken);
                true
            }
            _ => false,
        }
    }

    /// Take spare batteries `index` from where they lie.
    pub fn take_batteries(&mut self, index: usize, events: &mut Vec<Event>) -> bool {
        match self.progress.batteries_taken.get_mut(index) {
            Some(taken) if !*taken => {
                *taken = true;
                events.push(Event::BatteriesTaken);
                true
            }
            _ => false,
        }
    }

    /// Scatter pepper at `pos`.
    pub fn place_aji(&mut self, tuning: &Tuning, pos: Vec2, events: &mut Vec<Event>) {
        self.zones.push(AjiZone {
            pos,
            radius: tuning.aji_zone_radius,
            life: tuning.aji_zone_life,
            spent: false,
        });
        events.push(Event::AjiUsed);
    }

    /// Crank the pump for `dt` seconds.
    pub fn work_pump(&mut self, tuning: &Tuning, dt: f32, events: &mut Vec<Event>) {
        if self.progress.power_on() {
            return;
        }
        self.progress.power = (self.progress.power + dt / tuning.pump_hold).min(1.0);
        if self.progress.power_on() {
            events.push(Event::PowerRestored);
        }
    }

    /// Name him at the ceiba with every bundle laid there. The right name
    /// lays him to rest and ends the night; a wrong one rouses him, furious,
    /// and the ceiba will not hear another name for a while.
    pub fn name_him(
        &mut self,
        layout: &Layout,
        tuning: &Tuning,
        guess: Variant,
        watchers: &[Vec2],
        events: &mut Vec<Event>,
    ) -> Result<bool, &'static str> {
        if !self.progress.bones_home() {
            return Err("The father's bones are not all at rest yet.");
        }
        if self.progress.naming_cooldown > 0.0 {
            return Err("The ceiba is not listening yet.");
        }
        if guess == Variant::of(tuning.seed) {
            self.progress.banished = true;
            self.outcome = Outcome::Won;
            self.threat.presence = Presence::Sinking {
                t: 0.0,
                relocate: false,
            };
            events.push(Event::Banished);
            Ok(true)
        } else {
            self.progress.naming_cooldown = tuning.naming_cooldown;
            events.push(Event::NameWrong);
            self.rouse(layout, tuning, watchers, events);
            self.threat.cooldown = 0.0;
            Ok(false)
        }
    }

    /// Throw the panel's switch: the dynamo feeds the next pair of lines.
    /// Nothing to switch before the pump runs.
    pub fn switch_lines(&mut self, events: &mut Vec<Event>) -> bool {
        use crate::geometry::district::CIRCUIT_SETTINGS;
        if !self.progress.power_on() {
            return false;
        }
        let at = CIRCUIT_SETTINGS
            .iter()
            .position(|&c| c == self.progress.circuits)
            .unwrap_or(0);
        self.progress.circuits = CIRCUIT_SETTINGS[(at + 1) % CIRCUIT_SETTINGS.len()];
        events.push(Event::LinesSwitched);
        true
    }

    /// Try a combination on the key box's padlock.
    pub fn try_code(&mut self, tuning: &Tuning, code: [u8; 3], events: &mut Vec<Event>) -> bool {
        if self.progress.key {
            return false;
        }
        if code == lock_code(tuning.seed) {
            self.progress.key = true;
            events.push(Event::KeyFound);
            true
        } else {
            events.push(Event::LockRattle);
            false
        }
    }

    /// Turn the ignition for `dt` seconds. Refused until the bones are home,
    /// the power is on and the key is out of its box.
    pub fn work_truck(&mut self, tuning: &Tuning, dt: f32, events: &mut Vec<Event>) -> bool {
        if !self.progress.bones_home()
            || !self.progress.power_on()
            || !self.progress.key
            || self.progress.truck_running()
        {
            return false;
        }
        self.progress.truck = (self.progress.truck + dt / tuning.truck_hold).min(1.0);
        if self.progress.truck_running() {
            events.push(Event::TruckStarted);
        }
        true
    }

    pub fn light_beacon(&mut self, tuning: &Tuning, events: &mut Vec<Event>) -> bool {
        if !self.progress.beacon_ready() {
            return false;
        }
        self.progress.beacon = tuning.beacon_burn;
        self.progress.beacon_cooldown = tuning.beacon_cooldown;
        events.push(Event::BeaconLit);
        true
    }

    // ------------------------------------------------------------ the threat

    /// He rises far from everyone. Only from dormancy (or, rarely, hidden).
    pub fn manifest(&mut self, layout: &Layout, tuning: &Tuning, watchers: &[Vec2], events: &mut Vec<Event>) {
        let (node, _) = layout.patrol.farthest_from(watchers);
        let th = &mut self.threat;
        let nearest = watchers
            .iter()
            .copied()
            .min_by(|a, b| {
                a.distance_squared(layout.patrol.nodes[node])
                    .total_cmp(&b.distance_squared(layout.patrol.nodes[node]))
            })
            .unwrap_or(Vec2::ZERO);
        th.set_state(ThreatState::Stalking);
        th.presence = Presence::Rising { t: 0.0 };
        th.pos = layout.patrol.nodes[node];
        th.facing = (nearest - th.pos).normalize_or(Vec2::Y);
        th.movement = Movement::AtAnchor(node);
        th.prev = node;
        th.cooldown = tuning.first_warn_delay;
        th.exposure = 0.0;
        th.lurk_time = 0.0;
        th.circling = false;
        th.focus = None;
        events.push(Event::ThreatManifested);
    }

    /// The engine roars: whatever he was doing, he comes. Wakes him if needed.
    pub fn rouse(&mut self, layout: &Layout, tuning: &Tuning, watchers: &[Vec2], events: &mut Vec<Event>) {
        if self.threat.state == ThreatState::Dormant || matches!(self.threat.presence, Presence::Hidden) {
            self.manifest(layout, tuning, watchers, events);
        }
        self.threat.cooldown = self.threat.cooldown.min(2.0);
        self.threat.circling = true;
    }

    /// Instead of backing off after a catch, he stuffs the fallen into his
    /// sack and walks off toward the node farthest from everyone standing.
    pub fn haul(&mut self, layout: &Layout, watchers: &[Vec2], events: &mut Vec<Event>) {
        let (node, _) = layout.patrol.farthest_from(watchers);
        let th = &mut self.threat;
        th.set_state(ThreatState::Hauling);
        th.presence = Presence::Present;
        th.exposure = 0.0;
        th.has_sight = false;
        th.focus = None;
        th.circling = false;
        th.haul_to = node;
        th.hauled = 0.0;
        th.movement = Movement::Return {
            home: layout.patrol.nearest(th.pos),
        };
        events.push(Event::Hauled);
    }

    /// A dog barks at him up close: whatever he was doing, he flinches and
    /// sinks away into the grass, to rise far off. Whoever was in his sack
    /// falls out. True when he was there to be barked off.
    pub fn flinch(&mut self, events: &mut Vec<Event>) -> bool {
        let th = &self.threat;
        if th.state == ThreatState::Dormant || !matches!(th.presence, Presence::Present) {
            return false;
        }
        if th.state == ThreatState::Hauling {
            events.push(Event::SackDropped);
        }
        if matches!(
            th.state,
            ThreatState::Warning | ThreatState::Hunting | ThreatState::Hauling
        ) {
            self.stats.recoveries += 1;
        }
        events.push(Event::DogBark);
        self.withdraw();
        true
    }

    /// He backs off after a capture, or when his prey vanishes.
    pub fn withdraw(&mut self) {
        let th = &mut self.threat;
        if th.state == ThreatState::Dormant {
            return;
        }
        th.set_state(ThreatState::Stalking);
        th.exposure = 0.0;
        th.has_sight = false;
        th.cooldown = 10.0;
        th.movement = Movement::Still;
        th.focus = None;
        th.counting = 0.0;
        th.presence = Presence::Sinking { t: 0.0, relocate: true };
    }

    /// He hears a noise of audible `radius` at `pos` (already scaled for
    /// weather by the caller). Only a calm, present Silbón is drawn to it.
    pub fn hear(&mut self, tuning: &Tuning, pos: Vec2, radius: f32) {
        let th = &mut self.threat;
        if th.state != ThreatState::Stalking || !matches!(th.presence, Presence::Present) {
            return;
        }
        let reach = radius * tuning.hearing_gain(self.pressure) * Variant::of(tuning.seed).hearing();
        if th.pos.distance(pos) > reach {
            return;
        }
        th.focus = Some(Focus {
            pos,
            ttl: tuning.noise_memory,
        });
        th.circling = false;
        th.lurk_time = 0.0;
        if matches!(th.movement, Movement::Search) {
            th.search = tuning.search_time;
        }
    }

    /// Advance the threat by one tick against his current `prey`.
    /// `watchers` are the positions of every active player (safe relocation).
    pub fn update_threat(
        &mut self,
        layout: &Layout,
        base: &Tuning,
        prey: Option<Prey>,
        watchers: &[Vec2],
        dt: f32,
        events: &mut Vec<Event>,
    ) {
        let mut scaled = base.at_rage(self.progress.rage()).at_pressure(self.pressure);
        let pace = Variant::of(base.seed).pace();
        scaled.stalk_speed *= pace;
        scaled.creep_speed *= pace;
        scaled.counting_time *= Variant::of(base.seed).counting();
        let tuning = &scaled;
        let toward = prey.map_or(Vec2::Y, |p| p.pos);
        {
            let th = &mut self.threat;
            th.speed = 0.0;
            match th.presence {
                Presence::Hidden => return,
                Presence::Rising { t } => {
                    let t = t + dt;
                    th.facing = (toward - th.pos).normalize_or(th.facing);
                    th.presence = if t >= tuning.rise_time {
                        Presence::Present
                    } else {
                        Presence::Rising { t }
                    };
                    th.has_sight = false;
                    th.cooldown = (th.cooldown - dt).max(0.0);
                    return;
                }
                Presence::Sinking { t, relocate } => {
                    let t = t + dt;
                    th.has_sight = false;
                    if t < tuning.sink_time {
                        th.presence = Presence::Sinking { t, relocate };
                    } else if relocate {
                        let (node, _) = layout.patrol.farthest_from(watchers);
                        th.pos = layout.patrol.nodes[node];
                        th.facing = (toward - th.pos).normalize_or(th.facing);
                        th.movement = Movement::AtAnchor(node);
                        th.prev = node;
                        th.presence = Presence::Rising { t: 0.0 };
                        events.push(Event::ThreatReturned);
                    } else {
                        th.presence = Presence::Hidden;
                    }
                    return;
                }
                Presence::Present => {}
            }
            if th.state == ThreatState::Dormant {
                return;
            }
            th.cooldown = (th.cooldown - dt).max(0.0);
            th.state_time += dt;
            if let Some(f) = &mut th.focus {
                f.ttl -= dt;
                if f.ttl <= 0.0 {
                    th.focus = None;
                }
            }
        }

        // Hauling: he walks the patrol toward the far node with the fallen in
        // his sack, blind to everything else. A ward in his way makes him
        // count (and drop them); reaching the node, or taking too long to be
        // stopped, and they are gone.
        if self.threat.state == ThreatState::Hauling {
            self.threat.hauled += dt;
            self.threat.has_sight = false;
            let step = tuning.haul_speed * dt;
            let patrol = &layout.patrol;
            let before = self.threat.pos;
            let movement = match self.threat.movement {
                Movement::AtAnchor(i) if i == self.threat.haul_to => Movement::AtAnchor(i),
                Movement::AtAnchor(i) => Movement::Walk {
                    from: i,
                    to: patrol.next_hop(i, self.threat.haul_to),
                },
                Movement::Walk { from, to } => {
                    if self.step_toward(layout, patrol.nodes[to], step, tuning.counting_time) {
                        Movement::AtAnchor(to)
                    } else {
                        Movement::Walk { from, to }
                    }
                }
                Movement::Return { home } => {
                    if self.step_toward(layout, patrol.nodes[home], step, tuning.counting_time) {
                        Movement::AtAnchor(home)
                    } else {
                        Movement::Return { home }
                    }
                }
                other => other,
            };
            if self.threat.state == ThreatState::Counting {
                // A ward stopped him mid-stride: the sack drops.
                self.threat.movement = Movement::Still;
                self.threat.speed = 0.0;
                events.push(Event::CountingBegan);
                events.push(Event::SackDropped);
                return;
            }
            let th = &mut self.threat;
            th.movement = movement;
            th.speed = before.distance(th.pos) / dt.max(1e-6);
            let arrived = matches!(movement, Movement::AtAnchor(i) if i == th.haul_to);
            if arrived || th.hauled >= tuning.haul_time {
                events.push(Event::Taken);
                self.withdraw();
            }
            return;
        }

        // Counting bones: he sees and hears nothing until he is done.
        if self.threat.state == ThreatState::Counting {
            let th = &mut self.threat;
            th.has_sight = false;
            th.movement = Movement::Still;
            th.counting -= dt;
            if th.counting <= 0.0 {
                th.set_state(ThreatState::Stalking);
                th.cooldown = tuning.recover_cooldown;
                th.exposure = 0.0;
                th.focus = None;
                th.circling = false;
                th.lurk_time = 0.0;
                th.movement = Movement::Return {
                    home: layout.patrol.nearest(th.pos),
                };
                self.stats.recoveries += 1;
                events.push(Event::CountingEnded);
            }
            return;
        }

        let Some(prey) = prey else {
            return;
        };
        let to_prey = prey.pos - self.threat.pos;
        let dist = to_prey.length();
        {
            let th = &mut self.threat;
            let los = layout.line_of_sight(th.pos, prey.pos);
            th.has_sight = los && !(prey.concealed && dist > tuning.grass_sight);
        }
        let notice = tuning.warn_distance * prey.sight;

        match self.threat.state {
            ThreatState::Stalking => {
                self.stalk(layout, tuning, prey, dt);
                let th = &mut self.threat;
                let dist = th.pos.distance(prey.pos);
                if th.state == ThreatState::Stalking && th.has_sight && dist <= notice && th.cooldown <= 0.0 {
                    th.set_state(ThreatState::Warning);
                    th.movement = Movement::Still;
                    th.speed = 0.0;
                    th.focus = None;
                    self.stats.warnings += 1;
                    events.push(Event::WarningBegan);
                }
            }
            ThreatState::Warning => {
                let th = &mut self.threat;
                th.facing = to_prey.normalize_or(th.facing);
                if th.has_sight {
                    th.unseen = 0.0;
                } else {
                    th.unseen += dt;
                }
                if th.unseen >= tuning.warn_break_time {
                    th.set_state(ThreatState::Stalking);
                    th.cooldown = tuning.warn_recover_cooldown;
                    th.movement = Movement::Return {
                        home: layout.patrol.nearest(th.pos),
                    };
                    self.stats.recoveries += 1;
                    events.push(Event::WarningAverted);
                    th.averts += 1;
                    if th.averts >= tuning.averts_to_withdraw {
                        // Tired of waiting over his prey: he goes elsewhere.
                        th.averts = 0;
                        th.cooldown = tuning.recover_cooldown;
                        th.movement = Movement::Still;
                        th.circling = false;
                        th.lurk_time = 0.0;
                        th.focus = None;
                        th.presence = Presence::Sinking { t: 0.0, relocate: true };
                        events.push(Event::LostTrack);
                    }
                } else if th.state_time >= tuning.warn_time && th.has_sight {
                    th.set_state(ThreatState::Hunting);
                    th.exposure = 0.0;
                    th.averts = 0;
                    self.stats.hunts += 1;
                    events.push(Event::HuntBegan);
                }
            }
            ThreatState::Hunting => {
                if self.threat.has_sight {
                    self.threat.unseen = 0.0;
                    self.threat.facing = to_prey.normalize_or(self.threat.facing);
                    let stop = tuning.catch_distance * 0.8;
                    if dist > stop {
                        let step = (tuning.hunt_speed * dt).min(dist - stop);
                        let before = self.threat.pos;
                        self.step_toward(layout, prey.pos, step, tuning.counting_time);
                        let th = &mut self.threat;
                        th.speed = before.distance(th.pos) / dt.max(1e-6);
                    }
                    if self.threat.state == ThreatState::Counting {
                        events.push(Event::CountingBegan);
                        return;
                    }
                    let th = &mut self.threat;
                    let near = 1.0 - (dist / tuning.warn_distance).clamp(0.0, 1.0);
                    th.exposure += dt / tuning.exposure_time * (1.0 + tuning.exposure_near_boost * near);
                    let dist_now = th.pos.distance(prey.pos);
                    if th.exposure >= 1.0 || dist_now <= tuning.catch_distance {
                        th.exposure = th.exposure.min(1.0);
                        self.stats.downs += 1;
                        events.push(Event::Downed);
                        self.withdraw();
                    }
                } else {
                    let th = &mut self.threat;
                    th.unseen += dt;
                    th.exposure = (th.exposure - tuning.exposure_decay * dt).max(0.0);
                    if th.unseen >= tuning.lose_track_time {
                        th.set_state(ThreatState::Stalking);
                        th.exposure = 0.0;
                        th.cooldown = tuning.recover_cooldown;
                        th.movement = Movement::Still;
                        th.circling = false;
                        th.lurk_time = 0.0;
                        th.presence = Presence::Sinking { t: 0.0, relocate: true };
                        self.stats.recoveries += 1;
                        events.push(Event::LostTrack);
                    }
                }
            }
            ThreatState::Dormant | ThreatState::Counting | ThreatState::Hauling => {}
        }
        if self.threat.state == ThreatState::Counting {
            events.push(Event::CountingBegan);
        }
    }

    /// Move him up to `max` metres straight toward `target`, sliding along
    /// blockers. Pepper wards stop him: the first touch makes him count for
    /// `counting_time`, a spent ward is a wall he will not walk deeper into.
    /// Returns true once there.
    fn step_toward(&mut self, layout: &Layout, target: Vec2, max: f32, counting_time: f32) -> bool {
        let th = &mut self.threat;
        let d = target - th.pos;
        let len = d.length();
        if len < 1e-4 {
            return true;
        }
        let dir = d / len;
        th.facing = dir;
        let want = layout.move_circle(th.pos, dir * max.min(len), 0.42);
        for z in &mut self.zones {
            let (now, next) = (th.pos.distance(z.pos), want.distance(z.pos));
            if next < z.radius && next < now {
                if !z.spent {
                    z.spent = true;
                    th.pos = want;
                    th.facing = (z.pos - want).normalize_or(dir);
                    th.counting = counting_time;
                    th.set_state(ThreatState::Counting);
                    th.movement = Movement::Still;
                }
                return false;
            }
        }
        th.pos = want;
        want.distance(target) < 0.05
    }

    /// Stalking: walk the patrol toward what interests him, lurk at a
    /// standoff from a player he cannot see, investigate noise, creep in on a
    /// visible player, and roam when bored.
    fn stalk(&mut self, layout: &Layout, tuning: &Tuning, prey: Prey, dt: f32) {
        let patrol = &layout.patrol;
        let noise = self.threat.focus.map(|f| f.pos);
        let attention = noise.unwrap_or(prey.pos);
        let target = if noise.is_some() {
            patrol.nearest(attention)
        } else {
            patrol.lurk_node(attention, tuning.standoff)
        };
        let dist_prey = self.threat.pos.distance(prey.pos);
        let notice = tuning.warn_distance * prey.sight;
        if self.threat.has_sight {
            self.threat.lurk_time = 0.0;
            self.threat.circling = false;
        }
        let before = self.threat.pos;
        let mut walked = false;
        let movement = self.threat.movement;
        let next = match movement {
            Movement::Still => Movement::Return {
                home: patrol.nearest(self.threat.pos),
            },
            Movement::AtAnchor(i) => {
                let th = &mut self.threat;
                if th.circling {
                    let nbrs = patrol.neighbors(i);
                    let choices: Vec<usize> = nbrs.iter().copied().filter(|&n| n != th.prev).collect();
                    let pool = if choices.is_empty() { nbrs.to_vec() } else { choices };
                    let to = pool[th.turns as usize % pool.len()];
                    th.turns = th.turns.wrapping_add(1);
                    Movement::Walk { from: i, to }
                } else if i != target {
                    Movement::Walk {
                        from: i,
                        to: patrol.next_hop(i, target),
                    }
                } else if noise.is_some() {
                    if th.pos.distance(attention) > 3.0 {
                        Movement::Investigate { home: i }
                    } else {
                        th.search = tuning.search_time;
                        Movement::Search
                    }
                } else {
                    // Lurking at the right node.
                    th.facing = (prey.pos - th.pos).normalize_or(th.facing);
                    if th.has_sight && th.cooldown <= 0.0 && dist_prey > notice {
                        Movement::Creep { home: i }
                    } else {
                        if !th.has_sight {
                            th.lurk_time += dt;
                            if th.lurk_time >= tuning.patience {
                                th.circling = true;
                            }
                        }
                        Movement::AtAnchor(i)
                    }
                }
            }
            Movement::Walk { from, to } => {
                // Turn around if the short way to the target now lies behind him.
                let (from, to) = if !self.threat.circling && patrol.path_len(from, target) < patrol.path_len(to, target)
                {
                    (to, from)
                } else {
                    (from, to)
                };
                let arrived = self.step_toward(layout, patrol.nodes[to], tuning.stalk_speed * dt, tuning.counting_time);
                walked = true;
                if arrived {
                    self.threat.prev = from;
                    Movement::AtAnchor(to)
                } else {
                    Movement::Walk { from, to }
                }
            }
            Movement::Creep { home } => {
                let th = &self.threat;
                if !th.has_sight || patrol.lurk_node(prey.pos, tuning.standoff) != home {
                    Movement::Return { home }
                } else {
                    let stop = tuning.warn_distance * 0.5;
                    if dist_prey > stop {
                        self.step_toward(
                            layout,
                            prey.pos,
                            (tuning.creep_speed * dt).min(dist_prey - stop),
                            tuning.counting_time,
                        );
                        walked = true;
                    }
                    Movement::Creep { home }
                }
            }
            Movement::Return { home } => {
                let arrived = self.step_toward(
                    layout,
                    patrol.nodes[home],
                    tuning.stalk_speed * dt,
                    tuning.counting_time,
                );
                walked = true;
                if arrived {
                    self.threat.prev = home;
                    Movement::AtAnchor(home)
                } else {
                    Movement::Return { home }
                }
            }
            Movement::Investigate { home } => match noise {
                None => Movement::Return { home },
                Some(spot) => {
                    if self.threat.pos.distance(spot) <= 3.0 {
                        self.threat.search = tuning.search_time;
                        Movement::Search
                    } else {
                        self.step_toward(layout, spot, tuning.stalk_speed * dt, tuning.counting_time);
                        walked = true;
                        // Stalled against something: give the noise up.
                        let th = &mut self.threat;
                        let progress = th.pos.distance(before);
                        if progress < tuning.stalk_speed * dt * 0.25 {
                            th.stall += dt;
                        } else {
                            th.stall = 0.0;
                        }
                        if th.stall > 1.5 {
                            th.stall = 0.0;
                            th.focus = None;
                            Movement::Return { home }
                        } else {
                            Movement::Investigate { home }
                        }
                    }
                }
            },
            Movement::Search => {
                let th = &mut self.threat;
                match noise {
                    Some(spot) => {
                        th.facing = (spot - th.pos).normalize_or(th.facing);
                        th.search -= dt;
                        if th.search <= 0.0 {
                            th.focus = None;
                            Movement::Return {
                                home: patrol.nearest(th.pos),
                            }
                        } else {
                            Movement::Search
                        }
                    }
                    None => Movement::Return {
                        home: patrol.nearest(th.pos),
                    },
                }
            }
        };
        let th = &mut self.threat;
        if th.state == ThreatState::Counting {
            // A ward stopped him mid-step.
            th.movement = Movement::Still;
            th.speed = 0.0;
            return;
        }
        th.movement = next;
        th.speed = if walked {
            th.pos.distance(before) / dt.max(1e-6)
        } else {
            0.0
        };
        if th.circling && th.has_sight {
            th.circling = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Layout, Tuning, Encounter, Vec<Event>) {
        let layout = Layout::new();
        let enc = Encounter::new(&layout);
        (layout, Tuning::default(), enc, Vec::new())
    }

    fn tick(enc: &mut Encounter, l: &Layout, t: &Tuning, player: Vec2, ev: &mut Vec<Event>) {
        enc.tick_world(t, 1.0 / 60.0);
        enc.update_threat(l, t, Some(Prey::plain(player)), &[player], 1.0 / 60.0, ev);
    }

    fn run_for(enc: &mut Encounter, l: &Layout, t: &Tuning, player: Vec2, secs: f32, ev: &mut Vec<Event>) {
        for _ in 0..(secs * 60.0) as usize {
            tick(enc, l, t, player, ev);
        }
    }

    /// Put a present, stalking Silbón at `pos` with no cooldown.
    fn place_threat(enc: &mut Encounter, l: &Layout, pos: Vec2) {
        let th = &mut enc.threat;
        th.state = ThreatState::Stalking;
        th.presence = Presence::Present;
        th.pos = pos;
        th.movement = Movement::AtAnchor(l.patrol.nearest(pos));
        th.cooldown = 0.0;
    }

    /// Open ground in the ranch yard he can see across, and its patrol node.
    const YARD: Vec2 = Vec2::new(0.0, 12.0);
    const YARD_NODE: Vec2 = Vec2::new(16.0, 8.0);
    /// Inside the house, out of line with door and windows.
    const HIDDEN: Vec2 = Vec2::new(-3.8, -4.9);

    #[test]
    fn manifestation_keeps_clear_of_every_player() {
        let (l, t, mut enc, mut ev) = setup();
        let watchers = [Vec2::new(3.0, -3.0), Vec2::new(-20.0, 8.0)];
        enc.manifest(&l, &t, &watchers, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert!(matches!(enc.threat.presence, Presence::Rising { .. }));
        for w in watchers {
            assert!(enc.threat.pos.distance(w) >= t.manifest_min_distance);
        }
        assert!(ev.contains(&Event::ThreatManifested));
        assert!(enc.threat.cooldown >= t.first_warn_delay);
    }

    #[test]
    fn warning_then_hunt_when_he_keeps_seeing_you() {
        let (l, t, mut enc, mut ev) = setup();
        assert!(l.line_of_sight(YARD_NODE, YARD));
        place_threat(&mut enc, &l, YARD_NODE);
        tick(&mut enc, &l, &t, YARD, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Warning);
        assert!(ev.contains(&Event::WarningBegan));
        run_for(&mut enc, &l, &t, YARD, t.warn_time + 0.5, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        assert!(ev.contains(&Event::HuntBegan));
        // Standing in the open: exposure fills and the hunted player goes down.
        let mut guard = 0;
        while !ev.contains(&Event::Downed) {
            tick(&mut enc, &l, &t, YARD, &mut ev);
            guard += 1;
            assert!(guard < 60 * 30, "never caught while exposed");
        }
        // Fair: the hunt lasted at least a few seconds before the catch.
        assert!(guard as f32 / 60.0 > 1.5);
        // He withdraws after a catch instead of standing over the body.
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert!(matches!(enc.threat.presence, Presence::Sinking { relocate: true, .. }));
        assert_eq!(enc.stats.downs, 1);
    }

    #[test]
    fn breaking_sight_during_warning_averts_the_hunt() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        tick(&mut enc, &l, &t, YARD, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Warning);
        assert!(!l.line_of_sight(enc.threat.pos, HIDDEN));
        run_for(&mut enc, &l, &t, HIDDEN, t.warn_break_time + 0.2, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert!(ev.contains(&Event::WarningAverted));
        assert!(!ev.contains(&Event::HuntBegan));
        assert!(enc.threat.cooldown > 0.0);
    }

    #[test]
    fn breaking_sight_during_hunt_makes_him_lose_track_and_withdraw() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        run_for(&mut enc, &l, &t, YARD, t.warn_time + 0.2, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        run_for(&mut enc, &l, &t, YARD, 1.0, &mut ev);
        let exposure_seen = enc.threat.exposure;
        assert!(exposure_seen > 0.0);
        // Somewhere inside the house he cannot see from where he now stands.
        let hidden = [
            HIDDEN,
            Vec2::new(-4.3, -2.0),
            Vec2::new(4.2, -3.0),
            Vec2::new(-2.0, -5.5),
            Vec2::new(2.0, -5.6),
        ]
        .into_iter()
        .find(|c| !l.line_of_sight(enc.threat.pos, *c))
        .expect("some spot in the house is out of his sight");
        run_for(&mut enc, &l, &t, hidden, 1.0, &mut ev);
        // Still hunting, but no longer approaching and exposure recovering.
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        assert!(enc.threat.exposure < exposure_seen);
        let held = enc.threat.pos;
        run_for(&mut enc, &l, &t, hidden, 0.2, &mut ev);
        assert_eq!(enc.threat.pos, held);
        run_for(&mut enc, &l, &t, hidden, t.lose_track_time, &mut ev);
        assert!(ev.contains(&Event::LostTrack));
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        // He sinks and rises again far away, never on top of anyone.
        run_for(&mut enc, &l, &t, hidden, t.sink_time + t.rise_time + 0.3, &mut ev);
        assert!(ev.contains(&Event::ThreatReturned));
        assert!(enc.threat.pos.distance(hidden) >= t.manifest_min_distance);
        assert_eq!(enc.threat.presence, Presence::Present);
        assert!(!ev.contains(&Event::Downed));
    }

    #[test]
    fn averted_again_and_again_he_tires_of_waiting_and_rises_elsewhere() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        for round in 0..t.averts_to_withdraw {
            enc.threat.cooldown = 0.0;
            enc.threat.presence = Presence::Present;
            tick(&mut enc, &l, &t, YARD, &mut ev);
            assert_eq!(enc.threat.state, ThreatState::Warning, "round {round}");
            run_for(&mut enc, &l, &t, HIDDEN, t.warn_break_time + 0.1, &mut ev);
            assert_eq!(enc.threat.state, ThreatState::Stalking);
        }
        assert_eq!(ev.iter().filter(|e| **e == Event::WarningAverted).count(), 3);
        assert!(ev.contains(&Event::LostTrack), "he gives up his watch");
        assert!(matches!(enc.threat.presence, Presence::Sinking { relocate: true, .. }));
        run_for(&mut enc, &l, &t, HIDDEN, t.sink_time + t.rise_time + 0.3, &mut ev);
        assert!(enc.threat.pos.distance(HIDDEN) >= t.manifest_min_distance);
        assert_eq!(enc.threat.averts, 0);
    }

    #[test]
    fn he_never_approaches_through_walls() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        enc.threat.state = ThreatState::Hunting;
        let start = enc.threat.pos;
        run_for(&mut enc, &l, &t, HIDDEN, 1.5, &mut ev);
        assert_eq!(enc.threat.pos, start);
        assert_eq!(enc.threat.exposure, 0.0);
    }

    #[test]
    fn crouching_shrinks_his_notice_and_grass_hides_you_beyond_a_few_metres() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        // Visible and in range for a standing player, but a crouched one
        // farther than 0.6 x the warning distance goes unnoticed.
        let far = Vec2::new(-5.0, 14.0);
        let d = YARD_NODE.distance(far);
        assert!(
            d < t.warn_distance && d > t.warn_distance * t.sight_crouch,
            "test geometry: {d}"
        );
        assert!(l.line_of_sight(YARD_NODE, far));
        let sneaky = Prey {
            pos: far,
            sight: t.sight_crouch,
            concealed: false,
        };
        for _ in 0..30 {
            enc.tick_world(&t, 1.0 / 60.0);
            enc.update_threat(&l, &t, Some(sneaky), &[far], 1.0 / 60.0, &mut ev);
        }
        assert_ne!(
            enc.threat.state,
            ThreatState::Warning,
            "a crouched player should slip by"
        );
        // A concealed player in grass is not even seen at the same range.
        place_threat(&mut enc, &l, YARD_NODE);
        let hidden = Prey {
            pos: far,
            sight: 1.0,
            concealed: true,
        };
        for _ in 0..30 {
            enc.tick_world(&t, 1.0 / 60.0);
            enc.update_threat(&l, &t, Some(hidden), &[far], 1.0 / 60.0, &mut ev);
        }
        assert!(!enc.threat.has_sight);
        assert_ne!(enc.threat.state, ThreatState::Warning);
        // The same player standing is noticed at once.
        place_threat(&mut enc, &l, YARD_NODE);
        enc.update_threat(&l, &t, Some(Prey::plain(far)), &[far], 1.0 / 60.0, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Warning);
    }

    #[test]
    fn a_noise_draws_him_to_search_the_spot_then_he_gives_up() {
        let (l, t, mut enc, mut ev) = setup();
        // He stands at a far node; the player is elsewhere and out of sight.
        let node = Vec2::new(35.0, 2.0);
        place_threat(&mut enc, &l, node);
        let player = Vec2::new(-30.0, 0.0);
        let noise = Vec2::new(24.0, 14.0);
        // Out of earshot: ignored.
        enc.hear(&t, noise + Vec2::new(0.0, 70.0), 8.0);
        assert!(enc.threat.focus.is_none());
        // Within earshot: he takes an interest.
        enc.hear(&t, noise, 20.0);
        assert!(enc.threat.focus.is_some());
        let mut closest = f32::MAX;
        let mut searched = false;
        for _ in 0..(40 * 60) {
            tick(&mut enc, &l, &t, player, &mut ev);
            closest = closest.min(enc.threat.pos.distance(noise));
            searched |= matches!(enc.threat.movement, Movement::Search);
        }
        assert!(closest <= 3.5, "he never went to look: closest {closest}");
        assert!(searched, "he should stop and listen at the spot");
        assert!(enc.threat.focus.is_none(), "he must eventually give the noise up");
        assert_ne!(enc.threat.state, ThreatState::Dormant);
    }

    #[test]
    fn a_calm_present_threat_only_hears_and_a_busy_one_ignores() {
        let (l, t, mut enc, _) = setup();
        // Not manifested: deaf.
        enc.hear(&t, enc.threat.pos, 50.0);
        assert!(enc.threat.focus.is_none());
        place_threat(&mut enc, &l, Vec2::new(35.0, 2.0));
        enc.threat.state = ThreatState::Hunting;
        enc.hear(&t, Vec2::new(36.0, 3.0), 50.0);
        assert!(enc.threat.focus.is_none(), "a hunting threat is not distracted");
        enc.threat.state = ThreatState::Stalking;
        enc.pressure = 1.0;
        let calm_reach = 10.0;
        // The night lets him hear farther than the same noise by day.
        let at = Vec2::new(35.0 + calm_reach * 1.4, 2.0);
        enc.hear(&t, at, calm_reach);
        assert!(enc.threat.focus.is_some());
        enc.threat.focus = None;
        enc.pressure = 0.0;
        enc.hear(&t, at, calm_reach);
        assert!(enc.threat.focus.is_none());
    }

    #[test]
    fn pepper_ward_stops_a_hunt_makes_him_count_and_blocks_the_way() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, YARD_NODE);
        run_for(&mut enc, &l, &t, YARD, t.warn_time + 0.3, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Hunting);
        // The player scatters pepper between them.
        let between = YARD + (YARD_NODE - YARD).normalize() * 4.0;
        enc.place_aji(&t, between, &mut ev);
        assert!(ev.contains(&Event::AjiUsed));
        let mut guard = 0;
        while enc.threat.state == ThreatState::Hunting {
            tick(&mut enc, &l, &t, YARD, &mut ev);
            guard += 1;
            assert!(guard < 60 * 20, "he walked through the pepper");
        }
        assert_eq!(enc.threat.state, ThreatState::Counting);
        assert!(ev.contains(&Event::CountingBegan));
        assert!(!ev.contains(&Event::Downed));
        assert!(enc.threat.pos.distance(YARD) > 2.0, "he must stop short of the player");
        // While he counts he neither sees nor warns nor moves.
        let held = enc.threat.pos;
        run_for(&mut enc, &l, &t, YARD, t.counting_time - 0.5, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Counting);
        assert_eq!(enc.threat.pos, held);
        assert!(!enc.threat.has_sight);
        // Then he leaves, shaken, with a long cooldown and no exposure.
        run_for(&mut enc, &l, &t, YARD, 1.0, &mut ev);
        assert_eq!(enc.threat.state, ThreatState::Stalking);
        assert!(ev.contains(&Event::CountingEnded));
        assert!(enc.threat.cooldown > 5.0 && enc.threat.exposure == 0.0);
        // The spent ward is a wall for as long as it lasts.
        assert!(enc.zones.iter().any(|z| z.spent));
        let z = enc.zones[0];
        enc.threat.state = ThreatState::Hunting;
        enc.threat.cooldown = 0.0;
        run_for(&mut enc, &l, &t, YARD, 2.0, &mut ev);
        assert!(enc.threat.pos.distance(z.pos) >= z.radius - 0.6, "crossed a spent ward");
        // Wards fade.
        run_for(&mut enc, &l, &t, HIDDEN, t.aji_zone_life, &mut ev);
        assert!(enc.zones.is_empty());
    }

    #[test]
    fn roaming_never_doubles_back_needlessly_and_visits_the_whole_map() {
        let (l, t, mut enc, mut ev) = setup();
        place_threat(&mut enc, &l, Vec2::new(0.0, 10.0));
        enc.threat.circling = true;
        // Crouched in tall grass far away: he never sees his prey, so he
        // never stops roaming to lurk.
        let unseen = Prey {
            pos: HIDDEN,
            sight: 1.0,
            concealed: true,
        };
        let mut visited = std::collections::HashSet::new();
        for _ in 0..(600 * 60) {
            enc.threat.cooldown = 99.0;
            enc.tick_world(&t, 1.0 / 60.0);
            enc.update_threat(&l, &t, Some(unseen), &[HIDDEN], 1.0 / 60.0, &mut ev);
            enc.threat.circling = true;
            visited.insert(l.patrol.nearest(enc.threat.pos));
        }
        assert!(
            visited.len() * 2 > l.patrol.len(),
            "roaming covered only {} of {} nodes",
            visited.len(),
            l.patrol.len()
        );
    }

    #[test]
    fn bones_are_taken_dropped_and_delivered_with_ownership_rules() {
        let (l, t, mut enc, mut ev) = setup();
        let n = enc.progress.relics.len();
        assert!(n >= 5);
        assert!(enc.take_relic(0, 7, &mut ev));
        assert!(!enc.take_relic(0, 8, &mut ev), "already carried");
        assert_eq!(enc.progress.carried_by(7), 1);
        assert!(enc.drop_relic(7, Vec3::new(1.0, 0.0, 2.0), &mut ev));
        assert!(!enc.drop_relic(7, Vec3::ZERO, &mut ev));
        assert!(matches!(enc.progress.relics[0], Relic::Ground(p) if (p.y - 0.35).abs() < 1e-6));
        assert!(enc.take_relic(0, 8, &mut ev));
        assert!(!enc.deliver_relic(7, &mut ev), "only the carrier can deliver");
        assert!(enc.deliver_relic(8, &mut ev));
        assert_eq!(enc.progress.delivered(), 1);
        // Capture drops everything the player carried, apart on the ground.
        for i in 1..4 {
            assert!(enc.take_relic(i, 8, &mut ev));
        }
        enc.release_all(8, Vec3::new(4.0, 0.0, 4.0));
        assert_eq!(enc.progress.carried_total(), 0);
        let grounded: Vec<Vec3> = enc
            .progress
            .relics
            .iter()
            .filter_map(|r| if let Relic::Ground(p) = r { Some(*p) } else { None })
            .collect();
        assert!(grounded.len() >= 3);
        // Delivering all of them completes the bones exactly once.
        for i in 1..n {
            enc.progress.relics[i] = Relic::Carried(8);
        }
        while enc.progress.carried_by(8) > 0 {
            enc.deliver_relic(8, &mut ev);
        }
        assert!(enc.progress.bones_home());
        assert_eq!(ev.iter().filter(|e| **e == Event::AllBonesHome).count(), 1);
        let _ = (&l, &t);
    }

    #[test]
    fn the_truck_only_starts_with_the_bones_home_and_the_power_on() {
        let (_, t, mut enc, mut ev) = setup();
        assert!(!enc.work_truck(&t, 1.0, &mut ev), "no power, no bones");
        for _ in 0..(t.pump_hold as usize + 2) {
            enc.work_pump(&t, 1.0, &mut ev);
        }
        assert!(enc.progress.power_on());
        assert_eq!(ev.iter().filter(|e| **e == Event::PowerRestored).count(), 1);
        assert!(!enc.work_truck(&t, 1.0, &mut ev), "bones still missing");
        enc.progress.relics.fill(Relic::Delivered);
        assert!(!enc.work_truck(&t, 1.0, &mut ev), "the key is still locked away");
        let right = lock_code(t.seed);
        let wrong = [(right[0] % 9) + 1, right[1], right[2]];
        assert!(!enc.try_code(&t, wrong, &mut ev));
        assert!(ev.contains(&Event::LockRattle) && !enc.progress.key);
        assert!(enc.try_code(&t, right, &mut ev));
        assert!(ev.contains(&Event::KeyFound) && enc.progress.key);
        assert!(!enc.try_code(&t, right, &mut ev), "it opens once");
        for _ in 0..(t.truck_hold as usize + 2) {
            enc.work_truck(&t, 1.0, &mut ev);
        }
        assert!(enc.progress.truck_running());
        assert_eq!(ev.iter().filter(|e| **e == Event::TruckStarted).count(), 1);
        enc.tick_world(&t, 0.016);
        assert_eq!(enc.pressure, 1.0, "the engine is the finale");
    }

    #[test]
    fn every_bundle_laid_is_a_step_up_on_every_night_and_variant() {
        let l = Layout::new();
        // The first seed of each of his three returns.
        let mut returns: Vec<(Variant, u64)> = Vec::new();
        for seed in 0..60 {
            let v = Variant::of(seed);
            if !returns.iter().any(|&(w, _)| w == v) {
                returns.push((v, seed));
            }
        }
        assert_eq!(returns.len(), 3);
        for night in [
            crate::tuning::Night::Gentle,
            crate::tuning::Night::Normal,
            crate::tuning::Night::Hard,
        ] {
            for &(variant, seed) in &returns {
                let t = Tuning::with_seed(seed).with_night(night);
                let at = format!("{night:?} {variant:?}");
                let mut enc = Encounter::new(&l);
                let mut ev = Vec::new();
                // The night and the load still weigh on him.
                enc.tick_world(&t, 0.0);
                let early = enc.pressure;
                enc.elapsed = t.night_length * 0.8;
                enc.tick_world(&t, 0.0);
                assert!(enc.pressure > early, "{at}: the night");
                let late = enc.pressure;
                assert!(enc.take_relic(0, 1, &mut ev));
                enc.tick_world(&t, 0.0);
                assert!(enc.pressure > late, "{at}: the load");
                // A fall never angers him: the fallen's bundle goes back to the grass.
                enc.release_all(1, Vec3::new(4.0, 0.0, 4.0));
                assert_eq!(enc.progress.rage(), 0, "{at}: nothing unlocks because a player fell");
                // Every bundle laid, from dusk: a step up in pressure and in rage.
                let mut enc = Encounter::new(&l);
                enc.tick_world(&t, 0.0);
                for i in 0..enc.progress.relics.len() {
                    let (before, stage) = (enc.pressure, enc.progress.rage());
                    assert!(enc.take_relic(i, 1, &mut ev));
                    enc.tick_world(&t, 0.0);
                    let carried = enc.pressure;
                    assert!(enc.deliver_relic(1, &mut ev));
                    enc.tick_world(&t, 0.0);
                    assert!(
                        enc.pressure > before,
                        "{at}: bundle {i} laid, {before} -> {}",
                        enc.pressure
                    );
                    assert!(
                        enc.pressure > carried || enc.pressure == t.pressure_max,
                        "{at}: laying bundle {i} lowered pressure ({carried} -> {})",
                        enc.pressure
                    );
                    assert!(enc.pressure <= t.pressure_max);
                    assert_eq!(enc.progress.rage(), (stage + 1).min(crate::tuning::MAX_RAGE), "{at}");
                }
                assert_eq!(enc.progress.rage(), crate::tuning::MAX_RAGE);
            }
        }
    }

    #[test]
    fn a_crouched_player_seven_metres_off_in_grass_is_found_only_once_he_is_angry() {
        let (l, t, _, mut ev) = setup();
        let him = YARD + Vec2::new(7.0, 0.0);
        assert!(l.line_of_sight(him, YARD));
        let crouched = Prey {
            pos: YARD,
            sight: t.sight_crouch,
            concealed: true,
        };
        for stage in 0..=crate::tuning::MAX_RAGE {
            let mut enc = Encounter::new(&l);
            for r in enc.progress.relics.iter_mut().take(stage as usize) {
                *r = Relic::Delivered;
            }
            assert_eq!(enc.progress.rage(), stage);
            place_threat(&mut enc, &l, him);
            enc.threat.cooldown = 99.0;
            enc.tick_world(&t, 1.0 / 60.0);
            enc.update_threat(&l, &t, Some(crouched), &[YARD], 1.0 / 60.0, &mut ev);
            assert_eq!(
                enc.threat.has_sight,
                stage >= 3,
                "stage {stage}: crouched in the grass 7 m from him"
            );
        }
        // Standing in the open he sees them at any stage.
        let mut enc = Encounter::new(&l);
        place_threat(&mut enc, &l, him);
        enc.threat.cooldown = 99.0;
        enc.tick_world(&t, 1.0 / 60.0);
        enc.update_threat(&l, &t, Some(Prey::plain(YARD)), &[YARD], 1.0 / 60.0, &mut ev);
        assert!(enc.threat.has_sight);
    }

    #[test]
    fn every_seed_locks_the_key_with_its_own_three_digits() {
        let codes: std::collections::HashSet<[u8; 3]> = (0..50).map(lock_code).collect();
        assert!(codes.len() > 40, "the combination changes from night to night");
        assert!(codes.iter().all(|c| c[0] >= 1 && c.iter().all(|&d| d <= 9)));
        assert_eq!(lock_code(7), lock_code(7));
    }

    #[test]
    fn every_variant_walks_some_nights_and_only_the_right_name_lays_him_to_rest() {
        let seen: std::collections::HashSet<u8> = (0..60).map(|s| Variant::of(s).code()).collect();
        assert_eq!(seen.len(), 3, "all three returns happen");
        for v in Variant::ALL {
            assert_eq!(Variant::from_code(v.code()), Some(v));
        }
        let (l, t, mut enc, mut ev) = setup();
        let right = Variant::of(t.seed);
        let wrong = Variant::ALL.into_iter().find(|v| *v != right).unwrap();
        let watchers = [Vec2::new(-19.0, -44.0)];
        assert!(enc.name_him(&l, &t, right, &watchers, &mut ev).is_err(), "bones first");
        enc.progress.relics.fill(Relic::Delivered);
        assert_eq!(enc.name_him(&l, &t, wrong, &watchers, &mut ev), Ok(false));
        assert!(ev.contains(&Event::NameWrong));
        assert_ne!(enc.threat.state, ThreatState::Dormant, "a wrong name rouses him");
        assert!(enc.name_him(&l, &t, right, &watchers, &mut ev).is_err(), "not so soon");
        enc.tick_world(&t, t.naming_cooldown + 0.1);
        assert_eq!(enc.name_him(&l, &t, right, &watchers, &mut ev), Ok(true));
        assert!(ev.contains(&Event::Banished) && enc.progress.banished);
        assert_eq!(enc.outcome, Outcome::Won);
    }

    #[test]
    fn beacon_burns_then_cools_and_cannot_be_relit_early() {
        let (_, t, mut enc, mut ev) = setup();
        assert!(enc.light_beacon(&t, &mut ev));
        assert!(!enc.light_beacon(&t, &mut ev));
        enc.tick_world(&t, t.beacon_burn + 0.1);
        assert_eq!(enc.progress.beacon, 0.0);
        assert!(!enc.light_beacon(&t, &mut ev), "still cooling");
        enc.tick_world(&t, t.beacon_cooldown + 0.1);
        assert!(enc.light_beacon(&t, &mut ev));
    }

    #[test]
    fn restart_resets_all_truth_and_timers() {
        let (l, t, mut enc, mut ev) = setup();
        let pristine = enc.clone();
        enc.manifest(&l, &t, &[Vec2::new(0.0, 10.0)], &mut ev);
        enc.take_relic(0, 3, &mut ev);
        enc.place_aji(&t, Vec2::ZERO, &mut ev);
        run_for(&mut enc, &l, &t, YARD, 30.0, &mut ev);
        enc.progress.power = 0.5;
        assert_ne!(enc, pristine);
        enc.reset(&l);
        assert_eq!(enc, pristine);
        assert_eq!(enc.elapsed, 0.0);
        assert_eq!(enc.stats, Stats::default());
        assert!(enc.zones.is_empty() && enc.threat.focus.is_none());
    }

    #[test]
    fn event_codes_round_trip() {
        for (i, e) in Event::ALL.iter().enumerate() {
            assert_eq!(*e as u8 as usize, i);
            assert_eq!(Event::from_code(i as u8), Some(*e));
        }
        assert_eq!(Event::from_code(200), None);
    }
}

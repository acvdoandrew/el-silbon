//! ECS adapter for interaction: crosshair targeting from the latest snapshot
//! (the same rule the host validates with), reading notes, and clearing the
//! views when a run is reset.

use bevy::prelude::*;

use crate::app::{Flow, GameSet, LayoutRes, RunReset, TuningRes};
use crate::control::{Target, TargetKind, evaluate_target};
use crate::geometry::ground;
use crate::net::Network;
use crate::player::{CurrentIntent, Player};

/// What the crosshair is on this frame.
#[derive(Resource, Default)]
pub struct CurrentTarget(pub Option<Target>);

/// The note being read, if any (by note id).
#[derive(Resource, Default)]
pub struct NoteOpen(pub Option<u8>);

/// The key box's padlock panel: open or not, and the dials as this player
/// has set them (a local fiddle; only a tried combination reaches the host).
#[derive(Resource, Default)]
pub struct LockPanel {
    pub open: bool,
    pub dials: [u8; 3],
}

/// Naming him at the ceiba: the panel is open, and which of the three is
/// chosen (a `sim::Variant` code).
#[derive(Resource, Default)]
pub struct NamePanel {
    pub open: bool,
    pub choice: u8,
}

/// Every page this player has read, across restarts: the tale pieced
/// together over several nights.
#[derive(Resource, Default)]
pub struct PagesRead(pub std::collections::BTreeSet<u8>);

pub struct EncounterPlugin;

impl Plugin for EncounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>()
            .init_resource::<NoteOpen>()
            .init_resource::<PagesRead>()
            .init_resource::<LockPanel>()
            .init_resource::<NamePanel>()
            .add_systems(
                Update,
                (
                    reset_views.in_set(GameSet::Control),
                    update_target.in_set(GameSet::Target).run_if(in_state(Flow::Playing)),
                ),
            );
    }
}

fn update_target(
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    player: Single<&Player>,
    net: Res<Network>,
    intent: Res<CurrentIntent>,
    mut target: ResMut<CurrentTarget>,
    mut note: ResMut<NoteOpen>,
    mut read: ResMut<PagesRead>,
    mut lock: ResMut<LockPanel>,
    mut naming: ResMut<NamePanel>,
    keys: Res<ButtonInput<KeyCode>>,
    launch: Res<crate::app::Launch>,
) {
    let t = if net.active() {
        net.snapshot().and_then(|s| {
            let data = s.scene_data(net.id()?, net.stunned());
            evaluate_target(&layout.0, &tuning.0, &player.pose, &data.scene())
        })
    } else {
        None
    };
    target.0 = t;
    if intent.0.interact_pressed
        && let Some(t) = t
        && t.ready()
        && let TargetKind::Note(id) = t.kind
    {
        note.0 = if note.0 == Some(id) { None } else { Some(id) };
        read.0.insert(id);
    }
    if intent.0.interact_pressed
        && let Some(t) = t
        && t.ready()
        && t.kind == TargetKind::Radio
    {
        // The radio's page is heard, not read: turning the dial keeps it.
        let d = &layout.0.district;
        read.0.extend(d.notes.iter().filter(|n| n.pos == d.radio).map(|n| n.id));
    }
    if intent.0.interact_pressed
        && let Some(t) = t
        && t.ready()
        && t.kind == TargetKind::Lockbox
    {
        lock.open = !lock.open;
    }
    // At the ceiba with every bone laid down, N offers to name him.
    let at_altar = t.is_some_and(|t| t.ready() && t.kind == TargetKind::Altar);
    let bones_home = net
        .snapshot()
        .is_some_and(|s| s.world.delivered >= s.world.total && s.world.total > 0);
    if !launch.smoke && !launch.net_smoke && keys.just_pressed(KeyCode::KeyN) && at_altar && bones_home {
        naming.open = !naming.open;
    }
    if naming.open && (!at_altar || !bones_home || net.status() != 0) {
        naming.open = false;
    }
    // Walking away, going down or the box opening closes the padlock.
    let at_box = player.pose.pos.distance(ground(layout.0.district.lockbox)) <= tuning.0.site_reach + 1.0;
    if lock.open && (!at_box || net.status() != 0 || net.snapshot().is_none_or(|s| s.world.key)) {
        lock.open = false;
    }
    // Walking away, or going down, puts the page down.
    if let Some(id) = note.0 {
        let near = layout
            .0
            .district
            .notes
            .iter()
            .find(|n| n.id == id)
            .is_some_and(|n| player.pose.pos.distance(ground(n.pos)) <= tuning.0.note_reach + 1.0);
        if !near || net.status() != 0 {
            note.0 = None;
        }
    }
}

fn reset_views(
    mut requests: MessageReader<RunReset>,
    mut target: ResMut<CurrentTarget>,
    mut note: ResMut<NoteOpen>,
    mut lock: ResMut<LockPanel>,
    mut naming: ResMut<NamePanel>,
) {
    if requests.read().count() > 0 {
        target.0 = None;
        note.0 = None;
        *lock = LockPanel::default();
        *naming = NamePanel::default();
    }
}

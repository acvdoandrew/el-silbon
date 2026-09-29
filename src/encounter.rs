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

pub struct EncounterPlugin;

impl Plugin for EncounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>()
            .init_resource::<NoteOpen>()
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

fn reset_views(mut requests: MessageReader<RunReset>, mut target: ResMut<CurrentTarget>, mut note: ResMut<NoteOpen>) {
    if requests.read().count() > 0 {
        target.0 = None;
        note.0 = None;
    }
}

//! ECS adapter for the truth layer: crosshair targeting, the per-frame truth
//! step, perception → whistle cues, outcome transition, and the satchel views.

use bevy::prelude::*;

use crate::app::{EncounterMsg, Flow, GameSet, LayoutRes, RestartRequest, Truth, TuningRes, WhistleMsg};
use crate::control::{Target, TargetKind, evaluate_target, tick_input};
use crate::geometry::ground;
use crate::player::{CurrentIntent, Player};
use crate::sim::{Event, Objective};
use crate::world::{CarriedSatchel, TableSatchel, TreeSatchel};

/// What the crosshair is on this frame.
#[derive(Resource, Default)]
pub struct CurrentTarget(pub Option<Target>);

/// The note overlay is open.
#[derive(Resource, Default)]
pub struct NoteOpen(pub bool);

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
                    step_truth.in_set(GameSet::Simulate).run_if(in_state(Flow::Playing)),
                    sync_satchels.in_set(GameSet::Present),
                ),
            );
    }
}

fn update_target(
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    truth: Res<Truth>,
    player: Single<&Player>,
    intent: Res<CurrentIntent>,
    mut target: ResMut<CurrentTarget>,
    mut note: ResMut<NoteOpen>,
) {
    let t = evaluate_target(&layout.0, &tuning.0, &player.pose, &truth.encounter);
    target.0 = t;
    if intent.0.interact_pressed && t.is_some_and(|t| t.kind == TargetKind::Note && t.ready()) {
        note.0 = !note.0;
    }
    // Walking away from the table closes the note.
    if note.0 && player.pose.pos.distance(ground(layout.0.note)) > tuning.0.note_reach + 1.0 {
        note.0 = false;
    }
}

fn step_truth(
    time: Res<Time>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    intent: Res<CurrentIntent>,
    target: Res<CurrentTarget>,
    player: Single<&Player>,
    mut truth: ResMut<Truth>,
    mut events: Local<Vec<Event>>,
    mut encounter_out: MessageWriter<EncounterMsg>,
    mut whistle_out: MessageWriter<WhistleMsg>,
    mut next: ResMut<NextState<Flow>>,
) {
    let dt = time.delta_secs();
    let input = tick_input(dt, &player.pose, &intent.0, target.0);
    let Truth { encounter, cue } = &mut *truth;
    events.clear();
    encounter.step(&layout.0, &tuning.0, input, &mut events);
    for e in events.iter() {
        info!("encounter event: {e:?} at {:.1}s", encounter.elapsed);
        encounter_out.write(EncounterMsg(*e));
    }
    if let Some(phrase) = cue.tick(dt.min(tuning.0.max_step), encounter, player.pose.pos, &tuning.0) {
        whistle_out.write(WhistleMsg(phrase));
    }
    if encounter.objective.is_over() {
        next.set(Flow::Outcome);
    }
}

fn reset_views(
    mut requests: MessageReader<RestartRequest>,
    mut target: ResMut<CurrentTarget>,
    mut note: ResMut<NoteOpen>,
) {
    if requests.read().count() > 0 {
        target.0 = None;
        note.0 = false;
    }
}

type SatchelVis<'a> = &'a mut Visibility;

fn sync_satchels(
    truth: Res<Truth>,
    mut table: Query<SatchelVis, (With<TableSatchel>, Without<TreeSatchel>, Without<CarriedSatchel>)>,
    mut tree: Query<SatchelVis, (With<TreeSatchel>, Without<TableSatchel>, Without<CarriedSatchel>)>,
    mut carried: Query<SatchelVis, (With<CarriedSatchel>, Without<TableSatchel>, Without<TreeSatchel>)>,
) {
    let o = truth.encounter.objective;
    let set = |v: &mut Visibility, show: bool| {
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    };
    let returned = truth.encounter.restitution >= 1.0;
    for mut v in &mut table {
        set(&mut v, o == Objective::FindSatchel);
    }
    for mut v in &mut carried {
        set(&mut v, o == Objective::ReturnBones);
    }
    for mut v in &mut tree {
        set(&mut v, returned && matches!(o, Objective::Escape | Objective::Won));
    }
}

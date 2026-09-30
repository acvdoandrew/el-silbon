//! The title screen's night: the camera drifts slowly past the hacienda's
//! landmarks in the rain, fading between them; now and then a whistle, and
//! when lightning strikes, sometimes a tall hatted shape where there was none.
//! Presentation only: no run exists yet. Every stop comes from the layout.

use bevy::prelude::*;

use crate::app::{EncounterMsg, Flow, GameSet, LayoutRes, StormClock, TuningRes, WhistleMsg};
use crate::geometry::Layout;
use crate::geometry::district::LandmarkId;
use crate::perception::{WhistlePhrase, WhistleVariant};
use crate::player::Player;
use crate::sim::Event;

/// Seconds at each stop, and of the fade in and out of it.
pub(crate) const STOP: f32 = 17.0;
const FADE: f32 = 1.6;
/// How far the camera drifts at a stop (metres).
const DRIFT: f32 = 5.0;
/// Seconds between the faint whistles, and between two apparitions.
const WHISTLE_EVERY: (f32, f32) = (22.0, 38.0);
const APPARITION_COOL: f32 = 70.0;

#[derive(Component)]
pub(crate) struct TitleFade;

#[derive(Resource, Default)]
pub(crate) struct TitleNight {
    /// Seconds into the drift (the `--menu-shots` driver sets it).
    pub t: f32,
    whistle_in: f32,
    apparition_cool: f32,
    last_flash: f32,
    turn: u32,
}

/// Where the camera stands and what it looks at, for each stop.
fn stops(layout: &Layout) -> Vec<(Vec3, Vec3, Vec3)> {
    use LandmarkId::*;
    [Ranch, Shrine, Corral, Fields, Cano, Watchtower, Entry]
        .into_iter()
        .map(|id| {
            let m = layout.district.landmark(id);
            let look = m.look;
            let away = (m.approach - Vec2::new(look.x, look.z)).normalize_or(Vec2::Y);
            let at = m.approach + away * 6.0;
            let side = Vec2::new(-away.y, away.x) * DRIFT;
            let lift = layout.surface_height(at) + 3.0;
            let from = Vec3::new(at.x - side.x * 0.5, lift, at.y - side.y * 0.5);
            let to = Vec3::new(at.x + side.x * 0.5, lift + 0.6, at.y + side.y * 0.5);
            (from, to, look)
        })
        .collect()
}

pub(crate) fn spawn_fade(mut commands: Commands) {
    commands.spawn((
        Name::new("title fade"),
        TitleFade,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        GlobalZIndex(15),
        Visibility::Hidden,
    ));
}

#[allow(clippy::too_many_arguments)]
fn drift(
    time: Res<Time<Real>>,
    state: Res<State<Flow>>,
    layout: Res<LayoutRes>,
    tuning: Res<TuningRes>,
    clock: Res<StormClock>,
    menu: Res<super::menu::Menu>,
    mut night: ResMut<TitleNight>,
    mut camera: Single<&mut Transform, With<Player>>,
    fade: Single<(&mut BackgroundColor, &mut Visibility), With<TitleFade>>,
    mut events: MessageWriter<EncounterMsg>,
    mut whistles: MessageWriter<WhistleMsg>,
) {
    let (mut fade_bg, mut fade_vis) = fade.into_inner();
    if *state.get() != Flow::Title {
        if *fade_vis != Visibility::Hidden {
            *fade_vis = Visibility::Hidden;
        }
        night.t = 0.0;
        return;
    }
    let dt = time.delta_secs();
    night.t += dt;
    let stops = stops(&layout.0);
    let total = STOP * stops.len() as f32;
    let t = night.t % total;
    let i = (t / STOP) as usize % stops.len();
    let local = t - i as f32 * STOP;
    let (from, to, look) = stops[i];
    let k = local / STOP;
    let ease = k * k * (3.0 - 2.0 * k);
    let at = from.lerp(to, ease);
    **camera = Transform::from_translation(at).looking_at(look + Vec3::Y * 0.8, Vec3::Y);
    // Fade from black into each stop and back out of it.
    let black = if local < FADE {
        1.0 - local / FADE
    } else if local > STOP - FADE {
        (local - (STOP - FADE)) / FADE
    } else {
        0.0
    };
    let black = if night.t < FADE * 1.5 {
        1.0 - night.t / (FADE * 1.5)
    } else {
        black
    };
    // The calibration's hats must never dim with the stops.
    let black = if menu.calibrating(Flow::Title) { 0.0 } else { black };
    let want = Color::srgba(0.0, 0.0, 0.0, black.clamp(0.0, 1.0));
    if fade_bg.0 != want {
        fade_bg.0 = want;
    }
    if *fade_vis != Visibility::Inherited {
        *fade_vis = Visibility::Inherited;
    }

    // Now and then, a whistle, never loud.
    night.whistle_in -= dt;
    if night.whistle_in <= 0.0 {
        night.turn = night.turn.wrapping_add(1);
        let (lo, hi) = WHISTLE_EVERY;
        let h = (night.turn.wrapping_mul(2654435761) % 1000) as f32 / 1000.0;
        night.whistle_in = lo + (hi - lo) * h;
        if night.t > 6.0 {
            let variant = if night.turn.is_multiple_of(2) {
                WhistleVariant::Faint
            } else {
                WhistleVariant::Middling
            };
            whistles.write(WhistleMsg(WhistlePhrase {
                variant,
                gain: variant.gain(&tuning.0) * 0.7,
                speed: 0.96 + 0.05 * h,
                seeming_closeness: 0.3,
                phantom: false,
                take: (night.turn / 2 % crate::perception::WHISTLE_TAKES as u32) as u8,
            }));
        }
    }
    // A strike, and sometimes a shape at the edge of the light.
    night.apparition_cool = (night.apparition_cool - dt).max(0.0);
    let flash = crate::storm::flash(tuning.0.seed, clock.t);
    if flash > 0.5 && night.last_flash <= 0.5 && night.apparition_cool <= 0.0 && local > FADE && local < STOP - 4.0 {
        night.apparition_cool = APPARITION_COOL;
        events.write(EncounterMsg(Event::OmenPhantom));
    }
    night.last_flash = flash;
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<TitleNight>()
        .add_systems(Startup, spawn_fade)
        .add_systems(Update, drift.in_set(GameSet::Motion));
}

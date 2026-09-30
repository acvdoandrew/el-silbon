//! The launch splash: on a plain launch the cover art fills the screen while
//! the game starts (the title's models and fonts load, its night fades up
//! behind), holds, then fades into the title screen. Any key or click skips
//! it and does nothing else. Never for `--play`, `--host`/`--join` or the
//! debug drivers (`Launch::splash`). Presentation only.

use bevy::asset::LoadState;
use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::window::PrimaryWindow;

use super::Fonts;
use crate::app::{GameSet, Launch};
use crate::world::models::ModelsPending;

/// The cover (`assets/branding/`, shipped by `tools/package.sh`).
const COVER: &str = "branding/whistle-cover.png";
/// Seconds: the cover rising out of black, held, then fading into the title.
const FADE_IN: f32 = 0.5;
const HOLD: f32 = 1.6;
const FADE_OUT: f32 = 0.7;
/// A key or a click (or no cover to show): gone this quickly.
const SKIP_FADE: f32 = 0.25;
/// Wall seconds after which it leaves however unready the title still is.
const LIMIT: f32 = 6.0;
/// The most one frame counts for: a stalled first frame or a shader compile
/// must not eat the fade or the hold.
const MAX_STEP: f32 = 1.0 / 20.0;
/// How much of the art may be cut away on the axis that overflows the screen:
/// plenty of sky and mud above and below, only a sliver at the sides (the
/// painted title starts 4 % in from the left). Beyond that, black bars.
const CROP: Vec2 = Vec2::new(0.05, 0.18);

/// The art's drawn size on `screen`: it covers the screen, cropped evenly from
/// the centre as far as `CROP` allows, then sits in black bars. Never
/// stretched.
pub(crate) fn fit(screen: Vec2, art: Vec2) -> Vec2 {
    if !(screen.min_element() > 0.0 && art.min_element() > 0.0) {
        return art;
    }
    let ratio = screen / art;
    // Covering overflows the axis on which the screen is relatively shorter.
    let crop = if ratio.x > ratio.y { CROP.y } else { CROP.x };
    art * ratio.max_element().min(ratio.min_element() / (1.0 - crop))
}

/// How the splash looks this frame: the art's opacity and the black's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Look {
    pub art: f32,
    pub black: f32,
}

/// The splash's timing: black until the art is in, the art fades up and
/// holds, stays while the title is still getting ready (up to `LIMIT`), then
/// fades out.
#[derive(Default, Debug)]
pub(crate) struct Clock {
    /// Seconds up, each frame counting at most `MAX_STEP`.
    t: f32,
    /// Wall seconds up, for `LIMIT`.
    wall: f32,
    /// When (`t`) the art first showed.
    shown: Option<f32>,
    /// When (`t`) it began to leave, and over how long.
    leaving: Option<(f32, f32)>,
}

impl Clock {
    /// One frame of `dt` seconds. `art`: the cover is loaded; `ready`: the
    /// title behind it is; `leave`: a key or click, or no art to show.
    /// `None` once the splash has gone.
    pub(crate) fn step(&mut self, dt: f32, art: bool, ready: bool, leave: bool) -> Option<Look> {
        self.wall += dt.max(0.0);
        self.t += dt.clamp(0.0, MAX_STEP);
        if art && self.shown.is_none() {
            self.shown = Some(self.t);
        }
        if self.leaving.is_none() {
            let held = self.shown.is_some_and(|s| self.t - s >= FADE_IN + HOLD);
            if leave {
                self.leaving = Some((self.t, SKIP_FADE));
            } else if (held && ready) || self.wall >= LIMIT {
                self.leaving = Some((self.t, FADE_OUT));
            }
        }
        let rise = self.shown.map_or(0.0, |s| ((self.t - s) / FADE_IN).min(1.0));
        let stay = self.leaving.map_or(1.0, |(at, fade)| 1.0 - (self.t - at) / fade);
        (stay > 0.0).then_some(Look {
            art: rise * stay,
            black: stay,
        })
    }
}

/// The splash's black and the art on it.
#[derive(Component)]
struct Splash;
#[derive(Component)]
struct SplashArt;

/// Present while the splash is up.
#[derive(Resource)]
struct Up {
    clock: Clock,
    cover: Handle<Image>,
    /// A key or a click since the last frame.
    skip: bool,
}

fn spawn(mut commands: Commands, launch: Res<Launch>, assets: Res<AssetServer>) {
    if !launch.splash() {
        return;
    }
    let cover: Handle<Image> = assets.load(COVER);
    commands.spawn((
        Name::new("launch splash"),
        Splash,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        // Over the menus (no hover or click reaches them), under the network
        // and debug overlays.
        GlobalZIndex(60),
        FocusPolicy::Block,
        children![(
            SplashArt,
            ImageNode::new(cover.clone()).with_color(Color::WHITE.with_alpha(0.0)),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        )],
    ));
    commands.insert_resource(Up {
        clock: Clock::default(),
        cover,
        skip: false,
    });
}

/// While the cover is up a key or a click only skips it: the title's menus
/// beneath never see the press.
fn swallow(mut up: ResMut<Up>, mut keys: ResMut<ButtonInput<KeyCode>>, mut mouse: ResMut<ButtonInput<MouseButton>>) {
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        up.skip = true;
    }
    keys.clear();
    mouse.clear();
}

#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    time: Res<Time<Real>>,
    assets: Res<AssetServer>,
    images: Res<Assets<Image>>,
    (fonts, loaded): (Res<Fonts>, Res<Assets<Font>>),
    models: Res<ModelsPending>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut up: ResMut<Up>,
    root: Single<(Entity, &mut BackgroundColor), With<Splash>>,
    art: Single<(&mut ImageNode, &mut Node), With<SplashArt>>,
) {
    let cover = images.get(&up.cover);
    let missing = matches!(assets.load_state(&up.cover), LoadState::Failed(_));
    let ready = models.settled()
        && [&fonts.sans, &fonts.serif, &fonts.italic]
            .iter()
            .all(|h| loaded.contains(h.id()));
    let skip = std::mem::take(&mut up.skip);
    let (root, mut black) = root.into_inner();
    let Some(look) = up
        .clock
        .step(time.delta_secs(), cover.is_some(), ready, skip || missing)
    else {
        commands.entity(root).despawn();
        commands.remove_resource::<Up>();
        return;
    };
    black.set_if_neq(BackgroundColor(Color::BLACK.with_alpha(look.black)));
    let (mut image, mut node) = art.into_inner();
    let tint = Color::WHITE.with_alpha(look.art);
    if image.color != tint {
        image.color = tint;
    }
    if let Some(cover) = cover {
        let screen = window.resolution.size();
        let size = fit(screen, cover.size_f32());
        let corner = (screen - size) * 0.5;
        node.set_if_neq(Node {
            position_type: PositionType::Absolute,
            left: px(corner.x),
            top: px(corner.y),
            width: px(size.x),
            height: px(size.y),
            ..default()
        });
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(PreUpdate, swallow.after(InputSystems).run_if(resource_exists::<Up>))
        .add_systems(Update, show.in_set(GameSet::Present).run_if(resource_exists::<Up>));
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// Runs the clock at 60 fps, `input(t)` giving (art, ready, leave), until
    /// the splash has gone; returns when that was.
    fn gone_at(clock: &mut Clock, mut input: impl FnMut(f32) -> (bool, bool, bool)) -> f32 {
        let mut t = 0.0;
        while t < 30.0 {
            t += FRAME;
            let (art, ready, leave) = input(t);
            if clock.step(FRAME, art, ready, leave).is_none() {
                return t;
            }
        }
        panic!("the splash never left");
    }

    #[test]
    fn a_quick_start_shows_the_cover_for_about_three_seconds() {
        let mut clock = Clock::default();
        let mut looks = Vec::new();
        let gone = gone_at(&mut clock, |_| (true, true, false));
        assert!((2.5..=3.1).contains(&gone), "gone at {gone}");
        let mut clock = Clock::default();
        let mut t = 0.0;
        while let Some(look) = clock.step(FRAME, true, true, false) {
            t += FRAME;
            looks.push((t, look));
        }
        let at = |s: f32| looks.iter().find(|(t, _)| *t >= s).map(|(_, l)| *l).unwrap();
        assert_eq!(at(1.0), Look { art: 1.0, black: 1.0 }, "the cover, full, on black");
        assert!(at(0.2).art < 1.0, "it rises out of black");
        assert!(at(gone - 0.2).black < 1.0, "it fades into the title");
    }

    #[test]
    fn it_waits_for_the_title_but_not_forever() {
        let ready_at = 4.0;
        let gone = gone_at(&mut Clock::default(), |t| (true, t >= ready_at, false));
        assert!(
            gone > ready_at && gone <= ready_at + FADE_OUT + 3.0 * FRAME,
            "gone at {gone}"
        );
        let gone = gone_at(&mut Clock::default(), |_| (true, false, false));
        assert!(
            gone > LIMIT && gone <= LIMIT + FADE_OUT + 3.0 * FRAME,
            "never ready: gone at {gone}"
        );
    }

    #[test]
    fn a_late_cover_still_gets_its_hold() {
        let arrives = 1.0;
        let mut clock = Clock::default();
        assert_eq!(
            clock.step(FRAME, false, true, false),
            Some(Look { art: 0.0, black: 1.0 }),
            "black while the art loads"
        );
        let gone = gone_at(&mut clock, |t| (t >= arrives, true, false));
        assert!(
            gone >= arrives + FADE_IN + HOLD + FADE_OUT - 3.0 * FRAME,
            "gone at {gone}"
        );
    }

    #[test]
    fn a_key_or_click_or_a_missing_cover_ends_it_at_once() {
        let pressed = 1.0;
        let gone = gone_at(&mut Clock::default(), |t| (true, false, t >= pressed));
        assert!(gone <= pressed + SKIP_FADE + 3.0 * FRAME, "skipped: gone at {gone}");
        let gone = gone_at(&mut Clock::default(), |_| (false, true, true));
        assert!(gone <= SKIP_FADE + 3.0 * FRAME, "no art: gone at {gone}");
    }

    #[test]
    fn a_stalled_frame_does_not_eat_the_hold() {
        let stall = 1.5;
        let mut clock = Clock::default();
        let first = clock.step(stall, true, true, false).expect("still up after the stall");
        assert!(first.art < 0.5, "the fade has barely begun");
        let gone = stall + gone_at(&mut clock, |_| (true, true, false));
        assert!(gone >= stall + FADE_IN + HOLD + FADE_OUT - MAX_STEP, "gone at {gone}");
    }

    /// The cover's own size, from its PNG header.
    fn art() -> Vec2 {
        let png = include_bytes!("../../assets/branding/whistle-cover.png");
        let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as f32;
        Vec2::new(be(16), be(20))
    }

    #[test]
    fn the_cover_is_never_stretched_and_its_title_never_cut() {
        // The painted title's box on the art, as fractions of its size.
        let title = (Vec2::new(0.035, 0.125), Vec2::new(0.49, 0.395));
        let art = art();
        for (w, h) in [
            (1920, 1080),
            (1280, 720),
            (1920, 1200),
            (1536, 1024),
            (2560, 1080),
            (3440, 1440),
            (3840, 1080),
            (1024, 768),
            (1280, 1024),
            (900, 1600),
        ] {
            let screen = Vec2::new(w as f32, h as f32);
            let drawn = fit(screen, art);
            assert!(
                (drawn.x / drawn.y - art.x / art.y).abs() < 1e-3,
                "{screen}: aspect kept"
            );
            // The centred part of the art that is on screen.
            let seen = (screen / drawn).min(Vec2::ONE);
            let (lo, hi) = ((Vec2::ONE - seen) * 0.5, (Vec2::ONE + seen) * 0.5);
            assert!(
                lo.cmple(title.0).all() && hi.cmpge(title.1).all(),
                "{screen}: title cut, seen {lo}..{hi}"
            );
        }
    }

    #[test]
    fn common_screens_are_filled_edge_to_edge() {
        let art = art();
        for (w, h) in [(1920, 1080), (1600, 900), (2560, 1440), (1920, 1200), (1536, 1024)] {
            let screen = Vec2::new(w as f32, h as f32);
            let drawn = fit(screen, art);
            assert!(drawn.cmpge(screen - 0.5).all(), "{screen}: bars, drawn {drawn}");
        }
        // Wider or squarer screens get black bars rather than lose the title.
        assert!(fit(Vec2::new(3440.0, 1440.0), art).x < 3440.0);
        assert!(fit(Vec2::new(1024.0, 768.0), art).y < 768.0);
        // No screen yet: the art as it is.
        assert_eq!(fit(Vec2::ZERO, art), art);
    }
}

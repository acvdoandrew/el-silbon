//! El Silbón — The Return: first local playable encounter.
//!
//! Pure, headless modules (no ECS, fully unit-tested):
//! - [`tuning`]: every gameplay number.
//! - [`geometry`]: the authored layout, collision, line of sight, aiming.
//! - [`sim`]: encounter truth — objectives and the Silbón's hidden state.
//! - [`perception`]: truth → what a listener hears (inverted whistle).
//! - [`control`]: intent, first-person kinematics, crosshair targeting.
//! - [`body`]: gait, stamina, fear, downed state.
//! - [`storm`]: rain, lightning and thunder as functions of the run clock.
//! - [`skill`]: skill checks while working a long task.
//! - [`director`]: when the night sends one player an omen.
//! - [`script`], [`photos`]: DEBUG smoke route and photo viewpoints.
//!
//! Bevy adapters and presentation: [`app`], [`player`], [`encounter`],
//! [`audio`], [`ui`], [`world`], [`debug`].

// Bevy systems and procedural builders legitimately take many parameters
// and wide query types.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod app;
pub mod audio;
pub mod body;
pub mod control;
pub mod debug;
pub mod director;
pub mod encounter;
pub mod geometry;
pub mod lore;
pub mod net;
pub mod noise;
pub mod perception;
pub mod photos;
pub mod player;
pub mod profile;
pub mod rng;
pub mod script;
pub mod sim;
pub mod skill;
pub mod storm;
pub mod tuning;
pub mod ui;
pub mod world;

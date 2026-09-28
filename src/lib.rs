//! El Silbón — The Return: first local playable encounter.
//!
//! Pure, headless modules (no ECS, fully unit-tested):
//! - [`tuning`]: every gameplay number.
//! - [`geometry`]: the authored layout, collision, line of sight, aiming.
//! - [`sim`]: encounter truth — objectives and the Silbón's hidden state.
//! - [`perception`]: truth → what a listener hears (inverted whistle).
//! - [`control`]: intent, first-person kinematics, crosshair targeting.
//! - [`script`]: DEBUG deterministic smoke route.
//!
//! Bevy adapters and presentation: [`app`], [`player`], [`encounter`],
//! [`audio`], [`ui`], [`world`], [`debug`].

// Bevy systems and procedural builders legitimately take many parameters
// and wide query types.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod app;
pub mod audio;
pub mod control;
pub mod debug;
pub mod encounter;
pub mod geometry;
pub mod net;
pub mod perception;
pub mod player;
pub mod rng;
pub mod script;
pub mod sim;
pub mod tuning;
pub mod ui;
pub mod world;

//! The llano's other animals (`District::fauna`): a horse among the herd,
//! chigüires on the caño's bank, a baba in the water, egrets. Scenery only:
//! each is an empty anchor the model hangs from (`models.rs`), with the
//! herd's own idle life on its Neck, Head and Tail, scaled to the animal.
//! Nothing here touches the rules.

use bevy::prelude::*;

use super::SpawnCtx;
use super::herd::{CowPart, cow_pose};
use crate::geometry::district::{FaunaKind, WATER_LEVEL};

#[derive(Component)]
pub struct Fauna {
    pub kind: FaunaKind,
    phase: f32,
    rest: Vec3,
    yaw: f32,
}

impl FaunaKind {
    /// How much of the herd's grazing and looking round this animal does.
    fn liveliness(self) -> f32 {
        match self {
            FaunaKind::Horse => 1.0,
            FaunaKind::Capybara => 0.45,
            FaunaKind::Egret => 0.7,
            FaunaKind::Caiman => 0.12,
        }
    }

    /// The breath that lifts the body (a baba lies still in the water).
    fn breath(self) -> f32 {
        match self {
            FaunaKind::Caiman => 0.0,
            FaunaKind::Egret => 0.003,
            _ => 0.006,
        }
    }
}

pub fn spawn(ctx: &mut SpawnCtx) {
    let layout = ctx.layout;
    let mut rng = ctx.rng(0xFA);
    for f in &layout.district.fauna {
        let ground = layout.surface_height(f.at);
        // A baba lies in the channel with its back and eyes awash.
        let y = match f.kind {
            FaunaKind::Caiman => (WATER_LEVEL - 0.34).max(ground),
            _ => ground,
        };
        let rest = Vec3::new(f.at.x, y, f.at.y);
        ctx.commands.spawn((
            Name::new(format!("fauna {:?}", f.kind)),
            Fauna {
                kind: f.kind,
                phase: rng.range(0.0, std::f32::consts::TAU),
                rest,
                yaw: f.yaw,
            },
            Transform::from_translation(rest).with_rotation(Quat::from_rotation_y(f.yaw)),
            Visibility::Inherited,
        ));
    }
}

pub fn animate(
    time: Res<Time>,
    mut animals: Query<(Entity, &Fauna, &mut Transform)>,
    tree: Query<&Children>,
    mut parts: Query<(&CowPart, &mut Transform), Without<Fauna>>,
) {
    let t = time.elapsed_secs();
    for (entity, f, mut tf) in &mut animals {
        let k = f.kind.liveliness();
        for e in tree.iter_descendants(entity) {
            if let Ok((part, mut ptf)) = parts.get_mut(e) {
                let want = Quat::IDENTITY.slerp(cow_pose(*part, t, f.phase, false), k);
                if ptf.rotation != want {
                    ptf.rotation = want;
                }
            }
        }
        let breath = (t * 1.3 + f.phase).sin() * f.kind.breath();
        tf.translation = f.rest + Vec3::Y * breath.abs();
        tf.rotation = Quat::from_rotation_y(f.yaw + (t * 0.4 + f.phase).sin() * 0.03 * k);
    }
}

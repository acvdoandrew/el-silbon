//! The Blender-built glTF models (`assets/models/`, see `assets/SOURCES.md`)
//! over the procedural stand-ins. Each is spawned as a scene under the entity
//! it replaces; when the scene is ready the procedural parts go, and the
//! game's own markers move onto the model's named nodes, so the existing
//! animation (his walk, the catch, the dog's trot) drives the real rig.
//!
//! If a model cannot load (a clone without Git LFS gets pointer files), the
//! procedural one simply stays: the game is never left with nothing.

use bevy::asset::LoadState;
use bevy::gltf::GltfMaterialName;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady};

use super::CarriedSatchel;
use super::dog::{DogHead, DogJaw, DogLeg, DogRoot, DogTail};
use super::dynamic::{BundleView, RadioSpot};
use super::herd::Cow;
use super::silbon::{Joint, JointKind, SilbonBody, SilbonRoot};
use super::vehicles::{TruckAssets, TruckRoot};

/// Which model a scene child carries.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Silbon,
    Tureco,
    Cattle,
    Bundle,
    Radio,
    Truck,
}

impl Model {
    fn path(self, index: usize) -> &'static str {
        match self {
            Model::Silbon => "models/silbon.glb",
            Model::Tureco => "models/tureco.glb",
            // One bull, one calf, the rest cows.
            Model::Cattle => match index {
                0 => "models/bull.glb",
                4 => "models/calf.glb",
                _ => "models/cow.glb",
            },
            Model::Bundle => "models/bone_bundle.glb",
            Model::Radio => "models/radio.glb",
            Model::Truck => "models/truck.glb",
        }
    }

    /// How the model sits in its owner's space. The game builds cattle and
    /// the radio facing +X and the models face -Z; the truck model is a
    /// little longer than its site.
    fn placement(self) -> Transform {
        let quarter = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        match self {
            Model::Cattle | Model::Radio => Transform::from_rotation(quarter),
            Model::Truck => Transform::from_scale(Vec3::splat(TRUCK_SCALE)),
            Model::Bundle => Transform::from_scale(Vec3::splat(BUNDLE_SCALE)),
            Model::Silbon | Model::Tureco => Transform::IDENTITY,
        }
    }
}

/// The truck model is 5.78 m long; its site is 5.4 m.
const TRUCK_SCALE: f32 = 0.94;
/// The bundle model stands 0.59 m; the procedural sack about 0.45 m.
const BUNDLE_SCALE: f32 = 0.8;

/// Models still loading (the automated drivers wait for none).
#[derive(Resource, Default)]
pub struct ModelsPending(pub Vec<(Entity, Handle<WorldAsset>)>);

impl ModelsPending {
    pub fn settled(&self) -> bool {
        self.0.is_empty()
    }
}

/// A procedural part that a model replaces once it is ready.
#[derive(Component)]
pub struct Procedural;

/// Put a scene child under every entity a model replaces.
#[allow(clippy::too_many_arguments)]
pub fn attach(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut pending: ResMut<ModelsPending>,
    silbon: Query<Entity, With<SilbonRoot>>,
    dogs: Query<Entity, With<DogRoot>>,
    cows: Query<Entity, With<Cow>>,
    bundles: Query<Entity, Or<(With<BundleView>, With<CarriedSatchel>)>>,
    radios: Query<(Entity, &RadioSpot)>,
    trucks: Query<Entity, With<TruckRoot>>,
) {
    let mut add = |owner: Entity, model: Model, index: usize, placement: Option<Transform>| {
        let handle: Handle<WorldAsset> = assets.load(GltfAssetLabel::Scene(0).from_asset(model.path(index)));
        let child = commands
            .spawn((
                Name::new(format!("model {model:?}")),
                model,
                WorldAssetRoot(handle.clone()),
                placement.unwrap_or_else(|| model.placement()),
                Visibility::Inherited,
                ChildOf(owner),
            ))
            .id();
        pending.0.push((child, handle));
    };
    for e in &silbon {
        add(e, Model::Silbon, 0, None);
    }
    for e in &dogs {
        add(e, Model::Tureco, 0, None);
    }
    for (i, e) in cows.iter().enumerate() {
        add(e, Model::Cattle, i, None);
    }
    for e in &bundles {
        add(e, Model::Bundle, 0, None);
    }
    for (e, spot) in &radios {
        add(e, Model::Radio, 0, Some(spot.0));
    }
    for e in &trucks {
        add(e, Model::Truck, 0, None);
    }
}

/// A model that cannot load leaves its procedural stand-in in place.
pub fn watch_failures(assets: Res<AssetServer>, mut pending: ResMut<ModelsPending>) {
    pending.0.retain(|(_, handle)| {
        let failed = matches!(assets.load_state(handle), LoadState::Failed(_));
        if failed {
            warn!(
                "a model could not load (is Git LFS installed?); keeping the procedural one: {:?}",
                handle.path()
            );
        }
        !failed
    });
}

/// A model's scene is in: hide the stand-in and hand the markers over.
#[allow(clippy::too_many_arguments)]
pub fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut pending: ResMut<ModelsPending>,
    models: Query<(&Model, &ChildOf)>,
    children: Query<&Children>,
    names: Query<(&Name, &Transform)>,
    procedural: Query<
        (),
        (
            Or<(
                With<Mesh3d>,
                With<SilbonBody>,
                With<DogHead>,
                With<DogLeg>,
                With<Procedural>,
            )>,
        ),
    >,
    lamps: Query<&GltfMaterialName>,
    truck: Option<Res<TruckAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scene = event.entity;
    pending.0.retain(|(e, _)| *e != scene);
    let Ok((&model, parent)) = models.get(scene) else {
        return;
    };
    let owner = parent.parent();
    // The stand-in: the owner's own mesh and its procedural children (never
    // the scene itself).
    commands
        .entity(owner)
        .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
    for child in children.get(owner).into_iter().flatten() {
        if *child != scene && procedural.contains(*child) {
            commands.entity(*child).despawn();
        }
    }
    let nodes: Vec<Entity> = children.iter_descendants(scene).collect();
    let named = |want: &str| {
        nodes
            .iter()
            .copied()
            .find(|e| names.get(*e).is_ok_and(|(n, _)| n.as_str() == want))
    };
    match model {
        Model::Silbon => {
            if let Some(body) = named("Body") {
                commands.entity(body).insert(SilbonBody);
            }
            for (name, kind) in [
                ("HipL", JointKind::HipL),
                ("HipR", JointKind::HipR),
                ("KneeL", JointKind::KneeL),
                ("KneeR", JointKind::KneeR),
                ("Torso", JointKind::Torso),
                ("Head", JointKind::Head),
                ("ShoulderL", JointKind::ShoulderL),
                ("ElbowL", JointKind::ElbowL),
                ("ShoulderR", JointKind::ShoulderR),
                ("ElbowR", JointKind::ElbowR),
                ("Sack", JointKind::Sack),
                ("Coat", JointKind::Coat),
            ] {
                if let Some(e) = named(name)
                    && let Ok((_, tf)) = names.get(e)
                {
                    commands.entity(e).insert(Joint {
                        kind,
                        rest: tf.translation,
                    });
                }
            }
        }
        Model::Tureco => {
            if let Some(e) = named("Head") {
                commands.entity(e).insert(DogHead);
            }
            if let Some(e) = named("Jaw") {
                commands.entity(e).insert(DogJaw);
            }
            if let Some(e) = named("Tail") {
                commands.entity(e).insert(DogTail);
            }
            // The gait pairs diagonals: front-left with back-right.
            for (name, phase) in [
                ("LegFL", 0.0),
                ("LegBR", 0.0),
                ("LegFR", std::f32::consts::PI),
                ("LegBL", std::f32::consts::PI),
            ] {
                if let Some(e) = named(name) {
                    commands.entity(e).insert(DogLeg(phase));
                }
            }
        }
        Model::Truck => {
            // The lenses light only while the engine runs: the game's own
            // lamp materials, dark until then.
            let Some(truck) = truck else { return };
            let amber = materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.3, 0.05),
                perceptual_roughness: 0.2,
                ..default()
            });
            for e in &nodes {
                let Ok(name) = lamps.get(*e) else { continue };
                let material = match name.0.as_str() {
                    "truck_lamp_head" => truck.headlamp.clone(),
                    "truck_lamp_tail" => truck.taillamp.clone(),
                    "truck_lamp_amber" => amber.clone(),
                    _ => continue,
                };
                commands.entity(*e).insert(MeshMaterial3d(material));
            }
        }
        Model::Cattle | Model::Bundle | Model::Radio => {}
    }
}

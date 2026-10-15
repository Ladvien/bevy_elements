//! The examples' ground plane in the crate's own [`WeatheredMaterial`]: per-pixel cloud shadows,
//! wet darkening and gloss when it rains, rain streaks, and up-facing snow when it snows. A
//! consumer's own ground does the same (or, with a `Terrain` simulating the surface and the
//! cover maps' snow wanted, `ExtendedMaterial<StandardMaterial, GroundExtension>`).
//!
//! [`test_scene`](super::test_scene) spawns the plane with a plain `StandardMaterial` — one
//! material asset type per example — and this plugin swaps it for the weathered one on the first
//! frame.

use bevy::prelude::*;
use bevy_elements::{WeatheredExtension, WeatheredMaterial};

/// Marks the plane [`test_scene`](super::test_scene) spawns; the harness gives it the weathered
/// material.
#[derive(Component)]
pub struct GroundPlane;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, adopt_ground);
}

fn adopt_ground(
    mut commands: Commands,
    standard: Res<Assets<StandardMaterial>>,
    mut grounds: ResMut<Assets<WeatheredMaterial>>,
    planes: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<GroundPlane>>,
) {
    for (entity, material) in &planes {
        let Some(base) = standard.get(&material.0) else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<(GroundPlane, MeshMaterial3d<StandardMaterial>)>()
            .insert(MeshMaterial3d(grounds.add(WeatheredMaterial {
                base: base.clone(),
                extension: WeatheredExtension::default(),
            })));
    }
}

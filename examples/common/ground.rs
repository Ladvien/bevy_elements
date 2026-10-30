//! The examples' ground plane with per-pixel cloud shadows: a `StandardMaterial` extended to sample
//! [`CloudShadows`]. A consumer's own materials do the same with the mapping in `ground.wesl`.

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};
use bevy_elements::CloudShadows;

/// Marks the plane [`test_scene`](super::test_scene) spawns; the harness gives it the shadowed
/// material.
#[derive(Component)]
pub struct GroundPlane;

#[derive(Clone, Copy, Debug, Default, ShaderType, Reflect)]
pub struct CloudShadowParams {
    map: Vec4,
    light: Vec4,
}

/// The extension: the shadow map and where it sits.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct CloudShadowExtension {
    #[uniform(100)]
    params: CloudShadowParams,
    #[texture(101)]
    #[sampler(102)]
    map: Handle<Image>,
}

impl MaterialExtension for CloudShadowExtension {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(GROUND_SHADER)
    }
}

const GROUND_SHADER: Handle<Shader> = uuid_handle!("6d5c1f0e-6f3a-4d7c-9a3a-3a0c2e0b7f11");

type ShadowedGround = ExtendedMaterial<StandardMaterial, CloudShadowExtension>;

pub fn plugin(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ShadowedGround>::default());
    load_internal_asset!(app, GROUND_SHADER, "ground.wesl", Shader::from_wesl);
    app.add_systems(Update, (adopt_ground, update_params));
}

fn adopt_ground(
    mut commands: Commands,
    shadows: Option<Res<CloudShadows>>,
    standard: Res<Assets<StandardMaterial>>,
    mut grounds: ResMut<Assets<ShadowedGround>>,
    planes: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<GroundPlane>>,
) {
    let Some(shadows) = shadows else {
        return;
    };
    for (entity, material) in &planes {
        let Some(base) = standard.get(&material.0) else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<(GroundPlane, MeshMaterial3d<StandardMaterial>)>()
            .insert(MeshMaterial3d(grounds.add(ExtendedMaterial {
                base: base.clone(),
                extension: CloudShadowExtension {
                    params: CloudShadowParams::default(),
                    map: shadows.map.clone(),
                },
            })));
    }
}

fn update_params(shadows: Option<Res<CloudShadows>>, mut grounds: ResMut<Assets<ShadowedGround>>) {
    let Some(shadows) = shadows else {
        return;
    };
    let params = CloudShadowParams {
        map: Vec4::new(
            shadows.origin.x,
            shadows.origin.y,
            shadows.extent,
            shadows.camera_transmittance,
        ),
        light: shadows.light_direction.extend(0.0),
    };
    for (_, material) in grounds.iter_mut() {
        material.extension.params = params;
    }
}

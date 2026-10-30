//! Heat haze: a campfire, a torch on a post and a bonfire in the hills, each a [`HeatSource`]
//! whose plume bends the landscape and sky behind it.
//!
//! `cargo run --release --example heat_haze`
//!
//! The flames are stand-ins (additive emissive cones sized with [`HeatSource::flame_height_m`]
//! and a point light); a game draws its own and puts a `HeatSource` on them. A wooden post stands
//! between the camera and the campfire: the haze bends what is behind the plume, never the post.
//! `ELEMENTS_HEAT_GAIN` sets [`HeatHazeSettings::gain`] (default 10; 0 switches the haze off),
//! `ELEMENTS_HOUR` holds an hour (default 13), `ELEMENTS_PRESET` picks the weather (default
//! `Clear`), `ELEMENTS_WIND=6` holds the wind at that many m/s at 10 m, blowing toward +x.
//! `ELEMENTS_EYE=x,y,z` places the camera `y` metres above the ground at `x,z` (default `0,1.7,7`)
//! and `ELEMENTS_LOOK=x,y,z` aims it at a point `y` metres above the ground (default
//! `0,1.6,-20`). `ELEMENTS_SEED` picks the hills (default 42). `ELEMENTS_FRAMES`, `ELEMENTS_SHOT`,
//! `ELEMENTS_SHOT_DIR` and `ELEMENTS_OFFSCREEN` work as in the other examples.

mod common;

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, units};

/// The fires: where they stand (world XZ, m), how high above the ground their base is, m, and
/// what they are.
const FIRES: [(Vec2, f32, HeatSource); 3] = [
    // A campfire.
    (
        Vec2::new(0.0, 0.0),
        0.0,
        HeatSource {
            power_kw: 50.0,
            radius_m: 0.3,
        },
    ),
    // A torch on a post.
    (
        Vec2::new(2.6, -1.2),
        1.7,
        HeatSource {
            power_kw: 3.0,
            radius_m: 0.05,
        },
    ),
    // A bonfire down the valley.
    (
        Vec2::new(-22.0, -45.0),
        0.0,
        HeatSource {
            power_kw: 500.0,
            radius_m: 0.75,
        },
    ),
];

/// A post between the camera and the campfire, world XZ, m.
const POST: Vec2 = Vec2::new(0.45, 3.2);

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_vec3(name: &str, default: Vec3) -> Vec3 {
    std::env::var(name)
        .ok()
        .map(|v| {
            v.split(',')
                .filter_map(|s| s.trim().parse::<f32>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() == 3)
        .map_or(default, |v| Vec3::new(v[0], v[1], v[2]))
}

#[derive(Resource)]
struct Scene {
    preset: WeatherPreset,
    wind: Option<f32>,
    eye: Vec3,
    look: Vec3,
    seed: u64,
}

/// Something that stands `height` metres above the terrain at its XZ once the heights are read.
#[derive(Component)]
struct OnGround {
    height: f32,
}

/// The camera's aim, re-applied once the ground is known.
#[derive(Component)]
struct Aim;

fn main() {
    let preset = std::env::var("ELEMENTS_PRESET")
        .ok()
        .and_then(|name| {
            WeatherPreset::ALL
                .into_iter()
                .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
        })
        .unwrap_or(WeatherPreset::Clear);
    let clock = WorldClock {
        paused: true,
        ..WorldClock::new(CivilDate::new(2026, 7, 10))
            .at_hour(f64::from(env_f32("ELEMENTS_HOUR", 13.0)))
    };
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(clock)
        .insert_resource(Climate::temperate())
        .insert_resource(HeatHazeSettings {
            gain: env_f32("ELEMENTS_HEAT_GAIN", 10.0),
        })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .insert_resource(Scene {
            preset,
            wind: std::env::var("ELEMENTS_WIND")
                .ok()
                .and_then(|v| v.parse().ok()),
            eye: env_vec3("ELEMENTS_EYE", Vec3::new(0.0, 1.7, 7.0)),
            look: env_vec3("ELEMENTS_LOOK", Vec3::new(0.0, 1.6, -20.0)),
            seed: env_f32("ELEMENTS_SEED", 42.0) as u64,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            pin_wind
                .after(ElementsSystems::Weather)
                .before(ElementsSystems::Simulation),
        )
        .add_systems(Update, settle.after(ElementsSystems::Readback))
        .run();
}

fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut director: ResMut<WeatherDirector>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Terrain::procedural(ProceduralTerrain::hills(scene.seed)));
    director.force(scene.preset, Duration::ZERO);
    commands.spawn((
        camera(scene.eye, scene.look),
        OnGround {
            height: scene.eye.y,
        },
        Aim,
    ));

    let u = units();
    let flame = materials.add(StandardMaterial {
        base_color: Color::linear_rgba(1.0, 0.45, 0.12, 0.85),
        emissive: LinearRgba::rgb(60.0, 22.0, 5.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.22, 0.14),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (xz, lift, source) in FIRES {
        let height = source.flame_height_m().max(2.5 * source.radius_m);
        commands
            .spawn((
                source,
                Transform::from_xyz(xz.x * u, 0.0, xz.y * u),
                Visibility::default(),
                OnGround { height: lift },
            ))
            .with_children(|fire| {
                fire.spawn((
                    Mesh3d(meshes.add(Cone::new(source.radius_m * u, height * u))),
                    MeshMaterial3d(flame.clone()),
                    Transform::from_xyz(0.0, 0.5 * height * u, 0.0),
                ));
                fire.spawn((
                    PointLight {
                        color: Color::srgb(1.0, 0.6, 0.3),
                        intensity: source.power_kw * 2000.0 * u * u,
                        range: 30.0 * u,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.6 * height * u, 0.0),
                ));
            });
        if lift > 0.0 {
            // The torch's post.
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.05 * u, lift * u))),
                MeshMaterial3d(wood.clone()),
                Transform::from_xyz(xz.x * u, 0.0, xz.y * u),
                OnGround { height: 0.5 * lift },
            ));
        }
    }
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.07 * u, 2.4 * u))),
        MeshMaterial3d(wood),
        Transform::from_xyz(POST.x * u, 0.0, POST.y * u),
        OnGround { height: 1.2 },
    ));
}

/// Hold the wind, if asked.
fn pin_wind(scene: Res<Scene>, mut weather: ResMut<Weather>) {
    if let Some(speed) = scene.wind {
        weather.wind_speed_ms = speed;
        weather.wind_direction = Vec2::X;
    }
}

/// Stand the fires, the posts and the camera on the ground once its heights are read back, and
/// aim the camera.
fn settle(
    mut commands: Commands,
    env: Environment,
    scene: Res<Scene>,
    mut placed: Query<(Entity, &mut Transform, &OnGround, Has<Aim>)>,
) {
    let u = units();
    for (entity, mut transform, on, aim) in &mut placed {
        let xz = transform.translation.xz();
        let Some(ground) = env.terrain_height(xz) else {
            continue;
        };
        transform.translation.y = ground + on.height * u;
        if aim {
            let look_xz = scene.look.xz() * u;
            if let Some(look_ground) = env.terrain_height(look_xz) {
                let target = Vec3::new(look_xz.x, look_ground + scene.look.y * u, look_xz.y);
                transform.look_at(target, Vec3::Y);
            } else {
                continue;
            }
        }
        commands.entity(entity).remove::<OnGround>();
    }
}

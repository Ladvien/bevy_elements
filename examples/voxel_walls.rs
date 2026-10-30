//! Snow on many voxel walls over a game's own ground. Three crenellated walls, each its own
//! `bevy_isomesh` volume with `VoxelSnow`, stand on a plain mesh in `WeatheredMaterial`; the
//! crate spawns no terrain. One wall is turned 30° and raised on a plinth. As the snow falls,
//! each wall grows a slab on its walk and merlons (remeshed when the depth moves by half a voxel),
//! and the weathered material whitens what faces up.
//!
//! `cargo run --release --features isomesh --example voxel_walls`
//!
//! Snow is forced at 3 600 game seconds per real second (`ELEMENTS_TIME_SCALE` changes it).
//! `ELEMENTS_SNOW_FRAMES` stops the snow after that many frames and clears the sky;
//! `ELEMENTS_EYE` / `ELEMENTS_LOOK` (`x,y,z`) place the camera and `ELEMENTS_HOUR` sets the start
//! hour (default 10). With `ELEMENTS_FRAMES` the frames run at a fixed 1/60 s. The console
//! prints the point snowpack once a second.

mod common;

use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;
use bevy_isomesh::{ChunkMesh, IsomeshPlugin, MeshBudget, NeedsRemesh, VoxelChunk, VoxelVolume};
use common::{ExampleHarness, camera};
use isomesh::Sdf;
use isomesh::chunk::{ChunkId, ChunkLayout};

/// Cells of a chunk edge.
const CHUNK_CELLS: u32 = 16;
/// Voxel edge, m (a power of two: isomesh chunk seams close only then). A quarter metre keeps
/// the 1 m merlons square.
const CELL_SIZE: f32 = 0.25;
/// A wall volume is 4 × 3 × 4 chunks of 4 m: 16 m square, from 4 m under the ground to 8 m up.
const CHUNKS_XZ: i32 = 4;
const CHUNKS_Y: i32 = 3;
const Y_MIN: f32 = -4.0;
/// The wall: 14 m long along x, 2 m thick, a 5 m walk with 1 m merlons 1 m wide every 2 m. Its
/// body reaches 2 m under the ground, so the ground plane cuts it rather than its base.
const LENGTH: f32 = 14.0;
const THICK: f32 = 2.0;
const WALK: f32 = 5.0;
const BURIED: f32 = 2.0;
const MERLON: f32 = 1.0;

/// Signed distance to an axis-aligned box of half extents `h` centred at `c`.
fn cuboid(p: Vec3, c: Vec3, h: Vec3) -> f32 {
    let q = (p - c).abs() - h;
    q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
}

/// A crenellated wall in its volume's field space (x and z in [0, 16), y up from 0): the walk,
/// and merlons along both edges of it.
#[derive(Clone, Copy)]
struct Wall;

impl Sdf for Wall {
    type Scalar = f32;
    fn sample(&self, p: [f32; 3]) -> f32 {
        let p = Vec3::from_array(p);
        let mid = Vec3::new(8.0, 0.0, 8.0);
        let body = cuboid(
            p,
            mid + Vec3::Y * ((WALK - BURIED) * 0.5),
            Vec3::new(LENGTH * 0.5, (WALK + BURIED) * 0.5, THICK * 0.5),
        );
        // Merlons on the outer 0.5 m of each edge, 1 m on and 1 m off along the wall. The repeat
        // is centred on a merlon, so it wraps mid-crenel where the field is continuous.
        let along = (p.x - mid.x + LENGTH * 0.5 - MERLON * 0.5).rem_euclid(2.0 * MERLON) - MERLON;
        // Each merlon reaches 0.25 m down into the walk: a merlon whose base lay on the walk's
        // top would leave both fields exactly 0 on that sample plane, an air sheet under it.
        let merlons = [-1.0_f32, 1.0]
            .map(|side| {
                cuboid(
                    Vec3::new(along, p.y, p.z),
                    Vec3::new(
                        0.0,
                        WALK + MERLON * 0.5 - 0.125,
                        mid.z + side * (THICK * 0.5 - 0.25),
                    ),
                    Vec3::new(MERLON * 0.5, MERLON * 0.5 + 0.125, 0.25),
                )
                .max(cuboid(
                    p,
                    mid + Vec3::Y * WALK,
                    Vec3::new(LENGTH * 0.5, 2.0, 2.0),
                ))
            })
            .into_iter()
            .fold(f32::MAX, f32::min);
        body.min(merlons)
    }
}

/// The material every wall chunk mesh gets.
#[derive(Resource)]
struct Stone(Handle<WeatheredMaterial>);

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `x,y,z` from an environment variable.
fn vec3_env(name: &str) -> Option<Vec3> {
    let v = std::env::var(name).ok()?;
    let mut it = v.split(',').map(|s| s.trim().parse::<f32>());
    match (it.next(), it.next(), it.next()) {
        (Some(Ok(x)), Some(Ok(y)), Some(Ok(z))) => Some(Vec3::new(x, y, z)),
        _ => None,
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        ElementsPlugins,
        IsomeshPlugin,
        ExampleHarness,
    ))
    .insert_resource(
        WorldClock::new(CivilDate::new(2026, 1, 15))
            .at_hour(f64::from(env_f32("ELEMENTS_HOUR", 10.0)))
            .with_time_scale(env_f32("ELEMENTS_TIME_SCALE", 3600.0)),
    )
    .insert_resource(Climate::temperate())
    .insert_resource(MeshBudget {
        per_frame: Duration::from_millis(3),
        max_in_flight: 8,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (attach, clear_after_snow, report));
    if std::env::var("ELEMENTS_FRAMES").is_ok() {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
    }
    app.run();
}

/// One wall volume placed by `transform`, its chunks children of it so the transform places
/// their meshes.
fn spawn_wall(commands: &mut Commands, transform: Transform) {
    let layout =
        ChunkLayout::new(CHUNK_CELLS, CELL_SIZE, [0.0, Y_MIN, 0.0]).expect("a valid chunk layout");
    let volume = commands
        .spawn((
            VoxelVolume::new(layout, Wall),
            VoxelSnow,
            transform,
            Visibility::default(),
        ))
        .id();
    for x in 0..CHUNKS_XZ {
        for y in 0..CHUNKS_Y {
            for z in 0..CHUNKS_XZ {
                commands.spawn((
                    VoxelChunk {
                        id: ChunkId::new([x, y, z]),
                        volume,
                    },
                    NeedsRemesh,
                    Transform::default(),
                    Visibility::default(),
                    ChildOf(volume),
                ));
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WeatheredMaterial>>,
    mut director: ResMut<WeatherDirector>,
) {
    let weathered = |color: Color, roughness: f32| WeatheredMaterial {
        base: StandardMaterial {
            base_color: color,
            perceptual_roughness: roughness,
            ..default()
        },
        extension: WeatheredExtension::with_exposure(1.0),
    };
    // The game's own ground: a plain mesh, no crate terrain.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(weathered(Color::srgb(0.33, 0.36, 0.27), 0.95))),
    ));
    // A plinth the turned wall stands on.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(18.0, 1.0, 6.0))),
        MeshMaterial3d(materials.add(weathered(Color::srgb(0.45, 0.43, 0.40), 0.9))),
        Transform::from_xyz(14.0, 0.5, -4.0).with_rotation(Quat::from_rotation_y(-0.52)),
    ));
    // White: the chunk meshes carry their colour per vertex (`paint`).
    commands.insert_resource(Stone(materials.add(weathered(Color::WHITE, 0.9))));
    // Field space puts a wall's middle at (8, 8): offset each volume by that.
    let wall = |x: f32, y: f32, z: f32, yaw: f32| {
        let turn = Quat::from_rotation_y(yaw);
        Transform::from_translation(Vec3::new(x, y, z) - turn * Vec3::new(8.0, 0.0, 8.0))
            .with_rotation(turn)
    };
    spawn_wall(&mut commands, wall(-10.0, 0.0, 0.0, 0.0));
    spawn_wall(&mut commands, wall(14.0, 1.0, -4.0, -0.52));
    spawn_wall(
        &mut commands,
        wall(-2.0, 0.0, -16.0, std::f32::consts::FRAC_PI_2),
    );
    let eye = vec3_env("ELEMENTS_EYE").unwrap_or(Vec3::new(-6.0, 12.0, 24.0));
    let look = vec3_env("ELEMENTS_LOOK").unwrap_or(Vec3::new(2.0, 4.0, -4.0));
    commands.spawn(camera(eye, look));
    director.force(WeatherPreset::Snow, Duration::ZERO);
}

/// Stone and snow, linear RGBA, per vertex.
const STONE: [f32; 4] = [0.24, 0.21, 0.18, 1.0];
const SNOW: [f32; 4] = [0.92, 0.94, 0.97, 1.0];

/// Colour a chunk mesh per vertex: a vertex where the wall's own field is air lies on the snow
/// slab, so it is snow, sides and all; every other vertex is stone. (A material shading by the
/// normal alone would paint the slab's sides as stone.)
fn paint(mesh: &mut Mesh) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let colors: Vec<[f32; 4]> = positions
        .iter()
        .map(|&p| if Wall.sample(p) > 0.02 { SNOW } else { STONE })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// Give every chunk mesh, new or rebuilt, its colours and the stone material. A chunk that is
/// all stone or all air meshes to nothing and gets no `Mesh3d`.
fn attach(
    mut commands: Commands,
    stone: Option<Res<Stone>>,
    mut meshes: ResMut<Assets<Mesh>>,
    fresh: Query<(Entity, &ChunkMesh), Changed<ChunkMesh>>,
) {
    let Some(stone) = stone else {
        return;
    };
    for (entity, chunk) in &fresh {
        let Some(mut mesh) = meshes.get_mut(&chunk.0).filter(|m| m.count_vertices() > 0) else {
            commands.entity(entity).try_remove::<Mesh3d>();
            continue;
        };
        paint(&mut mesh);
        commands
            .entity(entity)
            .try_insert((Mesh3d(chunk.0.clone()), MeshMaterial3d(stone.0.clone())));
    }
}

/// After `ELEMENTS_SNOW_FRAMES` frames (default: never) the snow stops, the sky clears and the
/// clock slows to a minute a second.
fn clear_after_snow(
    mut director: ResMut<WeatherDirector>,
    mut clock: ResMut<WorldClock>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    if *frame == env_f32("ELEMENTS_SNOW_FRAMES", f32::MAX) as u32 {
        director.force(WeatherPreset::Clear, Duration::ZERO);
        clock.time_scale = 60.0;
    }
}

/// Once a second: the point snowpack every wall takes its snow from.
fn report(ground: Res<SurfaceWeathering>, mut last: Local<u64>, time: Res<Time<Real>>) {
    let second = time.elapsed_secs() as u64;
    if second == *last {
        return;
    }
    *last = second;
    println!("snowpack: {:.0} mm deep", ground.snow_depth_m * 1000.0);
}

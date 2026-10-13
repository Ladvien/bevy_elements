//! GPU tests of the terrain kernels that need the crate's internals (the CPU mirrors in
//! `model/mirror.rs`, the layer buffers of `TerrainState`). Tests of the public behaviour across
//! domains are in `tests/`.
//!
//! Needs a GPU: `cargo test --release --lib -- --ignored --test-threads=1 terrain`.

#[allow(clippy::duplicate_mod)]
#[path = "../../tests/support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::storage::ShaderBuffer;
use bevy::time::TimeUpdateStrategy;

use super::model::mirror::{generate_surface, slope_deg};
use super::{ProceduralTerrain, Terrain, TerrainState};
use crate::{ElementsCamera, ElementsPlugins, ElementsQuality, Environment};

/// One test builds one renderer at a time.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

pub(crate) fn serial() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Resource, Clone)]
struct Spawn(Terrain);

fn app(terrain: Terrain, quality: ElementsQuality) -> App {
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(quality)
        .insert_resource(Spawn(terrain))
        .add_systems(
            Startup,
            |mut commands: Commands, spawn: Res<Spawn>, mut images: ResMut<Assets<Image>>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Transform::from_xyz(0.0, 30.0, 0.0),
                ));
                commands.spawn(spawn.0.clone());
            },
        );
    support::finish(&mut app);
    app
}

fn finish(app: &mut App) {
    app.world_mut().write_message(AppExit::Success);
}

/// Update until `done` holds (at most 20 s of wall time).
fn run_until(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        app.update();
        if done(app) {
            return;
        }
    }
    panic!("timed out waiting for {what}");
}

fn state(app: &App) -> Option<TerrainState> {
    app.world().get_resource::<TerrainState>().cloned()
}

fn live(app: &mut App) -> bool {
    state(app).is_some_and(|s| s.is_live())
}

/// Read a storage buffer back; updates the app until the bytes arrive.
fn read_buffer(app: &mut App, buffer: Handle<ShaderBuffer>) -> Vec<u8> {
    let slot: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
    let sink = slot.clone();
    let entity = app
        .world_mut()
        .spawn(Readback::buffer(buffer))
        .observe(move |done: On<ReadbackComplete>| {
            let mut sink = sink.lock().expect("lock");
            if sink.is_none() {
                *sink = Some(done.data.clone());
            }
        })
        .id();
    run_until(app, "a buffer readback", |_| {
        slot.lock().expect("lock").is_some()
    });
    app.world_mut().despawn(entity);
    slot.lock()
        .expect("lock")
        .take()
        .expect("the readback completed")
}

fn floats(bytes: &[u8], count: usize) -> Vec<f32> {
    bytemuck::pod_collect_to_vec::<u8, f32>(&bytes[..count * 4])
}

fn height_at(app: &mut App, p: Vec2) -> Option<f32> {
    app.world_mut()
        .run_system_once(move |env: Environment| env.terrain_height(p))
        .expect("the query runs")
}

/// Validation 1 and 5: the GPU generator reproduces the CPU mirror, the packed buffer holds the
/// surface, the sediment and the slope, and `terrain_height` answers from the readback.
#[test]
#[ignore = "needs a GPU"]
fn the_gpu_generator_matches_the_cpu_mirror() {
    let _lock = serial();
    let hills = ProceduralTerrain::hills(42);
    let mut app = app(
        Terrain::procedural(hills).with_resolution(UVec2::splat(256)),
        ElementsQuality::High,
    );
    assert_eq!(
        height_at(&mut app, Vec2::ZERO),
        None,
        "no height before the first readback"
    );
    run_until(&mut app, "the generator", live);
    let state = state(&app).expect("a terrain state");
    assert_eq!(state.grid.size, UVec2::splat(256));
    assert_eq!(state.grid.cell_m, 8.0);
    assert_eq!(state.grid.origin, Vec2::splat(-1024.0));

    let n = 256 * 256;
    let cpu = generate_surface(&hills, UVec2::splat(256), 8.0);
    let cells = bytemuck::pod_collect_to_vec::<u8, [f32; 4]>(
        &read_buffer(&mut app, state.cells.clone())[..n * 16],
    );
    let mut diffs: Vec<f32> = cells
        .iter()
        .zip(&cpu)
        .map(|(c, h)| (c[0] - h).abs())
        .collect();
    diffs.sort_by(f32::total_cmp);
    println!(
        "generator: median diff {:.5} m, p99 {:.5} m, max {:.5} m",
        diffs[n / 2],
        diffs[n * 99 / 100],
        diffs[n - 1]
    );
    assert!(diffs[n / 2] < 0.01 && diffs[n * 99 / 100] < 0.1 && diffs[n - 1] < 1.0);

    // Sediment is zero, water is zero; the bedrock buffer is the surface.
    assert!(cells.iter().all(|c| c[1] == 0.0 && c[2] == 0.0));
    let bedrock = floats(&read_buffer(&mut app, state.bedrock.clone()), n);
    assert!(
        bedrock
            .iter()
            .zip(&cells)
            .all(|(b, c)| (b - c[0]).abs() < 1e-4)
    );

    // The packed gradient reproduces the slope of the stored surface.
    let surface: Vec<f32> = cells.iter().map(|c| c[0]).collect();
    let mut worst = 0.0_f32;
    for j in [0u32, 1, 100, 254, 255] {
        for i in 0..256u32 {
            let w = cells[(j * 256 + i) as usize][3].to_bits();
            let gx = support::half(w as u16);
            let gz = support::half((w >> 16) as u16);
            let angle = (gx * gx + gz * gz).sqrt().atan().to_degrees();
            worst = worst.max((angle - slope_deg(&surface, UVec2::splat(256), 8.0, i, j)).abs());
        }
    }
    println!("packed slope vs CPU slope: worst {worst:.4}°");
    assert!(worst < 0.1, "packed slope off by {worst}°");

    // The height snapshot arrives and agrees with the surface at a cell centre.
    run_until(&mut app, "the height readback", |app| {
        height_at(app, Vec2::ZERO).is_some()
    });
    let at = Vec2::new(-1024.0 + 100.5 * 8.0, -1024.0 + 60.5 * 8.0);
    let got = height_at(&mut app, at).expect("inside the terrain");
    assert!(
        (got - surface[60 * 256 + 100]).abs() < 1e-3,
        "{got} vs {}",
        surface[60 * 256 + 100]
    );
    assert_eq!(height_at(&mut app, Vec2::new(1100.0, 0.0)), None);
    finish(&mut app);
}

/// Validation 6: level ground reads back its height, and the leaf bounds say so.
#[test]
#[ignore = "needs a GPU"]
fn a_flat_terrain_reads_back_its_height() {
    let _lock = serial();
    let mut app = app(
        Terrain::flat(12.5).with_resolution(UVec2::splat(128)),
        ElementsQuality::High,
    );
    assert_eq!(height_at(&mut app, Vec2::ZERO), None);
    run_until(&mut app, "the height readback", |app| {
        height_at(app, Vec2::ZERO).is_some()
    });
    assert_eq!(height_at(&mut app, Vec2::ZERO), Some(12.5));
    assert_eq!(height_at(&mut app, Vec2::new(500.0, -300.0)), Some(12.5));
    assert_eq!(height_at(&mut app, Vec2::new(1030.0, 0.0)), None);
    // Leaf bounds: every one of the 4 × 4 leaves is flat at 12.5.
    let state = state(&app).expect("state");
    let bounds = bytemuck::pod_collect_to_vec::<u8, [f32; 4]>(
        &read_buffer(&mut app, state.leaf_bounds)[..16 * 16],
    );
    assert!(
        bounds
            .iter()
            .all(|b| b[0] == 12.5 && b[1] == 12.5 && b[2] == 12.5 && b[3] == 0.0),
        "{bounds:?}"
    );
    finish(&mut app);
}

/// Validation 6: a 256² `R16Uint` ramp resampled to 128².
#[test]
#[ignore = "needs a GPU"]
fn a_heightmap_is_resampled_by_box_average() {
    let _lock = serial();
    let mut data = Vec::new();
    for _ in 0..256 {
        for x in 0..256u32 {
            data.extend_from_slice(&((x * 257) as u16).to_le_bytes());
        }
    }
    let image = Image::new(
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R16Uint,
        bevy::asset::RenderAssetUsages::default(),
    );
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(ElementsQuality::High)
        .add_systems(
            Startup,
            move |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                ));
                let source = images.add(image.clone());
                commands.spawn(
                    Terrain::from_heightmap(source, 0.0..255.0).with_resolution(UVec2::splat(128)),
                );
            },
        );
    support::finish(&mut app);
    run_until(&mut app, "the heightmap import", live);
    let state = state(&app).expect("state");
    let bedrock = floats(&read_buffer(&mut app, state.bedrock.clone()), 128 * 128);
    let mut worst = 0.0_f32;
    for j in 0..128 {
        for i in 0..128 {
            let want = 2.0 * i as f32 + 0.5;
            worst = worst.max((bedrock[j * 128 + i] - want).abs());
            assert_eq!(
                bedrock[j * 128 + i],
                bedrock[i],
                "row {j} differs from row 0"
            );
        }
    }
    println!("heightmap ramp: worst error {worst:.5} m");
    assert!(worst < 0.01, "worst {worst}");
    finish(&mut app);
}

/// Validation 6: an sRGB heightmap is refused and the terrain stays unspawned.
#[test]
#[ignore = "needs a GPU"]
fn an_srgb_heightmap_leaves_the_terrain_unspawned() {
    let _lock = serial();
    let image = Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![128u8; 4 * 4 * 4],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    let mut a = App::new();
    a.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_systems(
            Startup,
            move |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                ));
                let source = images.add(image.clone());
                commands.spawn(Terrain::from_heightmap(source, 0.0..100.0));
            },
        );
    support::finish(&mut a);
    for _ in 0..60 {
        a.update();
    }
    assert!(
        state(&a).is_none(),
        "an sRGB heightmap must not spawn a terrain"
    );
    finish(&mut a);
}

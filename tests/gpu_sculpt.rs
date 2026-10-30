//! `SculptTerrain` on a real GPU: a stroke changes the bedrock under it and nowhere else, each kind
//! does what its table row says, and the ground query follows. See
//! `docs/research/terrain-sculpt.md` (Validation targets).
//!
//! Needs a GPU: `cargo test --release --test gpu_sculpt -- --ignored --test-threads=1`.

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const CELLS: u32 = 128;
const CELL_M: f32 = 2.0;

/// Spawn the terrain over this snapshot instead of the flat one.
#[derive(Resource)]
struct Restore(TerrainSnapshot);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, restore: Option<Res<Restore>>) {
    commands.spawn((
        Camera3d::default(),
        support::image_target(&mut images),
        ElementsCamera::default(),
        Transform::from_xyz(0.0, 30.0, 0.0),
    ));
    if let Some(restore) = restore {
        let terrain = restore.0.terrain(&mut images);
        commands.spawn((terrain, Transform::from_translation(restore.0.centre_m())));
        return;
    }
    commands.spawn(
        Terrain::flat(10.0)
            .with_extent(Vec2::splat(CELLS as f32 * CELL_M))
            .with_resolution(UVec2::splat(CELLS)),
    );
}

fn run_until(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        app.update();
        if done(app) {
            return;
        }
    }
    panic!("timed out waiting for {what}");
}

struct Sim {
    app: App,
    terrain: TerrainState,
}

impl Sim {
    fn new() -> Self {
        Self::build(None)
    }

    fn build(restore: Option<TerrainSnapshot>) -> Self {
        let mut app = App::new();
        if let Some(snapshot) = restore {
            app.insert_resource(Restore(snapshot));
        }
        app.add_plugins((support::headless_plugins(), ElementsPlugins))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / 60.0,
            )))
            // No water moves: the bedrock changes only by hand.
            .insert_resource(HydrologySettings {
                time_scale: 0.0,
                evaporation: false,
                ..default()
            })
            .add_systems(Startup, setup);
        support::finish(&mut app);
        run_until(&mut app, "the terrain", |app| {
            app.world()
                .get_resource::<TerrainState>()
                .is_some_and(|s| s.is_live())
        });
        let terrain = app.world().resource::<TerrainState>().clone();
        Self { app, terrain }
    }

    fn run_until(&mut self, what: &str, done: impl FnMut(&mut App) -> bool) {
        run_until(&mut self.app, what, done);
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.app.update();
        }
    }

    fn bedrock(&mut self) -> Vec<f32> {
        let slot: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
        let sink = slot.clone();
        let handle = self.terrain.bedrock.clone();
        let entity = self
            .app
            .world_mut()
            .spawn(Readback::buffer(handle))
            .observe(move |done: On<ReadbackComplete>| {
                let mut sink = sink.lock().expect("lock");
                if sink.is_none() {
                    *sink = Some(done.data.to_vec());
                }
            })
            .id();
        self.run_until("a bedrock readback", |_| {
            slot.lock().expect("lock").is_some()
        });
        self.app.world_mut().despawn(entity);
        let bytes = slot.lock().expect("lock").take().expect("read");
        bytemuck::pod_collect_to_vec::<u8, f32>(&bytes[..(CELLS * CELLS) as usize * 4])
    }

    /// Send a stroke and run until the render world has applied it and the pack followed.
    fn stroke(&mut self, stroke: SculptTerrain) {
        let before = self
            .terrain
            .version
            .load(std::sync::atomic::Ordering::Relaxed);
        self.app.world_mut().write_message(stroke);
        let version = self.terrain.version.clone();
        self.run_until("the stroke", move |_| {
            version.load(std::sync::atomic::Ordering::Relaxed) > before
        });
        self.frames(8);
    }

    /// The ground height at `p`, once the readback has landed (the app only ticks while there is
    /// no answer).
    fn ground(&mut self, p: Vec2) -> f32 {
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        while std::time::Instant::now() < deadline {
            let h = self
                .app
                .world_mut()
                .run_system_once(move |env: Environment| env.terrain_height(p))
                .expect("the query runs");
            if let Some(h) = h {
                return h;
            }
            self.app.update();
        }
        panic!("timed out waiting for the ground query");
    }

    fn finish(mut self) {
        self.app.world_mut().write_message(AppExit::Success);
    }
}

fn cell_centre(sim: &Sim, x: u32, z: u32) -> Vec2 {
    sim.terrain.grid.origin + (Vec2::new(x as f32, z as f32) + 0.5) * CELL_M
}

/// Every kind, in one app (a renderer per test would exhaust the adapter): the stroke's reach, its
/// plateau, the undo, flatten, smooth and noise.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_stroke_changes_the_bedrock_under_it_and_nowhere_else() {
    let mut sim = Sim::new();
    let flat = sim.bedrock();
    let level = flat[0];
    assert!(
        flat.iter().all(|&h| h == level),
        "a flat terrain starts level"
    );

    // Raise: 5 m, radius 40 m, plateau out to 20 m, at a cell centre near the middle.
    let centre = cell_centre(&sim, 64, 64);
    let raise =
        SculptTerrain::new(centre, 40.0, SculptKind::Raise { amount_m: 5.0 }).with_hardness(0.5);
    sim.stroke(raise);
    let raised = sim.bedrock();
    let mut inside = 0;
    for z in 0..CELLS {
        for x in 0..CELLS {
            let k = (z * CELLS + x) as usize;
            let d = cell_centre(&sim, x, z).distance(centre);
            if d >= 40.0 {
                assert_eq!(
                    raised[k], flat[k],
                    "cell ({x},{z}) at {d} m is outside the radius"
                );
            } else {
                inside += 1;
                let want = level + 5.0 * sculpt_weight(d, 40.0, 0.5);
                assert!(
                    (raised[k] - want).abs() < 2e-5,
                    "cell ({x},{z}) at {d} m: {} against {want}",
                    raised[k]
                );
            }
        }
    }
    assert!(inside > 1000, "{inside} cells under the brush");
    assert_eq!(raised[(64 * CELLS + 64) as usize], level + 5.0);

    // The ground query follows the readback.
    let up = sim.ground(centre);
    assert!((up - (10.0 + 5.0)).abs() < 0.05, "ground height {up}");
    let far = sim.ground(cell_centre(&sim, 5, 5));
    assert!(
        (far - 10.0).abs() < 0.05,
        "ground height away from the stroke {far}"
    );

    // Lower undoes it.
    sim.stroke(
        SculptTerrain::new(centre, 40.0, SculptKind::Lower { amount_m: 5.0 }).with_hardness(0.5),
    );
    let undone = sim.bedrock();
    let worst = undone
        .iter()
        .zip(&flat)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(worst < 1e-5, "raise then lower leaves {worst} m");

    // Flatten to 12 m with blend 1: the plateau is level at 12 m (metre frame, y0 = 0).
    let flatten = SculptTerrain::new(
        centre,
        30.0,
        SculptKind::Flatten {
            height_m: 12.0,
            blend: 1.0,
        },
    )
    .with_hardness(0.5);
    sim.stroke(flatten);
    let flattened = sim.bedrock();
    let y0 = sim.terrain.grid.y0;
    for z in 0..CELLS {
        for x in 0..CELLS {
            let d = cell_centre(&sim, x, z).distance(centre);
            if d <= 15.0 {
                let h = flattened[(z * CELLS + x) as usize] + y0;
                assert!((h - 12.0).abs() < 1e-4, "plateau cell at {d} m is {h}");
            }
        }
    }

    // Smooth: a spike's peak falls and the stroke conserves the sum to a small fraction.
    let spike_at = cell_centre(&sim, 30, 30);
    sim.stroke(
        SculptTerrain::new(spike_at, 1.0, SculptKind::Raise { amount_m: 4.0 }).with_hardness(0.9),
    );
    let spiked = sim.bedrock();
    let peak = spiked[(30 * CELLS + 30) as usize];
    sim.stroke(
        SculptTerrain::new(spike_at, 12.0, SculptKind::Smooth { blend: 1.0 }).with_hardness(0.9),
    );
    let smoothed = sim.bedrock();
    let after = smoothed[(30 * CELLS + 30) as usize];
    assert!(after < peak - 1.0, "the spike falls: {peak} to {after}");
    let sum = |v: &[f32]| v.iter().map(|&h| f64::from(h)).sum::<f64>();
    let moved = (sum(&smoothed) - sum(&spiked)).abs();
    assert!(
        moved < 0.5 * 4.0,
        "smoothing moved {moved} m of height in total"
    );

    // Noise: one seed twice gives the same stroke, another seed another one; it stays within its
    // amplitude.
    let noise_at = cell_centre(&sim, 100, 100);
    let noise = |seed| {
        SculptTerrain::new(
            noise_at,
            20.0,
            SculptKind::Noise {
                amplitude_m: 2.0,
                feature_m: 8.0,
                seed,
            },
        )
    };
    let base = sim.bedrock();
    sim.stroke(noise(7));
    let first = sim.bedrock();
    let delta_one: Vec<f32> = first.iter().zip(&base).map(|(a, b)| a - b).collect();
    sim.stroke(noise(7));
    let second = sim.bedrock();
    let delta_two: Vec<f32> = second.iter().zip(&first).map(|(a, b)| a - b).collect();
    assert!(
        delta_one.iter().any(|d| d.abs() > 0.1),
        "noise moved nothing"
    );
    assert!(
        delta_one.iter().all(|d| d.abs() <= 2.0 + 1e-4),
        "noise exceeds its amplitude"
    );
    assert_eq!(delta_one, delta_two, "the same seed is the same stroke");
    sim.stroke(noise(8));
    let third = sim.bedrock();
    let delta_three: Vec<f32> = third.iter().zip(&second).map(|(a, b)| a - b).collect();
    assert_ne!(delta_one, delta_three, "another seed is another stroke");

    sim.finish();
}

/// A sculpted terrain, saved as a snapshot and respawned from it in a fresh app, answers the same
/// heights at every cell centre. See `docs/research/terrain-heightfield-save.md`.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_saved_heightfield_respawns_as_the_same_ground() {
    let mut sim = Sim::new();
    let centre = cell_centre(&sim, 64, 64);
    sim.stroke(
        SculptTerrain::new(centre, 40.0, SculptKind::Raise { amount_m: 5.0 }).with_hardness(0.5),
    );
    sim.frames(8);
    let saved = sim
        .app
        .world_mut()
        .run_system_once(|env: Environment| env.terrain_snapshot())
        .expect("the query runs")
        .expect("a readback has landed");
    let range = saved.height_range_m();
    assert!((range.end - range.start - 5.0).abs() < 1e-3, "{range:?}");
    let want: Vec<f32> = (0..CELLS)
        .flat_map(|z| (0..CELLS).map(move |x| (x, z)))
        .map(|(x, z)| sim.ground(cell_centre(&sim, x, z)))
        .collect();
    sim.finish();

    let mut restored = Sim::build(Some(saved));
    restored.run_until("the first readback", |app| {
        app.world_mut()
            .run_system_once(|env: Environment| env.terrain_snapshot().is_some())
            .expect("the query runs")
    });
    let mut worst = 0.0_f32;
    for z in 0..CELLS {
        for x in 0..CELLS {
            let got = restored.ground(cell_centre(&restored, x, z));
            worst = worst.max((got - want[(z * CELLS + x) as usize]).abs());
        }
    }
    assert!(worst < 2e-4, "the restored ground differs by {worst} m");
    restored.finish();
}

/// A region restored from an earlier snapshot takes the old bedrock and sediment back inside its
/// rect and leaves the stroke's work outside it, and the ground query follows. See
/// `docs/research/terrain-restore.md`.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_restored_region_takes_the_old_ground_back_and_only_there() {
    let mut sim = Sim::new();
    sim.frames(8);
    let before = sim
        .app
        .world_mut()
        .run_system_once(|env: Environment| env.terrain_snapshot())
        .expect("the query runs")
        .expect("a readback has landed");
    let flat = sim.bedrock();

    let centre = cell_centre(&sim, 64, 64);
    sim.stroke(
        SculptTerrain::new(centre, 40.0, SculptKind::Raise { amount_m: 5.0 }).with_hardness(0.5),
    );
    let raised = sim.bedrock();
    assert!(
        raised.iter().zip(&flat).any(|(r, f)| r > f),
        "the hill rose"
    );

    // The east half of the hill: cells whose centres have x at or past the hill's centre.
    let region = Rect::new(
        centre.x,
        centre.y - 100.0,
        centre.x + 100.0,
        centre.y + 100.0,
    );
    let version = sim.terrain.version.clone();
    let seen = version.load(std::sync::atomic::Ordering::Relaxed);
    sim.app
        .world_mut()
        .write_message(RestoreTerrain::new(before.clone(), region));
    sim.run_until("the restore", move |_| {
        version.load(std::sync::atomic::Ordering::Relaxed) > seen
    });
    sim.frames(8);

    let restored = sim.bedrock();
    let (mut inside, mut outside) = (0, 0);
    for z in 0..CELLS {
        for x in 0..CELLS {
            let k = (z * CELLS + x) as usize;
            if region.contains(cell_centre(&sim, x, z)) {
                inside += 1;
                assert!(
                    (restored[k] - flat[k]).abs() < 1e-5,
                    "cell ({x},{z}) inside the region is {} not {}",
                    restored[k],
                    flat[k]
                );
            } else {
                outside += 1;
                assert_eq!(restored[k], raised[k], "cell ({x},{z}) outside was touched");
            }
        }
    }
    assert!(
        inside > 1000 && outside > 1000,
        "{inside} in, {outside} out"
    );
    // The ground query follows on both sides of the rect's edge.
    let east = sim.ground(cell_centre(&sim, 80, 64));
    let west = sim.ground(cell_centre(&sim, 48, 64));
    assert!(
        (east - 10.0).abs() < 1e-3,
        "east {east} should be level again"
    );
    assert!(west > 14.0, "west {west} should still carry the hill");
    sim.finish();
}

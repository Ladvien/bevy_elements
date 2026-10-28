//! Waves on a real GPU (`docs/research/waves.md`, Validations 3–6): a body at rest holds the
//! water down under it only, and the windows keep that dip in place as the camera moves; a moving
//! body leaves a wake inside Kelvin's angle; waves do not cross onto dry ground. Each test writes
//! the hydrology's water layer on a flat terrain with the simulation held (so the water stays
//! exactly where it is written) and reads the published field.
//!
//! Needs a GPU: `cargo test --release --test gpu_waves -- --ignored --test-threads=1`.

mod support;

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

const CELLS: u32 = 64;
const CELL_M: f32 = 2.0;
/// The water's depth where there is water, m.
const DEPTH_M: f32 = 1.0;
/// Kelvin's half-angle, degrees.
const KELVIN_DEG: f32 = 19.47;

#[derive(Component)]
struct Eye;

struct Sim {
    app: App,
}

impl Sim {
    /// A terrain at height 0 where `wet(x, z)` holds and a 3 m bank elsewhere, `DEPTH_M` of
    /// water on the low ground, the hydrology running in a closed tank (so the water stays level
    /// and the terrain's packed cells follow it).
    fn new(damping: f32, wet: fn(f32, f32) -> bool) -> Self {
        Self::build(damping, wet, DEPTH_M, None)
    }

    /// As [`Sim::new`], with `depth` m of simulated water on the low ground and, if `still` is
    /// set, a [`StillWater`] at that height.
    fn build(damping: f32, wet: fn(f32, f32) -> bool, depth: f32, still: Option<f32>) -> Self {
        let half = CELLS as f32 * CELL_M * 0.5;
        let cell_xz = move |i: u32| {
            (
                (i % CELLS) as f32 * CELL_M + 0.5 * CELL_M - half,
                (i / CELLS) as f32 * CELL_M + 0.5 * CELL_M - half,
            )
        };
        let heights: Vec<u8> = (0..CELLS * CELLS)
            .flat_map(|i| {
                let (x, z) = cell_xz(i);
                let h: f32 = if wet(x, z) { 0.0 } else { 3.0 / 4.0 };
                h.to_le_bytes()
            })
            .collect();
        let mut app = App::new();
        app.add_plugins((
            support::headless_plugins(),
            ElementsPlugins
                .build()
                .disable::<PrecipitationPlugin>()
                .disable::<LightningPlugin>()
                .disable::<CloudPlugin>(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(12.0)
        })
        .insert_resource(ErosionSettings {
            enabled: false,
            ..default()
        })
        .insert_resource(HydrologySettings {
            boundary: HydrologyBoundary::Closed,
            evaporation: false,
            ..default()
        })
        .insert_resource(WaveSettings {
            enabled: true,
            damping_per_s: damping,
        });
        if let Some(level) = still {
            app.insert_resource(StillWater { level });
        }
        app
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut director: ResMut<WeatherDirector>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Eye,
                    Transform::from_xyz(0.0, 3.0, 0.0)
                        .looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y),
                ));
                let map = images.add(Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: CELLS,
                        height: CELLS,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    heights.clone(),
                    bevy::render::render_resource::TextureFormat::R32Float,
                    bevy::asset::RenderAssetUsages::default(),
                ));
                commands.spawn(
                    Terrain::from_heightmap(map, 0.0..4.0)
                        .with_extent(Vec2::splat(CELLS as f32 * CELL_M))
                        .with_resolution(UVec2::splat(CELLS)),
                );
                director.force(WeatherPreset::Clear, Duration::ZERO);
            },
        );
        support::finish(&mut app);
        let mut sim = Self { app };
        sim.run_until("the wave field", |app| {
            app.world().get_resource::<WaveState>().is_some()
                && app
                    .world()
                    .get_resource::<TerrainState>()
                    .is_some_and(|t| t.is_live())
        });
        let water = sim.app.world().resource::<TerrainState>().water.clone();
        let data: Vec<f32> = (0..CELLS * CELLS)
            .map(|i| {
                let (x, z) = cell_xz(i);
                if wet(x, z) { depth } else { 0.0 }
            })
            .collect();
        {
            let mut buffers = sim.app.world_mut().resource_mut::<Assets<ShaderBuffer>>();
            let Some(mut buffer) = buffers.get_mut(&water) else {
                panic!("the water layer exists");
            };
            buffer.clear();
            buffer.extend_from_slice(&data);
        }
        sim.frames(60);
        sim
    }

    fn run_until(&mut self, what: &str, mut done: impl FnMut(&mut App) -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        while std::time::Instant::now() < deadline {
            self.app.update();
            if done(&mut self.app) {
                return;
            }
        }
        panic!("timed out waiting for {what}");
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.app.update();
        }
    }

    fn spawn(&mut self, contact: WaterContact, at: Vec3) -> Entity {
        self.app
            .world_mut()
            .spawn((contact, Transform::from_translation(at)))
            .id()
    }

    fn place(&mut self, entity: Entity, at: Vec3) {
        if let Some(mut t) = self.app.world_mut().get_mut::<Transform>(entity) {
            t.translation = at;
        }
    }

    fn move_eye(&mut self, to: Vec3) {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<&mut Transform, With<Eye>>();
        for mut t in q.iter_mut(world) {
            t.translation = to;
        }
    }

    /// The published field: every cascade's `h` per texel, the texels a side and each cascade's
    /// texel edge.
    fn field(&mut self) -> Field {
        let state = self.app.world().resource::<WaveState>().clone();
        let n = state.texels as usize;
        let layers = state.texel_m.len();
        let bytes = support::read_texture(&mut self.app, state.field, n * 8, n * layers);
        let h = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|t| support::half(u16::from_le_bytes([t[0], t[1]])))
            .collect();
        Field {
            h,
            n: state.texels as i32,
            texel: state.texel_m.to_vec(),
        }
    }

    fn finish(mut self) {
        self.app.world_mut().write_message(AppExit::Success);
        self.app.update();
    }
}

struct Field {
    h: Vec<f32>,
    n: i32,
    texel: Vec<f32>,
}

impl Field {
    /// The surface at world XZ: every cascade's nearest texel, summed (the camera stays within
    /// a few metres of the origin, so every point here is inside every window).
    fn at(&self, xz: Vec2) -> f32 {
        let n = self.n;
        self.texel
            .iter()
            .enumerate()
            .map(|(c, t)| {
                let w = (xz / *t).floor().as_ivec2();
                let s = ((w % n) + n) % n;
                self.h[c * (n * n) as usize + (s.y * n + s.x) as usize]
            })
            .sum()
    }
}

/// Validations 3 and 6: a still, half-submerged 0.5 m sphere holds the surface down under it and
/// leaves it flat a body-length away; moving the camera 3 m does not move the dip.
#[test]
#[ignore = "needs a GPU"]
fn a_body_at_rest_holds_the_water_down_under_it_only() {
    let _lock = serial();
    // Strong damping, so the waves of its arrival die out within the run.
    let mut sim = Sim::new(1.0, |_, _| true);
    let centre = Vec3::new(1.0, DEPTH_M, 4.0);
    sim.spawn(WaterContact::sphere(0.5), centre);
    sim.frames(600);
    let field = sim.field();
    let under = field.at(centre.xz());
    let far: f32 = [
        Vec2::new(3.5, 0.0),
        Vec2::new(-3.5, 0.0),
        Vec2::new(0.0, 3.5),
        Vec2::new(2.5, -2.5),
    ]
    .iter()
    .map(|d| field.at(centre.xz() + *d).abs())
    .fold(0.0, f32::max);
    println!("at rest: under {under:.4} m, largest |h| 3.5 m away {far:.5} m");
    assert!(under < -0.02, "the water under the body is at {under}");
    assert!(far < 0.002, "the water 3.5 m away is at {far}");

    sim.move_eye(Vec3::new(3.0, 3.0, 1.0));
    sim.frames(60);
    let moved = sim.field();
    let still_under = moved.at(centre.xz());
    println!("after the camera moved 3 m: under {still_under:.4} m");
    assert!(
        (still_under - under).abs() < 0.2 * under.abs(),
        "the dip moved: {under} then {still_under}"
    );
    sim.finish();
}

/// Validation 4: a 0.3 m sphere crossing still water at 1.5 m/s leaves waves behind it only within
/// Kelvin's 19.47° (plus 4° for the grid and the body's size) of its track.
#[test]
#[ignore = "needs a GPU"]
fn a_moving_body_leaves_a_kelvin_wake() {
    let _lock = serial();
    // Damping enough that the start's transient and the tank's reflections fade, little enough
    // that the wake (renewed every moment) stands.
    let mut sim = Sim::new(0.3, |_, _| true);
    let speed = 1.5;
    let start = Vec3::new(-6.0, DEPTH_M, 3.0);
    // Lowered into the water over 2 s, then held for 4 s, so its arrival's rings have gone.
    let body = sim.spawn(WaterContact::sphere(0.3), start + Vec3::Y * 0.4);
    for f in 1..=360 {
        let lowered = (f as f32 / 120.0).min(1.0);
        sim.place(body, start + Vec3::Y * 0.4 * (1.0 - lowered));
        sim.frames(1);
    }
    let frames = 480;
    for f in 1..=frames {
        let t = f as f32 / 60.0;
        sim.place(body, start + Vec3::X * speed * t);
        sim.frames(1);
    }
    // The readback takes frames: hold the body where it ended.
    let end = start + Vec3::X * speed * frames as f32 / 60.0;
    let field = sim.field();
    // The far field, 2 to 6 m behind: per row behind the body the half-width out to which waves
    // exceed a fifth of that row's largest (the wake decays away from the body; Airy's tail beyond
    // the cusp line decays but does not stop), and the angle is the slope of that half-width
    // against the distance — the wake's lines start from the body's sides, not its centre.
    let step = 0.05;
    let rows: Vec<(f32, Vec<(f32, f32)>)> = (0..((4.0 / step) as i32))
        .map(|i| {
            let behind = 2.0 + i as f32 * step;
            let row = (-((4.0 / step) as i32)..=((4.0 / step) as i32))
                .map(|j| {
                    let side = j as f32 * step;
                    (
                        field.at(Vec2::new(end.x - behind, end.z + side)).abs(),
                        side.abs(),
                    )
                })
                .collect();
            (behind, row)
        })
        .collect();
    let peak = rows
        .iter()
        .flat_map(|(_, r)| r.iter().map(|s| s.0))
        .fold(0.0, f32::max);
    let widths: Vec<(f32, f32)> = rows
        .iter()
        .map(|(behind, r)| {
            let top = r.iter().map(|s| s.0).fold(0.0, f32::max);
            let w = r
                .iter()
                .filter(|s| s.0 > 0.2 * top)
                .map(|s| s.1)
                .fold(0.0, f32::max);
            (*behind, w)
        })
        .collect();
    // Seen from the body's side (its radius off the track), the wake's edge must stay inside
    // Kelvin's sector everywhere and reach most of it far behind: Darmon et al. 2014, the wake is
    // bounded by 19.47° at every Froude number, its largest waves at a narrower angle as the
    // Froude number grows (here U/√(g·2r) = 0.62).
    let radius = 0.3;
    let angles: Vec<f32> = widths
        .iter()
        .map(|(behind, w)| ((w - radius).max(0.0) / behind).atan().to_degrees())
        .collect();
    let widest = angles.iter().copied().fold(0.0, f32::max);
    let far: Vec<f32> = widths
        .iter()
        .zip(&angles)
        .filter(|(w, _)| w.0 >= 5.0)
        .map(|(_, a)| *a)
        .collect();
    let far_mean = far.iter().sum::<f32>() / far.len() as f32;
    println!(
        "wake: peak {peak:.4} m; half-width {:.2} m at 2 m behind, {:.2} m at 6 m; edge at most \
         {widest:.1}°, {far_mean:.1}° from 5 to 6 m behind",
        widths[0].1,
        widths[widths.len() - 1].1
    );
    assert!(peak > 0.003, "no wake behind the body (peak {peak})");
    assert!(
        widest <= KELVIN_DEG + 2.0,
        "waves at {widest:.1}°, outside Kelvin's {KELVIN_DEG}°"
    );
    assert!(
        far_mean >= KELVIN_DEG - 6.0,
        "the wake reaches only {far_mean:.1}° of Kelvin's {KELVIN_DEG}°"
    );
    sim.finish();
}

/// Validation 5: waves from a bobbing body stop at the shore: nothing on the dry ground beyond it.
#[test]
#[ignore = "needs a GPU"]
fn waves_do_not_cross_onto_dry_ground() {
    let _lock = serial();
    // Water only where x < 4 m.
    let mut sim = Sim::new(0.05, |x, _| x < 4.0);
    let body = sim.spawn(WaterContact::sphere(0.3), Vec3::new(0.0, DEPTH_M, 4.0));
    for f in 0..360 {
        let t = f as f32 / 60.0;
        sim.place(
            body,
            Vec3::new(0.0, DEPTH_M + 0.15 * (std::f32::consts::TAU * t).sin(), 4.0),
        );
        sim.frames(1);
    }
    let field = sim.field();
    let wet = (0..40)
        .map(|i| field.at(Vec2::new(-1.0 - 0.05 * i as f32, 4.0)).abs())
        .fold(0.0, f32::max);
    let dry = (0..60)
        .map(|i| field.at(Vec2::new(6.0 + 0.05 * i as f32, 4.0)).abs())
        .fold(0.0, f32::max);
    println!("shore: largest |h| on the water {wet:.4} m, on the dry ground {dry:.6} m");
    assert!(wet > 0.002, "no waves on the water ({wet})");
    assert!(dry < 1e-4, "waves on the dry ground ({dry})");
    sim.finish();
}

/// #44 acceptance 2: a body released over a *still* sea (no simulated water, no source) sends rings
/// out. Over a flat bed at 0 m with `StillWater` at 1 m, a 0.25 m sphere dropped from 0.1 m above
/// the level holds an amplitude of at least 1 mm one metre from the impact within a second.
#[test]
#[ignore = "needs a GPU"]
fn a_body_dropped_on_a_still_sea_sends_out_rings() {
    let _lock = serial();
    let mut sim = Sim::build(0.2, |_, _| true, 0.0, Some(DEPTH_M));
    let start = Vec3::new(1.0, DEPTH_M + 0.35, 4.0);
    let body = sim.spawn(WaterContact::sphere(0.25), start);
    sim.frames(2);
    // Fall at 3 m/s to the surface and below it: an impact, then rings.
    let mut peak: f32 = 0.0;
    let mut y = start.y;
    for _ in 0..60 {
        y = (y - 3.0 / 60.0).max(DEPTH_M - 0.1);
        sim.place(body, Vec3::new(start.x, y, start.z));
        sim.frames(1);
        let field = sim.field();
        let ring = [
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, -1.0),
        ]
        .iter()
        .map(|d| field.at(start.xz() + *d).abs())
        .fold(0.0, f32::max);
        peak = peak.max(ring);
    }
    println!("still sea: largest |h| 1 m from the impact over 1 s {peak:.5} m");
    assert!(peak >= 0.001, "the still sea answers with {peak} m at 1 m");
    sim.finish();
}

//! Still water on a real GPU: a declared level over terrain with no hydrology water, drawn by
//! `WaterPlugin` and answered by `Environment::water_depth`. See `docs/research/still-water.md`
//! (Validation targets 1 and 2).
//!
//! Needs a GPU: `cargo test --release --test gpu_still_water -- --ignored --test-threads=1`.

mod support;

use std::sync::Mutex;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const DT: f64 = 1.0 / 60.0;
const TARGET: u32 = 256;
/// The flat bed's height, m.
const BED: f32 = -8.0;

fn pin_weather(mut weather: ResMut<Weather>) {
    weather.precipitation_mm_h = 0.0;
    weather.temperature_c = 15.0;
}

/// Where the camera stands when a test wants other than the default view from above.
#[derive(Resource)]
struct CameraAt(Transform);

#[derive(Resource)]
struct Target(Handle<Image>);

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut director: ResMut<WeatherDirector>,
    units: Res<ElementsUnits>,
    camera: Option<Res<CameraAt>>,
) {
    let u = units.units_per_metre;
    let at = camera.map_or(
        Transform::from_xyz(0.0, 40.0 * u, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
        |c| c.0,
    );
    let mut target = Image::new_target_texture(TARGET, TARGET, TextureFormat::Rgba16Float, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(target);
    commands.insert_resource(Target(image.clone()));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(image.into()),
        ElementsCamera::default(),
        at,
    ));
    commands.spawn(
        Terrain::flat(BED)
            .with_extent(Vec2::splat(256.0))
            .with_resolution(UVec2::splat(128)),
    );
    director.force(WeatherPreset::Clear, Duration::ZERO);
}

fn build() -> App {
    build_in(1.0)
}

/// The same scene in a world of `units` units to the metre: the terrain is built in metres, so
/// only the camera's placement changes.
fn build_in(units: f32) -> App {
    build_viewed(units, None)
}

fn build_viewed(units: f32, camera: Option<Transform>) -> App {
    let mut app = App::new();
    if let Some(camera) = camera {
        app.insert_resource(CameraAt(camera));
    }
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(ElementsUnits {
            units_per_metre: units,
        })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            DT,
        )))
        .insert_resource(ElementsQuality::Ultra)
        .insert_resource(WaterSettings {
            ripple_scale: 0.0,
            ..default()
        })
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(12.0)
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            pin_weather
                .after(ElementsSystems::Weather)
                .before(ElementsSystems::Simulation),
        );
    support::finish(&mut app);
    app
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

/// The mean linear radiance of the middle of the target.
fn centre_radiance(app: &mut App) -> Vec3 {
    let image = app.world().resource::<Target>().0.clone();
    let row = TARGET as usize * 8;
    let bytes = support::read_texture(app, image, row, TARGET as usize);
    let half = TARGET as usize / 2;
    let mut sum = Vec3::ZERO;
    for y in half - 32..half + 32 {
        for x in half - 32..half + 32 {
            let i = (y * TARGET as usize + x) * 8;
            let h = |o: usize| support::half(u16::from_le_bytes([bytes[i + o], bytes[i + o + 1]]));
            sum += Vec3::new(h(0), h(2), h(4));
        }
    }
    sum / (64.0 * 64.0)
}

fn depth_at_origin(app: &mut App) -> Option<f32> {
    app.world_mut()
        .run_system_once(|env: Environment| env.water_depth(Vec2::ZERO))
        .ok()
        .flatten()
}

/// A sea 0.3 m deep and one 8 m deep over the same flat bed, seen from above through the absorb
/// pass alone: the mean red falls by the Beer–Lambert ratio, the level change rewrites a uniform,
/// and `Environment::water_depth` says the depth the surface is drawn at.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_still_level_is_drawn_with_absorption_and_answered() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = build();
    run_until(&mut app, "the terrain", |app| {
        app.world()
            .get_resource::<TerrainState>()
            .is_some_and(|s| s.is_live())
    });
    for _ in 0..60 {
        app.update();
    }
    let dry = centre_radiance(&mut app);
    assert!(
        depth_at_origin(&mut app).is_none_or(|d| d == 0.0),
        "no still level, no depth"
    );

    app.world_mut()
        .insert_resource(StillWater { level: BED + 0.3 });
    run_until(&mut app, "the still water to be drawn", |app| {
        let world = app.world_mut();
        let mut query = world.query::<(&WaterPatch, &Visibility)>();
        query
            .iter(world)
            .any(|(p, v)| p.surface && *v == Visibility::Inherited)
    });
    // The absorb pass alone: the surface pass adds the sky's reflection.
    let surface: Vec<Entity> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &WaterPatch)>();
        query
            .iter(world)
            .filter(|(_, p)| p.surface)
            .map(|(e, _)| e)
            .collect()
    };
    for entity in surface {
        app.world_mut().despawn(entity);
    }
    for _ in 0..30 {
        app.update();
    }
    let shallow = centre_radiance(&mut app);
    run_until(&mut app, "the depth answer", |app| {
        depth_at_origin(app).is_some_and(|d| d > 0.0)
    });
    let shallow_depth = depth_at_origin(&mut app).unwrap_or(f32::NAN);

    app.world_mut()
        .insert_resource(StillWater { level: BED + 8.0 });
    for _ in 0..30 {
        app.update();
    }
    let deep = centre_radiance(&mut app);
    run_until(&mut app, "the deep answer", |app| {
        depth_at_origin(app).is_some_and(|d| d > 4.0)
    });
    let deep_depth = depth_at_origin(&mut app).unwrap_or(f32::NAN);

    println!("dry {dry:?}, 0.3 m {shallow:?}, 8 m {deep:?}");
    println!("depth answers {shallow_depth} and {deep_depth}");
    let sigma = app.world().resource::<WaterSettings>().absorption.x;
    let ratio = shallow.x / deep.x;
    // View-leg transmittance only: the sun leg is in both and cancels except for the bed's own
    // light, so the ratio is exp(σ (8 − 0.3)) up to the refracted path and the additive residual.
    let expected = (sigma * 7.7).exp();
    println!("red ratio {ratio:.1}, Beer–Lambert {expected:.1}");
    assert!(
        ratio >= 5.0,
        "red under 0.3 m is {ratio}x red under 8 m, expected >= 5"
    );
    assert!(shallow.x < dry.x, "the 0.3 m sea darkens the bed's red");
    assert!((shallow_depth - 0.3).abs() < 0.01, "answer {shallow_depth}");
    assert!((deep_depth - 8.0).abs() < 0.01, "answer {deep_depth}");
    app.world_mut().write_message(AppExit::Success);
}

/// One scene of the foot-world check: an 8 m sea over a flat bed at level 0, drawn and answered.
/// Returns the mean radiance of the target's middle and the depth the query gives at the origin,
/// in metres.
fn sea_at_level_zero(units: f32) -> (Vec3, f32) {
    let mut app = build_in(units);
    app.world_mut().insert_resource(StillWater { level: 0.0 });
    run_until(&mut app, "the terrain", |app| {
        app.world()
            .get_resource::<TerrainState>()
            .is_some_and(|s| s.is_live())
    });
    run_until(&mut app, "the still water to be drawn", |app| {
        let world = app.world_mut();
        let mut query = world.query::<(&WaterPatch, &Visibility)>();
        query
            .iter(world)
            .any(|(p, v)| p.surface && *v == Visibility::Inherited)
    });
    for _ in 0..60 {
        app.update();
    }
    run_until(&mut app, "the depth answer", |app| {
        depth_at_origin(app).is_some_and(|d| d > 0.0)
    });
    let depth = depth_at_origin(&mut app).unwrap_or(f32::NAN);
    let radiance = centre_radiance(&mut app);
    app.world_mut().write_message(AppExit::Success);
    (radiance, depth)
}

/// #44 acceptance 6: `units_per_metre = 3.2808` with a level of 0.0 draws and answers as the metre
/// world with a level of 0.0 (the bed 8 m down in both).
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_still_level_in_a_foot_world_draws_and_answers_as_in_metres() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (metres, depth_m) = sea_at_level_zero(1.0);
    let (feet, depth_ft) = sea_at_level_zero(3.2808);
    println!("metres {metres:?} depth {depth_m}; feet {feet:?} depth {depth_ft}");
    assert!((depth_m - -BED).abs() < 0.01, "metre answer {depth_m}");
    assert!((depth_ft - depth_m).abs() < 0.01, "foot answer {depth_ft}");
    let worst = (metres - feet).abs().max_element() / metres.max_element().max(1e-6);
    println!(
        "RESULT feet vs metres: worst channel difference {:.3} %",
        worst * 100.0
    );
    assert!(
        worst < 0.02,
        "the foot world draws {feet:?}, the metre world {metres:?}"
    );
}

/// One column of the target as luminance, top to bottom.
fn column_luminance(app: &mut App, x: usize) -> Vec<f32> {
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(app, image, TARGET as usize * 8, TARGET as usize);
    (0..TARGET as usize)
        .map(|y| {
            let i = (y * TARGET as usize + x) * 8;
            let h = |o: usize| support::half(u16::from_le_bytes([bytes[i + o], bytes[i + o + 1]]));
            0.2126 * h(0) + 0.7152 * h(2) + 0.0722 * h(4)
        })
        .collect()
}

/// #44 acceptance 5: looking along a sea from 12 m inside the terrain grid's edge, the water
/// runs on past the edge to the horizon. With the skirt removed the same frame jumps where the
/// water stops; with it the column falls smoothly from the sky's horizon to the sea.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_still_sea_runs_past_the_terrain_grids_edge() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    // The grid's edge is at z = -128; the eye is 3 m up at z = -116 and looks level toward -Z, so
    // the edge falls about 14 degrees below the horizon and the horizon is the middle row.
    let eye = Transform::from_xyz(0.0, 3.0, -116.0).looking_to(Vec3::NEG_Z, Vec3::Y);
    let mut app = build_viewed(1.0, Some(eye));
    app.world_mut().insert_resource(StillWater { level: 0.0 });
    run_until(&mut app, "the terrain", |app| {
        app.world()
            .get_resource::<TerrainState>()
            .is_some_and(|s| s.is_live())
    });
    for _ in 0..90 {
        app.update();
    }
    let with = column_luminance(&mut app, TARGET as usize / 2);

    let skirt: Vec<Entity> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &Name)>();
        query
            .iter(world)
            .filter(|(_, n)| n.as_str().starts_with("Water skirt"))
            .map(|(e, _)| e)
            .collect()
    };
    assert_eq!(skirt.len(), 8, "four tiles, two passes each");
    for entity in skirt {
        app.world_mut().despawn(entity);
    }
    for _ in 0..30 {
        app.update();
    }
    let without = column_luminance(&mut app, TARGET as usize / 2);

    // Rows from a little under the horizon down to the bottom of the frame.
    let jump = |c: &[f32]| {
        (140..TARGET as usize - 1)
            .map(|y| (c[y + 1] - c[y]).abs())
            .fold(0.0, f32::max)
    };
    let (j_with, j_without) = (jump(&with), jump(&without));
    println!("max row step with the skirt {j_with:.5}, without {j_without:.5}");
    println!(
        "with rows 130,150,180,200,215,240: {:?}",
        [130, 150, 180, 200, 215, 240].map(|y| with[y])
    );
    println!(
        "without: {:?}",
        [130, 150, 180, 200, 215, 240].map(|y| without[y])
    );
    assert!(
        j_without > 4.0 * j_with.max(1e-4),
        "the frame without the skirt steps {j_without} at the edge, with it {j_with}"
    );
    app.world_mut().write_message(AppExit::Success);
}

//! Scattered rocks on a real GPU: `RockSettings::allowed` keeps the scatter to a host's region
//! (`docs/research/rock-scatter.md`, addendum, #43). The rocks inside a region are the
//! unrestricted scatter's own, in the same places.
//!
//! Needs a GPU: `cargo test --release --test gpu_rocks -- --ignored --test-threads=1`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const CELLS: u32 = 128;
const CELL_M: f32 = 4.0;
/// The region: a block of the 512 m terrain, centred 60 m south-east of the eye.
const REGION: Rect = Rect {
    min: Vec2::new(0.0, 0.0),
    max: Vec2::new(120.0, 120.0),
};

/// The rocks' plan positions after the scatter settles under `settings`.
fn scatter(settings: RockSettings) -> Vec<Vec2> {
    let mut app = App::new();
    app.add_plugins((
        support::headless_plugins(),
        ElementsPlugins
            .build()
            .disable::<PrecipitationPlugin>()
            .disable::<LightningPlugin>()
            .disable::<CloudPlugin>(),
    ))
    .insert_resource(settings)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .insert_resource(WorldClock {
        paused: true,
        ..WorldClock::new(CivilDate::new(2026, 6, 15)).at_hour(12.0)
    })
    .insert_resource(ErosionSettings {
        enabled: false,
        ..default()
    })
    .insert_resource(HydrologySettings {
        time_scale: 0.0,
        ..default()
    })
    .add_systems(
        Startup,
        |mut commands: Commands,
         mut images: ResMut<Assets<Image>>,
         mut director: ResMut<WeatherDirector>| {
            commands.spawn((
                Camera3d::default(),
                support::image_target(&mut images),
                ElementsCamera::default(),
                Transform::from_xyz(60.0, 3.0, 60.0)
                    .looking_at(Vec3::new(60.0, 0.0, 120.0), Vec3::Y),
            ));
            commands.spawn(
                Terrain::flat(0.0)
                    .with_extent(Vec2::splat(CELLS as f32 * CELL_M))
                    .with_resolution(UVec2::splat(CELLS)),
            );
            director.force(WeatherPreset::Clear, Duration::ZERO);
        },
    );
    support::finish(&mut app);
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        app.update();
        if app
            .world()
            .get_resource::<TerrainState>()
            .is_some_and(|t| t.is_live())
        {
            break;
        }
    }
    for _ in 0..240 {
        app.update();
    }
    let world = app.world_mut();
    let mut rocks = world.query_filtered::<&GlobalTransform, With<ScatteredRock>>();
    let at = rocks
        .iter(world)
        .map(|t| t.translation().xz())
        .collect::<Vec<_>>();
    world.write_message(AppExit::Success);
    app.update();
    at
}

#[test]
#[ignore = "needs a GPU"]
fn the_allowed_region_keeps_the_rocks_inside_it_and_moves_none() {
    let all = scatter(RockSettings::default());
    let kept = scatter(RockSettings {
        allowed: Some(vec![REGION]),
        ..default()
    });
    let none = scatter(RockSettings {
        allowed: Some(vec![]),
        ..default()
    });
    // A rock's mesh is seated a little off its plan position by its tilt: allow a metre.
    let grown = Rect {
        min: REGION.min - Vec2::ONE,
        max: REGION.max + Vec2::ONE,
    };
    let inside = all.iter().filter(|p| grown.contains(**p)).count();
    println!(
        "unrestricted {} rocks, {inside} in the region; region {}; empty {}",
        all.len(),
        kept.len(),
        none.len()
    );
    assert!(inside > 0, "the region holds rocks to keep");
    assert!(
        all.iter().any(|p| !grown.contains(*p)),
        "the terrain has rocks outside the region to drop"
    );
    assert!(
        kept.iter().all(|p| grown.contains(*p)),
        "every kept rock is inside the region"
    );
    assert!(
        kept.len() >= inside.saturating_sub(inside / 10) && kept.len() <= inside,
        "the kept rocks are the unrestricted scatter's own inside the region: {} kept of {inside}",
        kept.len()
    );
    assert!(none.is_empty(), "an empty region keeps no rock");
}

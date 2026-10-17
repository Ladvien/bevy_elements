//! Domain plugins added on their own, without `ClimatePlugin` or `CelestialPlugin`: every
//! resource their systems read is created by the plugin itself, so none of them panics and the
//! wind field still goes live.
//!
//! Needs a GPU: `cargo test --test gpu_plugins -- --ignored`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

#[test]
#[ignore = "needs a GPU"]
fn domains_run_without_the_climate_and_celestial_plugins() {
    let mut app = App::new();
    app.add_plugins((
        support::headless_plugins(),
        WindPlugin,
        FogPlugin,
        CloudPlugin,
        GroundCoverPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .add_systems(
        Startup,
        |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
            commands.spawn((
                Camera3d::default(),
                support::image_target(&mut images),
                ElementsCamera::default(),
                Transform::from_xyz(0.0, 10.0, 0.0),
            ));
        },
    );
    support::finish(&mut app);
    for _ in 0..120 {
        app.update();
    }

    assert!(
        app.world().resource::<WindState>().field_live,
        "the wind field went live without the climate plugin"
    );
    app.world_mut().write_message(AppExit::Success);
    app.update();
}

//! The smallest complete scene: the plugin group, a clock, a climate and a camera. Terrain
//! arrives in 0.3; until then the ground is a plane.
//!
//! `cargo run --release --example quickstart`

mod common;

use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera, test_scene};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 6, 21))
                .at_hour(7.0)
                .with_time_scale(120.0),
        )
        .insert_resource(Climate::temperate())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(camera(
        Vec3::new(0.0, 30.0, 0.0),
        Vec3::new(150.0, 10.0, 80.0),
    ));
    test_scene(&mut commands, &mut meshes, &mut materials);
}

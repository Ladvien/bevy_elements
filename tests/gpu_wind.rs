//! The wind field on a real GPU: the kernels compile, the solver relaxes to the log-law wind and
//! the projection leaves it nearly divergence-free, read back through `Environment::wind_at`.
//!
//! Needs a GPU: `cargo test --test gpu_wind -- --ignored`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        // Overcast blows 5 m/s × mean_wind/4: mean 8 m/s gives exactly 10 m/s at 10 m.
        .insert_resource(Climate {
            mean_wind_ms: 8.0,
            ..Climate::temperate()
        })
        .add_systems(Startup, setup);
    support::finish(&mut app);
    app
}

fn setup(
    mut commands: Commands,
    mut director: ResMut<WeatherDirector>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Camera3d::default(),
        support::image_target(&mut images),
        ElementsCamera::default(),
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
    director.force(WeatherPreset::Overcast, Duration::ZERO);
}

#[test]
#[ignore = "needs a GPU"]
fn the_wind_field_relaxes_to_the_log_law_and_stays_divergence_free() {
    let mut app = headless_app();
    for _ in 0..120 {
        app.update();
    }

    let (age, u10, mean_speed, mean_div) = app
        .world_mut()
        .run_system_once(|env: Environment| {
            let h = WIND_CELL_M;
            let centre = |c: i32| (c as f32 + 0.5) * h;
            let mut speed = 0.0;
            let mut div = 0.0;
            let mut n = 0.0;
            // Interior cells of the High grid (64 × 16 × 64 around x = z = 0), 10 m above ground.
            for cx in -24..24 {
                for cz in -24..24 {
                    let p = Vec3::new(centre(cx), 10.0, centre(cz));
                    speed += env.wind_at(p).length();
                    let d = (env.wind_at(p + Vec3::X * h).x - env.wind_at(p - Vec3::X * h).x
                        + env.wind_at(p + Vec3::Y * h).y
                        - env.wind_at(p - Vec3::Y * h).y
                        + env.wind_at(p + Vec3::Z * h).z
                        - env.wind_at(p - Vec3::Z * h).z)
                        / (2.0 * h);
                    div += d.abs();
                    n += 1.0;
                }
            }
            (
                env.snapshot_age(),
                env.weather().wind_speed_ms,
                speed / n,
                div / n,
            )
        })
        .expect("the query system runs");
    app.world_mut().write_message(AppExit::Success);

    println!(
        "u10 {u10} m/s, mean |v| at 10 m {mean_speed} m/s, mean |div| {mean_div} 1/s, snapshot age {age:?}"
    );
    assert!(
        age < Duration::from_secs(1),
        "no wind readback arrived (age {age:?})"
    );
    assert!(
        (u10 - 10.0).abs() < 1e-4,
        "forced Overcast should blow 10 m/s, got {u10}"
    );
    assert!(
        (8.0..=12.0).contains(&mean_speed),
        "mean |v| at 10 m was {mean_speed} m/s"
    );
    assert!(
        mean_div < 1e-2,
        "mean |div| after projection was {mean_div} 1/s"
    );
}

//! A world in feet answers as the same world in metres (issue #8, `docs/research/units.md`): the
//! same terrain, wind and cloud shadows through `Environment`, asked at world positions 3.2808
//! times further out, with heights in world units and speeds in m/s.
//!
//! Needs a GPU: `cargo test --release --test gpu_units -- --ignored`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const FOOT: f32 = 3.2808;

/// Hills under an overcast 10 m/s wind, the camera 40 m up at the origin, in a world of `units`
/// units to the metre: only the camera's placement and `ElementsUnits` change.
fn scene(units: f32) -> App {
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(ElementsUnits {
            units_per_metre: units,
        })
        .insert_resource(Climate {
            mean_wind_ms: 8.0,
            ..Climate::temperate()
        })
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut director: ResMut<WeatherDirector>,
                  mut images: ResMut<Assets<Image>>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Transform::from_xyz(0.0, 40.0 * units, 0.0),
                ));
                commands.spawn(Terrain::procedural(ProceduralTerrain::hills(42)));
                director.force(WeatherPreset::Overcast, Duration::ZERO);
            },
        );
    support::finish(&mut app);
    app
}

/// What the scene says at a ring of points (metres from the origin), asked in world units.
struct Answers {
    heights_m: Vec<f32>,
    wind: Vec<Vec3>,
    shadow: Vec<Option<f32>>,
}

fn ask(app: &mut App, units: f32) -> Answers {
    let points: Vec<Vec2> = (0..24)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 24.0;
            Vec2::new(a.cos(), a.sin()) * (20.0 + 3.0 * i as f32)
        })
        .collect();
    app.world_mut()
        .run_system_once(move |env: Environment| {
            let mut answers = Answers {
                heights_m: Vec::new(),
                wind: Vec::new(),
                shadow: Vec::new(),
            };
            for p in &points {
                let world = *p * units;
                let ground = env.terrain_height(world).unwrap_or(f32::NAN);
                answers.heights_m.push(ground / units);
                let above = Vec3::new(world.x, ground + 10.0 * units, world.y);
                answers.wind.push(env.wind_at(above));
                answers.shadow.push(env.cloud_shadow_at(above));
            }
            answers
        })
        .expect("the query runs")
}

fn run(units: f32) -> Answers {
    let mut app = scene(units);
    for _ in 0..300 {
        app.update();
    }
    let answers = ask(&mut app, units);
    app.world_mut().write_message(AppExit::Success);
    answers
}

#[test]
#[ignore = "needs a GPU"]
fn a_world_in_feet_answers_as_the_same_world_in_metres() {
    let metres = run(1.0);
    let feet = run(FOOT);

    let mut worst = (0.0_f32, 0.0_f32, 0.0_f32);
    for i in 0..metres.heights_m.len() {
        let (hm, hf) = (metres.heights_m[i], feet.heights_m[i]);
        assert!(
            hm.is_finite() && hf.is_finite(),
            "point {i}: no terrain answer"
        );
        worst.0 = worst.0.max((hm - hf).abs());
        let (wm, wf) = (metres.wind[i], feet.wind[i]);
        assert!(wm.length() > 1.0, "point {i}: no wind in metres ({wm})");
        worst.1 = worst.1.max((wm - wf).length() / wm.length());
        if let (Some(sm), Some(sf)) = (metres.shadow[i], feet.shadow[i]) {
            worst.2 = worst.2.max((sm - sf).abs());
        }
    }
    println!(
        "RESULT feet vs metres: height {:.2e} m, wind {:.2} %, cloud shadow {:.3}",
        worst.0,
        worst.1 * 100.0,
        worst.2
    );
    assert!(worst.0 < 1e-3, "heights differ by {} m", worst.0);
    // The wind field is simulated on the GPU, which is not bit-deterministic; the two runs step
    // the same metre-frame grid from the same inputs.
    assert!(worst.1 < 0.02, "wind differs by {} %", worst.1 * 100.0);
    assert!(worst.2 < 0.02, "cloud shadow differs by {}", worst.2);
    assert!(
        metres
            .shadow
            .iter()
            .zip(&feet.shadow)
            .all(|(a, b)| a.is_some() == b.is_some()),
        "one world read the cloud shadow map and the other did not"
    );
}

//! A game keeps its own look and lights (`docs/research/integration.md`, issues #9 and #10): the
//! `ElementsCamera` flags decide what the crate puts on the camera, a flag turned off removes only
//! what the crate inserted, and `CelestialSettings` keeps the sun and moon computed with no light
//! spawned or touched.
//!
//! Needs a GPU (the dressing is part of the rendering core):
//! `cargo test --test gpu_dressing -- --ignored --test-threads=1`.

mod support;

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use bevy::app::AppExit;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, VolumetricFog};
use bevy::pbr::{AtmosphereSettings, DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// `ElementsPlugins` at noon on the June solstice, the clock paused, with `celestial` and a camera
/// spawned by `spawn`.
fn app(celestial: CelestialSettings, spawn: impl Fn(&mut World) + Send + Sync + 'static) -> App {
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(12.0)
        })
        .insert_resource(celestial)
        .add_systems(Startup, move |world: &mut World| spawn(world));
    support::finish(&mut app);
    app
}

fn camera(world: &mut World, dressing: ElementsCamera) -> Entity {
    let target = {
        let mut images = world.resource_mut::<Assets<Image>>();
        support::image_target(&mut images)
    };
    world
        .spawn((
            Camera3d::default(),
            target,
            dressing,
            Transform::from_xyz(0.0, 2.0, 0.0).looking_to(Vec3::X, Vec3::Y),
        ))
        .id()
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn finish(mut app: App) {
    app.world_mut().write_message(AppExit::Success);
    app.update();
}

#[derive(Resource, Clone, Copy)]
struct Cam(Entity);

/// What a camera carries of the dressing.
#[derive(Debug, PartialEq)]
struct Dressing {
    atmosphere: bool,
    environment_map: bool,
    depth_prepass: bool,
    volumetric_fog: bool,
    distance_fog: bool,
    ambient: bool,
    msaa: Msaa,
    clear: bool,
}

fn dressing(app: &mut App, e: Entity) -> Dressing {
    let world = app.world();
    let entity = world.entity(e);
    Dressing {
        atmosphere: entity.contains::<AtmosphereSettings>(),
        environment_map: entity.contains::<AtmosphereEnvironmentMapLight>(),
        depth_prepass: entity.contains::<DepthPrepass>(),
        volumetric_fog: entity.contains::<VolumetricFog>(),
        distance_fog: entity.contains::<DistanceFog>(),
        ambient: entity.contains::<AmbientLight>(),
        msaa: *entity.get::<Msaa>().expect("Camera requires Msaa"),
        clear: !matches!(
            entity.get::<Camera>().expect("a camera").clear_color,
            ClearColorConfig::Default
        ),
    }
}

fn count<C: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<C>>()
        .iter(app.world())
        .count()
}

/// `integration.md` Validation 2: the default flags dress the camera exactly as before, with one
/// atmosphere, a sun and a moon light, the moon disc and the star field.
#[test]
#[ignore = "needs a GPU"]
fn the_default_dressing_is_unchanged() {
    let _lock = serial();
    let mut app = app(CelestialSettings::default(), |world| {
        let e = camera(world, ElementsCamera::default());
        world.insert_resource(Cam(e));
    });
    run(&mut app, 3);
    let e = app.world().resource::<Cam>().0;
    assert_eq!(
        dressing(&mut app, e),
        Dressing {
            atmosphere: true,
            environment_map: true,
            depth_prepass: true,
            volumetric_fog: true,
            distance_fog: true,
            ambient: true,
            msaa: Msaa::Off,
            clear: true,
        }
    );
    assert_eq!(count::<Atmosphere>(&mut app), 1);
    assert_eq!(count::<DirectionalLight>(&mut app), 2);
    assert_eq!((count::<Sun>(&mut app), count::<Moon>(&mut app)), (1, 1));
    // The moon disc, the star field and the lightning pipeline's warm-up ribbon.
    assert_eq!(count::<Mesh3d>(&mut app), 3);
    finish(app);
}

/// `integration.md` Validation 1 and 4: with every flag off the crate inserts nothing on the
/// camera but the depth prepass its passes read, spawns no atmosphere, light, moon or stars, and
/// leaves the game's own fog, ambient, MSAA and `Sun`-marked light as the game set them; the sun
/// and moon are still computed, to the same values as with the crate's lights.
#[test]
#[ignore = "needs a GPU"]
fn a_bare_camera_without_lights_keeps_the_games_look() {
    let _lock = serial();
    let fog = DistanceFog {
        color: Color::srgb(0.8, 0.1, 0.1),
        falloff: FogFalloff::Linear {
            start: 5.0,
            end: 50.0,
        },
        ..default()
    };
    let game_fog = fog.clone();
    let mut app = app(
        CelestialSettings {
            lights: false,
            sky: false,
        },
        move |world| {
            let e = camera(world, ElementsCamera::bare());
            world.entity_mut(e).insert((
                game_fog.clone(),
                AmbientLight {
                    brightness: 123.0,
                    ..default()
                },
                Msaa::Sample4,
            ));
            world.insert_resource(Cam(e));
            world.spawn((
                Sun,
                DirectionalLight {
                    illuminance: 777.0,
                    ..default()
                },
                Transform::from_xyz(1.0, 2.0, 3.0),
            ));
        },
    );
    run(&mut app, 30);
    let e = app.world().resource::<Cam>().0;
    assert_eq!(
        dressing(&mut app, e),
        Dressing {
            atmosphere: false,
            environment_map: false,
            depth_prepass: true,
            volumetric_fog: false,
            distance_fog: true,
            ambient: true,
            msaa: Msaa::Sample4,
            clear: false,
        }
    );
    let world = app.world();
    let kept = world.get::<DistanceFog>(e).expect("the game's fog");
    assert_eq!(kept.color, fog.color);
    assert!(
        matches!(kept.falloff, FogFalloff::Linear { start, end } if start == 5.0 && end == 50.0)
    );
    assert_eq!(
        world.get::<AmbientLight>(e).map(|a| a.brightness),
        Some(123.0)
    );
    assert_eq!(count::<Atmosphere>(&mut app), 0);
    // Only the lightning pipeline's warm-up ribbon: no moon disc, no star field.
    assert_eq!(count::<Mesh3d>(&mut app), 1);
    assert_eq!(count::<DirectionalLight>(&mut app), 1);
    let (light, transform) = app
        .world_mut()
        .query_filtered::<(&DirectionalLight, &Transform), With<Sun>>()
        .single(app.world())
        .expect("the game's sun");
    assert_eq!(light.illuminance, 777.0);
    assert_eq!(transform.translation, Vec3::new(1.0, 2.0, 3.0));
    let bare = *app.world().resource::<SunState>();
    finish(app);

    // The same noon with the crate's lights: the same sun and moon.
    let mut lit = self::app(CelestialSettings::default(), |world| {
        camera(world, ElementsCamera::default());
    });
    run(&mut lit, 3);
    let with_lights = *lit.world().resource::<SunState>();
    finish(lit);
    assert!(
        bare.altitude_deg > 60.0,
        "noon sun at {}°",
        bare.altitude_deg
    );
    assert_eq!(bare.direction, with_lights.direction);
    assert_eq!(bare.moon_phase, with_lights.moon_phase);
}

/// `integration.md` Validation 3: a flag turned off at runtime removes what the crate inserted and
/// turned back on restores it; the clear colour the crate set goes back to the game's default.
#[test]
#[ignore = "needs a GPU"]
fn flags_flip_at_runtime() {
    let _lock = serial();
    let mut app = app(CelestialSettings::default(), |world| {
        let e = camera(world, ElementsCamera::default());
        world.insert_resource(Cam(e));
    });
    run(&mut app, 3);
    let e = app.world().resource::<Cam>().0;
    let set = |app: &mut App, f: fn(&mut ElementsCamera)| {
        f(&mut app
            .world_mut()
            .get_mut::<ElementsCamera>(e)
            .expect("the camera"));
        run(app, 2);
    };
    set(&mut app, |c| {
        c.atmosphere = false;
        c.volumetric_fog = false;
        c.msaa_off = false;
    });
    let off = dressing(&mut app, e);
    assert!(!off.atmosphere && !off.environment_map && !off.volumetric_fog && !off.clear);
    assert_eq!(off.msaa, Msaa::default());
    assert!(off.distance_fog && off.ambient && off.depth_prepass);
    set(&mut app, |c| *c = ElementsCamera::default());
    let on = dressing(&mut app, e);
    assert!(on.atmosphere && on.environment_map && on.volumetric_fog && on.clear);
    assert_eq!(on.msaa, Msaa::Off);
    assert_eq!(count::<Atmosphere>(&mut app), 1);
    finish(app);
}

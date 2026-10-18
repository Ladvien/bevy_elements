//! A few buildings in `WeatheredMaterial`, rained and then snowed on: walls darken and gloss and
//! run with streaks, roofs and the ground turn white, and the pavilion at the right keeps
//! `exposure` 0 under a `PrecipitationBlocker` roof, so it stays dry and bare while the same
//! weather falls everywhere else. A metal box and a glazed one show that only porous surfaces
//! darken.
//!
//! `cargo run --release --example weathered_town`
//!
//! The weather cycles Rain (45 s) then Snow (75 s) of real time at a clock 300 times real time.
//! `ELEMENTS_PRESET=Snow` (or `Rain`, `Clear`) fixes the weather instead, `ELEMENTS_HOUR=14`
//! starts the clock at that local hour, and `ELEMENTS_SCALE=600` changes the time scale; for
//! screenshots run `ELEMENTS_PRESET=Snow ELEMENTS_FRAMES=2400 ELEMENTS_SHOT=/tmp/x.png`. The
//! clock keeps running: wetness and snow only build while game time passes.
//!
//! `ELEMENTS_CALENDAR=november` lets the scheduler pick the weather of a cool-temperate November
//! instead (mean 9.4 °C, annual swing 10 °C, daily 4 °C, seed 1, 42.5° N, from 1 November 2026)
//! with a `SnowSeason` from 14 November to 31 March: the cold rain of the first fortnight wets the
//! town and never whitens it. `ELEMENTS_SNOW_SEASON=off` drops the season (the same weather then
//! lies as snow from the second night). With the `serde` feature, `ELEMENTS_SAVE=/tmp/town.json`
//! writes a save (clock, climate, location, weather director, snowpack) on the last frame and
//! `ELEMENTS_LOAD=/tmp/town.json` starts from one. `ELEMENTS_EV100=13` fixes the exposure, for
//! time-lapse stills (auto exposure adapts over 1.5 s of real time).

mod common;

use std::time::Duration;

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_elements::*;
use common::{ExampleHarness, camera};

const RAIN_S: f32 = 45.0;
const SNOW_S: f32 = 75.0;

fn env_f64(name: &str) -> Option<f64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

fn fixed_preset() -> Option<WeatherPreset> {
    let name = std::env::var("ELEMENTS_PRESET").ok()?;
    WeatherPreset::ALL
        .into_iter()
        .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
}

/// The snow may fall from 14 November to 31 March.
const SNOW_SEASON: SnowSeason = SnowSeason {
    first: (11, 14),
    last: (3, 31),
};

fn november() -> bool {
    std::env::var("ELEMENTS_CALENDAR").is_ok_and(|v| v.eq_ignore_ascii_case("november"))
}

fn main() {
    let hour = env_f64("ELEMENTS_HOUR").unwrap_or(13.0);
    let scale = env_f64("ELEMENTS_SCALE").unwrap_or(300.0) as f32;
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, ElementsPlugins, ExampleHarness))
        .add_systems(Startup, setup);
    if november() {
        let season = std::env::var("ELEMENTS_SNOW_SEASON").map_or(true, |v| v != "off");
        app.insert_resource(
            WorldClock::new(CivilDate::new(2026, 11, 1))
                .at_hour(hour)
                .with_time_scale(scale),
        )
        .insert_resource(GeoLocation {
            latitude_deg: 42.5,
            ..default()
        })
        .insert_resource(Climate {
            mean_temperature_c: 9.4,
            annual_amplitude_c: 10.0,
            diurnal_amplitude_c: 4.0,
            snow_season: season.then_some(SNOW_SEASON),
            ..Climate::temperate().with_seed(1)
        });
    } else {
        app.insert_resource(
            WorldClock::new(CivilDate::new(2026, 4, 15))
                .at_hour(hour)
                .with_time_scale(scale),
        )
        .insert_resource(Climate::temperate());
        if fixed_preset().is_none() {
            app.add_systems(Update, cycle.before(ElementsSystems::Clock));
        }
    }
    #[cfg(feature = "serde")]
    save::plugin(&mut app);
    app.run();
}

/// A save file: what the weather and the ground cannot re-derive.
#[cfg(feature = "serde")]
mod save {
    use bevy::prelude::*;
    use bevy_elements::*;

    type SaveFile = (
        WorldClock,
        Climate,
        GeoLocation,
        WeatherDirector,
        SurfaceWeathering,
    );

    pub fn plugin(app: &mut App) {
        if let Some(path) = std::env::var_os("ELEMENTS_LOAD") {
            let file = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("ELEMENTS_LOAD {path:?}: {e}"));
            let (mut clock, climate, location, director, ground): SaveFile =
                serde_json::from_str(&file)
                    .unwrap_or_else(|e| panic!("ELEMENTS_LOAD {path:?}: {e}"));
            // The saved clock runs at its own scale unless ELEMENTS_SCALE says otherwise.
            if let Some(scale) = super::env_f64("ELEMENTS_SCALE") {
                clock.time_scale = scale as f32;
            }
            app.insert_resource(clock)
                .insert_resource(climate)
                .insert_resource(location)
                .insert_resource(director)
                .insert_resource(ground);
        }
        if std::env::var_os("ELEMENTS_SAVE").is_some() {
            app.add_systems(Last, write_on_exit);
        }
    }

    fn write_on_exit(mut exits: MessageReader<AppExit>, world: &World) {
        if exits.read().next().is_none() {
            return;
        }
        let Some(path) = std::env::var_os("ELEMENTS_SAVE") else {
            return;
        };
        let file = serde_json::to_string(&(
            world.resource::<WorldClock>(),
            world.resource::<Climate>(),
            world.resource::<GeoLocation>(),
            world.resource::<WeatherDirector>(),
            world.resource::<SurfaceWeathering>(),
        ))
        .unwrap_or_else(|e| panic!("ELEMENTS_SAVE: {e}"));
        std::fs::write(&path, file).unwrap_or_else(|e| panic!("ELEMENTS_SAVE {path:?}: {e}"));
        info!("saved to {path:?}");
    }
}

/// Rain, then snow, then rain again.
fn cycle(
    time: Res<Time<Real>>,
    mut director: ResMut<WeatherDirector>,
    mut current: Local<Option<WeatherPreset>>,
) {
    let t = time.elapsed_secs() % (RAIN_S + SNOW_S);
    let want = if t < RAIN_S {
        WeatherPreset::Rain
    } else {
        WeatherPreset::Snow
    };
    if *current != Some(want) {
        director.force(want, Duration::from_secs(8));
        *current = Some(want);
    }
}

struct Builder<'a> {
    commands: Commands<'a, 'a>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<WeatheredMaterial>,
}

impl Builder<'_> {
    fn material(
        &mut self,
        color: Color,
        roughness: f32,
        metallic: f32,
        exposure: f32,
    ) -> Handle<WeatheredMaterial> {
        self.materials.add(WeatheredMaterial {
            base: StandardMaterial {
                base_color: color,
                perceptual_roughness: roughness,
                metallic,
                ..default()
            },
            extension: WeatheredExtension::with_exposure(exposure),
        })
    }

    /// A box of `size` standing on the ground at `x, z`.
    fn cuboid(&mut self, size: Vec3, at: Vec2, material: Handle<WeatheredMaterial>) -> Entity {
        self.commands
            .spawn((
                Mesh3d(self.meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(material),
                Transform::from_xyz(at.x, size.y * 0.5, at.y),
            ))
            .id()
    }

    /// A house with a gable roof of two slabs tilted 35°, ridge along x.
    fn house(&mut self, size: Vec3, at: Vec2, wall: Color, roof: Color) {
        let wall = self.material(wall, 0.9, 0.0, 1.0);
        self.cuboid(size, at, wall);
        let roof = self.material(roof, 0.85, 0.0, 1.0);
        let theta = 35.0_f32.to_radians();
        let overhang = 0.5;
        let slab = Vec3::new(
            size.x + 2.0 * overhang,
            0.25,
            (size.z * 0.5 + overhang) / theta.cos(),
        );
        let rise = (size.z * 0.5) * theta.tan();
        for side in [-1.0_f32, 1.0] {
            self.commands.spawn((
                Mesh3d(self.meshes.add(Cuboid::new(slab.x, slab.y, slab.z))),
                MeshMaterial3d(roof.clone()),
                Transform::from_xyz(
                    at.x,
                    size.y + rise * 0.5 + 0.125 / theta.cos(),
                    at.y + side * size.z * 0.25,
                )
                .with_rotation(Quat::from_rotation_x(side * theta)),
            ));
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WeatheredMaterial>>,
) {
    // `ELEMENTS_VIEW=wall` stands close to a brick wall (the rain streaks); the default sees the
    // whole town from above.
    let (eye, target) = match std::env::var("ELEMENTS_VIEW").as_deref() {
        Ok("wall") => (Vec3::new(-12.0, 2.5, 8.0), Vec3::new(-12.0, 2.5, 22.0)),
        _ => (Vec3::new(0.0, 12.0, -20.0), Vec3::new(0.0, 2.0, 26.0)),
    };
    let mut view = commands.spawn(camera(eye, target));
    // A fixed exposure for time-lapse stills: auto exposure adapts over 1.5 s of real time.
    if let Some(ev100) = env_f64("ELEMENTS_EV100") {
        view.insert((
            ElementsCamera {
                auto_exposure: false,
                ..default()
            },
            Exposure {
                ev100: ev100 as f32,
            },
        ));
    }
    if let Some(preset) = fixed_preset() {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<WeatherDirector>()
                .force(preset, Duration::ZERO);
        });
    }

    let mut b = Builder {
        commands: commands.reborrow(),
        meshes: &mut meshes,
        materials: &mut materials,
    };

    // The ground.
    let ground = b.material(Color::srgb(0.30, 0.34, 0.22), 0.95, 0.0, 1.0);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(600.0, 0.2, 600.0))),
        MeshMaterial3d(ground),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    // Houses: brick, plaster, and a timber shed.
    b.house(
        Vec3::new(9.0, 5.0, 7.0),
        Vec2::new(-12.0, 24.0),
        Color::srgb(0.55, 0.26, 0.20),
        Color::srgb(0.22, 0.20, 0.20),
    );
    b.house(
        Vec3::new(7.0, 8.0, 7.0),
        Vec2::new(-26.0, 36.0),
        Color::srgb(0.72, 0.64, 0.48),
        Color::srgb(0.30, 0.16, 0.12),
    );
    b.house(
        Vec3::new(5.0, 3.0, 4.0),
        Vec2::new(-20.0, 12.0),
        Color::srgb(0.38, 0.27, 0.16),
        Color::srgb(0.20, 0.20, 0.22),
    );

    // A metal box and a glazed one: low porosity, so they gloss but do not darken.
    let metal = b.material(Color::srgb(0.65, 0.66, 0.70), 0.35, 1.0, 1.0);
    b.cuboid(Vec3::new(2.5, 2.0, 2.5), Vec2::new(16.0, 12.0), metal);
    let glazed = b.material(Color::srgb(0.20, 0.45, 0.60), 0.35, 0.0, 1.0);
    b.cuboid(Vec3::new(2.5, 2.0, 2.5), Vec2::new(20.0, 12.0), glazed);

    // The pavilion: a flat roof (a `PrecipitationBlocker`, so no particle falls under it) over a
    // box and four posts whose exposure is 0, so they take no wetness, streaks or snow.
    let sheltered = b.material(Color::srgb(0.55, 0.26, 0.20), 0.9, 0.0, 0.0);
    b.cuboid(
        Vec3::new(4.0, 2.5, 4.0),
        Vec2::new(8.0, 22.0),
        sheltered.clone(),
    );
    for (dx, dz) in [(-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0), (5.0, 5.0)] {
        b.cuboid(
            Vec3::new(0.4, 4.0, 0.4),
            Vec2::new(8.0 + dx, 22.0 + dz),
            sheltered.clone(),
        );
    }
    let roof_material = b.material(Color::srgb(0.40, 0.40, 0.42), 0.85, 0.0, 1.0);
    let roof_half = Vec3::new(5.6, 0.2, 5.6);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(
            roof_half.x * 2.0,
            roof_half.y * 2.0,
            roof_half.z * 2.0,
        ))),
        MeshMaterial3d(roof_material),
        Transform::from_xyz(8.0, 4.2, 22.0),
        PrecipitationBlocker {
            half_extents: roof_half,
        },
    ));
    // The same box in the open, beside it, for comparison.
    let open = b.material(Color::srgb(0.55, 0.26, 0.20), 0.9, 0.0, 1.0);
    b.cuboid(Vec3::new(4.0, 2.5, 4.0), Vec2::new(-1.0, 22.0), open);
}

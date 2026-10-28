//! The clouds in the environment map (`docs/research/sky-light.md`): the followed camera's
//! environment map is filtered from a cube the clouds are composed into, which equals Bevy's
//! atmosphere cube without cloud, carries a graded overcast dome whose light replaces the
//! uniform cloud ambient, and shows broken cloud as cloud. Both cubes are read back: the
//! camera's `GeneratedEnvironmentMapLight::environment_map` is the atmosphere cube until the
//! probe is handed over, and the composed cube after.
//!
//! Needs a GPU: `cargo test --release --test gpu_sky_probe -- --ignored --test-threads=1`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::Hdr;
use bevy::light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

#[derive(Component)]
struct Eye;

/// The model's horizontal illuminance at the camera, lx (`Environment::illuminance_at`).
#[derive(Resource, Default)]
struct Model(f32);

/// What the camera saw: both cubes (linear RGB per texel, faces in order), their edge, the
/// irradiance Bevy filtered from the composed cube for normals up, sideways (+X) and down
/// (luminance of `E/π`, cd/m²), the model illuminance and the camera's ambient brightness.
struct Capture {
    atmosphere: Vec<Vec3>,
    composed: Vec<Vec3>,
    size: usize,
    sky_light: [f32; 3],
    model_lux: f32,
    ambient: f32,
}

/// Run `preset` at `hour` on the June solstice (45° N) for `frames` frames with the camera 2 m
/// up, with the cloud cover overridden by `cover` if given, and read both cubes in one frame.
fn capture(preset: WeatherPreset, hour: f64, cover: Option<f32>, frames: u32) -> Capture {
    let mut app = App::new();
    let plugins = ElementsPlugins
        .build()
        .disable::<PrecipitationPlugin>()
        .disable::<LightningPlugin>();
    app.add_plugins((support::headless_plugins(), plugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(hour)
        })
        .init_resource::<Model>()
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut director: ResMut<WeatherDirector>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Hdr,
                    Eye,
                    Transform::from_xyz(0.0, 2.0, 0.0).looking_to(Vec3::X, Vec3::Y),
                ));
                director.force(preset, Duration::ZERO);
            },
        )
        .add_systems(
            Update,
            (move |mut weather: ResMut<Weather>| {
                if let Some(cover) = cover {
                    weather.cloud_cover = cover;
                }
            })
            .after(ElementsSystems::Weather)
            .before(ElementsSystems::Celestial),
        )
        .add_systems(
            Update,
            (|env: Environment, mut model: ResMut<Model>| {
                model.0 = env.illuminance_at(Vec3::new(0.0, 2.0, 0.0));
            })
            .after(ElementsSystems::Readback),
        );
    support::finish(&mut app);
    let source = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<&GeneratedEnvironmentMapLight, With<Eye>>();
        q.single(app.world())
            .map(|g| g.environment_map.clone())
            .unwrap_or_else(|e| panic!("the camera has an environment map: {e}"))
    };
    app.update();
    app.update();
    let atmosphere = source(&mut app);
    for _ in 0..frames {
        app.update();
    }
    let composed = source(&mut app);
    assert_ne!(
        atmosphere, composed,
        "the probe took the environment map over"
    );
    let size = app
        .world()
        .resource::<Assets<Image>>()
        .get(&atmosphere)
        .map_or(0, |i| i.texture_descriptor.size.width as usize);
    assert!(size > 0, "the atmosphere cube is in the main world");
    // Bevy's irradiance map (32² per face) has no `COPY_SRC`; adding it re-creates the texture,
    // which the filter then fills again within a frame.
    let diffuse = {
        let mut lights = app
            .world_mut()
            .query_filtered::<&EnvironmentMapLight, With<Eye>>();
        lights
            .single(app.world())
            .map(|e| e.diffuse_map.clone())
            .unwrap_or_else(|e| panic!("the camera has an environment map light: {e}"))
    };
    if let Some(mut image) = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&diffuse)
    {
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    }
    for _ in 0..5 {
        app.update();
    }
    let irradiance_map = support::read_texture(&mut app, diffuse, 32 * 8, 32 * 6);
    let centre = |face: usize| {
        let t = &irradiance_map[((face * 32 + 16) * 32 + 16) * 8..][..8];
        let c = |k: usize| support::half(u16::from_le_bytes([t[k], t[k + 1]]));
        luminance(Vec3::new(c(0), c(2), c(4)))
    };
    let sky_light = [centre(2), centre(0), centre(3)];
    let mut bytes = support::read_textures(
        &mut app,
        vec![
            (atmosphere, size * 8, size * 6),
            (composed, size * 8, size * 6),
        ],
    );
    let decode = |bytes: Vec<u8>| -> Vec<Vec3> {
        bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|t| {
                let c = |k: usize| support::half(u16::from_le_bytes([t[k], t[k + 1]]));
                Vec3::new(c(0), c(2), c(4))
            })
            .collect()
    };
    let composed = decode(bytes.pop().unwrap_or_default());
    let atmosphere = decode(bytes.pop().unwrap_or_default());
    let model_lux = app.world().resource::<Model>().0;
    let mut ambient = app.world_mut().query_filtered::<&AmbientLight, With<Eye>>();
    let ambient = ambient
        .single(app.world())
        .map_or(f32::NAN, |a| a.brightness);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    Capture {
        atmosphere,
        composed,
        size,
        sky_light,
        model_lux,
        ambient,
    }
}

/// World direction of a cube texel (Bevy's `sample_cube_dir` with z negated, cubemaps being
/// left-handed) and its solid angle.
fn texel(face: usize, x: usize, y: usize, n: usize) -> (Vec3, f32) {
    let u = 2.0 * (x as f32 + 0.5) / n as f32 - 1.0;
    let v = 2.0 * (y as f32 + 0.5) / n as f32 - 1.0;
    let c = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    let d = c.normalize();
    let side = 2.0 / n as f32;
    (
        Vec3::new(d.x, d.y, -d.z),
        side * side / (1.0 + u * u + v * v).powf(1.5),
    )
}

/// Every texel with its direction and solid angle.
fn texels(cube: &[Vec3], n: usize) -> impl Iterator<Item = (Vec3, f32, Vec3)> + '_ {
    (0..6).flat_map(move |face| {
        (0..n).flat_map(move |y| {
            (0..n).map(move |x| {
                let (d, w) = texel(face, x, y, n);
                (d, w, cube[(face * n + y) * n + x])
            })
        })
    })
}

fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Mean luminance of the texels whose elevation lies in `[lo, hi)` degrees.
fn band(cube: &[Vec3], n: usize, lo: f32, hi: f32) -> f32 {
    let (mut sum, mut weight) = (0.0, 0.0);
    for (d, w, c) in texels(cube, n) {
        let e = d.y.clamp(-1.0, 1.0).asin().to_degrees();
        if e >= lo && e < hi {
            sum += luminance(c) * w;
            weight += w;
        }
    }
    sum / weight.max(1e-9)
}

/// Horizontal illuminance of the upper hemisphere, `∫ L cos θ dω`, lx.
fn irradiance(cube: &[Vec3], n: usize) -> f32 {
    texels(cube, n)
        .filter(|(d, _, _)| d.y > 0.0)
        .map(|(d, w, c)| luminance(c) * d.y * w)
        .sum()
}

/// Validation 1: without cloud and without the weather's air, the composed cube is the
/// atmosphere cube (the sky gain is 1 under a full beam and the cloud transmittance 1).
#[test]
#[ignore = "needs a GPU"]
fn without_cloud_the_probe_is_the_atmosphere_itself() {
    let c = capture(WeatherPreset::Clear, 12.0, Some(0.0), 60);
    let mut worst = 0.0f32;
    for (a, b) in c.atmosphere.iter().zip(&c.composed) {
        let la = luminance(*a);
        if la > 1e-3 {
            worst = worst.max((luminance(*b) - la).abs() / la);
        }
    }
    println!("RESULT clear: worst relative difference {worst:.4}");
    assert!(worst < 0.01, "worst relative difference {worst}");
}

/// Validation 2 and 5: an overcast dome at noon lights the ground within 35 % of the model's
/// diffuse light (the uniform ambient it replaces; measured 0.76), its zenith luminance over its
/// illuminance is that of an overcast sky (0.30–0.45: Lambertian 0.318, Kähler's graded skies
/// 0.382–0.409; measured 0.344), its zenith is no darker than its horizon, Bevy's irradiance map
/// filtered from it lights a wall at 0.55–0.85 of a top and an underside at 0.25–0.6 (measured
/// 0.68 and 0.40, where the uniform ambient gave all three the same), and the camera's ambient is
/// retired. Real overcasts are graded 2:1 to 3:1 from zenith to horizon (Kähler, CIE type 1); the
/// cloud model's diffusion term does not depend on the viewing direction, so its dome is nearly
/// flat (measured 1.08, `sky-light.md` Deviation 7).
#[test]
#[ignore = "needs a GPU"]
fn an_overcast_dome_carries_the_cloud_light() {
    let c = capture(WeatherPreset::Overcast, 12.0, None, 240);
    let n = c.size;
    let zenith = band(&c.composed, n, 75.0, 90.1);
    let horizon = band(&c.composed, n, 5.0, 10.0);
    let e = irradiance(&c.composed, n);
    let [up, side, down] = c.sky_light;
    println!(
        "RESULT overcast: zenith {zenith:.0} cd/m², 5–10° {horizon:.0} cd/m², ratio {:.2}; \
         E {e:.0} lx, model {:.0} lx, E/model {:.2}; L_Z/E {:.3}; sky light up {up:.0}, \
         side {:.2}, down {:.2}; ambient {}",
        zenith / horizon,
        c.model_lux,
        e / c.model_lux,
        zenith / e,
        side / up,
        down / up,
        c.ambient,
    );
    assert!(
        (1.0..=3.5).contains(&(zenith / horizon)),
        "gradation {}",
        zenith / horizon
    );
    assert!(
        (0.30..=0.45).contains(&(zenith / e)),
        "L_Z/E {}",
        zenith / e
    );
    assert!(
        (0.65..=1.35).contains(&(e / c.model_lux)),
        "E {e} against the model's {}",
        c.model_lux
    );
    assert!(
        (0.55..=0.85).contains(&(side / up)),
        "a wall's sky light {}",
        side / up
    );
    assert!(
        (0.25..=0.6).contains(&(down / up)),
        "an underside's sky light {}",
        down / up
    );
    assert_eq!(
        c.ambient, 0.0,
        "the cloud ambient is retired on the probe camera"
    );
}

/// Blue over red of a colour: the clear sky above 10° is above 2 (the composed zenith of
/// `PartlyCloudy` at noon reads 3.2, 10° 2.3), sunlit and shaded cumulus 1.0–1.3.
fn blueness(c: Vec3) -> f32 {
    c.z / c.x.max(1e-6)
}

/// Share of the dome above 10° whose texels are grey-white (blue over red under 1.6).
fn cloud_share(cube: &[Vec3], n: usize) -> f32 {
    let (mut cloudy, mut all) = (0.0f32, 0.0f32);
    for (d, w, c) in texels(cube, n) {
        if d.y < 10f32.to_radians().sin() {
            continue;
        }
        all += w;
        if blueness(c) < 1.6 {
            cloudy += w;
        }
    }
    cloudy / all
}

/// Validation 3: broken cumulus (cover 0.4) shows in the cube as cloud, grey-white texels over
/// a share of the dome near the cover, where Bevy's atmosphere cube is blue everywhere.
#[test]
#[ignore = "needs a GPU"]
fn broken_cloud_shows_in_the_probe() {
    let c = capture(WeatherPreset::PartlyCloudy, 12.0, None, 240);
    let (composed, clear) = (
        cloud_share(&c.composed, c.size),
        cloud_share(&c.atmosphere, c.size),
    );
    println!(
        "RESULT partly cloudy: cloud share above 10°: probe {composed:.3}, atmosphere {clear:.3}"
    );
    assert!(clear < 0.01, "the atmosphere cube has no cloud: {clear}");
    assert!((0.2..=0.7).contains(&composed), "cloud share {composed}");
}

/// A moonless night's fill lights surfaces but is not a bright horizon: the lift (by the camera's
/// exposure, `docs/research/night-sky-light.md` Deviation 2) fades out toward the horizon, so a
/// specular reflection there sees the sky the screen draws, and the irradiance on a horizontal
/// surface is what the uniform lift gave (3.29e-3 measured before the change).
#[test]
#[ignore = "needs a GPU"]
fn the_night_fill_lifts_the_light_on_surfaces_and_not_the_horizon() {
    let c = capture(WeatherPreset::Clear, 0.0, Some(0.0), 120);
    let n = c.size;
    let (low, high) = (
        band(&c.composed, n, 2.0, 6.0),
        band(&c.composed, n, 50.0, 90.0),
    );
    println!(
        "RESULT night: horizon band {low:.3e}, zenith band {high:.3e}, irradiance {:?}",
        c.sky_light
    );
    assert!(
        low < 0.15 * high,
        "the horizon band carries the lift: {low} against {high}"
    );
    assert!(
        (c.sky_light[0] - 3.29e-3).abs() < 0.2 * 3.29e-3,
        "the horizontal irradiance moved: {}",
        c.sky_light[0]
    );
}

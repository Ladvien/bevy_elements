//! The rainbow on a real GPU: a shower opposite a low sun draws Airy's bows in the sky composite
//! (`docs/research/rainbow.md`, Validation targets 7 and 9).
//!
//! Needs a GPU: `cargo test --release --test gpu_rainbow -- --ignored --test-threads=1`. Set
//! `ELEMENTS_TEST_DUMP=/path/prefix` to write each frame as a binary PPM.

mod support;

use std::sync::Mutex;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::{PerspectiveProjection, Projection, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const DT: f64 = 1.0 / 60.0;
const SIZE: u32 = 512;
/// Vertical and horizontal field of view, degrees.
const FOV_DEG: f32 = 100.0;
/// How far above the horizon the camera looks, degrees.
const PITCH_DEG: f32 = 8.0;

#[derive(Resource)]
struct Target(Handle<Image>);

/// The held weather: the rate, the cover and the temperature (which sets the share of the rate
/// that falls as snow).
#[derive(Resource, Clone, Copy)]
struct Shower {
    rate_mm_h: f32,
    cover: f32,
    temperature_c: f32,
}

fn pin_weather(mut weather: ResMut<Weather>, shower: Res<Shower>) {
    weather.precipitation_mm_h = shower.rate_mm_h;
    weather.cloud_cover = shower.cover;
    weather.temperature_c = shower.temperature_c;
    // June has no snow season: let the temperature alone decide the phase.
    weather.snow_possible = 1.0;
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut director: ResMut<WeatherDirector>,
) {
    let mut target = Image::new_target_texture(SIZE, SIZE, TextureFormat::Rgba16Float, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(target);
    commands.insert_resource(Target(image.clone()));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(image.into()),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEG.to_radians(),
            ..default()
        }),
        ElementsCamera::default(),
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
    commands.spawn(
        Terrain::flat(-8.0)
            .with_extent(Vec2::splat(256.0))
            .with_resolution(UVec2::splat(128)),
    );
    director.force(WeatherPreset::PartlyCloudy, Duration::ZERO);
}

/// Turn the camera to the antisolar point and raise it by [`PITCH_DEG`].
fn face_away_from_the_sun(sun: Res<SunState>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let away = Vec3::new(-sun.direction.x, 0.0, -sun.direction.z).normalize_or(Vec3::NEG_Z);
    let pitch = PITCH_DEG.to_radians();
    for mut transform in &mut cameras {
        let to = away * pitch.cos() + Vec3::Y * pitch.sin();
        *transform = Transform::from_xyz(0.0, 10.0, 0.0).looking_to(to, Vec3::Y);
    }
}

fn build(hour: f64, shower: Shower, strength: f32) -> App {
    let mut app = App::new();
    app.insert_resource(shower)
        .add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(RainbowSettings { strength })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            DT,
        )))
        .insert_resource(ElementsQuality::Ultra)
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(hour)
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                pin_weather
                    .after(ElementsSystems::Weather)
                    .before(ElementsSystems::Simulation),
                face_away_from_the_sun.after(ElementsSystems::Celestial),
            ),
        );
    support::finish(&mut app);
    app
}

/// A frame: the linear radiance of every pixel, row-major, after `frames` updates.
struct Frame {
    pixels: Vec<Vec3>,
    camera: Transform,
    sun: Vec3,
}

fn render(hour: f64, shower: Shower, strength: f32, dump: &str) -> Frame {
    let mut app = build(hour, shower, strength);
    // The clouds' history and the weather map need a few hundred frames to settle.
    for _ in 0..300 {
        app.update();
    }
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(&mut app, image, SIZE as usize * 8, SIZE as usize);
    let pixels: Vec<Vec3> = (0..(SIZE * SIZE) as usize)
        .map(|k| {
            let h = |o: usize| {
                support::half(u16::from_le_bytes([bytes[k * 8 + o], bytes[k * 8 + o + 1]]))
            };
            Vec3::new(h(0), h(2), h(4))
        })
        .collect();
    let camera = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(app.world())
        .next()
        .copied()
        .unwrap_or_default();
    let sun = app.world().resource::<SunState>().direction;
    if let Ok(base) = std::env::var("ELEMENTS_TEST_DUMP") {
        let mut out = format!("P6 {SIZE} {SIZE} 255\n").into_bytes();
        for p in &pixels {
            for c in 0..3 {
                out.push((p[c].clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8);
            }
        }
        let _ = std::fs::write(format!("{base}-{dump}.ppm"), out);
    }
    app.world_mut().write_message(AppExit::Success);
    Frame {
        pixels,
        camera,
        sun,
    }
}

impl Frame {
    /// The world direction of pixel `(x, y)`.
    fn direction(&self, x: usize, y: usize) -> Vec3 {
        let half = (FOV_DEG.to_radians() * 0.5).tan();
        let u = ((x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0) * half;
        let v = (1.0 - (y as f32 + 0.5) / SIZE as f32 * 2.0) * half;
        let local = Vec3::new(u, v, -1.0).normalize();
        self.camera.rotation * local
    }

    /// The scattering angle of pixel `(x, y)`, degrees, and whether it looks above the horizon.
    fn theta(&self, x: usize, y: usize) -> (f32, bool) {
        let d = self.direction(x, y);
        (
            d.dot(self.sun).clamp(-1.0, 1.0).acos().to_degrees(),
            d.y > 0.03,
        )
    }
}

/// The mean of `on - off` over the sky pixels whose scattering angle lies in a bin, per channel.
fn ring(on: &Frame, off: &Frame, lo: f32, hi: f32) -> Vec3 {
    let (mut sum, mut n) = (Vec3::ZERO, 0.0f32);
    for y in 0..SIZE as usize {
        for x in 0..SIZE as usize {
            let (theta, sky) = on.theta(x, y);
            if sky && (lo..hi).contains(&theta) {
                sum += on.pixels[y * SIZE as usize + x] - off.pixels[y * SIZE as usize + x];
                n += 1.0;
            }
        }
    }
    sum / n.max(1.0)
}

/// The mean of a frame's sky pixels whose scattering angle lies in a bin.
fn sky_mean(frame: &Frame, lo: f32, hi: f32) -> Vec3 {
    let (mut sum, mut n) = (Vec3::ZERO, 0.0f32);
    for y in 0..SIZE as usize {
        for x in 0..SIZE as usize {
            let (theta, sky) = frame.theta(x, y);
            if sky && (lo..hi).contains(&theta) {
                sum += frame.pixels[y * SIZE as usize + x];
                n += 1.0;
            }
        }
    }
    sum / n.max(1.0)
}

fn luminance(c: Vec3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

const RAIN: Shower = Shower {
    rate_mm_h: 5.0,
    cover: 0.45,
    temperature_c: 15.0,
};

/// [`RAIN`]'s rate at 0.5 °C: half of it falls as snow (`Weather::snow_fraction`).
const SLEET: Shower = Shower {
    temperature_c: 0.5,
    ..RAIN
};

/// [`RAIN`]'s rate at −5 °C: all of it falls as snow.
const SNOW: Shower = Shower {
    temperature_c: -5.0,
    ..RAIN
};

/// The light a frame's bow adds over the primary's band, 136–141°, against the sky behind it.
fn primary_band(on: &Frame, off: &Frame) -> (f32, f32) {
    (
        luminance(ring(on, off, 136.0, 141.0)),
        luminance(sky_mean(off, 136.0, 141.0)),
    )
}

/// Low sun, a shower behind the camera: the bow's ring adds light with red on its outside, the
/// secondary is fainter and reversed, the sky between is not lit, and a frame with the strength
/// at 0 is the frame without the feature.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_shower_opposite_a_low_sun_draws_a_rainbow() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let off = render(17.5, RAIN, 0.0, "off");
    let on = render(17.5, RAIN, RainbowSettings::default().strength, "on");
    let sun_alt = on.sun.y.asin().to_degrees();
    println!("ROW sun elevation {sun_alt:.1} deg");
    let mut peak = (0.0f32, 0.0f32);
    for k in 0..60 {
        let lo = 130.0 + k as f32 * 0.25;
        let r = ring(&on, &off, lo, lo + 0.25);
        println!("ROW theta {lo:.2}: {r:?}");
        if luminance(r) > peak.1 {
            peak = (lo, luminance(r));
        }
    }
    println!("ROW primary peak {:?}", peak);
    assert!(
        (136.0..141.0).contains(&peak.0),
        "the primary is at {}",
        peak.0
    );
    // A vivid bow: its ring adds at least a fifth to the sky it is seen against.
    let (lo, hi) = (peak.0, peak.0 + 0.25);
    let added = luminance(ring(&on, &off, lo, hi));
    let behind = luminance(sky_mean(&off, lo, hi));
    println!("ROW the ring adds {added:.4} to a sky of {behind:.4}");
    assert!(added > 0.2 * behind, "the ring adds {added} to {behind}");
    // Red lies on the outside of the primary (the smaller scattering angle), violet inside.
    let outside = ring(&on, &off, 137.5, 138.5);
    let inside = ring(&on, &off, 139.5, 140.5);
    assert!(
        outside.x / outside.z.max(1e-9) > inside.x / inside.z.max(1e-9),
        "red over blue: outside {outside:?}, inside {inside:?}"
    );
    // Alexander's dark band: nothing between the bows.
    let band = ring(&on, &off, 132.0, 134.0);
    assert!(
        luminance(band) < 0.1 * peak.1,
        "the dark band is {} against a peak of {}",
        luminance(band),
        peak.1
    );
    // Nothing near the sun.
    let near_sun = ring(&on, &off, 100.0, 110.0);
    assert!(luminance(near_sun).abs() < 2e-3, "{near_sun:?}");

    // Sleet's drops are those of the rain half of the same rate, and its snow, some thirty times
    // as opaque per mm/h, takes all but a few per cent of the curtain's light: a bow, and under a
    // fifth of the rain's. Not under the 3 % share itself: the rain alone is a thin curtain
    // (`1 − e^{−τ}` well under 1), the snow makes the sleet's thick (`docs/research/rainbow.md`,
    // Validation target 11).
    let sleet_off = render(17.5, SLEET, 0.0, "sleet-off");
    let sleet_on = render(17.5, SLEET, RainbowSettings::default().strength, "sleet-on");
    let (rain_added, _) = primary_band(&on, &off);
    let (sleet_added, sleet_sky) = primary_band(&sleet_on, &sleet_off);
    println!(
        "ROW primary band: rain adds {rain_added:.4}, sleet adds {sleet_added:.4} to {sleet_sky:.4}"
    );
    assert!(
        sleet_added < 0.2 * rain_added,
        "sleet adds {sleet_added}, rain {rain_added}"
    );
    assert!(
        sleet_added > 0.005 * rain_added,
        "sleet adds {sleet_added}, rain {rain_added}"
    );
}

/// A high sun puts the whole bow below the horizon, and a dry sky has no shower to draw it on (the
/// frames agree to the GPU's own run-to-run noise, a level or so of the cloud edges).
#[test]
#[ignore = "needs a GPU"]
fn gpu_no_rainbow_under_a_high_sun_or_without_rain() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let off = render(12.0, RAIN, 0.0, "noon-off");
    let on = render(12.0, RAIN, RainbowSettings::default().strength, "noon-on");
    let worst = on
        .pixels
        .iter()
        .zip(&off.pixels)
        .map(|(a, b)| (*a - *b).abs().max_element())
        .fold(0.0f32, f32::max);
    assert!(worst < 0.02, "a noon sun drew {worst}");
    let dry = Shower {
        rate_mm_h: 0.0,
        ..RAIN
    };
    let off = render(17.5, dry, 0.0, "dry-off");
    let on = render(17.5, dry, RainbowSettings::default().strength, "dry-on");
    let worst = on
        .pixels
        .iter()
        .zip(&off.pixels)
        .map(|(a, b)| (*a - *b).abs().max_element())
        .fold(0.0f32, f32::max);
    assert!(worst < 0.02, "a dry sky drew {worst}");
}

/// Falling snow makes no rainbow: a flake is not a liquid sphere. The same rate, cover and sun as
/// the rain's bow, at −5 °C (issue #51; `docs/research/rainbow.md`, Validation target 11).
#[test]
#[ignore = "needs a GPU"]
fn gpu_falling_snow_draws_no_rainbow() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let off = render(17.5, SNOW, 0.0, "snow-off");
    let on = render(17.5, SNOW, RainbowSettings::default().strength, "snow-on");
    let (added, behind) = primary_band(&on, &off);
    println!("ROW snow: the primary band adds {added:.4} to a sky of {behind:.4}");
    assert!(
        added.abs() < 0.02 * behind,
        "falling snow drew a bow: {added} on {behind}"
    );
}

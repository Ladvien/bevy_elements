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


/// Throwaway: the primary's band against the temperature through the sleet band.
#[test]
#[ignore = "needs a GPU"]
fn gpu_sweep_the_sleet_band() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    for (k, t) in [1.5f32, 1.0, 0.0, -0.5].into_iter().enumerate() {
        let shower = Shower { temperature_c: t, ..RAIN };
        let off = render(17.5, shower, 0.0, &format!("sweep{k}-off"));
        let on = render(17.5, shower, RainbowSettings::default().strength, &format!("sweep{k}-on"));
        let (added, behind) = primary_band(&on, &off);
        println!("SWEEP {t:5.1} C: primary band adds {added:.4} to {behind:.4}");
    }
}

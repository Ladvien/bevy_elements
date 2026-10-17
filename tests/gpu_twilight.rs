//! The twilight sky on a real GPU, read from a linear `Rgba16Float` target with no tonemapping
//! (`docs/research/twilight.md`, Validation 1, 2, 3 and 5). Clouds, rain and lightning are off,
//! so every sampled pixel is Bevy's atmosphere lit by the crate's sun and moon.
//!
//! Needs a GPU: `cargo test --test gpu_twilight -- --ignored`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::{Exposure, Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const SIZE: u32 = 32;

#[derive(Resource)]
struct Target(Handle<Image>);

/// A clear sky at `hour` on `date` (lat 45° N), camera 2 m up looking along `look`.
fn sky(date: CivilDate, hour: f64, auto_exposure: bool) -> App {
    let mut app = App::new();
    let plugins = ElementsPlugins
        .build()
        .disable::<CloudPlugin>()
        .disable::<PrecipitationPlugin>()
        .disable::<LightningPlugin>();
    app.add_plugins((support::headless_plugins(), plugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(date).at_hour(hour)
        })
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut director: ResMut<WeatherDirector>| {
                let mut image =
                    Image::new_target_texture(SIZE, SIZE, TextureFormat::Rgba16Float, None);
                image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
                let handle = images.add(image);
                commands.insert_resource(Target(handle.clone()));
                commands.spawn((
                    Camera3d::default(),
                    RenderTarget::Image(handle.into()),
                    ElementsCamera { auto_exposure },
                    Tonemapping::None,
                    DebandDither::Disabled,
                    Hdr,
                    Transform::from_xyz(0.0, 2.0, 0.0).looking_to(Vec3::Y, Vec3::Z),
                ));
                director.force(WeatherPreset::Clear, Duration::ZERO);
            },
        );
    support::finish(&mut app);
    app
}

fn look(app: &mut App, dir: Vec3) {
    let up = if dir.y > 0.99 { Vec3::Z } else { Vec3::Y };
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Transform, With<ElementsCamera>>();
    for mut transform in cameras.iter_mut(app.world_mut()) {
        *transform = Transform::from_translation(transform.translation).looking_to(dir, up);
    }
}

/// The centre pixel, linear RGB as written (scene luminance × exposure).
fn centre(app: &mut App) -> Vec3 {
    for _ in 0..30 {
        app.update();
    }
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(app, image, SIZE as usize * 8, SIZE as usize);
    let i = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 8;
    let c = |k: usize| support::half(u16::from_le_bytes([bytes[i + k], bytes[i + k + 1]]));
    Vec3::new(c(0), c(2), c(4))
}

fn exposure(app: &mut App) -> f32 {
    let mut q = app
        .world_mut()
        .query_filtered::<&Exposure, With<ElementsCamera>>();
    q.single(app.world()).map_or(f32::NAN, Exposure::exposure)
}

fn close(app: &mut App) {
    app.world_mut().write_message(AppExit::Success);
    app.update();
}

const SOLSTICE: CivilDate = CivilDate::new(2026, 6, 21);

/// Zenith luminance (cd/m²) and colour after sunset, at a fixed exposure.
fn zenith(hour: f64) -> (f32, Vec3, f32) {
    let mut app = sky(SOLSTICE, hour, false);
    let rgb = centre(&mut app) / exposure(&mut app);
    let altitude = app.world().resource::<SunState>().altitude_deg;
    close(&mut app);
    (rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722)), rgb, altitude)
}

/// Validation 1 and 2: from sunset to a sun 5.5° down, the zenith falls as Hulburt (1953)
/// measured it, 0.427 decades per degree (within a factor 2), and stays blue to blue-purple.
#[test]
#[ignore = "needs a GPU"]
fn the_twilight_zenith_falls_as_measured_and_stays_blue() {
    let (reference, _, h0) = zenith(19.8);
    assert!(
        (-1.0..0.0).contains(&h0),
        "the reference row is at sunset: {h0}"
    );
    for hour in [20.0, 20.2, 20.4] {
        let (l, rgb, h) = zenith(hour);
        let measured = l / reference;
        let hulburt = 10f32.powf(-0.427 * (h0 - h));
        println!(
            "sun {h:.2}°: zenith {l:.3e} cd/m², ratio {measured:.3e}, Hulburt {hulburt:.3e}, rgb {rgb}"
        );
        assert!(
            (0.5..=2.0).contains(&(measured / hulburt)),
            "sun {h}: {measured} against {hulburt}"
        );
        // Blue or blue-purple (Hulburt): blue leads, red may pass green near Haber's purple light.
        assert!(
            rgb.z > rgb.x && rgb.z > rgb.y,
            "the twilight zenith is blue: {rgb}"
        );
    }
}

/// Validation 3: with both lights off the sky behind the atmosphere is the faint night glow,
/// not Bevy's default clear colour, which showed through as 0.02 at any exposure; a camera that
/// chose its own clear colour keeps it.
#[test]
#[ignore = "needs a GPU"]
fn the_moonless_night_sky_is_dark() {
    // New moon, 23:00: the sun is ~20° down and the moon is below the horizon.
    let mut app = sky(CivilDate::new(2026, 6, 15), 23.0, false);
    let own = Color::srgb(0.2, 0.0, 0.0);
    let second = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let target = support::image_target(&mut images);
        app.world_mut()
            .spawn((
                Camera3d::default(),
                Camera {
                    clear_color: ClearColorConfig::Custom(own),
                    order: 1,
                    ..default()
                },
                target,
                ElementsCamera::default(),
            ))
            .id()
    };
    let rgb = centre(&mut app);
    let state = *app.world().resource::<SunState>();
    println!(
        "sun {:.1}°, moon {:.1}°: zenith {rgb}",
        state.altitude_deg, state.moon_altitude_deg
    );
    assert!(state.altitude_deg < -18.0 && state.moon_altitude_deg < 0.0);
    assert!(rgb.max_element() < 1e-5, "{rgb}");
    let kept = app
        .world()
        .get::<Camera>(second)
        .map(|camera| matches!(camera.clear_color, ClearColorConfig::Custom(c) if c == own));
    assert_eq!(kept, Some(true));
    close(&mut app);
}

/// Validation 5: under the automatic exposure the sky 5° above the horizon toward a low sun
/// stays at or under 3.2 times the display peak (+10 %), where it was 6–20 before the cap.
#[test]
#[ignore = "needs a GPU"]
fn the_glow_toward_a_low_sun_is_not_blown_out() {
    for hour in [19.5, 19.8, 20.0] {
        let mut app = sky(SOLSTICE, hour, true);
        app.update();
        let sun = *app.world().resource::<SunState>();
        let flat = Vec3::new(sun.direction.x, 0.0, sun.direction.z).normalize();
        look(&mut app, (flat + Vec3::Y * 0.0875).normalize());
        let rgb = centre(&mut app);
        let display = rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722));
        println!(
            "sun {:.2}°: glow display value {display:.3}",
            sun.altitude_deg
        );
        assert!(display <= 3.5, "sun {}: {display}", sun.altitude_deg);
        close(&mut app);
    }
}

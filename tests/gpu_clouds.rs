//! The clouds on a real GPU, through the public API: the weather map's area fraction, the shadow
//! map, the sun's light factor and the quality switch.
//!
//! Needs a GPU: `cargo test --release --test gpu_clouds -- --ignored`.

#![allow(clippy::chunks_exact_to_as_chunks)]

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::light::light_consts::lux;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

/// A cloud cover the test imposes after the scheduler ran (`None`: the preset's own).
#[derive(Resource, Clone, Copy, Default)]
struct CoverOverride(Option<f32>);

fn force_cover(over: Res<CoverOverride>, mut weather: ResMut<Weather>) {
    if let Some(cover) = over.0 {
        weather.cloud_cover = cover;
    }
}

/// What the shadow map looked like when it was created, before any kernel ran.
#[derive(Resource, Default)]
struct CreatedShadow {
    format: Option<TextureFormat>,
    size: (u32, u32),
    first_texels: Vec<u8>,
}

fn record_created_shadow(
    shadows: Res<CloudShadows>,
    images: Res<Assets<Image>>,
    mut created: ResMut<CreatedShadow>,
) {
    if let Some(image) = images.get(&shadows.map) {
        created.format = Some(image.texture_descriptor.format);
        created.size = (image.width(), image.height());
        created.first_texels = image.data.clone().unwrap_or_default();
    }
}

#[derive(Clone, Copy)]
struct Setup {
    preset: WeatherPreset,
    position: Vec3,
    quality: ElementsQuality,
}

fn app(setup: Setup) -> App {
    let mut app = App::new();
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(12.0)
        })
        .insert_resource(setup.quality)
        .init_resource::<CoverOverride>()
        .init_resource::<CreatedShadow>()
        .add_systems(
            Update,
            force_cover
                .after(ElementsSystems::Weather)
                .before(ElementsSystems::Celestial),
        )
        .add_systems(PostStartup, record_created_shadow)
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut director: ResMut<WeatherDirector>| {
                commands.spawn((
                    Camera3d::default(),
                    support::image_target(&mut images),
                    ElementsCamera::default(),
                    Transform::from_translation(setup.position).looking_to(Vec3::Y, Vec3::Z),
                ));
                director.force(setup.preset, Duration::ZERO);
            },
        );
    support::finish(&mut app);
    app
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn finish(app: &mut App) {
    app.world_mut().write_message(AppExit::Success);
}

/// Fraction of weather-map texels whose coverage byte is at least 128.
fn covered_fraction(app: &mut App) -> f32 {
    let map = app.world().resource::<CloudWeatherMap>().map.clone();
    let bytes = support::read_texture(app, map, 512 * 4, 512);
    let covered = bytes.chunks_exact(4).filter(|t| t[0] >= 128).count();
    covered as f32 / (512.0 * 512.0)
}

/// The transmittance channel of the shadow map, row-major 1024².
fn shadow_transmittance(app: &mut App) -> Vec<f32> {
    let map = app.world().resource::<CloudShadows>().map.clone();
    let bytes = support::read_texture(app, map, 1024 * 8, 1024);
    bytes
        .chunks_exact(8)
        .map(|t| support::half(u16::from_le_bytes([t[0], t[1]])))
        .collect()
}

/// The sun's light factor `0.25 + 0.75 T̄` as the lights currently carry it.
fn sun_factor(app: &mut App) -> f32 {
    let mut lights = app
        .world_mut()
        .query_filtered::<&DirectionalLight, With<Sun>>();
    let light = lights.single(app.world()).expect("one sun");
    light.illuminance / lux::RAW_SUNLIGHT
}

/// A mean over the 9×9 texels around 64 deterministic texel positions, the light factor a
/// camera standing there would get.
fn mean_light_factor(t: &[f32]) -> f32 {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 33) as usize
    };
    let mut total = 0.0;
    for _ in 0..64 {
        let (cx, cy) = (60 + next() % 900, 60 + next() % 900);
        let mut sum = 0.0;
        for y in cy - 4..=cy + 4 {
            for x in cx - 4..=cx + 4 {
                sum += t[y * 1024 + x];
            }
        }
        total += 0.25 + 0.75 * sum / 81.0;
    }
    total / 64.0
}

fn dark_fraction(t: &[f32], below: f32) -> f32 {
    t.iter().filter(|&&v| v <= below).count() as f32 / t.len() as f32
}

fn mean(t: &[f32]) -> f32 {
    t.iter().sum::<f32>() / t.len() as f32
}

fn setup(preset: WeatherPreset) -> Setup {
    Setup {
        preset,
        position: Vec3::new(0.0, 6.0, 0.0),
        quality: ElementsQuality::High,
    }
}

/// Validation 2: the fraction of the weather map that is cloud equals `cloud_cover`, wherever
/// the camera stands, increases strictly with the cover, and is exactly 0 and 1 at the ends.
#[test]
#[ignore = "needs a GPU"]
fn the_weather_map_holds_the_requested_sky_fraction() {
    let mut app = app(setup(WeatherPreset::Clear));
    for position in [
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(10_000.0, 6.0, 0.0),
        Vec3::new(0.0, 6.0, 10_000.0),
    ] {
        let mut cameras = app
            .world_mut()
            .query_filtered::<&mut Transform, With<ElementsCamera>>();
        for mut transform in cameras.iter_mut(app.world_mut()) {
            transform.translation = position;
        }
        let measure = |app: &mut App, cover: f32| {
            app.world_mut().resource_mut::<CoverOverride>().0 = Some(cover);
            run(app, 6);
            covered_fraction(app)
        };
        for cover in [0.05, 0.4, 0.6, 0.95] {
            let fraction = measure(&mut app, cover);
            println!("camera {position}: cover {cover} -> fraction {fraction:.4}");
            assert!(
                (fraction - cover).abs() <= 0.02,
                "cover {cover} gave {fraction} at {position}"
            );
        }
        assert_eq!(measure(&mut app, 0.0), 0.0, "no cloud at cover 0");
        assert_eq!(measure(&mut app, 1.0), 1.0, "all cloud at cover 1");
        let mut last = -1.0;
        for i in 0..20 {
            let cover = 0.025 + 0.05 * i as f32;
            let fraction = measure(&mut app, cover);
            assert!(
                fraction > last,
                "fraction {fraction} at cover {cover} is not above {last}"
            );
            last = fraction;
        }
    }
    finish(&mut app);
}

/// Validation 11 (a)–(e) and the light-factor wiring: the shadow map's coverage, the camera
/// factor `0.25 + 0.75 T̄` per weather, and the sun light it dims.
#[test]
#[ignore = "needs a GPU"]
fn the_shadow_map_and_the_sun_factor_follow_the_clouds() {
    let mut app = app(setup(WeatherPreset::Clear));
    let measure = |app: &mut App, preset: WeatherPreset, cover: Option<f32>| {
        app.world_mut().resource_mut::<CoverOverride>().0 = cover;
        app.world_mut()
            .resource_mut::<WeatherDirector>()
            .force(preset, Duration::ZERO);
        // Past the 1 s smoothing of the camera transmittance.
        run(app, 420);
        let t = shadow_transmittance(app);
        let camera = app.world().resource::<CloudShadows>().camera_transmittance;
        let sun = sun_factor(app);
        (t, camera, sun)
    };

    let (clear, clear_camera, clear_sun) = measure(&mut app, WeatherPreset::Clear, None);
    let clear_dark = dark_fraction(&clear, 0.5);
    let clear_factor = mean_light_factor(&clear);
    println!(
        "Clear: dark {clear_dark:.3}, factor {clear_factor:.3}, mean T {:.3}",
        mean(&clear)
    );
    assert!((0.01..=0.12).contains(&clear_dark), "{clear_dark}");
    assert!((0.93..=1.0).contains(&clear_factor), "{clear_factor}");

    let (overcast, overcast_camera, overcast_sun) =
        measure(&mut app, WeatherPreset::Overcast, None);
    let overcast_dark = dark_fraction(&overcast, 0.2);
    let overcast_factor = mean_light_factor(&overcast);
    println!("Overcast: dark {overcast_dark:.3}, factor {overcast_factor:.3}");
    assert!(overcast_dark >= 0.9, "{overcast_dark}");
    assert!(
        (0.25..=0.37).contains(&overcast_factor),
        "{overcast_factor}"
    );

    let (storm, _, _) = measure(&mut app, WeatherPreset::Thunderstorm, None);
    let storm_factor = mean_light_factor(&storm);
    println!("Thunderstorm: factor {storm_factor:.3}");
    assert!((0.25..=0.30).contains(&storm_factor), "{storm_factor}");

    // The lights carry the factor the readback gave (the `0.25 + 0.75 T̄` of the camera).
    for (camera, sun) in [(clear_camera, clear_sun), (overcast_camera, overcast_sun)] {
        assert!(
            (sun - (0.25 + 0.75 * camera)).abs() < 5e-3,
            "sun factor {sun} for camera transmittance {camera}"
        );
    }
    let ratio = overcast_sun / clear_sun;
    println!("sun light overcast / clear = {ratio:.3}");
    assert!(ratio <= 0.4, "{ratio}");

    // (d) The mean transmittance falls by at least 0.1 per step with the cover.
    let mut means = Vec::new();
    for cover in [0.05, 0.4, 0.95] {
        let (t, _, _) = measure(&mut app, WeatherPreset::Clear, Some(cover));
        means.push(mean(&t));
    }
    println!("mean T at cover 0.05 / 0.4 / 0.95: {means:?}");
    assert!(
        means[0] - means[1] >= 0.1 && means[1] - means[2] >= 0.1,
        "{means:?}"
    );
    finish(&mut app);
}

/// Validation 11 (g): the shadow map exists before the first dispatch, transmittance 1
/// everywhere, `Rgba16Float`, 1024².
#[test]
#[ignore = "needs a GPU"]
fn the_shadow_map_starts_unshadowed() {
    let mut app = app(setup(WeatherPreset::Clear));
    app.update();
    finish(&mut app);
    let created = app.world().resource::<CreatedShadow>();
    assert_eq!(created.format, Some(TextureFormat::Rgba16Float));
    assert_eq!(created.size, (1024, 1024));
    assert_eq!(created.first_texels.len(), 1024 * 1024 * 8);
    for texel in created.first_texels.chunks_exact(8) {
        assert_eq!(support::half(u16::from_le_bytes([texel[0], texel[1]])), 1.0);
    }
}

/// Validation 14 (GPU part): switching the quality at run time keeps the weather map, shadow
/// map and noise (the same handles, the same area fraction) and keeps rendering.
#[test]
#[ignore = "needs a GPU"]
fn changing_the_quality_keeps_the_weather_and_shadow_maps() {
    let mut app = app(setup(WeatherPreset::PartlyCloudy));
    run(&mut app, 60);
    let weather = app.world().resource::<CloudWeatherMap>().map.clone();
    let shadow = app.world().resource::<CloudShadows>().map.clone();
    let before = covered_fraction(&mut app);
    for quality in [
        ElementsQuality::Low,
        ElementsQuality::Medium,
        ElementsQuality::Ultra,
        ElementsQuality::High,
    ] {
        *app.world_mut().resource_mut::<ElementsQuality>() = quality;
        run(&mut app, 30);
        assert_eq!(app.world().resource::<CloudWeatherMap>().map, weather);
        assert_eq!(app.world().resource::<CloudShadows>().map, shadow);
        let after = covered_fraction(&mut app);
        assert!(
            (after - before).abs() < 0.02,
            "{quality:?}: {before} -> {after}"
        );
    }
    finish(&mut app);
}

/// Validation 11 (f): the consumer mapping is the note's formula and lands on the texel the
/// shadow ray crosses `y = 0` at.
#[test]
fn the_shadow_lookup_follows_the_ray_toward_the_light() {
    let shadows = CloudShadows {
        map: Handle::default(),
        origin: Vec2::new(-4000.0, -3500.0),
        extent: 8000.0,
        light_direction: Vec3::new(0.5, 0.5, -0.3).normalize() * Vec3::new(1.0, 1.0, 1.0),
        camera_transmittance: 1.0,
    };
    // A sun at 30°: l = (cos 30° · d, sin 30°).
    let l = Vec3::new(0.866_025_4 * 0.6, 0.5, 0.866_025_4 * 0.8);
    let shadows = CloudShadows {
        light_direction: l,
        ..shadows
    };
    let p = Vec3::new(120.0, 80.0, -40.0);
    let uv = shadows.uv(p);
    let formula = (p.xz() - l.xz() * p.y / l.y - shadows.origin) / shadows.extent;
    assert!((uv - formula).abs().max_element() < 1e-6);
    // The ground point q the shadow of P falls on: P lies on the ray from q toward the light.
    let q = uv * shadows.extent + shadows.origin;
    let t = p.y / l.y;
    let along = Vec3::new(q.x, 0.0, q.y) + l * t;
    assert!((along - p).length() < 1e-3, "{along} vs {p}");
    // A point on the ground maps to itself.
    let ground = Vec3::new(10.0, 0.0, 20.0);
    let uv = shadows.uv(ground);
    assert!((uv * shadows.extent + shadows.origin - ground.xz()).length() < 1e-3);
}

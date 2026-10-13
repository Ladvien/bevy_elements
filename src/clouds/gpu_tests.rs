//! GPU tests of the cloud kernels that need the crate's internals: the CPU mirrors of
//! `model.rs` and `noise_mirror.rs`, the kernel inputs (`CloudDebug`) and the per-view textures.
//! The tests of the public behaviour are `tests/gpu_clouds.rs`.
//!
//! Needs a GPU: `cargo test --release --lib -- --ignored clouds`.

#![allow(clippy::chunks_exact_to_as_chunks)]

#[path = "../../tests/support/mod.rs"]
mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::Hdr;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;

use super::model::{self, MirrorParams};
use super::{CloudDebug, CloudGpu, CloudQuality, CloudWeatherMap};
use crate::climate::{WeatherDirector, WeatherPreset};
use crate::flash::LightningFlash;
use crate::gpu::noise_mirror::{cloud_base_texel, cloud_detail_texel};
use crate::{
    CivilDate, ElementsCamera, ElementsPlugins, ElementsQuality, LightningPlugin,
    PrecipitationPlugin, WorldClock,
};

#[derive(Clone)]
struct Case {
    preset: WeatherPreset,
    hour: f64,
    position: Vec3,
    look: Vec3,
    quality: ElementsQuality,
    debug: CloudDebug,
    /// Add a wall of 2 km × 300 m 500 m ahead (−Z) of the camera.
    wall: bool,
    /// Render into an unclamped `Rgba16Float` target with no tonemapping.
    linear_target: bool,
}

impl Case {
    fn sky(preset: WeatherPreset) -> Self {
        Self {
            preset,
            hour: 12.0,
            position: Vec3::new(0.0, 2.0, 0.0),
            look: Vec3::Y,
            quality: ElementsQuality::High,
            debug: CloudDebug::default(),
            wall: false,
            linear_target: false,
        }
    }
}

#[derive(Resource)]
struct Target(Handle<Image>);

fn app(case: Case) -> App {
    let mut app = App::new();
    // Clouds alone: random strikes would overwrite the tests' own `LightningFlash`, and rain
    // streaks would change the scene pixels the composite tests compare.
    let plugins = ElementsPlugins
        .build()
        .disable::<LightningPlugin>()
        .disable::<PrecipitationPlugin>();
    app.add_plugins((support::headless_plugins(), plugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(WorldClock {
            paused: true,
            ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(case.hour)
        })
        .insert_resource(case.quality)
        .insert_resource(case.debug.clone())
        .add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  mut director: ResMut<WeatherDirector>| {
                let target = if case.linear_target {
                    let mut image =
                        Image::new_target_texture(64, 64, TextureFormat::Rgba16Float, None);
                    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
                    let handle = images.add(image);
                    commands.insert_resource(Target(handle.clone()));
                    RenderTarget::Image(handle.into())
                } else {
                    support::image_target(&mut images)
                };
                let up = if case.look.abs().dot(Vec3::Y) > 0.99 {
                    Vec3::Z
                } else {
                    Vec3::Y
                };
                let mut camera = commands.spawn((
                    Camera3d::default(),
                    target,
                    ElementsCamera {
                        auto_exposure: false,
                    },
                    Transform::from_translation(case.position).looking_to(case.look, up),
                ));
                if case.linear_target {
                    camera.insert((Tonemapping::None, DebandDither::Disabled, Hdr));
                }
                if case.wall {
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(2000.0, 300.0, 10.0))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.6, 0.5, 0.4),
                            unlit: true,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 150.0, -500.0),
                    ));
                }
                director.force(case.preset, Duration::ZERO);
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

fn set_debug(app: &mut App, f: impl FnOnce(&mut CloudDebug)) {
    f(&mut app.world_mut().resource_mut::<CloudDebug>());
}

/// The march result texture, rgba as f32, row-major, with its size.
fn march_texture(app: &mut App) -> (Vec<[f32; 4]>, UVec2) {
    let gpu: CloudGpu = app.world().resource::<CloudGpu>().clone();
    let view = gpu.view.expect("the view textures exist");
    let low = UVec2::new(gpu.params.sizes.z, gpu.params.sizes.w);
    let bytes = support::read_texture(app, view.march, low.x as usize * 8, low.y as usize);
    (rgba16(&bytes), low)
}

fn history_texture(app: &mut App, index: usize) -> (Vec<[f32; 4]>, UVec2) {
    let gpu: CloudGpu = app.world().resource::<CloudGpu>().clone();
    let view = gpu.view.expect("the view textures exist");
    let size = view.size;
    let bytes = support::read_texture(
        app,
        view.history[index].clone(),
        size.x as usize * 8,
        size.y as usize,
    );
    (rgba16(&bytes), size)
}

/// Both history textures as they were in the same frame: this frame's and the previous one's.
fn history_pair(app: &mut App) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let gpu: CloudGpu = app.world().resource::<CloudGpu>().clone();
    let view = gpu.view.expect("the view textures exist");
    let (row, rows) = (view.size.x as usize * 8, view.size.y as usize);
    let mut both = support::read_textures(
        app,
        vec![
            (view.history[0].clone(), row, rows),
            (view.history[1].clone(), row, rows),
        ],
    );
    let b = rgba16(&both.pop().expect("two textures"));
    let a = rgba16(&both.pop().expect("two textures"));
    (a, b)
}

fn rgba16(bytes: &[u8]) -> Vec<[f32; 4]> {
    bytes
        .chunks_exact(8)
        .map(|t| [0, 2, 4, 6].map(|i| support::half(u16::from_le_bytes([t[i], t[i + 1]]))))
        .collect()
}

fn luminance(c: [f32; 4]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn centre(texels: &[[f32; 4]], size: UVec2) -> [f32; 4] {
    texels[(size.y / 2 * size.x + size.x / 2) as usize]
}

// ---------------------------------------------------------------------------------------------
// Validation 3
// ---------------------------------------------------------------------------------------------

/// Mean absolute difference over all channels between two texel planes of a volume.
fn plane_difference(texels: &[u8], n: usize, axis: usize, a: usize, b: usize) -> f32 {
    let at =
        |x: usize, y: usize, z: usize, c: usize| f32::from(texels[((z * n + y) * n + x) * 4 + c]);
    let mut total = 0.0;
    for u in 0..n {
        for v in 0..n {
            for c in 0..4 {
                let (pa, pb) = match axis {
                    0 => (at(a, u, v, c), at(b, u, v, c)),
                    1 => (at(u, a, v, c), at(u, b, v, c)),
                    _ => (at(u, v, a, c), at(u, v, b, c)),
                };
                total += (pa - pb).abs();
            }
        }
    }
    total / (n * n * 4) as f32
}

/// Validation 3: the generated volumes tile (no seam: the wrap-around difference is no larger
/// than 1.5× the difference between neighbours), match the CPU mirror texel for texel within
/// 3/255, and have the channel means the mirror has.
#[test]
#[ignore = "needs a GPU"]
fn the_noise_volumes_tile_and_match_the_cpu_mirror() {
    let mut app = app(Case::sky(WeatherPreset::PartlyCloudy));
    run(&mut app, 12);
    let gpu: CloudGpu = app.world().resource::<CloudGpu>().clone();
    let base = support::read_texture(&mut app, gpu.base.clone(), 128 * 4, 128 * 128);
    let detail = support::read_texture(&mut app, gpu.detail.clone(), 32 * 4, 32 * 32);
    finish(&mut app);

    for (name, texels, n) in [("base", &base, 128usize), ("detail", &detail, 32)] {
        for axis in 0..3 {
            let wrap = plane_difference(texels, n, axis, 0, n - 1);
            let interior = (0..n - 1)
                .map(|i| plane_difference(texels, n, axis, i, i + 1))
                .sum::<f32>()
                / (n - 1) as f32;
            println!("{name} axis {axis}: wrap {wrap:.3}, interior {interior:.3}");
            assert!(
                wrap <= 1.5 * interior,
                "{name} has a seam along axis {axis}: {wrap} vs {interior}"
            );
        }
    }

    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = |m: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 24) as usize % m
    };
    let mut worst = 0.0_f32;
    for _ in 0..4096 {
        let (x, y, z) = (next(128), next(128), next(128));
        let expect = cloud_base_texel(x as u32, y as u32, z as u32);
        let at = ((z * 128 + y) * 128 + x) * 4;
        for c in 0..4 {
            worst = worst.max((f32::from(base[at + c]) / 255.0 - expect[c]).abs());
        }
        let (x, y, z) = (next(32), next(32), next(32));
        let expect = cloud_detail_texel(x as u32, y as u32, z as u32);
        let at = ((z * 32 + y) * 32 + x) * 4;
        for c in 0..3 {
            worst = worst.max((f32::from(detail[at + c]) / 255.0 - expect[c]).abs());
        }
    }
    println!(
        "worst texel difference to the mirror: {:.2}/255",
        worst * 255.0
    );
    assert!(
        worst <= 3.0 / 255.0,
        "worst difference {} / 255",
        worst * 255.0
    );

    let mean_a = base.chunks_exact(4).map(|t| f32::from(t[3])).sum::<f32>()
        / (base.len() / 4) as f32
        / 255.0;
    let mean_b = detail.chunks_exact(4).map(|t| f32::from(t[2])).sum::<f32>()
        / (detail.len() / 4) as f32
        / 255.0;
    println!("channel means: base A {mean_a:.4}, detail B {mean_b:.4}");
    assert!((mean_a - 0.4987).abs() < 0.05, "base A mean {mean_a}");
    assert!((mean_b - 0.4764).abs() < 0.05, "detail B mean {mean_b}");
}

// ---------------------------------------------------------------------------------------------
// Validation 2 (drift offsets)
// ---------------------------------------------------------------------------------------------

/// Validation 2: at cloud drifts of 0, 7 and 31 km the weather map's area fraction is the cover
/// and every texel agrees with the CPU mirror of the field kernel (exact quantile threshold).
#[test]
#[ignore = "needs a GPU"]
fn the_weather_map_follows_the_drift_and_the_cpu_field() {
    let cover = 0.4;
    let mut case = Case::sky(WeatherPreset::Clear);
    case.debug.cover = Some(cover);
    let mut app = app(case);
    for drift in [[0.0_f32, 0.0], [7_000.0, 0.0], [31_000.0, 12_000.0]] {
        set_debug(&mut app, |d| d.drift = Some(Vec2::from(drift)));
        run(&mut app, 8);
        let map: CloudWeatherMap = app.world().resource::<CloudWeatherMap>().clone();
        let bytes = support::read_texture(&mut app, map.map.clone(), 512 * 4, 512);
        let covered = bytes.chunks_exact(4).filter(|t| t[0] >= 128).count() as f32 / 262_144.0;
        let mirror = MirrorParams::new(
            1500.0,
            3300.0,
            0.5,
            f64::from(cover),
            [f64::from(drift[0]), f64::from(drift[1])],
        );
        let mut worst = 0.0_f64;
        for j in (0..512).step_by(5) {
            for i in (0..512).step_by(5) {
                let xz = [
                    f64::from(map.origin.x) + (i as f64 + 0.5) * 62.5,
                    f64::from(map.origin.y) + (j as f64 + 0.5) * 62.5,
                ];
                let (r, dome) = mirror.weather_at(xz);
                let t = &bytes[(j * 512 + i) * 4..][..4];
                worst = worst
                    .max((f64::from(t[0]) / 255.0 - r).abs())
                    .max((f64::from(t[3]) / 255.0 - dome).abs());
            }
        }
        println!(
            "drift {drift:?}: fraction {covered:.4}, worst texel difference to the mirror {worst:.3}"
        );
        assert!(
            (covered - cover).abs() <= 0.02,
            "{covered} at drift {drift:?}"
        );
        assert!(
            worst <= 0.08,
            "mirror disagrees by {worst} at drift {drift:?}"
        );
    }
    finish(&mut app);
}

// ---------------------------------------------------------------------------------------------
// Validation 7 and 8
// ---------------------------------------------------------------------------------------------

/// Validation 7: through a uniform slab, looking straight up, the march transmits `e^{−σ d}` to
/// 1 %; an opaque slab leaves a finite transmittance in [0, 0.01].
#[test]
#[ignore = "needs a GPU"]
fn the_march_transmits_exp_minus_sigma_d_through_a_uniform_slab() {
    let mut case = Case::sky(WeatherPreset::Clear);
    case.debug = CloudDebug {
        mode: 1.0,
        sigma: 0.002,
        layer: Some((500.0, 1500.0)),
        ..default()
    };
    let mut app = app(case);
    for (sigma, expect) in [(0.002_f32, (-2.0_f32).exp()), (0.0004, (-0.4_f32).exp())] {
        set_debug(&mut app, |d| d.sigma = sigma);
        run(&mut app, 8);
        let (texels, size) = march_texture(&mut app);
        let t = centre(&texels, size)[3];
        println!("sigma {sigma}: T {t:.4}, expected {expect:.4}");
        assert!((t / expect - 1.0).abs() <= 0.01, "T {t}, expected {expect}");
    }
    set_debug(&mut app, |d| d.sigma = 0.05);
    run(&mut app, 8);
    let (texels, size) = march_texture(&mut app);
    let c = centre(&texels, size);
    println!("opaque slab: {c:?}");
    assert!(c.iter().all(|v| v.is_finite()));
    assert!((0.0..=0.01).contains(&c[3]), "T {}", c[3]);
    finish(&mut app);
}

/// Validation 8: the light the kernel finds at the layer is the transmittance of the Earth
/// atmosphere integrated along the spherical ray, and the planet shadows the layer once the
/// sun is lower than the layer's horizon dip.
#[test]
#[ignore = "needs a GPU"]
fn the_light_at_the_layer_is_the_atmospheric_transmittance_and_the_planet_shadow() {
    let mut case = Case::sky(WeatherPreset::Clear);
    case.debug = CloudDebug {
        mode: 2.0,
        layer: Some((1500.0, 3300.0)),
        light: Some(Vec3::Y),
        light_e: Some(Vec3::ONE),
        ..default()
    };
    let mut app = app(case);
    let sun = |elevation_deg: f32| {
        let e = elevation_deg.to_radians();
        Vec3::new(e.cos(), e.sin(), 0.0)
    };
    let read = |app: &mut App, base: f32, elevation_deg: f32| {
        set_debug(app, |d| {
            d.layer = Some((base, base + 1800.0));
            d.light = Some(sun(elevation_deg));
        });
        run(app, 8);
        let (texels, size) = march_texture(app);
        centre(&texels, size)
    };
    // The camera is 2 m up: the layer's base is the point straight above it.
    let altitude = |base: f32| f64::from(base);
    for (base, elevation, tolerance) in [
        (1500.0, 90.0, 0.03),
        (3300.0, 90.0, 0.03),
        (1500.0, 10.0, 0.20),
    ] {
        let got = read(&mut app, base, elevation);
        let mu = f64::from(elevation).to_radians().sin();
        let expect = model::bevy_optical_transmittance(altitude(base), mu);
        println!("h {base} sun {elevation}°: kernel {got:?}, integral {expect:?}");
        for c in 0..3 {
            assert!(
                (f64::from(got[c]) / expect[c] - 1.0).abs() <= tolerance,
                "channel {c}: {} vs {}",
                got[c],
                expect[c]
            );
        }
    }
    // Planet shadow: lit one degree below the horizon at 1.5 km (dip 1.24°), dark at −1.5°;
    // lit at −3° at 9.5 km (dip 3.13°), dark by −3.6°.
    assert!(read(&mut app, 1500.0, -1.0)[0] > 0.0);
    assert_eq!(read(&mut app, 1500.0, -1.5)[0], 0.0);
    assert!(read(&mut app, 9500.0, -3.0)[0] > 0.0);
    assert_eq!(read(&mut app, 9500.0, -3.6)[0], 0.0);
    finish(&mut app);
}

// ---------------------------------------------------------------------------------------------
// Validation 10
// ---------------------------------------------------------------------------------------------

fn bits(target: &[u8]) -> &[u8] {
    target
}

/// Validation 10: a pixel whose scene surface is nearer than the cloud entry is exactly the scene
/// colour (the composite adds nothing there), cloud-free sky is unchanged, and the cloud changes
/// the sky where there is cloud. The target is linear `Rgba16Float` with no tonemapping, and the
/// same frame is rendered with and without the composite (`CloudDebug::mode` 4).
#[test]
#[ignore = "needs a GPU"]
fn the_composite_leaves_nearer_surfaces_and_clear_sky_untouched() {
    let mut case = Case::sky(WeatherPreset::Overcast);
    case.look = Vec3::new(0.0, 0.364, -1.0).normalize();
    case.wall = true;
    case.linear_target = true;
    let mut app = app(case);
    // Past the 1 s smoothing of the light factor: the lights must not drift between the frames.
    run(&mut app, 700);
    let target = app.world().resource::<Target>().0.clone();
    let with = support::read_texture(&mut app, target.clone(), 64 * 8, 64);
    set_debug(&mut app, |d| d.mode = 4.0);
    run(&mut app, 8);
    let without = support::read_texture(&mut app, target, 64 * 8, 64);
    finish(&mut app);

    let a = rgba16(bits(&with));
    let b = rgba16(bits(&without));
    // Camera pitch 20°, vertical field of view 45°: the wall (elevation −0.2° to 31°) fills the
    // rows from about the screen's 36 % mark counted from the top down to row 58; the rows
    // below it see the sky under the horizon, which the composite lifts with the sky gain.
    let mut wall_identical = 0;
    let mut sky_changed = 0;
    for y in 0..64 {
        for x in 0..64 {
            let (pa, pb) = (a[y * 64 + x], b[y * 64 + x]);
            if (41..59).contains(&y) {
                // The volumetric fog jitters every frame: equal up to a half-float step or two.
                let delta = (0..4).map(|c| (pa[c] - pb[c]).abs()).fold(0.0, f32::max);
                assert!(
                    delta <= 0.01,
                    "wall pixel ({x}, {y}) changed by {delta}: {pa:?} vs {pb:?}"
                );
                wall_identical += 1;
            } else if y < 6 && pa != pb {
                sky_changed += 1;
            }
        }
    }
    println!(
        "{wall_identical} wall pixels identical, {sky_changed} sky pixels changed by the cloud"
    );
    assert!(wall_identical > 1000);
    assert!(
        sky_changed > 100,
        "the overcast deck should show in the sky"
    );
}

// ---------------------------------------------------------------------------------------------
// Finite values, temporal behaviour, flash
// ---------------------------------------------------------------------------------------------

/// Validation 16: no infinity or NaN, and no negative or above-one transmittance, in the march
/// result and the history, at every quality, in the clearest and the thickest weather.
#[test]
#[ignore = "needs a GPU"]
fn the_cloud_textures_stay_finite_at_every_quality() {
    for quality in [
        ElementsQuality::Low,
        ElementsQuality::Medium,
        ElementsQuality::High,
        ElementsQuality::Ultra,
    ] {
        for (preset, hour) in [
            (WeatherPreset::PartlyCloudy, 12.0),
            (WeatherPreset::Thunderstorm, 19.0),
            (WeatherPreset::Overcast, 5.0),
        ] {
            let mut case = Case::sky(preset);
            case.quality = quality;
            case.hour = hour;
            case.look = Vec3::new(1.0, 0.2, 0.3).normalize();
            let mut app = app(case);
            run(&mut app, 40);
            let window = CloudQuality::of(quality).window as usize;
            run(&mut app, 2 * window);
            let (march, _) = march_texture(&mut app);
            let (h0, _) = history_texture(&mut app, 0);
            let (h1, _) = history_texture(&mut app, 1);
            finish(&mut app);
            for (name, texels) in [("march", &march), ("history 0", &h0), ("history 1", &h1)] {
                for t in texels {
                    assert!(
                        t.iter().all(|v| v.is_finite())
                            && t[3] >= -1e-3
                            && t[3] <= 1.001
                            && t[..3].iter().all(|v| *v >= -1e-3),
                        "{name} at {quality:?}/{preset:?}: {t:?}"
                    );
                }
            }
            println!(
                "{quality:?} / {preset:?}: all {} texels finite",
                march.len() + h0.len() + h1.len()
            );
        }
    }
}

/// Validation 9 (static camera): with the camera, time and drift frozen the history changes
/// little between consecutive frames, after the accumulation window has run.
#[test]
#[ignore = "needs a GPU"]
fn a_still_camera_gives_a_steady_history() {
    for quality in [ElementsQuality::High, ElementsQuality::Low] {
        let mut case = Case::sky(WeatherPreset::PartlyCloudy);
        case.quality = quality;
        case.look = Vec3::new(1.0, 0.25, 0.4).normalize();
        case.debug.drift = Some(Vec2::new(3_000.0, 2_000.0));
        let mut app = app(case);
        let window = CloudQuality::of(quality).window as usize;
        run(&mut app, 40 + 6 * window);
        let (a, b) = history_pair(&mut app);
        finish(&mut app);
        let mean = a.iter().map(|t| luminance(*t)).sum::<f32>() / a.len() as f32;
        let rms = (a
            .iter()
            .zip(&b)
            .map(|(x, y)| (luminance(*x) - luminance(*y)).powi(2))
            .sum::<f32>()
            / a.len() as f32)
            .sqrt();
        println!(
            "{quality:?}: mean luminance {mean:.4}, consecutive-frame rms {rms:.5} ({:.2} %)",
            100.0 * rms / mean
        );
        assert!(mean > 0.0);
        assert!(rms / mean <= 0.10, "{quality:?}: {}", rms / mean);
    }
}

/// Validation 13: with no flash the cloud is what it was; a 7.96×10⁶ cd flash at the layer base
/// lights the cloud over it at dusk by at least 50×, and changes the exposed noon sky by under
/// 1 %.
#[test]
#[ignore = "needs a GPU"]
fn a_flash_lights_the_cloud_at_dusk_and_not_at_noon() {
    for (hour, minimum_ratio, maximum_change) in [(21.0, 50.0, f32::MAX), (12.0, 0.0, 0.02)] {
        let mut case = Case::sky(WeatherPreset::Thunderstorm);
        case.hour = hour;
        let mut app = app(case);
        app.insert_resource(LightningFlash::default());
        run(&mut app, 80);
        let centre_mean = |texels: &[[f32; 4]], size: UVec2| {
            let mut sum = 0.0;
            let mut n = 0.0;
            for y in size.y / 2 - 8..size.y / 2 + 8 {
                for x in size.x / 2 - 8..size.x / 2 + 8 {
                    sum += luminance(texels[(y * size.x + x) as usize]);
                    n += 1.0;
                }
            }
            sum / n
        };
        let (off, size) = march_texture(&mut app);
        let dark = centre_mean(&off, size);
        *app.world_mut().resource_mut::<LightningFlash>() = LightningFlash {
            position: Vec3::new(0.0, 1500.0, 0.0),
            intensity_cd: 7.96e6,
        };
        run(&mut app, 8);
        let (on, _) = march_texture(&mut app);
        finish(&mut app);
        let lit = centre_mean(&on, size);
        println!(
            "hour {hour}: no flash {dark:.5}, flash {lit:.5}, ratio {:.1}",
            lit / dark.max(1e-9)
        );
        assert!(lit / dark.max(1e-9) >= minimum_ratio);
        assert!((lit - dark).abs() / dark.max(1e-9) <= maximum_change);
    }
}

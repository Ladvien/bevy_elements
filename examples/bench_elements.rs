//! GPU cost of every domain, per pass, printed every 120 frames.
//!
//! `cargo run --release --example bench_elements`
//!
//! Uses Bevy's `RenderDiagnosticsPlugin`: GPU timestamps on Vulkan and DX12, CPU-side timing only
//! elsewhere. Set `ELEMENTS_QUALITY=low|medium|high|ultra` (default high),
//! `ELEMENTS_RESOLUTION=WxH` (default 1920x1080), `ELEMENTS_PRESET=Thunderstorm` (default `Fog`;
//! the precipitation passes only run in a wet preset) and `ELEMENTS_BLOCKERS=256` (spawn that
//! many precipitation blockers around the camera).
//!
//! The timed passes are compute and do not depend on the resolution, but the GPU's clocks do: a
//! lightly loaded GPU idles in a low power state and every pass reads several times slower. Check
//! the clocks while it runs (`nvidia-smi --query-gpu=pstate,clocks.gr --format=csv`) and raise the
//! resolution until the GPU holds its full clock; the README numbers were taken that way.

mod common;

use bevy::diagnostic::{DiagnosticPath, DiagnosticsStore};
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::window::{PresentMode, WindowResolution};
use bevy_elements::*;
use common::{ExampleHarness, camera, test_scene};

/// The crate's GPU passes, by diagnostic span name.
const PASSES: [&str; 4] = [
    "elements_wind",
    "elements_fog",
    "elements_precip_sim",
    "elements_precip_draw",
];

fn main() {
    let (width, height) = std::env::var("ELEMENTS_RESOLUTION")
        .ok()
        .and_then(|r| {
            let (w, h) = r.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1920, 1080));
    let quality = match std::env::var("ELEMENTS_QUALITY").as_deref() {
        Ok("low") => ElementsQuality::Low,
        Ok("medium") => ElementsQuality::Medium,
        Ok("ultra") => ElementsQuality::Ultra,
        _ => ElementsQuality::High,
    };
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            RenderDiagnosticsPlugin,
            ElementsPlugins,
            ExampleHarness,
        ))
        .insert_resource(quality)
        .insert_resource(
            WorldClock::new(CivilDate::new(2026, 6, 21))
                .at_hour(16.0)
                .with_time_scale(60.0),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, report)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut director: ResMut<WeatherDirector>,
) {
    commands.spawn(camera(
        Vec3::new(0.0, 12.0, 0.0),
        Vec3::new(150.0, 10.0, 80.0),
    ));
    test_scene(&mut commands, &mut meshes, &mut materials);
    let preset = std::env::var("ELEMENTS_PRESET")
        .ok()
        .and_then(|name| {
            WeatherPreset::ALL
                .into_iter()
                .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
        })
        .unwrap_or(WeatherPreset::Fog);
    director.force(preset, std::time::Duration::ZERO);
    // Roofs scattered through the particle box: each particle tests every uploaded blocker.
    let blockers: usize = std::env::var("ELEMENTS_BLOCKERS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    for i in 0..blockers {
        let angle = i as f32 * 2.399_963;
        let radius = 4.0 + 26.0 * ((i as f32 + 0.5) / blockers as f32).sqrt();
        commands.spawn((
            PrecipitationBlocker {
                half_extents: Vec3::new(2.0, 0.1, 2.0),
            },
            Transform::from_xyz(
                angle.cos() * radius,
                3.0 + (i % 7) as f32 * 1.5,
                angle.sin() * radius,
            ),
        ));
    }
}

fn report(store: Res<DiagnosticsStore>, quality: Res<ElementsQuality>, mut frame: Local<u32>) {
    *frame += 1;
    if !(*frame).is_multiple_of(120) {
        return;
    }
    // The minimum over the history is the full-clock cost: a pass can only read slower than
    // that, when the GPU has dropped to a low power state; the average mixes both.
    let mut total = 0.0;
    let mut line = format!("{:?}:", *quality);
    for pass in PASSES {
        let gpu = DiagnosticPath::new(format!("render/{pass}/elapsed_gpu"));
        let cpu = DiagnosticPath::new(format!("render/{pass}/elapsed_cpu"));
        match store.get(&gpu) {
            Some(d) => {
                let min = d.values().copied().fold(f64::INFINITY, f64::min);
                let mean = d.average().unwrap_or(f64::NAN);
                total += min;
                line += &format!("  {pass} min {min:.3} / mean {mean:.3} ms GPU");
            }
            None => match store.get(&cpu).and_then(|d| d.smoothed()) {
                Some(ms) => line += &format!("  {pass} {ms:.3} ms CPU (no GPU timestamps)"),
                None => line += &format!("  {pass} -"),
            },
        }
    }
    println!("{line}  | total min {total:.3} ms GPU");
}

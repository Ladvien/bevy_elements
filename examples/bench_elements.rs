//! GPU cost of every domain, per pass, printed every 120 frames.
//!
//! `cargo run --release --example bench_elements`
//!
//! Uses Bevy's `RenderDiagnosticsPlugin`: GPU timestamps on Vulkan and DX12, CPU-side timing only
//! elsewhere. Set `ELEMENTS_QUALITY=low|medium|high|ultra` (default high),
//! `ELEMENTS_RESOLUTION=WxH` (default 1920x1080), `ELEMENTS_PRESET=Thunderstorm` (default `Fog`;
//! the precipitation passes only run in a wet preset) and `ELEMENTS_BLOCKERS=256` (spawn that
//! many precipitation blockers around the camera). `ELEMENTS_TERRAIN=hills|mountains|none`
//! (default `none`: the flat test scene) draws a terrain instead; its generator pass
//! (`elements_terrain`, once at spawn) is listed but not counted in the per-frame total, and the
//! main and depth passes it is drawn in are printed on their own line.
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
use common::{ExampleHarness, camera, stand_on_ground, test_scene};

/// The crate's GPU passes, by diagnostic span name.
const PASSES: [&str; 11] = [
    "elements_terrain",
    "elements_wind",
    "elements_fog",
    "elements_clouds_noise",
    "elements_clouds_weather",
    "elements_clouds_shadow",
    "elements_clouds_march",
    "elements_clouds_resolve",
    "elements_clouds_composite",
    "elements_precip_sim",
    "elements_precip_draw",
];

/// Bevy's own passes the terrain is drawn in, printed on a separate line.
const SCENE_PASSES: [&str; 2] = ["main_opaque_pass_3d", "prepass"];

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
    let mut eye = commands.spawn(camera(
        Vec3::new(0.0, 30.0, 0.0),
        // Looking 20° above the horizon: about half the frame is cloud.
        Vec3::new(150.0, 91.9, 80.0),
    ));
    match std::env::var("ELEMENTS_TERRAIN").as_deref() {
        Ok("hills") => {
            eye.insert(stand_on_ground(30.0));
            commands.spawn(Terrain::procedural(ProceduralTerrain::hills(42)));
        }
        Ok("mountains") => {
            eye.insert(stand_on_ground(30.0));
            commands.spawn(Terrain::procedural(ProceduralTerrain::mountains(42)));
        }
        _ => test_scene(&mut commands, &mut meshes, &mut materials),
    }
    // PartlyCloudy is the clouds note's bench scene; `ELEMENTS_PRESET=Thunderstorm` saturates the
    // precipitation particles.
    let preset = std::env::var("ELEMENTS_PRESET")
        .ok()
        .and_then(|name| {
            WeatherPreset::ALL
                .into_iter()
                .find(|p| format!("{p:?}").eq_ignore_ascii_case(&name))
        })
        .unwrap_or(WeatherPreset::PartlyCloudy);
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

fn report(
    store: Res<DiagnosticsStore>,
    quality: Res<ElementsQuality>,
    mut frame: Local<u32>,
    mut lowest: Local<[f64; PASSES.len()]>,
    mut lowest_scene: Local<[f64; SCENE_PASSES.len()]>,
) {
    *frame += 1;
    // The minimum over the whole run is the full-clock cost: a pass can only read slower than
    // that, when the GPU has dropped to a low power state; the history window and the average mix
    // both.
    for (slot, pass) in lowest.iter_mut().zip(PASSES) {
        let gpu = DiagnosticPath::new(format!("render/{pass}/elapsed_gpu"));
        if let Some(now) = store.get(&gpu).and_then(|d| d.value()) {
            *slot = if *slot > 0.0 { slot.min(now) } else { now };
        }
    }
    for (slot, pass) in lowest_scene.iter_mut().zip(SCENE_PASSES) {
        let gpu = DiagnosticPath::new(format!("render/{pass}/elapsed_gpu"));
        if let Some(now) = store.get(&gpu).and_then(|d| d.value()) {
            *slot = if *slot > 0.0 { slot.min(now) } else { now };
        }
    }
    if !(*frame).is_multiple_of(120) {
        return;
    }
    let mut total = 0.0;
    let mut clouds = 0.0;
    let mut line = format!("{:?}:", *quality);
    for pass in PASSES {
        let gpu = DiagnosticPath::new(format!("render/{pass}/elapsed_gpu"));
        let cpu = DiagnosticPath::new(format!("render/{pass}/elapsed_cpu"));
        match store.get(&gpu) {
            Some(d) => {
                let min = lowest[PASSES.iter().position(|p| *p == pass).unwrap_or(0)];
                let mean = d.average().unwrap_or(f64::NAN);
                // The noise volumes are generated once, in the first frames: not a per-frame cost.
                if !pass.ends_with("_noise") && pass != "elements_terrain" {
                    total += min;
                    if pass.starts_with("elements_clouds") {
                        clouds += min;
                    }
                }
                line += &format!("  {pass} min {min:.3} / mean {mean:.3} ms GPU");
            }
            None => match store.get(&cpu).and_then(|d| d.smoothed()) {
                Some(ms) => line += &format!("  {pass} {ms:.3} ms CPU (no GPU timestamps)"),
                None => line += &format!("  {pass} -"),
            },
        }
    }
    println!("{line}  | total min {total:.3} ms GPU, clouds min {clouds:.3} ms GPU");
    let mut scene = String::from("scene:");
    for pass in SCENE_PASSES {
        let gpu = DiagnosticPath::new(format!("render/{pass}/elapsed_gpu"));
        match store.get(&gpu) {
            Some(d) => {
                let min = lowest_scene[SCENE_PASSES.iter().position(|p| *p == pass).unwrap_or(0)];
                scene += &format!(
                    "  {pass} min {min:.3} / mean {:.3} ms GPU",
                    d.average().unwrap_or(f64::NAN)
                );
            }
            None => scene += &format!("  {pass} -"),
        }
    }
    println!("{scene}");
}

//! The heat haze on a real GPU: a campfire in front of a checkered wall bends the wall behind it
//! and nothing else (`docs/research/heat-haze.md`, Validation targets 2–6).
//!
//! The scene is unlit and has no sky or clouds, so two runs of it agree bit for bit and any
//! difference is the haze's. Needs a GPU:
//! `cargo test --release --test gpu_heat_haze -- --ignored --test-threads=1`. Set
//! `ELEMENTS_TEST_DUMP=/path/prefix` to write each frame as a binary PPM.

mod support;

use std::sync::Mutex;
use std::time::Duration;

use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{PerspectiveProjection, Projection, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const DT: f64 = 1.0 / 60.0;
const SIZE: u32 = 512;
const FOV_DEG: f32 = 30.0;
const EYE: Vec3 = Vec3::new(0.0, 1.5, 10.0);
const LOOK: Vec3 = Vec3::new(0.0, 2.0, 0.0);
/// The pole between the eye and the fire: centre, half extents.
const POLE: (Vec3, Vec3) = (Vec3::new(0.35, 2.0, 4.0), Vec3::new(0.05, 2.0, 0.05));
const POLE_RED: f32 = 0.9;

#[derive(Resource)]
struct Target(Handle<Image>);

#[derive(Resource, Clone, Copy)]
struct Scene {
    /// Where the fire stands, if there is one.
    fire: Option<Vec3>,
}

/// A black-and-white checker, `cells` squares a side.
fn checker(cells: u32) -> Image {
    let texels = 8 * cells;
    let mut data = Vec::with_capacity((texels * texels * 4) as usize);
    for y in 0..texels {
        for x in 0..texels {
            let v = if (x / 8 + y / 8) % 2 == 0 { 255 } else { 20 };
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    Image::new(
        Extent3d {
            width: texels,
            height: texels,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        Tonemapping::None,
        DebandDither::Disabled,
        Transform::from_translation(EYE).looking_at(LOOK, Vec3::Y),
    ));
    // A checkered wall 30 m behind the fire (0.5 m squares) and a checkered floor (1 m squares).
    let wall = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(checker(80))),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(40.0, 40.0))),
        MeshMaterial3d(wall),
        Transform::from_xyz(0.0, 10.0, -30.0),
    ));
    let floor = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(checker(80))),
        base_color: Color::srgb(0.5, 0.5, 0.5),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(floor),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(POLE.1 * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(POLE_RED, 0.0, 0.0),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(POLE.0),
    ));
    if let Some(at) = scene.fire {
        commands.spawn((
            HeatSource {
                power_kw: 50.0,
                radius_m: 0.3,
            },
            Transform::from_translation(at),
        ));
    }
}

fn build(scene: Scene, gain: f32) -> App {
    let mut app = App::new();
    app.insert_resource(scene)
        .add_plugins((support::headless_plugins(), HeatHazePlugin))
        .insert_resource(HeatHazeSettings { gain })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            DT,
        )))
        .add_systems(Startup, setup);
    support::finish(&mut app);
    app
}

/// The linear colour of every pixel, row-major.
type Frame = Vec<Vec3>;

fn read(app: &mut App, dump: &str) -> Frame {
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(app, image, SIZE as usize * 8, SIZE as usize);
    let frame: Frame = (0..(SIZE * SIZE) as usize)
        .map(|k| {
            let h = |o: usize| {
                support::half(u16::from_le_bytes([bytes[k * 8 + o], bytes[k * 8 + o + 1]]))
            };
            Vec3::new(h(0), h(2), h(4))
        })
        .collect();
    if let Ok(base) = std::env::var("ELEMENTS_TEST_DUMP") {
        let mut out = format!("P6 {SIZE} {SIZE} 255\n").into_bytes();
        for p in &frame {
            for c in 0..3 {
                out.push((p[c].clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8);
            }
        }
        let _ = std::fs::write(format!("{base}-{dump}.ppm"), out);
    }
    frame
}

/// Render `frames` updates of a scene and read it; then, if `again` > 0, that many more and read
/// it again.
fn render(scene: Scene, gain: f32, frames: u32, again: u32, dump: &str) -> (Frame, Frame) {
    let mut app = build(scene, gain);
    for _ in 0..frames {
        app.update();
    }
    let first = read(&mut app, &format!("{dump}-a"));
    let second = if again > 0 {
        for _ in 0..again {
            app.update();
        }
        read(&mut app, &format!("{dump}-b"))
    } else {
        Vec::new()
    };
    app.world_mut().write_message(AppExit::Success);
    (first, second)
}

/// Pixel `(x, y)` of the point `p` through the test camera.
fn project(p: Vec3) -> Vec2 {
    let view = Transform::from_translation(EYE).looking_at(LOOK, Vec3::Y);
    let local = view.compute_affine().inverse().transform_point3(p);
    let f = 1.0 / (FOV_DEG.to_radians() * 0.5).tan();
    let ndc = Vec2::new(local.x * f / -local.z, local.y * f / -local.z);
    Vec2::new(
        (ndc.x * 0.5 + 0.5) * SIZE as f32,
        (0.5 - ndc.y * 0.5) * SIZE as f32,
    )
}

fn at(frame: &Frame, x: u32, y: u32) -> Vec3 {
    frame[(y * SIZE + x) as usize]
}

/// The mean absolute difference of two frames over a pixel rectangle, and how many pixels differ.
fn difference(a: &Frame, b: &Frame, lo: UVec2, hi: UVec2) -> (f32, usize) {
    let (mut sum, mut changed, mut n) = (0.0, 0usize, 0usize);
    for y in lo.y..hi.y {
        for x in lo.x..hi.x {
            let d = (at(a, x, y) - at(b, x, y)).abs().max_element();
            sum += d;
            changed += usize::from(d > 0.0);
            n += 1;
        }
    }
    (sum / n.max(1) as f32, changed)
}

const FIRE: Vec3 = Vec3::ZERO;

/// Validation targets 3, 4, 5 and 6: with a campfire in view the wall behind it shifts above the
/// fire, the frame away from the plume is bit-identical to the frame without the haze, the
/// pattern moves from frame to frame, and the red pole in front of the plume keeps its own pixels
/// and is not pulled into the haze.
#[test]
#[ignore = "needs a GPU"]
fn gpu_a_campfire_bends_only_what_is_behind_it() {
    let _lock = ONE_AT_A_TIME.lock();
    let scene = Scene { fire: Some(FIRE) };
    let (plain, _) = render(scene, 0.0, 40, 0, "plain");
    let (hazy, later) = render(scene, 10.0, 40, 6, "hazy");

    // The plume over the fire, from 0.3 m to 4 m up, about a metre across.
    let base = project(FIRE + Vec3::Y * 0.3);
    let top = project(FIRE + Vec3::Y * 4.0);
    let half = (project(FIRE + Vec3::new(0.6, 2.0, 0.0)).x - project(FIRE + Vec3::Y * 2.0).x)
        .abs()
        .max(4.0);
    let lo = UVec2::new((base.x - half) as u32, top.y.max(0.0) as u32);
    let hi = UVec2::new((base.x + half) as u32, base.y as u32);
    let (mean, changed) = difference(&plain, &hazy, lo, hi);
    let area = ((hi.x - lo.x) * (hi.y - lo.y)) as usize;
    assert!(
        mean > 0.02 && changed > area / 4,
        "the wall behind the plume barely moves: mean {mean}, {changed} of {area} pixels"
    );

    // Away from the plume (more than 3 plume widths and 48 px to the side) nothing changes.
    let margin = (3.0 * half + 52.0) as u32;
    let left = (base.x as u32).saturating_sub(margin);
    let (_, outside_left) = difference(&plain, &hazy, UVec2::ZERO, UVec2::new(left, SIZE));
    let right = (base.x as u32 + margin).min(SIZE);
    let (_, outside_right) =
        difference(&plain, &hazy, UVec2::new(right, 0), UVec2::new(SIZE, SIZE));
    assert_eq!(
        outside_left + outside_right,
        0,
        "pixels away from the plume changed"
    );

    // It moves: a tenth of a second later the plume region differs again.
    let (moved, _) = difference(&hazy, &later, lo, hi);
    assert!(moved > 0.01, "the haze is frozen: {moved}");

    // The pole in front of the plume: its pixels unchanged, and no red outside it (dilated by one
    // pixel for the bilinear sample at its silhouette).
    let red = |c: Vec3| c.x > 0.5 && c.y < 0.1 && c.z < 0.1;
    let mut pole_pixels = 0;
    for y in 1..SIZE - 1 {
        for x in 1..SIZE - 1 {
            let was = red(at(&plain, x, y));
            if was {
                pole_pixels += 1;
                assert_eq!(at(&plain, x, y), at(&hazy, x, y), "pole pixel ({x}, {y})");
            }
            let near_pole = (-1i32..=1).any(|dy| {
                (-1i32..=1)
                    .any(|dx| red(at(&plain, (x as i32 + dx) as u32, (y as i32 + dy) as u32)))
            });
            if !near_pole {
                assert!(
                    at(&hazy, x, y).x - at(&hazy, x, y).y < 0.3,
                    "the pole is pulled into the haze at ({x}, {y}): {}",
                    at(&hazy, x, y)
                );
            }
        }
    }
    assert!(pole_pixels > 100, "the pole is not in frame: {pole_pixels}");
}

/// Validation target 2: with no heat source, with one behind the camera, and with the gain at 0
/// the pass does not run, and the frames agree bit for bit.
#[test]
#[ignore = "needs a GPU"]
fn gpu_no_plume_on_screen_draws_nothing() {
    let _lock = ONE_AT_A_TIME.lock();
    let (none, _) = render(Scene { fire: None }, 10.0, 30, 0, "none");
    let (behind, _) = render(
        Scene {
            fire: Some(EYE + Vec3::new(0.0, -1.5, 15.0)),
        },
        10.0,
        30,
        0,
        "behind",
    );
    let (off, _) = render(Scene { fire: Some(FIRE) }, 0.0, 30, 0, "off");
    let all = (UVec2::ZERO, UVec2::splat(SIZE));
    assert_eq!(difference(&none, &behind, all.0, all.1).1, 0);
    assert_eq!(difference(&none, &off, all.0, all.1).1, 0);
}

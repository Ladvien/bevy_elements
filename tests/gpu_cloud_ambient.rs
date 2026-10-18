//! The clouds take the colour of the light that fills them (`docs/research/clouds.md`
//! Deviation 30, Validation 22; `docs/research/cloud-lighting.md` Deviation 3): the sunlight
//! their diffusion carries and the sky's ambient in the sky's colour. The shaded deck overhead
//! under `Overcast` is read from a linear `Rgba16Float` target with no tonemapping, with the
//! camera's atmosphere environment map (the sky the ambient reads) and without it (the neutral
//! white ambient).
//!
//! Needs a GPU: `cargo test --test gpu_cloud_ambient -- --ignored`.

mod support;

use std::time::Duration;

use bevy::app::AppExit;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::light::AtmosphereEnvironmentMapLight;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const SIZE: u32 = 32;

#[derive(Resource)]
struct Target(Handle<Image>);

#[derive(Component)]
struct Deck;

/// The mean linear colour of the deck straight overhead under `Overcast` at `hour` on the June
/// solstice (lat 45° N), with the camera's environment map or without it, through `projection`.
fn deck(hour: f64, sky_probe: bool, projection: Projection) -> Vec3 {
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
                    ElementsCamera {
                        auto_exposure: false,
                    },
                    Tonemapping::None,
                    DebandDither::Disabled,
                    Hdr,
                    Deck,
                    projection.clone(),
                    Transform::from_xyz(0.0, 2.0, 0.0).looking_to(Vec3::Y, Vec3::Z),
                ));
                director.force(WeatherPreset::Overcast, Duration::ZERO);
            },
        );
    if !sky_probe {
        app.add_systems(
            PostStartup,
            |mut commands: Commands, cameras: Query<Entity, With<Deck>>| {
                for camera in &cameras {
                    commands
                        .entity(camera)
                        .remove::<AtmosphereEnvironmentMapLight>();
                }
            },
        );
    }
    support::finish(&mut app);
    // The deck accumulates over 4 frames; the weather map and shadow settle well before 200.
    for _ in 0..200 {
        app.update();
    }
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(&mut app, image, SIZE as usize * 8, SIZE as usize);
    let mut sum = Vec3::ZERO;
    for texel in bytes.as_chunks::<8>().0 {
        let c = |k: usize| support::half(u16::from_le_bytes([texel[k], texel[k + 1]]));
        sum += Vec3::new(c(0), c(2), c(4));
    }
    app.world_mut().write_message(AppExit::Success);
    app.update();
    sum / (SIZE * SIZE) as f32
}

/// Blue over red of a colour.
fn blueness(c: Vec3) -> f32 {
    c.z / c.x.max(1e-9)
}

/// Red over green of a colour: the purple light of civil twilight raises it.
fn purpleness(c: Vec3) -> f32 {
    c.x / c.y.max(1e-9)
}

/// In civil twilight (20:12 on the solstice) the purple light makes the deck purpler than the
/// neutral ambient leaves it: with the sun's light off the clouds, the diffuse light is the
/// sky's. At noon the inside of a thick deck is lit mostly by the sunlight its diffusion
/// carries, so the sky's ambient makes it bluer than the neutral deck but well short of an
/// ambient alone (1.27×, a blue deck). Measured: red/green 1.26 against 1.02 at dusk; blue/red
/// 0.892 against 0.797 at noon (1.12×; 1.05× when the ambient also stood in for the scattered
/// sun). The build before Deviation 30 gives the same deck with and without the map and fails
/// both.
#[test]
#[ignore = "needs a GPU"]
fn the_deck_takes_the_colour_of_its_light() {
    let perspective = Projection::default;
    let (noon, noon_neutral) = (
        deck(12.0, true, perspective()),
        deck(12.0, false, perspective()),
    );
    let (dusk, dusk_neutral) = (
        deck(20.2, true, perspective()),
        deck(20.2, false, perspective()),
    );
    println!("noon {noon} (neutral {noon_neutral}); dusk {dusk} (neutral {dusk_neutral})");
    assert!(
        purpleness(dusk) > purpleness(dusk_neutral) * 1.1,
        "dusk: {dusk} against {dusk_neutral}"
    );
    let ratio = blueness(noon) / blueness(noon_neutral);
    assert!(
        ratio > 1.0 && ratio < 1.2,
        "noon: {noon} against {noon_neutral} (blue/red ratio {ratio})"
    );
}

/// Issue #7 (`view-anchor.md` Validation 5): an orthographic camera looking straight up sees the
/// same overcast deck as a perspective one. Its rays are parallel, each from its own point on the
/// near plane; a march that still built them from one eye point would draw a fan of rays through
/// a 200 m wide pinhole, or none.
#[test]
#[ignore = "needs a GPU"]
fn an_orthographic_camera_sees_the_deck_a_perspective_one_does() {
    let perspective = deck(12.0, true, Projection::default());
    let orthographic = deck(
        12.0,
        true,
        Projection::from(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 200.0,
            },
            ..OrthographicProjection::default_3d()
        }),
    );
    println!("perspective {perspective}, orthographic {orthographic}");
    let ratio = luminance(orthographic) / luminance(perspective);
    assert!(
        (0.85..=1.15).contains(&ratio),
        "orthographic {orthographic} against perspective {perspective} ({ratio})"
    );
}

/// Relative luminance of a linear colour.
fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

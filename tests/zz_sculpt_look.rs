mod support;
use std::time::Duration;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::time::TimeUpdateStrategy;
use bevy_elements::*;

const W: u32 = 768;
const H: u32 = 512;
#[derive(Resource)] struct Target(Handle<Image>);
#[derive(Resource)] struct Cam(Transform);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, cam: Res<Cam>, mut director: ResMut<WeatherDirector>) {
    let mut t = Image::new_target_texture(W, H, TextureFormat::Rgba16Float, None);
    t.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(t);
    commands.insert_resource(Target(image.clone()));
    commands.spawn((Camera3d::default(), RenderTarget::Image(image.into()), ElementsCamera::default(), cam.0));
    commands.spawn(Terrain::flat(0.0).with_extent(Vec2::splat(512.0)).with_resolution(UVec2::splat(256)));
    director.force(WeatherPreset::Clear, Duration::ZERO);
}

fn shot(name: &str, hour: f64, cam: Transform) {
    let mut app = App::new();
    app.insert_resource(Cam(cam));
    app.add_plugins((support::headless_plugins(), ElementsPlugins))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
        .insert_resource(ElementsQuality::Ultra)
        .insert_resource(WorldClock { paused: true, ..WorldClock::new(CivilDate::new(2026, 6, 21)).at_hour(hour) })
        .add_systems(Startup, setup);
    support::finish(&mut app);
    for i in 0..150 {
        if i == 40 {
            let w = app.world_mut();
            w.write_message(SculptTerrain::new(Vec2::ZERO, 70.0, SculptKind::Raise { amount_m: 28.0 }).with_hardness(0.2));
            w.write_message(SculptTerrain::new(Vec2::new(130.0, 20.0), 35.0, SculptKind::Raise { amount_m: 18.0 }).with_hardness(0.95));
            w.write_message(SculptTerrain::new(Vec2::new(-130.0, 10.0), 30.0, SculptKind::Lower { amount_m: 10.0 }).with_hardness(0.6));
            w.write_message(SculptTerrain::new(Vec2::new(0.0, 120.0), 50.0, SculptKind::Noise { amplitude_m: 6.0, feature_m: 12.0, seed: 3 }).with_hardness(0.3));
        }
        app.update();
    }
    let image = app.world().resource::<Target>().0.clone();
    let bytes = support::read_texture(&mut app, image, W as usize * 8, H as usize);
    let mut out = format!("P6 {W} {H} 255\n").into_bytes();
    for px in bytes.as_chunks::<8>().0.iter().take((W * H) as usize) {
        for c in 0..3 {
            let v = support::half(u16::from_le_bytes([px[2 * c], px[2 * c + 1]]));
            out.push((v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8);
        }
    }
    let _ = std::fs::write(format!("/tmp/i99/{name}.ppm"), out);
    app.world_mut().write_message(bevy::app::AppExit::Success);
}

#[test]
#[ignore]
fn look() {
    std::fs::create_dir_all("/tmp/i99").ok();
    let aerial = Transform::from_xyz(0.0, 140.0, -260.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y);
    let low = Transform::from_xyz(-20.0, 6.0, -190.0).looking_at(Vec3::new(40.0, 20.0, 0.0), Vec3::Y);
    shot("aerial-noon", 12.0, aerial);
    shot("aerial-dusk", 18.6, aerial);
    shot("grazing-dusk", 18.6, low);
    shot("grazing-morn", 7.0, low);
}

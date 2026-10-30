//! A terrain snapshot becomes a height map and a terrain that redraws it. See
//! `docs/research/terrain-heightfield-save.md` (Validation targets).

use std::sync::Arc;

use bevy::asset::Assets;
use bevy::image::Image;
use bevy::math::{UVec2, Vec2, Vec3};
use bevy::render::render_resource::TextureFormat;
use bevy_elements::*;

fn snapshot(surface: Vec<f32>, size: UVec2) -> TerrainSnapshot {
    TerrainSnapshot {
        grid: TerrainGrid {
            origin: Vec2::new(-32.0, 16.0),
            cell_m: 2.0,
            size,
            y0: 7.0,
        },
        sediment: Arc::from(vec![0.0; surface.len()]),
        surface: Arc::from(surface),
        version: 3,
    }
}

fn texels(image: &Image) -> Vec<f32> {
    let data = image.data.as_ref().expect("the image holds its data");
    bytemuck::pod_collect_to_vec::<u8, f32>(data)
}

#[test]
fn the_height_map_decodes_to_the_surface_with_the_range_it_returns() {
    let size = UVec2::new(8, 4);
    let surface: Vec<f32> = (0..32).map(|i| -3.0 + 0.5 * i as f32).collect();
    let (image, range) = snapshot(surface.clone(), size).to_heightmap();
    assert_eq!(image.texture_descriptor.format, TextureFormat::R32Float);
    assert_eq!(
        (image.width(), image.height()),
        (size.x, size.y),
        "one texel per cell"
    );
    assert_eq!(range, -3.0..12.5);
    for (texel, h) in texels(&image).into_iter().zip(surface) {
        let decoded = range.start + texel * (range.end - range.start);
        assert!((decoded - h).abs() < 1e-5, "{decoded} against {h}");
    }
}

#[test]
fn level_ground_has_a_range_a_height_map_can_use() {
    let (image, range) = snapshot(vec![4.0; 16], UVec2::splat(4)).to_heightmap();
    assert_eq!(range, 4.0..5.0);
    assert!(texels(&image).iter().all(|&t| t == 0.0));
}

#[test]
fn the_respawned_terrain_keeps_the_grids_extent_resolution_and_place() {
    let snap = snapshot(vec![0.0; 64 * 32], UVec2::new(64, 32));
    let mut images = Assets::<Image>::default();
    let terrain = snap.terrain(&mut images);
    assert_eq!(terrain.extent_m, Vec2::new(128.0, 64.0));
    assert_eq!(terrain.resolution, UVec2::new(64, 32));
    assert_eq!(images.len(), 1);
    assert!(matches!(terrain.source, TerrainSource::Heightmap { .. }));
    assert_eq!(snap.centre_m(), Vec3::new(32.0, 7.0, 48.0));
}

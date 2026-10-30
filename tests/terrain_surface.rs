//! `ProceduralTerrain::sample_surface` generates a preset's surface on the CPU, with no app and no
//! GPU (bevy_emerge #99 bakes a preset into its own file this way).

use bevy::math::UVec2;
use bevy_elements::ProceduralTerrain;

fn presets(seed: u64) -> [(&'static str, ProceduralTerrain); 4] {
    [
        ("hills", ProceduralTerrain::hills(seed)),
        ("mountains", ProceduralTerrain::mountains(seed)),
        ("dunes", ProceduralTerrain::dunes(seed)),
        ("plains", ProceduralTerrain::plains(seed)),
    ]
}

#[test]
fn a_preset_is_deterministic_per_seed_and_differs_between_seeds() {
    let size = UVec2::new(24, 16);
    for ((name, a), (_, b)) in presets(7).into_iter().zip(presets(8)) {
        let first = a.sample_surface(size, 16.0);
        assert_eq!(first.len(), 24 * 16, "{name}");
        assert_eq!(first, a.sample_surface(size, 16.0), "{name} repeats");
        assert_ne!(first, b.sample_surface(size, 16.0), "{name} seed 8 differs");
    }
}

#[test]
fn heights_stay_within_the_relief_and_use_most_of_it() {
    for (name, p) in presets(3) {
        let h = p.sample_surface(UVec2::splat(64), 32.0);
        let (lo, hi) = h
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(
            lo >= p.base_height_m && hi <= p.base_height_m + p.relief_m,
            "{name}: {lo}..{hi}"
        );
        assert!(
            hi - lo > 0.2 * p.relief_m,
            "{name} spans {} of {} m",
            hi - lo,
            p.relief_m
        );
    }
}

#[test]
fn rows_run_along_x_and_sampling_a_cell_does_not_depend_on_the_grid_around_it() {
    // The cell (x, z) of a wide grid is the cell (x, z) of a narrow one: row-major with the
    // stride of the grid's width, centre-sampled.
    let p = ProceduralTerrain::hills(11);
    let wide = p.sample_surface(UVec2::new(32, 8), 8.0);
    let narrow = p.sample_surface(UVec2::new(12, 5), 8.0);
    for z in 0..5usize {
        for x in 0..12usize {
            assert_eq!(wide[z * 32 + x], narrow[z * 12 + x], "cell ({x}, {z})");
        }
    }
    assert_ne!(wide[1], wide[32], "x and z are different axes");
}

#[test]
fn an_empty_grid_or_a_bad_cell_size_gives_nothing() {
    let p = ProceduralTerrain::plains(1);
    assert!(p.sample_surface(UVec2::new(0, 8), 1.0).is_empty());
    assert!(p.sample_surface(UVec2::new(8, 0), 1.0).is_empty());
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(p.sample_surface(UVec2::splat(4), bad).is_empty(), "{bad}");
    }
}

//! `SurfaceSetup` and `TerrainSetup`: validated, round-trippable, applied to a running app without
//! touching what did not change. Headless, no GPU.

#![allow(clippy::field_reassign_with_default)]

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_elements::*;

type Change = Box<dyn Fn(&mut SurfaceSetup)>;

#[test]
fn each_bound_rejects_a_value_just_outside_it_and_names_the_field() {
    let cases: Vec<(&str, Change)> = vec![
        ("hydrology.step_s", Box::new(|s| s.hydrology.step_s = 0.0)),
        (
            "hydrology.time_scale",
            Box::new(|s| s.hydrology.time_scale = -0.1),
        ),
        (
            "hydrology.manning_n",
            Box::new(|s| s.hydrology.manning_n = f32::NAN),
        ),
        (
            "erosion.talus_deg",
            Box::new(|s| s.erosion.talus_deg = 90.0),
        ),
        ("erosion.strength", Box::new(|s| s.erosion.strength = -1.0)),
        ("snow.new_density", Box::new(|s| s.snow.new_density = 0.0)),
        (
            "snow.shed_slope_deg",
            Box::new(|s| s.snow.shed_slope_deg = [50.0, 40.0]),
        ),
        (
            "snow.shed_slope_deg",
            Box::new(|s| s.snow.shed_slope_deg = [30.0, 90.5]),
        ),
        (
            "wetness.puddle_full_m",
            Box::new(|s| s.wetness.puddle_full_m = s.wetness.puddle_min_m - 1e-4),
        ),
        (
            "water.refractive_index",
            Box::new(|s| s.water.refractive_index = 0.99),
        ),
        (
            "water.absorption",
            Box::new(|s| s.water.absorption.y = -0.01),
        ),
        (
            "water.foam_albedo",
            Box::new(|s| s.water.foam_albedo = 1.01),
        ),
        ("water.tile_m", Box::new(|s| s.water.tile_m = 0.0)),
        (
            "water.wind_speed_ms",
            Box::new(|s| s.water.wind_speed_ms = Some(-1.0)),
        ),
        (
            "waves.damping_per_s",
            Box::new(|s| s.waves.damping_per_s = f32::INFINITY),
        ),
        ("rocks.talus_cover", Box::new(|s| s.rocks.talus_cover = 1.5)),
        (
            "rocks.allowed",
            Box::new(|s| {
                s.rocks.allowed = Some(vec![Rect::new(10.0, 0.0, 0.0, 5.0)]);
            }),
        ),
        ("dust.talus_deg", Box::new(|s| s.dust.talus_deg = 0.0)),
        (
            "dust.saturation_length_m",
            Box::new(|s| s.dust.saturation_length_m = 0.0),
        ),
        ("trails.texel_m", Box::new(|s| s.trails.texel_m = Some(0.0))),
        (
            "trails.min_depth_m",
            Box::new(|s| s.trails.min_depth_m = -0.01),
        ),
    ];
    for (field, change) in cases {
        let mut s = SurfaceSetup::default();
        change(&mut s);
        let e = s.validate().expect_err(field);
        assert_eq!(e.field, field, "{e}");
    }
    // The edges themselves are valid, and so is every default.
    assert_eq!(SurfaceSetup::default().validate(), Ok(()));
    let mut s = SurfaceSetup::default();
    s.snow.shed_slope_deg = [0.0, 90.0];
    s.water.refractive_index = 1.0;
    s.rocks.allowed = Some(vec![Rect::new(0.0, 0.0, 0.0, 0.0)]);
    s.rocks.talus_cover = 1.0;
    s.trails.texel_m = None;
    assert_eq!(s.validate(), Ok(()));
}

#[test]
fn an_invalid_setup_changes_nothing() {
    let mut world = World::new();
    let mut s = SurfaceSetup::default();
    s.snow.new_density = 55.0; // valid, applied before the bad field in field order
    s.dust.talus_deg = 120.0;
    assert!(s.apply_to_world(&mut world).is_err());
    assert!(!world.contains_resource::<SnowSettings>());
    assert!(!world.contains_resource::<DustSettings>());
}

#[test]
fn applying_writes_each_setting_and_capture_reads_it_back() {
    let mut world = World::new();
    let mut s = SurfaceSetup::default();
    s.hydrology.boundary = HydrologyBoundary::Closed;
    s.erosion.enabled = false;
    s.snow.degree_day_factor = 6.5;
    s.water.ripple_scale = 0.25;
    s.rocks.seed = 42;
    s.rocks.allowed = Some(vec![Rect::new(-100.0, -100.0, 100.0, 100.0)]);
    s.trails.min_depth_m = 0.01;
    s.apply_to_world(&mut world).unwrap();
    assert_eq!(
        world.resource::<HydrologySettings>().boundary,
        HydrologyBoundary::Closed
    );
    assert_eq!(world.resource::<SnowSettings>().degree_day_factor, 6.5);
    assert_eq!(SurfaceSetup::capture_from_world(&world), s);

    // A world that has none of the resources captures the plugins' defaults.
    assert_eq!(
        SurfaceSetup::capture_from_world(&World::new()),
        SurfaceSetup::default()
    );
}

#[test]
fn only_a_setting_that_differs_is_written() {
    let mut world = World::new();
    let mut s = SurfaceSetup::default();
    s.apply_to_world(&mut world).unwrap();
    let stamp = |w: &World| {
        (
            w.resource_ref::<RockSettings>().last_changed().get(),
            w.resource_ref::<SnowSettings>().last_changed().get(),
        )
    };
    let before = stamp(&world);
    world.increment_change_tick();
    world.increment_change_tick();

    // An unchanged setup leaves every resource unmarked: a changed RockSettings rebuilds the scatter.
    s.apply_to_world(&mut world).unwrap();
    assert_eq!(stamp(&world), before);

    // Changing snow marks snow alone.
    s.snow.max_rate_hz = 5.0;
    s.apply_to_world(&mut world).unwrap();
    let after = stamp(&world);
    assert_eq!(after.0, before.0);
    assert!(after.1 > before.1);
}

#[test]
fn json_round_trips_defaults_fill_in_and_a_typo_is_an_error() {
    let mut s = SurfaceSetup::default();
    s.water.wind_speed_ms = Some(7.0);
    s.water.absorption = Vec3::new(0.2, 0.1, 0.05);
    s.rocks.allowed = Some(vec![Rect::new(0.0, 0.0, 50.0, 60.0)]);
    s.hydrology.boundary = HydrologyBoundary::Closed;
    let text = serde_json::to_string(&s).unwrap();
    let back: SurfaceSetup = serde_json::from_str(&text).unwrap();
    assert_eq!(back, s);

    let only: SurfaceSetup = serde_json::from_str(r#"{"snow":{"drift":false}}"#).unwrap();
    assert!(!only.snow.drift);
    assert_eq!(only.snow.new_density, SnowSettings::default().new_density);
    assert_eq!(only.water, WaterSettings::default());

    assert!(serde_json::from_str::<SurfaceSetup>(r#"{"snows":{}}"#).is_err());
}

#[test]
fn terrain_bounds_name_the_field() {
    type TChange = Box<dyn Fn(&mut TerrainSetup)>;
    let procedural = |f: fn(&mut ProceduralTerrain)| -> TChange {
        Box::new(move |t| {
            let TerrainSetupSource::Procedural(p) = &mut t.source else {
                unreachable!()
            };
            f(p)
        })
    };
    let cases: Vec<(&str, TChange)> = vec![
        ("terrain.extent_m.x", Box::new(|t| t.extent_m.x = 0.0)),
        ("terrain.extent_m.y", Box::new(|t| t.extent_m.y = f32::NAN)),
        ("terrain.resolution", Box::new(|t| t.resolution.y = 1)),
        ("terrain.source.relief_m", procedural(|p| p.relief_m = -1.0)),
        (
            "terrain.source.feature_size_m",
            procedural(|p| p.feature_size_m = 0.0),
        ),
        ("terrain.source.octaves", procedural(|p| p.octaves = 0)),
        ("terrain.source.ridged", procedural(|p| p.ridged = 1.1)),
        (
            "terrain.source.initial_sand_m",
            procedural(|p| p.initial_sand_m = -0.5),
        ),
        (
            "terrain.source",
            Box::new(|t| t.source = TerrainSetupSource::Flat(f32::INFINITY)),
        ),
        (
            "terrain.source.path",
            Box::new(|t| {
                t.source = TerrainSetupSource::Heightmap {
                    path: String::new(),
                    height_range_m: [0.0, 1.0],
                }
            }),
        ),
        (
            "terrain.source.height_range_m",
            Box::new(|t| {
                t.source = TerrainSetupSource::Heightmap {
                    path: "h.png".into(),
                    height_range_m: [5.0, 5.0],
                }
            }),
        ),
    ];
    for (field, change) in cases {
        let mut t = TerrainSetup::default();
        change(&mut t);
        let e = t.validate().expect_err(field);
        assert_eq!(e.field, field, "{e}");
    }
    assert_eq!(TerrainSetup::default().validate(), Ok(()));
}

fn assets() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    app
}

#[test]
fn a_terrain_setup_builds_the_terrain_and_captures_back() {
    let app = assets();
    let server = app.world().resource::<AssetServer>();

    for source in [
        TerrainSetupSource::Procedural(ProceduralTerrain::mountains(9)),
        TerrainSetupSource::Flat(12.5),
        TerrainSetupSource::Heightmap {
            path: "heights/valley.png".into(),
            height_range_m: [-20.0, 340.0],
        },
    ] {
        let setup = TerrainSetup {
            source,
            extent_m: Vec2::new(4096.0, 1024.0),
            resolution: UVec2::new(512, 128),
            external: true,
        };
        let terrain = setup.terrain(server).unwrap();
        assert_eq!(terrain.extent_m, setup.extent_m);
        assert_eq!(terrain.resolution, setup.resolution);
        assert_eq!(terrain.render, TerrainRender::External);
        assert_eq!(TerrainSetup::capture(&terrain).unwrap(), setup);
        if let TerrainSource::Heightmap { height_range_m, .. } = &terrain.source {
            assert_eq!(*height_range_m, -20.0..340.0);
        }
    }

    // An invalid setup builds nothing.
    let mut bad = TerrainSetup::default();
    bad.extent_m = Vec2::ZERO;
    assert!(bad.terrain(server).is_err());

    // A height map made in memory has no path a file can name.
    let memory = Terrain {
        source: TerrainSource::Heightmap {
            image: Handle::default(),
            height_range_m: 0.0..1.0,
        },
        ..Terrain::flat(0.0)
    };
    assert_eq!(
        TerrainSetup::capture(&memory).unwrap_err().field,
        "terrain.source.path"
    );
}

#[test]
fn a_terrain_setup_round_trips_through_json() {
    let mut t = TerrainSetup::default();
    t.source = TerrainSetupSource::Heightmap {
        path: "heights/valley.png".into(),
        height_range_m: [0.0, 500.0],
    };
    let text = serde_json::to_string(&t).unwrap();
    assert_eq!(serde_json::from_str::<TerrainSetup>(&text).unwrap(), t);
    let procedural = TerrainSetup {
        source: TerrainSetupSource::Procedural(ProceduralTerrain::dunes(3)),
        ..TerrainSetup::default()
    };
    let text = serde_json::to_string(&procedural).unwrap();
    assert_eq!(
        serde_json::from_str::<TerrainSetup>(&text).unwrap(),
        procedural
    );
    assert!(serde_json::from_str::<TerrainSetup>(r#"{"extent":[1,1]}"#).is_err());
}

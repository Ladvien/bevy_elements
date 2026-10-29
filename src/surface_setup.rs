//! The ground, water and cover settings as one value a host can save, validate and apply, and the
//! terrain's source as a value that survives a file.
//!
//! [`SurfaceSetup`] bundles the settings resources of hydrology, erosion, snow, wetness, water,
//! waves, rocks, aeolian dust and trails, the way [`EnvironmentSetup`](crate::EnvironmentSetup)
//! bundles the sky's. [`TerrainSetup`] describes a [`Terrain`] as plain data (a height map is a
//! path, not a `Handle`), so a level file can name the ground it wants and the host builds the
//! component with [`TerrainSetup::terrain`]. `docs/research/surface-setup.md`.

use std::ops::Range;

use bevy::asset::AssetServer;
use bevy::image::ImageLoaderSettings;
use bevy::prelude::*;

use crate::setup::{SetupError, bad, within};
use crate::{
    DustSettings, ErosionSettings, HydrologySettings, ProceduralTerrain, RockSettings,
    SnowSettings, Terrain, TerrainRender, TerrainSource, TrailSettings, WaterSettings,
    WaveSettings, WetnessSettings,
};

/// `v` is finite and at least `lo`.
fn at_least(field: &'static str, v: f32, lo: f32) -> Result<(), SetupError> {
    if v.is_finite() && v >= lo {
        Ok(())
    } else {
        Err(bad(field, format!("{v} is not finite and >= {lo}")))
    }
}

/// `v` is finite and above zero.
fn positive(field: &'static str, v: f32) -> Result<(), SetupError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(bad(field, format!("{v} is not finite and > 0")))
    }
}

/// `v` is in `[0, 1]`.
fn unit(field: &'static str, v: f32) -> Result<(), SetupError> {
    within(field, v as f64, 0.0, 1.0)
}

/// An angle strictly between 0° and 90°.
fn slope(field: &'static str, v: f32) -> Result<(), SetupError> {
    if v.is_finite() && v > 0.0 && v < 90.0 {
        Ok(())
    } else {
        Err(bad(field, format!("{v} is outside (0, 90) degrees")))
    }
}

/// Writes `value` into the world's resource only when it differs, so applying an unchanged setup
/// leaves change detection quiet (a changed [`RockSettings`] rebuilds the whole scatter).
fn put<R: Resource + Clone + PartialEq>(world: &mut World, value: &R) {
    if world.get_resource::<R>() != Some(value) {
        world.insert_resource(value.clone());
    }
}

/// The running resource, or the default the plugin would insert when it is not in the world.
fn get<R: Resource + Clone + Default>(world: &World) -> R {
    world.get_resource::<R>().cloned().unwrap_or_default()
}

/// The ground, water and cover settings: what an author tunes and a game loads.
///
/// Every field is a resource the plugins read each frame, so [`apply_to_world`] takes effect on a
/// running app. With the `serde` feature it reads and writes any serde format; absent fields take
/// their defaults and unknown fields are an error.
///
/// [`apply_to_world`]: Self::apply_to_world
#[derive(Clone, Debug, PartialEq, Default, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default, deny_unknown_fields)
)]
pub struct SurfaceSetup {
    /// The shallow-water simulation.
    pub hydrology: HydrologySettings,
    /// Hydraulic and thermal erosion.
    pub erosion: ErosionSettings,
    /// Snow density, melt, drift and shedding.
    pub snow: SnowSettings,
    /// How the ground wets and dries.
    pub wetness: WetnessSettings,
    /// How water looks.
    pub water: WaterSettings,
    /// The wave field.
    pub waves: WaveSettings,
    /// The scattered rocks.
    pub rocks: RockSettings,
    /// Wind-blown sand.
    pub dust: DustSettings,
    /// Footprints, ruts and tracks.
    pub trails: TrailSettings,
}

impl SurfaceSetup {
    /// Checks every field against the range its model is defined on; the first failure names the
    /// field as a path (`snow.shed_slope_deg`).
    pub fn validate(&self) -> Result<(), SetupError> {
        let h = &self.hydrology;
        positive("hydrology.step_s", h.step_s)?;
        at_least("hydrology.time_scale", h.time_scale, 0.0)?;
        positive("hydrology.manning_n", h.manning_n)?;

        let e = &self.erosion;
        for (field, v) in [
            ("erosion.capacity", e.capacity),
            ("erosion.dissolve", e.dissolve),
            ("erosion.deposit", e.deposit),
            ("erosion.min_tilt", e.min_tilt),
            ("erosion.thermal_rate", e.thermal_rate),
            ("erosion.strength", e.strength),
        ] {
            at_least(field, v, 0.0)?;
        }
        slope("erosion.talus_deg", e.talus_deg)?;

        let s = &self.snow;
        positive("snow.new_density", s.new_density)?;
        positive("snow.settled_density", s.settled_density)?;
        positive("snow.settle_days", s.settle_days)?;
        for (field, v) in [
            ("snow.degree_day_factor", s.degree_day_factor),
            ("snow.drift_threshold_ms", s.drift_threshold_ms),
            ("snow.drift_coefficient", s.drift_coefficient),
            ("snow.max_rate_hz", s.max_rate_hz),
        ] {
            at_least(field, v, 0.0)?;
        }
        let [from, to] = s.shed_slope_deg;
        if !(from.is_finite() && to.is_finite() && 0.0 <= from && from <= to && to <= 90.0) {
            return Err(bad(
                "snow.shed_slope_deg",
                format!("[{from}, {to}] is not 0 <= from <= to <= 90 degrees"),
            ));
        }

        let w = &self.wetness;
        for (field, v) in [
            ("wetness.wet_per_mm", w.wet_per_mm),
            ("wetness.dry_rate_h", w.dry_rate_h),
            ("wetness.puddle_min_m", w.puddle_min_m),
        ] {
            at_least(field, v, 0.0)?;
        }
        at_least("wetness.puddle_full_m", w.puddle_full_m, w.puddle_min_m)?;

        let a = &self.water;
        for (field, v) in [
            ("water.absorption", a.absorption),
            ("water.deep_color", a.deep_color),
        ] {
            for c in v.to_array() {
                at_least(field, c, 0.0)?;
            }
        }
        for c in a.silt_tint.to_array() {
            unit("water.silt_tint", c)?;
        }
        at_least("water.refractive_index", a.refractive_index, 1.0)?;
        for (field, v) in [
            ("water.turbidity_scattering", a.turbidity_scattering),
            ("water.clear_scattering", a.clear_scattering),
            ("water.foam_speed_ms", a.foam_speed_ms),
            ("water.foam_thickness_m", a.foam_thickness_m),
            ("water.flow_distortion", a.flow_distortion),
            ("water.ripple_scale", a.ripple_scale),
        ] {
            at_least(field, v, 0.0)?;
        }
        unit("water.foam_albedo", a.foam_albedo)?;
        unit("water.foam_roughness", a.foam_roughness)?;
        positive("water.tile_m", a.tile_m)?;
        positive("water.flow_cycle_s", a.flow_cycle_s)?;
        if let Some(wind) = a.wind_speed_ms {
            at_least("water.wind_speed_ms", wind, 0.0)?;
        }

        at_least("waves.damping_per_s", self.waves.damping_per_s, 0.0)?;

        let r = &self.rocks;
        unit("rocks.talus_cover", r.talus_cover)?;
        unit("rocks.meadow_cover", r.meadow_cover)?;
        unit("rocks.meadow_floor", r.meadow_floor)?;
        at_least("rocks.range_scale", r.range_scale, 0.0)?;
        for rect in r.allowed.iter().flatten() {
            let finite = rect.min.is_finite() && rect.max.is_finite();
            if !(finite && rect.min.x <= rect.max.x && rect.min.y <= rect.max.y) {
                return Err(bad(
                    "rocks.allowed",
                    format!("{rect:?} is not a finite rectangle with min <= max"),
                ));
            }
        }

        let d = &self.dust;
        at_least("dust.threshold_ms", d.threshold_ms, 0.0)?;
        at_least("dust.flux_coefficient", d.flux_coefficient, 0.0)?;
        positive("dust.saturation_length_m", d.saturation_length_m)?;
        slope("dust.talus_deg", d.talus_deg)?;
        at_least("dust.step_hz", d.step_hz, 0.0)?;

        if let Some(texel) = self.trails.texel_m {
            positive("trails.texel_m", texel)?;
        }
        at_least("trails.min_depth_m", self.trails.min_depth_m, 0.0)?;
        Ok(())
    }

    /// Validates, then writes each setting into the world. On an error nothing is changed.
    ///
    /// A resource whose value already equals the setup's is left alone, so applying an unchanged
    /// file does not mark it changed (and does not rebuild the rocks' scatter). A resource a
    /// missing plugin would have inserted is inserted here; the plugin, added later, keeps it.
    pub fn apply_to_world(&self, world: &mut World) -> Result<(), SetupError> {
        self.validate()?;
        put(world, &self.hydrology);
        put(world, &self.erosion);
        put(world, &self.snow);
        put(world, &self.wetness);
        put(world, &self.water);
        put(world, &self.waves);
        put(world, &self.rocks);
        put(world, &self.dust);
        put(world, &self.trails);
        Ok(())
    }

    /// The running settings as a setup, to save. A setting whose plugin has not added its resource
    /// reads as the default the plugin would use.
    pub fn capture_from_world(world: &World) -> Self {
        Self {
            hydrology: get(world),
            erosion: get(world),
            snow: get(world),
            wetness: get(world),
            water: get(world),
            waves: get(world),
            rocks: get(world),
            dust: get(world),
            trails: get(world),
        }
    }
}

/// Where a [`TerrainSetup`]'s surface comes from, as data.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub enum TerrainSetupSource {
    /// Generated from noise when the terrain spawns.
    Procedural(ProceduralTerrain),
    /// Level ground at this height, m.
    Flat(f32),
    /// Resampled from an image's red channel, read linearly (a 16-bit file is recommended).
    Heightmap {
        /// The asset path of the image.
        path: String,
        /// Heights of the lowest and highest values, m.
        height_range_m: [f32; 2],
    },
}

/// A [`Terrain`] as plain data: what a level file stores and [`terrain`](Self::terrain) builds.
///
/// A `Terrain` is built once when its entity spawns, so a changed setup takes effect by
/// despawning the terrain entity and spawning a new one from [`terrain`](Self::terrain).
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default, deny_unknown_fields)
)]
pub struct TerrainSetup {
    /// Where the surface comes from. Default rolling hills, seed 0.
    pub source: TerrainSetupSource,
    /// World size, m. Default 2048 × 2048.
    pub extent_m: Vec2,
    /// Cells per axis; the plugin clamps it to the quality level and rounds down to a power of
    /// two. Default 1024².
    pub resolution: UVec2,
    /// Whether the consumer's meshes draw the ground ([`TerrainRender::External`]). Default
    /// `false`.
    pub external: bool,
}

impl Default for TerrainSetup {
    fn default() -> Self {
        Self {
            source: TerrainSetupSource::Procedural(ProceduralTerrain::hills(0)),
            extent_m: Vec2::splat(2048.0),
            resolution: UVec2::splat(1024),
            external: false,
        }
    }
}

impl TerrainSetup {
    /// Checks the extent, resolution and source.
    pub fn validate(&self) -> Result<(), SetupError> {
        positive("terrain.extent_m.x", self.extent_m.x)?;
        positive("terrain.extent_m.y", self.extent_m.y)?;
        if self.resolution.x < 2 || self.resolution.y < 2 {
            return Err(bad(
                "terrain.resolution",
                format!("{} has fewer than 2 cells on an axis", self.resolution),
            ));
        }
        match &self.source {
            TerrainSetupSource::Procedural(p) => {
                if !p.base_height_m.is_finite() {
                    return Err(bad("terrain.source.base_height_m", "is not finite"));
                }
                at_least("terrain.source.relief_m", p.relief_m, 0.0)?;
                positive("terrain.source.feature_size_m", p.feature_size_m)?;
                if p.octaves < 1 {
                    return Err(bad(
                        "terrain.source.octaves",
                        "is 0; at least 1 (the generator stops at the Nyquist limit itself)",
                    ));
                }
                unit("terrain.source.ridged", p.ridged)?;
                at_least("terrain.source.initial_sand_m", p.initial_sand_m, 0.0)?;
            }
            TerrainSetupSource::Flat(height) => {
                if !height.is_finite() {
                    return Err(bad(
                        "terrain.source",
                        format!("flat height {height} is not finite"),
                    ));
                }
            }
            TerrainSetupSource::Heightmap {
                path,
                height_range_m: [lo, hi],
            } => {
                if path.is_empty() {
                    return Err(bad("terrain.source.path", "is empty"));
                }
                if !(lo.is_finite() && hi.is_finite() && lo < hi) {
                    return Err(bad(
                        "terrain.source.height_range_m",
                        format!("[{lo}, {hi}] is not finite with lo < hi"),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validates, then builds the [`Terrain`]. A height map is loaded through `assets` as the
    /// terrain requires (linear, single channel, no sRGB), kept off the GPU.
    pub fn terrain(&self, assets: &AssetServer) -> Result<Terrain, SetupError> {
        self.validate()?;
        let source = match &self.source {
            TerrainSetupSource::Procedural(p) => TerrainSource::Procedural(*p),
            TerrainSetupSource::Flat(h) => TerrainSource::Flat(*h),
            TerrainSetupSource::Heightmap {
                path,
                height_range_m: [lo, hi],
            } => TerrainSource::Heightmap {
                image: assets
                    .load_builder()
                    .with_settings(|s: &mut ImageLoaderSettings| {
                        s.is_srgb = false;
                        s.expand_grayscale = false;
                    })
                    .load(path.clone()),
                height_range_m: Range {
                    start: *lo,
                    end: *hi,
                },
            },
        };
        Ok(Terrain {
            source,
            extent_m: self.extent_m,
            resolution: self.resolution,
            render: if self.external {
                TerrainRender::External
            } else {
                TerrainRender::Owned
            },
        })
    }

    /// A terrain component as a setup, to save. Errors for a height map whose handle has no asset
    /// path (an image made in memory has nothing a file can name).
    pub fn capture(terrain: &Terrain) -> Result<Self, SetupError> {
        let source = match &terrain.source {
            TerrainSource::Procedural(p) => TerrainSetupSource::Procedural(*p),
            TerrainSource::Flat(h) => TerrainSetupSource::Flat(*h),
            TerrainSource::Heightmap {
                image,
                height_range_m,
            } => TerrainSetupSource::Heightmap {
                path: image
                    .path()
                    .ok_or_else(|| {
                        bad(
                            "terrain.source.path",
                            "the height map was not loaded from a path",
                        )
                    })?
                    .to_string(),
                height_range_m: [height_range_m.start, height_range_m.end],
            },
        };
        Ok(Self {
            source,
            extent_m: terrain.extent_m,
            resolution: terrain.resolution,
            external: terrain.render == TerrainRender::External,
        })
    }
}

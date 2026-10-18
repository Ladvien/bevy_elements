#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod celestial;
mod climate;
mod clouds;
mod cover;
mod dust;
mod flash;
#[cfg(feature = "flora")]
mod flora;
mod fog;
mod gpu;
mod hydrology;
mod lightning;
mod plugin;
mod precipitation;
mod query;
mod terrain;
mod trails;
#[cfg(feature = "isomesh")]
mod voxel;
mod waves;
mod weathered;
mod wind;

// Plugin group, system sets, camera, its focus, quality.
pub use plugin::{
    ElementsCamera, ElementsFocus, ElementsPlugins, ElementsQuality, ElementsSystems,
};

// Readback cadence.
pub use gpu::readback::ElementsReadback;

// Gameplay queries.
pub use query::{Environment, EnvironmentSnapshot};

// Celestial: clock, location, sun, moon, exposure; `astro` holds the ephemeris functions.
pub use celestial::{
    CelestialPlugin, CelestialSettings, CivilDate, GeoLocation, Moon, Sun, SunState, WorldClock,
    astro,
};

// Climate, seasons, weather.
pub use climate::{
    Climate, ClimatePlugin, LAPSE_RATE_C_PER_M, Season, SeasonChanged, SeasonState, SnowSeason,
    SurfaceWeathering, Weather, WeatherChanged, WeatherDirector, WeatherPreset,
};

// Wind.
pub use wind::{
    MAX_GUSTS, WIND_CELL_M, WIND_ROUGHNESS_M, WindFieldParams, WindGust, WindPlugin, WindState,
    log_wind_speed,
};

// Precipitation.
pub use precipitation::{
    MAX_PRECIPITATION_BLOCKERS, PRECIPITATION_BOX_M, PrecipitationBlocker, PrecipitationPlugin,
    PrecipitationState,
};

// Fog.
pub use fog::{
    CLEAR_AIR_EXTINCTION, CLEAR_VISIBILITY_M, FogPlugin, GROUND_FOG_EXTENT, GROUND_FOG_TEXELS,
    GroundFog, KOSCHMIEDER, distance_fog_falloff, extinction_for_visibility, fog_hue,
    orthographic_fog_falloff, orthographic_near_depth, sky_air_extinction,
    volume_light_compensation,
};

// Clouds.
pub use clouds::{CloudLayer, CloudPlugin, CloudQuality, CloudShadows, CloudWeatherMap};

// Lightning and thunder; `LightningFlash` is shared with the cloud and precipitation passes.
pub use flash::LightningFlash;
pub use lightning::{
    FLASH_LUMENS, LightningPlugin, LightningStrike, SPEED_OF_SOUND_MS, STRIKE_RADIUS_M,
    StrikeLightning, Thunder,
};

// Terrain.
pub use terrain::{
    GroundExtension, GroundMaterial, GroundParams, ProceduralTerrain, Terrain, TerrainExtension,
    TerrainGrid, TerrainMaterial, TerrainMaterialParams, TerrainPlugin, TerrainRender,
    TerrainSnapshot, TerrainSource, TerrainState,
};

// Aeolian transport.
pub use dust::{DustPlugin, DustSettings, DustStats};

// Hydrology and erosion.
pub use hydrology::{
    ErodeTerrain, ErosionSettings, HydrologyBoundary, HydrologyPlugin, HydrologySettings,
    HydrologyStats, MAX_WATER_SOURCES, TerrainBaked, WaterExtension, WaterMaterial, WaterParams,
    WaterPatch, WaterPlugin, WaterSettings, WaterSink, WaterSource, WaterState, evaporation_mm_h,
};

// Ground cover: snow and wetness maps.
pub use cover::{CoverState, CoverStats, GroundCoverPlugin, SnowSettings, WetnessSettings};

// Trails: footprints, ruts and tracks in snow, sand and mud.
pub use trails::{Imprint, TrailSettings, TrailState, TrailsPlugin};

// Waves: ripples and wakes from bodies in the water.
pub use waves::{WaterContact, WaveSettings, WaveState, WavesPlugin};

// `bevy_flora` integration.
#[cfg(feature = "flora")]
pub use flora::{ElementsFloraPlugin, FLORA_FULL_WIND_MS};

// Weathered materials.
pub use weathered::{
    ELEMENTS_GLOBALS_BUFFER, ElementsGlobals, SNOW_ALBEDO, SNOW_ROUGHNESS, WeatheredExtension,
    WeatheredMaterial, WeatheredPlugin,
};

// Volumetric snow on `bevy_isomesh` voxel volumes.
#[cfg(feature = "isomesh")]
pub use voxel::{SnowyField, VoxelGround, VoxelGroundSystems, VoxelPlugin};

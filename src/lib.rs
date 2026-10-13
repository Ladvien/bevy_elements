#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod celestial;
mod climate;
mod clouds;
mod flash;
#[cfg(feature = "flora")]
mod flora;
mod fog;
mod gpu;
mod lightning;
mod plugin;
mod precipitation;
mod query;
mod terrain;
mod wind;
mod weathered;

// Plugin group, system sets, camera, quality.
pub use plugin::{ElementsCamera, ElementsPlugins, ElementsQuality, ElementsSystems};

// Readback cadence.
pub use gpu::readback::ElementsReadback;

// Gameplay queries.
pub use query::{Environment, EnvironmentSnapshot};

// Celestial: clock, location, sun, moon, exposure; `astro` holds the ephemeris functions.
pub use celestial::{
    CelestialPlugin, CivilDate, GeoLocation, Moon, Sun, SunState, WorldClock, astro,
};

// Climate, seasons, weather.
pub use climate::{
    Climate, ClimatePlugin, LAPSE_RATE_C_PER_M, Season, SeasonChanged, SeasonState,
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
    FogPlugin, GROUND_FOG_EXTENT, GROUND_FOG_TEXELS, GroundFog, KOSCHMIEDER, distance_fog_falloff,
    extinction_for_visibility, fog_hue, volume_light_compensation,
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

// `bevy_flora` integration.
#[cfg(feature = "flora")]
pub use flora::{ElementsFloraPlugin, FLORA_FULL_WIND_MS};

// Weathered materials.
pub use weathered::{
    ElementsGlobals, SNOW_ALBEDO, SNOW_ROUGHNESS, WeatheredExtension, WeatheredMaterial,
    WeatheredPlugin,
};

# EnvironmentSetup: the sky-and-weather settings as one saved, validated, live-appliable value

An editor (bevy_emerge's Environment panel, issue #98 there) lets an author choose a climate, a
place, a date and clock, and a forced weather, saves it with the map, and previews it live with this
crate's own plugins. Today it defines a wrapper type of its own because the crate offers no single
value for those four resources plus the director's forced preset, whose field is private, and
because an unchanged forced preset must not be forced again (that restarts the blend). This note
adds that value to the crate, in its general form: a game loading a level's environment, a debug
console and a test harness need the same.

## Sources

- The crate's own `docs/research/climate.md` and `docs/research/weather.md` define the ranges of
  `Climate` (humidity, shares in 0-1, unit prevailing wind, `warmest_day` 1-365) and the director's
  force/release semantics (`WeatherDirector::force`, `release`; the scheduler keeps stepping while
  forced). No new physics.
- Corpus search for method phrases (`preset`, `save state`) returns nothing applicable: this is API
  plumbing, not a simulation model. Ranges for location come from the geographic coordinate
  definition (latitude -90..90, longitude -180..180) and the UTC offset range in use, -12..+14 h.
- The editor's own use, read from `Ladvien/bevy_emerge` `crates/bevy_emerge_mapper/src/environment.rs`
  (shallow clone, 2026-10-29): `ElementsSettings { clock, location, climate, forced_weather }`,
  `apply` with an `applied_force` local, errors shown on a status line.

## What great looks like

A host reads a RON file, gets either a value or an error naming the field and the bound it broke,
applies it to a running app without restarting a forced blend it already holds, and can capture the
running state back into the same value to save it. No visual change: nothing rendered differs.

## Model adopted

`EnvironmentSetup { clock: WorldClock, location: GeoLocation, climate: Climate, forced_weather:
Option<WeatherPreset>, blend_minutes: f32 }`, serde with `default` and `deny_unknown_fields` behind
the `serde` feature. `validate()` returns `SetupError { field, problem }`; `apply(...)` validates
first and changes nothing on error; forcing happens only when the director's forced preset differs
(new `WeatherDirector::forced()`); `capture(...)` reads the four resources back; `apply_to_world` and
`capture_from_world` are the `&mut World` forms.

## Parameters

`blend_minutes` default 10 (game time), the editor's value (`BLEND`), finite and >= 0.
Bounds: latitude [-90, 90], longitude [-180, 180], UTC offset [-14, 14], hour [0, 24), time scale
finite and >= 0; climate fractions in [0, 1], amplitudes and wind speed finite and >= 0,
`warmest_day` 1..=366, prevailing wind finite and non-zero.

## GPU mapping

None. No shader, buffer or binding changes; no benchmark applies.

## Validation targets

- A default setup validates and applies to default resources unchanged.
- Each bound above rejects a value just outside it, naming the field.
- Applying twice with the same forced preset forces once (the director's pending request is not
  replaced); changing it forces again; `None` releases only a forced director.
- A RON/JSON round trip equals the original; an unknown field is rejected.
- `capture` after `apply` returns what was applied.

## Deviations from this plan

None: no default changes.

## Pending index

None; no new corpus document.

# The snowpack survives a save, and a snow season (issues #14, #15)

A game saves at dawn and loads later: the lying snow and the wet ground must come back with it (#14). A game's calendar says when snow may fall: "rain first, snow from 14 November" must hold even on a near-freezing night before then (#15).

## Sources

- Jennings, K. S., Winchell, T. S., Livneh, B., Molotch, N. P. (2018). Spatial variation of the rain–snow temperature threshold across the Northern Hemisphere. *Nature Communications* 9:1148. DOI `10.1038/s41467-018-03629-7`. Downloaded through `hs paper download`, read from the PDF (`pdftotext -layout`, 12 pages). Used: the 50 % rain–snow air-temperature threshold over 6883 stations "averaging 1.0 °C and ranging from −0.4 to 2.4 °C for 95 %" of them (abstract, Fig. 1); the threshold falls from 4.5 °C at 40–50 % relative humidity to 0.7 °C at 90–100 % (Fig. 2a), so a single place's threshold is not one number; the 2–98 % transition spans "a minimum width of 2.6 °C and a maximum of 4.6 °C". The crate's ramp (`Weather::snow_fraction`, snow 1 at −1 °C, 0 at 2 °C, 50 % at 0.5 °C, 3 °C wide) sits inside those ranges: rain at 0.5 °C on a late-autumn night is physical snow half the time, and no retuning of the ramp can keep snow out of a near-freezing fortnight.
- Patry, J. (2021). Advances in real-time rendering, *Ghost of Tsushima* (corpus `s2021-advances-pdf-66-mb`, ll. 229–267), already read for `docs/research/integration.md`: the weather states are physically based "but can deviate from physical correctness" through per-state art knobs. A snow season is such a knob: an authored calendar over a physical phase model.
- Repository notes and code: `docs/research/climate-weather.md` (the scheduler's temperature gates, the forced-snow −2 °C cap, the phase ramp), `docs/research/snow-cover.md` (the shared WGSL `snow_fraction_at` evaluated per cell in both the cover and the hydrology kernels, so the rain and snow shares add up to the precipitation in every cell; the degree-day melt; the point model in `src/climate/weathering.rs`), `docs/research/wetness.md`, `tests/save.rs` (the weather's save state: `WorldClock`, `Climate`, `GeoLocation`, `WeatherDirector`).
- Issues #14 and #15 (filed from Sanctuary), with their acceptance criteria and edge cases.

## Model adopted

### A snow season (#15)

`SnowSeason { first: (month, day), last: (month, day) }` on `Climate::snow_season: Option<SnowSeason>`, `None` by default (today's behaviour). The dates are the local calendar's (`WorldClock::date`), inclusive, and the season may wrap the year (14 November to 31 March). They are not shifted in the southern hemisphere: a game states its own calendar.

Outside the season:

1. **The scheduler gates Snow and Blizzard** exactly as it gates them above 1 °C (`schedule`, `weather.rs`): weight 0, and a chain sitting in Snow leaves it to Overcast. Inside the season the weights are unchanged, so an in-season chain is identical hour for hour to one with no season.
2. **Precipitation falls liquid.** `Weather` carries `snow_possible` (0–1) and `Weather::snow_fraction()` is the ramp times it, so every reader of the split (the point model, the cover map's snow and wetness kernels, hydrology's rain, the rain and snow particles, the snow extinction in the fog, the weathered materials' liquid rate) agrees. The GPU kernels take it through their uniforms and the shared WGSL `snow_fraction_at(t, possible)`.
3. **Forcing still snows.** `WeatherDirector::force(Snow | Blizzard, ..)` already caps the temperature at −2 °C through the blend's `snow_cap`; `snow_possible` is raised to that cap, so a forced snowfall falls as snow out of season too (blending in with the transition).
4. **Snow already lying stays** and melts by the degree-day model; nothing is removed at the season's end.

**The boundary ramps, outside the season.** A step from 0 to 1 at midnight would turn falling rain into falling snow in one frame. `snow_possible` ramps linearly over the hour before the first day starts (23:00–24:00 of the day before `first`) and over the hour after the last day ends (00:00–01:00 of the day after `last`), so the season itself is untouched (an in-season hour is identical to a run without a season) and the change of phase takes an hour of game time.

### The snowpack in a save (#14)

`SurfaceWeathering` derives `Serialize`/`Deserialize` with the `serde` feature: the published `wetness`, `snow_cover`, `snow_depth_m`, the point pack (`snow_water_mm`, `snow_density`) and `last_hours`.

**After a load the clock may not match.** A restored `SurfaceWeathering` (it carries a `restored` flag that is never written and is true when read back) integrates its first step normally when the clock is within one game hour of the saved `last_hours`, so a save and load at the same clock continues exactly as the unsaved run. A load onto a clock further away re-arms on that clock without integrating: the pack neither melts nor grows across a gap the game did not simulate. (Without this a load a day later would integrate 24 h of melt in one frame, the clamp `MAX_STEP_H`.)

**The cover maps are re-derived from the pack.** With a terrain the published values come from the ground-cover maps (`cover::weathering_from_map`), a per-cell GPU state (16 + 4 B a cell, 4 MB on a 512² grid) that starts zeroed. Saving it is out of proportion to the issue, and an empty map would overwrite the restored pack within a frame. Instead the maps are **seeded from the pack**: when they are created and when a restored `SurfaceWeathering` is inserted, one `cover_seed` dispatch, after the shelter mask is built, sets each cell's snow to the published depth (the mean the player saw around the camera at the save) at the pack's bulk density, times the cell's snow exposure (the slope shed and the blocker shelter), and its wetness to the published wetness times its rain exposure. Drifts, the lapse-rate line and the shelter's finer structure re-form as the simulation runs. A terrain rebuilt mid-game now keeps its snow too, uniform, instead of starting bare.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| `Climate::snow_season` | `None` | issue #15 (today's behaviour) |
| Season ramp | 1 game hour, outside the season | this note (no phase pop; the season is untouched) |
| `snow_possible` under a forced Snow/Blizzard | the blend's `snow_cap` (0 → 1 with the transition) | `climate-weather.md` (the forced −2 °C cap) |
| Restore tolerance | 1 game hour between the loaded clock and `last_hours` | this note |
| Seed snow density | the point pack's bulk density (100–300 kg/m³) | `snow-cover.md` |

## GPU mapping

- `climate/snow.wesl`: `snow_fraction_at(t_c, possible) = clamp((2 − t_c)/3, 0, 1) · possible`.
- Cover (`cover/common.wesl` `CoverParams`): `air.y` carries `snow_possible`; a new `seed: vec4` carries (depth m, SWE m, wetness, —). A `cover_seed` entry point in `cover/snow.wesl` writes `snow[k] = (d·e, w·e, 0, 0)` with `e` the snow exposure and `wetness[k] = wet · shelter[k].x`; it runs once per seed request (a `seed_epoch` handshake like the shelter's), in the snow pass after the shelter pass and before the steps.
- Hydrology (`substep.wesl` `HydroParams`): `misc.w` carries `snow_possible`.
- No new buffers; one 8×8 dispatch per seed.

## What great looks like

Sanctuary's November in the `weathered_town`: rain on the first nights wets the boxes and the ground, which never whiten before 14 November; from the 14th the first cold rain turns to snow over an hour and lies. A save taken in the snow and loaded on a fresh app shows the same snow on the same ground, under the same roofs bare, from the first frame the maps are live, with no flash of bare ground and no melt-through.

## Validation targets

- (#15) With `SnowSeason { first: (11, 14), last: (3, 31) }` and Sanctuary's climate (mean 9.4 °C, annual 10 °C, diurnal 4 °C, seed 1, 42.5° N), an hourly run from 1 to 13 November never schedules Snow or Blizzard and leaves `snow_cover == 0` and `snow_depth_m == 0`, with the same liquid precipitation as without the season.
- (#15) An in-season hour is identical, hour for hour, to a run with `snow_season = None` from the same start.
- (#15) A Rain hour at −3 °C outside the season has `snow_fraction() == 0`; inside it, the ramp's value.
- (#15) Forcing Snow outside the season still snows; a season wrapping the year contains 31 December and 1 January; a `Climate` saved without the field loads.
- (#15) Every existing test passes with `None`.
- (#14) A serde round trip keeps `snow_depth_m`, and stepping on continues identically to the unserialized original; a load onto a clock a day later neither melts nor grows the pack on its first step.
- (#14) GPU: a restored pack seeds the cover maps: open flat cells read the restored depth, a cell under a `PrecipitationBlocker` reads none.

## Deviations from this plan

(none yet)

## Pending index

- Brown, R. D., Mote, P. W. (2009). The response of Northern Hemisphere snow cover to a changing climate. *J. Climate* 22:2124–2145, DOI `10.1175/2008JCLI2665.1`: `hs paper download` found no copy (CORE needs a key, Crossref's AMS link 404s, the AMS download link returns HTML). Wanted for the observed onset and duration of the snow season as context for authored dates; nothing in the model depends on it.
- `10.1038_s41467-018-03629-7` is downloaded but not converted or indexed (read from the PDF).

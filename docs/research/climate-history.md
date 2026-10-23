# Climate history: a record of recent days (issue #31)

A game asks a coarse, human-scale question of the weather: what have the last few days been like — how many wet days, how many frosts, the mean temperature and humidity. `Environment` answers only the present, and `Climate` is configuration, so nothing can answer it. Issue #31 asks for the query and offers two shapes: a game-flavoured summary type, or the history itself with the game doing the arithmetic. The crate keeps no game vocabulary (`AGENTS.md`), so it takes the second shape and adds the climatologist's standard daily indices, which name the thresholds the issue asks to be documented.

## Sources

- **Issue #31** (the requirement, the edge cases and the acceptance criteria).
- **ETCCDI, "Definitions of the 27 core indices"** (Expert Team on Climate Change Detection and Indices, WMO CCl/CLIVAR/JCOMM; https://etccdi.pacificclimate.org/list_27_indices.shtml, read 2026-10-23). The indices are computed from a *daily* record of TX (daily maximum temperature), TN (daily minimum), TG (daily mean) and RR (daily precipitation amount). Verbatim: "FD, Number of frost days: Annual count of days when TN (daily minimum temperature) < 0 °C"; "ID, Number of icing days: … when TX (daily maximum temperature) < 0 °C"; a wet day is "RR ≥ 1mm" (SDII, CWD, R95pTOT) and a dry day "RR < 1mm" (CDD).
- **Zhang, Alexander, Hegerl, Jones, Klein Tank, Peterson, Trewin & Zwiers 2011**, "Indices for monitoring changes in extremes based on daily temperature and precipitation data", *WIREs Climate Change* 2:851 (DOI 10.1002/wcc.147): the paper behind the ETCCDI list — "assessing days with temperature or precipitation observations above or below specific physically-based thresholds" (abstract, from the catalog record). Not in the corpus (see Pending index); the definitions above are the Expert Team's own page.
- **Thurner et al. 2017**, *Global Change Biology* (DOI 10.1111/gcb.13660, corpus): "Icing days are defined as the annual number of days with a daily maximum temperature below 0 °C, whereas frost days are the annual number of days with a daily minimum temperature below 0 °C" — the same thresholds used outside the index list.
- **Richardson 1981**, "Stochastic simulation of daily precipitation, temperature, and solar radiation", *Water Resources Research* 17(1):182 (DOI 10.1029/WR017i001p00182, corpus): the daily weather record is "the daily precipitation amount, … the maximum temperature, … the minimum temperature, and … the solar radiation", with precipitation the primary variable and the others conditioned on "whether the day was wet or dry"; there "a day with total rainfall of 0.2 mm or more was considered a wet day". The ancestor of the crate's scheduler (`docs/research/climate-weather.md`).
- The crate: `docs/research/climate-weather.md` (the scheduler: a Markov chain over hourly presets that starts `Clear` on the first frame's hour), `docs/research/weather-save.md` (the save set), `src/climate/weathering.rs` (how the point snowpack integrates the clock: at most 24 h a step, a rewind integrates nothing, a load more than an hour off re-arms).

## Model adopted

1. **Recorded, not replayed.** The weather at a past hour cannot be recomputed: the scheduler's chain started on the first frame's hour, which is not stored; a backward or year-long jump keeps the old preset; a forced preset changes the blend but not the chain. So the history is *recorded* as the clock runs, from the `Weather` every system sees, and it is part of a save (it cannot be re-derived after a load).
2. **A day is the ETCCDI's daily record.** For each local calendar day: TG, the mean temperature; TN and TX, the minimum and maximum; the mean relative humidity; RR, the precipitation total (liquid-water equivalent, snow included); the snowfall total (water equivalent, `Weather::snow_fraction`); and the hours of the day that were observed. Temperatures are at the climate's reference elevation, as `Weather::temperature_c`.
3. **Integrated over game time, per frame.** Each frame adds the interval since the last one, weighted by its length in game hours, with this frame's `Weather` (the interval's right end, as `SurfaceWeathering` does). A frame across midnight is split at midnight between the two days. Means are the time-weighted means; TN and TX the extremes over the frames. Sums are kept in f64 while the day is open (a day at time scale 60 and 60 frames per second is 86 400 frames). The record does not depend on the frame rate beyond the sampling of a blend that changes over 20–60 minutes.
4. **Jumps follow the snowpack's rules.** A step of up to 24 game hours is integrated with this frame's weather (`SurfaceWeathering`'s `MAX_STEP_H`): the record and the snowpack agree about a skipped evening. A longer jump closes the day in progress and starts recording on the new day: the days between are not invented, they are absent. A rewind forgets the days from the new date on (they are now the future) and starts again. A load within an hour of the saved clock continues as if never saved; further, it is a jump.
5. **The indices.** A *wet day* has RR ≥ 1 mm and a *frost day* TN < 0 °C (ETCCDI; Thurner et al.), an *icing day* TX < 0 °C. They are public constants and per-day predicates, and a summary over a window counts them with the time-weighted means, the extremes and the totals. "Damp" in the issue is the wet day; "cold" the frost day.
6. **The window is calendar days before today.** "The last N days" is the completed days dated from today − N to yesterday; the day in progress is reported separately with its observed hours. A window of 0 is empty, a window longer than the record is the record, and a summary of no days is `None` — never a panic, never a mean of nothing.
7. **A bounded record.** The resource keeps the last 31 completed days by default (a month: the issue's week, and monthly indices); a game that wants a season inserts one with a longer capacity before the plugin.
8. **The query is in two places, one implementation.** `ClimateHistory` is a resource (it is saved); `Environment` answers `recent_days(n)` and `recent_climate(n)` from it, as it answers `weather()` from `Weather`, and answers empty when no `ClimatePlugin` records one.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| Wet day | RR ≥ 1.0 mm | ETCCDI (SDII, CWD, R95pTOT) |
| Frost day | TN < 0 °C | ETCCDI FD; Thurner et al. 2017 |
| Icing day | TX < 0 °C | ETCCDI ID; Thurner et al. 2017 |
| Longest step integrated | 24 game hours | `SurfaceWeathering::MAX_STEP_H` |
| Restore tolerance | 1 game hour | `SurfaceWeathering::RESTORE_TOLERANCE_H` |
| Default capacity | 31 days | a month (Model adopted 7) |

## GPU mapping

None: the record is CPU-side, from `Weather`, in `ElementsSystems::Weather` after `run_weather`. It is never stale: `Environment::snapshot_age` concerns the GPU readback, which the record does not read.

## Validation targets

The issue's acceptance criteria, each with its check:

1. **Deterministic** — "a headless test with a fixed `Climate` and a fixed seed produces a deterministic [summary] for a given day; the same inputs give the same output on two runs": an integration test runs the climate twice and compares the records exactly.
2. **Window 0** — well defined, no panic: empty days, `None` summary.
3. **A window longer than the history** — clamps to the record.
4. **Documented** — what a wet, frost and icing day mean numerically, in the rustdoc and the README.
5. **No existing API changes behaviour** — the existing tests pass; `Environment` takes the record as optional, so a world built by hand without it still works.

And the model's own:

- A day of constant weather is its own mean, its total the rate × 24 h; a frame across midnight splits by its hours.
- A jump over a day leaves a gap; a rewind forgets the days from the new date.
- A restored record continues the same days as the run it was saved from.
- Recorded at a quarter of the frame step, a week's daily means and totals are within the blend's sampling error of the coarse record.

## Deviations from this plan

1. **The issue's preferred `GrainCondition` is not adopted**: it is game vocabulary. Its fields map onto the summary: `damp_days` → wet days, `cold_days` → frost days, `mean_humidity` and `mean_temperature_c` → the time-weighted means.
2. **The wet-day threshold is the ETCCDI's 1 mm, not Richardson's 0.2 mm**: the index list is the coordinated international definition, and each day keeps its RR, so a game can apply another threshold.
3. **Humidity is the presets' blend.** `Weather::relative_humidity` takes ten preset values (0.15–0.99) blended through transitions, and `Climate::mean_humidity` is not used by the scheduler; the record's mean humidity is the mean of what the weather reported, not a physical moisture budget.

## Pending index

- Zhang et al. 2011 (DOI 10.1002/wcc.147): paywalled (Wiley TDM 400, PDF 403); the SciDB page did not load (DDoS-Guard challenge; a browser load stayed at `about:blank` after 60 s). The definitions used are the Expert Team's own page, quoted above.

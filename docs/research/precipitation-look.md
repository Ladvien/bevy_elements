# A game's look for rain and snow, a weather forecast, and a comparable snowpack (issues #25, #27, #23)

Three requests from a shipping game (Sanctuary: an orthographic camera pitched about 35°, a world in feet). The main one, #25: under such a camera the crate's physically sized rain reads as short, near-horizontal dashes ("sleet") and its snow as uniform 1–3 px dots ("screen noise"). The baseline (`/tmp/i26/before/iso_rain_12.png`, `iso_snow_12.png`, 4× crops `z_rain.png`, `z_snow.png`) confirms both: under `Rain` (4 mm/h, 7 m/s wind) every streak is 12–18 px long and leans about 40° from the vertical, all of the same grey; under `Snow` every flake is the same 3 × 3 px plus-shaped blob, because a 4–8 mm flake is 0.1–0.2 px at a 40 m view height and every one is clamped to the 1 px minimum half width. The two smaller requests: a forecast from the scheduler (#27) and `PartialEq` on `SurfaceWeathering` (#23).

## Sources

- **Tatarchuk 2006**, *Artist-Directable Real-Time Rain Rendering in City Environments* (SIGGRAPH Advances; corpus `s2006-advances-materials-6`, read for this note). The design goal is "a high degree of artistic control for the final look": "The artists specify the rain direction and speed in world-space to simulate different rainfall strength" (§rainfall, ll. 194–214), independent of any physical simulation; the dripping-drop particles are a "stretch billboard based on velocity", and "to reduce [repetition], randomly scale particle size and transparency" (l. 307).
- **Lagarde, Zanuttini et al. 2012**, *Water drop 2a – Dynamic rain and its effects* (Dontnod, *Remember Me*; `https://seblagarde.wordpress.com/2012/12/27/water-drop-2a-dynamic-rain-and-its-effects/`, read): "as raindrop is very faint in bright regions and tends to appear stronger in dark area, it is not a good idea to match exactly the reality in game"; the rain layers' slant follows "artistic values … rather than the wind direction"; splash effects "are often in the hand of the FX artists, [who] do whatever they feel looking good regardless of the physics".
- **Wang & Wade 2004**, *Rendering falling rain and snow* (SIGGRAPH sketch, in `precipitation.md`): a streak is the particle's velocity times an exposure, the model the crate's draw pass uses; a draw-only scale of the velocity components is a scale of that streak and nothing else.
- **Garg & Nayar 2007**, *Vision and rain* (in `precipitation.md`): a streak's on-screen length and orientation follow from the drop's velocity relative to the camera and the projection, which is what the projection test below computes.
- **Tiira, Moisseev et al. 2016**, *Ensemble mean density and its connection to other microphysical properties of falling snow as observed in Southern Finland*, AMT 9:4825 (`10.5194/amt-9-4825-2016`, downloaded through home-still, read §2–4): snowflake size distributions are normalised gammas `N(D) = N_w f(µ) (D/D_0)^µ exp(−Λ D)` with µ near 0 (exponential) and median volume diameters `D_0` up to 5 mm; an exponential's sizes have a coefficient of variation of 1. The crate draws flakes uniform in 4–8 mm, a coefficient of variation of 0.19, and under an orthographic view all of them at the same 1 px minimum: real snowfall varies far more in size than the drawn one.
- **Langer et al. 2004** (in `precipitation.md`): snow seen as a field of flakes at many sizes and depths; a field of identical dots carries no depth.
- **`view-anchor.md` Deviation 3**: the orthographic stylisation already in place (three exposures per streak, coverage 0.12, flakes at full opacity), whose captures are the baseline here.
- **`climate-weather.md`** and `src/climate/weather.rs`: the scheduler is a pure function `schedule(climate, southern, previous preset, hour index)` driven hour by hour by `run_weather`, so its coming hours can be computed without running it (#27).

## Model adopted

**`PrecipitationLook`** (resource, `init_resource`, defaults equal today's look exactly; draw-only: the simulation, the wetness, the splashes, the fog and the gameplay queries never read it):

1. **Rain streak.** The streak `s = (v − v_cam) · T` (T the exposure, × 3 under an orthographic view as before) is scaled per component before it is projected: `s′ = (s.x · w, s.y · f, s.z · w)` with `f = rain_fall_scale` and `w = rain_wind_scale` (Tatarchuk's artist-set direction and speed, here as factors on the simulated ones so the rain still turns with the wind). The drop's own position, life and collisions are untouched.
2. **Rain colour.** `rain_tint` multiplies the streak's radiance (rgb) and its opacity (alpha); the default `(1, 1, 1, 1)` is today's colour.
3. **Flake size.** Each flake's drawn half width is `max(true half width, flake_min_half_width_px) · exp(σ z − σ²/2)`, with `z` a standard normal from the particle's own hash (Box–Muller on a stream no other draw uses) and `σ = flake_size_spread`: a log-normal of mean 1 and coefficient of variation `√(e^{σ²} − 1)` (0.53 at σ = 0.5), applied after the minimum so a sub-pixel flake varies too. Dust sprites keep their own size (the issue's edge case).
4. **`PrecipitationLook::readable()`**, the stylised preset for orthographic and strategy cameras, chosen in the capture review and listed under Parameters.

**`WeatherDirector::forecast(climate, location, clock, hours)`** returns the preset the scheduler will have chosen for each of the next `hours` game hours, by running `schedule` forward from the director's chain exactly as `run_weather` does: from the chain's current preset and its last stepped hour (or a fresh chain starting Clear at the clock's hour, as on a first frame; or the chain restarted at the clock's hour after a jump of more than a year). The forced preset does not enter it: the scheduler keeps stepping underneath a force, and the forecast is that chain. It takes `&self`.

**`SurfaceWeathering: PartialEq`** (#23): every field is an `f32` or an `Option<f64>`; NaN never occurs in the model.

## Parameters

| Parameter | Default (today) | `readable()` | Source |
|---|---|---|---|
| `rain_fall_scale` | 1 | 4 | Tatarchuk 2006 (artist rain speed); capture review |
| `rain_wind_scale` | 1 | 0.5 | Tatarchuk 2006 (artist rain direction); Lagarde 2012 (slant by art, not wind) |
| `rain_tint` | `(1, 1, 1, 1)` | `(0.85, 0.9, 1.0, 0.7)` | Lagarde 2012 (rain reads stronger than physics in bright scenes); capture review (Deviation 1) |
| `flake_size_spread` σ | 0 | 0.5 | Tiira et al. 2016 (exponential snow PSD, CV 1); Tatarchuk 2006 (random size) |
| `flake_min_half_width_px` | 1 | 1.5 | capture review |

The `readable()` values are the capture review's; a change to them moves no default.

## GPU mapping

- Two `vec4`s more in the draw pass's `DrawParams` uniform (`look`: fall scale, wind scale, size spread, minimum flake half width; `tint`): no new binding, buffer or pass.
- Per vertex: three multiplies on the streak, one `exp`, one `log`, one `cos`, one `sqrt` for a flake's normal (six vertices per particle, the same instance), a four-component multiply for the tint. The cost is within the run-to-run spread of `elements_precip_draw`.
- Defaults: every factor is exactly 1 or the old constant (`exp(0 · z − 0) = 1`), so the default draw is bit-identical.

## What great looks like

- Under an orthographic camera pitched 30–35° in `Rain`: long, near-vertical, slightly wind-slanted streaks, reading at a glance as rain and not sleet, without greying the town; still thin; still lit by the sky and darker against a bright ground than a white dash.
- Under `Snow`: flakes of visibly different sizes, the big ones soft discs, the small ones pinpricks, so the field reads as falling snow with depth and not a uniform dot pattern; `Blizzard` dense without becoming noise.
- Perspective cameras, and every camera at the defaults, unchanged.

## Validation targets

1. **Defaults identical**: every existing GPU test passes; an isometric and a perspective capture at the defaults match the build before the change to within the particles' own randomness (they are re-randomised per run).
2. **Rain angle (#25 acceptance)**: with `rain_fall_scale 4, rain_wind_scale 0.5`, an orthographic camera pitched 35° and the `Rain` preset's drops (water-weighted Marshall–Palmer at 4 mm/h, Atlas–Ulbrich speeds, 7 m/s wind across the view, the worst case), the mean on-screen streak angle from vertical is below 25° (a unit test on the CPU mirror of the streak projection); at the defaults it is above 45° (today's sleet).
3. **Flake sizes (#25 acceptance)**: with `flake_size_spread 0.5` the drawn flake half widths over 10 000 particles have a coefficient of variation of at least 0.3, and a mean within 5 % of the unspread size (a unit test on the CPU mirror of the size factor); at 0 every factor is exactly 1.
4. **Visual**: `isometric` with `ELEMENTS_PRECIPITATION=readable` under `Rain`, `Thunderstorm`, `Drizzle`, `Snow` and `Blizzard` at noon and dusk, and the game look: GREAT in the critique, 4× crops of streaks and flakes.
5. **Forecast (#27 acceptance)**: the 24-hour forecast equals the presets the director schedules when the clock is advanced hour by hour with no force; it is the same while a preset is forced; calling it does not change the director.
6. **`SurfaceWeathering` (#23 acceptance)**: a stepped pack round-trips through serde and compares equal.
7. **Cost**: `elements_precip_draw` at High in a 25 mm/h `Thunderstorm` within 5 % of before.

## Deviations from this plan

1. **`readable()`'s rain opacity is 0.7, not 1.3.** At 1.3 the 25 mm/h `Thunderstorm` became an opaque white wall over the town (ground mean 153 DN against 140 physical; critique round 1); at 0.7 it is 147 against 149 physical, and the 4 mm/h `Rain` streaks still read because the fall scale lengthens them. A brighter crate tint (1.15–1.35, round 3) greyed the crate-lit town into a veil, so it was not taken. Lagarde's point that rain should read stronger than physics in bright scenes is a per-game call: the `isometric` example's game look (its own bright key light) sets `rain_tint (2.4, 2.5, 2.8, 0.8)` on top of `readable()`, which is what the field is for.
2. **The flake size factor's normal is clamped to ±2σ.** Unclamped, the log-normal's tail drew a few flakes four to five times the mean as soft blobs (critique round 1). The clamp keeps 95 % of the distribution; at σ = 0.5 the measured coefficient of variation is still above 0.3 and the mean within 5 % (`a_size_spread_varies_the_flakes_about_their_own_size`).
3. **Measured:** under an orthographic camera pitched 35° the mean streak angle from vertical is 63.4° physical and 15.3° readable (`a_readable_rain_stands_upright_under_an_orthographic_camera`). Cost: `elements_precip_draw` at High, 3840 × 2160, `Thunderstorm`, minimum over two runs each: 0.122 / 0.126 ms before, 0.123 / 0.115 ms with `readable()` (the GPU at 1695–1755 MHz, P2): within the run-to-run spread. Defaults: the isometric `Rain` and `Snow` captures' mean colour moved by at most 0.02 DN against the build before the change.
4. **Night and storms under an orthographic camera are unchanged by this note and stay open:** a 25 mm/h storm hides the town under both looks, and the isometric town at 22 h under `Rain` renders black (mean 1 DN) under both. Neither is the draw's style.

## Pending index

- `10.5194_amt-9-4825-2016` (Tiira et al. 2016): downloaded to home-still, read from its PDF text; not converted or indexed.
- Lagarde 2012: a web article, no DOI.

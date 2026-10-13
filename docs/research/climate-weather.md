# Climate, seasons and weather — research note

## Sources

- Richardson, C. W. (1981). Stochastic simulation of daily precipitation, temperature, and solar radiation. *Water Resources Research* 17(1):182–190. DOI `10.1029/WR017i001p00182`. Stem `10.1029_wr017i001p00182`. Read from the PDF with `pdftotext` (scribe markdown was not available): Markov chain-exponential precipitation model (eqs. 1–3), Fourier-series seasonal parameters (eq. 10), wet/dry-conditioned temperature means (Tables 2–4), multivariate residual model (eqs. 4, 8, 9).
- ISO 2533:1975 *Standard Atmosphere* (identical to the ICAO Standard Atmosphere, Doc 7488): sea-level 288.15 K, temperature gradient −6.50 K/km from 0 to 11 km. Table read from `https://cdn.standards.iteh.ai/samples/7472/c203e9121d4c40e5bdc98844b1a1e2f4/ISO-2533-1975.pdf` (rows "−2.00 … 301.15 … −6.50", "0.00 … 288.15 … −6.50").
- Jennings, K. S., Winchell, T. S., Livneh, B., Molotch, N. P. (2018). Spatial variation of the rain–snow temperature threshold across the Northern Hemisphere. *Nature Communications* 9:1148. DOI `10.1038/s41467-018-03629-7`, `https://www.nature.com/articles/s41467-018-03629-7`. 17.8 M observations: the air temperature at which rain and snow fall with equal frequency averages 1.0 °C, with a 95 % range of −0.4 to 2.4 °C across stations; maritime sites are coolest, continental warmest; relative humidity and elevation explain most of the variation. (Figures taken from the abstract/search summary; the full text was not opened.)
- Parton, W. J., Logan, J. A. (1981). A model for diurnal variation in soil and air temperature. *Agricultural Meteorology* 23:205–216. Daily air-temperature maximum in the mid-afternoon (the model's default time of maximum is 14:00–16:00 local time; GLOBE example peaks at 16:00 local standard time, `https://www.globe.gov/explore-science/scientists-blog/archived-posts/sciblog/2008/02/27/how-the-temperature-varies-during-the-day-and-night/comment-page-1/index.html`). Secondary summary only; paper not opened.
- Koschmieder visibility law, V = ln(1/ε)/σ with contrast threshold ε = 0.02 → V = 3.912/σ. Defined and used in `docs/research/fog.md`. Bevy 0.20's own constant is ε = 0.05 (`bevy_pbr-0.20.0/src/fog.rs:437`, `koschmieder` at `:464`).
- Steele, G. L., Lea, D., Flood, C. H. (2014). Fast splittable pseudorandom number generators. OOPSLA. DOI `10.1145/2660193.2660195`. SplitMix64 finaliser constants; test vector verified below.
- Meteorological seasons (Dec–Feb winter in the northern hemisphere, whole-month boundaries) — convention used by the plan's `Season`.

## Model adopted

**What Richardson 1981 gives.** Weather is generated day by day. Precipitation occurrence is a first-order two-state Markov chain (wet: ≥ 0.2 mm/day), fully defined by $P_i(W|W)$ and $P_i(W|D)$, with $P_i(D|W)=1-P_i(W|W)$ and $P_i(D|D)=1-P_i(W|D)$. Wet-day amounts are exponential, $f(y)=\lambda e^{-\lambda y}$ (mean $1/\lambda$). Daily maximum temperature, minimum temperature and solar radiation are generated from a multivariate lag-1 autoregressive process on standardised residuals $\chi_i(j)$, then de-standardised with a mean $\mu_j$ and standard deviation $\sigma_j$ that are conditioned on the wet/dry state of that day, $x = \mu_j^{wd} + \sigma_j^{wd}\chi$. Every parameter ($P(W|W)$, $P(W|D)$, $\lambda$, $\mu$, $\sigma$) has a seasonal cycle represented by a Fourier series:

$$ v_i = C_0 + \sum_{j} C_j \cos\!\Big(\tfrac{j\,i}{T} - \theta_j\Big),\qquad T = 365/2\pi \quad (\text{Richardson eq. 10}) $$

with zero to three significant harmonics per parameter and exactly one harmonic for the temperature means and standard deviations (Tables 2–4). Wet days are cooler in Tmax: Temple TX dry/wet $C_0$ = 25.6/22.3 °C, Atlanta GA 21.5/20.0 °C, Spokane WA 14.9/12.2 °C (Δ = −3.3, −1.5, −2.7 °C); Tmin has no detectable wet/dry difference.

**What the plan uses (mapping).**

| Richardson 1981 | `bevy_elements` plan 1.2 |
|---|---|
| Two-state wet/dry Markov chain | Ten-state preset graph, one transition draw per game hour. Stay probability 0.85, else a weighted draw from graph neighbours |
| $P(W\|W)$, $P(W\|D)$ seasonal Fourier series | `Climate::wet_hour_probability` scales all wetter-preset neighbour weights by $p_{wet}/0.35$ (constant in the year); season enters through the temperature gates (Snow/Blizzard weight 0 above 1 °C, Rain/Drizzle/Thunderstorm weight 0 below −1 °C) and the Thunderstorm summer factor |
| Exponential wet-day amount | Fixed per-preset intensity (mm/h table). No amount draw |
| $\mu(t)$ = one harmonic, wet/dry conditioned | $T = T_{mean} + A_{ann}\cos\frac{2\pi(doy-d_{warm})}{365.25} + A_{diu}\cos\frac{2\pi(h-15)}{24} + \Delta T_{preset}$; the wet/dry conditioning is the per-preset $\Delta T$ |
| Residual AR(1) | None; the blend between presets (smoothstep over 20–60 game minutes) supplies temporal correlation |

Temperature, with elevation:

$$ T(h,\mathrm{doy},z) = T_{mean} + A_{ann}\cos\frac{2\pi(\mathrm{doy}-d_{warm})}{365.25} + A_{diu}\cos\frac{2\pi(h-15)}{24} + \Delta T_{preset} - \Gamma\,\frac{z - z_{ref}}{1000},\quad \Gamma = 6.5\ \mathrm{K\,km^{-1}} $$

($T$ in °C, $h$ in local hours 0–24, $z$ in m). $d_{warm}$ is shifted by +182 for latitude < 0.

**Where the plan diverges from Richardson, and why.**

1. *Hourly vs daily step.* Richardson steps once per day because his inputs feed daily hydrologic models. The plan steps hourly because the game clock runs at a time scale of 60–600 and the visible state (sun, fog, rain onset, flash storms) changes within a day; a daily-constant weather could never produce a morning fog that burns off or an afternoon thunderstorm (the plan's Fog weight ×3 for 04:00–09:00 and Thunderstorm summer factor need sub-daily resolution). With stay probability $p=0.85$ the dwell time in a preset is geometric with mean $1/(1-p) = 6.67$ h, so a wet spell of 2–3 presets lasts roughly a day, the same order as Richardson's daily wet spells ($1/(1-P(W|W))$ days).
2. *Preset graph instead of amounts.* The plan needs discrete visual states (fog, snow, dust) that a wet/dry + exponential amount pair cannot express. The amount distribution is replaced by the per-preset intensity table (Rain 4 mm/h, Thunderstorm 25 mm/h, …), which makes the amount deterministic given the preset.
3. *No autoregressive temperature residual.* Not needed: the diurnal/annual harmonics plus the preset offset give the intended structure; the sign of the wet-day offset matches Richardson (wet cooler than dry by 1.5–3.3 °C Tmax); the plan's Rain −3 °C matches the upper end, Thunderstorm −5 °C and Blizzard −6 °C are intentionally larger event offsets.
4. *Seasonal precipitation probability.* Richardson makes $P(W|W), P(W|D)$ seasonal. The plan keeps it implicit through the temperature gates. This is a plan simplification, kept as is.

**Snow/rain partition.** `snow_fraction(T) = clamp((2 − T)/3, 0, 1)` — 1 at ≤ −1 °C, 0 at ≥ 2 °C, linear. Consistency with Jennings et al. 2018: the ramp's 50 % point is 0.5 °C, the study's mean 50 % threshold is 1.0 °C, and the 95 % station range −0.4…2.4 °C lies inside the ramp's span (−1…2 °C) except the top 0.4 °C. The ramp is therefore consistent as a continental–maritime compromise (it is 0.5 °C cooler than the study mean, so it slightly favours rain). Jennings finds the threshold depends on humidity and elevation; the plan does not model that and records it as a limitation, not a deviation.

**Diurnal maximum at 15:00.** Within the 14:00–16:00 window of Parton & Logan 1981; a symmetric cosine cannot reproduce their sine-exponential asymmetry (slow evening decay), an accepted simplification.

**Season curves** (plan 1.2): smoothstep in day-of-year; northern-hemisphere dates, +182 days in the southern hemisphere.

**Scheduler hash.** $h = \mathrm{mix}\big(\mathrm{seed} \oplus (i\cdot \mathtt{0x9E3779B97F4A7C15}) \bmod 2^{64}\big)$ where mix is the SplitMix64 finaliser: $z \leftarrow (z \oplus z\gg 30)\cdot \mathtt{0xBF58476D1CE4E5B9}$; $z \leftarrow (z \oplus z\gg 27)\cdot \mathtt{0x94D049BB133111EB}$; $z \oplus z\gg 31$ (all mod $2^{64}$). The plan says "SplitMix64 over (seed ^ hour_index·0x9E37…)"; this note fixes the reading as *finaliser applied to that value with no additional state increment*. (With increment-then-mix, `hash64(0, 0)` would equal `mix(G)`; the finaliser-only reading gives `hash64(0, 1) = mix(G)`.) This is the plan's reading, recorded here so the golden is unambiguous.

**Unspecified in the plan, fixed here as [INFERENCE]:** base neighbour weights are 1.0 before the multipliers above; `hour_index` counts game hours since the clock's start instant (the golden uses 2026-01-01 00:00 as index 0); a neighbour whose weight is 0 is removed, and if all are 0 the preset stays.

## Parameters

| name | symbol | default | unit | source |
|---|---|---|---|---|
| Mean temperature (temperate / alpine / desert / tropical / tundra) | $T_{mean}$ | 11 / 2 / 24 / 27 / −8 | °C | plan 1.2 preset table (no external source; presets are design values, plausible against Köppen climates) |
| Annual amplitude | $A_{ann}$ | 9 / 10 / 9 / 2 / 18 | °C | plan 1.2; form: single annual harmonic, Richardson 1981 eq. 10 and Tables 2–4 (one significant harmonic for temperature) |
| Diurnal amplitude | $A_{diu}$ | 5 / 7 / 14 / 6 / 4 | °C | plan 1.2; harmonic form: Parton & Logan 1981 |
| Warmest day of year | $d_{warm}$ | 200 (+182 if latitude < 0) | day | plan 1.2; northern-hemisphere mid-July |
| Hour of daily maximum | $h_{max}$ | 15 | h local | Parton & Logan 1981 (14:00–16:00) |
| Mean relative humidity | $RH_{mean}$ | 0.75 / 0.70 / 0.25 / 0.85 / 0.80 | 1 | plan 1.2 |
| Wet-hour probability | $p_{wet}$ | 0.35 / 0.40 / 0.03 / 0.55 / 0.25 | 1 | plan 1.2; wet-state concept: Richardson 1981 (wet ≥ 0.2 mm) |
| Storm / fog / dust share | $s_{storm}, s_{fog}, s_{dust}$ | temperate 0.05/0.08/0; alpine 0.04/0.06/0; desert 0.01/0/0.08; tropical 0.20/0.03/0; tundra 0/0.05/0 | 1 | plan 1.2 |
| Mean wind | $\bar u$ | 4 / 6 / 5 / 3 / 7 | m/s | plan 1.2 |
| Prevailing wind direction | — | $+X$ | unit XZ vector (blows toward) | plan 1.2 |
| Reference elevation | $z_{ref}$ | 0 | m | plan 1.2 |
| Lapse rate | $\Gamma$ | 6.5 | K/km | ISO 2533:1975 / ICAO standard atmosphere, 0–11 km |
| Snow fully falling below | $T_s$ | −1 | °C | plan 1.2; consistent with Jennings 2018 (95 % range −0.4…2.4 °C lies above it) |
| Rain fully falling above | $T_r$ | 2 | °C | plan 1.2; Jennings 2018 95 % upper bound 2.4 °C |
| Forced-snow temperature cap | — | −2.0 | °C | plan 1.2 (guarantees `snow_fraction() = 1` since $-2 \le T_s$) |
| Stay probability | $p_{stay}$ | 0.85 | 1 | plan 1.2; mean dwell 6.67 h (derived above) |
| Preset wetness reference | — | 0.35 | 1 | plan 1.2 (weight = $p_{wet}/0.35$) |
| Thunderstorm summer factor | — | 1.0 summer, 0.3 otherwise | 1 | plan 1.2 |
| Fog dawn boost | — | ×3 for 04–09 h and RH > 0.85 | 1 | plan 1.2 (radiation fog) |
| Snow gate | — | weight 0 if climate T > 1 | °C | plan 1.2; 1 °C = Jennings 2018 mean threshold |
| Rain gate | — | weight 0 if climate T < −1 | °C | plan 1.2; −1 °C = ramp lower end |
| Transition duration | — | 20–60 (hash-drawn) | game min | plan 1.2 |
| Hash constants | — | `0x9E3779B97F4A7C15`, `0xBF58476D1CE4E5B9`, `0x94D049BB133111EB` | — | Steele, Lea, Flood 2014 (SplitMix64) |
| Preset targets (cover, mm/h, RH, wind m/s, gust, visibility m, storm/min, dust, ΔT °C) | — | plan 1.2 table (Clear 0.05, 0, 0.50, 3, 0.2, 40000, 0, 0, +2 … DustStorm 0.30, 0, 0.15, 16, 0.8, 800, 0, 0.9, +3) | mixed | plan 1.2; sign of ΔT for wet presets agrees with Richardson 1981 Tables 2–4 (wet-day Tmax lower by 1.5–3.3 °C) |
| Wind speed scale | — | `mean_wind_ms / 4` | 1 | plan 1.2 |
| Wind direction jitter | — | ±30° (hash) | deg | plan 1.2 |
| Wet-day threshold in Richardson (not used) | — | 0.2 | mm/day | Richardson 1981 (reference only) |

## GPU mapping

CPU only. Climate, season curves, the scheduler, `Weather`, `WeatherDirector`, `SurfaceWeathering` run on the CPU in `Update` inside `ElementsSystems::Weather`. They are deterministic and seeded. Outputs reach the GPU only as uniforms consumed by other domains.

## Validation targets

1. **Forced transition (plan M1, `tests/walk.rs`, `MinimalPlugins`, no window, no GPU):** `WeatherDirector::force(Thunderstorm, 60 s)` at `time_scale` 1; after 61 s of `TimeUpdateStrategy::ManualDuration` steps `Weather.precipitation_mm_h == 25.0` exactly, and exactly one `WeatherChanged { to: Thunderstorm, forced: true }` was emitted. The blend is smoothstep, so at 30 s the blend factor is $s(0.5) = 0.5$ and precipitation is the mean of the previous preset's and 25.0 (e.g. from Clear: 12.5 mm/h) ±1e-3. Source: plan 1.2/Verification M1; smoothstep: plan 1.2.
2. **`release`** returns control to the scheduler: after `release(Duration::ZERO)` and the next hour boundary, `WeatherDirector::is_forced() == false` and `current()` equals the scheduler's preset for that hour index. Source: plan 1.2.
3. **Snow fraction endpoints:** `snow_fraction()` = 1.0 at $T$ = −1 °C, exactly 0.0 at 2 °C, 0.5 at 0.5 °C, 1.0 at −10 °C, 0.0 at 30 °C. Source: plan 1.2 (ramp); ramp consistency with Jennings 2018 above.
4. **Forced Snow at `Climate::tropical()`:** by the formula, January 15:00 at doy 1 with mean 27, amplitude 2, warmest day 200 gives $T = 27 + 2\cos(2\pi(1-200)/365.25) + 6\cos 0 + 0 = 31.08$ °C (computed); the forced-snow cap gives effective temperature $\min(T-4, -2) = -2$ °C, so `snow_fraction() == 1.0`. Source: plan 1.2/Verification M1.
5. **Temperature harmonics (temperate, `warmest_day` 200, no weather offset):** $T$(doy 200, 15:00) = 25.0 °C; $T$(doy 200, 03:00) = 15.0 °C; $T$(doy 17, 03:00) = −3.00 °C (−2.9998 computed). Elevation: +1000 m above $z_{ref}$ lowers $T$ by exactly 6.5 °C. Sources: plan 1.2 formula; lapse rate ISO 2533.
6. **Hash vector:** `mix(0x9E3779B97F4A7C15) == 0xE220A8397B1DCDAF` (the first output of the reference SplitMix64 generator for seed 0), hence `hash64(0, 1) == 0xE220A8397B1DCDAF`. Computed values: `hash64(42, 0) == 0xA759EA27D4727622`, `hash64(42, 1) == 0xBDD732262FEB6E95`, `hash64(42, 8759) == 0xAA5EC7C5CF517539`. Source: Steele, Lea, Flood 2014 for the algorithm; the vectors were computed in this research session with an independent Python implementation of the finaliser.
7. **Golden `the_weather_schedule_is_frozen`:** `Climate::temperate()`, seed 42, start 2026-01-01 00:00, 8760 hourly presets (`hour_index` 0…8759), each preset's discriminant byte (declaration order Clear = 0 … DustStorm = 9) folded with 64-bit FNV-1a (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`) equals a blessed constant, blessed once after implementation. The doc comment says a red result means the scheduler model moved. Source: plan Verification M1.
8. **Scheduler statistics (informational, assert only loose bounds):** with $p_{stay} = 0.85$ the mean run length over 8760 h is $6.67 \pm 10\,\%$ h in a long run. Source: derivation above.

## Deviations from this plan

None.

## Pending index

`distill_index` skipped for every stem below because distill/Qdrant is down; scribe markdown missing for the first.

- `10.1029_wr017i001p00182` (Richardson 1981; read from PDF)
- `10.1038_s41467-018-03629-7` (Jennings 2018; not downloaded)
- `10.1145_2660193.2660195` (Steele, Lea, Flood 2014; not downloaded)

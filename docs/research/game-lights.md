# A game lights its own night: the exposure, and what a light has to be to read

The crate simulates a day and a night, a sun and a moon, a sky and a metered exposure. A game
that puts a character in that world at night has **nothing to light them with**: no query tells
it how bright the night is in the camera's own terms, and nothing tells it what power its torch
has to be to read against the exposure the crate is metering.

The crate's *own* machinery for this exists and is private. `src/lightning/visual.rs` writes the
bolt's `PointLight` in physical units from the flash's luminance
(`light.intensity = FLASH_LUMENS * active.mean * u * u`) and the bolt's *material* colour from
the same flash through the camera's `Exposure` (`layer_color(active.mean_first, exposure)`). A
game cannot do any of that: `layer_color` and `display_value` are private, the flash's luminance
is a private constant, and the exposure is read inside the crate's own system.

## Sources

- **Patry, J. (2021)**, *Real-time samurai cinema*, SIGGRAPH *Advances* (corpus
  `s2021-advances-pdf-66-mb`, the "Tonemapping | Exposure" section, lines 2990–3060, and the
  low-light vision section, lines 3855–3900). Used:
  - "Lights are authored using **physical units**, and we support energy conserving area
    lights."
  - "we also switched to a **primarily illuminance-based exposure system**, which takes albedo
    out of the equation altogether", with luminance used "only if the highlight luminance
    exceeds a threshold" — for example, "fire particles, bright specular highlights".
  - The *feedback loop* they had before: "Our assumption was that the feedback loop between
    luminance-based exposure and albedo was largely responsible for this… resulted in albedo
    values that were too dark". The crate meters on illuminance, so it does not have that
    loop, and **a game's own light does not move the camera's exposure** — which is the stable
    behaviour, and is why the gap is not an exposure bug.
  - The low-light vision section: "Rods (low-light vision, monochromatic)… the result is that
    low-light vision is not 'black and white', but there's a color shift introduced by the
    rods as they take over from the cones, which is sometimes called the **Purkinje shift**."
- **The crate's own `celestial.md`**: the exposure's model — `ev100_for(lux)`,
  `exposure_for_adaptation(la)`, and the meter's `scene_illuminance`, which is the sun's and the
  moon's clear-sky illuminance times the cloud's total, plus the night sky's floor of
  0.0005 lx.
- **The crate's own `night-sky-light.md`**: the illuminance floor and the *scotopic* colour, and
  the note that "a game at night wants its own lights (a torch, a street lamp, a window), and
  the crate gives a game nothing to place them with".
- **The crate's own `lightning/visual.rs`**: the machinery this note makes public —
  `display_value(luminance, exposure)` and the bolt's `PointLight` in physical units.
- **Bevy 0.20 source** (`$R` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`):
  `$R/bevy_camera-0.20.0/src/exposure.rs` — `Exposure::exposure()` is
  `2^(−EV₁₀₀) / 1.2` divided by the white point's luminance; and
  `$R/bevy_light-0.20.0/src/point_light.rs` — a `PointLight`'s `intensity` is in candela and its
  illuminance at a distance `d` is `I / d²`.
- **CIE 1951** and the scotopic curve, through Jensen et al. 2001 §7 and the crate's
  `celestial.md`: the *scotopic* colour, which the crate already uses for the night sky and
  which a game's light at night should match.

## Model adopted

**1. The crate publishes the camera's exposure, so a game can balance a light against it.**

A game that places a `PointLight` needs to know the camera's exposure to answer "how bright
is a surface 3 m from my lamp, in the camera's terms?". The crate meters the exposure and keeps
it in the camera's `Exposure` component, which a game can read — but the crate's own
`Exposure` is a *component Bevy owns*, and the game has to know that the exposure is the
illuminance-based one and how it is computed.

So the crate publishes:

- **`ElementsExposure`** — a resource holding the camera's EV100 and its exposure factor,
  refreshed every frame. The celestial plugin writes the *followed* camera's exposure there,
  whether the crate metered it or the game drove it (`ElementsCamera::auto_exposure` off).
- **`Environment::exposure()`** — the same factor, read from a gameplay system.

**2. The crate publishes the scene-linear value, in both directions.**

The exposure is applied inside Bevy's PBR shader *before* the tonemapper
(`bevy_pbr-0.20.0/src/render/pbr_functions.wesl:850`), so a scene luminance times the exposure
is the tonemapper's input — the number the picture is built from. A scene-linear value of
**0.18 is 18 % grey, the middle of the picture**, and the crate's metering is built so that an
18 % grey surface reads at 0.18 *at every hour* (measured below).

- **`display_value(luminance_cd_m2)`** — the scene-linear value a luminance has under the
  camera's exposure. The forward direction: given the light you have, how bright does it read?
- **`luminance_for_display(display)`** — the inverse. The direction a game authors in: "I want
  the ground under my lamp to read at 0.18, so it must be lit to this many cd/m²."

**3. The crate publishes the light's power, because the units are the whole difficulty.**

- **`light_intensity(luminance, distance, albedo)`** — the `PointLight::intensity` or
  `SpotLight::intensity` (Bevy's **luminous power in lumens**) that lights a surface at that
  distance to that luminance.
- **`light_illuminance(intensity, distance)`** — the inverse of the falloff.

**4. The colours are the crate's, so a game does not have to know the physics.**

The *scotopic* colour (`NIGHT_SKY_COLOUR`, (0.72, 0.95, 1.28) normalised to luminance 1) is
what the eye reads at night, and a game's light should be that colour or the scene reads as two
different nights. So the crate publishes it.

## The numbers the model produces

The crate's metering, measured through the crate's own `ev100_for` and `exposure_for_adaptation`:

| the scene | the illuminance | the EV100 | the exposure | 1 cd/m² reads at |
|---|---|---|---|---|
| the moonless night | 0.0005 lx | −4.75 | 22.370 | 22.370 |
| a bright moonlit night | 1 lx | −0.68 | 1.338 | 1.338 |
| the noon sun | 10 000 lx | 9.75 | 0.001 002 | 0.001 002 |

An 18 % grey surface reads at **0.18 at every one of them**: the metering is exactly the
standard "middle grey" one.

**The consequence for a game's light, and the thing worth knowing:** the power a light needs
scales *inversely* with the exposure. A surface of albedo 0.14 that should read at 0.06 wants

| the hour | the exposure | the luminance | the power at 1.92 m |
|---|---|---|---|
| noon | 0.001 002 | 59.89 cd/m² | 62 253 lm |
| midnight | 22.370 | 0.002 68 cd/m² | 2.8 lm |

A game that hard-codes a lamp's lumens is wrong by five orders of magnitude between noon and
midnight. A game that asks the crate is never wrong.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The scene-linear value a surface reads at | the game's own choice | — |
| The illuminance floor | 0.0005 lx | the crate's `ILLUMINANCE_FLOOR_LUX`; Jensen et al. 2001 Fig. 2's sum |
| The scotopic colour | `(0.72, 0.95, 1.28)`, luminance 1 | iteration 31; Jensen §7's blue shift |
| A light's falloff | `E = Φ / 4πd²` | Bevy's `light.rs:574` and `pbr_lighting.wesl`'s `getDistanceAttenuation` |
| The largest scene-linear value | 4096 | the crate's own `DISPLAY_MAX`; `Rgba16Float` stays finite |

## GPU mapping

- **Nothing.** The crate's exposure is already a number on the CPU, and every answer is
  arithmetic on it. No new pass, no new buffer, no new binding, and the fragment stage's
  storage-buffer budget is untouched.

## What great looks like

- **A game's light reads.** The game places a lamp, asks the crate what power it needs, and the
  ground under it is a pool of light with the night around it — not a black screen and not a
  white spot.
- **The night is one night.** The lamp's colour and the sky's colour are the same scotopic
  blue-green, so the light does not read as a different scene laid over the sky.
- **The day is unchanged.** The exposure is the crate's, and the crate meters on illuminance, so
  a game's light does not move it (Patry 2021: the illuminance-based system "takes albedo out
  of the equation altogether").
- **The light reads the same at every hour.** The same light, balanced once, is the same
  picture at noon and at midnight. A game never re-tunes it.

## Validation targets

1. **The round trip holds**: `display_value(luminance_for_display(v)) == v` for `v` in the
   display range, under the crate's own exposure. *(`celestial::light::tests::the_round_trip_holds`,
   over four exposures.)*
2. **The exposure is the crate's**: `exposure()` equals the camera's `Exposure::exposure()`, and
   it is the illuminance-based one — a scene of `0.0005 lx` gives a finite exposure and a
   scene of 10 000 lx gives a much smaller one. *(`tests/game_lights.rs`.)*
3. **A light's power is right**: the illuminance at `d` from an `intensity` computed by
   `light_intensity` is the luminance asked for, within 0.1 %. *(`celestial::light::tests`.)*
4. **The units match Bevy's own calibration**: a 1 000 000-lumen cinema light at 5 m on a
   0.5-albedo surface reads in the scene-linear range at EV100 12, Bevy's "very overcast day".
   *(`celestial::light::tests::the_units_match_bevys_own_calibration`.)*
5. **The colour is the scotopic one**: the published colour's blue is above its green and its
   red, and the crate's night sky uses the same one.
6. **The day is untouched**: the noon capture's mean within 0.5 DN of the baseline's.
   *Measured: 143.03 against 142.62, a 0.41 DN difference, with the standard deviation within
   0.03.*
7. **The night reads**: the frame's standard deviation rises from 2.44 DN to 14.06.

## Deviations from this plan

1. **A spot light takes the same power as a point light, not less.** The plan modelled a
   `SpotLight` as concentrating its lumens into its cone, dividing the power by the cone's
   solid angle. Bevy does not: it maps *both* a point light's and a spot light's power to
   `Φ / 4π` — "we choose to use the same 4\*PI divisor in both cases so that toggling between
   point light and spot light keeps lit areas lit equally"
   (`bevy_pbr-0.20.0/src/render/light.rs:711`), where Filament would use `π` for a spot — and
   the cone only *attenuates* the edges (`spot_scale` and `spot_offset` in
   `pbr_lighting.wesl`). So the brightness on the axis is the same and the cone decides only
   *where* the light lands. The API has **one** power function for both, and the plan's
   separate `spot_light_intensity` is gone.
2. **The value is named a *scene-linear* value, not a *display* value.** The plan called it a
   display value, but Bevy applies the exposure *inside* the PBR shader and the tonemapper
   after it (`bevy_pbr-0.20.0/src/render/pbr_functions.wesl:850`), so the number is the
   tonemapper's *input*, not the final pixel. The crate's own `display_value` name is kept for
   the function (it is the crate's existing name, and the lightning's layers use it), but every
   document now says what it is: the scene-linear value, where 0.18 is 18 % grey.
3. **The exposure is published from the camera, not from the sky.** The plan had the crate
   publish the metered target. A game needs the exposure the camera *really has*, which is
   different when the game drives the exposure itself (`ElementsCamera::auto_exposure` off).
   So the celestial plugin publishes the followed camera's EV100, and falls back to the metered
   target only when there is no camera at all.
4. **`ElementsExposure` is a resource of its own.** The plan did not name it. A resource is
   what the crate's other published state is (`Weather`, `SunState`, `WindState`), and it lets
   a game read the exposure from a system that does not hold an `Environment`.

## Pending index

- Patry 2021 (corpus `s2021-advances-pdf-66-mb`) is **in the corpus** and was read for this
  note. **Not indexed.**
- `distill_index` is skipped for the above.


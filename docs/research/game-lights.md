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

So the crate publishes, on the `Environment`:

- **`exposure()`** — the camera's exposure factor, the same number `Exposure::exposure()`
  returns, so a game can convert a scene luminance to a display value.
- **`display_value(luminance_cd_m2)`** — the display value a scene luminance of that many
  cd/m² has under the camera's current exposure. This is the crate's own `display_value`, made
  public, and it is the *forward* direction: given the light you have, how bright does it read?

**2. The crate publishes the *inverse*, because that is the direction a game authors in.**

The question a game actually asks when placing a torch is: "what intensity should this lamp
have so that it lights the ground to a readable brightness?" The answer needs the exposure and
the surface's albedo, and the crate knows the exposure.

So:

- **`luminance_for_display(display)`** — the scene luminance, in cd/m², that a surface reads at
  a given display value under the camera's exposure. The inverse of `display_value`.
- **`point_light_intensity(luminance_cd_m2, distance_m)`** — the `PointLight::intensity` (in
  candela) that gives a surface at that distance the given scene luminance, from `E = I / d²`.
  With the surface's albedo folded in, so the argument is the luminance *of the light falling
  on the surface* and the game can divide by its own albedo.

Together these answer the whole question: "the ground here is 0.0005 lx and the camera is
exposed for it; I want my lamp to put the ground at display value 0.3, so it needs
`I = 0.3 / (exposure · albedo) · d²` candela."

**3. The colours are the crate's, so a game does not have to know the physics.**

The *scotopic* colour (`NIGHT_SKY_COLOUR`, (0.72, 0.95, 1.28) normalised to luminance 1) is
what the eye reads at night, and a game's light should be that colour or the scene reads as two
different nights. So the crate publishes it.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The display value a surface reads at | the game's own choice | — |
| The illuminance floor | 0.0005 lx | the crate's `ILLUMINANCE_FLOOR_LUX`; Jensen et al. 2001 Fig. 2's sum |
| The scotopic colour | `(0.72, 0.95, 1.28)`, luminance 1 | iteration 31; Jensen §7's blue shift |
| A point light's falloff | `E = I / d²` | Bevy's `PointLight`; the crate's own `u * u` scaling in `lightning/visual.rs` |

## GPU mapping

- **Nothing.** The crate's exposure is already a number on the CPU, and every answer is
  arithmetic on it. No new pass, no new buffer, no new binding, and the fragment stage's
  storage-buffer budget is untouched.

## What great looks like

- **A game's torch reads.** The game places a lamp, asks the crate what power it needs, and the
  ground under it is a pool of light with the night around it — not a black screen and not a
  white spot.
- **The night is one night.** The lamp's colour and the sky's colour are the same scotopic
  blue-green, so the light does not read as a different scene laid over the sky.
- **The day is unchanged.** The exposure is the crate's, and the crate meters on illuminance, so
  a game's light does not move it (Patry 2021: the illuminance-based system "takes albedo out
  of the equation altogether").

## Validation targets

1. **The round trip holds**: `display_value(luminance_for_display(v)) == v` for `v` in the
   display range, under the crate's own exposure.
2. **The exposure is the crate's**: `exposure()` equals the camera's `Exposure::exposure()`, and
   it is the illuminance-based one — a scene of `0.0005 lx` gives a finite exposure and a
   scene of 130 000 lx gives a much smaller one.
3. **A point light's power is right**: the illuminance at `d` from an `intensity` computed by
   `point_light_intensity` is the luminance asked for, within 0.1 %.
4. **The colour is the scotopic one**: the published colour's blue/green ratio is above 1.2, and
   the crate's night sky uses the same one.
5. **The day is untouched**: the noon capture's mean within 0.5 DN of the baseline's.

## Deviations from this plan

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

## Pending index

- Patry 2021 (corpus `s2021-advances-pdf-66-mb`) is **in the corpus** and was read for this
  note. **Not indexed.**
- `distill_index` is skipped for the above.

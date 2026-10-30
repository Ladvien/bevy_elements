# The night sky lights the ground: a moonless night is not a black screen

A game's night scene is the crate's oldest open defect. The isometric town at 20 h renders at
**1 DN**: the sky, the ground, the houses' windows, everything. A player cannot see their
character.

The cause is precise and the fix is one term. The night sky's own illumination — airglow,
zodiacal light, starlight — is the crate's `ILLUMINANCE_FLOOR_LUX = 0.0005 lx`, and the crate
uses it for exactly two things: the **exposure meter's** floor (`scene_illuminance`) and the
**background's** clear colour (`NIGHT_SKY_LUMINANCE`). The **ambient light that illuminates
surfaces** comes from `cloud_ambient`, which returns `(Vec3::ONE, 0.0)` when neither the sun nor
the moon is above the horizon. So a moonless night has *no ambient at all*: the sky is lit and
the world on it is not.

The crate's own research already says what should happen. `twilight.md`'s *What great looks
like*: "**Night:** behind the atmosphere only the night sky's own glow; a moonless sky is
near-black with stars and **the horizon's silhouettes still readable against it**." The
silhouettes are there — the sky behind them is lit — but there is nothing under them.

## Sources

- **Jensen, H. W., Durand, F., Stark, M. M., Premože, S., Dorsey, J., Shirley, P. (2001)**,
  *A Physically-Based Night Sky Model*, ACM SIGGRAPH 2001, DOI `10.1145/383259.383306` (corpus
  stem `10.1145_383259.383306`, read in full for this note). Used:
  - **Figure 2**, the sources of natural illumination at night: zodiacal light $1.2\times10^{-7}$,
    integrated starlight $3.0\times10^{-8}$, airglow $5.1\times10^{-8}$, diffuse galactic light
    $9.1\times10^{-9}$ and cosmic light $9.1\times10^{-10}$ W/m², with the full moon at
    $2.1\times10^{-3}$ — the moon is four orders of magnitude brighter than the sum of the
    moonless sources, which is why a moonless night is dark and a moonlit one is not.
  - **§6**, on the four sources of diffuse night light, and the sentence that is the reason for
    this note: "These effects are especially important on moonless nights, when **a small change
    in illumination can determine whether an object in the scene is visible or invisible**."
  - **§6.2 Airglow**: "Airglow is the principal source of light in the night sky on moonless
    nights", emitted at 557.7 nm (O-I), 630 nm (O-I) and 589.0–589.6 nm (Na-I).
  - **§7**, on the results: "**The sky is never completely black.**"
  - **§7**, on the blue shift: "a very important component of our tone-mapping model is the
    blue-shift (the subjective impression that night scenes exhibit a bluish tint). This
    phenomena is supported by psychophysical data", and the scotopic caveat: "Because our scenes
    are at scotopic viewing levels (rod vision), special care must be taken with tone mapping."
- **The crate's own `twilight.md`** (the note this changes): the *What great looks like* above;
  Model 1, the night background's radiance `L_night = E_floor / π = 1.59\times10^{-4}$ cd/m²;
  and item 8, the visual target of "silhouettes against a moonless sky".
- **The crate's own `celestial.md`**: Jensen's numbers as the crate uses them — the moon's
  disc-mean luminance of 4843 cd/m² at full phase, the star's illuminance
  $E_\star = 10^{-0.4(V+13.99)}$ lx, and the non-lunar night illumination "zodiacal
  $1.2\times10^{-7}$, integrated starlight $3.0\times10^{-8}$, airglow $5.1\times10^{-8}$,
  galactic $9.1\times10^{-9}$, cosmic $9.1\times10^{-10}$ W/m²; sum $\approx 2.4\times10^{-7}$".
- **CIE 1951 / the scotopic luminous efficiency function** (through Jensen §7's blue shift): the
  night sky's own light is not neutral, and the *scotopic* eye weights it by the rod's curve,
  whose peak is at 507 nm.

## Model adopted

**1. The night sky's own radiance is part of the ambient, at all times.**

The ambient that lights the surfaces is the sky's radiance, and at a moonless night the sky's
radiance is the night sky's own glow. So the ambient's radiance is

$$ L = L_{\text{cloud}} + L_{\text{night}} $$

where `L_night = E_floor / π` is the crate's existing `NIGHT_SKY_LUMINANCE` — the *same* number
the background is cleared to, so the sky and the ground under it are lit by the same sky.

The night sky's light is *not* the cloud's: it is behind the atmosphere (Jensen §6, "the four
sources of diffuse visible color in the night sky"), so a cloud deck does not dim it, and a
cloud *does* dim the light that comes through it. The floor is therefore added, not multiplied.

**2. The night sky's light is blue-shifted, not neutral.**

Jensen §7: night scenes "exhibit a bluish tint", supported by psychophysical data, and the
scotopic eye's curve peaks at 507 nm. The airglow's own lines are green (557.7 nm O-I) and red
(630 nm O-I), but the *scotopic* response at 630 nm is small and at 507 nm it is at its peak, so
the sky's own glow reads blue-green.

So the night sky's radiance is a **cool colour**, not white, and the ground under it takes that
colour. A neutral night ambient would make the scene grey; a blue one makes it read as night.

**3. The ground is lit by the sky, and the sky is brighter than the ground.**

A uniform sky of radiance `L` gives a horizontal surface the illuminance `π L`, and a vertical
one half that. The crate's ambient is Bevy's `AmbientLight`, a uniform sky radiance, so the
crate's existing `BEVY_AMBIENT_DIFFUSE_SCALE` (the reciprocal of Bevy's `F_ab(1, NdotV)` = 0.452)
is the right conversion, exactly as the cloud's ambient already uses it.

**4. The moon still wins, by four orders of magnitude.**

Jensen's Figure 2: the full moon is $2.1\times10^{-3}$ W/m² and the sum of the moonless sources
is $2.4\times10^{-7}$. So the night sky's own light is a *floor*, not a competitor: at a full
moon the moon is 8700 times brighter, and the night sky's contribution is invisible beside it.
This is why the fix can be an additive term with no tuning: it changes the moonless night
completely and the moonlit night not at all.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The night sky's radiance | `E_floor / π = 1.59×10⁻⁴` cd/m² | the crate's `ILLUMINANCE_FLOOR_LUX = 0.0005` lx; Jensen Fig. 2's sum |
| The night sky's colour | a cool blue-green, weighted by the scotopic curve at 507 nm | Jensen §7's blue shift; the airglow's 557.7 nm line |
| The ambient's conversion | `1 / 0.452` | the crate's `BEVY_AMBIENT_DIFFUSE_SCALE` |
| The floor's share of the cloud's light | none: the floor is *added* | the night sky is behind the atmosphere (Jensen §6) |

## GPU mapping

- **No new pass, no new buffer, no new binding.** The floor is one term in `cloud_ambient`, which
  the CPU already evaluates once per frame, and in the sky probe's `atmo` uniform, which
  already carries `NIGHT_SKY_LUMINANCE`.
- The cloud's composite is unchanged: the probe's cube already has the night sky's radiance in
  its atmosphere floor, so the *sky* needs nothing.
- The exposure is unchanged: the meter already has the floor.

## What great looks like

- **A moonless night is dark but readable.** The ground is a deep blue-grey, the houses' windows
  are the brightest thing in the frame, and the silhouettes of everything else read against the
  sky. A player can see their character and the ground they stand on.
- **The night is blue, not grey and not black.** Jensen's blue shift, and the scotopic eye's
  curve: the ground's own colour survives, the way it does under moonlight.
- **The moon still dominates.** A moonlit night is *much* brighter than a moonless one, and
  the moon's shadows are still there.
- **Twilight is unchanged.** The sun's fall and the Belt of Venus are untouched; the fix only
  adds what is there when the sun and the moon are both down.
- **The day is untouched.** The floor is 0.0005 lx against 130 000 lx of sunlight: a millionth.

## Validation targets

1. **A moonless night is not black**: the isometric town at 22 h in `Clear` has a mean luminance
   above 2 DN, and the ground under the houses is above 1 DN (it is 1 DN now, and the whole
   frame is 1 DN).
2. **The night is blue**: the frame's B/R ratio is above 1.05 (a neutral night would be 1.0,
   and Jensen's blue shift makes it above).
3. **The day is unchanged**: the noon capture's mean luminance within 0.5 DN of the baseline's,
   and the 18 h capture's within 0.5 DN.
4. **The moon still wins**: a moonlit night's mean luminance is at least 20× a moonless one's.
5. **Twilight is unchanged**: the sun's fall, the Belt of Venus and the twilight's own captures
   are as they were.
6. **Cost**: no change to the opaque pass (the fix is on the CPU, in the ambient).

7. **A sea at a grazing view reflects the sky, not the fill (iteration 88)**: at midnight, clear, the sea's brightest row below the horizon (8° below, eye 30 m up) is within 4x of the sky's row just above the horizon in linear luminance (it was ~100x, `docs/loop-log.md` iteration 88), and the moonless town at 22 h still has a mean luminance above 2 DN with the ground above 1 DN (target 1 holds).

## Deviations from this plan

1. **The night sky's light reaches the ground by two paths, and both were needed.** The plan
   put the night sky's radiance in the *ambient* (`cloud_ambient`), which is right for a camera
   *without* a sky probe. But a camera *with* a probe has its ambient set to 0 — the light
   comes from the probe's cube — and the isometric town has one, so the ambient change alone
   left it black (0.907 → 0.9085 DN, no change). The probe's cube therefore carries the night
   sky's radiance too.
2. **The cube is display-referred, so the radiance is converted with the exposure.** The
   probe's cube holds *display* values, not radiances (`params.probe.x` is the camera's
   exposure), and adding the raw radiance to it put a value 10⁵ times too small in it: the
   frame's mean went from 0.91 to 2.15 DN, the houses appeared and the ground did not. The
   conversion is the same one the fog's haze uses: `L · exposure`.
3. **The night sky's colour is the *scotopic* one, and it is applied to the cube as well as to
   the ambient.** Jensen et al. 2001 §7 on the blue shift — night scenes "exhibit a bluish
   tint" — and the rod's luminous efficiency, whose peak is at 507 nm. The colour is
   `(0.72, 0.95, 1.28)`, which the ambient normalises to luminance 1 and the cube keeps as it
   is.
4. **The plan's "the moon still wins" is a property of the construction, not of a tuning.** The
   night sky's term is *added* to the bodies' light and never multiplied by it, and the full
   moon is 8700 times the moonless sources (Jensen's Figure 2), so a moonlit night is
   untouched to within a part in 10⁵.
5. **The lift is for light that falls on surfaces; at the horizon the cube is the sky the screen draws (iteration 88).** The cube is read by Bevy as *radiance*, and Bevy multiplies it by the camera's exposure `e`. Deviation 2 put `L · e` into it, so a surface lit by it is lifted `e`-fold (about 22 at the night's exposure ceiling) above its physical level. That lift is deliberate (it is what makes the ground readable), but a *specular* reflection of the same cube is lifted as well: a sea at a grazing view reflects the cube near the horizon at Fresnel ~1, and drew `F · L · c · e²`, 0.03–0.07 linear (42–80 DN) under a sky drawn at `L · c · e · T/T0` (7.8 DN, 4.8×10⁻⁴). Measured by elimination on `tests/gpu_still_water.rs` at midnight: the bare terrain 4.5 DN, the sea 62–80 DN; despawning the directional lights, the fog volume and the distance fog changed nothing; a perceptual roughness of 1 brought the sea to 10–22 DN and 0.15/0.3/0.6 to 62/48/26; the specular map's mip 0 read back equal to the composed cube (the cube is not stale), and the cube's maximum was 4.5×10⁻³ (it holds no hot texel), so the extra factor is Bevy's second `e` (22.4 at this scene: the observed sea over the cube is 13–20, Fresnel included). The same lift reaches the horizon band of the irradiance for no gain: below 10° elevation the band carries 3 % of a horizontal surface's irradiance. The night term is therefore weighted by elevation: `L · c · (r (1 − w) + k e w)` with `w = smoothstep(sin 8°, sin 40°, y)`, `r = 0.15` (the drawn low sky over the zenith, measured: 3.2×10⁻⁴ over 3.3×10⁻³ linear luminance at midnight) and `k = 1.198` the constant that keeps the horizontal surface's irradiance unchanged (`2 ∫ w s ds = 0.835`, numerically). Near the horizon the cube is the physical radiance, so Bevy's `e` makes it the sky the screen draws; above 40° it is the old lift times `k`. A vertical surface's share falls to 0.62 of what it was (numerically, `∫ w k cos² / ∫ cos²`), so a wall reads darker than a roof at night; the old uniform cube shaded both alike (see `AGENTS.md`, ambient). Citation: the iteration's own measurements; Jensen et al. 2001 §7 for the sky it must match ("the sky is never completely black", and a surface seen against it).


6. **The ramp starts higher (iteration 99).** Iteration 88 left the sea's horizon at 4.1x the sky above it (row means, midnight, clear, `gpu_a_night_sea_is_not_brighter_than_the_sky_it_reflects`). Measured by elimination: with the night term zeroed the same rows read 0.4x the sky, so the whole step is the lifted cube, reached through the lobe of the reflection (roughness 0.089 plus the wave normals, so a ray 2° under the horizon reflects a cone that touches 10-20° over it). The ramp now rises from 20° to 60° (was 8° to 40°), the horizon share is 0.05 (was 0.15), and the normalisation is 1.608 (was 1.198), computed numerically so a horizontal surface keeps its irradiance: measured 3.26e-3 against 3.29e-3, inside the existing 20 % gate. Result: the sea is 2.3x the sky (was 4.1x); the centre column's sea rows read (7-8, 9-10, 11-14) against (9-11, 11-13, 15-17) before, the sky above the horizon is unchanged (7, 5, 5). The cost is vertical surfaces: their share of the night fill falls from about 0.69 to about 0.50 of the uniform lift (computed from the ramp, not measured), a darker wall under a moonless sky, as the sky overhead is brighter than the horizon's. A steeper ramp (25-30° to 70°) would take the sea nearer 1.5x and the walls to 0.40-0.44; not taken. [INFERENCE: the lobe's reach is from the roughness and the measured ratios, not read back from the filtered map.]

## Pending index

- `10.1145/383259.383306` (Jensen et al. 2001): **in the corpus** (stem
  `10.1145_383259.383306`) and read in full for this note. **Not indexed** (the ONNX
  out-of-memory error recorded in `corpus.md`).
- `distill_index` is skipped for the above; the stem is listed here instead.

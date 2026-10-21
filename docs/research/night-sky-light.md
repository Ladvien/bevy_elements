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

## Deviations from this plan

*(filled in as the implementation settles; every changed default is recorded here with its
citation.)*

## Pending index

- `10.1145/383259.383306` (Jensen et al. 2001): **in the corpus** (stem
  `10.1145_383259.383306`) and read in full for this note. **Not indexed** (the ONNX
  out-of-memory error recorded in `corpus.md`).
- `distill_index` is skipped for the above; the stem is listed here instead.

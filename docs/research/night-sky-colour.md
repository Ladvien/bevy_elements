# The night sky is blue, not brown: what is behind the atmosphere

The night sky is **sepia**. Measured on the `day_night` example at 22 h (2026-07-14, `Clear`,
the sun at −17.5° and the moon at −14.2°, so neither is up), the sky's linear RGB is
**(0.00297, 0.00195, 0.00099)**: a red/green ratio of **1.52** and a blue/green of **0.51**. The
crate's own `twilight.md` says the night should be "**Nothing brown**".

The cause is not what iteration 12 found and fixed. That was Bevy's default `ClearColor` — a
neutral grey written to the target before the sky and never exposed — and the crate replaced it
with the night sky's own radiance. The clear colour *is* now neutral (the probe reads
`LinearRgba(0.0035603, 0.0035603, 0.0035603)`), and the sky is still brown.

## The cause

The clear colour is what lies **behind** the atmosphere, and the atmosphere is drawn over it with
its **transmittance**. At night that transmittance is measured, from the capture and the probe's
neutral clear colour:

| elevation | the transmittance | R/G | B/G |
|---|---|---|---|
| the zenith | (0.904, 0.640, 0.353) | 1.41 | 0.55 |
| 30° up | (0.754, 0.433, 0.186) | 1.74 | 0.43 |
| the low sky | (0.357, 0.169, 0.162) | 2.12 | 0.96 |

That is **Rayleigh extinction**: the blue is scattered out of the line of sight most, so whatever
is behind the atmosphere comes through warm. A *neutral* clear colour behind a warm-extinguishing
atmosphere is a brown sky. The crate's own note said the right thing — "Bevy's atmosphere draws
over it with its transmittance, **as airglow and zodiacal light are seen through it**" — but it
made the clear colour *neutral*, and the extinction is not.

## Sources

- **Jensen, H. W., Durand, F., Stark, M. M., Premože, S., Dorsey, J., Shirley, P. (2001)**,
  *A Physically-Based Night Sky Model*, ACM SIGGRAPH 2001, DOI `10.1145/383259.383306` (**in the
  corpus**, stem `10.1145_383259.383306`, read in full). Used:
  - **§6.2 Airglow**: "Airglow is faint light that is continuously emitted by the entire upper
    atmosphere with a main concentration at an elevation of approximately 110 km… **Airglow is the
    principal source of light in the night sky on moonless nights**", with its emission lines at
    557.7 nm (O-I), 630 nm (O-I) and 589.0–589.6 nm (Na-I). It is "integrated into the
    simulation by adding an **active layer to the atmosphere** (altitude 80 km)".
  - **§6**: the four sources of diffuse night light — the zodiacal light, the airglow, the
    diffuse galactic light and the cosmic light — and the sentence that is the reason for this
    note: "These effects are especially important on moonless nights, when a small change in
    illumination can determine whether an object in the scene is visible or invisible."
  - **§7**: "**The sky is never completely black**", and the blue shift — "a very important
    component of our tone-mapping model is the blue-shift (the subjective impression that night
    scenes exhibit a bluish tint). This phenomena is supported by psychophysical data."
- **The crate's own `twilight.md`**: the *What great looks like* — "**Night:** behind the
  atmosphere only the night sky's faint glow; a moonless sky is near-black with stars and the
  horizon's silhouettes still readable against it; a moonlit sky is a dim blue. **Nothing
  brown.**" — and its Model 1, which puts the night sky's radiance behind the atmosphere and
  lets the transmittance do the rest.
- **The crate's own `celestial.md`** and `night-sky-light.md`: the night sky's radiance
  (`ILLUMINANCE_FLOOR_LUX = 0.0005 lx`, `NIGHT_SKY_LUMINANCE = 1.59×10⁻⁴` cd/m²) and the
  *scotopic* colour (iteration 31's `NIGHT_SKY_COLOUR = (0.72, 0.95, 1.28)`), which the
  *ambient* uses and the *clear colour* does not.
- **The CIE 1951 scotopic luminous efficiency function**, through Jensen §7's blue shift: the
  rod's curve peaks at **507 nm** and falls away on both sides, so the night sky's own lines
  (557.7 nm green, 630 nm red) are read as a cool blue-green and the eye's own response makes
  the night look blue.

## Model adopted

**1. The clear colour carries the *scotopic* colour, and it is bright enough in blue to survive
the extinction.**

The clear colour is the night sky's glow *behind* the atmosphere, and the atmosphere removes
most of the blue from it. So the clear colour must be blue enough that what comes through is the
scotopic blue-green the eye expects.

The crate's `NIGHT_SKY_COLOUR = (0.72, 0.95, 1.28)` is the *result* — the colour of the night
sky as seen. The clear colour is the colour *before* the extinction, and the extinction is
Rayleigh: the transmittance's ratio to the zenith's, in the red and the blue.

So the clear colour is

$$ \mathbf{c} = \frac{\mathbf{L}_{\text{scotopic}}}{T} $$

where `T` is the atmosphere's transmittance at the zenith — and since the *clear colour* is a
single value for the whole sky, `T` is the zenith's. The zenith's transmittance is the
*shortest* path, so the sky's *lower* parts are more reddened, which is right: the horizon is
the warmest, as the capture shows (R/G 1.41 at the zenith, 2.12 at the low sky).

**2. The *ambient*'s colour and the *clear colour*'s colour are different things, and the crate
already had them right.**

The ambient lights the ground with the sky's glow as it reaches the ground — through the
atmosphere — so it uses the *scotopic* colour directly (iteration 31). The clear colour is the
glow *behind* the atmosphere, so it must be pre-extinction. The two are related by the
extinction, and that is the whole fix.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The clear colour's blue | `L_scotopic / T_blue` with `T_blue = 0.353` at the zenith | measured from the capture (Deviation 1) |
| The clear colour's red | `L_scotopic / T_red` with `T_red = 0.904` | measured (Deviation 1) |
| The scotopic colour | `(0.72, 0.95, 1.28)`, normalised to luminance 1 | iteration 31; Jensen §7's blue shift |
| The zenith's transmittance | `(0.904, 0.640, 0.353)` | measured (Deviation 1) |

## GPU mapping

- **No new pass, no new buffer, no new binding.** The clear colour is one `vec3` written per
  camera per frame, as it is now.
- The exposure is unchanged.

## What great looks like

- **The night sky is blue-black, not sepia.** The zenith reads a deep blue-black with the
  stars in it, and the horizon is a little warmer — the airglow's own green-red lines read
  through the scotopic eye as a cool glow, and the path is longer near the horizon.
- **A moonlit night is a dim blue** (`twilight.md`'s own wording), and the moon's blue-white
  light is not drowned by a brown sky.
- **The day is untouched**: the clear colour is only the night sky's, and the exposure is what
  decides when it shows.
- **The twilight is untouched**: the blue hour is blue, and the Belt of Venus is pink, and
  neither moves.

## Validation targets

1. **The night sky is not brown**: the sky's red/green ratio at the zenith is below 1.0 and its
   blue/green is above 0.8 (it is 1.52 and 0.51 today).
2. **The night is blue**: the blue/green ratio at the zenith is above 0.8.
3. **The horizon is warmer than the zenith**: the red/green ratio rises toward the horizon
   (it does today: 1.41 at the zenith, 2.12 at the low sky).
4. **The day is untouched**: the noon capture's sky within 0.5 DN of the baseline's, and the
   twilight's within 0.5 DN.
5. **The night is still dark**: the sky's luminance is within 20 % of the baseline's — the fix
   changes the *hue*, not the brightness.
6. **Cost**: no change to the opaque pass.

## Deviations from this plan

1. **The clear colour is the scotopic colour *divided by* the transmittance, and the
   transmittance is measured, not assumed.** The plan divided by "the transmittance at the
   zenith"; the value is the one measured from the capture against the probe's neutral clear
   colour — (0.904, 0.640, 0.353) — and it is a *measurement*, so a change to Bevy's atmosphere
   or to the crate's exposure would move it. The clear colour is `NIGHT_SKY_COLOUR /
   ZENITH_TRANSMITTANCE`, and the two constants sit next to each other in `celestial/mod.rs` so
   the relation is visible.
2. **The first attempt put the scotopic colour into the clear colour directly, and the sky
   stayed brown** (the zenith's red/green 1.411 → 1.415). The reason is the same fact the
   note's own diagnosis turns on: the clear colour is *behind* the atmosphere, and the
   atmosphere's transmittance is warm, so a blue clear colour comes through *less* blue. The
   division is not a refinement; it is the whole fix.
3. **The horizon is warmer than the zenith, and the fix does not change that.** The clear
   colour is one value for the whole sky, so the zenith's transmittance is the one it is
   divided by, and the lower sky — whose path is longer and whose transmittance is warmer —
   comes through warmer. Measured: the red/green ratio 0.783 at the zenith, 0.993 at 30° and
   1.626 at the low sky, against 1.411, 1.742 and 2.119 before.

## Pending index

- `10.1145/383259.383306` (Jensen et al. 2001): **in the corpus** and read in full for this
  note. **Not indexed** (the ONNX out-of-memory error recorded in `corpus.md`).
- `distill_index` is skipped for the above; the stem is listed here instead.

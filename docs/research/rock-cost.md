# The rock's material is half the frame: band-limit the field, not the feature

The crate's rock is the most expensive thing it draws. Measured on the build before this note,
the mountains' opaque pass is **0.72–0.79 ms** with the rock's material and **0.13–0.38 ms**
without it: the material is **roughly half the pass**, and the crate's notes have carried a
warning about it since iteration 29 ("the crate's hot path", "the mountains' opaque pass must
not move materially").

The material is not *wasteful* in the way a leak is wasteful. Every one of its fields is
band-limited, and every one of those band-limits is *correct*. The waste is that the **field is
evaluated where its own band-limit has already cancelled it**.

## Sources

- **Lagae, A., Lefèbvre, S., Cook, R., DeRose, T., Drettakis, G., Ebert, D., Lewis, J. P.,
  Perlin, K., Zwicker, M. (2010)**, *A Survey of Procedural Noise Functions*, Computer Graphics
  Forum 29(6), DOI `10.1111/j.1467-8659.2010.01827.x` (in the corpus, read). Used:
  - "Since 1985, compute speed has increased much faster than memory bandwidth. In a sense, we
    can now consider that **cycles are free**, in reference to the fact that most programs in
    today's architectures spend a large amount of their time waiting for cache misses and other
    kinds of memory access" — the survey's framing of *why* the noise's cost is the thing to
    measure, and it is the framing of this note: the noise's *count* is the cost, and a field
    that is fetched where it cannot change the answer is pure waste.
  - "**90% of 3D rendering time is spent in shading**, and much of that time is spent computing
    **Perlin noise**" — the survey's own headline for why the noise's cost is a first-class
    problem and not an implementation detail.
  - The survey's speed comparison: "sparse convolution noises, wavelet noise and better
    gradient noise are generally **slower than Perlin noise**, whereas anisotropic noise is
    generally faster" — so the crate's Perlin is the *right* choice and the count is the thing
    to cut, not the function.
- **The crate's own `rock-material.md`** (iteration 29): the rule that every octave is band-
  limited by its own feature size, and the warning that "every octave is a noise evaluation in
  the crate's hottest pass — the mountains' opaque pass is 0.138 ms on the baseline build", with
  the measured consequence: "The 3D noise is twice the 2D one's per octave, and the mountains'
  opaque pass is the crate's hot path: **four octaves took it from 0.136 ms to 0.249 ms**. One
  octave leaves it at [0.144 ms]". So the crate has always known that a *count* of octaves is
  the cost, and the material has grown from two octaves to **five** `perlin3_d` and a cell
  network since.
- **The crate's own `rock-blocks.md`** (iteration 35): the band-limit's own rule — "The
  band-limit is *known before the network is looked up*, and the lookup is the expensive part
  — 27 cells, each a hash and a distance. So where the blocks cannot resolve, they are not
  looked up at all: the far terrain pays nothing for them, and the mountains' opaque pass does
  not carry the cost over the whole massif." The *pattern* of this note is that note's, applied
  to the fields the block's note did not know about.
- **The crate's own `rock-fragments.md`** (iteration 37): the warp's own band-limit — "a warp
  that varies faster than the cell it moves **scrambles** the lattice instead of displacing
  it", and the scale's rule that the warp must be *coherent* over many cells.
- **The crate's own `rock-palette.md`** (iteration 38): the **rind**, the film of iron and
  manganese on the surface, which is a *fine* feature — "a coating 100 to 500 um thick" — and so
  is band-limited away with the distance.
- **The crate's own `bed-colour.md`** (iteration 39): the **region**, the slow field that biases
  the bed's selection so a whole massif leans towards one rock, and the *bed's colour*'s own
  band-limit — the bed's colour is mixed in only where the *fragments* do not resolve.

## Model adopted

**1. Every field is gated by the band-limit of the feature it feeds, and the gate skips the
*evaluation*.**

The crate's rule is already "the band-limit is known before the network is looked up". The
extension is: the same for a *field*. A field that is cancelled by a band-limit is not fetched
at all.

There are two such fields in the rock's material, and both are on the crate's hot path:

- **The bed's region** — the slow field that biases the bed's colour, at 260 m. The bed's
  *colour* is band-limited against the *fragments*: where a fragment resolves it wears its own
  rock and the bed's colour is multiplied by zero. So the region — a whole `perlin3_d` — is
  fetched and **thrown away** on every near rock pixel.
- **The second set's segment field** — the field that breaks the second joint set's planes into
  segments. The planes are non-persistent, so the joint is *absent* over most of the face, and
  the field is fetched over all of it.

**2. A gate skips the *evaluation*, and not the field's *value*.**

The first attempt at the region's gate multiplied the field by the keep. That changed **13 % of
the near face** — because the keep is between zero and one in the band where the two
band-limits cross, and scaling the field weakens the bias there. A performance change must not
change the image, so the gate skips the *evaluation* and the value is the field's own.

**3. A gate must cover the feature's *whole* extent, tail included.**

The second set's joint is `1 − smoothstep(0, soft, d)`, and a smoothstep's tail reaches past
`soft`. A gate on `d < soft` cut the tail, and the face changed in **13 %** of its pixels
again. The gate covers `d < 2·soft`, and then the change is **0.34 %** — the GPU's own
non-determinism, and the same as an ungated build.

**4. The gate is the feature's own width, so it is exact.**

The region's gate is the *bed's colour's* keep, which is the *fragments'* band-limit — the
same number the fragments use. The segment's gate is the second set's joint's own width. So
neither gate is a new constant that can drift from the feature it gates.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| The region's gate | the bed's colour's keep (the fragments' band-limit) | the feature's own band-limit |
| The segment's gate | twice the second set's joint's width | the smoothstep's tail |
| The material's total | the whole `rock_material` | the measurement that started it |
| The value-only noise | the same eight gradients and the same interpolation, without the derivative's terms | the value is bit-identical to `perlin3_d(p).x`, proved by a test |

## GPU mapping

- **Inside the terrain's own fragment shader**, where the rock's material already is
  (`src/terrain/common.wesl`, `rock_material` and `rock_blocks`). **No new pass, no new buffer,
  no new binding.**
- The gates are `if` statements on a value the shader already has, so they cost a branch and
  save a field.

## What great looks like

- **The rock's material is invisible as a cost**: the mountains' opaque pass falls by a
  measurable amount, and the face is **the same picture** to the last pixel that can be
  compared.
- **The distance is calm** and the meadow is untouched.

## Validation targets

1. **The image does not change.** The near face, the face at 12 m and at 30 m, the dusk, the
   massif at 120 m and the `plains` preset: the differing pixels are under **1 %** of the frame
   and the mean moves by less than **0.2 DN** — the GPU's own non-determinism, and the same as
   an ungated build.
2. **The cost falls.** The mountains' opaque pass, measured by toggling the change in the
   shader in one session, falls by at least **5 %**.
3. **The meadow is untouched** and the distance is unchanged.

## Deviations from this plan

1. **The plan gated two fields; a third change was needed, and it is the bigger one.** The
   plan's two gates are worth about 5 %. But the *bigger* waste is inside the noise itself:
   `perlin3_d` computes the value **and the derivative**, and the derivative is a second large
   polynomial — eight vector differences, a `du` term and three swizzles. The crate makes
   **five** evaluations per rock pixel and **four** of them use the value alone: the bed's
   region, the rock's rind, the second joint set's segment field, the turf's patch field and
   the scree's grain. So the derivative is computed and thrown away most of the time, on the
   crate's hottest path.
   So `perlin3` is now the **value-only** function — the same eight lattice gradients and the
   same interpolation, with only the derivative's terms left out — and the value is
   **bit-identical** to `perlin3_d(p).x`, so a call site can switch without changing a pixel.
   The crate already had a `perlin3` that was *exactly* `perlin3_d(p).x`, so the change is
   that the wrapper is gone and the function is the value.
   And the change is **crate-wide**: the hydrology's phase noise, the terrain's generator, the
   surface cover's field and the noise's own `fbm3` all call `perlin3`, and every one of them
   was paying for a derivative it never used.
2. **The gates skip the *evaluation*, and not the field's *value*.** The first attempt at the
   region's gate multiplied the field by the keep. That changed **13 %** of the near face —
   because the keep is between zero and one where the two band-limits cross, and scaling the
   field weakens the bias there. A performance change must not change the image.
3. **A gate must cover the feature's *whole* extent, tail included.** The second set's joint is
   `1 − smoothstep(0, soft, d)`, and a smoothstep's tail reaches past `soft`. A gate on
   `d < soft` cut the tail, and the face changed in **13 %** of its pixels again. The gate
   covers `d < 2·soft`, and then the change is **0.34 %** — the GPU's own non-determinism, and
   the same as an ungated build.
4. **The value-only noise is proved identical by a test, not by an inspection.** The mirror's
   `perlin3` and `perlin3_d(p).x` are compared over 500 points, and the comparison is an
   `assert_eq!` on the `f32`s — so a change to either function that broke the identity would
   fail the test rather than silently change a pixel.

## Pending index

- Lagae et al. 2010 (`10.1111/j.1467-8659.2010.01827.x`): **in the corpus** and read.
  **Not indexed.**
- `distill_index` is skipped for the above.

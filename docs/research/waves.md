# Waves: ripples and wakes from bodies in the water

Anything that moves through the crate's water should disturb it: a wader's legs, a floating crate bobbing, a boat's hull, a stone dropped in a pond. Today the water surface (`WaterPlugin`, `water-rendering.md`) is a flow-mapped normal texture advected by the hydrology velocity; nothing a game does to the water shows on it. This note adds the missing direction of the interaction, after trails (`trails.md`) did it for the ground: a dispersive linear wave field on the water surface, in a camera-following window, forced by the bodies that pierce it, reflected by the shores, carried by the current, and drawn by the water's surface and absorb passes.

## Sources

Read for this note (local copies `/tmp/i15/pdf/*.txt`; home-still stems where ingested):

- **Tessendorf 2004, *Interactive Water Surfaces*** (Game Programming Gems 4, pp. 265–274; author PDF `jtessen.people.clemson.edu/reports/papers_files/Interactive_Water_Surfaces.pdf`). iWave: the linearised Bernoulli equation `h_tt + a h_t = −g √(−∇²) h` with the vertical-derivative operator as a 13 × 13 convolution (`P = 6`, "the smallest value that gives clearly water-like motion"), the explicit three-level update (his eq. 3), obstruction masks (`h *= mask`) that reflect waves, sources (`h += s`), wakes from `s = 1 − mask` "including the V-shaped Kelvin wake", periodic and reflecting borders, and a surface-tension kernel `q²(1 + q² L_T²)`.
- **Canabal, Miraut, Thuerey, Kim, Portilla & Otaduy 2016, *Dispersion kernels for water wave simulation*** (`10.1145/2980179.2982415`, SIGGRAPH Asia). The convolution form `ḧ = f ∗ h` with `F(k) = −ω²(k)`, `ω² = (g k + σ/ρ k³) tanh(k d)` (their eqs. 2–6); naive spatial truncation spills error into low frequencies, which "produces volume change and low-frequency oscillations that turn into instability over time" (§4, Fig. 6); iWave's kernel is scale-independent and "considerably underestimates the propagation velocity" (Fig. 2) when used without the grid's scale; obstacles by a shadowed convolution. Their pyramid kernels are accurate but cost 0.6–5 fps at 1024², far beyond a game's budget.
- **Ottosson 2011, *Real-time Interactive Water Waves*** (KTH thesis at EA DICE, prototyped in Frostbite; `media.contentapi.ea.com/content/dam/eacom/frostbite/files/water-interaction-ottosson-bjorn.pdf`). The game-engine view: dispersion approximated by simulating the surface at several resolutions in a Laplacian-pyramid structure, each with its own wave speed; interaction by "simple linear displacement" decomposed into the pyramid's bands; bodies as ellipsoids, their waterline as ellipses; "a horizontally moving body is modeled by increasing the water height in front of it and decreasing it behind", vertical motion by "the change of submerged volume" (§4.4); a pool of grids for constant cost (§4.6); iWave's zero-height shore is a rigid boundary, "completely unphysical", but cheap (§4.8). 3 ms per step on one CPU core in 2011.
- **Crest** (Bowles et al., SIGGRAPH 2017 *Advances* "Crest: novel ocean rendering techniques in an open source framework"; MIT source `wave-harmonic/crest`, `UpdateDynWaves.compute`; docs `crest.readthedocs.io/en/4.15.2/user/ocean-simulation.html`). The shipping-game recipe: the 2D wave equation per cascade of camera-centred textures, each cascade's wave speed `c = √(g λ / 2π)` at `λ = 2 Δ` (deep-water phase speed) clamped to a Courant number of 0.7, symplectic Euler, damping 0.05 s⁻¹, semi-Lagrangian advection by the flow, zero height at dry cells ("wave reflections off geometry"), a feathered edge that damps toward the window border, attenuation in shallows when `d < λ/2`, interaction by nested spheres; 60 updates a second.
- **Yuksel, House & Keyser 2007, *Wave particles*** (`10.1145/1275808.1276501`; author PDF `cemyuksel.com/research/waveparticles/waveparticles.pdf`). Wave generation "based on volume conservation": the volume an object displaces by its motion relative to the fluid becomes the waves; buoyancy `F = −g ρ V_displaced` from the same height field.
- **Jeschke & Wojtan 2017, *Water wave packets*** (`10.1145/3072959.3073678`; author PDF `pub.ista.ac.at/group_wojtan/projects/2017_Jeschke_WaterWavePackets/wavepackets_author.pdf`). Lagrangian packets carrying groups of wave trains with correct dispersion, real time for moderate scenes; the reference for what dispersive wakes and ripples look like, and the reason not to use one wave speed.
- **Schreck, Hafner & Wojtan 2019, *Fundamental solutions for water wave animation*** (`10.1145/3306346.3323002`). Closed-form circular ripples and boundary interaction by equivalent sources; their comparisons with real ripples set the look of a dropped object's rings (fast short waves ahead, slow long ones behind).
- **Kass & Miller 1990, *Rapid, stable fluid dynamics for computer graphics*** (`10.1145/97880.97884`). The height-field wave equation with `c² = g d` (shallow water) — the long-wave limit of the dispersion relation used here.
- **Colen & Kolomeisky 2021, *Kelvin–Froude wake patterns of a traveling pressure disturbance*** (`10.1016/j.euromechflu.2020.10.008`, arXiv 1902.01884). The forced oscillator per wave vector, `(ρ/k) ζ̈ = −ρ g ζ − p` (their eq. 8), and "by interpreting p(k) as an effective pressure distribution which can mimic effects of a ship hull or underwater objects" the same integral covers hulls and submerged bodies; submerged sources act through `exp(−k f)` at depth `f`.
- **Darmon, Benzaquen & Raphaël 2014, *Kelvin wake pattern at large Froude numbers*** (`10.1017/jfm.2013.607` as stored; arXiv 1309.6751). Modelling the moving object by an axisymmetric pressure field, the wake region is "delimited by an angle equal to arcsin 1/3 = 19.47°" for all Froude numbers, while the angle of maximum amplitude scales as `Fr⁻¹` at large `Fr`.
- **Tessendorf 2004, *Simulating Ocean Water*** (course notes, `jtessen.people.clemson.edu/reports/papers_files/coursenotes2004.pdf`). The dispersion relations `ω² = g k`, `ω² = g k tanh(k d)` and the capillary term, and the ocean-scale look the wakes sit beside.
- **Tatarchuk 2006, *Artist-directable real-time rain rendering in city environments*** (SIGGRAPH *Advances*, corpus `s2006-advances-materials-5`, `-6`). ToyShop's GPU membrane simulation for puddle ripples on a 256² lattice: raindrops and "object outlines into the seeding texture to generate wakes", Sobel normals from the heights, one shared simulation sampled by world XZ.
- **Solenthaler, Bucher, Chentanez, Müller & Gross 2011, *SPH based shallow water simulation*** (corpus `eth-cgl-sim_anim-Sol11b`). The survey of height-field methods for games: the wave equation on a 2D height field from Kass & Miller and Tessendorf "extended to simulate rigid body interaction".

Not obtained: Jeschke et al. 2018 *Water surface wavelets* (`10.1145/3197517.3201336`, ACM 403, no author PDF found); Raphaël & de Gennes 1996 (`10.1103/PhysRevE.53.3448`, paywalled); Cords 2008.

## Model adopted

**The field.** A height `h(x, t)` of the water surface above its undisturbed level (the hydrology's bed + depth), small amplitude, linear. Per wave vector it is the forced oscillator of Colen & Kolomeisky's eq. 8 with the full Airy dispersion of Canabal's eq. 6:

$$\ddot h = -\,\Omega^2 * \big(h + \eta\big) - a\,\dot h,\qquad \widehat{\Omega^2}(k) = \Big(g k + \tfrac{\sigma}{\rho} k^3\Big)\tanh(k d)$$

with `g = 9.81 m/s²`, `σ/ρ = 7.28 × 10⁻⁵ m³/s²` (clean water at 20 °C), `d` the local water depth and `η` the **pressure head** of the bodies in the water (`p/(ρ g)`, m). A body at rest holds the surface exactly at `h = −η` under it and flat elsewhere (the static solution of `Ω² ∗ (h + η) = 0`); moving, the same patch radiates the dispersive wake, bounded by Kelvin's 19.47° (Darmon et al.); bobbing, it radiates rings. One equation covers hulls, waders and dropped stones, with no tuned source amplitude.

**Pressure head of a body.** A `WaterContact` is an ellipsoid (half extents along the entity's local axes, any rotation). Above a surface point the vertical line through it cuts the ellipsoid between `y_b` and `y_t` (a quadratic in `y` with the ellipsoid's matrix `M = R diag(a⁻², b⁻², c⁻²) Rᵀ`). A wave of wavenumber `k` feels the body's occupied column weighted by `e^{−k z}` at depth `z` (the decay of its velocity potential; Colen & Kolomeisky's submerged sources), so the head is the column integrated against that weight:

$$\eta(x) = \frac{e^{-k_c\,\max(0,\;y_w - y_t)} - e^{-k_c\,\max(0,\;y_w - y_b)}}{k_c}$$

`y_w` the undisturbed surface. For a shallow hull (`k_c T ≪ 1`) this is its draft `T` (the classical ship-as-pressure-distribution); for a deep leg it saturates at `1/k_c`; a body entirely above the water gives 0 and one entirely below it decays smoothly with its depth, so a body sinking or surfacing changes `η` continuously. `k_c` is the cascade's band-centre wavenumber (below), so each band sees the part of the body its waves reach. This is the physical content of Yuksel's volume conservation and Ottosson's "change of submerged volume": both are what the `η` term does to the surface.

**Two cascades, a band each (Ottosson's pyramid, Crest's cascades).** A compact kernel cannot be accurate for every wavelength (Canabal §4), and one grid cannot hold a wader's 15 cm ripples and a hull's 10 m transverse waves. Two camera-centred toroidal windows of the same size: **fine** (High: 512² of 5 cm, 25.6 m) and **coarse** (512² of 40 cm, 204.8 m). Each carries `(h_n, h_{n−1})` per texel and its own kernel fitted to `Ω²` over its band (λ from 2.5 Δ to 32 Δ: fine 12.5 cm–1.6 m, coarse 1 m–12.8 m). The forcing is split by band like a Laplacian pyramid: the coarse cascade is forced by `η` box-averaged over its texel, the fine one by `η` minus the coarse forcing interpolated to its texel, so a wave is born in exactly one cascade. The water draws the sum of the two.

**The kernel.** For a cascade of spacing `Δ`, a radially symmetric 13 × 13 kernel `K` (Tessendorf's `P = 6`; 27 free values for `0 ≤ n ≤ m ≤ 6` with the centre eliminated by `Σ K = 0`, so a uniform rise produces no acceleration and volume is conserved — the lowest-order form of Canabal's low-frequency compensation) is fitted on the CPU at start-up by least squares to `Ω²_deep(k) = g k + (σ/ρ) k³` over polar samples of the band, weighted for relative error in phase speed. The fit's effective dispersion is checked in a unit test. Depth enters per texel as the factor `tanh(k_c d)` on the acceleration (the shallow-water slowing toward a shore; exact at the band centre).

**Time stepping.** iWave's explicit three-level scheme (his eq. 3) at a fixed 1/60 s substep in virtual time (at most 4 substeps a frame):

$$h_{n+1} = \frac{(2 - a\Delta t)\,h_n - h_{n-1} - \Delta t^2\,\tanh(k_c d)\,K * (h_n + \eta)}{1 + a\Delta t}$$

Stable while `Δt √(max K̂) < 2` (checked at fit time). `a = 0.05 s⁻¹` (Crest's default damping; viscous damping `2 ν k²` is below 10⁻³ s⁻¹ at these wavelengths, so `a` stands for surface films and wind).

**Boundaries.** Shores reflect: after each step `h_{n+1}` (and `h_n`) are multiplied by `smoothstep(0.003, 0.03, d)` (iWave's obstruction mask; Crest's zero height at dry cells; Ottosson's caveat that this is a rigid boundary is accepted). The window's outer 6 % is a sponge (`×(0.95 → 1)` toward the edge, Crest's feather) so waves leave instead of wrapping; a texel entering the window starts flat.

**Current.** Where the hydrology flows, `(h_n, h_{n−1})` are fetched at `x − u Δt` (semi-Lagrangian, Crest), so a wader's ripples are carried downstream.

**Rendering.** The field publishes `(h, ∂h/∂x, ∂h/∂z)` per texel to a two-layer `rgba16float` texture (layer 0 fine, 1 coarse). The water's surface and absorb passes add the two cascades' slopes to the flow-mapped normal map's slope (both are small-slope height fields, so slopes add), fading each cascade over the last 10 % of its window; Fresnel, refraction thickness, the sun glint and the cloud-shadowed diffuse all follow the combined normal. No vertex displacement: the CDLOD water patches are metres apart, and amplitudes here are centimetres.

**Not modelled.** Breaking, spray and foam from steep crests (the existing foam rules stay); two-way forces on the bodies (buoyancy readback); rain impacts (sub-texel at 5 cm: a 2 mm drop's crater is about 6 mm, Engel's energy balance, so the existing ripple normal map keeps rain); waves on water the hydrology does not hold.

## Parameters

| Quantity | Value | Source |
|---|---|---|
| Gravity `g` | 9.81 m/s² | — |
| Surface tension / density `σ/ρ` | 7.28 × 10⁻⁵ m³/s² | clean water, 20 °C (Canabal eq. 2) |
| Kernel | 13 × 13, radially symmetric, `Σ K = 0`, least-squares fit to `Ω²_deep` over λ ∈ [2.5 Δ, 32 Δ] | Tessendorf (`P = 6`), Canabal (LF compensation) |
| Fine cascade | Low 256² × 8 cm, Medium 512² × 6 cm, High 512² × 5 cm, Ultra 1024² × 4 cm | ToyShop 256², Crest 512² cascades |
| Coarse cascade | 8 × the fine texel, same size | Ottosson's pyramid ratio, Crest's cascades |
| Band-centre wavenumber `k_c` | `2π / (8 Δ)` | — |
| Substep | 1/60 s, ≤ 4 a frame | Crest's 60 Hz |
| Damping `a` | 0.05 s⁻¹ | Crest default |
| Shore mask | `smoothstep(0.003, 0.03, d)` | the water's own edge (`water-rendering.md`) |
| Window sponge | outer 6 %, factor 0.95 → 1 | Crest's feather (0.1 of the window, 0.95) |
| Contacts | ≤ 32 nearest the camera | as `Imprint`'s 64 |

## GPU mapping

`WavesPlugin` (module `waves`), with the hydrology. Storage buffers: `state` (vec2 `(h_n, h_{n−1})` per texel per cascade) in two ping-pong copies; `head` (one f32 per fine texel and one per coarse texel: `η` of this frame, written as `atomicMax` of non-negative f32 bits, since bodies overlap); `contacts` (≤ 32 entries: centre, the ellipsoid matrix's six terms, its XZ bound); `kernel` (two × 28 weights); the hydrology's `terrain_cells` (bed, depth) and `flow_cells` (velocity) read-only. Kernels per substep: `wave_head` (per contact, its XZ bound in each cascade: the chord and `η`; skipped when no contact), `wave_step` (every texel of both cascades: advect, convolve `h + η_band` with the 13 × 13 kernel from a 16 × 16 tile plus a 6-texel halo in workgroup memory, update, mask, sponge, reset on entry; `a` → `b`), then `wave_publish` (Sobel slopes into the write-only two-layer storage texture). The render world keeps the window origin of the last frame it ran (pipelined rendering, as `trails`). The texture is toroidal by world texel with a repeating sampler; the water shader finds each window from the view position, so its uniform does not change as the camera moves. WebGPU: four storage buffers and one storage texture per kernel; the water material gains one texture array and one sampler.

Cost estimate: 2 × 512² texels × 169 taps from workgroup memory at 60 Hz ≈ 89 M multiply-adds a frame: under 0.1 ms on a 3090-class card.

## Validation targets

1. **Dispersion (CPU).** The fitted kernel's phase speed `√K̂(k)/k` is within 5 % of Airy's deep-water `√(g/k + σ k/ρ)` for λ ∈ [3 Δ, 24 Δ] along the axes and the diagonal, for both cascades at every quality, and `Δt √(max K̂) < 2`.
2. **Pressure head (CPU).** `η` of a sphere of radius `r` floating half-submerged (`y_w` at its centre) is `(1 − e^{−k r})/k` at its centre; 0 for a sphere above the water; continuous through submersion; a rotated ellipsoid's chord matches a brute-force march to 1 mm.
3. **A body at rest depresses the water under it only (GPU).** After 10 s with a still, half-submerged 0.5 m sphere: the fine cascade's `h` under the centre is `−η` (± 10 %), and `|h| < 2 mm` more than 1 m from it.
4. **A moving body leaves a Kelvin wake (GPU).** A 0.3 m sphere at 1.5 m/s along +X for 8 s: waves of `|h| > 1 mm` behind it are confined to the sector of half-angle 19.47° (+ 4° for the grid and the source size) about its track, and there are such waves.
5. **Shores reflect (GPU).** Waves from a body bobbing near a dry bank do not appear beyond it (`|h| = 0` on dry texels).
6. **The window keeps the waves in place (GPU).** As in `trails.md`: a ring stays at its world position (± 1 texel) while the camera moves 3 m.
7. **Visual.** A wader, a bobbing float and a moving hull in a calm lake at noon and at a low sun, at 2–30 m: rings that spread with short waves outrunning long ones; a V wake of Kelvin's angle with curved transverse crests inside it behind the hull; reflection off the shore; no seam where a cascade's window ends, no ringing around a still body, no swimming as the camera moves; the sun's glint broken by the wake.
8. **Performance.** High: all wave kernels ≤ 0.15 ms with 8 contacts; the water's added shading ≤ 0.05 ms at 1920 × 1080.

## What great looks like

A still pond at golden hour, the sun's glint a long smear: a duck-sized float paddles across and the glint shatters into the V of its wake, Kelvin's half-angle crisp, the short wavelets outrunning the long, curved transverse crests filling the V; the wake reaches the reed bank and comes back, crossing itself in a diamond lattice. A stone dropped in: a ring of fast capillary wavelets leads, slower long waves follow, the ring breaks over a submerged rock. A boat leaves a wake that is still visible 100 m behind it, slowly fading. In a river the wader's ripples are swept downstream into a teardrop. And nothing gives the machinery away: no hard edge where the fine window ends, no ring standing round a body at rest, no stair-stepped crests, no shimmer at a grazing angle.

## Deviations from this plan

None yet; the plan is this note.

## Pending index

Stored in home-still by `hs paper download` (conversion by the watcher, not checked): `10.1145_2980179.2982415` (Canabal), `10.1145_3306346.3323002` (Schreck), `10.1145_97880.97884` (Kass & Miller, markdown present), `10.1017_jfm.2013.607` (stored file is Darmon et al. 2014), `10.1016_j.euromechflu.2020.10.008` (Colen & Kolomeisky), `10.1017_jfm.2014.530`. Read from local copies, not in home-still: Tessendorf 2004 (iWave and course notes), Ottosson 2011, Yuksel 2007, Jeschke & Wojtan 2017, Crest source and docs. `distill_index` skipped: the MCP bridge returns HTTP 404.

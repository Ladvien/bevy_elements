# Wind field — research note

## Sources

- Stam, J. (1999). Stable Fluids. *SIGGRAPH '99*, pp. 121–128. DOI `10.1145/311535.311548`. Stem `10.1145_311535.311548` (scribe markdown read): §2 "Stable Navier–Stokes" (Helmholtz–Hodge projection, Poisson equation with Neumann boundary condition, Eq. 4), the method-of-characteristics advection step (unconditionally stable: the new value never exceeds the largest old value), the solver step order add-force → advect → diffuse → project, and the remark that relaxation schemes "have poor convergence and usually require many iterations".
- Bridson, R., Hourihan, J., Nordenstam, M. (2007). Curl-noise for procedural fluid flow. *ACM Trans. Graph.* 26(3) (SIGGRAPH 2007), art. 46. DOI `10.1145/1275808.1276435`. Read from `https://www.cs.ubc.ca/~rbridson/docs/bridson-siggraph2007-curlnoise.pdf`: $v = \nabla\times\psi$ is exactly divergence-free; 3D uses a vector potential of three noise components (same noise at large offsets); speeds scale as $O(1/L)$ for scale $L$; modulate the potential, not the velocity ($v=\nabla\times(A\psi)$); finite-difference derivatives with a small displacement; ramp the tangential potential to zero near a solid.
- WMO *Guide to Instruments and Methods of Observation* (CIMO), Annex 1.B, siting classification: wind at 10 m geometric height over roughness length $z_0 = 0.03$ m is the WMO standard surface wind for land stations (`https://www.eoas.ubc.ca/courses/atsc303/Instruments/wmo_guides/CIMO_Guide_2014-Met_Site_Classification.pdf`). Davenport–Wieringa class 3 "Open: open flat terrain; grass, few isolated obstacles", $z_0 = 0.03$ m (`https://codes.wmo.int/wmdr/SurfaceRoughnessDavenport/open`). Other classes used for context: sea 0.0002, snow/mud flat 0.005, low crops 0.10, high crops 0.25, parkland 0.5, forest/suburb 1.0 m.
- Harper, B. A., Kepert, J. D., Ginger, J. D. (2010). Guidelines for converting between various wind averaging periods in tropical cyclone conditions. WMO/TD-No. 1555. Read from `https://www.systemsengineeringaustralia.com.au/download/WMO_TC_Wind_Averaging_27_Aug_2010.pdf`: Table 1.1 gust factors at +10 m, reference period 600 s, gust duration 3 s: in-land roughly open terrain 1.66, off-land 1.52, off-sea 1.38, at-sea 1.23; Table 4.1 turbulence intensity $I_u$ and $z_0$: in-land 0.25 / 0.18 m, off-land 0.20 / 0.07 m, off-sea 0.15 / 0.013 m, at-sea 0.10 / 0.0005 m. Both tables are for tropical-cyclone (strong-wind) conditions.
- CEN EN 1991-1-4 (Eurocode 1, wind actions), §4.3.2 and §4.4: neutral log profile and turbulence intensity $I_v(z) = k_I / \ln(z/z_0)$ with $k_I = 1.0$ (from memory of the standard; not re-read this session — treat as [INFERENCE] until checked). It gives $I_v(10\,\mathrm{m}) = 1/\ln(10/0.03) = 0.172$ for $z_0$ = 0.03 m.
- Hillaire (2020) `10.1111/cgf.14050` and Bruneton & Neyret (2008) are not used by wind.

## Model adopted

**Log-law forcing profile** (neutral surface layer; WMO reference exposure):

$$ u(z) = u_{10}\,\frac{\ln\!\big(\max(z_{agl}, z_0)/z_0\big)}{\ln(10/z_0)},\qquad z_0 = 0.03\ \mathrm{m},\ u_{10}\ [\mathrm{m\,s^{-1}}],\ z_{agl}\ [\mathrm m] $$

At $z_{agl} = 10$ m the numerator and denominator are identical, so $u(10\,\mathrm m) = u_{10}$ exactly. Other values (computed): $u(1\,\mathrm m)/u_{10} = 0.604$, $u(2) = 0.723$, $u(5) = 0.881$, $u(20) = 1.119$, $u(50) = 1.277$. $\ln(10/0.03) = 5.8091$. The profile depends only on height, so it has zero divergence on flat ground ($\partial u/\partial x = 0$).

**One simulation step** (Stam 1999 structure; no viscosity/diffusion step: air viscosity is negligible at 4 m cells):

1. *Forcing:* $\mathbf u \leftarrow \mathbf u + (1 - e^{-r\,\Delta t})\,(\mathbf u_{target} - \mathbf u)$ with relaxation rate $r = 0.5\ \mathrm{s^{-1}}$ and $\mathbf u_{target} = \hat{\mathbf d}\,u(z) + \mathbf u_{curl} + \mathbf u_{gust}$ ($\hat{\mathbf d}$ = prevailing wind direction, unit XZ vector, blows toward).
2. *Curl noise:* $\mathbf u_{curl} = 0.35\,g\,u_{10}\ \nabla\times\boldsymbol\psi(\mathbf x/L, t)$, $g$ = `gustiness` ∈ [0, 1], $\boldsymbol\psi$ a vector of three Perlin-noise components (Bridson 2007 §2.2); the curl is taken with central differences of displacement 1e-3 of the noise cell (Bridson used 1e-4 of the domain in single precision; a larger step is chosen for f32 stability on the GPU, [INFERENCE]), normalised so its rms is 1 (the curl of unit-amplitude noise has speeds $O(1/L)$). The potential is multiplied by $A(z_{agl})$ rising smoothly from 0 at the ground to 1 above 4 m (Bridson §2.3–2.4: modulate the potential so the result stays divergence-free; tangential ramp near solids).
3. *Semi-Lagrangian advection:* $\mathbf u^{n+1}(\mathbf x) = \mathbf u^n(\mathbf x - \Delta t\,\mathbf u^n(\mathbf x))$ with trilinear interpolation (Stam 1999 method of characteristics; unconditionally stable).
4. *Solid mask* $m(\mathbf x) \in \{0, 1\}$: 1 where the cell centre is below terrain height; velocity in solid cells is 0 and the Poisson stencil treats them as Neumann walls. In M1 the ground is the plane $y = 0$.
5. *Divergence* (central differences, cell size $h = 4$ m): $\nabla\!\cdot\mathbf u = \frac{u_{x+} - u_{x-} + v_{y+} - v_{y-} + w_{z+} - w_{z-}}{2h}$.
6. *Jacobi pressure solve* of $\nabla^2 p = \nabla\!\cdot\mathbf u$ (Stam Eq. 4, Neumann boundary): $p^{k+1}_{ijk} = \tfrac16\big(\sum_{6\text{ nbrs}} p^k - h^2\,\mathrm{div}_{ijk}\big)$, neighbours in solid cells replaced by the centre value; 10/20/30/40 iterations by quality; pressure warm-started from the previous frame.
7. *Projection:* $\mathbf u \leftarrow \mathbf u - \nabla p$ (central differences, same $h$).
8. *Publish:* $(\mathbf u_x, \mathbf u_y, \mathbf u_z, I)$ with $I$ = local turbulence intensity $= |\mathbf u_{curl}|/\max(|\mathbf u|, 0.1)$ is written to the `rgba16float` 3D texture.

**Gusts and gust factor.** Gust events are a Poisson process of rate $0.1\,g\ \mathrm{s^{-1}}$. A gust is a travelling band that adds a peak speed $0.5\,g\,u_{10}$ along $\hat{\mathbf d}$ inside a 30 m wide band moving at $1.2\,u_{10}$. Over open land the peak 3-s gust relative to the 10-min mean is the *gust factor* $G = 1 + g_p I_u$ with turbulence intensity $I_u \approx 0.17$ at 10 m for $z_0 = 0.03$ m (Eurocode form, $= 1/\ln(10/z_0)$) and peak factor $g_p \approx 3.5$–4. The plan's maximum combined fluctuation is $u_{curl}$ rms-scale $0.35\,g$ plus gust-band peak $0.5\,g$, i.e. an effective gust factor $G = 1 + 0.5\,g$ from the band alone ($G$ = 1.10 at $g$ = 0.2, 1.15 at 0.3, 1.25 at 0.5, 1.45 at 0.9), rising to $1 + 0.85\,g$ when a band coincides with the curl peak (1.77 at $g$ = 0.9). WMO/TD-1555 Table 1.1 gives 1.38–1.66 for 3-s gusts in 600 s over open land in tropical-cyclone conditions; the plan's range 1.10–1.45 (band alone) is below that, which is intended: TC turbulence exceeds fair-weather turbulence, and only Thunderstorm/Blizzard (g = 0.9) reaches the lower edge of the table. `WindState.gust_factor` is defined here as $G = 1 + 0.5\,g$ [INFERENCE: the plan names the field but not its formula]. The curl-noise amplitude $0.35\,g\,u_{10}$ corresponds to $I_u = 0.35\,g$: 0.175 at $g = 0.5$, matching the Eurocode $I_u = 0.172$ at 10 m for $z_0 = 0.03$ m, and 0.32 at $g = 0.9$ (inside WMO Table 4.1's 0.25 in-land strong-wind value plus margin).

**Toroidal domain.** Cell $(i,j,k)$ stores world cell $(\lfloor x/h\rfloor,\lfloor y/h\rfloor,\lfloor z/h\rfloor)$ with $i = \lfloor x/h\rfloor \bmod N_x$ etc. The domain re-centres only in whole cells; newly exposed slabs are initialised to the forcing profile. Because addressing is toroidal, the published texture is sampled in world space as $uvw = \mathrm{world}/\mathrm{extent}$ with `ImageAddressMode::Repeat`, so no origin subtraction is needed; `WindFieldParams` still carries `origin` and `extent` so a consumer can fall back to the prevailing wind outside $[\mathrm{origin}, \mathrm{origin} + \mathrm{extent})$. This mapping is not spelled out in the plan; it is the only layout that avoids a per-frame un-wrapping copy.

**Expected Jacobi convergence.** For the 7-point Laplacian with Neumann boundaries on an $N_x\times N_y\times N_z$ grid the Jacobi iteration matrix has eigenvalues $\lambda_{pqr} = \tfrac13(\cos\tfrac{\pi p}{N_x} + \cos\tfrac{\pi q}{N_y} + \cos\tfrac{\pi r}{N_z})$. After $n$ iterations the residual of mode $pqr$ is multiplied by $\lambda_{pqr}^n$. Computed for the plan's quality settings:

| grid | iterations | smoothest-mode factor $\lambda^n$ | rms factor over all modes (cold start, white divergence spectrum) | rms factor over modes with $\lvert\lambda\rvert<0.9$ |
|---|---|---|---|---|
| 32×8×32 (Low) | 10 | 0.984 | 0.088 | 0.042 |
| 48×12×48 (Medium) | 20 | 0.986 | 0.053 | 0.010 |
| 64×16×64 (High) | 30 | 0.988 | 0.039 | 0.0028 |
| 96×24×96 (Ultra) | 40 | 0.993 | 0.031 | 0.00083 |

So 30 Jacobi iterations on 64×16×64 reduce a *white* residual to ≈ 3.9 % rms (residual of the smoothest mode, $p=1$ along one horizontal axis, only to 98.8 %; the checkerboard mode has $\lambda = -1$ and is not damped at all), which is why the pressure is warm-started and why the forcing is built to be nearly divergence-free (log profile: exactly 0; curl noise: 0 up to finite-difference error; only advection interpolation and the gust band inject divergence). A smooth, near-divergence-free field converges across frames because the solve resumes from the previous pressure. The `gpu_wind` divergence bound (< 1e-2 s⁻¹) is expected to hold when the pre-projection mean $|\nabla\!\cdot\mathbf u|$ is below ≈ 0.25 s⁻¹ ($1\times10^{-2}/0.039$); this is [INFERENCE] and the test decides it.

## Parameters

| name | symbol | default | unit | source |
|---|---|---|---|---|
| Cell size, horizontal | $h_{xz}$ | 4 | m | plan 1.3 |
| Cell size, vertical | $h_y$ | 4 | m | plan 1.3 |
| Grid, Low / Medium / High / Ultra | $N_x\times N_y\times N_z$ | 32×8×32 / 48×12×48 / 64×16×64 / 96×24×96 | cells | plan Shared architecture (ElementsQuality) |
| Domain extent, High | — | 256×64×256 | m | derived ($N h$) |
| Jacobi iterations, Low / Medium / High / Ultra | $n$ | 10 / 20 / 30 / 40 | 1 | plan 1.3; convergence analysis above |
| Roughness length | $z_0$ | 0.03 | m | WMO CIMO Annex 1.B; Davenport–Wieringa class 3 "Open" |
| Reference height | $z_{ref}$ | 10 | m | WMO standard surface wind height |
| Forcing relaxation rate | $r$ | 0.5 | s⁻¹ | plan 1.3 (design value; e-folding 2 s) |
| Curl-noise amplitude | $c_{curl}$ | 0.35 × gustiness × $u_{10}$ | m/s | plan 1.3; $I_u = 0.35g$ checked against Eurocode $I_u(10\,\mathrm m) = 0.172$ and WMO TD-1555 Table 4.1 |
| Curl-noise length scale | $L$ | 32 | m | plan: not specified; [INFERENCE] 8 cells, speeds $O(1/L)$ per Bridson 2007 §2.2 |
| Gust rate | $\lambda_g$ | 0.1 × gustiness | s⁻¹ | plan 1.3 |
| Gust spawn distance | — | 200 | m upwind of camera | plan 1.3 |
| Gust speed | $c_g$ | 1.2 $u_{10}$ | m/s | plan 1.3 |
| Gust band width | $w_g$ | 30 | m | plan 1.3 |
| Gust strength | $a_g$ | 0.5 × gustiness × $u_{10}$ | m/s | plan 1.3; gust factor vs WMO TD-1555 Table 1.1 above |
| Gust factor | $G$ | $1 + 0.5\,g$ | 1 | derived (this note) |
| Wind speed scale | $\bar u/4$ | Climate.mean_wind_ms / 4 | 1 | plan 1.2 |
| Unit workgroup (3D) | — | 4×4×4 | threads | plan Shared architecture; Bevy WebGPU baseline 256 invocations (`docs/research/bevy-0.20-render.md`) |
| Published texture format | — | `rgba16float`, 3D, filterable, Repeat | — | plan 1.3; `bevy-0.20-render.md` storage format table (write-only, `rgba16float` allowed) |

## GPU mapping

Domain: camera-centred, whole-cell scrolling, toroidal addressing (above). One compute pass per kernel per frame, workgroup `@workgroup_size(4, 4, 4)` (64 invocations; limit 256). Dispatch counts: High 64×16×64 cells → 16×4×16 = 1024 workgroups per kernel; Ultra 96×24×96 → 24×6×24 = 3456.

State buffers: two ping-pong `Handle<ShaderBuffer>` of `array<vec4<f32>>` (velocity xyz, w = turbulence), `read`/`read_write` via `var<storage, ...>`; one `array<f32>` divergence; two `array<f32>` pressure (ping-pong); one `array<u32>` solid mask. Sizes at High: vec4 buffer 65 536 × 16 B = 1 MiB; at Ultra 221 184 × 16 B = 3.4 MiB; f32 buffers 0.25 / 0.84 MiB. All created with `RenderAssetUsages::RENDER_WORLD`, usage `STORAGE | COPY_SRC | COPY_DST` (the `ShaderBuffer` default).

Per-kernel binding count (storage buffers / storage textures / uniforms), all ≤ 8 buffers and ≤ 4 storage textures per stage (WebGPU defaults, `docs/research/bevy-0.20-render.md` limits table: 8 and 4):

| kernel | storage buffers | storage textures | notes |
|---|---|---|---|
| forcing (+ curl noise + gust bands) | 2 (vel in, vel out) | 0 | uniform: params; gust bands (≤ 16) in the params uniform |
| advect | 2 (vel in, vel out) + 1 (mask) = 3 | 0 | |
| solid mask | 2 (terrain height in, mask out); 1 in M1 (flat ground: writes mask only) | 0 | M3 binds the terrain height buffer |
| divergence | 3 (vel in, mask in, div out) | 0 | |
| Jacobi (×n) | 4 (p in, p out, div in, mask in) | 0 | ping-pong swaps bind groups per iteration |
| project | 3 (vel in/out, pressure in) + 1 (mask) = 4 | 0 | |
| publish | 1 (vel in) | 1 (`rgba16float` write-only 3D) | |

Maximum: 4 storage buffers in one stage, 1 storage texture in one stage. No `read_write` storage texture is used (the publish texture is write-only; consumers sample it as a filterable 3D texture). If a device reports the `downlevel` limit of 4 storage buffers the Jacobi and project kernels still fit exactly (4); the plan's splitting rule is not needed.

Readback: the plan's `EnvironmentSnapshot` copies the velocity buffer (High: 1 MiB at 4 Hz); Ultra's 3.4 MiB is also below the 1024² downsample threshold. Frame cost at High is the plan's wind + fog compute ≤ 0.3 ms (Verification M1); 30 Jacobi dispatches of 65 536 cells are small, but each is a separate dispatch with a barrier, so the bench must measure it.

## Validation targets

1. **`gpu_wind`** (plan Verification M1, needs a GPU, `#[ignore = "needs a GPU"]`): after 120 frames with $u_{10}$ = 10 m/s, the mean $|\mathbf u|$ over the horizontal slab at 10 m AGL is in [8, 12] m/s and the mean $|\nabla\!\cdot\mathbf u|$ after projection is < 1e-2 s⁻¹. Check of the forcing: the log-law gives $u(10\,\mathrm m) = u_{10}$ exactly (ratio 1.000 by construction), so the field relaxes to 10 m/s at 10 m with the noise on top; with curl-noise rms $0.35\,g\,u_{10}$ (3.5 m/s for $g = 1$, 1.75 m/s for $g = 0.5$) the mean speed magnitude is $\approx\sqrt{100 + \sigma^2}$ = 10.15 (g = 0.5) to 10.6 (g = 1), inside [8, 12]. Source: plan; arithmetic here.
2. **Log-law values (CPU unit test of the shared `wind_profile` fn):** $u(10)/u_{10} = 1.0$ (±1e-6); $u(1)/u_{10} = 0.6036$, $u(2) = 0.7229$, $u(5) = 0.8807$, $u(20) = 1.1193$, $u(50) = 1.2771$ (±1e-3); $u(z \le z_0) = 0$ is avoided by $\max(z, z_0)$, which gives $u(z \le 0.03) = 0$. Source: formula, WMO $z_0$.
3. **Divergence-free forcing:** curl-noise potential evaluated at 10⁴ random points gives $|\nabla\!\cdot(\nabla\times\psi)|$ below $10^{-3}\,u_{rms}/h$ with central differences (analytic identity, Bridson 2007 §2.1; finite-difference residual only).
4. **Jacobi residual (informational, GPU):** the residual norm after 30 iterations on 64×16×64 for a white-noise divergence cold start is ≤ 0.04 of the initial norm (computed 0.039); smoothest-mode factor 0.988. Source: eigenvalue analysis above.
5. **Gust relations (CPU):** with $g = 0.9$ and $u_{10} = 10$: gust band strength 4.5 m/s, speed 12 m/s, width 30 m, origin = camera XZ − 200 m·$\hat d$; the Poisson rate is 0.09 s⁻¹, so over 3600 s the count is in [250, 400] (mean 324, ±4σ ≈ 72). Source: plan 1.3 numbers.
6. **Toroidal scrolling:** moving the camera by exactly $k$ cells shifts the sampled field by $k$ cells with no value change in the overlap (bit-equal on CPU model). Source: design requirement of the plan 1.3.

## Deviations from this plan

None. (Items marked [INFERENCE] are choices where the plan was silent: curl-noise length scale, finite-difference step, the exact definition of `gust_factor`, the toroidal Repeat-sampled `WindField` layout.)

## Pending index

`distill_index` skipped (distill/Qdrant down) for:

- `10.1145_311535.311548` (Stam 1999; markdown present, not indexed)
- `10.1145_1275808.1276435` (Bridson et al. 2007; read from the author's PDF, not in home-still, not downloaded/converted)
- `wmo-td-1555-harper-kepert-ginger-2010` (no DOI; URL above)

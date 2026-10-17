# Celestial — research note

Scope: plan §1.1 — `WorldClock`/`GeoLocation`, `astro.rs` (Sun, Moon, sidereal time, horizontal coordinates, refraction), the `Sun`/`Moon` light entities, moon-phase illuminance, moon disc, stars, auto exposure. Everything in `astro.rs` is pure `f64`, CPU only, deterministic.

Method note: the Meeus book was not available as text. Its worked examples were confirmed by (a) an independent re-implementation of the formulas below (throw-away Python prototype, not committed), and (b) the MIT-licensed `commenthol/astronomia` JS port, whose unit tests assert the book's published example numbers (cited per row). The SPA, Krisciunas & Schaefer, Jensen, Ballesteros, NASA, CDS and HEASARC items were read from the primary documents in this session.

## Sources

Papers (all five DOIs were read from `/tmp/research/{md,pdf}`; Ballesteros read from the author's PDF):
- `10.1016/j.solener.2003.12.003` — Reda & Andreas, "Solar position algorithm for solar radiation applications", Solar Energy 76(5), 2004. Worked example: NREL/TP-560-34302 (rev. Jan 2008) appendix A.5, Table A5.1, https://docs.nlr.gov/docs/fy08osti/34302.pdf (the journal PDF has no numeric example; the tech report does).
- `10.1086/132921` — Krisciunas & Schaefer, "A model of the brightness of moonlight", PASP 103, 1033, 1991. Eqs. (8), (9), (20) read from the PDF.
- `10.1145/383259.383306` — Jensen, Durand, Dorsey, Stark, Shirley, Premože, "A physically-based night sky model", SIGGRAPH 2001. §3.3, §4.2–4.3, §5, §6, Fig. 2, Eqs. 5–7, Appendix.
- `10.1111/cgf.14050` — Hillaire, "A scalable and production ready sky and atmosphere rendering technique", CGF 39(4), 2020 (the model Bevy's `Atmosphere` implements).
- `10.1111/j.1467-8659.2008.01245.x` — Bruneton & Neyret, "Precomputed atmospheric scattering", CGF 27(4), 2008 (context only; Bevy does not use its tables).
- `10.1209/0295-5075/97/34008` — Ballesteros, "New insights into black bodies", EPL 97, 34008, 2012 (arXiv:1201.1809), https://www.uv.es/ferbaro/papers/new.pdf. Eq. (14) read.
- Meeus, *Astronomical Algorithms*, 2nd ed., Willmann-Bell 1998 (no DOI): ch. 7, 12, 13, 16, 21, 22, 25, 47, 48. Example numbers cross-checked against https://github.com/commenthol/astronomia (`test/{julian,sidereal,solar,moonposition,moonillum}.test.js`) and the prototype.
- Espenak & Meeus, ΔT polynomials, https://eclipse.gsfc.nasa.gov/SEhelp/deltatpoly2004.html (read).
- Exposure calibration constants: https://en.wikipedia.org/wiki/Light_meter (Calibration constants: K = 12.5 reflected, C = 250 flat incident receptor, ISO 2720:1974); Bevy `Exposure` at `bevy_camera-0.20.0/src/camera.rs:233-284` (`exposure = 2^-ev100 / 1.2`, Filament formula); `bevy-0.20-render.md` §Atmosphere/Exposure.
- Kang et al., "Design of advanced color temperature control system for HDTV applications", J. Korean Phys. Soc. 41(6), 2002 — Planckian-locus cubic spline, coefficients confirmed at https://en.wikipedia.org/wiki/Planckian_locus (source dump read).
- Patry, J. (2021). Real-time samurai cinema: lighting, atmosphere and tonemapping in Ghost of Tsushima. SIGGRAPH 2021, *Advances in Real-Time Rendering in Games*. Corpus stem `s2021-advances-pdf-66-mb` (shard `s2/`; the markdown has no page numbers, so sections are cited by slide title with corpus line numbers). Used: "Tonemapping | Exposure" (lines 2990–3060: switch to "a primarily illuminance-based exposure system, which takes albedo out of the equation altogether"; luminance used "only if the highlight luminance exceeds a threshold", e.g. fire particles, bright specular highlights); "Tonemapping | Purkinje Shift" (lines 3800–4340: constants and the 0.05 lux demonstrations); "Atmospheric Lighting" (line 2043: the 3D LUTs are resampled each frame "for current sun and moon angles"). Not used: the clouds, haze and sun/moon bounce sections (see `fog.md`; clouds are M2).
- Akenine-Möller, Haines, Hoffman, et al., *Real-Time Rendering* 4th ed. Corpus stem `book_real_time_rendering_4th_akenine`: §8.1 photometry footnote 1 (corpus line 8077): photopic vision above 3.4 cd/m² ("twilight or brighter"), scotopic below 0.034 cd/m² ("a moonless night or darker"). §14.4 (sky rendering) was read for celestial content and adds nothing beyond Hillaire 2020: it has no moon, night-sky or exposure material.
- Checked, nothing for celestial: `s2009-advances-materials-updated-july-12th2016-slides-pdf` (Bungie 2009: Bruneton-based sky and a CIE sky-luminance distribution table, no night, moon or exposure content), `s2017-advances-pdf-slides-2` (Decima 2017: GGX area lights, AA, height fog; used in `fog.md`), `s2006-advances-materials-11` and `-12` (Wenzel 2006: Nishita sky updated on the CPU over time of day, no moon, night or exposure content; used in `fog.md`). A corpus search for "Krisciunas", "moonlight … lux", "Julian day" and "night sky" with rendering terms found no astronomy or night-sky model.

Data:
- Yale Bright Star Catalogue, 5th rev. ed. (Hoffleit & Warren 1991, preliminary). CDS V/50: https://cdsarc.cds.unistra.fr/ftp/cats/V/50/ (`ReadMe`, `catalog.gz`, 9110 records). HEASARC table `bsc5p`: https://heasarc.gsfc.nasa.gov/W3Browse/star-catalog/bsc5p.html (derived by HEASARC from the ADC/CDS file).
- NASA SVS CGI Moon Kit (ID 4720), https://svs.gsfc.nasa.gov/4720/ ; map https://svs.gsfc.nasa.gov/vis/a000000/a004700/a004720/lroc_color_2k.jpg (2048×1024 JPEG, 447 KB, sRGB, centred on 0° longitude).

### Data licensing (explicit answers)

**Bright Star Catalogue.** Evidence read in this session:
- HEASARC "Use of HEASARC Data" (https://heasarc.gsfc.nasa.gov/docs/heasarc/data_policy.html): "HEASARC materials are all available freely for your use." Only a citation/acknowledgement request for publications.
- The CDS V/50 `ReadMe` contains description, byte layout and history only — no restriction on redistribution. Origin: NASA GSFC NSSDC/Astronomical Data Center (Warren) and Yale (Hoffleit).
- A web search summary reports the data.gov record for the catalogue listing licence https://www.usa.gov/government-works (US-government work, public domain). `catalog.data.gov/dataset/bright-star-catalog` returned HTTP 404 when fetched directly, so this one is not independently verified.
- The crate ships only 4 × f32 per star (RA, Dec, V, B−V): astronomical facts, not creative expression.

**Decision: the catalogue path is used; the plan's "delete the catalogue, generate procedurally" contingency is NOT triggered.** The terms are permissive (free use, no copyleft, no non-commercial clause), therefore MIT/Apache-compatible. Residual risk: no file carries an explicit CC0/PD notice; this is a reading of the sources, not legal advice. `PROVENANCE.md` must quote the HEASARC sentence, give both URLs, and state "positions/magnitudes are facts; derived binary of 8404 records". If Main wants zero residual risk, switch to the procedural generator (seed `0x5EED_57A5`).

**Moon map.** NASA Images and Media Usage Guidelines, https://www.nasa.gov/nasa-brand-center/images-and-media/ : NASA content, including "files used in the rendition of 3-dimensional models, such as texture maps", is "generally … not subject to copyright in the United States"; "NASA should be acknowledged as the source"; no implied endorsement. The SVS page asks for credit: "NASA's Scientific Visualization Studio" (visualizer Ernie Wright, scientist Noah Petro; data LRO LROC WAC Hapke-normalised mosaic + LOLA LDAM). Verdict: usable and redistributable with attribution; `PROVENANCE.md` carries the credit line, the exact URL and the processing (downscale to 512×256 to luminance-normalised albedo). The page notes the map is "optimized for aesthetics, not science" (white balance and range adjusted) — hence the photometric calibration below instead of trusting texel values.

## Model adopted

Conventions: angles in degrees unless `rad`; $T$ in Julian centuries of 36525 ephemeris days from J2000.0; all trigonometric arguments converted to radians; longitude east-positive (`GeoLocation`); azimuth measured from north through east (SPA/Meeus-ΔA+180° convention). World axes (implementation decision, no source): $+Y$ up, east $=+X$, north $=-Z$; direction toward a body is $\hat d = (\cos h\sin A,\ \sin h,\ -\cos h\cos A)$.

### Time (Meeus ch. 7, 12; Espenak–Meeus)

Julian day (Meeus 7.1, Gregorian only — `CivilDate` is proleptic Gregorian, the Julian-calendar branch is not implemented). With $Y,M$ adjusted ($M\le2 \Rightarrow Y\leftarrow Y-1,\ M\leftarrow M+12$), $A=\lfloor Y/100\rfloor$, $B=2-A+\lfloor A/4\rfloor$, $D$ = day + UT/24 (UT in hours = local hour − `utc_offset_hours`; date rolls with it):

$$JD=\lfloor 365.25(Y+4716)\rfloor+\lfloor 30.6001(M+1)\rfloor+D+B-1524.5\quad[\text{days}]$$

Ephemeris day and century: $JDE=JD_{UT}+\Delta T/86400$, $T=(JDE-2451545)/36525$. ΔT (s) by Espenak–Meeus with $y=\text{year}+(\text{month}-0.5)/12$:

| range of $y$ | $\Delta T$ [s], $t$ as given |
|---|---|
| 1900–1920 | $-2.79+1.494119t-0.0598939t^2+0.0061966t^3-0.000197t^4$, $t=y-1900$ |
| 1920–1941 | $21.20+0.84493t-0.076100t^2+0.0020936t^3$, $t=y-1920$ |
| 1941–1961 | $29.07+0.407t-t^2/233+t^3/2547$, $t=y-1950$ |
| 1961–1986 | $45.45+1.067t-t^2/260-t^3/718$, $t=y-1975$ |
| 1986–2005 | $63.86+0.3345t-0.060374t^2+0.0017275t^3+0.000651814t^4+0.00002373599t^5$, $t=y-2000$ |
| 2005–2050 | $62.92+0.32217t+0.005589t^2$, $t=y-2000$ |
| 2050–2150 | $-20+32\,((y-1820)/100)^2-0.5628\,(2150-y)$ |
| outside 1900–2150 | $-20+32\,((y-1820)/100)^2$ (accuracy not claimed) |

Mean sidereal time at Greenwich, $T_u=(JD_{UT}-2451545)/36525$ (Meeus 12.4):

$$\theta_0=280.46061837+360.98564736629\,(JD_{UT}-2451545)+0.000387933\,T_u^2-T_u^3/38710000\quad[°,\ \bmod 360]$$

Apparent: $\theta_0+\Delta\psi\cos\varepsilon$. Local apparent sidereal time $\theta=\theta_0+\Delta\psi\cos\varepsilon+\lambda_{east}$.

### Nutation and obliquity (Meeus ch. 22 low accuracy, ≤0.5″/0.1″)

$\Omega=125.04452-1934.136261T$, $L=280.4665+36000.7698T$, $L'=218.3165+481267.8813T$ (°):

$$\Delta\psi=\frac{-17.20\sin\Omega-1.32\sin2L-0.23\sin2L'+0.21\sin2\Omega}{3600},\qquad \Delta\varepsilon=\frac{9.20\cos\Omega+0.57\cos2L+0.10\cos2L'-0.09\cos2\Omega}{3600}$$

$$\varepsilon_0=23°26'21.448''-46.8150''T-0.00059''T^2+0.001813''T^3,\qquad \varepsilon=\varepsilon_0+\Delta\varepsilon$$

### Sun (Meeus ch. 25, low accuracy ≈ 0.01°)

$$L_0=280.46646+36000.76983T+0.0003032T^2,\quad M=357.52911+35999.05029T-0.0001537T^2,\quad e=0.016708634-0.000042037T-0.0000001267T^2$$
$$C=(1.914602-0.004817T-0.000014T^2)\sin M+(0.019993-0.000101T)\sin2M+0.000289\sin3M$$
$$\odot=L_0+C,\quad \nu=M+C,\quad R=\frac{1.000001018(1-e^2)}{1+e\cos\nu}\ [\text{AU}]$$
$$\Omega_s=125.04-1934.136T,\quad \lambda=\odot-0.00569-0.00478\sin\Omega_s,\quad \varepsilon_s=\varepsilon_0+0.00256\cos\Omega_s$$
$$\alpha=\operatorname{atan2}(\cos\varepsilon_s\sin\lambda,\ \cos\lambda),\qquad \delta=\arcsin(\sin\varepsilon_s\sin\lambda)$$

($\varepsilon_0$ here from the polynomial above, $\Delta\varepsilon$ not added; the $0.00256\cos\Omega_s$ term is Meeus's low-accuracy replacement.) For lunar phase the Sun's *true* RA/Dec ($\odot$ and $\varepsilon$ with nutation, no aberration/Ω term) are used; the difference is $<0.001°$. Sun distance in km: $R\cdot149597870.7$.

### Equatorial → horizontal (Meeus ch. 13)

Hour angle (west positive): $H=\theta-\alpha$ with $\theta$ already including $\lambda_{east}$.

$$\sin h=\sin\varphi\sin\delta+\cos\varphi\cos\delta\cos H,\qquad A_N=\operatorname{atan2}(\sin H,\ \cos H\sin\varphi-\tan\delta\cos\varphi)+180°$$

Equivalent east/north/up components for star vectors: $E=-\cos\delta\sin H$, $N=\sin\delta\cos\varphi-\cos\delta\cos H\sin\varphi$, $U=\sin\delta\sin\varphi+\cos\delta\cos H\cos\varphi$.

### Refraction (Meeus ch. 16 Saemundsson = SPA Eq. 42)

For true altitude $h$ (°), pressure $P$ (mbar), temperature $T_c$ (°C):

$$R=\frac{1.02}{60\tan\!\big(h+\tfrac{10.3}{h+5.11}\big)}\cdot\frac{P}{1010}\cdot\frac{283}{273+T_c}\quad[°],\qquad h_{app}=h+R$$

SPA Eq. 42 is this exact expression (read from the paper, with $P/1010\cdot283/(273+T)$). Applied only when $h\ge-0.8333°$ (SPA's sunrise convention; the cut-off itself was not read in the SPA reference code — [INFERENCE]); below it $R=0$ and no test asserts a value there. Default $P=1010$ mbar, $T_c=10$ °C (the formula's reference state). The 4 µ-degree $R(90°)$ residual Meeus mentions is omitted. `SunState.altitude_deg` is the **true (unrefracted)** altitude and `is_daylight()` tests $h>-0.833°$ on it; the light `Transform` uses $h_{app}$.

Sun parallax for observers: $\xi=8.794''/R_{AU}$, topocentric $h\leftarrow h-\xi\cos h$ (≤0.0024°; used by the SPA validation, optional in the crate).

### Moon position (Meeus ch. 47, truncated)

Mean elements ($T$ from JDE), all in degrees:

$$L'=218.3164477+481267.88123421T-0.0015786T^2+T^3/538841-T^4/65194000$$
$$D=297.8501921+445267.1114034T-0.0018819T^2+T^3/545868-T^4/113065000$$
$$M=357.5291092+35999.0502909T-0.0001536T^2+T^3/24490000$$
$$M'=134.9633964+477198.8675055T+0.0087414T^2+T^3/69699-T^4/14712000$$
$$F=93.272095+483202.0175233T-0.0036539T^2-T^3/3526000+T^4/863310000$$
$$A_1=119.75+131.849T,\quad A_2=53.09+479264.29T,\quad A_3=313.45+481266.484T,\quad E=1-0.002516T-0.0000074T^2$$

Periodic sums, with argument $\phi_i=D\,d_i+M\,m_i+M'\,m'_i+F\,f_i$ and factor $E^{|m_i|}$ on every term:

$$\Sigma_l=\sum_i l_i\,E^{|m_i|}\sin\phi_i+3958\sin A_1+1962\sin(L'-F)+318\sin A_2$$
$$\Sigma_r=\sum_i r_i\,E^{|m_i|}\cos\phi_i$$
$$\Sigma_b=\sum_j b_j\,E^{|m_j|}\sin\phi_j-2235\sin L'+382\sin A_3+175\sin(A_1-F)+175\sin(A_1+F)+127\sin(L'-M')-115\sin(L'+M')$$
$$\lambda_{geo}=L'+\Sigma_l\cdot10^{-6},\qquad \beta=\Sigma_b\cdot10^{-6},\qquad \Delta=385000.56+\Sigma_r\cdot10^{-3}\ [\text{km}],\qquad \lambda=\lambda_{geo}+\Delta\psi$$

Truncation rule (chosen by the prototype over 3000 random dates, 1900–2100, against the full 60+60-term series): keep Table 47.A rows with $|l|\ge500$ or $|r|\ge2000$ (51 of 60 rows; dropped rows: 50, 51, 53, 54, 55, 56, 57, 58, 59) and Table 47.B rows with $|b|\ge500$ (34 of 60; dropped rows 35–60). The additive $A_1,A_2,A_3$ terms are always kept. Measured truncation error: $|\Delta\lambda|\le0.0023°$, $|\Delta\beta|\le0.0041°$, $|\Delta\Delta|\le2.9$ km over that range; at Ex. 47.a: $0.0001°$, $0.0004°$, $0.69$ km. (Thresholds 1000/3000/1000 give 42+29 terms and ≤0.0053°/0.0066°/12.6 km — rejected, breaks the ±10 km target margin.)

Kept terms (coefficients as published in Meeus Tables 47.A/47.B, verified by reproducing Ex. 47.a to all printed digits with the full tables). $l$ in $10^{-6}$°, $r$ in $10^{-3}$ km, $b$ in $10^{-6}$°:

| # | D | M | M′ | F | l | r |
|---|---|---|---|---|---|---|
| 1 | 0 | 0 | 1 | 0 | 6288774 | -20905355 |
| 2 | 2 | 0 | -1 | 0 | 1274027 | -3699111 |
| 3 | 2 | 0 | 0 | 0 | 658314 | -2955968 |
| 4 | 0 | 0 | 2 | 0 | 213618 | -569925 |
| 5 | 0 | 1 | 0 | 0 | -185116 | 48888 |
| 6 | 0 | 0 | 0 | 2 | -114332 | -3149 |
| 7 | 2 | 0 | -2 | 0 | 58793 | 246158 |
| 8 | 2 | -1 | -1 | 0 | 57066 | -152138 |
| 9 | 2 | 0 | 1 | 0 | 53322 | -170733 |
| 10 | 2 | -1 | 0 | 0 | 45758 | -204586 |
| 11 | 0 | 1 | -1 | 0 | -40923 | -129620 |
| 12 | 1 | 0 | 0 | 0 | -34720 | 108743 |
| 13 | 0 | 1 | 1 | 0 | -30383 | 104755 |
| 14 | 2 | 0 | 0 | -2 | 15327 | 10321 |
| 15 | 0 | 0 | 1 | 2 | -12528 | 0 |
| 16 | 0 | 0 | 1 | -2 | 10980 | 79661 |
| 17 | 4 | 0 | -1 | 0 | 10675 | -34782 |
| 18 | 0 | 0 | 3 | 0 | 10034 | -23210 |
| 19 | 4 | 0 | -2 | 0 | 8548 | -21636 |
| 20 | 2 | 1 | -1 | 0 | -7888 | 24208 |
| 21 | 2 | 1 | 0 | 0 | -6766 | 30824 |
| 22 | 1 | 0 | -1 | 0 | -5163 | -8379 |
| 23 | 1 | 1 | 0 | 0 | 4987 | -16675 |
| 24 | 2 | -1 | 1 | 0 | 4036 | -12831 |
| 25 | 2 | 0 | 2 | 0 | 3994 | -10445 |
| 26 | 4 | 0 | 0 | 0 | 3861 | -11650 |
| 27 | 2 | 0 | -3 | 0 | 3665 | 14403 |
| 28 | 0 | 1 | -2 | 0 | -2689 | -7003 |
| 29 | 2 | 0 | -1 | 2 | -2602 | 0 |
| 30 | 2 | -1 | -2 | 0 | 2390 | 10056 |
| 31 | 1 | 0 | 1 | 0 | -2348 | 6322 |
| 32 | 2 | -2 | 0 | 0 | 2236 | -9884 |
| 33 | 0 | 1 | 2 | 0 | -2120 | 5751 |
| 34 | 0 | 2 | 0 | 0 | -2069 | 0 |
| 35 | 2 | -2 | -1 | 0 | 2048 | -4950 |
| 36 | 2 | 0 | 1 | -2 | -1773 | 4130 |
| 37 | 2 | 0 | 0 | 2 | -1595 | 0 |
| 38 | 4 | -1 | -1 | 0 | 1215 | -3958 |
| 39 | 0 | 0 | 2 | 2 | -1110 | 0 |
| 40 | 3 | 0 | -1 | 0 | -892 | 3258 |
| 41 | 2 | 1 | 1 | 0 | -810 | 2616 |
| 42 | 4 | -1 | -2 | 0 | 759 | -1897 |
| 43 | 0 | 2 | -1 | 0 | -713 | -2117 |
| 44 | 2 | 2 | -1 | 0 | -700 | 2354 |
| 45 | 2 | 1 | -2 | 0 | 691 | 0 |
| 46 | 2 | -1 | 0 | -2 | 596 | 0 |
| 47 | 4 | 0 | 1 | 0 | 549 | -1423 |
| 48 | 0 | 0 | 4 | 0 | 537 | -1117 |
| 49 | 4 | -1 | 0 | 0 | 520 | -1571 |
| 52 | 0 | 0 | 2 | -2 | -381 | -4421 |
| 60 | 2 | 0 | -1 | -2 | 0 | 8752 |

| # | D | M | M′ | F | b |
|---|---|---|---|---|---|
| 1 | 0 | 0 | 0 | 1 | 5128122 |
| 2 | 0 | 0 | 1 | 1 | 280602 |
| 3 | 0 | 0 | 1 | -1 | 277693 |
| 4 | 2 | 0 | 0 | -1 | 173237 |
| 5 | 2 | 0 | -1 | 1 | 55413 |
| 6 | 2 | 0 | -1 | -1 | 46271 |
| 7 | 2 | 0 | 0 | 1 | 32573 |
| 8 | 0 | 0 | 2 | 1 | 17198 |
| 9 | 2 | 0 | 1 | -1 | 9266 |
| 10 | 0 | 0 | 2 | -1 | 8822 |
| 11 | 2 | -1 | 0 | -1 | 8216 |
| 12 | 2 | 0 | -2 | -1 | 4324 |
| 13 | 2 | 0 | 1 | 1 | 4200 |
| 14 | 2 | 1 | 0 | -1 | -3359 |
| 15 | 2 | -1 | -1 | 1 | 2463 |
| 16 | 2 | -1 | 0 | 1 | 2211 |
| 17 | 2 | -1 | -1 | -1 | 2065 |
| 18 | 0 | 1 | -1 | -1 | -1870 |
| 19 | 4 | 0 | -1 | -1 | 1828 |
| 20 | 0 | 1 | 0 | 1 | -1794 |
| 21 | 0 | 0 | 0 | 3 | -1749 |
| 22 | 0 | 1 | -1 | 1 | -1565 |
| 23 | 1 | 0 | 0 | 1 | -1491 |
| 24 | 0 | 1 | 1 | 1 | -1475 |
| 25 | 0 | 1 | 1 | -1 | -1410 |
| 26 | 0 | 1 | 0 | -1 | -1344 |
| 27 | 1 | 0 | 0 | -1 | -1335 |
| 28 | 0 | 0 | 3 | 1 | 1107 |
| 29 | 4 | 0 | 0 | -1 | 1021 |
| 30 | 4 | 0 | -1 | 1 | 833 |
| 31 | 0 | 0 | 1 | -3 | 777 |
| 32 | 4 | 0 | -2 | 1 | 671 |
| 33 | 2 | 0 | 0 | -3 | 607 |
| 34 | 2 | 0 | 2 | -1 | 596 |

Equatorial coordinates (Meeus 13.3–13.4): $\alpha=\operatorname{atan2}(\sin\lambda\cos\varepsilon-\tan\beta\sin\varepsilon,\ \cos\lambda)$, $\delta=\arcsin(\sin\beta\cos\varepsilon+\cos\beta\sin\varepsilon\sin\lambda)$, $\varepsilon=\varepsilon_0+\Delta\varepsilon$. Horizontal parallax $\sin\pi_m=6378.137/\Delta$ (Ex. 47.a: $\pi_m=0.991990°$). Topocentric Moon (needed: up to 1° offset): in the geocentric horizontal frame, $\vec m=\Delta\,\hat d_{geo}$, observer $=(0,0,6378.137)$ km up; topocentric direction $=\widehat{\vec m-\vec o}$. Neglects Earth oblateness and the geodetic/geocentric latitude difference (≤0.19° of latitude ⇒ ≤~0.003° of parallax) — [INFERENCE] size of the error, not asserted by a test.

### Moon phase and bright limb (Meeus ch. 48)

With Sun $(\alpha_0,\delta_0,R)$ and Moon $(\alpha,\delta,\Delta)$, $R$ and $\Delta$ in km:

$$\cos\psi=\sin\delta_0\sin\delta+\cos\delta_0\cos\delta\cos(\alpha_0-\alpha),\qquad \tan i=\frac{R\sin\psi}{\Delta-R\cos\psi},\qquad k=\frac{1+\cos i}{2}$$

$i$ = phase angle (0° full, 180° new), $k$ = illuminated fraction. Bright-limb position angle: $\tan\chi=\dfrac{\cos\delta_0\sin(\alpha_0-\alpha)}{\sin\delta_0\cos\delta-\cos\delta_0\sin\delta\cos(\alpha_0-\alpha)}$. The low-accuracy shortcut (Meeus 48.4, $i\approx180°-D-6.289\sin M'+2.100\sin M-1.274\sin(2D-M')-0.658\sin2D-0.214\sin2M'-0.110\sin D$) is NOT used; it gives $k=0.6801$ vs $0.6786$ for Ex. 48.a. `SunState.moon_illuminated_fraction` $=k$, `moon_phase` $=k$ (the plan defines "0 new..1 full"; the two fields are therefore identical — Main may prefer `moon_phase` to be the synodic cycle fraction $((\lambda_{moon}-\lambda_{sun})/360°)\bmod1$ to carry waxing/waning; not required by any target).

### Moonlight illuminance (Krisciunas & Schaefer 1991)

Eq. (9), read from the paper: $m=-12.73+0.026|\alpha|+4\times10^{-9}\alpha^4$ with $\alpha$ the phase angle in degrees ("= −12.73 for full Moon"). Eq. (8): $I^*=10^{-0.4(m+16.57)}$ **footcandles** outside the atmosphere. Eq. (20) combines them: $I^*=10^{-0.4(3.84+0.026|\alpha|+4\times10^{-9}\alpha^4)}$ fc. Converting ($1\,\text{fc}=10.763910\,\text{lx}$) and normalising to full moon:

$$E_{moon}(i)=E_{full}\,f(i),\quad f(i)=10^{-0.4\,(0.026|i|+4\times10^{-9}i^4)},\quad E_{full}=10.763910\times10^{-0.4\cdot3.84}=0.31331\ \text{lx}$$

$E_{full}$ is the **top-of-atmosphere** illuminance on a surface normal to the Moon — the right input for Bevy's `DirectionalLight`, which the atmosphere attenuates itself (same convention as `RAW_SUNLIGHT`). $E_{moon}$ is multiplied by nothing else; set `illuminance = 0` and disable shadows when the Moon is below −0.5° (apparent). The $(385000.56/\Delta)^2$ distance factor is not part of the K&S fit and is not applied.

### Moon disc (billboard)

Angular diameter $2\arcsin(1737.4\ \text{km}/\Delta)$ (reference radius 1737.4 km from the SVS page): 0.517° at mean $\Delta=385000.56$ km, 0.540° at Ex. 47.a. Draw as a camera-following quad whose fragment shader intersects the view ray with a unit sphere in a moon-fixed frame ($+X_{moon}$ toward the observer, $+Z_{moon}$ = ecliptic north; libration and the 1.54° axis tilt are neglected — Jensen §3.3 gives libration formulas, not adopted). Shading uses Lommel–Seeliger, which Jensen Eq. 5 uses for moonlight and which keeps the full moon a flat disc, unlike Lambert:

$$L(\mathbf p)=\frac{A(\mathbf p)\,E_{sun}}{\pi}\cdot\frac{2\mu_0}{\mu_0+\mu}\ \ [\text{cd/m}^2],\qquad \mu_0=\hat n\cdot\hat s_{sun}\ (\le0\Rightarrow0),\ \ \mu=\hat n\cdot\hat v$$

$E_{sun}=130000$ lx (`lux::RAW_SUNLIGHT`). At full phase $\mu_0=\mu$ so $L=A\,E_{sun}/\pi$ everywhere. Calibration: the disc-mean luminance at full moon must equal $E_{full}/\Omega_{disc}$ with $\Omega_{disc}=\pi(0.2602°\cdot\pi/180)^2\approx6.47\times10^{-5}$ sr, i.e. $4843$ cd/m² (observationally plausible); this fixes the mean albedo $\bar A=4843\pi/130000=0.117$. Texel albedo: $A(\mathbf p)=\bar A\cdot t_{lin}(\mathbf p)/\overline{t_{lin}}$ ($t_{lin}$ = sRGB→linear luminance of the 512×256 map). Jensen's $C=0.072$ is the mean reflectance in their irradiance integral (Eq. 5), a different normalisation, so it is not used. Earthshine on the unlit side: $L_{es}=0.0002\,A(\mathbf p)\,E_{sun}/\pi\,(1-k)$ — Jensen §3.3/4.3: the Earth is modelled as a second light source whose phase is complementary to the Moon's, strongest at new Moon (Jensen Eq. 1; the paper gives no single number, the plan's 0.0002 is retained; an [INFERENCE] Lambert estimate for a full Earth gives $\approx5.6\times10^{-5}$, within a factor 4).

### Stars

Catalogue records $(\alpha_{J2000},\delta_{J2000},V,B-V)$. Precession J2000 → date (Meeus 21.2/21.4, $t=T$ of date, arcsec→deg):

$$\zeta=2306.2181''t+0.30188''t^2+0.017998''t^3,\ \ z=2306.2181''t+1.09468''t^2+0.018203''t^3,\ \ \theta_p=2004.3109''t-0.42665''t^2-0.041833''t^3$$
$$A=\cos\delta\sin(\alpha+\zeta),\ B=\cos\theta_p\cos\delta\cos(\alpha+\zeta)-\sin\theta_p\sin\delta,\ C=\sin\theta_p\cos\delta\cos(\alpha+\zeta)+\cos\theta_p\sin\delta,\ \alpha'=\operatorname{atan2}(A,B)+z,\ \delta'=\arcsin C$$

Proper motion is ignored. The CPU builds one 3×3 matrix per frame (precession, then $H=\theta-\alpha'$ rotation, then the east/north/up → world mapping above); the shader applies it to the catalogue unit vector.

Illuminance (K&S Eq. 8 applied to a star, converted to lux): $E_\star=10^{-0.4(V+13.99)}$ lx ($13.99=16.57-2.5\log_{10}10.76391$). Check: $V=0\Rightarrow2.535\times10^{-6}$ lx; $V=-26.74$ (Sun) $\Rightarrow1.26\times10^{5}$ lx vs `RAW_SUNLIGHT` 130000. Jensen Eq. 6: $E_s=10^{0.4(-m-19)}$ W/m² (Sun: $1.2\times10^3$ vs their table 1.3×10³); their Eq. 7 subtracts 0.4 mag ("atmospheric absorption") because they apply their own atmosphere. Catalogue $V$ is already outside-the-atmosphere, so that 0.4 mag is not applied. Sprite radiance, pixel angle $p=2\tan(\text{fov}_y/2)/H_{px}$ rad, sprite solid angle $\Omega=(1.5\,p)^2$ sr (energy-normalised soft disc):

$$L_\star=\frac{E_\star}{\Omega}\,(1-c)\ \ [\text{cd/m}^2],\qquad c=\texttt{Weather.cloud\_cover}$$

Colour (Ballesteros Eq. 14, read):

$$T_{eff}=4600\,\Big(\frac{1}{0.92(B-V)+1.7}+\frac{1}{0.92(B-V)+0.62}\Big)\ [\text{K}],\ \text{clamped to }[1667,25000]$$

then Planckian-locus chromaticity (Kang 2002; $x_c$ then $y_c$; confirmed coefficients):

$x_c=-0.2661239\tfrac{10^9}{T^3}-0.2343589\tfrac{10^6}{T^2}+0.8776956\tfrac{10^3}{T}+0.179910$ for $1667\le T\le4000$; $x_c=-3.0258469\tfrac{10^9}{T^3}+2.1070379\tfrac{10^6}{T^2}+0.2226347\tfrac{10^3}{T}+0.240390$ for $4000\le T\le25000$.
$y_c=-1.1063814x_c^3-1.34811020x_c^2+2.18555832x_c-0.20219683$ ($T<2222$); $-0.9549476x_c^3-1.37418593x_c^2+2.09137015x_c-0.16748867$ ($2222\le T<4000$); $3.0817580x_c^3-5.87338670x_c^2+3.75112997x_c-0.37001483$ ($T\ge4000$).

$X=x_c/y_c$, $Y=1$, $Z=(1-x_c-y_c)/y_c$; linear sRGB $=M\,[X,Y,Z]^T$ with the standard sRGB/D65 matrix (rows $[3.2404542,-1.5371385,-0.4985314]$, $[-0.9692660,1.8760108,0.0415560]$, $[0.0556434,-0.2040259,1.0572252]$), negatives clamped to 0, so each star has luminance 1 before the $L_\star$ scale. Stars with $\text{alt}<0$ are discarded (the ground sphere hides them); no explicit daylight fade (exposure hides them: sprite $L$ is $\sim10^1$ cd/m² vs daytime sky $\sim10^4$).

### Sun and moon entities

- Sun: `DirectionalLight { illuminance: 130000 * atten }` with `atten = 1` — the Bevy atmosphere (Hillaire 2020 model: transmittance, multiple-scattering, sky-view, aerial-view LUTs, photometric units, "a sun and a moon" as $N_{light}$ directional lights, sun disc composited separately after the sky-view LUT) does the atmospheric extinction. Orientation from the refracted sun altitude/azimuth. Bevy draws the Earth-size disc for every light without `SunDisk` (`SunDisk::EARTH`, 0.00930842 rad $=0.533°$, `bevy-0.20-render.md` §Sun disk); the moon light carries `SunDisk::OFF`.
- Moon: `DirectionalLight { illuminance: E_moon(i) }` + `SunDisk::OFF`, topocentric apparent direction. At night the atmosphere scatters it automatically (the atmosphere loops over all directional lights).
- Bruneton & Neyret 2008 is not relied on; Bevy's LUT parameterisation follows Hillaire.

### Auto exposure

Incident-light calibration: $N^2/t=E\,S/C$ ⇒ for ISO $S=100$ and flat-receptor $C=250$ (ISO 2720:1974; Wikipedia Light meter): $EV_{100}=\log_2(E\cdot100/250)$ — the plan's form is confirmed. Reflected-light form $EV_{100}=\log_2(L\cdot100/K)$, $K=12.5$ (Canon/Nikon; same article) is the luminance equivalent ($R=\pi K/C=15.7\%$ for a flat receptor, stated there); the crate uses the illuminance form because the inputs (sun/moon lux) are illuminances. Bevy converts with $\text{exposure}=2^{-EV_{100}}/1.2$. Check against Bevy: `Exposure::SUNLIGHT` $=15.0$ vs $\log_2(10^5\cdot0.4)=15.29$.

$$E=130000\max(\sin h_\odot,0)+E_{moon}\max(\sin h_m,0)+0.15\cdot130000\max(\sin h_\odot,0)+0.0005\ \text{lx},\qquad EV^{target}=\log_2(0.4\,E)$$

(moon term uses true moon altitude and 0 when below the horizon). Smoothing in real time: $EV\leftarrow EV+(EV^{target}-EV)(1-e^{-\Delta t_{real}/1.5})$; the first frame sets $EV=EV^{target}$. Cloud dimming (M2): each body's illuminance is multiplied by the *total* cloud factor $0.25+0.75\,\bar T$ ($\bar T$ the cloud shadow map's mean transmittance at the camera; Deviation 18 and `clouds.md` Deviation 24), so the meter does not depend on how the light divides into beam and diffuse. For context, Jensen Fig. 2 lists non-lunar night illumination (zodiacal $1.2\times10^{-7}$, integrated starlight $3.0\times10^{-8}$, airglow $5.1\times10^{-8}$, galactic $9.1\times10^{-9}$, cosmic $9.1\times10^{-10}$ W/m²; sum $\approx2.4\times10^{-7}$ W/m², about $2.4\times10^{-5}$ lx at the Sun's 98 lx per W/m² — [INFERENCE] broadband conversion); the plan's 0.0005 lx floor is a deliberate exposure clamp, not a physical claim, and is kept.

**Corpus corroboration and limits (Patry 2021; RTR4 §8.1).**

1. *Illuminance metering is the production choice.* Patry's team moved from luminance-based exposure, whose feedback with albedo made authored albedos too dark, to illuminance-based exposure with luminance used only for highlights above a threshold. This note's $EV_{100} = \log_2(0.4E)$ meters illuminance only, so it matches; the corollary for M2 is that the lightning flash (an emissive bolt of 50 000 cd/m² plus a point light) must not enter $E$: no term in the formula above sees it, and it must stay that way.
2. *Where moonlight sits photometrically.* RTR4 puts the photopic/scotopic boundaries at 3.4 and 0.034 cd/m². A 0.18 grey card under $E_{full} = 0.31331$ lx (zenith, top of atmosphere) has $L = 0.18\,E/\pi = 0.01795$ cd/m², below the scotopic limit; at the Meeus Ex. 48.a phase ($E = 0.05510$ lx) 0.00316 cd/m²; at the 0.0005 lx floor $2.9\times10^{-5}$ cd/m²; under the sun at 68.4° (no atmosphere) $6.9\times10^{3}$ cd/m². Moonlit scenes are therefore rod-dominated, which is the regime of Patry's Purkinje shift, where "an illuminance averaging about 0.05 lux" is the demonstration level. That level is about 10 % below this note's moonlight at Ex. 48.a (0.0551 lx) and equals Bevy's `lux::FULL_MOON_NIGHT` (0.05 lx, `bevy-0.20-render.md`), which corresponds here to a phase angle of ≈ 72° ($k \approx 0.65$), not to a full moon (0.313 lx).
3. *Purkinje shift: not adopted, constants recorded.* The plan has no tonemapping stage of its own and Bevy 0.20 has no low-light colour shift; adding one is outside the plan. For an opt-in later, Patry (after Cao 2008 and Kirk 2011) converts scene RGB to cone/rod responses $\mathbf q$ (LMSR, matrix from Smits reflectance spectra under D65), computes the gain $g_i = [1 + 0.33\,(q_i + k_i\,q_R)/m_i]^{-1/2}$ with $\mathbf k = (0.2, 0.2, 0.29)$ and $\mathbf m = (0.63721, 0.39242, 1.6064)$, and adds the rod-induced opponent-space shift scaled by $K/S$ with $K = 45$, $S = 10$, $k_3 = 0.6$, $r_w = 0.139$, $p = 0.6189$; the shader is three lines (`lmsGain = rsqrt(1 + lmsr.xyz); rgb += M_gain * lmsGain * lmsr.w`). The rod curves, the RGB→LMSR matrix and the full $\Delta\mathbf o$ matrix are not reliably recoverable from the corpus text, so this is a pointer, not an implementable spec.
4. *Moon in the atmosphere.* Patry resamples the sky LUTs for the current sun and moon angles every frame, the same arrangement as Bevy's atmosphere looping over all directional lights (Sun and moon entities above); no change.

## Parameters

| name | symbol | default | unit | source |
|---|---|---|---|---|
| `GeoLocation.latitude_deg` | $\varphi$ | 45 | ° | plan §1.1 |
| `GeoLocation.longitude_deg` (east +) | $\lambda$ | 0 | ° | plan §1.1 |
| `GeoLocation.utc_offset_hours` | — | 0 | h | plan §1.1 |
| `WorldClock.time_scale` | — | 60 | game s per real s | plan §1.1 |
| ΔT model | $\Delta T$ | Espenak–Meeus polynomials (table above) | s | NASA eclipse site |
| Refraction pressure / temperature | $P$, $T_c$ | 1010 / 10 | mbar / °C | Meeus ch. 16, SPA Eq. 42 |
| Refraction cut-off | — | −0.8333 | ° | SPA convention (see note) |
| Sun illuminance (TOA) | $E_{sun}$ | 130000 | lx | `lux::RAW_SUNLIGHT`, `bevy-0.20-render.md` |
| Sun disc size (Bevy-fixed) | — | 0.00930842 | rad | `SunDisk::EARTH` |
| Moon full-moon TOA illuminance | $E_{full}$ | 0.31331 | lx | K&S Eq. 8/20, ×10.76391 lx/fc |
| Moon phase slope | — | 0.026 | mag/° | K&S Eq. 9 |
| Moon phase quartic | — | 4e-9 | mag/°⁴ | K&S Eq. 9 |
| Moon light cut-off altitude | — | −0.5 | ° | design choice (no source) |
| Moon series thresholds (A: l, r; B: b) | — | 500, 2000; 500 | 10⁻⁶°, 10⁻³ km | prototype, this note |
| Moon radius / Earth equatorial radius | — | 1737.4 / 6378.137 | km | SVS page / Meeus ch. 40 |
| Moon mean albedo (calibrated) | $\bar A$ | 0.117 | — | derived: $E_{full}/\Omega$ with 130000 lx |
| Moon disc BRDF | — | Lommel–Seeliger | — | Jensen Eq. 5 |
| Earthshine factor | — | 0.0002 | × sun radiance, × (1−k) | plan (retained); Jensen §3.3 qualitative |
| Moon texture | — | 512×256 equirect, centred 0° lon | px | SVS 4720 |
| Star limit | $V_{max}$ | 6.5 | mag | plan; BSC definition |
| Star illuminance zero point | $Z_0$ | 13.99 | mag | K&S Eq. 8 (plan had 14.18) |
| Star sprite size | — | 1.5 | px | plan |
| Star B−V default (missing, 244 of 8404) | — | 0.48 | mag | median of the 8160 catalogue values |
| Star temperature clamp | $T_{eff}$ | 1667–25000 | K | Kang 2002 validity range |
| Ballesteros constants | — | 4600, 0.92, 1.7, 0.62 | K, —, —, — | EPL 97, 34008, Eq. 14 |
| Exposure ISO / calibration | $S$, $C$ | 100 / 250 | — | ISO 2720 via Light meter article |
| Ambient fraction of sun | — | 0.15 | — | plan |
| Illuminance floor | — | 0.0005 | lx | plan |
| Exposure smoothing time constant | $\tau$ | 1.5 | s (real) | plan |
| Photopic / scotopic luminance limits | — | 3.4 / 0.034 | cd/m² | RTR4 §8.1 photometry footnote |
| Night-scene reference illuminance (not used in code) | — | 0.05 | lx | Patry 2021 Purkinje demo; equals Bevy `lux::FULL_MOON_NIGHT`; this note's moonlight at Ex. 48.a is 0.0551 lx |

## GPU mapping

- `astro.rs`, `WorldClock`, `SunState`, exposure estimate: **CPU only**.
- Stars (`stars.wgsl`): vertex-less instanced draw, `draw(0..6, 0..8404)`, drawn after the sky/opaque and transparent passes (additive blend, depth test against the main depth so only far-plane pixels are hit; ordering per `bevy-0.20-render.md` §Post-process insertion — the atmosphere's `render_sky` cannot be named, so order `.after(main_transparent_pass_3d)` within `Core3dSystems::MainPass`). Resources: 1 read-only storage buffer `array<vec4<f32>>` of 8404 × 16 B = 134 464 B (embedded via `include_bytes!` as `Handle<ShaderBuffer>`, `RENDER_WORLD`), read in the vertex stage; 1 uniform (mat3 as 3×vec4, `pixel_angle`, `star_scale`, `cloud_cover`). That is 1 of the 8 `max_storage_buffers_per_shader_stage` (default 8 per `bevy-0.20-render.md`; wgpu `downlevel_webgl2_defaults` = 0 — WebGL2 is unsupported by the plan anyway). No storage textures, no compute, no workgroups.
- Moon disc (`moon.wgsl`): `MoonMaterial` fragment shader, 1 sampled 2D texture (512×256 `rgba8unorm`, filtered, 128 KiB) + 1 uniform (moon-fixed frame, sun direction, $\bar A$ scale, phase/earthshine factor); no storage resources.
- Sun/moon lights: Bevy's own shadow and atmosphere passes; the moon adds one more light to the atmosphere loops (cost scales with light count, `bevy-0.20-render.md` §Atmosphere); at most 2 shadowed directional lights, well inside the layer limit.
- Stars/moon are camera-relative (follow `ElementsCamera`); with no `ElementsCamera` they idle, as in the plan's architecture.

## Validation targets

Sources marked **book** = Meeus's published example, confirmed through the `astronomia` test suite and the prototype; **NREL** = TP-560-34302 Table A5.1; **K&S**, **Jensen**, **Ballesteros** = papers read; **proto** = regression value from the throw-away prototype implementing exactly the formulas above (Meeus-accuracy; no independent ephemeris cross-check) and the stated tolerance.

Time
1. $JD$(1957-10-04.81) $=2436116.31$ (Meeus Ex. 7.a). $JD$(2000-01-01 12h) $=2451545.0$; (1987-01-27 0h) $=2446822.5$; (1900-01-01 0h) $=2415020.5$ (Meeus ch. 7). A Julian-calendar example (333 Jan 27.5 $\to$ 1842713.0, Ex. 7.b) is out of scope — the crate is Gregorian-only; the astronomia suite records 1842712 for the proleptic-Gregorian input, not asserted.
2. SPA instant: 2003-10-17 12:30:30 at UTC−7 $\Rightarrow$ UT 19:30:30 $\Rightarrow JD_{UT}=2452930.312847$ (NREL Table A5.1), exact to 1e-6 d.
3. Mean sidereal time at 1987-04-10 0h UT ($JD=2446895.5$) $=13^h10^m46.3668^s=197.693195°$ (Meeus Ex. 12.a; prototype 197.693195090°) ±1e-5°. Apparent $=13^h10^m46.1351^s$ (Ex. 12.a): with the 4-term nutation the equation of the equinoxes is $-0.2363^s$ vs $-0.2317^s$ — assert ±0.0005°.
4. Mean sidereal at 1987-04-10 19:21:00 UT $=8^h34^m57.0896^s=128.737873°$ (Meeus Ex. 12.b; prototype 128.737873°) ±1e-5°.
5. ΔT: $\Delta T(2000.0)=63.86$ s, $\Delta T(2050.0)=93.0$ s (Espenak's own 2050 anchor 93 s; polynomial gives 93.001), $\Delta T(2010.0)=66.70$ s (their stated 2010 value is 66.9 s — polynomial, ±0.3 s), tolerance ±0.01 s. (SPA example uses 67 s as an input.)

Sun (Meeus Ex. 25.a, 1992-10-13 0h TD, $JDE=2448908.5$)
6. $T=-0.072183436$; $L_0=201.80720°$; $M=-2241.00603°$ ($\equiv278.99397°$); $e=0.016711668$; $C=-1.89732°$; $\odot=199.90987°$ (Meeus 199.90988°); $R=0.99766$ AU; $\lambda_{app}=199°54'32''=199.90889°$ (prototype 199.90894°); $\varepsilon=23.43999°$. Tolerance ±0.0005° / ±1e-5 AU. (astronomia tests: `solar.test.js` lines asserting 199.90987, −2241.00603, 0.016711668, 0.99766.)
7. **Apparent RA/Dec — plan values corrected.** Ex. 25.a (low accuracy, what the crate implements): $\alpha=13^h13^m31.4^s=198.38083°$, $\delta=-7°47'06''=-7.78507°$ (prototype 198.380825°, −7.785070°). The plan's $\delta=-7.78361°$ ($-7°47'01''$) is the *high-accuracy VSOP87* result of Ex. 25.b ($\alpha=13^h13^m30.749^s=198.378121°$, $\delta=-7°47'01.741''=-7.783817°$); the plan's RA is the 25.a value, so it mixed the two examples. Assert $\alpha=198.38083°$ ±0.005°, $\delta=-7.78507°$ ±0.005° (also passes the plan's ±0.01° against 25.b: $|\Delta\alpha|=0.0027°$, $|\Delta\delta|=0.0013°$).

Moon (Meeus Ex. 47.a, 1992-04-12 0h TD, $JDE=2448724.5$)
8. Full series (60+60 terms): $\lambda_{geo}=133.162655°$, $\beta=-3.229126°$, $\Delta=368409.7$ km, $\pi_m=0.991990°$ — reproduced digit for digit by the prototype (133.162655, −3.229126, 368409.68). **Plan values confirmed.**
9. Truncated series (51+34 terms, shipped): $\lambda_{geo}=133.16256°$, $\beta=-3.22881°$, $\Delta=368410.4$ km (proto). Assert vs the book values within ±0.01°, ±0.01°, ±10 km (plan); margin is 0.0001° / 0.0004° / 0.7 km.
10. Apparent: $\Delta\psi=+0.004610°$ gives $\lambda_{app}=133.167265°$, $\alpha=134.688470°$, $\delta=+13.768368°$ (Meeus Ex. 47.a); proto with truncated nutation: 133.16723°, 134.68844°, 13.76838°. Assert ±0.005°.
11. Truncation envelope over 1900–2100 (3000 random dates, proto): $\le0.0023°$ in $\lambda$, $\le0.0041°$ in $\beta$, $\le2.9$ km in $\Delta$; a test samples 100 dates against the in-test full series and asserts $\le0.01°$, $\le0.01°$, $\le10$ km.

Phase (Meeus Ex. 48.a, same date; inputs $\alpha_0=20.6579°$, $\delta_0=8.6964°$, $R=149971520$ km, $\alpha=134.6885°$, $\delta=13.7684°$, $\Delta=368410$ km)
12. $i=69.0756°$, $k=0.6786$ (Meeus Ex. 48.a via eq. 48.2; astronomia `phaseAngleEquatorial` asserts 69.0756). Prototype from its own Sun/Moon: $\psi=110.792°$, $i=69.076°$, $k=0.67856$. Assert $i=69.0756°$ ±0.01°, $k=0.6786$ ±0.0005. Bright-limb position angle $\chi=285.0°$ (Meeus Ex. 48.a; proto 285.04°) ±0.1°. (For information: the low-accuracy path gives $i=68.88°$, $k=0.6801$.)

SPA example (NREL Table A5.1: 2003-10-17 12:30:30 LST, TZ −7 h, lon −105.1786°, lat 39.742476°, elevation 1830.14 m, $P=820$ mbar, $T=11$ °C, $\Delta T=67$ s)
13. SPA outputs: $\theta=50.11162°$ (topocentric zenith, refraction included), $\Phi=194.34024°$ (azimuth, N→E), topocentric $\alpha'=202.22704°$, $\delta'=-9.316179°$, $H'=11.10629°$, equation of time $14.641503$ min. **Plan values confirmed.** Atmospheric refraction in SPA uses $P=820$, $T=11$ (Eq. 42), so the test must pass those to `refraction_deg`.
14. Prototype (Meeus ch. 25 sun, ΔT = 67 s, parallax $\xi\cos h$, Saemundsson with the SPA $P/T$): zenith $50.1125°$ (error 0.0009°), azimuth $194.3345°$ (error 0.0057°; RA of the low-accuracy Sun is 0.004° off SPA's $\alpha=202.22741°$: proto 202.2317°). With the default 1010 mbar/10 °C: zenith $50.1087°$ (0.0029°); with no refraction: $50.1289°$ (0.0171°). A Meeus-accuracy implementation can honestly claim ≈0.01°; the plan's ±0.05° is safe. Tests assert ±0.02° on both angles (tightening of the plan's bound, 3× margin to the observed worst error) and also ±0.05° as the plan states, with the SPA refraction parameters and `time = 67 s` ΔT override.

Refraction
15. Saemundsson at defaults ($P=1010$, $T_c=10$): $R(0°)=0.48303°$ (29.0′), $R(10°)=0.09013°$, $R(45°)=0.016878°$, $R(-0.8333°)=0.6182°$ (proto formula evaluation), ±1e-5°. With $P=820$, $T_c=11$ the same is multiplied by $0.81188\times283/284$.

Scene at lat 45°, lon 0°, UTC offset 0, 2026-06-21, ΔT from the polynomial (proto regression, tolerance ±0.1° on altitudes/azimuths, ±0.01 on $k$; Meeus-level, not cross-checked against an independent ephemeris)
16. 12:00: sun altitude $68.43°$, azimuth $178.86°$ (solstice noon altitude $90-45+23.44=68.44°$). 19:30: sun $2.18°$, azimuth $301.66°$. 23:00: sun $-20.12°$; moon alt $8.50°$, az $257.4°$, $k=0.506$, phase angle $89.3°$, $\Delta=388664$ km (**the moon is up at 23:00; the plan's visual check needs no `T` key advance**). 06:00: sun alt $16.04°$, moon $-42.94°$, $k=0.432$.
17. **Plan visual check corrected:** at lat 45° on the solstice the sun rises at ≈04:11 and sets at ≈19:49 local solar time ($\cos H_0=(\sin(-0.833°)-\sin45°\sin23.44°)/(\cos45°\cos23.44°)=-0.456$, $H_0=117.1°$). 06:00 is a bright morning (sun 16°), not a "horizon glow". Use 04:15 for the dawn shot.

Moon light
18. $E_{full}=0.31331$ lx (K&S: $10^{-0.4\cdot3.84}=0.029107$ fc $\times10.76391$). **Plan's 0.32 lx is the rounded version of this.** $f(i)$: $f(0)=1$; $f(30°)=0.48608$; $f(60°)=0.22660$; $f(69.0756°)=0.17587$ (0.05510 lx); $f(90°)=0.090996$ (0.028510 lx); $f(120°)=0.026316$; $f(150°)=0.0042658$; $f(180°)=2.8079\times10^{-4}$ — tolerance 1e-6 relative.
19. Disc: angular diameter $0.5404°$ at $\Delta=368409.7$ km, $0.5171°$ at $385000.56$ km (±1e-4°). Full-moon disc-mean luminance $=E_{full}/\Omega=4843$ cd/m² ($\Omega=6.469\times10^{-5}$ sr) ±1; $\bar A=0.1170$. Lit limb: for a rendered unit-test of `MoonMaterial`'s CPU mirror, the brightest texel's direction projected on the disc points toward the Sun's projected direction within 1° at $k=0.5$.

Stars
20. Catalogue: CDS V/50 `catalog.gz` has 9110 records (ReadMe); records with a J2000 position and $V\le6.5$: **8404**; of those 244 lack $B-V$ (default 0.48); brightest $V=-1.46$. `stars.bin` size $=8404\times16=134464$ bytes. (Counts from the 2026-10-13 download.)
21. Sirius (HR 2491, `9Alp CMa`): J2000 RA $06^h45^m08.9^s=101.28708°=1.7677931$ rad, Dec $-16°42'58''=-16.71611°=-0.2917512$ rad, $V=-1.46$, $B-V=0.00$ (CDS catalogue line). Record in `stars.bin` equals those to f32 rounding (1e-6 rad, 0.005 mag).
22. $E_\star=10^{-0.4(V+13.99)}$ lx: $V=0\to2.5351\times10^{-6}$; $V=-1.46\to9.7275\times10^{-6}$; $V=1\to1.0093\times10^{-6}$; $V=6.5\to6.368\times10^{-9}$ (rel. tol. 1e-4). Sun cross-check $V=-26.74\to1.259\times10^{5}$ lx (±0.5 % of 1.26e5).
23. Ballesteros Eq. 14: $T(B-V=0.65)=5778.4$ K (the Sun's 5772 K); $T(0)=10125.2$ K; $T(1.0)=4742.7$ K; $T(1.5)=3793.5$ K; $T(-0.33)=17832.7$ K (±0.1 K).
24. Kang locus: $T=5778$ K → $(x_c,y_c)=(0.32634,0.33573)$; $T=10125$ K → $(0.28002,0.28750)$; $T=6504$ K → $(0.31343,0.32360)$, which equals the published Planckian chromaticity of 6500 K (0.3135, 0.3237) (±1e-4). Normalised linear sRGB (Y=1 before clamp): $T=5778$ K → $(1.1109,0.9757,0.9142)$; $10125$ K → $(0.8690,0.9945,1.4406)$; $3794$ K → $(1.4711,0.9123,0.4810)$ (±1e-3).

Exposure
25. $EV_{100}=\log_2(0.4E)$: sun at zenith ($E=130000+19500+0.0005$ lx) $=15.8679$; full moon at zenith, sun below horizon ($E=0.31331+0.0005$) $=-2.9940$; moonless night (floor only) $=-12.2877$; sun altitude 68° with $E_\odot=130000\sin68°$ plus 15 % ambient $=15.7588$ (±1e-3). Smoothing: after $3\tau=4.5$ s from a step the residual is $e^{-3}=4.98\%$ of the step (±0.1 %).
26. Photometric regime of moonlight (arithmetic, CPU test of a helper or a doc test): grey card $\rho = 0.18$, $L = \rho E/\pi$: full moon at zenith ($E = 0.31331$ lx) $0.017951$ cd/m² (< 0.034, scotopic); Ex. 48.a ($E = 0.05510$ lx) $0.003157$ cd/m²; floor ($0.0005$ lx) $2.865\times10^{-5}$ cd/m²; sun at 68.43° ($E = 130000\sin 68.43°$ lx) $6926.8$ cd/m² (> 3.4, photopic); tolerance 1e-3 relative. Source: RTR4 §8.1 footnote (limits), K&S (illuminances); arithmetic here.
27. (M2, forward-looking) A `LightningStrike` flash (point light and emissive bolt) leaves the metered $E$ and `target EV` unchanged: the exposure estimate is a function of `SunState` and cloud-dimming only. Source: Patry 2021, "Tonemapping | Exposure" (luminance used only above a highlight threshold; this crate uses none).

## Deviations from this plan

Each is backed by the cited source; none changes a public API shape.

1. **Moon full-moon illuminance 0.32 lx → 0.31331 lx.** K&S Eqs. 8/9/20 give $10^{-0.4\cdot3.84}$ fc $=0.029107$ fc $=0.31331$ lx; 0.32 was a rounded recollection. Also recorded: it is the top-of-atmosphere value.
2. **Star illuminance zero point 14.18 → 13.99.** $E=10^{-0.4(V+14.18)}$ gives 2.13e-6 lx at $V=0$ and 1.06e5 lx for the Sun; K&S Eq. 8 converted to lux gives 13.99, which reproduces the standard $V=0\to2.54\times10^{-6}$ lx and $V=-26.74\to1.26\times10^{5}$ lx (and is consistent with the moon constant in deviation 1).
3. **Moon disc shading Lambertian → Lommel–Seeliger** (Jensen Eq. 5 / §4.3), with the mean albedo calibrated to the K&S full-moon illuminance (0.117). A Lambertian disc would show a dark limb at full phase.
4. **Moon angular diameter fixed 0.52° → $2\arcsin(1737.4/\Delta)$** (0.489°–0.559° over the orbit; 1737.4 km is the SVS reference radius).
5. **Validation value: Sun declination −7.78361° → −7.78507°** (Meeus Ex. 25.a). −7.78361° is the Ex. 25.b/VSOP87 figure; see Validation 7. The plan's RA 198.38083° is correct for 25.a.
6. **Visual check: 06:00 is not a horizon glow** at lat 45° on 2026-06-21 (Validation 17); use 04:15. At 23:00 the moon is already up (8.5°).
7. **Stars are precessed from J2000 to the date** (Meeus ch. 21, ≈0.36° by 2026) in addition to the plan's LST/latitude rotation.
8. **Star catalogue is read from the CDS V/50 `catalog.gz`** (fixed-width, byte layout in `ReadMe`: J2000 RA bytes 76–83 `h m s`, Dec 84–90 `± d m s`, Vmag 103–107, B−V 110–114; ADC/CDS is the file HEASARC's `bsc5p` was built from) rather than a HEASARC table export, because it is a single stable HTTP file with a documented layout (HEASARC TAP rejected `FORMAT=csv`). `tools/gen_stars.rs` records both URLs in `PROVENANCE.md`.
9. **`SunState.altitude_deg` is the true altitude;** refraction applies only to the light direction. **`moon_phase` is the synodic fraction** $((\lambda_{moon}-\lambda_{sun})/360°)\bmod 1$ (0 new, 0.25 first quarter, 0.5 full, 0.75 last quarter): the plan's "0 new..1 full" would duplicate `moon_illuminated_fraction` ($k$) and lose waxing vs waning.
10. **Tolerance:** SPA example asserted at ±0.02° (plan ±0.05°) — tighter, observed worst error 0.0057°.
11. **Exposure matches visibility thresholds instead of fully adapting to the scene illuminance.** The plan's $EV_{100} = \log_2(E\cdot100/250)$ exposes every scene to the same display brightness, so a moonlit scene shows as daylight: the first GPU renders at 23:00 (half moon at 8.5°) had a daylight-blue sky, saturated stars and a blown-out moon, because Bevy's atmosphere scatters moonlight exactly like sunlight and the exposure rose 25 EV to meet it. Jensen et al. 2001 §6 (read from the PDF) say night scenes are scotopic and need perceptual tone reproduction; they used the operator of Larson, Rushmeier & Piatko 1997 ("A visibility matching tone reproduction operator for high dynamic range scenes", IEEE TVCG 3(4); read from `https://radsite.lbl.gov/radiance/papers/lbnl39882/tonemap.pdf`), whose Eq. 7 applies Ward's technique (Ward 1994, "A contrast-based scalefactor for luminance display", *Graphics Gems IV* 415–421): scale world luminance by $\Delta L_t(L_{d,a})/\Delta L_t(L_{w,a})$ so a just-visible world contrast is just visible on screen. Adopted as one global scale with their Table 1 threshold function (rods and cones, after Ferwerda et al. 1996): $L_{w,a} = 0.18\,E/\pi$, $L_{d,a} = L_{d,max}/2 = 50$ cd/m², $L_{d,max} = 100$ cd/m², $EV_{100} = -\log_2(1.2\,m/L_{d,max})$ (Bevy multiplies by $2^{-EV}/1.2$ and treats 1 as display peak). Values: sun at zenith 13.65 (Bevy's own atmosphere example uses 13.0), full moon at zenith −2.15, half moon at 8.5° −4.69 (was −9.05: 4.4 EV darker on screen), moonless floor −4.75 (was −12.29); the day–night span is 18 EV instead of 28. The metered quantity is still illuminance (Patry 2021), so Validation 27 (lightning) is unchanged. Validation 25's numbers are replaced by these (`exposure_targets_match_the_research_note`).
12. **Stars and moon are drawn at depth 0 (the sky), not at 0.999 × far** (sky implementation): the vertex shaders write clip z = 0, which under Bevy's infinite reverse-Z is the far sky, so geometry always occludes them, no camera-follow system is needed and they do not depend on `PerspectiveProjection::far`.
13. **Moon mean albedo 0.117036, not 0.117** (= 4843π/130000 exactly): at 0.117 the full-moon disc mean is 4841.5 cd/m², outside Validation 19's ±1. `gen_moon` normalises the disc-area-weighted mean over the sub-observer hemisphere to 128 (1.1 % of texels clip at 255).
14. **B−V is clamped to [−0.4, 4.0] before Ballesteros eq. 14**, which has a pole at B−V = −0.674; the catalogue spans [−0.28, 3.86], so no real star changes.
15. **Metered illuminance includes twilight: USNO total horizontal illuminance replaces "direct × sin h + 15 % sky".** The plan's estimate is zero as soon as the sun's centre is below the horizon, so at 04:15 on the solstice (sun at about 0°) the exposure dropped to the night floor while the dawn sky was already bright, and the frame rendered pure white. Adopted: $E = (E_{\odot}\,F(h_\odot') + E_{moon}\,F(h_m'))\cdot T_{cloud} + 0.0005$ lx, with $h'$ the refracted altitude and $F$ subroutine `ATMOS` of Janiczek & DeYoung 1987, *Computer programs for sun and moon illuminance with contingent tables and diagrams*, US Naval Observatory Circular 171 (read from the Internet Archive scan `DTIC_ADA182110`, BASIC listing line 2240 and Fortran `SUBROUTINE ATMOS`; also transcribed in the `bluegreen-labs/skylight` package): $U = \sin h'$, $S = \arcsin(X\cos h'/(X+1))$, $M = X(\cos S - U) + \cos S$, $F = e^{-0.21M}U + 0.0289\,e^{-0.042M}\big(1 + (h'+90)\,U/57.29578\big)$, $X = 753.66156$. It is the direct plus sky illuminance relative to the body outside the atmosphere: 0.925 at the zenith, 0.00566 on the horizon (USNO 757 lx), 2.24e-5 at −6° (2.99 lx: civil twilight), 3.3e-8 at −12°. The same $F$ attenuates moonlight. New exposure values (Deviation 11's model): sun at zenith 13.34, sunrise (true altitude 0°) 6.48, civil twilight 0.86, full moon at zenith −2.24, floor −4.75.
16. **The camera's ambient light starts at zero.** Bevy's default `GlobalAmbientLight` is a constant 80 cd/m², negligible by day and lifted to white by the night exposure (the first 23:00 render had a white ground under a starry sky). The `ElementsCamera` dressing inserts a per-camera `AmbientLight { brightness: 0 }` where none is set (it overrides the global for that camera only, `bevy_light-0.20.0/src/ambient_light.rs`); sky light comes from `AtmosphereEnvironmentMapLight`, which follows the sun and moon. Bevy's own atmosphere example sets `GlobalAmbientLight::NONE` for the same reason. Since M2 the celestial systems drive this component with the cloud-scattered diffuse light (Deviation 18): it is exactly 0 whenever the sky is clear and whenever neither the sun's nor the moon's light is on, so the reason for the zero stands.
17. **The moon disc is displayed at its own adaptation (local adaptation).** With one scene exposure (Deviation 11, about 21 at night) the lit disc at ~4843 cd/m² lands ~10⁵ past display white and the earthshine side (~0.4 cd/m²) at ~10: both saturate, the sunlit half as featureless white and the earthshine half as the brighter-looking, textured one (seen in a 1.5° view at 23:00). Larson, Rushmeier & Piatko 1997 handle bright fixated regions with local (foveal) adaptation; adopted for the disc only: its radiance is shown at `min(scene exposure, exposure for an eye adapted to the lit lunar surface)`, the latter `exposure_for_adaptation(4843 cd/m²)` = 1.15×10⁻⁴ (lit surface ≈ 0.56 of display peak, maria visible; earthshine ≈ 3×10⁻⁵, black). By day the scene exposure is already the smaller, so the daytime moon is unchanged. The radiance model and Validation 19 are unchanged; only the display scale.
18. **Cloud splits the light into a beam and a diffuse remainder; the sun's and moon's `DirectionalLight` carry the beam, the camera's `AmbientLight` the remainder, and the exposure still meters the total** (M2). Under cloud the direct beam goes and what is left is light the cloud scatters. Kasten & Czeplak (1980, *Solar Energy* 24: 177), through the excerpts of Perez et al. (UCSD ENG 255 forecasting notes, `http://newmaeweb.ucsd.edu/groups/kleissl/255/PerezetalForecastSolarRadWeather.pdf`, eq. 2) and of the reviews found by search (the original is not in the corpus): `GHI / GHI_clear = 1 − 0.75 (N/8)^3.4` for `N` in oktas, independent of solar elevation (ten years of hourly data, Hamburg 1964–1973); at `N = 1` the global irradiance is a quarter of the clear value. The same paper's diffuse fraction (`D/G = 0.3 + 0.7 N²`, quoted in the task) is not in any text this note read and is not used. The model needs only the overcast limit, where the beam is zero (the model stratus has a column optical depth of 6.04, `clouds.md` Validation 7: $e^{-6}$ = 0.0025) and all of the 0.25 is diffuse. A point on the ground is either lit by the beam or shut off from it, so over the camera's neighbourhood the beam fraction is the measured transmittance $\bar T$ and the rest of the ground has the overcast diffuse level:

    $$ \text{direct} = \bar T,\qquad \text{diffuse} = 0.25\,(1 - \bar T),\qquad \text{total} = 0.25 + 0.75\,\bar T = 1 - 0.75\,(1-\bar T), $$

    which is Kasten & Czeplak's global factor when $\bar T = 1 - N^{3.4}$ (the value before the first readback). `celestial::cloud_light` is the one place the split is computed; `CloudLight::direct`, `diffuse` and `total` are what `update_sky`, `update_ambient`, `scene_illuminance`, `auto_exposure`, the distance fog, the precipitation lighting and the cloud pass read. (a) *Lights:* sun and moon `DirectionalLight::illuminance` $= E\,\bar T$, so their shadows fade out with the beam. Bevy's atmosphere (sky, aerial view and the `AtmosphereEnvironmentMapLight` sky light) is lit by these lights' `color · illuminance` (`bevy_pbr-0.20.0/src/atmosphere/functions.wesl:200-220`, `render/light.rs:1817-1819`), so it follows $\bar T$ too. (b) *Ambient:* Bevy's per-camera `AmbientLight` is a uniform radiance $L$ in cd/m² (`bevy_light-0.20.0/src/ambient_light.rs:12-26`; `bevy_pbr` multiplies `colour · brightness` by the diffuse term, `render/light.rs:1846-1848`, `render/pbr_ambient.wesl:26`), a surface of albedo $\rho$ shows $\rho L$, a horizontal illuminance of $\pi L$. So `brightness` $= \sum_{b\in\{\odot,\mathrm{moon}\}} E_b\,F(h_b')\cdot 0.25\,(1-\bar T)/\pi$ (the metered clear-sky horizontal illuminance of Deviation 15 for each body that is up, i.e. whose light is on) and `color` is the two bodies' tints weighted by those shares (the sun's `dusk_tint`, white above 10° and orange at the horizon, and the moon's light colour), normalised to luminance 1 so the colour does not change the illuminance. The total horizontal illuminance is therefore $E F(h')\,[\bar T + 0.25\,(1-\bar T)]$ = the previous metered value, to the sky light Bevy adds on top of both: exposure is unchanged by the split. Numbers, sun at zenith (E = 130000 · 0.925353 = 120 296 lx clear): $\bar T$ = 1 → sun 130 000 lx, ambient 0; 0.5 → 65 000 lx, 4 787 cd/m² ($\pi L$ = 15 037 lx = 0.125 E); 0 → 0 lx, 9 573 cd/m² ($\pi L$ = 30 074 lx = 0.25 E). (c) *Night:* the ambient takes only bodies whose `DirectionalLight` is on (sun above −2°, moon above −0.5°), so with neither it is 0 and the night is not lifted (Deviation 16); a moonlit overcast night gets the moon's share. Twilight below −2° stays dark under cloud as it does under clear sky. Alternative considered: `AtmosphereEnvironmentMapLight::intensity` scales the sky light on surfaces only (`bevy_pbr-0.20.0/src/atmosphere/environment.rs:261`), a uniform sky colour from the atmosphere with no relation to the cloud, and leaves the sky, aerial view and fog volumes unchanged; it cannot add light that the atmosphere does not have at $\bar T = 0$, so `AmbientLight` was chosen. [INFERENCE] The tint is a stand-in for "the light that reaches the layer": the sun's transmittance at the layer is a GPU lookup (`clouds.md`), the CPU has only the altitude.
19. **Twilight, the night background and the glow-capped exposure (amends 11, 15 and 18, including 18 (c)'s "twilight below −2° stays dark"; iteration 12).** The sun's light is no longer switched off at −2°: it lights the atmosphere to −18°, scaled below the horizon to follow Hulburt's measured zenith twilight (1953), and once its disc has set its cloud factor is the dome's total, not the beam, so `sky_gain` is 1 then; under cloud the ambient now carries the twilight. A camera that kept Bevy's default clear colour is cleared to the night sky's background radiance at its exposure. The auto exposure is the larger of the illuminance target (unchanged, Validation 25) and a cap from the sky toward the sun (Patry 2021). Sources, measurements and parameters: `docs/research/twilight.md`.

## Pending index

`distill_index` skipped (distill/Qdrant down, MCP bridge 404; `hs restart` not attempted by this task). Stems to index when distill is healthy:
- `10.1016_j.solener.2003.12.003`
- `10.1145_383259.383306`
- `10.1111_cgf.14050`
- `10.1111_j.1467-8659.2008.01245.x`
- `10.1086_132921`
- `10.1209_0295-5075_97_34008` (Ballesteros 2012; read from https://www.uv.es/ferbaro/papers/new.pdf, not downloaded into the paper store)

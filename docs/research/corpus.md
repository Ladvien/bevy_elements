# The home-still corpus — what it already holds for this crate

The home-still library has 12 178 converted documents. New downloads are mostly blocked (paywalls, scribe region failures on bmb, distill and Qdrant down), but a lot of what this crate needs is already there, much of it under non-DOI stems: the SIGGRAPH *Advances in Real-Time Rendering* course material is filed as `s<year>-advances-*`, books as `book_*`, SIGGRAPH 2024/2026 papers as `sig20xx_*`, ETH CGL papers as `eth-cgl-*`. Catalog titles are missing for about half the rows, so search the text, not the catalog.

## How to search it

1. Sync once: `aws --endpoint-url "$EP" s3 sync s3://home-still/markdown/ ~/.cache/bevy_elements_corpus/markdown/` (credentials and endpoint as in skill `ingest-dois-via-anna-scidb`; 828 MB).
2. Search by method phrase, not topic word: `grep` over `~/.cache/bevy_elements_corpus/markdown` for e.g. `virtual pipe`, `Henyey`, `froxel`, `talus`, `flow map`. Topic words (`rain`, `gust`, `snow`) drown in the non-graphics part of the library.
3. Read the hit as a file; cite it in the note's `## Sources` by stem.

The scan behind this list (2026-10-13) scored every document against per-domain phrase sets weighted toward method terms; the lists below are the hits a human check confirmed as relevant.

## Reading list by domain

Stems are file names under `markdown/<shard>/`; ★ marks a plan seed that is present.

### Sky, atmosphere, celestial (M1)

| stem | what it is | use |
|---|---|---|
| `10.1111_cgf.14050` ★ | Hillaire 2020, *A scalable and production ready sky and atmosphere rendering technique* | Bevy's atmosphere model |
| `10.1111_j.1467-8659.2008.01245.x` ★ | Bruneton & Neyret 2008, *Precomputed atmospheric scattering* | reference model |
| `s2021-advances-pdf-66-mb` | Patry 2021, *Real-Time Samurai Cinema* (Ghost of Tsushima): sky, atmosphere, clouds, wind, night | production parameters, wind and sky together |
| `s2017-advances-pdf-slides-2` | Decima engine 2017 (Horizon Zero Dawn) lighting | aerial perspective in production |
| `s2009-advances-materials-updated-july-12th2016-slides-pdf` | Bungie 2009, *Lighting research at Bungie* (Bruneton-based sky) | sky LUT practice |
| `s2006-advances-materials-11`, `-12` | Wenzel 2006, *Real-time atmospheric effects in games* (CryEngine 2): sky, height fog, clouds | analytic height fog |
| `book_real_time_rendering_4th_akenine` | *Real-Time Rendering* 4th ed., ch. 14 (volumetric and translucency rendering: sky, clouds, fog) | survey and formulas |

Not in the corpus: Meeus, Reda & Andreas (SPA), Jensen et al. 2001, Krisciunas & Schaefer 1991 — read from `/tmp/research/pdf` this session.

### Wind (M1)

| stem | what it is | use |
|---|---|---|
| `10.1145_311535.311548` ★ | Stam 1999, *Stable fluids* | solver |
| `GPU Gems- Programming Techniques, …` | *GPU Gems* 1 (2004): ch. 38 Harris, *Fast fluid dynamics simulation on the GPU*; ch. 7 *Rendering countless blades of waving grass* | GPU Jacobi/projection layout |
| `s2021-advances-pdf-66-mb` | Patry 2021 (Ghost of Tsushima wind system) | how a shipped game drives foliage from a wind field |
| `eth-cgl-sim_anim-Kim08` | Kim et al. 2008, *Wavelet turbulence* | sub-grid detail |
| `eth-cgl-sim_anim-Pfa10`, `-Pfa12` | Pfaff et al., turbulence particles | sub-grid detail |
| `sig2024_Cyclogenesis_Simulating_Hurricanes_and_Tornadoes` | Herrera et al. 2024, *Cyclogenesis* | atmospheric simulation in graphics |
| `10.1111_j.1467-8659.2010.01827.x` | Lagae et al. 2010, *A survey of procedural noise functions* | `noise.wesl` |

### Fog (M1)

| stem | what it is | use |
|---|---|---|
| `s2014-advances-pdf-slides-5-mb` | Wronski 2014, *Volumetric fog: unified compute shader based solution to atmospheric scattering* | seed, present |
| `s2006-advances-materials-11`, `-12` | Wenzel 2006 (height fog) | analytic height fog |
| `PBR3_11_Volume_Scattering`, `PBR3_14_Light_Transport_II_Volume_Rendering` | *PBRT* 3rd ed. ch. 11, 14 | phase functions, transmittance |
| `book_real_time_rendering_4th_akenine` | RTR4 ch. 14 | |

Hillaire 2015 (Frostbite volumetrics) is not in the corpus; the 2015 `froxel` hits are the Dreams talk.

### Clouds (M2)

| stem | what it is | use |
|---|---|---|
| `s2015-advances-ppt-944-mb-pdf-5-mb-2` ★ | Schneider 2015, *The real-time volumetric cloudscapes of Horizon Zero Dawn* | seed: Perlin-Worley, weather map, Beer–powder, temporal reprojection |
| `s2017-advances-pdf-slides-13mb` | Schneider 2017, *Nubis: authoring real-time volumetric cloudscapes with the Decima engine* | improvements over 2015 |
| `s2021-advances-pdf-66-mb` | Patry 2021, *Real-time samurai cinema* (Ghost of Tsushima): atmospheric lighting ll. 1883–2060, cloud slides ll. 2181–2380 | shipped alternative: paraboloid cloud map time-sliced over 60 frames |
| `eth-cgl-visualization-Rim17a`, `-Rim18a` | Rimensberger, *Visualization of clouds and atmospheric air flows* | |
| `book_real_time_rendering_4th_akenine` §14.4.2 (ll. 17713–17895) | Real-Time Rendering 4th ed., *Clouds* | σ_t ranges for stratus and cumulus, 4×4 reprojection |
| `10.48550_arxiv.2310.15364` | Sampling-pattern paper, §2 on interleaved gradient noise | why IGN suits temporal accumulation |

Not in the corpus (read from PDFs or URLs, see `clouds.md`): Hillaire 2016 *Physically based sky, atmosphere and cloud rendering in Frostbite* (EA PDF), Högfeldt 2016 Chalmers MSc thesis, Häggström *Real-time rendering of volumetric clouds* (OpenAlex W2562034945, download stalls), Wrenninge et al. 2013 *Oz: the great and volumetric*, Kasten & Czeplak 1980. Not useful for clouds: `s2025-advances-materials*` (talk titles only), `eth-cgl-various-Ke24a`.

### Precipitation, lightning, wetness and puddles (M2, M4)

| stem | what it is | use |
|---|---|---|
| `s2006-advances-materials-5`, `-6`, `s2006-advances-full-course-notes-pdf` | Tatarchuk 2006, *Artist-directable real-time rain rendering in city environments* (ToyShop): rain streaks, splashes, lightning, wet surfaces, puddles | seed `10.1145/1185657.1185828`, present under course stems |
| `s2017-advances-pdf-slides-notes` | Wronski? 2017, *Optimized pixel-projected reflections for planar reflectors* | puddle reflections |
| `book_real_time_rendering_4th_akenine` §§13.6, 13.6.2, 13.8, 5.5 | Real-Time Rendering 4th ed.: axial billboards, soft particles, particle systems, additive blending | rain streaks and snow sprites |
| `kim-lin-2004` (home-still S3 PDF only, not converted) | Kim & Lin 2004, *Physically based animation and rendering of lightning* (`10.1109/pccga.2004.1348357`) | contrast to Reed & Wyvill |

Not in the corpus (open PDFs or SciDB, under `/tmp/research/pdf`; see `precipitation.md`, `lightning.md`): Reed & Wyvill 1994, Garg & Nayar 2006/2007, Langer et al. 2004, Wang & Wade 2004 (the plan's seed `10.1145/1186223.1186241`), Uijlenhoet 2001, Kanazawa & Uchida 2025, Rasmussen et al. 1999, Poelman et al. 2021, Rakov. The library has no other precipitation or lightning rendering work.

### Terrain and LOD (M3)

| stem | what it is | use |
|---|---|---|
| `10.1080_2151237x.2009.10129287` | Strugar 2009, *Continuous distance-dependent level of detail for rendering heightmaps* (CDLOD) | seed, present |
| `s2017-advances-pdf-slides-notes` | Cichocki 2017, *Optimized pixel-projected reflections for planar reflectors* | puddle reflections |
| `s2007-advances-course-notes-1-6-mb`, `10.1145_1281500.1281668` | Andersson 2007, *Terrain rendering in Frostbite using procedural shader splatting* | layer blending |
| `s2021-advances-pdf-5-mb` | Deliot et al. 2021, concurrent binary trees for large-scale terrain (Unity) | |
| `10.1111_cgf.12276` | Smelik et al. 2014, *A survey on procedural modelling for virtual worlds* | |

### Erosion and hydrology (M3)

| stem | what it is | use |
|---|---|---|
| `10.1145_74334.74337` ★ | Musgrave et al. 1989, *The synthesis and rendering of eroded fractal terrains* (thermal + hydraulic) | seed |
| `10.1111_cgf.15033` | Tzathas et al. 2024, *Physically-based analytical erosion for fast terrain generation* (stream power, thermal) | |
| `10.1145_3658213` | Jain, Benes & Cordonnier 2024, *Efficient debris-flow simulation for steep terrain erosion* | |
| `10.21203_rs.3.rs-4256563_v1` | Hartley et al. 2024, *Flexible terrain erosion* | |
| `10.48550_arxiv.2210.14496` | Lim et al. 2022, *Visually improved erosion algorithm for tile-based terrain* | |
| `eth-cgl-sim_anim-Sol11b` | Solenthaler et al. 2011, SPH-based shallow water for real-time | shallow-water alternative |

Fetched for M3 (2026-10-13), text from `pdftotext -layout` in `/tmp/research/pdf/<name>.txt`, not yet converted by scribe:

| file | what it is | how obtained |
|---|---|---|
| `10.1109_pg.2007.15` | Mei, Decaudin & Hu 2007, *Fast hydraulic erosion simulation and visualization on GPU* (virtual pipes) | SciDB, ingested into home-still |
| `jako2011` | Jákó 2011, *Fast hydraulic and thermal erosion on the GPU* (supervised by Tóth), CESCG 2011 | open PDF `old.cescg.org/CESCG-2011/papers/TUBudapest-Jako-Balazs.pdf`; the plan's DOI `10.2312/eg2011/short/057-060` is not this paper's (it is a CESCG student paper, not an EG 2011 short) and is absent from SciDB |
| `10.1111_cgf.13657` | Galin et al. 2019, *A review of digital terrain modeling* | `hs paper download` |
| `10.1111_cgf.12820` | Cordonnier et al. 2016, *Large scale terrain generation from tectonic uplift and fluvial erosion* | `hs paper download` |
| `10.1111_j.1467-8659.2009.01361.x` | Krištof et al. 2009, *Hydraulic erosion using smoothed particle hydrodynamics* | SciDB, ingested |
| `10.1145_97880.97884` | Kass & Miller 1990, *Rapid, stable fluid dynamics for computer graphics* | SciDB, ingested |
| `chentanez2010` | Chentanez & Müller 2010, *Real-time simulation of large bodies of water with small scale details* (SCA) | open PDF `matthias-research.github.io/pages/publications/hfFluid.pdf` |

### Water rendering (M3)

| stem | what it is | use |
|---|---|---|
| `s2010-advances-pdf-slides-10` | Vlachos 2010, *Water flow in Portal 2* | seed, present |

### Snow cover and melt (M4)

No graphics paper on snow cover is in the corpus. Hydrology papers with degree-day snowmelt routines are: `10.1029_2021wr030993` (Xu et al. 2022, snow-dominated karst watershed), `10.5194_hess-26-2405-2022`, `10.5194_hess-27-1961-2023`, `10.5194_hess-28-2505-2024` — useful for degree-day factors and melt thresholds. Fearing 2000, Festenberg & Gumhold 2011 and Hock 2003 are not in the corpus.

### Dust and aeolian transport (M4)

| stem | what it is | use |
|---|---|---|
| `10.1111_cgf.13815` ★ | Paris et al. 2019, *Desertscape simulation* | seed |

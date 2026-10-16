# Saving and restoring the weather

A save game must put the weather back exactly as it was. Before this note, a game could serialize `WorldClock`, `GeoLocation`, `Climate` and `Weather` (all `serde` behind the `serde` feature), but not `WeatherDirector`, the scheduler's state: its fields were private and it derived neither `serde` nor `Reflect`. A reload therefore started a fresh director, which arms on its first frame at `Clear` (`run_weather`, `last_hour: None`), drops a forced preset, cuts a transition in progress, and from then on walks a different Markov chain than the run that was saved, because each hour's step depends on the previous preset.

## Sources

- Gregory, J. (2018). *Game Engine Architecture*, 3rd ed., A K Peters/CRC, §16.4.5 "Saved Games" (corpus: `Ga/Game Engine Architecture, Third Edition…`, lines 24758–24785). A save-anywhere file holds "the current locations and internal states of every game object whose state is relevant to gameplay"; static data that can be re-derived is omitted. Primary for the split below: the schedule is re-derived from `Climate::seed`, the director's chain position is not.
- Gregory (2018), §"Record and playback" (same file, line 19506): playback "requires every engine system to behave in a deterministic manner". The weather scheduler is deterministic per hour (`docs/research/climate-weather.md`), so a restored chain state continues bit-for-bit.
- Steele, G. L., Lea, D., Flood, C. H. (2014). Fast splittable pseudorandom number generators. OOPSLA. DOI `10.1145/2660193.2660195`. The scheduler's draw for hour `i` is a pure hash of `(seed, i)` with no generator state, so the only state to save is the chain's previous preset and the blend in progress, not an RNG cursor.

## Model adopted

The weather's save state is the set of CPU resources that are not re-derivable: `WorldClock` (time), `Climate` (its seed and shares), `GeoLocation`, and `WeatherDirector` (scheduled preset and wind turn, forced preset, pending request, transition in progress, current blend, last stepped hour). `Weather` is recomputed from these every frame and need not be saved. `WeatherDirector` and its private parts derive `serde::{Serialize, Deserialize}` behind the existing `serde` feature, and `Reflect`, so it can be written by `serde` directly or included in a Bevy `DynamicScene` like the other resources.

Restore: insert the saved `WorldClock`, `Climate`, `GeoLocation` and `WeatherDirector` before the next `ElementsSystems::Weather` run. Because the director keeps `last_hour`, the first frame after a restore steps zero hours and the chain continues where it stopped.

Out of scope here (recorded in the loop backlog): the GPU layers (snow cover, wetness, water, terrain, sand) live in storage buffers; saving them needs a readback of whole layers and is a separate change.

## Parameters

None. The change adds no constants.

## GPU mapping

None. CPU only; no GPU cost.

## Validation targets

- **Resume**: an app run for 72 game hours, saved through `serde` (JSON), and restored into a fresh app produces the same `Weather` as the uninterrupted app at every update for a further 72 game hours, including across scheduler changes and during a transition.
- **Forced**: a director saved mid-force (a forced Thunderstorm halfway through its blend) restores forced, at the same blend.
- **Counter-check**: restoring only `WorldClock` without the director diverges (the chain restarts at Clear), which is the bug this fixes.
- **Visual inventory**: none. The change alters no rendered element; the check is that a windowed `weather_tour` frame is unchanged (before/after pair).

## Deviations from this plan

None.

## Pending index

None: no new document was added to the corpus. `hs status` failed in this shell ("No such device or address", no TTY); research used the synced corpus only.

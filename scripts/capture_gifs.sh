#!/usr/bin/env bash
# Capture the showcase animations in docs/.
#
#   scripts/capture_gifs.sh <output directory>
#
# Every animation is captured headless (Xvfb), at a fixed clock, seed and camera, so the same
# frames come out on every run. The frames are then assembled into a GIF with a *flattened*
# alpha channel and an ordered palette: a GIF that keeps its alpha shows as a speckled mess
# over whatever the viewer puts behind it (the first `docs/sky_light.gif` did, and read as a
# "bowl of jello"), and a dithered palette on a smooth sky reads as noise.
#
# The scenes:
#   sky_light         the pond under a partly cloudy sky: the clouds in the environment map
#                     and their reflection in the water (docs/research/sky-light.md).
#   precipitation_look  the isometric town in rain, the physical look beside the readable one
#                     (docs/research/precipitation-look.md).
set -euo pipefail

OUT=${1:?usage: capture_gifs.sh <output directory>}
REPO=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO"

mkdir -p "$OUT"

# The examples' own capture knobs: a fixed 1/60 s step, so the frames are comparable.
COMMON=(ELEMENTS_FRAMES=1200)

run() {
  local dir=$1 example=$2
  shift 2
  rm -rf "$dir"
  mkdir -p "$dir"
  env "${COMMON[@]}" ELEMENTS_SHOT_DIR="$dir" "$@" \
    xvfb-run -a -s "-screen 0 1920x1080x24" "./target/release/examples/$example" \
    >/dev/null 2>&1
}

# The pond: a low camera just above the water, the clouds in the environment map.
run "$OUT/sky_light" waves \
  ELEMENTS_SHOT_EVERY=32 ELEMENTS_SHOT_FROM=100 \
  ELEMENTS_HOUR=16.5 ELEMENTS_PRESET=PartlyCloudy \
  ELEMENTS_EYE=-9,6.6,-11 ELEMENTS_LOOK=0,5.7,-2

# The town in rain, the physical look on the left of the frame and the readable one on the
# right: two runs, joined side by side.
for look in physical readable; do
  run "$OUT/precipitation_$look" isometric \
    ELEMENTS_SHOT_EVERY=32 ELEMENTS_SHOT_FROM=100 \
    ELEMENTS_HOUR=15 ELEMENTS_PRESET=Rain \
    ELEMENTS_PITCH=60 ELEMENTS_ZOOM=26 \
    ELEMENTS_PRECIPITATION="$look"
done

assemble() {
  # $1: the frames, $2: the output GIF, $3: the width.
  local frames=$1 out=$2 width=$3
  # The frames carry an alpha channel, and a GIF that keeps it shows over whatever the viewer
  # puts behind it (the first `docs/sky_light.gif` did, and read as a "bowl of jello"). Flatten
  # the frames onto their own background before the palette is cut.
  magick "$frames"/frame*.png \
    -resize "${width}x" \
    -alpha off \
    -colors 256 -dither FloydSteinberg \
    -coalesce -layers Optimize "$out"
}

assemble "$OUT/sky_light" "$OUT/sky_light.gif" 560
echo "wrote $OUT/sky_light.gif"

# The two precipitation runs, side by side, joined into one animation. Each frame of the
# physical run is joined to the frame of the readable run, so the two looks can be compared at
# the same instant.
rm -f "$OUT"/joined*.png
for f in "$OUT/precipitation_physical"/frame*.png; do
  n=$(basename "$f")
  magick "$f" "$OUT/precipitation_readable/$n" +append -alpha off "$OUT/joined$n"
done
magick "$OUT"/joined*.png -resize 800x -colors 256 -dither FloydSteinberg \
  -coalesce -layers Optimize "$OUT/precipitation_look.gif"
rm -f "$OUT"/joined*.png
echo "wrote $OUT/precipitation_look.gif"

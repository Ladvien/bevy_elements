#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "pillow", "scipy", "zstandard"]
# ///
"""Pack a ground texture set for bevy_elements' terrain (`docs/research/ground-textures.md`).

    uv run tools/ground-textures/prep.py tools/ground-textures/golem.json assets/ground/ground.ktx2

The manifest names, for each of the terrain's ground layers (sand, soil, grass), an albedo image,
the mean tilt of its relief's normals in degrees (`tilt_deg`) and, optionally, its de-lighting
exponent (`contrast`, default 0.65); its `tile_m` is the tile size the set was made for, which a
game sets in `GroundTextures::tile_m`. The output is a KTX2 2D array texture of six layers
(`R8G8B8A8_UNORM`, linear, every mip level, zstd supercompression) which a game loads with
`GroundTextures::load` and hands to `GroundTextures`:

- layer `2k`:     RGB albedo **detail** (the albedo over its own blur: a multiplier with mean 1,
                  stored as `value * 64 / 255`), A height (0-1, mean 0.5);
- layer `2k + 1`: RG the tangent-space normal's u and v components (`(c * 127 + 128) / 255`; u
                  along the image's columns, v along its rows), B roughness detail (a multiplier,
                  `value * 128 / 255`), A ambient occlusion (0-1).

The albedo is made tileable with a variance-preserving cross-fade against its own half-period
offset (Heitz & Neyret 2018: a blend that preserves the variance keeps the contrast), the low
frequencies a generated photograph carries (vignetting, a composition) are divided out, and the
height, normal, roughness and occlusion are derived from the detail. The mip levels average the
decoded values, so the detail's mean stays 1 at every level and the farthest level is neutral.
Any albedo works: a stylised set is a stylistic variant of the ground.
"""

import argparse
import json
import struct
from pathlib import Path

import numpy as np
import zstandard
from PIL import Image
from scipy import ndimage

LAYERS = ("sand", "soil", "grass")
VK_FORMAT_R8G8B8A8_UNORM = 37
SUPERCOMPRESSION_ZSTD = 2


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def load_linear(path: Path, size: int) -> np.ndarray:
    image = Image.open(path).convert("RGB").resize((size, size), Image.LANCZOS)
    return srgb_to_linear(np.asarray(image, dtype=np.float64) / 255.0)


def blur(a: np.ndarray, sigma: float) -> np.ndarray:
    """A Gaussian blur that wraps, so a tileable image stays tileable."""
    s = (sigma, sigma) + (0,) * (a.ndim - 2)
    return ndimage.gaussian_filter(a, s, mode="wrap")


def make_tileable(a: np.ndarray, band: float = 0.18) -> np.ndarray:
    """Cross-fade the image with its half-period roll near the borders, preserving variance.

    The rolled copy is continuous across the image's borders (its own seams sit in the middle,
    where its weight is zero), so the result tiles. A linear blend of two uncorrelated patches
    loses contrast where both weigh half; dividing the centred blend by `sqrt(w1² + w2²)` keeps
    the variance (Heitz & Neyret 2018).
    """
    n = a.shape[0]
    rolled = np.roll(a, (n // 2, n // 2), axis=(0, 1))
    x = np.arange(n)
    edge = np.minimum(x, n - 1 - x) / (band * n)
    w1d = 1.0 - np.clip(edge, 0.0, 1.0)
    w1d = w1d * w1d * (3.0 - 2.0 * w1d)
    w = np.maximum(w1d[:, None], w1d[None, :])[..., None]
    mean = a.mean(axis=(0, 1), keepdims=True)
    blended = ((1.0 - w) * (a - mean) + w * (rolled - mean)) / np.sqrt((1.0 - w) ** 2 + w**2)
    return np.clip(mean + blended, 1e-4, 1.0)


# The albedo detail's encoding: `value * ALBEDO_SCALE / 255`, so 64 decodes to exactly one and
# the range is [0, 4). Linear, so the GPU's filtering and the mips average it correctly.
ALBEDO_SCALE = 64.0
LUMA = np.array([0.2126, 0.7152, 0.0722])


def derive_layer(
    albedo: np.ndarray, tilt_deg: float, contrast: float
) -> tuple[np.ndarray, np.ndarray]:
    """The two decoded array layers of one ground layer: (detail rgb, height) and
    (normal u, normal v, roughness detail, occlusion)."""
    n = albedo.shape[0]
    tiled = make_tileable(albedo)
    # The detail: the albedo over its own low frequencies, mean 1.
    detail = tiled / blur(tiled, n / 12.0)
    # De-lighting. A photograph's contrast is its albedo's *and* its light's: the lit tops and
    # the shadowed gaps of a turf span ten to one in linear light, where the albedo of a blade
    # and of the soil under it span three or four. The luminance ratio is compressed by the
    # exponent `contrast`; the hue against the local mean is kept.
    lum = np.maximum(detail @ LUMA, 1e-4)
    detail = detail * (lum ** (contrast - 1.0))[..., None]
    # The mean is restored after the clip, a few times, until the two agree.
    for _ in range(6):
        detail = np.clip(detail / detail.mean(axis=(0, 1), keepdims=True), 0.0, 254.0 / ALBEDO_SCALE)
    lum = detail @ LUMA
    # The height: the luminance band-passed (bright is high: lit tops, dark gaps), normalised.
    band = blur(lum, 1.2) - blur(lum, n / 24.0)
    height = np.clip(0.5 + band / (4.0 * band.std() + 1e-9), 0.0, 1.0)
    # The normal: the height's slope, scaled so that the normal's mean tilt is `tilt_deg`. A depth
    # in metres is the wrong knob: the slope depends on the height's frequency content, and a
    # turf's 2 mm detail at a "3 cm" depth tilted its normals 47° on average, which darkened the
    # meadow by a fifth under a high sun (the mean of N·L is the mean of the normals' z).
    dhdu = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) / 2.0
    dhdv = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) / 2.0
    gradient = np.sqrt(dhdu**2 + dhdv**2)
    low, high = 0.0, 1.0e3
    for _ in range(60):
        scale = 0.5 * (low + high)
        if np.degrees(np.arctan(scale * gradient)).mean() < tilt_deg:
            low = scale
        else:
            high = scale
    dhdu, dhdv = dhdu * scale, dhdv * scale
    norm = np.sqrt(dhdu**2 + dhdv**2 + 1.0)
    # Roughness: the hollows rougher than the tops, a multiplier with mean 1.
    rough = 1.0 + 0.5 * (0.5 - height)
    # Occlusion: the depth of the cavity below its neighbourhood, only in the deepest hollows, so
    # the ground's mean light barely moves.
    cavity = np.clip((blur(height, n / 96.0) - height) * 4.0, 0.0, 1.0)
    occlusion = 1.0 - 0.2 * cavity
    first = np.dstack([detail, height])
    second = np.dstack([-dhdu / norm, -dhdv / norm, rough, occlusion])
    return first, second


def encode(first: np.ndarray, second: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    def unit(v: np.ndarray) -> np.ndarray:
        return np.clip(np.round(v * 255.0), 0, 255).astype(np.uint8)

    a = np.dstack([unit(first[..., :3] * ALBEDO_SCALE / 255.0), unit(first[..., 3])])
    b = np.dstack(
        [
            unit((second[..., 0] * 127.0 + 128.0) / 255.0),
            unit((second[..., 1] * 127.0 + 128.0) / 255.0),
            unit(np.clip(second[..., 2], 0.0, 254.0 / 128.0) * 128.0 / 255.0),
            unit(second[..., 3]),
        ]
    )
    return a, b


def halve(a: np.ndarray) -> np.ndarray:
    return 0.25 * (a[0::2, 0::2] + a[1::2, 0::2] + a[0::2, 1::2] + a[1::2, 1::2])


def basic_dfd() -> bytes:
    """The Khronos basic data format descriptor of a linear `R8G8B8A8_UNORM` texel."""
    samples = 4
    block = 24 + 16 * samples
    out = struct.pack("<I", 4 + block)
    out += struct.pack("<IHH", 0, 2, block)  # vendor 0, type 0; version 2; block size
    out += struct.pack("<4B", 1, 1, 1, 0)  # RGBSDA, BT.709, linear, straight alpha
    out += struct.pack("<4B", 0, 0, 0, 0)  # a 1x1x1x1 texel block
    out += struct.pack("<8B", 4, 0, 0, 0, 0, 0, 0, 0)  # four bytes in plane 0
    for i, channel in enumerate((0, 1, 2, 15)):  # R, G, B, A
        out += struct.pack("<HBB4BII", i * 8, 7, channel, 0, 0, 0, 0, 0, 255)
    return out


def write_ktx2(path: Path, levels: list[bytes], size: int, layers: int) -> None:
    """A KTX2 file: each level is every layer of that mip, zstd supercompressed."""
    compressor = zstandard.ZstdCompressor(level=19)
    packed = [compressor.compress(level) for level in levels]
    dfd = basic_dfd()
    header = b"\xabKTX 20\xbb\r\n\x1a\n" + struct.pack(
        "<9I",
        VK_FORMAT_R8G8B8A8_UNORM,
        1,
        size,
        size,
        0,
        layers,
        1,
        len(levels),
        SUPERCOMPRESSION_ZSTD,
    )
    index_end = len(header) + 4 * 4 + 2 * 8 + 24 * len(levels)
    dfd_offset = index_end
    data_offset = dfd_offset + len(dfd)
    # The levels are stored smallest first, as the specification recommends.
    offsets = [0] * len(levels)
    cursor = data_offset
    for i in reversed(range(len(levels))):
        offsets[i] = cursor
        cursor += len(packed[i])
    index = struct.pack("<IIIIQQ", dfd_offset, len(dfd), 0, 0, 0, 0)
    for i in range(len(levels)):
        index += struct.pack("<QQQ", offsets[i], len(packed[i]), len(levels[i]))
    body = b"".join(packed[i] for i in reversed(range(len(levels))))
    path.write_bytes(header + index + dfd + body)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--size", type=int, default=1024)
    args = parser.parse_args()
    if args.size & (args.size - 1):
        parser.error("--size must be a power of two")
    manifest = json.loads(args.manifest.read_text())
    root = args.manifest.parent
    decoded = []
    for name in LAYERS:
        layer = manifest["layers"][name]
        albedo = load_linear(root / layer["albedo"], args.size)
        decoded.append(
            derive_layer(albedo, layer["tilt_deg"], layer.get("contrast", 0.65))
        )
        print(f"{name}: tile {layer['tile_m']} m, tilt {layer['tilt_deg']}°")
    levels = []
    size = args.size
    while True:
        layers = [arr for pair in decoded for arr in encode(*pair)]
        levels.append(b"".join(layer.tobytes() for layer in layers))
        if size == 1:
            break
        decoded = [(halve(a), halve(b)) for a, b in decoded]
        size //= 2
    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_ktx2(args.output, levels, args.size, 2 * len(LAYERS))
    print(f"{args.output}: {args.size}² x {2 * len(LAYERS)} layers, {len(levels)} levels, "
          f"{args.output.stat().st_size // 1024} KiB")


if __name__ == "__main__":
    main()

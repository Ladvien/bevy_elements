//! Builds `src/celestial/moon_albedo.r8` from the NASA SVS CGI Moon Kit colour map (SVS 4720).
//!
//! ```text
//! curl -fsSL -o /tmp/lroc_color_2k.jpg https://svs.gsfc.nasa.gov/vis/a000000/a004700/a004720/lroc_color_2k.jpg
//! cargo run --manifest-path tools/Cargo.toml --bin gen_moon -- /tmp/lroc_color_2k.jpg src/celestial/moon_albedo.r8
//! ```
//!
//! Processing, in order:
//! 1. decode the 2048x1024 sRGB JPEG (equirectangular, column 0 = longitude -180 degrees, top row =
//!    north);
//! 2. sRGB to linear light per channel, then linear luminance `Y = 0.2126 R + 0.7152 G + 0.0722 B`;
//! 3. box-filter 4x4 down to 512x256 (the average is taken in linear light);
//! 4. scale so that the mean over the full-moon disc, as seen from the sub-observer point
//!    (latitude 0, longitude 0, north up), equals 128 of 255 after rounding to `u8` and clipping at
//!    255. The disc mean is the quantity the shader's photometric calibration fixes (the disc-mean
//!    albedo of the Moon), not the plain mean over the equirectangular map, which over-weights the
//!    poles and the far side.
//!
//! The disc quadrature ([`disc_mean_byte`]) is mirrored in `src/celestial/moon.rs` (`disc_mean_albedo`),
//! whose unit test fails if the two drift apart.

use std::error::Error;
use std::f64::consts::PI;

/// Output width, texels.
const WIDTH: usize = 512;
/// Output height, texels.
const HEIGHT: usize = 256;
/// Box-filter factor from the 2048x1024 source.
const BOX: usize = 4;
/// The disc-mean texel value the shader's albedo formula assumes.
const TARGET_MEAN: f64 = 128.0;
/// Quadrature grid of the disc, cells per side.
const DISC_SAMPLES: usize = 1024;

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Bilinear sample of a `WIDTH` x `HEIGHT` byte map at selenographic `lat`/`lon` (radians):
/// longitude wraps, latitude clamps.
fn sample(bytes: &[u8], lat: f64, lon: f64) -> f64 {
    let x = (lon + PI) / (2.0 * PI) * WIDTH as f64 - 0.5;
    let y = (PI / 2.0 - lat) / PI * HEIGHT as f64 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |xi: f64, yi: f64| {
        let col = (xi as i64).rem_euclid(WIDTH as i64) as usize;
        let row = (yi as i64).clamp(0, HEIGHT as i64 - 1) as usize;
        f64::from(bytes[row * WIDTH + col])
    };
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
    let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Mean texel value (0..255) over the full-moon disc seen from latitude 0, longitude 0.
fn disc_mean_byte(bytes: &[u8]) -> f64 {
    let n = DISC_SAMPLES;
    let (mut sum, mut count) = (0.0, 0u64);
    for j in 0..n {
        let y = (j as f64 + 0.5) / n as f64 * 2.0 - 1.0;
        for i in 0..n {
            let x = (i as f64 + 0.5) / n as f64 * 2.0 - 1.0;
            let r2 = x * x + y * y;
            if r2 >= 1.0 {
                continue;
            }
            // Moon-fixed frame: +X toward the observer, +Y east (screen right), +Z north (up).
            let mu = (1.0 - r2).sqrt();
            sum += sample(bytes, y.asin(), x.atan2(mu));
            count += 1;
        }
    }
    sum / count as f64
}

fn quantise(luminance: &[f64], scale: f64) -> Vec<u8> {
    luminance
        .iter()
        .map(|&v| (v * scale).round().clamp(0.0, 255.0) as u8)
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        return Err("usage: gen_moon <lroc_color_2k.jpg> <moon_albedo.r8>".into());
    };
    let rgb = image::open(&input)?.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    if (w, h) != (WIDTH * BOX, HEIGHT * BOX) {
        return Err(format!("expected {}x{}, got {w}x{h}", WIDTH * BOX, HEIGHT * BOX).into());
    }

    let linear: Vec<f64> = rgb
        .pixels()
        .map(|p| {
            let [r, g, b] = p.0.map(|c| srgb_to_linear(f64::from(c) / 255.0));
            0.2126 * r + 0.7152 * g + 0.0722 * b
        })
        .collect();
    let mut luminance = vec![0.0f64; WIDTH * HEIGHT];
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            let mut sum = 0.0;
            for dy in 0..BOX {
                for dx in 0..BOX {
                    sum += linear[(row * BOX + dy) * w + col * BOX + dx];
                }
            }
            luminance[row * WIDTH + col] = sum / (BOX * BOX) as f64;
        }
    }

    // The disc mean rises monotonically with the scale (clipping only flattens it), so bisect.
    let (mut lo, mut hi): (f64, f64) = (1.0, 1.0e6);
    for _ in 0..80 {
        let mid = (lo * hi).sqrt();
        if disc_mean_byte(&quantise(&luminance, mid)) < TARGET_MEAN {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let scale = (lo * hi).sqrt();
    let bytes = quantise(&luminance, scale);
    let mean = disc_mean_byte(&bytes);
    let clipped = bytes.iter().filter(|&&b| b == 255).count();
    std::fs::write(&output, &bytes)?;
    println!(
        "{WIDTH}x{HEIGHT} -> {output}: scale {scale:.3}, disc mean {mean:.4}/255, {clipped} texels clipped, max {}",
        bytes.iter().max().copied().unwrap_or(0)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_map_has_that_disc_mean() {
        let flat = vec![100u8; WIDTH * HEIGHT];
        assert!((disc_mean_byte(&flat) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn srgb_endpoints() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-12);
    }
}

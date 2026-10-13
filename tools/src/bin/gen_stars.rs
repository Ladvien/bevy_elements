//! Builds `src/celestial/stars.bin` from the CDS copy of the Yale Bright Star Catalogue (V/50).
//!
//! ```text
//! curl -fsSL -o /tmp/catalog.gz https://cdsarc.cds.unistra.fr/ftp/cats/V/50/catalog.gz
//! cargo run --manifest-path tools/Cargo.toml --bin gen_stars -- /tmp/catalog.gz src/celestial/stars.bin
//! ```
//!
//! Fixed-width layout, from `https://cdsarc.cds.unistra.fr/ftp/cats/V/50/ReadMe` (1-based byte
//! columns, inclusive): RA J2000 hours 76-77, minutes 78-79, seconds 80-83 (`F4.1`); Dec J2000 sign
//! 84, degrees 85-86, arcminutes 87-88, arcseconds 89-90; `Vmag` 103-107; `B-V` 110-114.
//!
//! A record is kept when it has a J2000 position and a `Vmag` of at most [`MAX_VMAG`]. A blank `B-V`
//! becomes [`DEFAULT_B_MINUS_V`], the median of the catalogue values. Output: little-endian
//! `[f32; 4]` per star, `(ra_rad, dec_rad, vmag, b_minus_v)`, in catalogue (HR) order.

use std::error::Error;
use std::f64::consts::PI;
use std::io::Read;

/// Faintest star kept, apparent visual magnitude.
const MAX_VMAG: f64 = 6.5;
/// `B-V` used where the catalogue has none, magnitudes (median of the 8160 catalogued values among
/// the kept stars).
const DEFAULT_B_MINUS_V: f64 = 0.48;

/// One kept star.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Star {
    ra_rad: f64,
    dec_rad: f64,
    vmag: f64,
    b_minus_v: f64,
}

/// The text of 1-based inclusive byte columns `first..=last`, trimmed; `None` if the line is too
/// short or the field is blank.
fn field(line: &str, first: usize, last: usize) -> Option<&str> {
    let text = line.get(first - 1..last.min(line.len()))?.trim();
    (!text.is_empty()).then_some(text)
}

fn number(line: &str, first: usize, last: usize) -> Option<f64> {
    field(line, first, last)?.parse().ok()
}

/// Parse one catalogue line into a star and whether the catalogue gave its `B-V`; `None` for a
/// star without a J2000 position or without `Vmag`, or fainter than [`MAX_VMAG`].
fn parse_line(line: &str) -> Option<(Star, bool)> {
    let ra_h = number(line, 76, 77)?;
    let ra_m = number(line, 78, 79)?;
    let ra_s = number(line, 80, 83)?;
    let sign = match field(line, 84, 84)? {
        "+" => 1.0,
        "-" => -1.0,
        _ => return None,
    };
    let dec_d = number(line, 85, 86)?;
    let dec_m = number(line, 87, 88)?;
    let dec_s = number(line, 89, 90)?;
    let vmag = number(line, 103, 107)?;
    if vmag > MAX_VMAG {
        return None;
    }
    let measured = number(line, 110, 114);
    let ra_deg = (ra_h + ra_m / 60.0 + ra_s / 3600.0) * 15.0;
    let dec_deg = sign * (dec_d + dec_m / 60.0 + dec_s / 3600.0);
    let star = Star {
        ra_rad: ra_deg * PI / 180.0,
        dec_rad: dec_deg * PI / 180.0,
        vmag,
        b_minus_v: measured.unwrap_or(DEFAULT_B_MINUS_V),
    };
    Some((star, measured.is_some()))
}

fn encode(stars: &[Star]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stars.len() * 16);
    for s in stars {
        for v in [s.ra_rad, s.dec_rad, s.vmag, s.b_minus_v] {
            out.extend_from_slice(&(v as f32).to_le_bytes());
        }
    }
    out
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        return Err("usage: gen_stars <catalog.gz> <stars.bin>".into());
    };
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(&input)?).read_to_string(&mut text)?;
    let parsed: Vec<(Star, bool)> = text.lines().filter_map(parse_line).collect();
    let missing_color = parsed.iter().filter(|(_, measured)| !measured).count();
    let stars: Vec<Star> = parsed.into_iter().map(|(star, _)| star).collect();
    std::fs::write(&output, encode(&stars))?;
    println!(
        "{} stars (V <= {MAX_VMAG}), {missing_color} without B-V -> {output}",
        stars.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 197-column line with the given 1-based columns filled in.
    fn line(fields: &[(usize, &str)]) -> String {
        let mut cols = vec![b' '; 197];
        for &(at, text) in fields {
            cols[at - 1..at - 1 + text.len()].copy_from_slice(text.as_bytes());
        }
        String::from_utf8(cols).expect("ascii")
    }

    #[test]
    fn sirius_parses_to_the_catalogue_values() {
        // HR 2491: 06h45m08.9s, -16d42m58s, V -1.46, B-V 0.00.
        let sirius = line(&[
            (76, "064508.9"),
            (84, "-164258"),
            (103, "-1.46"),
            (110, " 0.00"),
        ]);
        let (star, measured) = parse_line(&sirius).expect("kept");
        assert!(measured);
        assert!((star.ra_rad - 1.767_793).abs() < 1e-6, "{}", star.ra_rad);
        assert!((star.dec_rad - -0.291_751).abs() < 1e-6, "{}", star.dec_rad);
        assert_eq!(star.vmag, -1.46);
        assert_eq!(star.b_minus_v, 0.0);
    }

    #[test]
    fn a_blank_colour_takes_the_default() {
        let blank = line(&[(76, "064508.9"), (84, "+164258"), (103, " 3.00")]);
        let (star, measured) = parse_line(&blank).expect("kept");
        assert!(!measured);
        assert_eq!(star.b_minus_v, DEFAULT_B_MINUS_V);
        assert!(star.dec_rad > 0.0);
    }

    #[test]
    fn faint_and_positionless_lines_are_dropped() {
        assert!(parse_line("").is_none());
        let faint = line(&[(76, "064508.9"), (84, "-164258"), (103, " 6.51")]);
        assert!(parse_line(&faint).is_none());
        let no_position = line(&[(103, " 3.00")]);
        assert!(parse_line(&no_position).is_none());
    }
}

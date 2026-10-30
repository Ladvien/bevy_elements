//! A fixed snow and wetness cover for one material: [`SurfaceCover`], its two [`CoverBand`]s and
//! the CPU query the shader mirrors. See `docs/research/surface-cover.md`.

use bevy::prelude::*;

/// Where one kind of cover (snow or a wet film) lies on a surface: an amount, the steepest face
/// that holds it and the heights it reaches. Evaluated per face, from its geometric normal and
/// its height, by [`coverage`](Self::coverage); the shader of a
/// [`WeatheredExtension`](crate::WeatheredExtension) evaluates the same function per pixel.
///
/// The slope is the angle of the face's normal from straight up: 0° is a floor, 90° a wall, 180°
/// an underside. Coverage is `amount` where the face is flatter than `max_slope_deg`, falls
/// smoothly to 0 over the `slope_soft_deg` below it (a step when that is 0), and is 0 on a steeper
/// face. Heights are metres of world Y ([`ElementsUnits`](crate::ElementsUnits) divided out) and
/// work the same way: full between `min_height_m` and `max_height_m`, falling to 0 over
/// `height_soft_m` inside each end, and `None` for an end that is not there.
///
/// The default is bare: `amount` 0. [`CoverBand::snow`] and [`CoverBand::wet`] are bands with the
/// slope a scene of that cover usually has.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct CoverBand {
    /// Coverage of a face that passes the slope and height tests, 0–1 (clamped).
    pub amount: f32,
    /// The steepest face that holds the cover, degrees from horizontal (0–180).
    pub max_slope_deg: f32,
    /// The slope range over which coverage falls to 0 below `max_slope_deg`, degrees; 0 for a hard edge.
    pub slope_soft_deg: f32,
    /// The lowest height covered, m; `None` for no floor.
    pub min_height_m: Option<f32>,
    /// The highest height covered, m; `None` for no ceiling.
    pub max_height_m: Option<f32>,
    /// The height range over which coverage falls to 0 at each end, m; 0 for a hard edge.
    pub height_soft_m: f32,
}

impl Default for CoverBand {
    fn default() -> Self {
        Self {
            amount: 0.0,
            max_slope_deg: 180.0,
            slope_soft_deg: 0.0,
            min_height_m: None,
            max_height_m: None,
            height_soft_m: 0.0,
        }
    }
}

/// A smooth 0–1 ramp of `t`, or a step at 0 when `soft` is 0.
fn ramp(value: f32, soft: f32) -> f32 {
    if soft > 0.0 {
        let t = (value / soft).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    } else if value >= 0.0 {
        1.0
    } else {
        0.0
    }
}

impl CoverBand {
    /// Snow of `amount`: faces up to 45° from horizontal hold it, shedding from 30°.
    pub fn snow(amount: f32) -> Self {
        Self {
            amount,
            max_slope_deg: 45.0,
            slope_soft_deg: 15.0,
            ..Self::default()
        }
    }

    /// A wet film of `amount` on every face that is not an underside (to 150°, from 120°).
    pub fn wet(amount: f32) -> Self {
        Self {
            amount,
            max_slope_deg: 150.0,
            slope_soft_deg: 30.0,
            ..Self::default()
        }
    }

    /// The coverage of a face with the world `normal` (any length, not zero) at `height_m`, 0–1.
    pub fn coverage(&self, normal: Vec3, height_m: f32) -> f32 {
        let amount = self.amount.clamp(0.0, 1.0);
        if amount == 0.0 {
            return 0.0;
        }
        let angle = normal.normalize_or_zero().y.clamp(-1.0, 1.0).acos().to_degrees();
        let slope = ramp(self.max_slope_deg - angle, self.slope_soft_deg);
        let above = self.min_height_m.map_or(1.0, |lo| ramp(height_m - lo, self.height_soft_m));
        let below = self.max_height_m.map_or(1.0, |hi| ramp(hi - height_m, self.height_soft_m));
        amount * slope * above * below
    }

    /// The band as the shader reads it: `[amount, max slope, slope soft, height soft]` and
    /// `[min height, max height, 0, 0]`, an absent end as ±1e30.
    pub(crate) fn packed(&self) -> [Vec4; 2] {
        [
            Vec4::new(
                self.amount.clamp(0.0, 1.0),
                self.max_slope_deg,
                self.slope_soft_deg.max(0.0),
                self.height_soft_m.max(0.0),
            ),
            Vec4::new(
                self.min_height_m.unwrap_or(-1.0e30),
                self.max_height_m.unwrap_or(1.0e30),
                0.0,
                0.0,
            ),
        ]
    }
}

/// A snow and wetness cover that a material wears whatever the weather is: put it on a
/// [`WeatheredExtension`](crate::WeatheredExtension) with
/// [`with_cover`](crate::WeatheredExtension::with_cover), for the mesh of a voxel volume, a
/// level's prop or a preview of a season the scene is not in.
///
/// While a material has a cover, its snow and its base wetness are the cover's, in place of the
/// scene's [`SurfaceWeathering`](crate::SurfaceWeathering): the slope and the height decide per face
/// where each lies, [`exposure`](crate::WeatheredExtension::exposure) still scales both, and the
/// rain's streaks on walls still follow the weather. Edit the cover of a live material through
/// [`WeatheredExtension::set_cover`](crate::WeatheredExtension::set_cover): the material is
/// re-prepared in the next frame.
///
/// [`snow_at`](Self::snow_at) and [`wetness_at`](Self::wetness_at) answer the same coverage on
/// the CPU, with no GPU, for a face or a point of the mesh, so a game can ask whether a spot is
/// white or wet. With the `serde` feature it is data a tool can save and reload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct SurfaceCover {
    /// Where snow lies: bare by default.
    pub snow: CoverBand,
    /// Where the surface is wet: dry by default.
    pub wetness: CoverBand,
}

impl SurfaceCover {
    /// A cover of this `snow` and `wetness` (each 0–1) with [`CoverBand::snow`]'s and
    /// [`CoverBand::wet`]'s slopes.
    pub fn new(snow: f32, wetness: f32) -> Self {
        Self {
            snow: CoverBand::snow(snow),
            wetness: CoverBand::wet(wetness),
        }
    }

    /// The snow cover of a face with the world `normal` at `height_m` (metres of world Y), 0–1.
    pub fn snow_at(&self, normal: Vec3, height_m: f32) -> f32 {
        self.snow.coverage(normal, height_m)
    }

    /// The wetness of a face with the world `normal` at `height_m`, 0–1 (before the rain's streaks).
    pub fn wetness_at(&self, normal: Vec3, height_m: f32) -> f32 {
        self.wetness.coverage(normal, height_m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sphere of radius 2 m centred 5 m up: a point's normal is its direction from the centre.
    fn sphere_normal(polar_deg: f32) -> (Vec3, f32) {
        let p = polar_deg.to_radians();
        (Vec3::new(p.sin(), p.cos(), 0.0), 5.0 + 2.0 * p.cos())
    }

    #[test]
    fn a_wet_sphere_is_fully_wet_on_top_and_dry_beneath() {
        let cover = SurfaceCover {
            wetness: CoverBand {
                amount: 1.0,
                max_slope_deg: 100.0,
                ..CoverBand::default()
            },
            ..SurfaceCover::default()
        };
        let (up, h) = sphere_normal(0.0);
        assert_eq!(cover.wetness_at(up, h), 1.0);
        let (side, h) = sphere_normal(60.0);
        assert_eq!(cover.wetness_at(side, h), 1.0);
        let (under, h) = sphere_normal(135.0);
        assert_eq!(cover.wetness_at(under, h), 0.0);
        assert_eq!(cover.snow_at(up, h), 0.0, "a default band is bare");
    }

    #[test]
    fn snow_sheds_smoothly_between_its_soft_edge_and_its_limit() {
        let band = CoverBand::snow(0.8);
        let at = |deg: f32| band.coverage(sphere_normal(deg).0, 0.0);
        assert_eq!(at(0.0), 0.8);
        assert_eq!(at(30.0), 0.8);
        assert!(at(37.5) > 0.3 && at(37.5) < 0.5, "half way: {}", at(37.5));
        assert_eq!(at(45.0), 0.0);
        assert_eq!(at(90.0), 0.0);
        assert!(at(34.0) > at(40.0), "monotone");
    }

    #[test]
    fn a_height_window_bounds_the_cover_and_an_amount_is_clamped() {
        let band = CoverBand {
            amount: 3.0,
            min_height_m: Some(10.0),
            max_height_m: Some(20.0),
            height_soft_m: 2.0,
            ..CoverBand::default()
        };
        let up = Vec3::Y;
        assert_eq!(band.coverage(up, 9.0), 0.0);
        assert_eq!(band.coverage(up, 10.0), 0.0);
        assert_eq!(band.coverage(up, 12.0), 1.0);
        assert_eq!(band.coverage(up, 15.0), 1.0);
        assert_eq!(band.coverage(up, 19.0), 0.5);
        assert_eq!(band.coverage(up, 21.0), 0.0);
        assert_eq!(band.coverage(up * 7.0, 15.0), 1.0, "the normal's length does not matter");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_cover_round_trips_and_missing_fields_are_the_defaults() {
        let cover = SurfaceCover {
            snow: CoverBand {
                max_height_m: Some(40.0),
                ..CoverBand::snow(0.5)
            },
            wetness: CoverBand::wet(0.25),
        };
        let json = serde_json::to_string(&cover).unwrap();
        assert_eq!(serde_json::from_str::<SurfaceCover>(&json).unwrap(), cover);
        let partial: SurfaceCover = serde_json::from_str(r#"{"snow":{"amount":1.0}}"#).unwrap();
        assert_eq!(partial.snow.amount, 1.0);
        assert_eq!(partial.snow.max_slope_deg, 180.0);
        assert_eq!(partial.wetness, CoverBand::default());
    }
}

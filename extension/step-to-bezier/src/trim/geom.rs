//! Small 2D helpers shared by the trim sub modules.

use rendiation_algebra::*;

use crate::math::*;

pub(super) type V2 = Vec2<f64>;
pub(super) type BBox = (V2, V2);

pub(super) fn cross(a: V2, b: V2) -> f64 {
  a.x * b.y - a.y * b.x
}

pub(super) fn coord(p: V2, axis: usize) -> f64 {
  if axis == 0 { p.x } else { p.y }
}

pub(super) fn set_coord(p: &mut V2, axis: usize, value: f64) {
  if axis == 0 {
    p.x = value;
  } else {
    p.y = value;
  }
}

pub(super) fn bbox_overlaps(a: BBox, b: BBox) -> bool {
  a.0.x <= b.1.x && b.0.x <= a.1.x && a.0.y <= b.1.y && b.0.y <= a.1.y
}

pub(super) fn bbox_union(a: BBox, b: BBox) -> BBox {
  (
    V2::new(a.0.x.min(b.0.x), a.0.y.min(b.0.y)),
    V2::new(a.1.x.max(b.1.x), a.1.y.max(b.1.y)),
  )
}

pub(super) fn bbox_expand(b: BBox, margin: f64) -> BBox {
  let m = V2::new(margin, margin);
  (b.0 - m, b.1 + m)
}

pub(super) fn bbox_translate(b: BBox, offset: V2) -> BBox {
  (b.0 + offset, b.1 + offset)
}

/// Length of the Euclidean control polygon, an upper bound of the curve length (positive
/// weights make rational Bézier curves variation diminishing).
pub(super) fn control_polygon_length(curve: &RationalBezierCurve2) -> f64 {
  curve
    .control_points
    .windows(2)
    .map(|w| (w[1].to_euclid() - w[0].to_euclid()).length())
    .sum()
}

const GAUSS_8: [(f64, f64); 4] = [
  (0.183_434_642_495_649_8, 0.362_683_783_378_362),
  (0.525_532_409_916_329, 0.313_706_645_877_887_3),
  (0.796_666_477_413_626_7, 0.222_381_034_453_374_5),
  (0.960_289_856_497_536_3, 0.101_228_536_290_376_3),
];

/// `½ ∫ (x·y' − y·x') dt` over the curve: summed over a closed loop this is the signed area
/// enclosed (Green's theorem), positive for counter-clockwise loops.
pub(super) fn curve_area_term(curve: &RationalBezierCurve2, subdivisions: usize) -> f64 {
  let h = 1. / subdivisions as f64;
  let mut sum = 0.;
  for k in 0..subdivisions {
    let mid = (k as f64 + 0.5) * h;
    for (x, w) in GAUSS_8 {
      for t in [mid - 0.5 * h * x, mid + 0.5 * h * x] {
        let (p, d) = curve.eval_with_derivative(t);
        sum += w * 0.5 * h * 0.5 * cross(p, d);
      }
    }
  }
  sum
}

pub(super) fn loop_signed_area<'a>(
  curves: impl IntoIterator<Item = &'a RationalBezierCurve2>,
  subdivisions: usize,
) -> f64 {
  curves
    .into_iter()
    .map(|c| curve_area_term(c, subdivisions))
    .sum()
}

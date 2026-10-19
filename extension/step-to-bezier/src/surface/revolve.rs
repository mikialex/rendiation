//! Exact rotational patches shared by cylinder, cone, sphere, torus and surfaces of revolution.

use super::*;

/// Rotation axis, angles are right handed about `dir` (unit).
#[derive(Debug, Clone, Copy)]
pub struct RevolveAxis {
  pub origin: Vec3<f64>,
  pub dir: Vec3<f64>,
}

impl RevolveAxis {
  /// Foot point on the axis and radial part of `q`.
  pub fn split(&self, q: Vec3<f64>) -> (Vec3<f64>, Vec3<f64>) {
    let foot = self.origin + self.dir * (q - self.origin).dot(self.dir);
    (foot, q - foot)
  }

  /// `q` rotated by `angle`, radial part scaled by `scale`.
  pub fn rotate_scaled(&self, q: Vec3<f64>, angle: f64, scale: f64) -> Vec3<f64> {
    let (foot, radial) = self.split(q);
    let (s, c) = angle.sin_cos();
    foot + (radial * c + self.dir.cross(radial) * s) * scale
  }

  /// Rotated point and its derivative with respect to the angle.
  pub fn rotate_with_derivative(&self, q: Vec3<f64>, angle: f64) -> (Vec3<f64>, Vec3<f64>) {
    let (foot, radial) = self.split(q);
    let (s, c) = angle.sin_cos();
    let side = self.dir.cross(radial);
    (foot + radial * c + side * s, side * c - radial * s)
  }

  /// Linear part of the rotation applied to a vector.
  pub fn rotate_vector(&self, v: Vec3<f64>, angle: f64) -> Vec3<f64> {
    let axial = self.dir * v.dot(self.dir);
    let radial = v - axial;
    let (s, c) = angle.sin_cos();
    axial + radial * c + self.dir.cross(radial) * s
  }
}

/// Exact patch of `generatrix` (the `v` direction) rotated over the angles `[a0, a1]`
/// (`0 < a1 - a0 <= π/2`, the `u` direction). The `u` parameter relates to the angle through
/// `arc_t_from_angle` with span `a1 - a0`, `v` is the generatrix parameter.
///
/// Every control point of the generatrix is mapped by the three affine "rotations" of the
/// rational quadratic arc (`a0`; the mid angle with the radial part scaled by
/// `1 / cos(φ/2)`; `a1`), weights multiply. Since affine maps commute with the rational
/// combination this is exact for any rational generatrix.
pub fn revolve_patch(
  axis: &RevolveAxis,
  a0: f64,
  a1: f64,
  generatrix: &RationalBezierCurve3,
) -> RationalBezierSurface {
  let half = 0.5 * (a1 - a0);
  let w = half.cos();
  let controls = [(a0, 1., 1.), (a0 + half, 1. / w, w), (a1, 1., 1.)];
  let mut cp = Vec::with_capacity(3 * generatrix.control_points.len());
  for q in &generatrix.control_points {
    let (qe, qw) = (q.to_euclid(), q.w);
    for (angle, scale, weight) in controls {
      cp.push(Vec4::from_euclid(
        axis.rotate_scaled(qe, angle, scale),
        qw * weight,
      ));
    }
  }
  RationalBezierSurface::new(2, generatrix.degree(), cp)
}

/// P space of a rotational surface: `u` angular per `u_map` (circular arcs), one patch per
/// pair of `u` arc and generatrix segment. `generatrix` lists `(cell index, segment)`.
pub fn revolve_param_space(
  axis: &RevolveAxis,
  u_map: PiecewiseParamMap,
  v_map: PiecewiseParamMap,
  generatrix: &[(usize, RationalBezierCurve3)],
) -> SurfaceParamSpace {
  let mut patches = Vec::with_capacity(u_map.segment_count() * generatrix.len());
  for (j, segment) in generatrix {
    for i in 0..u_map.segment_count() {
      patches.push(CellPatch {
        cell: (i, *j),
        surface: revolve_patch(axis, u_map.breaks[i], u_map.breaks[i + 1], segment),
      });
    }
  }
  SurfaceParamSpace {
    u_map,
    v_map,
    patches,
  }
}

/// Cells and generatrix segments for an analytic generatrix: one per segment of `v_map`,
/// `segment(b0, b1)` builds the generatrix over native `[b0, b1]`.
pub fn analytic_generatrix(
  v_map: &PiecewiseParamMap,
  segment: impl Fn(f64, f64) -> RationalBezierCurve3,
) -> Vec<(usize, RationalBezierCurve3)> {
  (0..v_map.segment_count())
    .map(|j| (j, segment(v_map.breaks[j], v_map.breaks[j + 1])))
    .collect()
}

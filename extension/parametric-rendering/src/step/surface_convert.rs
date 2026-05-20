use rendiation_algebra::*;
use rendiation_step_reader::entities::{
  self, Axis2Placement3d, BSplineSurfaceWithKnots, CurveAny, NonRationalBSplineSurface,
  RationalBSplineSurface, SurfaceAny,
};

use crate::curve3d::RationalBezierCurve3d;
use crate::step::curve_convert::{axis2_x_dir, cartesian_point_to_vec3, direction_to_vec3};
use crate::step::StepReadError;
use crate::surface::NurbsSurface;
use crate::surface::RationalBezierSurface;

/// The original (pre-decomposition) surface, used for projecting 3D points
/// into global parameter space in a single call — avoiding per-patch
/// projection loops.
#[derive(Debug, Clone)]
pub enum OriginalSurface {
  /// B-spline / NURBS surface (including single-span Bézier-form NURBS).
  Nurbs(NurbsSurface<f32>),
  /// Infinite plane parameterized by signed distances along u_dir/v_dir.
  Plane {
    origin: Vec3<f32>,
    u_dir: Vec3<f32>,
    v_dir: Vec3<f32>,
    normal: Vec3<f32>,
  },
  /// Cylinder: u = angle in [0, 2π), v = height in [v_min, v_max].
  Cylinder {
    origin: Vec3<f32>,
    axis: Vec3<f32>,
    x_dir: Vec3<f32>,
    y_dir: Vec3<f32>,
    radius: f32,
    v_range: (f32, f32),
  },
  /// Cone: u = angle in [0, 2π), v = height in [v_min, v_max].
  Cone {
    origin: Vec3<f32>,
    axis: Vec3<f32>,
    x_dir: Vec3<f32>,
    y_dir: Vec3<f32>,
    radius: f32,
    tan_semi_angle: f32,
    v_range: (f32, f32),
  },
  /// Sphere: u = longitude in [0, 2π), v = colatitude in [0, π].
  Sphere {
    center: Vec3<f32>,
    radius: f32,
    polar: Vec3<f32>,
    x_dir: Vec3<f32>,
    y_dir: Vec3<f32>,
  },
  /// Torus: u = major-circle angle in [0, 2π), v = minor-circle angle in [0, 2π).
  Torus {
    center: Vec3<f32>,
    axis: Vec3<f32>,
    x_dir: Vec3<f32>,
    y_dir: Vec3<f32>,
    major_radius: f32,
    minor_radius: f32,
  },
  /// Linear extrusion: u ∈ [0, 1] spans all curve segments, v ∈ [0, 1] is the
  /// extrusion fraction.
  Extrusion {
    curve_segments: Vec<RationalBezierCurve3d<f32>>,
    extrusion_dir: Vec3<f32>,
    extrusion_mag: f32,
  },
}

impl OriginalSurface {
  /// Project a 3D point onto the surface.
  ///
  /// Returns `(u_global, v_global, distance)` in global parameter space.
  /// The returned (u, v) lie in the same coordinate system used by
  /// `PatchParamRange::u_range` / `v_range`.
  pub fn project_point(
    &self,
    point: Vec3<f32>,
    grid: usize,
    tolerance: f32,
    max_iter: usize,
  ) -> Option<(f32, f32, f32)> {
    match self {
      OriginalSurface::Nurbs(n) => n.project_point(point, grid, tolerance, max_iter),
      OriginalSurface::Plane {
        origin,
        u_dir,
        v_dir,
        normal,
      } => Some(project_point_plane(point, *origin, *u_dir, *v_dir, *normal)),
      OriginalSurface::Cylinder {
        origin,
        axis,
        x_dir,
        y_dir,
        radius,
        v_range,
      } => Some(project_point_cylinder(
        point, *origin, *axis, *x_dir, *y_dir, *radius, *v_range,
      )),
      OriginalSurface::Cone {
        origin,
        axis,
        x_dir,
        y_dir,
        radius,
        tan_semi_angle,
        v_range,
      } => Some(project_point_cone(
        point,
        *origin,
        *axis,
        *x_dir,
        *y_dir,
        *radius,
        *tan_semi_angle,
        *v_range,
      )),
      OriginalSurface::Sphere {
        center,
        radius,
        polar,
        x_dir,
        y_dir,
      } => project_point_sphere(point, *center, *radius, *polar, *x_dir, *y_dir),
      OriginalSurface::Torus {
        center,
        axis,
        x_dir,
        y_dir,
        major_radius,
        minor_radius,
      } => project_point_torus(
        point,
        *center,
        *axis,
        *x_dir,
        *y_dir,
        *major_radius,
        *minor_radius,
      ),
      OriginalSurface::Extrusion {
        curve_segments,
        extrusion_dir,
        extrusion_mag,
      } => project_point_extrusion(point, curve_segments, *extrusion_dir, *extrusion_mag),
    }
  }
}

// ── Projection helpers for analytic surfaces ──────────────────────────

/// Project a 3D point onto a plane.
///
/// u/v are signed distances along u_dir/v_dir from origin.
/// normal must be unit-length and orthogonal to u_dir, v_dir.
pub fn project_point_plane(
  point: Vec3<f32>,
  origin: Vec3<f32>,
  u_dir: Vec3<f32>,
  v_dir: Vec3<f32>,
  normal: Vec3<f32>,
) -> (f32, f32, f32) {
  let t = (point - origin).dot(normal);
  let projected = point - t * normal;
  let d = projected - origin;
  (d.dot(u_dir), d.dot(v_dir), t.abs())
}

/// Project a 3D point onto a cylindrical surface.
///
/// u ∈ [0, 2π) is the angle around axis from x_dir, v ∈ [v_min, v_max]
/// is the signed distance along axis from origin.
pub fn project_point_cylinder(
  point: Vec3<f32>,
  origin: Vec3<f32>,
  axis: Vec3<f32>,
  x_dir: Vec3<f32>,
  y_dir: Vec3<f32>,
  radius: f32,
  v_range: (f32, f32),
) -> (f32, f32, f32) {
  let d = point - origin;
  let h = d.dot(axis);
  let v = h.clamp(v_range.0, v_range.1);
  let radial = d - h * axis;
  let mut u = radial.dot(y_dir).atan2(radial.dot(x_dir));
  if u < 0.0 {
    u += std::f32::consts::TAU;
  }
  let closest = origin + v * axis + radius * (u.cos() * x_dir + u.sin() * y_dir);
  let dist = (point - closest).length();
  (u, v, dist)
}

/// Project a 3D point onto a conical surface.
///
/// The cone is defined by its origin, axis, base radius, and semi-angle.
/// r(v) = radius - v * tan(semi_angle).
/// u ∈ [0, 2π) is the angle, v ∈ [v_min, v_max] is the height along axis.
pub fn project_point_cone(
  point: Vec3<f32>,
  origin: Vec3<f32>,
  axis: Vec3<f32>,
  x_dir: Vec3<f32>,
  y_dir: Vec3<f32>,
  radius: f32,
  tan_semi_angle: f32,
  v_range: (f32, f32),
) -> (f32, f32, f32) {
  let d = point - origin;
  let p_axial = d.dot(axis);
  let radial = d - p_axial * axis;
  let p_radial = radial.length();
  let cos2 = 1.0 / (1.0 + tan_semi_angle * tan_semi_angle);
  let mut v = cos2 * (p_axial - tan_semi_angle * (p_radial - radius));
  v = v.clamp(v_range.0, v_range.1);

  let r_v = radius - v * tan_semi_angle;
  let mut u = radial.dot(y_dir).atan2(radial.dot(x_dir));
  if u < 0.0 {
    u += std::f32::consts::TAU;
  }
  let closest = origin + v * axis + r_v * (u.cos() * x_dir + u.sin() * y_dir);
  let dist = (point - closest).length();
  (u, v, dist)
}

/// Project a 3D point onto a spherical surface.
///
/// u ∈ [0, 2π) is longitude from x_dir toward y_dir,
/// v ∈ [0, π] is colatitude from polar axis.
/// Returns None only when the point is at the sphere center (degenerate).
pub fn project_point_sphere(
  point: Vec3<f32>,
  center: Vec3<f32>,
  radius: f32,
  polar: Vec3<f32>,
  x_dir: Vec3<f32>,
  y_dir: Vec3<f32>,
) -> Option<(f32, f32, f32)> {
  let d = point - center;
  let len = d.length();
  if len < 1e-12 {
    return None;
  }
  let dir = d / len;
  let projected = center + radius * dir;
  let mut u = dir.dot(y_dir).atan2(dir.dot(x_dir));
  if u < 0.0 {
    u += std::f32::consts::TAU;
  }
  let v = dir.dot(polar).clamp(-1.0, 1.0).acos();
  let dist = (point - projected).length();
  Some((u, v, dist))
}
/// Uses the point's planar angle as initial guess, then refines with
/// golden-section search on the major-circle angle u. For a given u the
/// optimal minor-circle angle v is found analytically.
fn project_point_torus(
  point: Vec3<f32>,
  center: Vec3<f32>,
  axis: Vec3<f32>,
  x_dir: Vec3<f32>,
  y_dir: Vec3<f32>,
  major_radius: f32,
  minor_radius: f32,
) -> Option<(f32, f32, f32)> {
  // Helper: evaluate f(u) = min_v |p - S(u,v)|²
  let eval = |u: f32| -> (f32, f32, f32) {
    let cu = u.cos();
    let su = u.sin();
    let radial_dir = x_dir * cu + y_dir * su;
    let plane_center = center + major_radius * radial_dir;
    let d = point - plane_center;
    let v = d.dot(axis).atan2(d.dot(radial_dir));
    let cv = v.cos();
    let sv = v.sin();
    let closest = plane_center + minor_radius * (cv * radial_dir + sv * axis);
    let dist_sq = (point - closest).length2();
    (u, v, dist_sq)
  };

  // Coarse sampling
  let n_coarse = 16;
  let mut best_u = 0.0f32;
  let mut best_v = 0.0f32;
  let mut best_d2 = f32::MAX;
  for i in 0..n_coarse {
    let u = i as f32 * std::f32::consts::TAU / n_coarse as f32;
    let (_, v, d2) = eval(u);
    if d2 < best_d2 {
      best_d2 = d2;
      best_u = u;
      best_v = v;
    }
  }

  // Golden-section refinement around best_u
  let half_window = std::f32::consts::PI / n_coarse as f32;
  let mut a = best_u - half_window;
  let mut b = best_u + half_window;
  let inv_phi = 0.6180339887498949f32; // 1/φ
  let mut c = b - inv_phi * (b - a);
  let mut d = a + inv_phi * (b - a);

  let (_, _, mut fc_d2) = eval(c);
  let (_, _, mut fd_d2) = eval(d);

  for _ in 0..15 {
    if fc_d2 < fd_d2 {
      b = d;
      d = c;
      fd_d2 = fc_d2;
      c = b - inv_phi * (b - a);
      let (_, vc, d2c) = eval(c);
      fc_d2 = d2c;
      if fc_d2 < best_d2 {
        best_d2 = fc_d2;
        best_u = c;
        best_v = vc;
      }
    } else {
      a = c;
      c = d;
      fc_d2 = fd_d2;
      d = a + inv_phi * (b - a);
      let (_, vd, d2d) = eval(d);
      fd_d2 = d2d;
      if fd_d2 < best_d2 {
        best_d2 = fd_d2;
        best_u = d;
        best_v = vd;
      }
    }
  }

  // Normalize u to [0, 2π)
  best_u = best_u.rem_euclid(std::f32::consts::TAU);
  best_v = best_v.rem_euclid(std::f32::consts::TAU);

  Some((best_u, best_v, best_d2.sqrt()))
}

/// Project a 3D point onto an extrusion surface.
///
/// The surface is S(u,v) = C(u) + v * extrusion_vec, where C is the
/// generator curve composed of multiple Bezier segments.
///
/// Global parameters: u ∈ [0, 1] spans all segments uniformly,
/// v ∈ [0, 1] is the extrusion fraction.
fn project_point_extrusion(
  point: Vec3<f32>,
  curve_segments: &[RationalBezierCurve3d<f32>],
  extrusion_dir: Vec3<f32>,
  extrusion_mag: f32,
) -> Option<(f32, f32, f32)> {
  let n_seg = curve_segments.len() as f32;
  if n_seg < 1.0 {
    return None;
  }

  let mut best_u_global = 0.0f32;
  let mut best_v = 0.0f32;
  let mut best_d2 = f32::MAX;

  for (seg_idx, seg) in curve_segments.iter().enumerate() {
    // Dense sample the 3D curve
    let n_samples = 64;
    let mut seg_best_t = 0.0f32;
    let mut seg_best_d2 = f32::MAX;
    let mut seg_best_v = 0.0f32;

    for s in 0..=n_samples {
      let t = s as f32 / n_samples as f32;
      let c = seg.evaluate(t);
      let d = point - c;
      let v = (d.dot(extrusion_dir) / extrusion_mag).clamp(0.0, 1.0);
      let closest = c + v * extrusion_dir * extrusion_mag;
      let d2 = (point - closest).length2();
      if d2 < seg_best_d2 {
        seg_best_d2 = d2;
        seg_best_t = t;
        seg_best_v = v;
      }
    }

    if seg_best_d2 < best_d2 {
      best_d2 = seg_best_d2;
      best_u_global = (seg_idx as f32 + seg_best_t) / n_seg;
      best_v = seg_best_v;
    }
  }

  Some((best_u_global, best_v, best_d2.sqrt()))
}

/// A Bezier patch paired with its parameter range in the original surface's
/// parameter space. Used for remapping trim curves after surface decomposition.
#[derive(Debug, Clone)]
pub struct PatchParamRange {
  pub surface: RationalBezierSurface<f32>,
  /// u range of this patch in the original surface's parameter space
  pub u_range: (f32, f32),
  /// v range of this patch in the original surface's parameter space
  pub v_range: (f32, f32),
}

// ── B-spline / NURBS ──────────────────────────────────────────────────

/// Convert a B-spline surface (unweighted) to Bezier patches via knot insertion.
///
/// Each patch covers one knot span with the parameter range recorded.
/// Control points are a 2D grid: `control_points[row][col]` where `row` = v-index,
/// `col` = u-index.
pub fn convert_bspline_surface_to_bezier_patches(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
  u_knots: Vec<f32>,
  v_knots: Vec<f32>,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let v_count = control_points.len();
  let u_count = control_points[0].len();

  let u_knots = fix_knot_length(u_knots, u_count, u_degree, "u");
  let v_knots = fix_knot_length(v_knots, v_count, v_degree, "v");

  // Flatten to row-major Vec<Vec4<f32>> (all weights = 1)
  let flat_cp: Vec<Vec4<f32>> = control_points
    .iter()
    .flat_map(|row| row.iter().map(|p| Vec4::new(p.x, p.y, p.z, 1.0)))
    .collect();

  let nurbs = crate::surface::NurbsSurface::new(
    flat_cp,
    u_count,
    v_count,
    u_degree,
    v_degree,
    u_knots.clone(),
    v_knots.clone(),
  );

  let patches = nurbs.to_bezier_patches();

  let result = build_patch_param_ranges(patches, &u_knots, &v_knots, u_degree, v_degree);
  (result, OriginalSurface::Nurbs(nurbs))
}

/// Convert a rational B-spline surface to Bezier patches.
pub fn convert_rational_bspline_surface_to_bezier_patches(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
  weights: Vec<Vec<f32>>,
  u_knots: Vec<f32>,
  v_knots: Vec<f32>,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let v_count = control_points.len();
  let u_count = control_points[0].len();

  let u_knots = fix_knot_length(u_knots, u_count, u_degree, "u");
  let v_knots = fix_knot_length(v_knots, v_count, v_degree, "v");

  let weights_owned = weights;
  let flat_cp: Vec<Vec4<f32>> = control_points
    .iter()
    .enumerate()
    .flat_map(|(vi, row)| {
      let w_row = &weights_owned[vi];
      row.iter().enumerate().map(move |(ui, p)| {
        let w = w_row[ui];
        Vec4::new(p.x * w, p.y * w, p.z * w, w)
      })
    })
    .collect();

  let nurbs = crate::surface::NurbsSurface::new(
    flat_cp,
    u_count,
    v_count,
    u_degree,
    v_degree,
    u_knots.clone(),
    v_knots.clone(),
  );

  let patches = nurbs.to_bezier_patches();
  let result = build_patch_param_ranges(patches, &u_knots, &v_knots, u_degree, v_degree);
  (result, OriginalSurface::Nurbs(nurbs))
}

/// Build PatchParamRange from the knot vectors and decomposition result.
fn build_patch_param_ranges(
  patches: Vec<Vec<RationalBezierSurface<f32>>>,
  u_knots: &[f32],
  v_knots: &[f32],
  u_degree: usize,
  v_degree: usize,
) -> Vec<PatchParamRange> {
  let v_segments = patches.len();
  let u_segments = patches.first().map(|r| r.len()).unwrap_or(0);

  if u_segments == 0 {
    return Vec::new();
  }

  // Build u_range and v_range boundaries from the actual patch count.
  // After knot insertion, patch count may differ from simple knot-span counting.
  let u_bounds = build_span_bounds_from_patches(u_knots, u_degree, u_segments);
  let v_bounds = build_span_bounds_from_patches(v_knots, v_degree, v_segments);

  let mut result = Vec::new();
  for (vi, row) in patches.into_iter().enumerate() {
    for (ui, surface) in row.into_iter().enumerate() {
      if ui + 1 < u_bounds.len() && vi + 1 < v_bounds.len() {
        result.push(PatchParamRange {
          surface,
          u_range: (u_bounds[ui], u_bounds[ui + 1]),
          v_range: (v_bounds[vi], v_bounds[vi + 1]),
        });
      }
    }
  }
  result
}

fn build_span_bounds_from_patches(knots: &[f32], degree: usize, segments: usize) -> Vec<f32> {
  if segments <= 1 {
    return vec![knots[degree], knots[knots.len() - degree - 1]];
  }
  // For multiple segments, use the distinct interior knots plus endpoints
  let distinct = distinct_interior_knots(knots, degree);
  let mut bounds = Vec::with_capacity(segments + 1);
  bounds.push(knots[degree]);
  for k in &distinct {
    bounds.push(*k);
  }
  bounds.push(knots[knots.len() - degree - 1]);

  // If the computed bounds don't match the expected segment count
  // (due to knot insertion changing the patch count), pad or trim.
  while bounds.len() < segments + 1 {
    // Pad with the last value
    let last = *bounds.last().unwrap();
    bounds.push(last);
  }
  bounds.truncate(segments + 1);
  bounds
}

fn distinct_interior_knots(knots: &[f32], degree: usize) -> Vec<f32> {
  if knots.len() <= 2 * (degree + 1) {
    return vec![];
  }
  let interior = &knots[degree + 1..knots.len() - degree - 1];
  let mut result = Vec::new();
  let mut prev: Option<f32> = None;
  for &k in interior {
    if prev != Some(k) {
      result.push(k);
      prev = Some(k);
    }
  }
  result
}

#[allow(dead_code)]
fn build_span_bounds(knots: &[f32], degree: usize, segments: usize) -> Vec<f32> {
  let mut bounds = Vec::with_capacity(segments + 1);
  if segments == 1 {
    bounds.push(knots[degree]);
    bounds.push(knots[knots.len() - degree - 1]);
  } else {
    let distinct = distinct_interior_knots(knots, degree);
    bounds.push(knots[degree]);
    for k in &distinct {
      bounds.push(*k);
    }
    bounds.push(knots[knots.len() - degree - 1]);
  }
  bounds
}

/// Convert a Bezier surface (no knots) to a single patch.
pub fn convert_bezier_surface_to_patch(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let u_count = control_points[0].len();
  let v_count = control_points.len();

  let flat_cp: Vec<Vec4<f32>> = control_points
    .into_iter()
    .flat_map(|row| row.into_iter().map(|p| Vec4::new(p.x, p.y, p.z, 1.0)))
    .collect();

  let surface = RationalBezierSurface::new(flat_cp.clone(), u_degree, v_degree);
  let patch = PatchParamRange {
    surface,
    u_range: (0.0, 1.0),
    v_range: (0.0, 1.0),
  };

  // Build a single-span NurbsSurface for unified projection.
  let n_u_knots = u_count + u_degree + 1;
  let mut u_knots = vec![0.0f32; n_u_knots];
  for i in u_degree + 1..n_u_knots {
    u_knots[i] = 1.0;
  }
  let n_v_knots = v_count + v_degree + 1;
  let mut v_knots = vec![0.0f32; n_v_knots];
  for i in v_degree + 1..n_v_knots {
    v_knots[i] = 1.0;
  }
  let nurbs = NurbsSurface::new(
    flat_cp, u_count, v_count, u_degree, v_degree, u_knots, v_knots,
  );

  (vec![patch], OriginalSurface::Nurbs(nurbs))
}

// ── Plane ─────────────────────────────────────────────────────────────

/// Convert a plane to a single degree-1 Bezier patch.
///
/// The patch is a bilinear quadrilateral in the plane. The corners are
/// chosen at (±1, ±1) in the plane's local coordinates — this gives a
/// finite patch for projection. Trim curves will determine the actual
/// extent.
pub fn convert_plane_to_bezier_patch(
  position: &Axis2Placement3d,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let center = cartesian_point_to_vec3(&position.location);
  let normal = direction_to_vec3(&position.axis);
  let x_dir = axis2_x_dir(position);
  let y_dir = normal.cross(x_dir).normalize();

  let extent = 1.0;
  let corners = [
    center - x_dir * extent - y_dir * extent,
    center + x_dir * extent - y_dir * extent,
    center - x_dir * extent + y_dir * extent,
    center + x_dir * extent + y_dir * extent,
  ];

  let cp: Vec<Vec4<f32>> = corners
    .iter()
    .map(|&p| Vec4::new(p.x, p.y, p.z, 1.0))
    .collect();

  let surface = RationalBezierSurface::new(cp, 1, 1);
  let patch = PatchParamRange {
    surface,
    u_range: (-extent, extent),
    v_range: (-extent, extent),
  };
  let orig = OriginalSurface::Plane {
    origin: center,
    u_dir: x_dir,
    v_dir: y_dir,
    normal,
  };
  (vec![patch], orig)
}

// ── Cylinder ──────────────────────────────────────────────────────────

/// Convert a cylindrical surface to 4 biquadratic rational Bezier patches.
///
/// u direction: 4 × 90° circular arcs (degree 2 rational)
/// v direction: degree 1 (linear extrusion)
///
/// v_range specifies the height extent along the cylinder axis. If `None`,
/// defaults to [0.0, 1.0].
pub fn convert_cylinder_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
  v_range: Option<(f64, f64)>,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let center = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();
  let x_dir = axis2_x_dir(position);
  let y_dir = axis.cross(x_dir).normalize();

  let r = radius as f32;
  let (v0, v1) = v_range.unwrap_or((0.0, 1.0));
  let v0 = v0 as f32;
  let v1 = v1 as f32;

  let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2;
  let mut patches = Vec::with_capacity(4);

  for i in 0..4 {
    let angle0 = i as f32 * std::f32::consts::FRAC_PI_2;
    let angle1 = (i + 1) as f32 * std::f32::consts::FRAC_PI_2;
    let angle_mid = (angle0 + angle1) * 0.5;

    let cos0 = angle0.cos();
    let sin0 = angle0.sin();
    let cos1 = angle1.cos();
    let sin1 = angle1.sin();
    let cos_mid = angle_mid.cos();
    let sin_mid = angle_mid.sin();

    let p0_bot = center + (x_dir * cos0 + y_dir * sin0) * r + axis * v0;
    let p1_bot = center + (x_dir * cos_mid + y_dir * sin_mid) * r / w_mid + axis * v0;
    let p2_bot = center + (x_dir * cos1 + y_dir * sin1) * r + axis * v0;

    let p0_top = center + (x_dir * cos0 + y_dir * sin0) * r + axis * v1;
    let p1_top = center + (x_dir * cos_mid + y_dir * sin_mid) * r / w_mid + axis * v1;
    let p2_top = center + (x_dir * cos1 + y_dir * sin1) * r + axis * v1;

    let cp = vec![
      Vec4::new(p0_bot.x, p0_bot.y, p0_bot.z, 1.0),
      Vec4::new(p1_bot.x * w_mid, p1_bot.y * w_mid, p1_bot.z * w_mid, w_mid),
      Vec4::new(p2_bot.x, p2_bot.y, p2_bot.z, 1.0),
      Vec4::new(p0_top.x, p0_top.y, p0_top.z, 1.0),
      Vec4::new(p1_top.x * w_mid, p1_top.y * w_mid, p1_top.z * w_mid, w_mid),
      Vec4::new(p2_top.x, p2_top.y, p2_top.z, 1.0),
    ];

    let surface = RationalBezierSurface::new(cp, 2, 1);
    patches.push(PatchParamRange {
      surface,
      u_range: (angle0, angle1),
      v_range: (v0, v1),
    });
  }

  let orig = OriginalSurface::Cylinder {
    origin: center,
    axis,
    x_dir,
    y_dir,
    radius: r,
    v_range: (v0, v1),
  };
  (patches, orig)
}

// ── Cone ──────────────────────────────────────────────────────────────

/// Convert a conical surface to 4 biquadratic rational Bezier patches.
///
/// The cone is defined by its position (axis + location), base radius at the
/// location point, and semi_angle. The apex is at a distance of
/// `radius / tan(semi_angle)` along the axis from the location point.
///
/// v_range specifies the distance along the axis. Defaults to [0.0, 1.0].
pub fn convert_cone_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
  semi_angle: f64,
  v_range: Option<(f64, f64)>,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let center = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();
  let x_dir = axis2_x_dir(position);
  let y_dir = axis.cross(x_dir).normalize();

  let r = radius as f32;
  let (v0, v1) = v_range.unwrap_or((0.0, 1.0));
  let v0 = v0 as f32;
  let v1 = v1 as f32;

  let tan_angle = (semi_angle as f32).tan();
  let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2;
  let mut patches = Vec::with_capacity(4);

  for i in 0..4 {
    let angle0 = i as f32 * std::f32::consts::FRAC_PI_2;
    let angle1 = (i + 1) as f32 * std::f32::consts::FRAC_PI_2;
    let angle_mid = (angle0 + angle1) * 0.5;

    let cos0 = angle0.cos();
    let sin0 = angle0.sin();
    let cos1 = angle1.cos();
    let sin1 = angle1.sin();
    let cos_mid = angle_mid.cos();
    let sin_mid = angle_mid.sin();

    let r0 = r - v0 * tan_angle;
    let r1 = r - v1 * tan_angle;

    let p0_bot = center + (x_dir * cos0 + y_dir * sin0) * r0 + axis * v0;
    let p1_bot = center + (x_dir * cos_mid + y_dir * sin_mid) * r0 / w_mid + axis * v0;
    let p2_bot = center + (x_dir * cos1 + y_dir * sin1) * r0 + axis * v0;

    let p0_top = center + (x_dir * cos0 + y_dir * sin0) * r1 + axis * v1;
    let p1_top = center + (x_dir * cos_mid + y_dir * sin_mid) * r1 / w_mid + axis * v1;
    let p2_top = center + (x_dir * cos1 + y_dir * sin1) * r1 + axis * v1;

    let cp = vec![
      Vec4::new(p0_bot.x, p0_bot.y, p0_bot.z, 1.0),
      Vec4::new(p1_bot.x * w_mid, p1_bot.y * w_mid, p1_bot.z * w_mid, w_mid),
      Vec4::new(p2_bot.x, p2_bot.y, p2_bot.z, 1.0),
      Vec4::new(p0_top.x, p0_top.y, p0_top.z, 1.0),
      Vec4::new(p1_top.x * w_mid, p1_top.y * w_mid, p1_top.z * w_mid, w_mid),
      Vec4::new(p2_top.x, p2_top.y, p2_top.z, 1.0),
    ];

    let surface = RationalBezierSurface::new(cp, 2, 1);
    patches.push(PatchParamRange {
      surface,
      u_range: (angle0, angle1),
      v_range: (v0, v1),
    });
  }

  let orig = OriginalSurface::Cone {
    origin: center,
    axis,
    x_dir,
    y_dir,
    radius: r,
    tan_semi_angle: tan_angle,
    v_range: (v0, v1),
  };
  (patches, orig)
}

// ── Sphere ────────────────────────────────────────────────────────────

/// Convert a spherical surface to 8 biquadratic rational Bezier patches.
///
/// The sphere is split into 2 hemispheres (north and south), each split into
/// 4 quadrants around the polar axis. Each patch is a biquadratic rational
/// Bezier (3×3 control points, degree 2 in both directions).
///
/// u ∈ [0, 2π]: longitude, v ∈ [0, π]: colatitude (angle from north pole)
pub fn convert_sphere_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let center = cartesian_point_to_vec3(&position.location);
  let polar = direction_to_vec3(&position.axis).normalize();
  let x_dir = axis2_x_dir(position);
  let y_dir = polar.cross(x_dir).normalize();

  let r = radius as f32;
  let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2;
  let w_sq = w_mid * w_mid; // 0.5
  let mut patches = Vec::with_capacity(8);

  // Build sphere using 8 biquadratic rational Bezier octants.
  //
  // Each octant is bounded by a 90°×90° region in (longitude, colatitude).
  // The control points follow the standard CAGD formula: corners are on the
  // sphere (weight=1), edge midpoints are at tangent intersections of adjacent
  // corners (weight=√2/2), and the center CP is at the cube corner (weight=1/2).
  //
  // The pole row degenerates to a single point with varying weights, ensuring
  // the u-isocurve collapses correctly.

  // Reference octant: longitude [0, π/2], colatitude [0, π/2] (north pole → equator)
  let ref_cps: [Vec4<f32>; 9] = [
    // Row 0 — pole edge (colatitude 0, all u map to north pole)
    Vec4::new(0.0, 0.0, r, 1.0),           //(0,0): pole
    Vec4::new(0.0, 0.0, r * w_mid, w_mid), //(1,0): pole, reduced weight
    Vec4::new(0.0, 0.0, r, 1.0),           //(2,0): pole
    // Row 1 — mid-latitude (colatitude π/4)
    Vec4::new(r * w_mid, 0.0, r * w_mid, w_mid), //(0,1): XZ tangent intersec
    Vec4::new(r * w_sq, r * w_sq, r * w_sq, w_sq), //(1,1): cube corner
    Vec4::new(0.0, r * w_mid, r * w_mid, w_mid), //(2,1): YZ tangent intersec
    // Row 2 — equator edge (colatitude π/2)
    Vec4::new(r, 0.0, 0.0, 1.0),                 //(0,2): equator at u=0
    Vec4::new(r * w_mid, r * w_mid, 0.0, w_mid), //(1,2): equator tangent
    Vec4::new(0.0, r, 0.0, 1.0),                 //(2,2): equator at u=π/2
  ];

  let rotate_z = |p: Vec4<f32>, q: usize| -> Vec4<f32> {
    match q {
      0 => p, // identity
      1 => {
        // (x,y,z) → (-y,x,z)
        Vec4::new(-p.y, p.x, p.z, p.w)
      }
      2 => {
        // (x,y,z) → (-x,-y,z)
        Vec4::new(-p.x, -p.y, p.z, p.w)
      }
      3 => {
        // (x,y,z) → (y,-x,z)
        Vec4::new(p.y, -p.x, p.z, p.w)
      }
      _ => unreachable!(),
    }
  };

  for h in 0..2 {
    for q in 0..4 {
      let u0 = q as f32 * std::f32::consts::FRAC_PI_2;
      let u1 = (q + 1) as f32 * std::f32::consts::FRAC_PI_2;
      let v0 = h as f32 * std::f32::consts::FRAC_PI_2;
      let v1 = (h + 1) as f32 * std::f32::consts::FRAC_PI_2;

      let local_cps: Vec<Vec4<f32>> = ref_cps
        .iter()
        .map(|&c| {
          let rot = rotate_z(c, q);
          if h == 1 {
            // South hemisphere: negate Z
            Vec4::new(rot.x, rot.y, -rot.z, rot.w)
          } else {
            rot
          }
        })
        .collect();

      // Transform from local (X,Y,Z) to global coordinates.
      // The local frame: X=x_dir, Y=y_dir, Z=polar, origin=center.
      let global_cp: Vec<Vec4<f32>> = local_cps
        .into_iter()
        .map(|c| {
          let local_p = Vec3::new(c.x / c.w, c.y / c.w, c.z / c.w);
          let global_p = center + x_dir * local_p.x + y_dir * local_p.y + polar * local_p.z;
          Vec4::new(global_p.x * c.w, global_p.y * c.w, global_p.z * c.w, c.w)
        })
        .collect();

      let surface = RationalBezierSurface::new(global_cp, 2, 2);
      patches.push(PatchParamRange {
        surface,
        u_range: (u0, u1),
        v_range: (v0, v1),
      });
    }
  }

  let orig = OriginalSurface::Sphere {
    center,
    radius: r,
    polar,
    x_dir,
    y_dir,
  };
  (patches, orig)
}

/// Convert a toroidal surface to 16 biquadratic rational Bezier patches.
///
/// u direction: 4 × 90° segments around the big circle (major radius)
/// v direction: 4 × 90° segments around the small circle (minor radius)
///
/// Each patch is a biquadratic rational Bezier (3×3 grid, degree 2).
///
/// u ∈ [0, 2π]: big circle angle, v ∈ [0, 2π]: small circle angle
pub fn convert_torus_to_bezier_patches(
  position: &Axis2Placement3d,
  major_radius: f64,
  minor_radius: f64,
) -> (Vec<PatchParamRange>, OriginalSurface) {
  let center = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();
  let x_dir = axis2_x_dir(position);
  let y_dir = axis.cross(x_dir).normalize();

  let r_major = major_radius as f32;
  let r_minor = minor_radius as f32;
  let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2;

  let mut patches = Vec::with_capacity(16);

  for vi in 0..4 {
    let v0 = vi as f32 * std::f32::consts::FRAC_PI_2;
    let v1 = (vi + 1) as f32 * std::f32::consts::FRAC_PI_2;
    let v_mid = (v0 + v1) * 0.5;

    for ui in 0..4 {
      let u0 = ui as f32 * std::f32::consts::FRAC_PI_2;
      let u1 = (ui + 1) as f32 * std::f32::consts::FRAC_PI_2;
      let u_mid = (u0 + u1) * 0.5;

      // A point on the torus at (u_big, v_small):
      // P(u, v) = center
      //   + (x_dir * cos(u) + y_dir * sin(u)) * (R + r * cos(v))
      //   + axis * r * sin(v)
      let torus_point = |u: f32, v: f32| -> Vec3<f32> {
        let radial_dir = x_dir * u.cos() + y_dir * u.sin();
        let radial_dist = r_major + r_minor * v.cos();
        center + radial_dir * radial_dist + axis * r_minor * v.sin()
      };

      let p00 = torus_point(u0, v0);
      let p02 = torus_point(u1, v0);
      let p20 = torus_point(u0, v1);
      let p22 = torus_point(u1, v1);

      // Edge midpoints
      let p01 = torus_point(u_mid, v0) * (1.0 / w_mid);
      let p10 = torus_point(u0, v_mid) * (1.0 / w_mid);
      let p21 = torus_point(u1, v_mid) * (1.0 / w_mid);
      let p12 = torus_point(u_mid, v1) * (1.0 / w_mid);

      // Center point
      let p11_w = w_mid * w_mid;
      let p11 = torus_point(u_mid, v_mid) * (1.0 / p11_w);

      let cp = vec![
        // v = v0 row
        Vec4::new(p00.x, p00.y, p00.z, 1.0),
        Vec4::new(p01.x * w_mid, p01.y * w_mid, p01.z * w_mid, w_mid),
        Vec4::new(p02.x, p02.y, p02.z, 1.0),
        // v = v_mid row
        Vec4::new(p10.x * w_mid, p10.y * w_mid, p10.z * w_mid, w_mid),
        Vec4::new(p11.x * p11_w, p11.y * p11_w, p11.z * p11_w, p11_w),
        Vec4::new(p12.x * w_mid, p12.y * w_mid, p12.z * w_mid, w_mid),
        // v = v1 row
        Vec4::new(p20.x, p20.y, p20.z, 1.0),
        Vec4::new(p21.x * w_mid, p21.y * w_mid, p21.z * w_mid, w_mid),
        Vec4::new(p22.x, p22.y, p22.z, 1.0),
      ];

      let surface = RationalBezierSurface::new(cp, 2, 2);
      patches.push(PatchParamRange {
        surface,
        u_range: (u0, u1),
        v_range: (v0, v1),
      });
    }
  }

  let orig = OriginalSurface::Torus {
    center,
    axis,
    x_dir,
    y_dir,
    major_radius: major_radius as f32,
    minor_radius: minor_radius as f32,
  };
  (patches, orig)
}

// ── Surface of Linear Extrusion ───────────────────────────────────────

/// Convert a surface of linear extrusion to Bezier patches.
///
/// Takes a generator curve and extrudes it linearly. Each Bezier segment of
/// the curve becomes one patch (degree in u = curve degree, degree in v = 1).
pub fn convert_extrusion_surface_to_bezier_patches(
  swept_curve: &CurveAny,
  extrusion_axis: &entities::Vector,
) -> Result<(Vec<PatchParamRange>, OriginalSurface), StepReadError> {
  let curve_segments = crate::step::curve_convert::convert_any_curve_to_bezier(swept_curve)?;
  let dir = direction_to_vec3(&extrusion_axis.orientation);
  let mag = extrusion_axis.magnitude as f32;
  let extrusion_vec = dir * mag;

  let n_seg = curve_segments.len() as f32;
  let mut patches = Vec::with_capacity(curve_segments.len());
  for (si, seg) in curve_segments.iter().enumerate() {
    let degree = seg.degree();
    let cp_count = degree + 1;

    let bottom_cp = seg.control_points().to_vec();
    let top_cp: Vec<Vec4<f32>> = bottom_cp
      .iter()
      .map(|&cp4| {
        let p = Vec3::new(cp4.x / cp4.w, cp4.y / cp4.w, cp4.z / cp4.w);
        let p_extruded = p + extrusion_vec;
        Vec4::new(
          p_extruded.x * cp4.w,
          p_extruded.y * cp4.w,
          p_extruded.z * cp4.w,
          cp4.w,
        )
      })
      .collect();

    let mut all_cp = Vec::with_capacity(cp_count * 2);
    for (&bot, _) in bottom_cp.iter().zip(top_cp.iter()) {
      all_cp.push(bot);
    }
    for (_, &top) in bottom_cp.iter().zip(top_cp.iter()) {
      all_cp.push(top);
    }

    let surface = RationalBezierSurface::new(all_cp, degree, 1);
    // Each patch covers u ∈ [si/N, (si+1)/N] in global parameter space
    let u0 = si as f32 / n_seg;
    let u1 = (si + 1) as f32 / n_seg;
    patches.push(PatchParamRange {
      surface,
      u_range: (u0, u1),
      v_range: (0.0, 1.0),
    });
  }

  let orig = OriginalSurface::Extrusion {
    curve_segments,
    extrusion_dir: dir,
    extrusion_mag: mag,
  };
  Ok((patches, orig))
}

// ── Top-level dispatch ────────────────────────────────────────────────

/// Convert any STEP surface to Bezier patches with parameter range info,
/// together with an `OriginalSurface` for single-call point projection.
pub fn convert_any_surface_to_bezier_patches(
  surface: &SurfaceAny,
) -> Result<(Vec<PatchParamRange>, OriginalSurface), StepReadError> {
  match surface {
    SurfaceAny::BSplineSurfaceWithKnots(b) => {
      let u_degree = b.u_degree as usize;
      let v_degree = b.v_degree as usize;
      let control_points: Vec<Vec<Vec3<f32>>> = b
        .control_points_list
        .iter()
        .map(|row| row.iter().map(cartesian_point_to_vec3).collect())
        .collect();
      let u_knots = expand_surface_knots(&b.u_knots, &b.u_multiplicities);
      let v_knots = expand_surface_knots(&b.v_knots, &b.v_multiplicities);
      Ok(convert_bspline_surface_to_bezier_patches(
        u_degree,
        v_degree,
        control_points,
        u_knots,
        v_knots,
      ))
    }
    SurfaceAny::BezierSurface(b) => {
      let control_points: Vec<Vec<Vec3<f32>>> = b
        .control_points_list
        .iter()
        .map(|row| row.iter().map(cartesian_point_to_vec3).collect())
        .collect();
      let u_degree = b.u_degree as usize;
      let v_degree = b.v_degree as usize;
      Ok(convert_bezier_surface_to_patch(
        u_degree,
        v_degree,
        control_points,
      ))
    }
    SurfaceAny::RationalBSplineSurface(r) => {
      let (u_degree, v_degree, control_points, weights, u_knots, v_knots) =
        extract_rational_bspline_surface_data(r)?;
      Ok(convert_rational_bspline_surface_to_bezier_patches(
        u_degree,
        v_degree,
        control_points,
        weights,
        u_knots,
        v_knots,
      ))
    }
    SurfaceAny::Plane(p) => Ok(convert_plane_to_bezier_patch(&p.position)),
    SurfaceAny::CylindricalSurface(c) => Ok(convert_cylinder_to_bezier_patches(
      &c.position,
      c.radius,
      None,
    )),
    SurfaceAny::ConicalSurface(c) => Ok(convert_cone_to_bezier_patches(
      &c.position,
      c.radius,
      c.semi_angle,
      None,
    )),
    SurfaceAny::SphericalSurface(s) => Ok(convert_sphere_to_bezier_patches(&s.position, s.radius)),
    SurfaceAny::ToroidalSurface(t) => Ok(convert_torus_to_bezier_patches(
      &t.position,
      t.major_radius,
      t.minor_radius,
    )),
    SurfaceAny::SurfaceOfLinearExtrusion(e) => {
      convert_extrusion_surface_to_bezier_patches(&e.swept_curve, &e.extrusion_axis)
    }
    SurfaceAny::SurfaceOfRevolution(_) => Err(StepReadError::UnsupportedSurface {
      entity_type: "SurfaceOfRevolution",
      id: 0,
    }),
    SurfaceAny::OffsetSurface(_) => Err(StepReadError::UnsupportedSurface {
      entity_type: "OffsetSurface",
      id: 0,
    }),
  }
}

// --- Internal helpers ---

/// Ensure knot vector length matches `count + degree + 1`.
///
/// STEP exporters disagree on how knot multiplicities are stored:
/// - Some include endpoint multiplicities of `degree + 1` (clamped B-spline),
///   producing exactly `count + degree + 1` knots after expansion.
/// - Some omit endpoint multiplicities entirely, giving too few knots.
/// - Some store endpoint multiplicities of `degree + 2`, giving too many.
///
/// Both cases are defensive: too-short knots are padded by repeating the last
/// value; too-long knots are truncated.
fn fix_knot_length(mut knots: Vec<f32>, count: usize, degree: usize, dir: &str) -> Vec<f32> {
  let expected = count + degree + 1;
  if knots.len() != expected {
    crate::step::step_dbg!(
      "step: {}--knots length mismatch: {} != {} (count={} + degree={} + 1)",
      dir,
      knots.len(),
      expected,
      count,
      degree
    );
    if knots.len() < expected {
      let last = *knots.last().unwrap_or(&0.0);
      knots.resize(expected, last);
    } else {
      knots.truncate(expected);
    }
  }
  knots
}

fn expand_surface_knots(knots: &[f64], multiplicities: &[i64]) -> Vec<f32> {
  let mut result = Vec::new();
  for (&k, &m) in knots.iter().zip(multiplicities.iter()) {
    for _ in 0..m {
      result.push(k as f32);
    }
  }
  result
}

fn extract_rational_bspline_surface_data(
  r: &RationalBSplineSurface,
) -> Result<
  (
    usize,
    usize,
    Vec<Vec<Vec3<f32>>>,
    Vec<Vec<f32>>,
    Vec<f32>,
    Vec<f32>,
  ),
  StepReadError,
> {
  match &r.non_rational_b_spline_surface {
    NonRationalBSplineSurface::BSplineSurfaceWithKnots(b) => {
      extract_surface_from_bspline(b, &r.weights_data)
    }
    NonRationalBSplineSurface::BezierSurface(b) => {
      let u_degree = b.u_degree as usize;
      let v_degree = b.v_degree as usize;
      let control_points: Vec<Vec<Vec3<f32>>> = b
        .control_points_list
        .iter()
        .map(|row| row.iter().map(cartesian_point_to_vec3).collect())
        .collect();
      let v_count = control_points.len();
      let u_count = control_points[0].len();
      let weights: Vec<Vec<f32>> = if r.weights_data.is_empty() {
        vec![vec![1.0; u_count]; v_count]
      } else {
        r.weights_data
          .iter()
          .map(|row| row.iter().map(|&w| w as f32).collect())
          .collect()
      };
      let n_knots = u_count + u_degree + 1;
      let mut u_knots = vec![0.0; n_knots];
      for i in 0..u_degree + 1 {
        u_knots[i] = 0.0;
        u_knots[n_knots - 1 - i] = 1.0;
      }
      let n_v_knots = v_count + v_degree + 1;
      let mut v_knots = vec![0.0; n_v_knots];
      for i in 0..v_degree + 1 {
        v_knots[i] = 0.0;
        v_knots[n_v_knots - 1 - i] = 1.0;
      }
      Ok((
        u_degree,
        v_degree,
        control_points,
        weights,
        u_knots,
        v_knots,
      ))
    }
    NonRationalBSplineSurface::QuasiUniformSurface(b)
    | NonRationalBSplineSurface::UniformSurface(b) => {
      extract_surface_from_bspline(b, &r.weights_data)
    }
  }
}

fn extract_surface_from_bspline(
  b: &BSplineSurfaceWithKnots,
  weights_data: &[Vec<f64>],
) -> Result<
  (
    usize,
    usize,
    Vec<Vec<Vec3<f32>>>,
    Vec<Vec<f32>>,
    Vec<f32>,
    Vec<f32>,
  ),
  StepReadError,
> {
  let u_degree = b.u_degree as usize;
  let v_degree = b.v_degree as usize;
  let control_points: Vec<Vec<Vec3<f32>>> = b
    .control_points_list
    .iter()
    .map(|row| row.iter().map(cartesian_point_to_vec3).collect())
    .collect();
  let v_count = control_points.len();
  let u_count = control_points[0].len();
  let weights: Vec<Vec<f32>> = if weights_data.is_empty() {
    vec![vec![1.0; u_count]; v_count]
  } else {
    weights_data
      .iter()
      .map(|row| row.iter().map(|&w| w as f32).collect())
      .collect()
  };
  let u_knots = expand_surface_knots(&b.u_knots, &b.u_multiplicities);
  let v_knots = expand_surface_knots(&b.v_knots, &b.v_multiplicities);
  Ok((
    u_degree,
    v_degree,
    control_points,
    weights,
    u_knots,
    v_knots,
  ))
}

#[cfg(test)]
mod tests {
  use rendiation_step_reader::entities::{CartesianPoint, Direction};

  use super::*;

  #[test]
  fn cylinder_patch_evaluates_on_cylinder() {
    // Create a simple cylinder along Z axis
    let position = Axis2Placement3d {
      label: String::new(),
      location: CartesianPoint {
        label: String::new(),
        coordinates: vec![0.0, 0.0, 0.0],
      },
      axis: Direction {
        label: String::new(),
        direction_ratios: vec![0.0, 0.0, 1.0],
      },
      ref_direction: Some(Direction {
        label: String::new(),
        direction_ratios: vec![1.0, 0.0, 0.0],
      }),
    };

    let (patches, _orig) = convert_cylinder_to_bezier_patches(&position, 2.0, Some((0.0, 5.0)));
    assert_eq!(patches.len(), 4);

    for patch in &patches {
      // Sample points and check distance from axis
      for &(u, v) in &[(0.1, 0.1), (0.5, 0.5), (0.9, 0.9), (0.3, 0.7)] {
        let pt = patch.surface.evaluate(u, v);
        let dist_from_z = (pt.x * pt.x + pt.y * pt.y).sqrt();
        assert!(
          (dist_from_z - 2.0).abs() < 0.01,
          "point ({pt:?}) distance from Z axis: {dist_from_z}, expected 2.0"
        );
      }
    }
  }

  #[test]
  fn sphere_patch_evaluates_on_sphere() {
    let position = Axis2Placement3d {
      label: String::new(),
      location: CartesianPoint {
        label: String::new(),
        coordinates: vec![1.0, 0.0, 0.0],
      },
      axis: Direction {
        label: String::new(),
        direction_ratios: vec![0.0, 0.0, 1.0],
      },
      ref_direction: Some(Direction {
        label: String::new(),
        direction_ratios: vec![1.0, 0.0, 0.0],
      }),
    };

    let center = Vec3::new(1.0, 0.0, 0.0);
    let radius = 3.0;

    let (patches, _orig) = convert_sphere_to_bezier_patches(&position, radius);
    assert_eq!(patches.len(), 8);

    for patch in &patches {
      for &(u, v) in &[
        (0.0, 0.0),
        (0.0, 1.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.5, 0.5),
        (0.2, 0.3),
        (0.7, 0.8),
      ] {
        let pt = patch.surface.evaluate(u, v);
        let dist = (pt - center).length();
        assert!(
          (dist - radius as f32).abs() < 1e-3,
          "patch u_range={:?} v_range={:?}: point at (u={u}, v={v}) = {pt:?}, dist={dist}, expected {radius}",
          patch.u_range,
          patch.v_range
        );
      }
    }
  }

  // ── Projection tests ───────────────────────────────────────────────

  /// Helper: create Z-up frame at origin for cylinder/cone/sphere tests.
  fn z_up_frame() -> (Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>) {
    let origin = Vec3::zero();
    let axis = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);
    (origin, axis, x_dir, y_dir)
  }

  #[test]
  fn plane_project_on_surface() {
    let origin = Vec3::new(1.0, 2.0, 3.0);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let u_dir = Vec3::new(1.0, 0.0, 0.0);
    let v_dir = Vec3::new(0.0, 1.0, 0.0);

    // Point exactly on the plane
    let p = origin + u_dir * 3.0 + v_dir * 4.0;
    let (u, v, dist) = project_point_plane(p, origin, u_dir, v_dir, normal);
    assert!((u - 3.0).abs() < 1e-6, "u={u}");
    assert!((v - 4.0).abs() < 1e-6, "v={v}");
    assert!(dist < 1e-6, "dist={dist}");
  }

  #[test]
  fn plane_project_off_surface() {
    let origin = Vec3::zero();
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let u_dir = Vec3::new(1.0, 0.0, 0.0);
    let v_dir = Vec3::new(0.0, 1.0, 0.0);

    let p = Vec3::new(5.0, 5.0, 10.0);
    let (u, v, dist) = project_point_plane(p, origin, u_dir, v_dir, normal);
    assert!((u - 5.0).abs() < 1e-6, "u={u}");
    assert!((v - 5.0).abs() < 1e-6, "v={v}");
    assert!((dist - 10.0).abs() < 1e-6, "dist={dist}");
  }

  #[test]
  fn cylinder_project_on_surface() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 2.0;
    let v_range = (0.0, 5.0);

    // Point at angle 0, height 2
    let p = origin + x_dir * radius + axis * 2.0;
    let (u, v, dist) = project_point_cylinder(p, origin, axis, x_dir, y_dir, radius, v_range);
    assert!((u - 0.0).abs() < 1e-6, "u={u}");
    assert!((v - 2.0).abs() < 1e-6, "v={v}");
    assert!(dist < 1e-6, "dist={dist}");
  }

  #[test]
  fn cylinder_project_angle_90() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 3.0;
    let v_range = (0.0, 10.0);

    // Point at angle π/2
    let p = origin + y_dir * radius + axis * 5.0;
    let (u, v, dist) = project_point_cylinder(p, origin, axis, x_dir, y_dir, radius, v_range);
    assert!((u - std::f32::consts::FRAC_PI_2).abs() < 1e-6, "u={u}");
    assert!((v - 5.0).abs() < 1e-6, "v={v}");
    assert!(dist < 1e-6, "dist={dist}");
  }

  #[test]
  fn cylinder_project_off_surface() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 2.0;
    let v_range = (0.0, 10.0);

    // Point at radius 5.0 (3.0 outside surface) at angle 0
    let p = origin + x_dir * 5.0 + axis * 3.0;
    let (u, v, dist) = project_point_cylinder(p, origin, axis, x_dir, y_dir, radius, v_range);
    assert!(u.abs() < 1e-6, "u should be near 0, got {u}");
    assert!((v - 3.0).abs() < 1e-6, "v={v}");
    assert!((dist - 3.0).abs() < 1e-6, "dist should be 3.0, got {dist}");
  }

  #[test]
  fn cylinder_project_v_clamped() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 2.0;
    let v_range = (0.0, 5.0);

    // Point at height 20 but clamped v_range goes to 0..5
    let p = origin + x_dir * radius + axis * 20.0;
    let (_u, v, _dist) = project_point_cylinder(p, origin, axis, x_dir, y_dir, radius, v_range);
    assert!(
      (v - 5.0).abs() < 1e-6,
      "v should be clamped to 5.0, got {v}"
    );
  }

  #[test]
  fn cone_project_on_surface() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 3.0;
    let tan_a = 0.5; // cone narrows by 0.5 per unit height
    let v_range = (0.0, 4.0);
    // At v=2, radius = 3 - 2*0.5 = 2.0
    let v_test = 2.0;
    let r_at_v = radius - v_test * tan_a;
    let p = origin + x_dir * r_at_v + axis * v_test;
    let (u, v, dist) = project_point_cone(p, origin, axis, x_dir, y_dir, radius, tan_a, v_range);
    assert!(u.abs() < 1e-6, "u={u}");
    assert!((v - v_test).abs() < 1e-4, "v={v}");
    assert!(dist < 1e-4, "dist={dist}");
  }

  #[test]
  fn cone_project_off_surface() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 4.0;
    let tan_a = 1.0;
    let v_range = (0.0, 3.0);
    // Point at x=5, z=1. Optimal v = cos²(π/4)*(1 - 1*(5-4)) = 0.5*0 = 0.
    // At v=0: r=4, closest=(4,0,0), dist = sqrt((5-4)²+(1-0)²) = sqrt(2).
    let p = origin + x_dir * 5.0 + axis * 1.0;
    let (u, v, dist) = project_point_cone(p, origin, axis, x_dir, y_dir, radius, tan_a, v_range);
    assert!(u.abs() < 1e-6, "u should be near 0, got {u}");
    let expected_dist = 2.0f32.sqrt();
    assert!(
      (dist - expected_dist).abs() < 1e-4,
      "dist should be √2 ≈ {expected_dist}, got {dist}"
    );
    assert!(v.abs() < 1e-3, "optimal v should be near 0, got {v}");
  }

  #[test]
  fn cone_project_v_clamped() {
    let (origin, axis, x_dir, y_dir) = z_up_frame();
    let radius = 3.0;
    let tan_a = 0.5;
    let v_range = (0.0, 4.0);
    // Point far above cone, v should clamp to 4.0
    let r_at_vmax = radius - 4.0 * tan_a; // 3 - 2 = 1.0
    let p = origin + x_dir * r_at_vmax + axis * 10.0;
    let (_u, v, _dist) = project_point_cone(p, origin, axis, x_dir, y_dir, radius, tan_a, v_range);
    assert!(
      (v - 4.0).abs() < 1e-3,
      "v should be clamped to 4.0, got {v}"
    );
  }

  #[test]
  fn sphere_project_on_surface() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let radius = 5.0;
    let polar = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);

    // Point on equator at longitude 0
    let p = center + x_dir * radius + polar * 0.0;
    let (u, v, dist) = project_point_sphere(p, center, radius, polar, x_dir, y_dir).unwrap();
    assert!(u.abs() < 1e-6, "u={u}");
    assert!((v - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "v={v}");
    assert!(dist < 1e-6, "dist={dist}");
  }

  #[test]
  fn sphere_project_north_pole() {
    let center = Vec3::zero();
    let radius = 2.0;
    let polar = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);

    let p = center + polar * radius;
    let (_, v, dist) = project_point_sphere(p, center, radius, polar, x_dir, y_dir).unwrap();
    assert!(v < 1e-6, "v should be near 0 at north pole, got {v}");
    assert!(dist < 1e-6, "dist={dist}");
  }

  #[test]
  fn sphere_project_off_surface() {
    let center = Vec3::zero();
    let radius = 3.0;
    let polar = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);

    // Point at distance 10 from center, sphere radius 3 → dist should be 7
    let p = Vec3::new(0.0, 10.0, 0.0);
    let (u, v, dist) = project_point_sphere(p, center, radius, polar, x_dir, y_dir).unwrap();
    assert!((u - std::f32::consts::FRAC_PI_2).abs() < 1e-6, "u={u}");
    assert!((v - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "v={v}");
    assert!((dist - 7.0).abs() < 1e-6, "dist={dist}");
  }

  #[test]
  fn sphere_project_at_center_is_none() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let radius = 5.0;
    let polar = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);

    let result = project_point_sphere(center, center, radius, polar, x_dir, y_dir);
    assert!(result.is_none(), "point at center should yield None");
  }

  #[test]
  fn torus_project_on_surface() {
    let center = Vec3::zero();
    let axis = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);
    let major_r = 4.0;
    let minor_r = 1.0;

    // Point on the torus: u=0 (x_dir), v=0 (outward in radial plane)
    // S(0,0) = center + (R + r)*x_dir = (5, 0, 0)
    let p = center + x_dir * (major_r + minor_r);
    let (u, v, dist) =
      project_point_torus(p, center, axis, x_dir, y_dir, major_r, minor_r).unwrap();
    assert!(u.abs() < 1e-3, "u={u}"); // angle near 0
    assert!(v.abs() < 1e-3, "v={v}"); // v near 0 (outermost)
    assert!(dist < 1e-4, "dist={dist}");
  }

  #[test]
  fn torus_project_known_point() {
    let center = Vec3::zero();
    let axis = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);
    let major_r = 4.0;
    let minor_r = 1.0;

    // Point on top of torus at u=0: S(0, π/2) = (R, 0, r) = (4, 0, 1)
    let p = center + x_dir * major_r + axis * minor_r;
    let (u, v, dist) =
      project_point_torus(p, center, axis, x_dir, y_dir, major_r, minor_r).unwrap();
    assert!(u.abs() < 1e-3, "u should be near 0, got {u}");
    assert!(
      (v - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
      "v should be π/2, got {v}"
    );
    assert!(dist < 1e-4, "dist={dist}");
  }

  #[test]
  fn torus_project_large_distance() {
    let center = Vec3::zero();
    let axis = Vec3::new(0.0, 0.0, 1.0);
    let x_dir = Vec3::new(1.0, 0.0, 0.0);
    let y_dir = Vec3::new(0.0, 1.0, 0.0);
    let major_r = 3.0;
    let minor_r = 1.0;

    // Point far outside torus: (10, 0, 0)
    let p = Vec3::new(10.0, 0.0, 0.0);
    let (u, v, dist) =
      project_point_torus(p, center, axis, x_dir, y_dir, major_r, minor_r).unwrap();
    // Closest point on torus from (10,0,0) is at u=0, v=0: (4, 0, 0)
    assert!(u.abs() < 1e-3, "u should be near 0, got {u}");
    assert!(v.abs() < 1e-3, "v should be near 0, got {v}");
    assert!((dist - 6.0).abs() < 1e-3, "dist should be ~6.0, got {dist}");
  }

  #[test]
  fn extrusion_project_single_segment() {
    use crate::curve3d::RationalBezierCurve3d;

    // Line segment from (0,0,0) to (4,0,0), extruded along Z by mag=3
    let ctrl = vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(4.0, 0.0, 0.0, 1.0)];
    let curve = RationalBezierCurve3d::new(ctrl, 1);
    let segments = vec![curve];
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let mag = 3.0;

    // Point on surface: u=0.5 (center of segment), v=0.5 → (2, 0, 1.5)
    let p = Vec3::new(2.0, 0.0, 1.5);
    let (u, v, dist) = project_point_extrusion(p, &segments, dir, mag).unwrap();
    assert!((u - 0.5).abs() < 1e-2, "u should be ~0.5, got {u}");
    assert!((v - 0.5).abs() < 1e-2, "v should be ~0.5, got {v}");
    assert!(dist < 0.01, "dist={dist}");
  }

  #[test]
  fn extrusion_project_off_surface() {
    use crate::curve3d::RationalBezierCurve3d;

    let ctrl = vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(4.0, 0.0, 0.0, 1.0)];
    let curve = RationalBezierCurve3d::new(ctrl, 1);
    let segments = vec![curve];
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let mag = 3.0;

    // Point offset from surface: (2, 5, 1.5) → 5 units away in Y
    let p = Vec3::new(2.0, 5.0, 1.5);
    let (u, v, dist) = project_point_extrusion(p, &segments, dir, mag).unwrap();
    assert!((u - 0.5).abs() < 1e-2, "u should be ~0.5, got {u}");
    assert!((v - 0.5).abs() < 1e-2, "v should be ~0.5, got {v}");
    assert!((dist - 5.0).abs() < 1e-2, "dist should be ~5.0, got {dist}");
  }

  #[test]
  fn extrusion_project_v_clamped() {
    use crate::curve3d::RationalBezierCurve3d;

    let ctrl = vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(2.0, 0.0, 0.0, 1.0)];
    let curve = RationalBezierCurve3d::new(ctrl, 1);
    let segments = vec![curve];
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let mag = 3.0;

    // Point way above extrusion (v would be 2.0, but clamps to 1.0)
    let p = Vec3::new(1.0, 0.0, 6.0);
    let (u, v, dist) = project_point_extrusion(p, &segments, dir, mag).unwrap();
    assert!((u - 0.5).abs() < 1e-2, "u should be ~0.5, got {u}");
    assert!(
      (v - 1.0).abs() < 1e-2,
      "v should be clamped to 1.0, got {v}"
    );
    assert!((dist - 3.0).abs() < 1e-2, "dist should be ~3.0, got {dist}");
  }

  #[test]
  fn extrusion_project_two_segments() {
    use crate::curve3d::RationalBezierCurve3d;

    // Two line segments: (0,0,0)→(2,0,0) and (2,0,0)→(4,0,0), extruded along Y by mag=2
    let ctrl1 = vec![Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(2.0, 0.0, 0.0, 1.0)];
    let ctrl2 = vec![Vec4::new(2.0, 0.0, 0.0, 1.0), Vec4::new(4.0, 0.0, 0.0, 1.0)];
    let segments = vec![
      RationalBezierCurve3d::new(ctrl1, 1),
      RationalBezierCurve3d::new(ctrl2, 1),
    ];
    let dir = Vec3::new(0.0, 1.0, 0.0);
    let mag = 2.0;

    // Point on second segment, u_local=0.5 → u_global = (1 + 0.5)/2 = 0.75
    let p = Vec3::new(3.0, 1.0, 0.0);
    let (u, v, dist) = project_point_extrusion(p, &segments, dir, mag).unwrap();
    assert!((u - 0.75).abs() < 1e-2, "u should be ~0.75, got {u}");
    assert!((v - 0.5).abs() < 1e-2, "v should be ~0.5, got {v}");
    assert!(dist < 0.01, "dist={dist}");
  }
}

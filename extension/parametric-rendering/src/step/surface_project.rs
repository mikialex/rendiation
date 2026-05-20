use crate::*;

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
  /// `SurfaceSubPatch::u_range` / `v_range`.
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

/// Project a 3D point onto a plane.
///
/// u/v are signed distances along u_dir/v_dir from origin.
/// normal must be unit-length and orthogonal to u_dir, v_dir.
fn project_point_plane(
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
fn project_point_cylinder(
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
fn project_point_cone(
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
fn project_point_sphere(
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

#[cfg(test)]
mod tests {
  use super::*;

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

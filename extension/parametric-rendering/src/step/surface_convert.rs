use rendiation_step_reader::entities::{
  self, Axis2Placement3d, BSplineSurfaceWithKnots, CurveAny, NonRationalBSplineSurface,
  RationalBSplineSurface, SurfaceAny,
};

use super::*;
use crate::step::StepReadError;
use crate::*;

/// A Bezier patch paired with its parameter range in the original surface's
/// parameter space. Used for remapping trim curves after surface decomposition.
#[derive(Debug, Clone)]
pub struct SurfaceSubPatch {
  pub surface: RationalBezierSurface<f32>,
  pub sub_range: SubRange,
}

#[derive(Debug, Clone, Copy)]
pub struct SubRange {
  /// u range of this patch in the original surface's parameter space
  pub u_range: (f32, f32),
  /// v range of this patch in the original surface's parameter space
  pub v_range: (f32, f32),
}

/// Convert any STEP surface to Bezier patches with parameter range info,
/// together with an `OriginalSurface` for single-call point projection.
pub fn convert_and_split_any_surface_to_bezier_patches(
  surface: &SurfaceAny,
) -> Result<(Vec<SurfaceSubPatch>, OriginalSurface), StepReadError> {
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

/// Convert a B-spline surface (unweighted) to Bezier patches via knot insertion.
///
/// Each patch covers one knot span with the parameter range recorded.
/// Control points are a 2D grid: `control_points[row][col]` where `row` = v-index,
/// `col` = u-index.
fn convert_bspline_surface_to_bezier_patches(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
  u_knots: Vec<f32>,
  v_knots: Vec<f32>,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
fn convert_rational_bspline_surface_to_bezier_patches(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
  weights: Vec<Vec<f32>>,
  u_knots: Vec<f32>,
  v_knots: Vec<f32>,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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

/// Build SurfaceSubPatch from the knot vectors and decomposition result.
fn build_patch_param_ranges(
  patches: Vec<Vec<RationalBezierSurface<f32>>>,
  u_knots: &[f32],
  v_knots: &[f32],
  u_degree: usize,
  v_degree: usize,
) -> Vec<SurfaceSubPatch> {
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
        result.push(SurfaceSubPatch {
          surface,
          sub_range: SubRange {
            u_range: (u_bounds[ui], u_bounds[ui + 1]),
            v_range: (v_bounds[vi], v_bounds[vi + 1]),
          },
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

/// Convert a Bezier surface (no knots) to a single patch.
fn convert_bezier_surface_to_patch(
  u_degree: usize,
  v_degree: usize,
  control_points: Vec<Vec<Vec3<f32>>>,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
  let u_count = control_points[0].len();
  let v_count = control_points.len();

  let flat_cp: Vec<Vec4<f32>> = control_points
    .into_iter()
    .flat_map(|row| row.into_iter().map(|p| Vec4::new(p.x, p.y, p.z, 1.0)))
    .collect();

  let surface = RationalBezierSurface::new(flat_cp.clone(), u_degree, v_degree);
  let patch = SurfaceSubPatch {
    surface,
    sub_range: SubRange {
      u_range: (0.0, 1.0),
      v_range: (0.0, 1.0),
    },
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

/// Convert a plane to a single degree-1 Bezier patch.
///
/// The patch is a bilinear quadrilateral in the plane. The corners are
/// chosen at (±1, ±1) in the plane's local coordinates — this gives a
/// finite patch for projection. Trim curves will determine the actual
/// extent.
fn convert_plane_to_bezier_patch(
  position: &Axis2Placement3d,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
  let patch = SurfaceSubPatch {
    surface,
    sub_range: SubRange {
      u_range: (-extent, extent),
      v_range: (-extent, extent),
    },
  };
  let orig = OriginalSurface::Plane {
    origin: center,
    u_dir: x_dir,
    v_dir: y_dir,
    normal,
  };
  (vec![patch], orig)
}

/// Convert a cylindrical surface to 4 biquadratic rational Bezier patches.
///
/// u direction: 4 × 90° circular arcs (degree 2 rational)
/// v direction: degree 1 (linear extrusion)
///
/// v_range specifies the height extent along the cylinder axis. If `None`,
/// defaults to [0.0, 1.0].
fn convert_cylinder_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
  v_range: Option<(f64, f64)>,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
    patches.push(SurfaceSubPatch {
      surface,
      sub_range: SubRange {
        u_range: (angle0, angle1),
        v_range: (v0, v1),
      },
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

/// Convert a conical surface to 4 biquadratic rational Bezier patches.
///
/// The cone is defined by its position (axis + location), base radius at the
/// location point, and semi_angle. The apex is at a distance of
/// `radius / tan(semi_angle)` along the axis from the location point.
///
/// v_range specifies the distance along the axis. Defaults to [0.0, 1.0].
fn convert_cone_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
  semi_angle: f64,
  v_range: Option<(f64, f64)>,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
    patches.push(SurfaceSubPatch {
      surface,
      sub_range: SubRange {
        u_range: (angle0, angle1),
        v_range: (v0, v1),
      },
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

/// Convert a spherical surface to 8 biquadratic rational Bezier patches.
///
/// The sphere is split into 2 hemispheres (north and south), each split into
/// 4 quadrants around the polar axis. Each patch is a biquadratic rational
/// Bezier (3×3 control points, degree 2 in both directions).
///
/// u ∈ [0, 2π]: longitude, v ∈ [0, π]: colatitude (angle from north pole)
fn convert_sphere_to_bezier_patches(
  position: &Axis2Placement3d,
  radius: f64,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
      patches.push(SurfaceSubPatch {
        surface,
        sub_range: SubRange {
          u_range: (u0, u1),
          v_range: (v0, v1),
        },
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
fn convert_torus_to_bezier_patches(
  position: &Axis2Placement3d,
  major_radius: f64,
  minor_radius: f64,
) -> (Vec<SurfaceSubPatch>, OriginalSurface) {
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
      patches.push(SurfaceSubPatch {
        surface,
        sub_range: SubRange {
          u_range: (u0, u1),
          v_range: (v0, v1),
        },
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

/// Convert a surface of linear extrusion to Bezier patches.
///
/// Takes a generator curve and extrudes it linearly. Each Bezier segment of
/// the curve becomes one patch (degree in u = curve degree, degree in v = 1).
fn convert_extrusion_surface_to_bezier_patches(
  swept_curve: &CurveAny,
  extrusion_axis: &entities::Vector,
) -> Result<(Vec<SurfaceSubPatch>, OriginalSurface), StepReadError> {
  let curve_segments = convert_any_curve_to_bezier(swept_curve)?;
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
    patches.push(SurfaceSubPatch {
      surface,
      sub_range: SubRange {
        u_range: (u0, u1),
        v_range: (0.0, 1.0),
      },
    });
  }

  let orig = OriginalSurface::Extrusion {
    curve_segments,
    extrusion_dir: dir,
    extrusion_mag: mag,
  };
  Ok((patches, orig))
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
          patch.sub_range.u_range,
          patch.sub_range.v_range
        );
      }
    }
  }
}

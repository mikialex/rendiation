use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::*;

struct Lcg(u64);

impl Lcg {
  fn next(&mut self) -> f64 {
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    (self.0 >> 11) as f64 / (1u64 << 53) as f64
  }

  fn range(&mut self, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * self.next()
  }
}

fn ctx() -> ConvertCtx {
  ConvertCtx::new(1e-7, 0.1, 3)
}

fn tilted_frame() -> Frame {
  let z = Vec3::new(0.3, -0.2, 1.).normalize();
  let x = Vec3::new(1., 0.4, 0.);
  let x = (x - z * x.dot(z)).normalize();
  Frame {
    origin: Vec3::new(1., -2., 0.5),
    x,
    y: z.cross(x),
    z,
  }
}

fn rect(u: (f64, f64), v: (f64, f64)) -> NativeRect {
  NativeRect { u, v }
}

/// Patch exactness against native evaluation at random points of every cell, and coverage of
/// the rect by existing cells. `size` is the model scale for the relative error.
fn check_patches(s: &dyn SurfaceModel, r: NativeRect, size: f64) -> SurfaceParamSpace {
  let space = s.build_param_space(r, &ctx()).unwrap();
  assert!(!space.patches.is_empty());
  let mut rng = Lcg(7);
  for patch in &space.patches {
    let (i, j) = patch.cell;
    let mut params = vec![(0., 0.), (1., 0.), (0., 1.), (1., 1.), (0.5, 0.5)];
    params.extend((0..40).map(|_| (rng.next(), rng.next())));
    for (a, b) in params {
      let native = space.to_native(Vec2::new(i as f64 + a, j as f64 + b));
      let expected = s.eval(native);
      let got = patch.surface.eval(a, b);
      let err = (got - expected).length();
      assert!(
        err <= 1e-12 * (size + expected.length()),
        "{} cell {:?} at {a} {b}: err {err}",
        s.kind_name(),
        patch.cell
      );
    }
  }
  for _ in 0..200 {
    let native = Vec2::new(rng.range(r.u.0, r.u.1), rng.range(r.v.0, r.v.1));
    let p = space.to_p(native);
    let cell = (p.x.floor() as i64, p.y.floor() as i64);
    let found = space.patches.iter().any(|c| {
      let (ci, cj) = (c.cell.0 as i64, c.cell.1 as i64);
      (ci == cell.0 || (ci + 1 == cell.0 && p.x == cell.0 as f64))
        && (cj == cell.1 || (cj + 1 == cell.1 && p.y == cell.1 as f64))
    });
    assert!(
      found,
      "{} native {native:?} p {p:?} not covered",
      s.kind_name()
    );
    let back = space.to_native(p);
    assert!((back - native).length() <= 1e-12 * (1. + native.length()));
  }
  space
}

fn reduce_diff(d: f64, period: Option<f64>) -> f64 {
  match period {
    Some(t) => d - t * (d / t).round(),
    None => d,
  }
}

/// Projection round trips at `natives` (none of them at a pole): without guess, with a nearby
/// guess, with a guess shifted by whole periods, and for points offset along the normal.
fn check_projection(s: &dyn SurfaceModel, natives: &[Vec2<f64>], size: f64) {
  let period = s.period();
  for native in natives {
    let q = s.eval(*native);
    let r = s.project(q, None).unwrap();
    assert!(
      r.distance <= 1e-9 * size,
      "{} {native:?}: {}",
      s.kind_name(),
      r.distance
    );
    assert!((s.eval(r.native) - q).length() <= 1e-9 * size);
    for axis in 0..2 {
      let d = [r.native.x - native.x, r.native.y - native.y][axis];
      assert!(
        reduce_diff(d, period[axis]).abs() <= 1e-9,
        "{} {native:?} -> {:?}",
        s.kind_name(),
        r.native
      );
    }

    let guess = *native + Vec2::new(0.01, -0.01);
    let r = s.project(q, Some(guess)).unwrap();
    assert!(
      (r.native - *native).length() <= 1e-9,
      "{} {native:?} -> {:?}",
      s.kind_name(),
      r.native
    );

    let shift = Vec2::new(
      period[0].map_or(0., |t| 2. * t),
      period[1].map_or(0., |t| -t),
    );
    let r = s
      .project(q, Some(*native + shift + Vec2::new(0.01, 0.01)))
      .unwrap();
    assert!(
      (r.native - (*native + shift)).length() <= 1e-9,
      "{} periodic representative {native:?} -> {:?}",
      s.kind_name(),
      r.native
    );

    let (_, su, sv) = s.eval_with_partials(*native);
    let n = su.cross(sv);
    if n.length() > 1e-6 * su.length() * sv.length() {
      let h = 0.01 * size;
      let off = q + n.normalize() * h;
      let r = s.project(off, Some(*native)).unwrap();
      assert!(
        (r.distance - h).abs() <= 1e-9 * size,
        "{} offset {native:?}: {} vs {h}",
        s.kind_name(),
        r.distance
      );
      assert!((r.native - *native).length() <= 1e-7);
    }
  }
}

fn random_natives(n: usize, u: (f64, f64), v: (f64, f64)) -> Vec<Vec2<f64>> {
  let mut rng = Lcg(42);
  (0..n)
    .map(|_| Vec2::new(rng.range(u.0, u.1), rng.range(v.0, v.1)))
    .collect()
}

fn model_with(surfaces: Vec<SurfaceGeom>, curves: Vec<CurveGeom>) -> BrepModel {
  BrepModel {
    surfaces,
    curves,
    ..Default::default()
  }
}

fn build(geom: SurfaceGeom) -> Box<dyn SurfaceModel> {
  build_surface(&model_with(vec![geom], Vec::new()), SurfaceId(0), &ctx()).unwrap()
}

#[test]
fn plane() {
  let f = tilted_frame();
  let s = build(SurfaceGeom::Plane(f));
  assert_eq!(s.kind_name(), "Plane");
  let p = s.eval(Vec2::new(2., -1.));
  assert!((p - (f.origin + f.x * 2. - f.y)).length() < 1e-15);
  check_patches(&*s, rect((-1., 2.), (0.5, 3.)), 3.);
  check_projection(&*s, &random_natives(20, (-5., 5.), (-5., 5.)), 5.);
  let r = s.project(f.origin + f.z * 2. + f.x, None).unwrap();
  assert!((r.distance - 2.).abs() < 1e-14);
  assert!(
    s.build_param_space(rect((1., 1.), (0., 1.)), &ctx())
      .is_err()
  );
}

#[test]
fn cylinder() {
  let f = tilted_frame();
  let s = build(SurfaceGeom::Cylinder {
    frame: f,
    radius: 3.,
  });
  assert_eq!(s.period(), [Some(TAU), None]);
  let p = s.eval(Vec2::new(FRAC_PI_2, 2.));
  assert!((p - (f.origin + f.y * 3. + f.z * 2.)).length() < 1e-14);
  for r in [
    rect((0., 1.), (-1., 2.)),
    rect((-2.5, -0.3), (0., 0.1)),
    rect((1., 4.), (-3., -1.)),
  ] {
    let space = check_patches(&*s, r, 5.);
    assert_eq!(space.u_map.period, None);
    assert_eq!(space.u_map.breaks[0], r.u.0);
  }
  let space = check_patches(&*s, rect((0.7, 0.7 + TAU), (0., 1.)), 5.);
  assert_eq!(space.u_map.period, Some(TAU));
  assert_eq!(space.u_map.segment_count(), 4);
  assert_eq!(space.patches.len(), 4);
  assert!(
    s.build_param_space(rect((0., 7.), (0., 1.)), &ctx())
      .is_err()
  );

  check_projection(&*s, &random_natives(30, (-PI, PI), (-4., 4.)), 5.);
  // On the axis any angle is valid, the guess is kept.
  let r = s
    .project(f.origin + f.z * 1.5, Some(Vec2::new(5.5, 0.)))
    .unwrap();
  assert_eq!(r.native.x, 5.5);
  assert!((r.native.y - 1.5).abs() < 1e-14);
  assert!((r.distance - 3.).abs() < 1e-14);
}

#[test]
fn cone() {
  let f = tilted_frame();
  let (radius, alpha) = (1.5, 0.4);
  let s = build(SurfaceGeom::Cone {
    frame: f,
    radius,
    semi_angle: alpha,
  });
  // STEP convention: the radius grows with v for a positive semi angle.
  let dist_axis = |p: Vec3<f64>| {
    let d = p - f.origin;
    (d - f.z * d.dot(f.z)).length()
  };
  let r1 = dist_axis(s.eval(Vec2::new(0.3, 1.)));
  assert!((r1 - (radius + alpha.tan())).abs() < 1e-14);
  assert!(r1 > radius);
  let apex = -radius / alpha.tan();
  let poles = s.poles();
  assert_eq!(poles.len(), 1);
  assert_eq!(poles[0].axis, Axis::V);
  assert!((poles[0].value - apex).abs() < 1e-15);
  for u in [0., 1., 4.] {
    assert!((s.eval(Vec2::new(u, apex)) - poles[0].position).length() < 1e-14);
  }

  check_patches(&*s, rect((-0.5, 2.), (-1., 2.)), 5.);
  check_patches(&*s, rect((2., 2. + TAU), (apex, 1.)), 5.);
  check_patches(&*s, rect((0., 1.), (apex - 1., apex + 1.)), 5.);

  check_projection(&*s, &random_natives(30, (-PI, PI), (apex + 0.2, 3.)), 5.);
  // The other nappe.
  check_projection(
    &*s,
    &random_natives(10, (-PI, PI), (apex - 3., apex - 0.2)),
    5.,
  );
  let r = s
    .project(poles[0].position, Some(Vec2::new(1.3, apex + 0.1)))
    .unwrap();
  assert_eq!(r.native, Vec2::new(1.3, apex));
  assert!(r.distance < 1e-14);
  // A point beyond the apex along the axis projects onto the other nappe (double cone).
  let r = s.project(f.origin + f.z * (apex - 1.), None).unwrap();
  assert!((r.distance - alpha.sin()).abs() < 1e-12);
}

#[test]
fn sphere() {
  let f = tilted_frame();
  let s = build(SurfaceGeom::Sphere {
    frame: f,
    radius: 2.,
  });
  assert!((s.eval(Vec2::new(0.4, FRAC_PI_2)) - (f.origin + f.z * 2.)).length() < 1e-14);
  assert!((s.eval(Vec2::new(0., 0.)) - (f.origin + f.x * 2.)).length() < 1e-14);
  assert!((s.eval(Vec2::new(FRAC_PI_2, 0.)) - (f.origin + f.y * 2.)).length() < 1e-14);
  assert!((s.eval(Vec2::new(0., -FRAC_PI_2)) - (f.origin - f.z * 2.)).length() < 1e-14);
  assert_eq!(s.natural_domain(), [None, Some((-FRAC_PI_2, FRAC_PI_2))]);
  assert_eq!(s.poles().len(), 2);

  check_patches(&*s, rect((-1., 0.5), (-0.3, 1.2)), 3.);
  let space = check_patches(&*s, rect((-3., -3. + TAU), (-FRAC_PI_2, FRAC_PI_2)), 3.);
  assert_eq!(space.patches.len(), 8);
  check_patches(&*s, rect((0., 2.), (1., FRAC_PI_2)), 3.);
  assert!(
    s.build_param_space(rect((0., 1.), (0., 2.)), &ctx())
      .is_err()
  );

  check_projection(&*s, &random_natives(30, (-PI, PI), (-1.5, 1.5)), 3.);
  let north = f.origin + f.z * 2.;
  let r = s.project(north, Some(Vec2::new(0.7, 1.))).unwrap();
  assert_eq!(r.native, Vec2::new(0.7, FRAC_PI_2));
  let r = s
    .project(f.origin - f.z * 5., Some(Vec2::new(-0.2, -1.)))
    .unwrap();
  assert_eq!(r.native, Vec2::new(-0.2, -FRAC_PI_2));
  assert!((r.distance - 3.).abs() < 1e-14);
}

#[test]
fn torus() {
  let f = tilted_frame();
  let s = build(SurfaceGeom::Torus {
    frame: f,
    major_radius: 3.,
    minor_radius: 1.,
  });
  assert_eq!(s.period(), [Some(TAU), Some(TAU)]);
  assert!((s.eval(Vec2::new(0., FRAC_PI_2)) - (f.origin + f.x * 3. + f.z)).length() < 1e-14);
  check_patches(&*s, rect((-1., 0.5), (-2., 1.)), 5.);
  check_patches(&*s, rect((0.3, 0.3 + TAU), (-1., -1. + TAU)), 5.);
  check_projection(&*s, &random_natives(30, (-PI, PI), (-PI, PI)), 5.);
  // Spindle torus: both minor circles of the meridian plane are candidates.
  let spindle = build(SurfaceGeom::Torus {
    frame: f,
    major_radius: 0.5,
    minor_radius: 1.,
  });
  check_projection(&*spindle, &random_natives(30, (-PI, PI), (-2.5, 2.5)), 2.);
}

/// Rational quadratic full circle as a B-spline (9 control points, 4 spans): points and
/// weights, knots over `[0, 4]`.
fn bspline_circle(radius: f64) -> (Vec<Vec3<f64>>, Vec<f64>, Vec<f64>) {
  let h = std::f64::consts::FRAC_1_SQRT_2;
  let pts = [
    (1., 0.),
    (1., 1.),
    (0., 1.),
    (-1., 1.),
    (-1., 0.),
    (-1., -1.),
    (0., -1.),
    (1., -1.),
    (1., 0.),
  ];
  let points = pts
    .iter()
    .map(|(x, y)| Vec3::new(x * radius, y * radius, 0.))
    .collect();
  let weights = (0..9).map(|i| if i % 2 == 1 { h } else { 1. }).collect();
  let knots = vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.];
  (points, weights, knots)
}

/// A closed B-spline cylinder (`u` closed) whose bottom row optionally collapses to a point.
fn closed_bspline(bottom_radius: f64) -> BSplineSurfaceData {
  let (circle, weights, knots_u) = bspline_circle(2.);
  let control_points = circle
    .iter()
    .map(|p| {
      vec![
        Vec3::new(p.x * bottom_radius / 2., p.y * bottom_radius / 2., 0.),
        *p + Vec3::new(0., 0., 1.5),
      ]
    })
    .collect();
  BSplineSurfaceData {
    degree_u: 2,
    degree_v: 1,
    knots_u,
    knots_v: vec![0., 0., 2., 2.],
    control_points,
    weights: Some(weights.iter().map(|w| vec![*w, *w]).collect()),
    u_closed: None,
    v_closed: None,
  }
}

fn random_bspline() -> BSplineSurfaceData {
  let mut rng = Lcg(3);
  let (nu, nv) = (7, 5);
  let control_points = (0..nu)
    .map(|i| {
      (0..nv)
        .map(|j| {
          Vec3::new(
            i as f64 + rng.range(-0.2, 0.2),
            j as f64 * 1.3 + rng.range(-0.2, 0.2),
            (i as f64 * 0.7).sin() + (j as f64 * 0.9).cos() * 0.5,
          )
        })
        .collect()
    })
    .collect();
  let weights = (0..nu)
    .map(|_| (0..nv).map(|_| rng.range(0.5, 2.)).collect())
    .collect();
  BSplineSurfaceData {
    degree_u: 3,
    degree_v: 2,
    // Unclamped in u.
    knots_u: (0..nu + 4).map(|i| i as f64 * 0.5 - 1.).collect(),
    knots_v: vec![0., 0., 0., 0.4, 0.4, 1., 1., 1.],
    control_points,
    weights: Some(weights),
    u_closed: None,
    v_closed: None,
  }
}

#[test]
fn bspline_open_rational() {
  let data = random_bspline();
  let s = build(SurfaceGeom::BSpline(data.clone()));
  assert_eq!(s.kind_name(), "BSpline");
  assert_eq!(s.period(), [None, None]);
  assert!(s.poles().is_empty());
  let [du, dv] = s.natural_domain();
  let (du, dv) = (du.unwrap(), dv.unwrap());
  assert_eq!(du, (0.5, 2.5));
  assert_eq!(dv, (0., 1.));
  // STEP layout `[u][v]`: clamped v corner equals the control point at v index 0.
  let space = check_patches(&*s, rect(du, dv), 8.);
  assert_eq!(space.patches.len(), 4 * 2);
  assert_eq!(space.u_map.breaks, vec![0.5, 1., 1.5, 2., 2.5]);
  let sub = check_patches(&*s, rect((0.6, 0.9), (0.5, 0.9)), 8.);
  assert_eq!(sub.patches.len(), 1);
  assert_eq!(sub.patches[0].cell, (0, 1));
  assert!(s.build_param_space(rect((0.2, 0.9), dv), &ctx()).is_err());

  let natives = random_natives(30, (0.55, 2.45), (0.05, 0.95));
  check_projection(&*s, &natives, 8.);
  // Points outside the domain project onto the boundary.
  let corner = s.eval(Vec2::new(du.1, dv.1));
  let (_, su, sv) = s.eval_with_partials(Vec2::new(du.1, dv.1));
  let outside = corner + su.normalize() * 0.5 + sv.normalize() * 0.5;
  let r = s.project(outside, None).unwrap();
  assert!(r.native.x <= du.1 && r.native.y <= dv.1);

  // Non rational: weights absent.
  let mut plain = data;
  plain.weights = None;
  let s = build(SurfaceGeom::BSpline(plain.clone()));
  let corner = s.eval(Vec2::new(0.5, 0.));
  assert!(corner.length() > 0.);
  check_patches(&*s, rect((0.5, 2.5), (0., 1.)), 8.);
  check_projection(&*s, &natives, 8.);

  let mut clamped = plain;
  clamped.knots_u = vec![0., 0., 0., 0., 0.3, 0.6, 0.8, 1., 1., 1., 1.];
  let s = build(SurfaceGeom::BSpline(clamped.clone()));
  let p = s.eval(Vec2::new(1., 0.));
  assert!((p - clamped.control_points[6][0]).length() < 1e-14);
  let p = s.eval(Vec2::new(0., 1.));
  assert!((p - clamped.control_points[0][4]).length() < 1e-14);
}

#[test]
fn bspline_closed_and_pole() {
  let s = build(SurfaceGeom::BSpline(closed_bspline(2.)));
  assert_eq!(s.period(), [Some(4.), None]);
  assert_eq!(s.natural_domain(), [None, Some((0., 2.))]);
  assert!(s.poles().is_empty());
  // Geometrically a cylinder of radius 2.
  for k in 0..20 {
    let p = s.eval(Vec2::new(k as f64 * 0.37 - 3., 1.));
    assert!((p.x.hypot(p.y) - 2.).abs() < 1e-13);
  }
  let space = check_patches(&*s, rect((0., 4.), (0., 2.)), 3.);
  assert_eq!(space.u_map.period, Some(4.));
  assert_eq!(space.u_map.p_period(), Some(4.));
  // Across the seam: cells 3 and 4 (4 is span 0 again).
  let space = check_patches(&*s, rect((3.5, 4.5), (0.5, 1.)), 3.);
  let cells: Vec<_> = space.patches.iter().map(|c| c.cell).collect();
  assert_eq!(cells, vec![(3, 0), (4, 0)]);
  assert!(
    s.build_param_space(rect((4.6, 5.), (0., 1.)), &ctx())
      .is_err()
  );
  assert!(
    s.build_param_space(rect((-0.5, 0.5), (0., 1.)), &ctx())
      .is_err()
  );
  assert!(
    s.build_param_space(rect((0.5, 5.), (0., 1.)), &ctx())
      .is_err()
  );

  check_projection(&*s, &random_natives(20, (0., 4.), (0.1, 1.9)), 3.);
  // Periodic representative near the guess across the seam.
  let q = s.eval(Vec2::new(0.1, 1.));
  let r = s.project(q, Some(Vec2::new(3.95, 1.))).unwrap();
  assert!((r.native - Vec2::new(4.1, 1.)).length() < 1e-9);
  let r = s.project(q, None).unwrap();
  assert!((r.native - Vec2::new(0.1, 1.)).length() < 1e-9);

  let cone = build(SurfaceGeom::BSpline(closed_bspline(0.)));
  assert_eq!(cone.period(), [Some(4.), None]);
  let poles = cone.poles();
  assert_eq!(poles.len(), 1);
  assert_eq!(poles[0].axis, Axis::V);
  assert_eq!(poles[0].value, 0.);
  assert!(poles[0].position.length() < 1e-15);
  let r = cone
    .project(Vec3::new(0., 0., 0.), Some(Vec2::new(2.3, 0.2)))
    .unwrap();
  assert_eq!(r.native, Vec2::new(2.3, 0.));
  check_patches(&*cone, rect((1.5, 4.), (0., 2.)), 3.);
  check_projection(&*cone, &random_natives(20, (0., 4.), (0.2, 1.9)), 3.);
}

#[test]
fn bspline_invalid() {
  let mut data = random_bspline();
  data.control_points[2].pop();
  let m = model_with(vec![SurfaceGeom::BSpline(data)], Vec::new());
  assert!(build_surface(&m, SurfaceId(0), &ctx()).is_err());
  let mut data = random_bspline();
  data.knots_u.pop();
  let m = model_with(vec![SurfaceGeom::BSpline(data)], Vec::new());
  assert!(build_surface(&m, SurfaceId(0), &ctx()).is_err());
}

#[test]
fn unsupported_kinds() {
  let m = model_with(
    vec![
      SurfaceGeom::Offset {
        basis: SurfaceId(1),
        distance: 1.,
      },
      SurfaceGeom::Unsupported {
        step_id: 77,
        type_name: "FOO".into(),
      },
    ],
    Vec::new(),
  );
  let e = build_surface(&m, SurfaceId(0), &ctx()).err().unwrap();
  assert_eq!(e.kind, ErrorKind::Unsupported);
  let e = build_surface(&m, SurfaceId(1), &ctx()).err().unwrap();
  assert_eq!(e.kind, ErrorKind::Unsupported);
  assert_eq!(e.step_id, Some(77));
  assert!(build_surface(&m, SurfaceId(5), &ctx()).is_err());
}

#[test]
fn extrusion_of_conic_and_line() {
  let f = tilted_frame();
  let vector = Vec3::new(0.2, 0.5, 1.5);
  let m = model_with(
    vec![
      SurfaceGeom::Extrusion {
        curve: CurveId(0),
        vector,
      },
      SurfaceGeom::Extrusion {
        curve: CurveId(1),
        vector,
      },
    ],
    vec![
      CurveGeom::Ellipse {
        frame: f,
        semi_axis1: 3.,
        semi_axis2: 1.5,
      },
      CurveGeom::Line {
        origin: Vec3::new(1., 2., 3.),
        vector: Vec3::new(2., 0., 0.5),
      },
    ],
  );
  let s = build_surface(&m, SurfaceId(0), &ctx()).unwrap();
  assert_eq!(s.kind_name(), "Extrusion");
  assert_eq!(s.period(), [Some(TAU), None]);
  let p = s.eval(Vec2::new(FRAC_PI_2, 2.));
  assert!((p - (f.origin + f.y * 1.5 + vector * 2.)).length() < 1e-14);
  check_patches(&*s, rect((-1., 2.), (-0.5, 2.)), 5.);
  let space = check_patches(&*s, rect((0.3, 0.3 + TAU), (0., 1.)), 5.);
  assert_eq!(space.u_map.period, Some(TAU));
  check_projection(&*s, &random_natives(30, (-PI, PI), (-2., 2.)), 5.);

  let s = build_surface(&m, SurfaceId(1), &ctx()).unwrap();
  assert_eq!(s.period(), [None, None]);
  check_patches(&*s, rect((-1., 2.), (-0.5, 2.)), 5.);
  check_projection(&*s, &random_natives(20, (-3., 3.), (-2., 2.)), 5.);

  let degenerate = model_with(
    vec![SurfaceGeom::Extrusion {
      curve: CurveId(0),
      vector: Vec3::new(4., 0., 1.),
    }],
    vec![CurveGeom::Line {
      origin: Vec3::new(1., 2., 3.),
      vector: Vec3::new(2., 0., 0.5),
    }],
  );
  assert!(build_surface(&degenerate, SurfaceId(0), &ctx()).is_err());
}

/// Fixed swept curve from a NURBS curve decomposition.
fn nurbs_swept(degree: usize, knots: Vec<f64>, cp: Vec<Vec4<f64>>) -> SweptCurve {
  let curve = NurbsCurve3::new(degree, knots, cp).unwrap();
  let segments = curve.to_bezier_segments();
  let mut breaks = vec![segments[0].0.0];
  breaks.extend(segments.iter().map(|s| s.0.1));
  let n = segments.len();
  let map = PiecewiseParamMap::new(breaks, vec![SegmentMap::Affine; n], None);
  SweptCurve::fixed(map, segments.into_iter().map(|s| s.1).collect(), ctx().tol).unwrap()
}

fn wavy_curve() -> SweptCurve {
  let cp = (0..6)
    .map(|i| {
      let x = i as f64;
      Vec4::from_euclid(
        Vec3::new(1. + 0.3 * (x * 1.1).sin(), 0., x * 0.8),
        1. + 0.3 * (x * 0.7).cos().abs(),
      )
    })
    .collect();
  // Unclamped knots.
  nurbs_swept(3, (0..10).map(|i| i as f64 * 0.25).collect(), cp)
}

#[test]
fn extrusion_of_fixed_curve() {
  let s = Extrusion::new(wavy_curve(), Vec3::new(0.3, 1.2, 0.1), &ctx()).unwrap();
  let [du, _] = s.natural_domain();
  let du = du.unwrap();
  assert_eq!(du, (0.75, 1.5));
  check_patches(&s, rect(du, (-1., 1.)), 5.);
  let space = check_patches(&s, rect((0.8, 0.9), (0., 1.)), 5.);
  assert_eq!(space.patches.len(), 1);
  assert!(
    s.build_param_space(rect((0.5, 1.), (0., 1.)), &ctx())
      .is_err()
  );
  check_projection(&s, &random_natives(20, (0.8, 1.45), (-1., 1.)), 5.);

  // A closed curve (rational circle as B-spline) is periodic.
  let (points, weights, knots) = bspline_circle(2.);
  let cp = points
    .iter()
    .zip(&weights)
    .map(|(p, w)| Vec4::from_euclid(*p, *w))
    .collect();
  let closed = nurbs_swept(2, knots, cp);
  assert_eq!(closed.period(), Some(4.));
  let s = Extrusion::new(closed, Vec3::new(0., 0.5, 2.), &ctx()).unwrap();
  assert_eq!(s.period(), [Some(4.), None]);
  let space = check_patches(&s, rect((3.2, 4.7), (0., 1.)), 3.);
  assert_eq!(space.patches.len(), 2);
  check_projection(&s, &random_natives(20, (0., 4.), (-1., 1.)), 3.);
}

#[test]
fn revolution_matches_elementary() {
  let f = tilted_frame();
  let (radius, alpha) = (1.5, 0.4);
  let cone = Cone::new(f, radius, alpha).unwrap();
  let line = SweptCurve::line(f.origin + f.x * radius, f.x * alpha.tan() + f.z).unwrap();
  let s = Revolution::new(line, f.origin, f.z * 2., &ctx()).unwrap();
  assert_eq!(s.kind_name(), "Revolution");
  let mut rng = Lcg(5);
  for _ in 0..50 {
    let n = Vec2::new(rng.range(-4., 4.), rng.range(-5., 5.));
    assert!((s.eval(n) - cone.eval(n)).length() < 1e-13);
  }
  let poles = s.poles();
  assert_eq!(poles.len(), 1);
  assert!((poles[0].value - cone.poles()[0].value).abs() < 1e-13);
  check_patches(&s, rect((-1., 2.), (-1., 2.)), 5.);
  check_projection(&s, &random_natives(20, (-PI, PI), (-3., 3.)), 5.);

  let torus = Torus::new(f, 3., 1.).unwrap();
  let circle_frame = Frame {
    origin: f.origin + f.x * 3.,
    x: f.x,
    y: f.z,
    z: f.x.cross(f.z),
  };
  let circle = SweptCurve::conic(circle_frame, 1., 1.).unwrap();
  let s = Revolution::new(circle, f.origin, f.z, &ctx()).unwrap();
  for _ in 0..50 {
    let n = Vec2::new(rng.range(-4., 4.), rng.range(-4., 4.));
    assert!((s.eval(n) - torus.eval(n)).length() < 1e-13);
  }
  assert_eq!(s.period(), [Some(TAU), Some(TAU)]);
  check_patches(&s, rect((-1., 2.), (0.5, 0.5 + TAU)), 5.);
  check_projection(&s, &random_natives(20, (-PI, PI), (-PI, PI)), 5.);
}

#[test]
fn revolution_of_fixed_curve() {
  // Generatrix in the xz plane, starting on the z axis (pole).
  let cp = (0..5)
    .map(|i| {
      let x = i as f64;
      let r = if i == 0 { 0. } else { 0.5 + 0.4 * x };
      Vec4::from_euclid(Vec3::new(r, 0.2 * x * x, x * 0.7), 1. + 0.2 * x)
    })
    .collect();
  let curve = nurbs_swept(3, vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.], cp);
  let s = Revolution::new(curve, Vec3::new(0., 0., 0.), Vec3::new(0., 0., 1.), &ctx()).unwrap();
  assert_eq!(s.period(), [Some(TAU), None]);
  assert_eq!(s.natural_domain(), [None, Some((0., 1.))]);
  let poles = s.poles();
  assert_eq!(poles.len(), 1);
  assert_eq!(poles[0].value, 0.);
  // u = 0 is the generatrix itself.
  let q = s.eval(Vec2::new(0., 1.));
  assert!((q - Vec3::new(2.1, 3.2, 2.8)).length() < 1e-13);
  // Right handed rotation about the axis.
  let q = s.eval(Vec2::new(FRAC_PI_2, 1.));
  assert!((q - Vec3::new(-3.2, 2.1, 2.8)).length() < 1e-13);
  check_patches(&s, rect((-2., -2. + TAU), (0., 1.)), 4.);
  check_patches(&s, rect((0.3, 1.), (0.6, 0.9)), 4.);
  check_projection(&s, &random_natives(20, (-PI, PI), (0.1, 0.95)), 4.);
  let r = s
    .project(Vec3::new(0., 0., 0.), Some(Vec2::new(1.1, 0.1)))
    .unwrap();
  assert_eq!(r.native, Vec2::new(1.1, 0.));
}

/// Sanity run over every surface of the steam controller sample (skipped when the file is
/// missing): build, patch exactness over the natural domain (or a bounded rect), projection
/// round trips. Run with `--ignored --nocapture`.
#[test]
#[ignore]
fn steam_controller_surfaces() {
  use fast_hash_collection::FastHashMap;
  use step_reader::table::Table;

  let path = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../temp/step-test/steam-controller.stp"
  );
  let Ok(bytes) = std::fs::read(path) else {
    println!("missing {path}");
    return;
  };
  let parsed = Table::from_step(&String::from_utf8_lossy(&bytes), None);
  let model = build_brep_model(&parsed.table);
  let ctx = ConvertCtx::new(model.uncertainty.unwrap_or(1e-3), 0.1, 3);
  let mut kinds: FastHashMap<String, usize> = FastHashMap::default();
  let mut errors: FastHashMap<String, usize> = FastHashMap::default();
  let mut periodic: FastHashMap<String, usize> = FastHashMap::default();
  let mut poles: FastHashMap<String, usize> = FastHashMap::default();
  let (mut worst_patch, mut worst_projection) = (0f64, 0f64);
  let mut projection_failures = 0;
  let mut rng = Lcg(11);
  for id in 0..model.surfaces.len() {
    let s = match build_surface(&model, SurfaceId(id as u32), &ctx) {
      Ok(s) => s,
      Err(e) => {
        *errors.entry(e.message).or_default() += 1;
        continue;
      }
    };
    *kinds.entry(s.kind_name().to_string()).or_default() += 1;
    for (i, t) in s.period().iter().enumerate() {
      if t.is_some() {
        *periodic
          .entry(format!("{} {}", s.kind_name(), ["u", "v"][i]))
          .or_default() += 1;
      }
    }
    *poles.entry(s.kind_name().to_string()).or_default() += s.poles().len();
    let domain = s.natural_domain();
    let period = s.period();
    let start = s.period_start();
    let axis = |i: usize| match (period[i], domain[i]) {
      (Some(t), _) => (start[i].unwrap(), start[i].unwrap() + t),
      (None, Some(d)) => d,
      (None, None) => (-5., 5.),
    };
    let r = rect(axis(0), axis(1));
    let space = match s.build_param_space(r, &ctx) {
      Ok(space) => space,
      Err(e) => {
        *errors
          .entry(format!("{}: param space: {}", s.kind_name(), e.message))
          .or_default() += 1;
        continue;
      }
    };
    let mut size = 0f64;
    for patch in &space.patches {
      let (i, j) = patch.cell;
      for _ in 0..4 {
        let (a, b) = (rng.next(), rng.next());
        let native = space.to_native(Vec2::new(i as f64 + a, j as f64 + b));
        let expected = s.eval(native);
        size = size.max(expected.length());
        worst_patch = worst_patch.max((patch.surface.eval(a, b) - expected).length());
      }
    }
    for _ in 0..10 {
      let native = Vec2::new(
        rng.range(r.u.0, r.u.1),
        rng.range(
          r.v.0 + 0.02 * (r.v.1 - r.v.0),
          r.v.1 - 0.02 * (r.v.1 - r.v.0),
        ),
      );
      let q = s.eval(native);
      for guess in [None, Some(native)] {
        match s.project(q, guess) {
          Some(p) => {
            let d = (s.eval(p.native) - q).length();
            worst_projection = worst_projection.max(d);
            if d > ctx.tol {
              projection_failures += 1;
            }
          }
          None => projection_failures += 1,
        }
      }
    }
  }
  println!("kinds {kinds:?}");
  println!("errors {errors:?}");
  println!("periodic axes {periodic:?}");
  println!("poles {poles:?}");
  println!(
    "worst patch error {worst_patch:e}, worst projection error {worst_projection:e}, \
     projection failures {projection_failures}"
  );
  assert!(worst_patch < 1e-9);
}

#[test]
fn swept_bspline_curve_end_to_end() {
  let curve = CurveGeom::BSpline(BSplineCurveData {
    degree: 2,
    knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
    control_points: vec![
      Vec3::new(1., 0., 0.),
      Vec3::new(2., 0., 1.),
      Vec3::new(1.5, 0., 2.),
      Vec3::new(0., 0., 3.),
    ],
    weights: Some(vec![1., 0.7, 1.3, 1.]),
    closed: None,
  });
  let m = model_with(
    vec![
      SurfaceGeom::Extrusion {
        curve: CurveId(0),
        vector: Vec3::new(0., 2., 0.5),
      },
      SurfaceGeom::Revolution {
        curve: CurveId(0),
        axis_origin: Vec3::new(0., 0., 0.),
        axis_dir: Vec3::new(0., 0., 3.),
      },
    ],
    vec![curve],
  );
  let s = build_surface(&m, SurfaceId(0), &ctx()).unwrap();
  assert_eq!(s.natural_domain(), [Some((0., 1.)), None]);
  check_patches(&*s, rect((0., 1.), (-1., 1.)), 4.);
  check_projection(&*s, &random_natives(20, (0.05, 0.95), (-1., 1.)), 4.);
  let s = build_surface(&m, SurfaceId(1), &ctx()).unwrap();
  assert_eq!(s.natural_domain(), [None, Some((0., 1.))]);
  // The generatrix ends on the axis: pole at v = 1.
  let poles = s.poles();
  assert_eq!(poles.len(), 1);
  assert_eq!(poles[0].value, 1.);
  check_patches(&*s, rect((0.5, 0.5 + TAU), (0., 1.)), 4.);
  check_projection(&*s, &random_natives(20, (-PI, PI), (0.05, 0.9)), 4.);
}

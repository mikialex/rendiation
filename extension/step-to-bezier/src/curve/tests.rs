use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::*;
use crate::surface::{NativeRect, Pole, SurfaceParamSpace, SurfaceProjection};

const EXACT: f64 = 1e-12;

fn ctx() -> ConvertCtx {
  ConvertCtx::new(1e-7, 0.1, 3)
}

#[derive(Default)]
struct Builder {
  m: BrepModel,
}

impl Builder {
  fn vertex(&mut self, p: Vec3<f64>) -> VertexId {
    self.m.vertices.push(Vertex {
      step_id: 0,
      position: p,
    });
    VertexId(self.m.vertices.len() as u32 - 1)
  }

  fn curve(&mut self, c: CurveGeom) -> CurveId {
    self.m.curves.push(c);
    CurveId(self.m.curves.len() as u32 - 1)
  }

  fn curve2(&mut self, c: Curve2Geom) -> Curve2Id {
    self.m.curves_2d.push(c);
    Curve2Id(self.m.curves_2d.len() as u32 - 1)
  }

  fn edge(&mut self, start: VertexId, end: VertexId, curve: CurveId, same_sense: bool) -> EdgeId {
    self.m.edges.push(Edge {
      step_id: 0,
      start,
      end,
      curve,
      same_sense,
      pcurves: Vec::new(),
      is_seam: false,
    });
    EdgeId(self.m.edges.len() as u32 - 1)
  }

  /// Edge between two new vertices.
  fn edge_at(&mut self, a: Vec3<f64>, b: Vec3<f64>, curve: CurveId, same_sense: bool) -> EdgeId {
    let (va, vb) = (self.vertex(a), self.vertex(b));
    self.edge(va, vb, curve, same_sense)
  }
}

fn frame(origin: Vec3<f64>, z: Vec3<f64>, x_ref: Vec3<f64>) -> Frame {
  let z = z.normalize();
  let x = (x_ref - z * x_ref.dot(z)).normalize();
  Frame {
    origin,
    x,
    y: z.cross(x),
    z,
  }
}

fn tilted() -> Frame {
  frame(
    Vec3::new(1., -2., 0.5),
    Vec3::new(1., 2., 3.),
    Vec3::new(0., 0., 1.),
  )
}

fn std_frame(origin: Vec3<f64>) -> Frame {
  Frame {
    origin,
    x: Vec3::new(1., 0., 0.),
    y: Vec3::new(0., 1., 0.),
    z: Vec3::new(0., 0., 1.),
  }
}

fn conic_point(f: &Frame, rx: f64, ry: f64, a: f64) -> Vec3<f64> {
  f.origin + f.x * (rx * a.cos()) + f.y * (ry * a.sin())
}

fn conic_angle(f: &Frame, rx: f64, ry: f64, p: Vec3<f64>) -> f64 {
  let d = p - f.origin;
  (d.dot(f.y) / ry).atan2(d.dot(f.x) / rx)
}

fn wrap_pi(a: f64) -> f64 {
  a - TAU * ((a + PI) / TAU).floor()
}

fn close(a: Vec3<f64>, b: Vec3<f64>, eps: f64) -> bool {
  (a - b).length() <= eps * (1. + a.length().max(b.length()))
}

/// Checks every segment is an exact arc of the conic (point and parameterization) and returns
/// the unwrapped angles at the joints, starting at the first point.
fn walk_conic(segments: &[RationalBezierCurve3], f: &Frame, rx: f64, ry: f64) -> Vec<f64> {
  let mut angles = vec![conic_angle(f, rx, ry, segments[0].start())];
  for seg in segments {
    assert_eq!(seg.degree(), 2);
    let a0 = angles[angles.len() - 1];
    let dir = wrap_pi(conic_angle(f, rx, ry, seg.eval(0.5)) - a0).signum();
    let span = (dir * (conic_angle(f, rx, ry, seg.end()) - a0)).rem_euclid(TAU);
    assert!(span <= FRAC_PI_2 * (1. + 1e-9), "arc span {span}");
    for i in 0..=32 {
      let t = i as f64 / 32.;
      let expected = conic_point(f, rx, ry, a0 + dir * arc_angle_from_t(t, span));
      assert!(close(seg.eval(t), expected, EXACT), "t {t}");
    }
    angles.push(a0 + dir * span);
  }
  angles
}

fn assert_joints(segments: &[RationalBezierCurve3]) {
  for w in segments.windows(2) {
    assert_eq!(w[0].end(), w[1].start());
  }
}

#[test]
fn full_circle_edge() {
  let f = tilted();
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::Circle {
    frame: f,
    radius: 2.,
  });
  let p = conic_point(&f, 2., 2., 0.7);
  let v = b.vertex(p);
  let forward = b.edge(v, v, c, true);
  let backward = b.edge(v, v, c, false);

  let e = build_edge3(&b.m, forward, &ctx()).unwrap();
  assert!(e.closed);
  assert!(matches!(e.hint, CurveHint::Conic { rx: 2., ry: 2., .. }));
  assert_eq!(e.segments.len(), 4);
  assert_eq!(e.segments[0].start(), p);
  assert_eq!(e.segments[3].end(), p);
  assert_joints(&e.segments);
  let angles = walk_conic(&e.segments, &f, 2., 2.);
  assert!((angles[4] - angles[0] - TAU).abs() < 1e-12);

  let e = build_edge3(&b.m, backward, &ctx()).unwrap();
  let angles = walk_conic(&e.segments, &f, 2., 2.);
  assert!((angles[4] - angles[0] + TAU).abs() < 1e-12);
}

#[test]
fn arc_crossing_parameter_origin() {
  let f = tilted();
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::Circle {
    frame: f,
    radius: 1.5,
  });
  let p58 = conic_point(&f, 1.5, 1.5, 5.8);
  let p04 = conic_point(&f, 1.5, 1.5, 0.4);
  let cases = [
    // (start, end, same_sense, expected signed angle)
    (p58, p04, true, 0.4 + TAU - 5.8),
    (p04, p58, false, -(0.4 + TAU - 5.8)),
    (p58, p04, false, -(5.8 - 0.4)),
    (p04, p58, true, 5.8 - 0.4),
  ];
  for (start, end, same_sense, expected) in cases {
    let edge = b.edge_at(start, end, c, same_sense);
    let e = build_edge3(&b.m, edge, &ctx()).unwrap();
    assert!(!e.closed);
    assert_eq!(e.segments[0].start(), start);
    assert_eq!(e.segments[e.segments.len() - 1].end(), end);
    let angles = walk_conic(&e.segments, &f, 1.5, 1.5);
    let total = angles[angles.len() - 1] - angles[0];
    assert!((total - expected).abs() < 1e-12, "{total} {expected}");
    assert_eq!(
      e.segments.len(),
      (expected.abs() / FRAC_PI_2).ceil() as usize
    );
  }
}

#[test]
fn ellipse_in_tilted_plane() {
  let f = frame(
    Vec3::new(0.3, 0.2, -1.),
    Vec3::new(0., 1., 1.),
    Vec3::new(1., 0., 1.),
  );
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::Ellipse {
    frame: f,
    semi_axis1: 3.,
    semi_axis2: 1.2,
  });
  let start = conic_point(&f, 3., 1.2, 2.);
  let end = conic_point(&f, 3., 1.2, 4.5);
  let edge = b.edge_at(start, end, c, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  let angles = walk_conic(&e.segments, &f, 3., 1.2);
  assert!((angles[angles.len() - 1] - angles[0] - 2.5).abs() < 1e-12);
  // Every point lies in the ellipse plane.
  for seg in &e.segments {
    for i in 0..=8 {
      assert!((seg.eval(i as f64 / 8.) - f.origin).dot(f.z).abs() < 1e-12);
    }
  }
  let edge = b.edge_at(end, start, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  let angles = walk_conic(&e.segments, &f, 3., 1.2);
  assert!((angles[angles.len() - 1] - angles[0] + 2.5).abs() < 1e-12);
}

#[test]
fn line_edge() {
  let origin = Vec3::new(1., 2., 3.);
  let vector = Vec3::new(0.3, -0.4, 1.2) * 2.5;
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::Line { origin, vector });
  let start = origin + vector * 2.;
  let end = origin + vector * -5.;
  let edge = b.edge_at(start, end, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert_eq!(e.segments.len(), 1);
  assert_eq!(e.segments[0].start(), start);
  assert_eq!(e.segments[0].end(), end);
  match e.hint {
    CurveHint::Line { origin: o, dir } => {
      assert_eq!(o, start);
      assert!(close(dir, vector.normalize() * -1., EXACT));
    }
    _ => panic!("line hint expected"),
  }

  // Slightly off the line (5 tol): accepted and snapped.
  let off = vector.cross(Vec3::new(1., 0., 0.)).normalize() * 5e-7;
  let edge = b.edge_at(start + off, end, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert_eq!(e.segments[0].start(), start + off);

  // Far off the line: rejected.
  let edge = b.edge_at(start + off * 1000., end, c, false);
  let err = build_edge3(&b.m, edge, &ctx()).unwrap_err();
  assert_eq!(err.kind, ErrorKind::InvalidData);

  // Same vertex: degenerate edge.
  let v = b.vertex(start);
  let edge = b.edge(v, v, c, true);
  assert!(build_edge3(&b.m, edge, &ctx()).unwrap().segments.is_empty());
}

fn test_nurbs(data: &BSplineCurveData<Vec3<f64>>) -> NurbsCurve3 {
  let cps = data
    .control_points
    .iter()
    .enumerate()
    .map(|(i, p)| Vec4::from_euclid(*p, data.weights.as_ref().map_or(1., |w| w[i])))
    .collect();
  NurbsCurve::new(data.degree, data.knots.clone(), cps).unwrap()
}

fn clamped_spline() -> BSplineCurveData<Vec3<f64>> {
  BSplineCurveData {
    degree: 3,
    knots: vec![0., 0., 0., 0., 1., 2.5, 3., 3., 3., 3.],
    control_points: vec![
      Vec3::new(0., 0., 0.),
      Vec3::new(1., 2., 0.5),
      Vec3::new(2., 2.5, -1.),
      Vec3::new(4., 1., 0.),
      Vec3::new(5., -1., 2.),
      Vec3::new(6., 0., 1.),
    ],
    weights: Some(vec![1., 0.7, 1.3, 0.9, 1.1, 1.]),
    closed: None,
  }
}

/// Segment `i` covers native `ranges[i]` affinely (in edge direction when `reversed`).
fn check_affine_segments(
  segments: &[RationalBezierCurve3],
  ranges: &[(f64, f64)],
  eval: impl Fn(f64) -> Vec3<f64>,
) {
  assert_eq!(segments.len(), ranges.len());
  for (seg, (lo, hi)) in segments.iter().zip(ranges) {
    for i in 0..=32 {
      let t = i as f64 / 32.;
      assert!(
        close(seg.eval(t), eval(lo + (hi - lo) * t), EXACT),
        "{lo} {hi} {t}"
      );
    }
  }
}

#[test]
fn clamped_bspline_edge() {
  let data = clamped_spline();
  let nurbs = test_nurbs(&data);
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::BSpline(data));
  let (start, end) = (nurbs.eval(0.4), nurbs.eval(2.8));
  let edge = b.edge_at(start, end, c, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert!(matches!(e.hint, CurveHint::Other));
  assert_joints(&e.segments);
  check_affine_segments(&e.segments, &[(0.4, 1.), (1., 2.5), (2.5, 2.8)], |t| {
    nurbs.eval(t)
  });

  let edge = b.edge_at(end, start, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  check_affine_segments(&e.segments, &[(2.8, 2.5), (2.5, 1.), (1., 0.4)], |t| {
    nurbs.eval(t)
  });

  // same_sense contradicting the vertex order on an open curve.
  let edge = b.edge_at(end, start, c, true);
  assert!(build_edge3(&b.m, edge, &ctx()).is_err());
}

/// Uniform cubic with wrapped control points: closed (C2), domain [3, 9], unclamped knots.
fn closed_spline() -> BSplineCurveData<Vec3<f64>> {
  let base = [
    Vec3::new(1., 0., 0.),
    Vec3::new(0.5, 1., 0.2),
    Vec3::new(-0.5, 1., 0.),
    Vec3::new(-1., 0., -0.3),
    Vec3::new(-0.5, -1., 0.),
    Vec3::new(0.5, -1., 0.1),
  ];
  let control_points = (0..9).map(|i| base[i % 6]).collect();
  BSplineCurveData {
    degree: 3,
    knots: (0..13).map(|i| i as f64).collect(),
    control_points,
    weights: None,
    closed: Some(true),
  }
}

#[test]
fn closed_bspline_edges() {
  let data = closed_spline();
  let nurbs = test_nurbs(&data);
  let periodic = |t: f64| nurbs.eval(3. + (t - 3.).rem_euclid(6.));
  let mut b = Builder::default();
  let c = b.curve(CurveGeom::BSpline(data));

  // Wrapping through the domain end.
  let (start, end) = (nurbs.eval(8.2), nurbs.eval(3.7));
  let edge = b.edge_at(start, end, c, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert_joints(&e.segments);
  check_affine_segments(&e.segments, &[(8.2, 9.), (9., 9.7)], periodic);

  let edge = b.edge_at(end, start, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  check_affine_segments(&e.segments, &[(9.7, 9.), (9., 8.2)], periodic);

  // The other way around covers the long part.
  let edge = b.edge_at(start, end, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  let ranges = [(8.2, 8.), (8., 7.), (7., 6.), (6., 5.), (5., 4.), (4., 3.7)];
  check_affine_segments(&e.segments, &ranges, periodic);

  // Closed edge: full loop.
  let p = nurbs.eval(5.5);
  let v = b.vertex(p);
  let edge = b.edge(v, v, c, false);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert_eq!(e.segments[0].start(), p);
  assert_eq!(e.segments[e.segments.len() - 1].end(), p);
  let ranges = [
    (5.5, 5.),
    (5., 4.),
    (4., 3.),
    (9., 8.),
    (8., 7.),
    (7., 6.),
    (6., 5.5),
  ];
  check_affine_segments(&e.segments, &ranges, periodic);

  // Unclamped open spline.
  let mut open = closed_spline();
  open.control_points.truncate(7);
  open.knots.truncate(11);
  let nurbs = test_nurbs(&open);
  assert_eq!(nurbs.domain(), (3., 7.));
  let c = b.curve(CurveGeom::BSpline(open));
  let edge = b.edge_at(nurbs.eval(3.2), nurbs.eval(6.9), c, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  let ranges = [(3.2, 4.), (4., 5.), (5., 6.), (6., 6.9)];
  check_affine_segments(&e.segments, &ranges, |t| nurbs.eval(t));
}

fn eval_space(space: &CurveParamSpace, x: f64) -> Vec3<f64> {
  let p = space.map.to_p(x);
  let n = space.segments.len() as i64;
  let mut i = p.floor() as i64;
  let mut t = p - p.floor();
  if space.map.period.is_some() {
    i = i.rem_euclid(n);
  } else if i >= n {
    i = n - 1;
    t = p - i as f64;
  } else if i < 0 {
    i = 0;
    t = p;
  }
  space.segments[i as usize].eval(t)
}

#[test]
fn trimmed_curves() {
  let f = tilted();
  let mut b = Builder::default();
  let circle = b.curve(CurveGeom::Circle {
    frame: f,
    radius: 1.,
  });
  let trimmed = |b: &mut Builder, t1: f64, t2: f64, sense: bool| {
    b.curve(CurveGeom::Trimmed {
      basis: circle,
      trim1: vec![TrimSelect::Parameter(t1)],
      trim2: vec![TrimSelect::Parameter(t2)],
      sense_agreement: sense,
    })
  };
  let reversed = trimmed(&mut b, 1., 0.2, false);
  let wrapping = trimmed(&mut b, 1., 0.2, true);
  let full = trimmed(&mut b, 0., TAU, true);

  let space = build_curve_param_space(&b.m, reversed, None, &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0., 0.8]);
  assert_eq!(space.map.segments, vec![SegmentMap::CircularArc]);
  assert_eq!(space.map.period, None);
  for i in 0..=40 {
    let s = 0.8 * i as f64 / 40.;
    let expected = conic_point(&f, 1., 1., 1. - s);
    assert!(close(
      eval_curve(&b.m, reversed, s).unwrap(),
      expected,
      EXACT
    ));
    assert!(close(eval_space(&space, s), expected, EXACT));
  }

  let space = build_curve_param_space(&b.m, wrapping, None, &ctx()).unwrap();
  let length = TAU - 0.8;
  assert!((space.map.breaks[space.map.breaks.len() - 1] - length).abs() < 1e-14);
  assert_eq!(space.segments.len(), 4);
  for i in 0..=40 {
    let s = length * i as f64 / 40.;
    let expected = conic_point(&f, 1., 1., 1. + s);
    assert!(close(
      eval_curve(&b.m, wrapping, s).unwrap(),
      expected,
      EXACT
    ));
    assert!(close(eval_space(&space, s), expected, EXACT));
  }

  let space = build_curve_param_space(&b.m, full, None, &ctx()).unwrap();
  assert_eq!(space.map.period, Some(TAU));

  // Edge on the reversed trimmed arc, following the trimmed curve direction.
  let start = conic_point(&f, 1., 1., 1.);
  let end = conic_point(&f, 1., 1., 0.2);
  let edge = b.edge_at(start, end, reversed, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert!(matches!(e.hint, CurveHint::Conic { .. }));
  let angles = walk_conic(&e.segments, &f, 1., 1.);
  assert!((angles[angles.len() - 1] - angles[0] + 0.8).abs() < 1e-12);
  // Projection on the trimmed curve.
  let (s, d) = project_on_curve(&b.m, reversed, conic_point(&f, 1., 1., 0.45) * 1.).unwrap();
  assert!((s - 0.55).abs() < 1e-12 && d < 1e-12, "{s} {d}");

  // Trimmed line with point trims, reversed.
  let origin = Vec3::new(0., 1., 2.);
  let vector = Vec3::new(2., 0., -1.);
  let line = b.curve(CurveGeom::Line { origin, vector });
  let tl = b.curve(CurveGeom::Trimmed {
    basis: line,
    trim1: vec![TrimSelect::Point(origin + vector * 3.)],
    trim2: vec![TrimSelect::Point(origin + vector * 1.)],
    sense_agreement: false,
  });
  let space = build_curve_param_space(&b.m, tl, None, &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0., 2.]);
  for i in 0..=10 {
    let s = 0.2 * i as f64;
    let expected = origin + vector * (3. - s);
    assert!(close(eval_curve(&b.m, tl, s).unwrap(), expected, EXACT));
    assert!(close(eval_space(&space, s), expected, EXACT));
  }

  // Open basis with trims contradicting the sense.
  let spline = b.curve(CurveGeom::BSpline(clamped_spline()));
  let bad = b.curve(CurveGeom::Trimmed {
    basis: spline,
    trim1: vec![TrimSelect::Parameter(2.)],
    trim2: vec![TrimSelect::Parameter(1.)],
    sense_agreement: true,
  });
  let err = build_curve_param_space(&b.m, bad, None, &ctx()).unwrap_err();
  assert_eq!(err.kind, ErrorKind::InvalidData);
}

#[test]
fn composite_curve() {
  let mut b = Builder::default();
  let line = b.curve(CurveGeom::Line {
    origin: Vec3::new(0., 0., 0.),
    vector: Vec3::new(1., 0., 0.),
  });
  let seg_a = b.curve(CurveGeom::Trimmed {
    basis: line,
    trim1: vec![TrimSelect::Parameter(0.)],
    trim2: vec![TrimSelect::Parameter(2.)],
    sense_agreement: true,
  });
  let f = std_frame(Vec3::new(2., 1., 0.));
  let circle = b.curve(CurveGeom::Circle {
    frame: f,
    radius: 1.,
  });
  // Runs from (3, 1, 0) to (2, 0, 0) in decreasing angle.
  let seg_b = b.curve(CurveGeom::Trimmed {
    basis: circle,
    trim1: vec![TrimSelect::Parameter(0.)],
    trim2: vec![TrimSelect::Parameter(1.5 * PI)],
    sense_agreement: false,
  });
  let comp = b.curve(CurveGeom::Composite(vec![(seg_a, true), (seg_b, false)]));
  let expected = |s: f64| {
    if s <= 2. {
      Vec3::new(s, 0., 0.)
    } else {
      conic_point(&f, 1., 1., s - 2. - FRAC_PI_2)
    }
  };
  let space = build_curve_param_space(&b.m, comp, None, &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0., 2., 2. + FRAC_PI_2]);
  assert_eq!(
    space.map.segments,
    vec![SegmentMap::Affine, SegmentMap::CircularArc]
  );
  for i in 0..=50 {
    let s = (2. + FRAC_PI_2) * i as f64 / 50.;
    assert!(close(eval_space(&space, s), expected(s), EXACT), "{s}");
    assert!(close(
      eval_curve(&b.m, comp, s).unwrap(),
      expected(s),
      EXACT
    ));
  }
  let (s, d) = project_on_curve(&b.m, comp, Vec3::new(1.3, 0.5, 0.)).unwrap();
  assert!((s - 1.3).abs() < 1e-12 && (d - 0.5).abs() < 1e-12);

  let edge = b.edge_at(Vec3::new(1., 0., 0.), expected(2. + 0.3), comp, true);
  let e = build_edge3(&b.m, edge, &ctx()).unwrap();
  assert_eq!(e.segments.len(), 2);
  assert_joints(&e.segments);
  check_affine_segments(&e.segments[..1], &[(1., 2.)], expected);
  walk_conic(&e.segments[1..], &f, 1., 1.);
}

#[test]
fn param_spaces() {
  let f = tilted();
  let mut b = Builder::default();
  let circle = b.curve(CurveGeom::Circle {
    frame: f,
    radius: 0.7,
  });
  let space = build_curve_param_space(&b.m, circle, None, &ctx()).unwrap();
  assert_eq!(space.map.period, Some(TAU));
  assert_eq!(space.map.segments, vec![SegmentMap::CircularArc; 4]);
  for i in -100..=200 {
    let x = i as f64 * 0.0731;
    assert!(close(
      eval_space(&space, x),
      conic_point(&f, 0.7, 0.7, x),
      EXACT
    ));
    assert!(close(
      eval_curve(&b.m, circle, x).unwrap(),
      conic_point(&f, 0.7, 0.7, x),
      EXACT
    ));
  }
  // Range crossing the period start.
  let space = build_curve_param_space(&b.m, circle, Some((-1., 2.)), &ctx()).unwrap();
  assert_eq!(space.segments.len(), 2);
  assert_eq!(space.map.period, None);
  for i in 0..=30 {
    let x = -1. + 0.1 * i as f64;
    assert!(close(
      eval_space(&space, x),
      conic_point(&f, 0.7, 0.7, x),
      EXACT
    ));
  }
  let (s, d) = project_on_curve(&b.m, circle, conic_point(&f, 0.7, 0.7, 5.) * 1.).unwrap();
  assert!((s - 5.).abs() < 1e-12 && d < 1e-12);

  let origin = Vec3::new(1., 1., 1.);
  let vector = Vec3::new(0., 3., 4.);
  let line = b.curve(CurveGeom::Line { origin, vector });
  assert!(build_curve_param_space(&b.m, line, None, &ctx()).is_err());
  let space = build_curve_param_space(&b.m, line, Some((-2., 5.)), &ctx()).unwrap();
  assert_eq!(space.map, PiecewiseParamMap::single_affine(-2., 5.));
  assert_eq!(space.segments[0].start(), origin + vector * -2.);
  assert_eq!(space.segments[0].end(), origin + vector * 5.);

  let data = clamped_spline();
  let nurbs = test_nurbs(&data);
  let spline = b.curve(CurveGeom::BSpline(data));
  let space = build_curve_param_space(&b.m, spline, Some((0.5, 2.7)), &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0.5, 1., 2.5, 2.7]);
  for i in 0..=44 {
    let x = 0.5 + 0.05 * i as f64;
    assert!(close(eval_space(&space, x), nurbs.eval(x), EXACT));
  }
  let space = build_curve_param_space(&b.m, spline, None, &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0., 1., 2.5, 3.]);
  assert_eq!(space.map.period, None);
  let (s, d) = project_on_curve(&b.m, spline, nurbs.eval(1.7)).unwrap();
  assert!((s - 1.7).abs() < 1e-10 && d < 1e-12, "{s}");

  let closed = b.curve(CurveGeom::BSpline(closed_spline()));
  let space = build_curve_param_space(&b.m, closed, None, &ctx()).unwrap();
  assert_eq!(space.map.period, Some(6.));

  let polyline = b.curve(CurveGeom::Polyline(vec![
    Vec3::new(0., 0., 0.),
    Vec3::new(1., 0., 0.),
    Vec3::new(1., 2., 0.),
  ]));
  let space = build_curve_param_space(&b.m, polyline, None, &ctx()).unwrap();
  assert_eq!(space.map.breaks, vec![0., 1., 2.]);
  assert!(close(eval_space(&space, 1.5), Vec3::new(1., 1., 0.), EXACT));
  assert!(close(
    eval_curve(&b.m, polyline, 1.5).unwrap(),
    Vec3::new(1., 1., 0.),
    EXACT
  ));
}

#[test]
fn invalid_and_unsupported_curves() {
  let mut b = Builder::default();
  let circle = b.curve(CurveGeom::Circle {
    frame: tilted(),
    radius: 0.,
  });
  let offset = b.curve(CurveGeom::Offset {
    basis: circle,
    distance: 1.,
    ref_direction: Vec3::new(0., 0., 1.),
  });
  let unknown = b.curve(CurveGeom::Unsupported {
    step_id: 42,
    type_name: "HYPERBOLA".into(),
  });
  let nan_line = b.curve(CurveGeom::Line {
    origin: Vec3::new(f64::NAN, 0., 0.),
    vector: Vec3::new(1., 0., 0.),
  });
  let kind = |c| {
    build_curve_param_space(&b.m, c, Some((0., 1.)), &ctx())
      .unwrap_err()
      .kind
  };
  assert_eq!(kind(circle), ErrorKind::InvalidData);
  assert_eq!(kind(offset), ErrorKind::Unsupported);
  assert_eq!(kind(unknown), ErrorKind::Unsupported);
  assert_eq!(kind(nan_line), ErrorKind::InvalidData);
  assert_eq!(kind(CurveId(99)), ErrorKind::InvalidData);
  let err = build_curve_param_space(&b.m, unknown, None, &ctx()).unwrap_err();
  assert_eq!(err.step_id, Some(42));
}

/// `S(u, v) = (cos u, sin u, v)`.
struct TestCylinder;

impl SurfaceModel for TestCylinder {
  fn kind_name(&self) -> &'static str {
    "TestCylinder"
  }
  fn eval(&self, n: Vec2<f64>) -> Vec3<f64> {
    Vec3::new(n.x.cos(), n.x.sin(), n.y)
  }
  fn eval_with_partials(&self, n: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    (
      self.eval(n),
      Vec3::new(-n.x.sin(), n.x.cos(), 0.),
      Vec3::new(0., 0., 1.),
    )
  }
  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), None]
  }
  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, None]
  }
  fn poles(&self) -> Vec<Pole> {
    Vec::new()
  }
  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let mut u = p.y.atan2(p.x);
    if let Some(g) = guess {
      u += TAU * ((g.x - u) / TAU).round();
    }
    let native = Vec2::new(u, p.z);
    Some(SurfaceProjection {
      native,
      distance: (self.eval(native) - p).length(),
    })
  }
  fn build_param_space(
    &self,
    _rect: NativeRect,
    _ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    Err(ConvertError::unsupported(None, "test surface"))
  }
}

fn cyl(u: f64, v: f64) -> Vec3<f64> {
  TestCylinder.eval(Vec2::new(u, v))
}

fn uv_close(a: Vec2<f64>, b: Vec2<f64>, eps: f64) -> bool {
  (a - b).length() <= eps
}

/// Every pcurve point maps onto `edge3` and the chain is connected.
fn check_pcurve_on_edge(e2: &Edge2, e3: &Edge3, eps: f64) {
  for w in e2.segments.windows(2) {
    assert!(uv_close(w[0].end(), w[1].start(), 1e-12));
  }
  for seg in &e2.segments {
    for i in 0..=16 {
      let p = TestCylinder.eval(seg.eval(i as f64 / 16.));
      let d = closest_point_on_chain(&e3.segments, p).unwrap().2;
      assert!(d < eps, "{d}");
    }
  }
  let first = e2.segments[0].start();
  let last = e2.segments[e2.segments.len() - 1].end();
  assert!(close(
    TestCylinder.eval(first),
    e3.segments[0].start(),
    1e-9
  ));
  assert!(close(
    TestCylinder.eval(last),
    e3.segments[e3.segments.len() - 1].end(),
    1e-9
  ));
}

#[test]
fn pcurves_on_cylinder() {
  let ctx = ctx();
  let mut b = Builder::default();
  let circle = b.curve(CurveGeom::Circle {
    frame: std_frame(Vec3::new(0., 0., 1.)),
    radius: 1.,
  });
  let u_line = b.curve2(Curve2Geom::Line {
    origin: Vec2::new(0., 1.),
    vector: Vec2::new(1., 0.),
  });
  let seam_spline = b.curve2(Curve2Geom::BSpline(BSplineCurveData {
    degree: 1,
    knots: vec![0., 0., 1., 1.],
    control_points: vec![Vec2::new(-1., 1.), Vec2::new(1., 1.)],
    weights: None,
    closed: None,
  }));

  // Arc crossing the seam, pcurve as an unbounded line.
  let edge = b.edge_at(cyl(5.5, 1.), cyl(0.5, 1.), circle, true);
  let e3 = build_edge3(&b.m, edge, &ctx).unwrap();
  let e2 = build_edge_pcurve(&b.m, edge, u_line, &TestCylinder, &e3, &ctx).unwrap();
  assert_eq!(e2.iso, Some(IsoLine::V(1.)));
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  let (first, last) = (e2.segments[0].start(), e2.segments[0].end());
  assert!((last.x - first.x - (TAU - 5.)).abs() < 1e-9);
  assert!(((first.x - 5.5) / TAU - ((first.x - 5.5) / TAU).round()).abs() < 1e-10);

  // Same edge, bounded spline pcurve over [-1, 1] in u: the seam crossing representative.
  let e2 = build_edge_pcurve(&b.m, edge, seam_spline, &TestCylinder, &e3, &ctx).unwrap();
  assert_eq!(e2.iso, Some(IsoLine::V(1.)));
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  assert!(uv_close(
    e2.segments[0].start(),
    Vec2::new(5.5 - TAU, 1.),
    1e-9
  ));
  assert!(uv_close(
    e2.segments[e2.segments.len() - 1].end(),
    Vec2::new(0.5, 1.),
    1e-9
  ));

  // Reversed edge on the same pcurve: ordered from the edge start.
  let edge_r = b.edge_at(cyl(0.5, 1.), cyl(5.5, 1.), circle, false);
  let e3r = build_edge3(&b.m, edge_r, &ctx).unwrap();
  let e2 = build_edge_pcurve(&b.m, edge_r, seam_spline, &TestCylinder, &e3r, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3r, 1e-12);
  assert!(uv_close(e2.segments[0].start(), Vec2::new(0.5, 1.), 1e-9));

  // Full circle, decreasing: the pcurve runs one period down from the vertex.
  let v = b.vertex(cyl(0.5, 1.));
  let full = b.edge(v, v, circle, false);
  let e3 = build_edge3(&b.m, full, &ctx).unwrap();
  let e2 = build_edge_pcurve(&b.m, full, u_line, &TestCylinder, &e3, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  let (first, last) = (e2.segments[0].start(), e2.segments[0].end());
  assert!((last.x - first.x + TAU).abs() < 1e-9);
  let quarter = e2.segments[0].eval(0.25);
  assert!(close(
    cyl(quarter.x, quarter.y),
    cyl(0.5 - FRAC_PI_2, 1.),
    1e-9
  ));

  // Full circle with a closed-in-image spline pcurve over [0, 2π], vertex mid-way: continued
  // through the period.
  let period_spline = b.curve2(Curve2Geom::BSpline(BSplineCurveData {
    degree: 1,
    knots: vec![0., 0., 1., 1.],
    control_points: vec![Vec2::new(0., 1.), Vec2::new(TAU, 1.)],
    weights: None,
    closed: None,
  }));
  let full_fwd = b.edge(v, v, circle, true);
  let e3 = build_edge3(&b.m, full_fwd, &ctx).unwrap();
  let e2 = build_edge_pcurve(&b.m, full_fwd, period_spline, &TestCylinder, &e3, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  assert!(uv_close(e2.segments[0].start(), Vec2::new(0.5, 1.), 1e-9));
  assert!(uv_close(
    e2.segments[e2.segments.len() - 1].end(),
    Vec2::new(0.5 + TAU, 1.),
    1e-9
  ));
  assert_eq!(e2.iso, Some(IsoLine::V(1.)));
}

#[test]
fn iso_pcurves() {
  let ctx = ctx();
  let mut b = Builder::default();
  let line = b.curve(CurveGeom::Line {
    origin: Vec3::new(0.3f64.cos(), 0.3f64.sin(), -1.),
    vector: Vec3::new(0., 0., 0.5),
  });
  let edge = b.edge_at(cyl(0.3, 0.), cyl(0.3, 2.), line, true);
  let e3 = build_edge3(&b.m, edge, &ctx).unwrap();

  // Reversed line in another periodic representative.
  let u_iso = b.curve2(Curve2Geom::Line {
    origin: Vec2::new(0.3 + TAU, 5.),
    vector: Vec2::new(0., -2.),
  });
  let e2 = build_edge_pcurve(&b.m, edge, u_iso, &TestCylinder, &e3, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  let u = 0.3 + TAU;
  assert_eq!(e2.iso, Some(IsoLine::U(u)));
  for p in e2.segments.iter().flat_map(|s| &s.control_points) {
    assert_eq!(p.x / p.z, u);
  }
  assert!(uv_close(e2.segments[0].start(), Vec2::new(u, 0.), 1e-9));
  assert!(uv_close(e2.segments[0].end(), Vec2::new(u, 2.), 1e-9));

  let polyline = b.curve2(Curve2Geom::Polyline(vec![
    Vec2::new(0.3, -1.),
    Vec2::new(0.3, 0.5),
    Vec2::new(0.3, 3.),
  ]));
  let e2 = build_edge_pcurve(&b.m, edge, polyline, &TestCylinder, &e3, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  assert_eq!(e2.iso, Some(IsoLine::U(0.3)));
  assert_eq!(e2.segments.len(), 2);

  // Quadratic spline with constant v (uneven speed in u).
  let circle = b.curve(CurveGeom::Circle {
    frame: std_frame(Vec3::new(0., 0., 1.)),
    radius: 1.,
  });
  let edge = b.edge_at(cyl(0.1, 1.), cyl(0.6, 1.), circle, true);
  let e3 = build_edge3(&b.m, edge, &ctx).unwrap();
  let v_iso = b.curve2(Curve2Geom::BSpline(BSplineCurveData {
    degree: 2,
    knots: vec![0., 0., 0., 1., 1., 1.],
    control_points: vec![Vec2::new(0., 1.), Vec2::new(0.1, 1.), Vec2::new(1., 1.)],
    weights: None,
    closed: None,
  }));
  let e2 = build_edge_pcurve(&b.m, edge, v_iso, &TestCylinder, &e3, &ctx).unwrap();
  check_pcurve_on_edge(&e2, &e3, 1e-12);
  assert_eq!(e2.iso, Some(IsoLine::V(1.)));
  assert!(uv_close(e2.segments[0].start(), Vec2::new(0.1, 1.), 1e-9));
}

#[test]
fn helix_pcurve_crossing_seam() {
  // Coarse polyline of the helix (cos θ, sin θ, 0.1·θ): sag ~3e-6.
  let ctx = ConvertCtx::new(1e-5, 0.1, 3);
  let theta = |i: usize| 5.6 + 0.005 * i as f64;
  let helix = |t: f64| Vec3::new(t.cos(), t.sin(), 0.1 * t);
  let mut b = Builder::default();
  let points = (0..=280).map(|i| helix(theta(i))).collect();
  let polyline = b.curve(CurveGeom::Polyline(points));
  let (t0, t1) = (theta(40), theta(240));
  let edge = b.edge_at(helix(t0), helix(t1), polyline, true);
  let e3 = build_edge3(&b.m, edge, &ctx).unwrap();
  assert_eq!(e3.segments.len(), 200);
  let pcurve = b.curve2(Curve2Geom::Line {
    origin: Vec2::new(0., 0.),
    vector: Vec2::new(1., 0.1),
  });
  let e2 = build_edge_pcurve(&b.m, edge, pcurve, &TestCylinder, &e3, &ctx).unwrap();
  assert_eq!(e2.iso, None);
  check_pcurve_on_edge(&e2, &e3, 1e-5);
  assert!(uv_close(
    e2.segments[0].start(),
    Vec2::new(t0, 0.1 * t0),
    1e-9
  ));
  assert!(uv_close(
    e2.segments[0].end(),
    Vec2::new(t1, 0.1 * t1),
    1e-9
  ));
}

#[test]
fn pcurve_not_matching_edge() {
  let ctx = ctx();
  let mut b = Builder::default();
  let circle = b.curve(CurveGeom::Circle {
    frame: std_frame(Vec3::new(0., 0., 1.)),
    radius: 1.,
  });
  let edge = b.edge_at(cyl(0.1, 1.), cyl(0.6, 1.), circle, true);
  let e3 = build_edge3(&b.m, edge, &ctx).unwrap();
  let wrong = b.curve2(Curve2Geom::Line {
    origin: Vec2::new(0., 2.),
    vector: Vec2::new(1., 0.),
  });
  let err = build_edge_pcurve(&b.m, edge, wrong, &TestCylinder, &e3, &ctx).unwrap_err();
  assert_eq!(err.kind, ErrorKind::InvalidData);
}

use std::f64::consts::FRAC_PI_2;

use step_reader::table::{Table, TableParseErrors};

use super::*;
use crate::trim::{CellTrim, TrimParams, trim_cells};

fn ctx() -> ConvertCtx {
  ConvertCtx::new(1e-6, 0.1, 3)
}

fn std_frame(origin: Vec3<f64>) -> Frame {
  Frame {
    origin,
    x: Vec3::new(1., 0., 0.),
    y: Vec3::new(0., 1., 0.),
    z: Vec3::new(0., 0., 1.),
  }
}

fn frame(origin: Vec3<f64>, x: Vec3<f64>, y: Vec3<f64>) -> Frame {
  Frame {
    origin,
    x,
    y,
    z: x.cross(y),
  }
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

  fn surface(&mut self, s: SurfaceGeom) -> SurfaceId {
    self.m.surfaces.push(s);
    SurfaceId(self.m.surfaces.len() as u32 - 1)
  }

  fn edge(&mut self, start: VertexId, end: VertexId, curve: CurveId, same_sense: bool) -> EdgeId {
    self.m.edges.push(Edge {
      step_id: self.m.edges.len() as u64 + 1000,
      start,
      end,
      curve,
      same_sense,
      pcurves: Vec::new(),
      is_seam: false,
    });
    EdgeId(self.m.edges.len() as u32 - 1)
  }

  fn line_edge(&mut self, a: VertexId, b: VertexId) -> EdgeId {
    let (pa, pb) = (
      self.m.vertices[a.index()].position,
      self.m.vertices[b.index()].position,
    );
    let curve = self.curve(CurveGeom::Line {
      origin: pa,
      vector: pb - pa,
    });
    self.edge(a, b, curve, true)
  }

  fn circle_edge(&mut self, v: VertexId, frame: Frame, radius: f64) -> EdgeId {
    let curve = self.curve(CurveGeom::Circle { frame, radius });
    self.edge(v, v, curve, true)
  }

  /// A face with edge loops `(bound orientation, [(edge, orientation)])`.
  fn face(
    &mut self,
    surface: SurfaceId,
    same_sense: bool,
    bounds: Vec<(bool, Vec<(EdgeId, bool)>)>,
  ) -> FaceId {
    let bounds = bounds
      .into_iter()
      .map(|(orientation, uses)| FaceBound {
        step_id: 0,
        is_outer: false,
        orientation,
        kind: BoundKind::Edges(
          uses
            .into_iter()
            .map(|(edge, orientation)| OrientedEdgeUse { edge, orientation })
            .collect(),
        ),
      })
      .collect();
    self.m.faces.push(Face {
      step_id: 1,
      surface,
      same_sense,
      bounds,
    });
    FaceId(self.m.faces.len() as u32 - 1)
  }
}

/// Converts the face and checks the `FaceTrim` contract: bitwise connected loops, band loops
/// ending at `start + winding · p_period`, every edge curve on its 3D edge (within float noise
/// for exact curves, `tol` for fitted ones), split into cells by `trim` without error, every
/// emitted cell with a positive trimmed area.
fn convert(model: &BrepModel, face: FaceId) -> FaceTrim {
  let ctx = ctx();
  let f = &model.faces[face.index()];
  let surface = build_surface(model, f.surface, &ctx).unwrap();
  let mut edges = Edge3Cache::default();
  let trim = build_face_trim(model, face, surface.as_ref(), &mut edges, &ctx).unwrap();
  assert!(trim.warnings.is_empty(), "{:?}", trim.warnings);

  for l in &trim.loops {
    let n = l.curves.len();
    assert!(n > 0);
    for w in l.curves.windows(2) {
      assert_eq!(w[0].curve.end(), w[1].curve.start());
    }
    let mut offset = Vec2::new(0., 0.);
    for axis in 0..2 {
      if l.winding[axis] != 0 {
        set_coord(
          &mut offset,
          axis,
          l.winding[axis] as f64 * trim.p_period[axis].unwrap(),
        );
      }
    }
    assert_eq!(
      l.curves[n - 1].curve.end(),
      l.curves[0].curve.start() + offset
    );

    for c in &l.curves {
      let TrimCurveSource::Edge { edge, .. } = c.source else {
        continue;
      };
      let edge3 = edges.get(model, edge, &ctx).unwrap();
      let limit = if c.exact { 1e-9 } else { ctx.tol };
      for k in 0..=32 {
        let p = c.curve.eval(k as f64 / 32.);
        let x = surface.eval(trim.param_space.to_native(p));
        let (_, _, d) = closest_point_on_chain(&edge3.segments, x).unwrap();
        assert!(d <= limit, "deviation {d:e} (exact {})", c.exact);
      }
    }
  }

  let cells: Vec<_> = trim.param_space.patches.iter().map(|p| p.cell).collect();
  let cell_trims = trim_cells(&trim.loops, trim.p_period, &cells, &TrimParams::default()).unwrap();
  assert!(cell_trims.iter().any(|t| matches!(t, CellTrim::Trimmed(_))));
  for t in &cell_trims {
    if let CellTrim::Trimmed(loops) = t {
      let area: f64 = loops.iter().map(|l| p_area(&l.curves)).sum();
      assert!(area > 0., "cell area {area}");
    }
  }
  trim
}

/// Signed area enclosed by curves (sampled shoelace), the P area of a closed loop.
fn p_area(curves: &[TrimCurve]) -> f64 {
  let points: Vec<_> = curves
    .iter()
    .flat_map(|c| (0..64).map(move |k| c.curve.eval(k as f64 / 64.)))
    .collect();
  let n = points.len();
  (0..n)
    .map(|i| {
      let (a, b) = (points[i], points[(i + 1) % n]);
      0.5 * (a.x * b.y - b.x * a.y)
    })
    .sum()
}

fn edge_curves(l: &PLoop) -> impl Iterator<Item = &TrimCurve> {
  l.curves
    .iter()
    .filter(|c| matches!(c.source, TrimCurveSource::Edge { .. }))
}

/// Square `[0, 4]²` in the plane z = 0, counter-clockwise seen from +z.
fn square(b: &mut Builder) -> Vec<EdgeId> {
  let v: Vec<_> = [(0., 0.), (4., 0.), (4., 4.), (0., 4.)]
    .iter()
    .map(|(x, y)| b.vertex(Vec3::new(*x, *y, 0.)))
    .collect();
  (0..4).map(|i| b.line_edge(v[i], v[(i + 1) % 4])).collect()
}

#[test]
fn plane_square_with_hole() {
  let mut b = Builder::default();
  let plane = b.surface(SurfaceGeom::Plane(std_frame(Vec3::new(0., 0., 0.))));
  let sides = square(&mut b);
  let v = b.vertex(Vec3::new(3., 2., 0.));
  let hole = b.circle_edge(v, std_frame(Vec3::new(2., 2., 0.)), 1.);
  let outer = sides.iter().map(|e| (*e, true)).collect();
  let face = b.face(
    plane,
    true,
    vec![(true, outer), (true, vec![(hole, false)])],
  );

  let trim = convert(&b.m, face);
  assert!(!trim.flip_normal);
  assert_eq!(trim.fitted_curve_count, 0);
  assert_eq!(trim.p_period, [None, None]);
  assert_eq!(trim.loops.len(), 2);
  assert!(trim.loops.iter().all(|l| l.winding == [0, 0]));
  assert!(trim.loops.iter().flat_map(|l| &l.curves).all(|c| c.exact));
  assert!(p_area(&trim.loops[0].curves) > 0.);
  assert!(p_area(&trim.loops[1].curves) < 0.);
  // The rect has a margin: the loops lie strictly inside the single cell.
  for c in trim.loops.iter().flat_map(|l| &l.curves) {
    let (lo, hi) = c.curve.control_bbox();
    assert!(lo.x > 0. && lo.y > 0. && hi.x < 1. && hi.y < 1.);
  }
  // The hole edge is used reversed.
  let reversed = edge_curves(&trim.loops[1])
    .all(|c| matches!(c.source, TrimCurveSource::Edge { reversed: true, .. }));
  assert!(reversed);
}

#[test]
fn orientation_flags() {
  // Clockwise square: reversed by the bound orientation, or by same_sense (flip_normal).
  let mut b = Builder::default();
  let plane = b.surface(SurfaceGeom::Plane(std_frame(Vec3::new(0., 0., 0.))));
  let sides = square(&mut b);
  let cw: Vec<_> = sides.iter().rev().map(|e| (*e, false)).collect();
  let by_bound = b.face(plane, true, vec![(false, cw.clone())]);
  let by_sense = b.face(plane, false, vec![(true, cw.clone())]);
  let both = b.face(
    plane,
    false,
    vec![(false, sides.iter().map(|e| (*e, true)).collect())],
  );

  for (face, flip) in [(by_bound, false), (by_sense, true), (both, true)] {
    let trim = convert(&b.m, face);
    assert_eq!(trim.flip_normal, flip);
    assert!(p_area(&trim.loops[0].curves) > 0.);
  }

  // Inconsistent data (clockwise outer loop with same_sense true) is reported and fixed.
  let bad = b.face(plane, true, vec![(true, cw)]);
  let ctx = ctx();
  let surface = build_surface(&b.m, plane, &ctx).unwrap();
  let trim = build_face_trim(
    &b.m,
    bad,
    surface.as_ref(),
    &mut Edge3Cache::default(),
    &ctx,
  )
  .unwrap();
  assert_eq!(trim.warnings.len(), 1);
  assert!(p_area(&trim.loops[0].curves) > 0.);
}

fn cylinder(b: &mut Builder) -> SurfaceId {
  b.surface(SurfaceGeom::Cylinder {
    frame: std_frame(Vec3::new(0., 0., 0.)),
    radius: 2.,
  })
}

#[test]
fn cylinder_band_loops() {
  let mut b = Builder::default();
  let cyl = cylinder(&mut b);
  let v0 = b.vertex(Vec3::new(0., 2., 0.));
  let v1 = b.vertex(Vec3::new(0., 2., 5.));
  let bottom = b.circle_edge(v0, std_frame(Vec3::new(0., 0., 0.)), 2.);
  let top = b.circle_edge(v1, std_frame(Vec3::new(0., 0., 5.)), 2.);
  let face = b.face(
    cyl,
    true,
    vec![(true, vec![(bottom, true)]), (true, vec![(top, false)])],
  );

  let trim = convert(&b.m, face);
  assert_eq!(trim.p_period, [Some(4.), None]);
  let windings: Vec<_> = trim.loops.iter().map(|l| l.winding).collect();
  assert_eq!(windings, vec![[1, 0], [-1, 0]]);
  // Coaxial circles are exact v iso-lines.
  for l in &trim.loops {
    assert!(l.curves.iter().all(|c| c.exact && c.curve.degree() == 1));
    let y = l.curves[0].curve.start().y;
    assert!(l.curves.iter().all(|c| c.curve.end().y == y));
  }
  assert!(trim.loops[0].curves[0].curve.start().y < trim.loops[1].curves[0].curve.start().y);
  assert_eq!(trim.param_space.patches.len(), 4);
}

#[test]
fn cylinder_seam_projected() {
  let mut b = Builder::default();
  let cyl = cylinder(&mut b);
  let v0 = b.vertex(Vec3::new(2., 0., 0.));
  let v1 = b.vertex(Vec3::new(2., 0., 5.));
  let bottom = b.circle_edge(v0, std_frame(Vec3::new(0., 0., 0.)), 2.);
  let top = b.circle_edge(v1, std_frame(Vec3::new(0., 0., 5.)), 2.);
  let seam = b.line_edge(v0, v1);
  let face = b.face(
    cyl,
    true,
    vec![(
      true,
      vec![(bottom, true), (seam, true), (top, false), (seam, false)],
    )],
  );

  let trim = convert(&b.m, face);
  assert_eq!(trim.loops.len(), 1);
  assert_eq!(trim.loops[0].winding, [0, 0]);
  assert_eq!(trim.p_period, [Some(4.), None]);
  assert!(trim.loops[0].curves.iter().all(|c| c.exact));
  // The two seam uses are one period apart.
  let seams: Vec<_> = trim.loops[0]
    .curves
    .iter()
    .filter(|c| {
      c.source
        == TrimCurveSource::Edge {
          edge: seam,
          reversed: false,
        }
    })
    .chain(trim.loops[0].curves.iter().filter(|c| {
      c.source
        == TrimCurveSource::Edge {
          edge: seam,
          reversed: true,
        }
    }))
    .collect();
  assert_eq!(seams.len(), 2);
  assert_eq!(
    (seams[0].curve.start().x - seams[1].curve.start().x).abs(),
    4.
  );
  assert!(p_area(&trim.loops[0].curves) > 0.);
}

#[test]
fn sphere_cap_reaches_pole() {
  let mut b = Builder::default();
  let r = 3.;
  let sphere = b.surface(SurfaceGeom::Sphere {
    frame: std_frame(Vec3::new(0., 0., 0.)),
    radius: r,
  });
  let lat: f64 = 0.5;
  let v = b.vertex(Vec3::new(r * lat.cos(), 0., r * lat.sin()));
  let circle = b.circle_edge(
    v,
    std_frame(Vec3::new(0., 0., r * lat.sin())),
    r * lat.cos(),
  );
  let face = b.face(sphere, true, vec![(true, vec![(circle, true)])]);

  let trim = convert(&b.m, face);
  assert_eq!(trim.loops[0].winding, [1, 0]);
  let v_map = &trim.param_space.v_map;
  assert_eq!(v_map.breaks[v_map.breaks.len() - 1], FRAC_PI_2);
  assert!(v_map.breaks[0] < lat);
}

#[test]
fn sphere_octant_pole_segment() {
  let mut b = Builder::default();
  let r = 3.;
  let o = Vec3::new(0., 0., 0.);
  let sphere = b.surface(SurfaceGeom::Sphere {
    frame: std_frame(o),
    radius: r,
  });
  let (ex, ey, ez) = (
    Vec3::new(1., 0., 0.),
    Vec3::new(0., 1., 0.),
    Vec3::new(0., 0., 1.),
  );
  let a = b.vertex(ex * r);
  let bv = b.vertex(ey * r);
  let n = b.vertex(ez * r);
  let equator = b.curve(CurveGeom::Circle {
    frame: std_frame(o),
    radius: r,
  });
  let meridian_y = b.curve(CurveGeom::Circle {
    frame: frame(o, ey, ez),
    radius: r,
  });
  let meridian_x = b.curve(CurveGeom::Circle {
    frame: frame(o, ex, ez),
    radius: r,
  });
  let e0 = b.edge(a, bv, equator, true);
  let e1 = b.edge(bv, n, meridian_y, true);
  let e2 = b.edge(a, n, meridian_x, true);
  let face = b.face(
    sphere,
    true,
    vec![(true, vec![(e0, true), (e1, true), (e2, false)])],
  );

  let trim = convert(&b.m, face);
  let l = &trim.loops[0];
  assert_eq!(l.winding, [0, 0]);
  let poles: Vec<_> = l
    .curves
    .iter()
    .filter(|c| c.source == TrimCurveSource::Pole)
    .collect();
  assert_eq!(poles.len(), 1);
  // Along the upper pole line, with the material below: towards -u.
  let pole = &poles[0].curve;
  assert_eq!(pole.start().y, pole.end().y);
  assert!(pole.end().x < pole.start().x);
  assert!(l.curves.iter().all(|c| c.exact));
  assert_eq!(trim.fitted_curve_count, 0);
  assert!(p_area(&l.curves) > 0.);
}

#[test]
fn cone_to_apex() {
  let mut b = Builder::default();
  let (radius, semi_angle) = (2., 0.4f64);
  let cone = b.surface(SurfaceGeom::Cone {
    frame: std_frame(Vec3::new(0., 0., 0.)),
    radius,
    semi_angle,
  });
  let v = b.vertex(Vec3::new(radius, 0., 0.));
  let base = b.circle_edge(v, std_frame(Vec3::new(0., 0., 0.)), radius);
  // Material between the base circle (v = 0) and the apex below: base traversed towards -u.
  let face = b.face(cone, true, vec![(true, vec![(base, false)])]);

  let trim = convert(&b.m, face);
  assert_eq!(trim.loops[0].winding, [-1, 0]);
  let apex = -radius / semi_angle.tan();
  assert_eq!(trim.param_space.v_map.breaks[0], apex);
}

/// A flat B-spline patch (z = 0) with a non uniform parameterization along `u` and two knot
/// spans per axis, so a circle on it is neither an iso-line nor affine in P.
fn flat_bspline() -> SurfaceGeom {
  let xs = [0., 0.5, 3., 5., 6.];
  let ys = [0., 1.5, 3., 4.5, 6.];
  let control_points = xs
    .iter()
    .map(|x| ys.iter().map(|y| Vec3::new(*x, *y, 0.)).collect())
    .collect();
  SurfaceGeom::BSpline(BSplineSurfaceData {
    degree_u: 2,
    degree_v: 2,
    knots_u: vec![0., 0., 0., 1., 2., 3., 3., 3.],
    knots_v: vec![0., 0., 0., 1., 2., 3., 3., 3.],
    control_points,
    weights: None,
    u_closed: None,
    v_closed: None,
  })
}

#[test]
fn bspline_fitted_circle() {
  let mut b = Builder::default();
  let surface = b.surface(flat_bspline());
  let v = b.vertex(Vec3::new(4.5, 3., 0.));
  let circle = b.circle_edge(v, std_frame(Vec3::new(3., 3., 0.)), 1.5);
  let face = b.face(surface, true, vec![(true, vec![(circle, true)])]);

  let trim = convert(&b.m, face);
  assert_eq!(trim.fitted_curve_count, 1);
  let l = &trim.loops[0];
  assert!(l.curves.iter().all(|c| !c.exact && c.curve.is_polynomial()));
  // Fitted pieces are split where they cross the knot lines, exactly on the lines.
  let on_line = |x: f64| x == x.round();
  let joints_on_lines = l
    .curves
    .iter()
    .filter(|c| on_line(c.curve.start().x) || on_line(c.curve.start().y))
    .count();
  assert!(joints_on_lines >= 4, "{joints_on_lines}");
  for c in &l.curves {
    let mid = c.curve.eval(0.5);
    let (lo, hi) = c.curve.control_bbox();
    assert!(lo.x >= mid.x.floor() - 1e-9 && hi.x <= mid.x.floor() + 1. + 1e-9);
    assert!(lo.y >= mid.y.floor() - 1e-9 && hi.y <= mid.y.floor() + 1. + 1e-9);
  }
}

#[test]
fn bspline_boundary_iso_lines() {
  // A face covering the whole patch: its edges run along the natural boundary.
  let mut b = Builder::default();
  let surface = b.surface(flat_bspline());
  let v: Vec<_> = [(0., 0.), (6., 0.), (6., 6.), (0., 6.)]
    .iter()
    .map(|(x, y)| b.vertex(Vec3::new(*x, *y, 0.)))
    .collect();
  let sides: Vec<_> = (0..4)
    .map(|i| (b.line_edge(v[i], v[(i + 1) % 4]), true))
    .collect();
  let face = b.face(surface, true, vec![(true, sides)]);

  let trim = convert(&b.m, face);
  let l = &trim.loops[0];
  assert!(l.curves.iter().all(|c| c.curve.degree() == 1));
  let (lo, hi) = l
    .curves
    .iter()
    .fold((Vec2::new(9., 9.), Vec2::new(-9., -9.)), |(lo, hi), c| {
      let (a, b) = c.curve.control_bbox();
      (
        Vec2::new(lo.x.min(a.x), lo.y.min(a.y)),
        Vec2::new(hi.x.max(b.x), hi.y.max(b.y)),
      )
    });
  assert_eq!((lo, hi), (Vec2::new(0., 0.), Vec2::new(3., 3.)));
  assert_eq!(trim.param_space.patches.len(), 9);
}

/// Millimetre / degree context with a 0.01 mm uncertainty (`#1`), a 2D parametric context
/// (`#7`) and the world placement `#10`.
const DEGREE_CONTEXT: &str = "
#1=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#6))GLOBAL_UNIT_ASSIGNED_CONTEXT((#2,#4,#5))REPRESENTATION_CONTEXT('',''));
#2=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#3=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#4);
#4=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));
#5=(CONVERSION_BASED_UNIT('DEGREE',#3)NAMED_UNIT(*)PLANE_ANGLE_UNIT());
#6=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#2,'distance_accuracy_value','');
#7=(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('',''));
#8=CARTESIAN_POINT('',(0.,0.,0.));
#9=DIRECTION('',(0.,0.,1.));
#10=AXIS2_PLACEMENT_3D('',#8,#9,$);
";

/// A full cylinder side (radius 2, height 5, degrees) bounded by two circles and a seam edge
/// with pcurves at u = 0 and u = 360 degrees. The loop is listed counter-clockwise in uv, its
/// `FACE_BOUND .F.` and the face `same_sense .F.` compensate each other.
const CYLINDER_WITH_SEAM: &str = "
#20=CYLINDRICAL_SURFACE('',#10,2.);
#21=CARTESIAN_POINT('',(2.,0.,0.));
#22=CARTESIAN_POINT('',(2.,0.,5.));
#23=VERTEX_POINT('',#21);
#24=VERTEX_POINT('',#22);
#25=CIRCLE('',#10,2.);
#26=CARTESIAN_POINT('',(0.,0.,5.));
#27=AXIS2_PLACEMENT_3D('',#26,#9,$);
#28=CIRCLE('',#27,2.);
#29=EDGE_CURVE('',#23,#23,#25,.T.);
#30=EDGE_CURVE('',#24,#24,#28,.T.);
#31=DIRECTION('',(0.,0.,1.));
#32=VECTOR('',#31,1.);
#33=LINE('',#21,#32);
#40=CARTESIAN_POINT('',(0.,0.));
#41=DIRECTION('',(0.,1.));
#42=VECTOR('',#41,1.);
#43=LINE('',#40,#42);
#44=DEFINITIONAL_REPRESENTATION('',(#43),#7);
#45=PCURVE('',#20,#44);
#46=CARTESIAN_POINT('',(360.,0.));
#47=LINE('',#46,#42);
#48=DEFINITIONAL_REPRESENTATION('',(#47),#7);
#49=PCURVE('',#20,#48);
#50=SEAM_CURVE('',#33,(#45,#49),.PCURVE_S1.);
#51=EDGE_CURVE('',#23,#24,#50,.T.);
#52=ORIENTED_EDGE('',*,*,#29,.T.);
#53=ORIENTED_EDGE('',*,*,#51,.T.);
#54=ORIENTED_EDGE('',*,*,#30,.F.);
#55=ORIENTED_EDGE('',*,*,#51,.F.);
#56=EDGE_LOOP('',(#52,#53,#54,#55));
#57=FACE_BOUND('',#56,.F.);
#58=ADVANCED_FACE('',(#57),#20,.F.);
#59=CLOSED_SHELL('',(#58));
#60=MANIFOLD_SOLID_BREP('',#59);
#61=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#60,#10),#1);
";

#[test]
fn cylinder_seam_pcurves() {
  let step = format!(
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
     FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\n\
     DATA;\n{DEGREE_CONTEXT}{CYLINDER_WITH_SEAM}\nENDSEC;\nEND-ISO-10303-21;\n"
  );
  let parsed = Table::from_step(&step, None);
  assert!(matches!(&parsed.errors, TableParseErrors::EntityErrors(e) if e.is_empty()));
  let model = build_brep_model(&parsed.table);
  let trim = convert(&model, FaceId(0));
  assert!(trim.flip_normal);
  assert_eq!(trim.fitted_curve_count, 0);
  assert_eq!(trim.loops[0].winding, [0, 0]);
  assert!(trim.loops[0].curves.iter().all(|c| c.exact));
  assert!(p_area(&trim.loops[0].curves) > 0.);
  // The loop covers the full turn: u spans exactly one P period.
  let xs: Vec<_> = trim.loops[0]
    .curves
    .iter()
    .map(|c| c.curve.start().x)
    .collect();
  let span =
    xs.iter().fold(f64::MIN, |m, x| m.max(*x)) - xs.iter().fold(f64::MAX, |m, x| m.min(*x));
  assert_eq!(span, 4.);
}

#[test]
fn full_surface_without_loops() {
  let mut b = Builder::default();
  let torus = b.surface(SurfaceGeom::Torus {
    frame: std_frame(Vec3::new(0., 0., 0.)),
    major_radius: 3.,
    minor_radius: 1.,
  });
  let face = b.face(torus, true, Vec::new());
  let ctx = ctx();
  let surface = build_surface(&b.m, torus, &ctx).unwrap();
  let trim = build_face_trim(
    &b.m,
    face,
    surface.as_ref(),
    &mut Edge3Cache::default(),
    &ctx,
  )
  .unwrap();
  assert!(trim.loops.is_empty());
  assert_eq!(trim.p_period, [Some(4.), Some(4.)]);
  assert_eq!(trim.param_space.patches.len(), 16);

  let plane = b.surface(SurfaceGeom::Plane(std_frame(Vec3::new(0., 0., 0.))));
  let face = b.face(plane, true, Vec::new());
  let surface = build_surface(&b.m, plane, &ctx).unwrap();
  let err = build_face_trim(
    &b.m,
    face,
    surface.as_ref(),
    &mut Edge3Cache::default(),
    &ctx,
  );
  assert!(matches!(err, Err(e) if e.kind == ErrorKind::Topology));
}

#[test]
fn band_loops_on_doubly_bounded_face() {
  // A torus band between two parallels (v iso circles), material between them.
  let mut b = Builder::default();
  let (big, small) = (3., 1.);
  let torus = b.surface(SurfaceGeom::Torus {
    frame: std_frame(Vec3::new(0., 0., 0.)),
    major_radius: big,
    minor_radius: small,
  });
  let parallel = |b: &mut Builder, v: f64| {
    let rho = big + small * v.cos();
    let vertex = b.vertex(Vec3::new(rho, 0., small * v.sin()));
    b.circle_edge(vertex, std_frame(Vec3::new(0., 0., small * v.sin())), rho)
  };
  let lower = parallel(&mut b, -0.3);
  let upper = parallel(&mut b, 0.8);
  let face = b.face(
    torus,
    true,
    vec![(true, vec![(lower, true)]), (true, vec![(upper, false)])],
  );
  let trim = convert(&b.m, face);
  assert_eq!(trim.p_period, [Some(4.), None]);
  assert!(trim.loops.iter().flat_map(|l| &l.curves).all(|c| c.exact));
}

#[test]
fn cylinder_patch_with_hole_across_angle_cut() {
  // Outer loop over u ∈ [2.5, 3.8] and a hole over u ∈ [3.3, 3.5]: the hole projects on the
  // other side of the ±π cut and must be moved next to the outer loop, the rect stays partial.
  let mut b = Builder::default();
  let cyl = cylinder(&mut b);
  let at = |b: &mut Builder, u: f64, z: f64| b.vertex(Vec3::new(2. * u.cos(), 2. * u.sin(), z));
  let rectangle = |b: &mut Builder, (u0, u1): (f64, f64), (z0, z1): (f64, f64)| {
    let v00 = at(b, u0, z0);
    let v10 = at(b, u1, z0);
    let v11 = at(b, u1, z1);
    let v01 = at(b, u0, z1);
    let bottom = b.curve(CurveGeom::Circle {
      frame: std_frame(Vec3::new(0., 0., z0)),
      radius: 2.,
    });
    let top = b.curve(CurveGeom::Circle {
      frame: std_frame(Vec3::new(0., 0., z1)),
      radius: 2.,
    });
    let e0 = b.edge(v00, v10, bottom, true);
    let e1 = b.line_edge(v10, v11);
    let e2 = b.edge(v01, v11, top, true);
    let e3 = b.line_edge(v00, v01);
    vec![(e0, true), (e1, true), (e2, false), (e3, false)]
  };
  let outer = rectangle(&mut b, (2.5, 3.8), (0., 4.));
  let hole: Vec<_> = rectangle(&mut b, (3.3, 3.5), (1., 2.))
    .into_iter()
    .rev()
    .map(|(e, o)| (e, !o))
    .collect();
  let face = b.face(cyl, true, vec![(true, outer), (true, hole)]);

  let trim = convert(&b.m, face);
  assert_eq!(trim.p_period, [None, None]);
  assert!(trim.loops.iter().flat_map(|l| &l.curves).all(|c| c.exact));
  assert!(p_area(&trim.loops[0].curves) > 0.);
  assert!(p_area(&trim.loops[1].curves) < 0.);
  let bbox = |l: &PLoop| {
    l.curves.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| {
      let (a, b) = c.curve.control_bbox();
      (lo.min(a.x), hi.max(b.x))
    })
  };
  let (outer, hole) = (bbox(&trim.loops[0]), bbox(&trim.loops[1]));
  assert!(outer.0 < hole.0 && hole.1 < outer.1);
}

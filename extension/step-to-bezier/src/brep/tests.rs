use rendiation_step_reader::table::{Table, TableParseErrors};

use super::*;

/// Wrap DATA section entities into a complete exchange structure and resolve it. Parse errors
/// fail the test.
fn model(data: &str) -> BrepModel {
  let step = format!(
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
     FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\n\
     DATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
  );
  let parsed = Table::from_step(&step, None);
  match &parsed.errors {
    TableParseErrors::EntityErrors(e) if e.is_empty() => {}
    e => panic!("parse errors: {e:?}"),
  }
  build_brep_model(&parsed.table)
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

/// A full cylinder side face (radius 2, height 5, degrees) bounded by two circles and a seam
/// edge carrying two pcurves at u = 0 and u = 360 degrees.
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
#58=ADVANCED_FACE('',(#57),#20,.T.);
#59=CLOSED_SHELL('',(#58));
#60=MANIFOLD_SOLID_BREP('',#59);
#61=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#60,#10),#1);
";

#[test]
fn cylinder_with_seam() {
  let m = model(&format!("{DEGREE_CONTEXT}{CYLINDER_WITH_SEAM}"));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  assert_eq!(m.faces.len(), 1);
  assert_eq!(m.edges.len(), 3, "the seam edge used twice is one edge");
  assert_eq!(m.vertices.len(), 2);
  assert_eq!(m.bodies.len(), 1);
  assert_eq!(m.occurrences.len(), 1);
  assert_eq!(m.length_unit_mm, Some(1.0));
  assert_eq!(m.uncertainty, Some(0.01));
  let rad = m.plane_angle_unit_rad.unwrap();
  assert!((rad - std::f64::consts::PI / 180.0).abs() < 1e-15);

  let face = &m.faces[0];
  assert!(
    matches!(m.surfaces[face.surface.index()], SurfaceGeom::Cylinder { radius, .. } if radius == 2.0)
  );
  let bound = &face.bounds[0];
  assert!(!bound.orientation);
  assert!(!bound.is_outer);
  let BoundKind::Edges(uses) = &bound.kind else {
    panic!("edge loop expected");
  };
  assert_eq!(uses.len(), 4);
  assert_eq!(uses[1].edge, uses[3].edge);
  assert!(uses[1].orientation && !uses[3].orientation);

  let seam = &m.edges[uses[1].edge.index()];
  assert!(seam.is_seam);
  assert_eq!(seam.pcurves.len(), 2);
  assert!(matches!(
    m.curves[seam.curve.index()],
    CurveGeom::Line { .. }
  ));
  // The second pcurve is at u = 360 degrees, converted to radians.
  let Curve2Geom::Line { origin, vector } = &m.curves_2d[seam.pcurves[1].curve.index()] else {
    panic!("2D line expected");
  };
  assert!((origin.x - std::f64::consts::TAU).abs() < 1e-12);
  assert_eq!(vector.y, 1.0);
  assert_eq!(seam.pcurves[0].surface, face.surface);

  let circle = &m.edges[uses[0].edge.index()];
  assert_eq!(circle.start, circle.end);
  assert!(!circle.is_seam && circle.pcurves.is_empty());
}

#[test]
fn trimmed_circle_and_cone_angles() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CIRCLE('',#10,3.);
#21=CARTESIAN_POINT('',(0.,3.,0.));
#22=TRIMMED_CURVE('',#20,(PARAMETER_VALUE(0.)),(#21,PARAMETER_VALUE(90.)),.T.,.PARAMETER.);
#23=CARTESIAN_POINT('',(3.,0.,0.));
#24=VERTEX_POINT('',#23);
#25=VERTEX_POINT('',#21);
#26=EDGE_CURVE('',#24,#25,#22,.F.);
#27=ORIENTED_EDGE('',*,*,#26,.T.);
#28=EDGE_LOOP('',(#27));
#29=FACE_OUTER_BOUND('',#28,.T.);
#30=CONICAL_SURFACE('',#10,3.,30.);
#31=ADVANCED_FACE('',(#29),#30,.F.);
#32=OPEN_SHELL('',(#31));
#33=SHELL_BASED_SURFACE_MODEL('',(#32));
"
  ));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  let face = &m.faces[0];
  assert!(!face.same_sense);
  assert!(face.bounds[0].is_outer);
  let SurfaceGeom::Cone {
    semi_angle, radius, ..
  } = m.surfaces[face.surface.index()]
  else {
    panic!("cone expected");
  };
  assert_eq!(radius, 3.0);
  assert!((semi_angle - std::f64::consts::FRAC_PI_6).abs() < 1e-12);

  let edge = &m.edges[0];
  assert!(!edge.same_sense);
  let CurveGeom::Trimmed {
    basis,
    trim1,
    trim2,
    sense_agreement,
  } = &m.curves[edge.curve.index()]
  else {
    panic!("trimmed curve expected");
  };
  assert!(*sense_agreement);
  assert!(matches!(m.curves[basis.index()], CurveGeom::Circle { radius, .. } if radius == 3.0));
  assert!(matches!(trim1.as_slice(), [TrimSelect::Parameter(p)] if *p == 0.0));
  match trim2.as_slice() {
    [TrimSelect::Point(p), TrimSelect::Parameter(v)] => {
      assert_eq!(*p, Vec3::new(0.0, 3.0, 0.0));
      assert!((v - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }
    other => panic!("unexpected trims {other:?}"),
  }
}

#[test]
fn bspline_curves_and_frames() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CARTESIAN_POINT('',(0.,0.,0.));
#21=CARTESIAN_POINT('',(1.,1.,0.));
#22=CARTESIAN_POINT('',(2.,1.,0.));
#23=CARTESIAN_POINT('',(3.,0.,0.));
#24=CARTESIAN_POINT('',(4.,0.,0.));
#25=B_SPLINE_CURVE_WITH_KNOTS('',2,(#20,#21,#22,#23,#24),.UNSPECIFIED.,.F.,.F.,(3,1,1,3),(0.,1.,2.,3.),.UNSPECIFIED.);
#26=(BOUNDED_CURVE()B_SPLINE_CURVE(2,(#20,#21,#22),.UNSPECIFIED.,.F.,.F.)B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.PIECEWISE_BEZIER_KNOTS.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE((1.,0.5,1.))REPRESENTATION_ITEM(''));
#27=QUASI_UNIFORM_CURVE('',2,(#20,#21,#22,#23,#24),.UNSPECIFIED.,.F.,.F.);
#28=(BOUNDED_CURVE()B_SPLINE_CURVE(2,(#20,#21,#22,#23,#24),.UNSPECIFIED.,.F.,.F.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE((1.,2.,3.,2.,1.))REPRESENTATION_ITEM('')UNIFORM_CURVE());
#29=VERTEX_POINT('',#20);
#30=VERTEX_POINT('',#24);
#31=VERTEX_POINT('',#22);
#40=EDGE_CURVE('',#29,#30,#25,.T.);
#41=EDGE_CURVE('',#29,#31,#26,.T.);
#42=EDGE_CURVE('',#29,#30,#27,.T.);
#43=EDGE_CURVE('',#29,#30,#28,.T.);
#50=ORIENTED_EDGE('',*,*,#40,.T.);
#51=ORIENTED_EDGE('',*,*,#41,.T.);
#52=ORIENTED_EDGE('',*,*,#42,.T.);
#53=ORIENTED_EDGE('',*,*,#43,.T.);
#54=EDGE_LOOP('',(#50,#51,#52,#53));
#55=FACE_OUTER_BOUND('',#54,.T.);
#60=DIRECTION('',(1.,0.,0.));
#61=DIRECTION('',(2.,0.,0.));
#62=AXIS2_PLACEMENT_3D('',#20,#60,#61);
#63=PLANE('',#62);
#64=ADVANCED_FACE('',(#55),#63,.T.);
#65=OPEN_SHELL('',(#64));
#66=SHELL_BASED_SURFACE_MODEL('',(#65));
"
  ));
  assert_eq!(m.edges.len(), 4);
  let bspline = |i: usize| match &m.curves[m.edges[i].curve.index()] {
    CurveGeom::BSpline(b) => b.clone(),
    other => panic!("B-spline expected, got {other:?}"),
  };
  let b = bspline(0);
  assert_eq!(b.knots, vec![0., 0., 0., 1., 2., 3., 3., 3.]);
  assert_eq!(b.closed, Some(false));
  assert!(b.weights.is_none());
  let b = bspline(1);
  assert_eq!(b.knots, vec![0., 0., 0., 1., 1., 1.]);
  assert_eq!(b.weights, Some(vec![1., 0.5, 1.]));
  let b = bspline(2);
  assert_eq!(b.knots, vec![0., 0., 0., 1., 2., 3., 3., 3.]);
  let b = bspline(3);
  assert_eq!(b.knots, vec![-2., -1., 0., 1., 2., 3., 4., 5.]);
  assert_eq!(b.weights.as_ref().map(|w| w.len()), Some(5));

  // Reference direction parallel to the axis: warned, a perpendicular is picked.
  assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
  let SurfaceGeom::Plane(f) = &m.surfaces[0] else {
    panic!("plane expected");
  };
  assert_eq!(f.z, Vec3::new(1.0, 0.0, 0.0));
  assert_eq!(f.x, Vec3::new(0.0, 1.0, 0.0));
  assert!(f.x.dot(f.z).abs() < 1e-15 && (f.x.length() - 1.0).abs() < 1e-15);
  assert!((f.x.cross(f.y) - f.z).length() < 1e-15);
}

#[test]
fn default_frame_axes() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CARTESIAN_POINT('',(1.,2.,3.));
#21=AXIS2_PLACEMENT_3D('',#20,$,$);
#22=DIRECTION('',(1.,1.,0.));
#23=AXIS2_PLACEMENT_3D('',#20,#9,#22);
#24=PLANE('',#21);
#25=PLANE('',#23);
#26=VERTEX_POINT('',#20);
#27=VERTEX_LOOP('',#26);
#28=FACE_BOUND('',#27,.T.);
#29=ADVANCED_FACE('',(#28),#24,.T.);
#30=ADVANCED_FACE('',(#28),#25,.T.);
#31=OPEN_SHELL('',(#29,#30));
#32=SHELL_BASED_SURFACE_MODEL('',(#31));
"
  ));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  let SurfaceGeom::Plane(a) = &m.surfaces[0] else {
    panic!();
  };
  assert_eq!(a.origin, Vec3::new(1.0, 2.0, 3.0));
  assert_eq!(
    (a.x, a.y, a.z),
    (
      Vec3::new(1., 0., 0.),
      Vec3::new(0., 1., 0.),
      Vec3::new(0., 0., 1.)
    )
  );
  let SurfaceGeom::Plane(b) = &m.surfaces[1] else {
    panic!();
  };
  let s = std::f64::consts::FRAC_1_SQRT_2;
  assert!((b.x - Vec3::new(s, s, 0.0)).length() < 1e-15);
  assert!((b.y - Vec3::new(-s, s, 0.0)).length() < 1e-15);
  assert!(matches!(m.faces[0].bounds[0].kind, BoundKind::Vertex(_)));
}

#[test]
fn shared_entities_and_orientations() {
  // Two plane faces sharing an edge, one used through an ORIENTED_FACE .F., and a void shell
  // given as ORIENTED_CLOSED_SHELL .F. reusing the second face.
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CARTESIAN_POINT('',(0.,0.,0.));
#21=CARTESIAN_POINT('',(1.,0.,0.));
#22=VERTEX_POINT('',#20);
#23=VERTEX_POINT('',#21);
#24=DIRECTION('',(1.,0.,0.));
#25=VECTOR('',#24,2.);
#26=LINE('',#20,#25);
#27=EDGE_CURVE('',#22,#23,#26,.T.);
#28=ORIENTED_EDGE('',*,*,#27,.T.);
#29=ORIENTED_EDGE('',*,*,#27,.F.);
#30=EDGE_LOOP('',(#28));
#31=EDGE_LOOP('',(#29));
#32=FACE_OUTER_BOUND('',#30,.T.);
#33=FACE_OUTER_BOUND('',#31,.T.);
#34=PLANE('',#10);
#35=ADVANCED_FACE('',(#32),#34,.T.);
#36=ADVANCED_FACE('',(#33),#34,.F.);
#37=ORIENTED_FACE('',*,#36,.F.);
#38=CLOSED_SHELL('',(#35,#37));
#39=CLOSED_SHELL('',(#36));
#40=ORIENTED_CLOSED_SHELL('',*,#39,.F.);
#41=BREP_WITH_VOIDS('',#38,(#40));
"
  ));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  assert_eq!(m.faces.len(), 2);
  assert_eq!(m.edges.len(), 1);
  assert_eq!(m.surfaces.len(), 1);
  let CurveGeom::Line { vector, .. } = &m.curves[0] else {
    panic!();
  };
  assert_eq!(*vector, Vec3::new(2.0, 0.0, 0.0));
  let body = &m.bodies[0];
  assert_eq!(body.shells.len(), 2);
  assert_eq!(body.shells[0], vec![(FaceId(0), true), (FaceId(1), false)]);
  assert_eq!(body.shells[1], vec![(FaceId(1), false)]);
}

#[test]
fn unresolvable_entities_are_skipped() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CARTESIAN_POINT('',(0.,0.,0.));
#22=VERTEX_POINT('',#20);
#27=EDGE_CURVE('',#22,#22,#999,.T.);
#28=ORIENTED_EDGE('',*,*,#27,.T.);
#30=EDGE_LOOP('',(#28));
#32=FACE_OUTER_BOUND('',#30,.T.);
#34=PLANE('',#10);
#35=ADVANCED_FACE('',(#32),#34,.T.);
#36=ADVANCED_FACE('',(#32),#34,.T.);
#37=VERTEX_LOOP('',#22);
#38=FACE_BOUND('',#37,.T.);
#39=ADVANCED_FACE('',(#38),#998,.T.);
#40=ADVANCED_FACE('',(#38),#34,.T.);
#41=CLOSED_SHELL('',(#35,#36,#39,#40));
#42=MANIFOLD_SOLID_BREP('',#41);
"
  ));
  assert_eq!(m.faces.len(), 1);
  assert_eq!(m.edges.len(), 0);
  assert_eq!(m.bodies[0].shells[0].len(), 1);
  let joined = m.warnings.join("\n");
  assert!(joined.contains("edge #27 skipped: curve #999"), "{joined}");
  assert!(
    joined.contains("face #35 skipped: uses skipped edge #27"),
    "{joined}"
  );
  assert!(
    joined.contains("face #36 skipped: uses skipped edge #27"),
    "{joined}"
  );
  assert!(
    joined.contains("face #39 skipped: surface #998"),
    "{joined}"
  );
  assert_eq!(m.warnings.len(), 4, "{joined}");
}

#[test]
fn inch_units() {
  let m = model(
    "
#1=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#6))GLOBAL_UNIT_ASSIGNED_CONTEXT((#5,#4))REPRESENTATION_CONTEXT('',''));
#2=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#3=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#2);
#4=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));
#5=(CONVERSION_BASED_UNIT('INCH',#3)LENGTH_UNIT()NAMED_UNIT(*));
#6=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#2,'distance_accuracy_value','');
#7=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));
",
  );
  assert_eq!(m.length_unit_mm, Some(25.4));
  assert_eq!(m.plane_angle_unit_rad, Some(1.0));
  // 0.0254 mm expressed in inches.
  assert!((m.uncertainty.unwrap() - 0.001).abs() < 1e-15);
}

/// Child part: a plane face with a vertex loop in `#210` / `#220`, the root product `#100`
/// places it with `#240` (translation (10, 0, 0), rotation of 90 degrees around z).
fn assembly(relationship: &str) -> String {
  format!(
    "{DEGREE_CONTEXT}
#100=PRODUCT_DEFINITION('root','',#900,#901);
#101=PRODUCT_DEFINITION_SHAPE('','',#100);
#102=SHAPE_DEFINITION_REPRESENTATION(#101,#110);
#110=SHAPE_REPRESENTATION('',(#10,#240),#1);
#200=PRODUCT_DEFINITION('child','',#900,#901);
#201=PRODUCT_DEFINITION_SHAPE('','',#200);
#202=SHAPE_DEFINITION_REPRESENTATION(#201,#210);
#210=SHAPE_REPRESENTATION('',(#10),#1);
#211=SHAPE_REPRESENTATION_RELATIONSHIP('','',#220,#210);
#220=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#10,#236),#1);
#230=PLANE('',#10);
#231=CARTESIAN_POINT('',(1.,0.,0.));
#232=VERTEX_POINT('',#231);
#233=VERTEX_LOOP('',#232);
#234=FACE_BOUND('',#233,.T.);
#235=ADVANCED_FACE('',(#234),#230,.T.);
#237=CLOSED_SHELL('',(#235));
#236=MANIFOLD_SOLID_BREP('',#237);
#238=CARTESIAN_POINT('',(10.,0.,0.));
#239=DIRECTION('',(0.,1.,0.));
#240=AXIS2_PLACEMENT_3D('',#238,#9,#239);
#300=NEXT_ASSEMBLY_USAGE_OCCURRENCE('','','',#100,#200,$);
#301=PRODUCT_DEFINITION_SHAPE('','',#300);
#302=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#310,#301);
{relationship}
"
  )
}

fn check_child_placement(m: &BrepModel) {
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  assert_eq!(m.occurrences.len(), 1);
  let o = &m.occurrences[0];
  let p = o.transform * m.vertices[0].position;
  assert!((p - Vec3::new(10.0, 1.0, 0.0)).length() < 1e-12, "{p:?}");
  assert!(o.path.contains("NAUO #300"), "{}", o.path);
}

#[test]
fn assembly_nauo_with_item_defined_transformation() {
  // Standard order: rep_1 is the child representation, item_1 belongs to rep_1.
  let m = model(&assembly(
    "#310=(REPRESENTATION_RELATIONSHIP('','',#210,#110)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#311)SHAPE_REPRESENTATION_RELATIONSHIP());
#311=ITEM_DEFINED_TRANSFORMATION('','',#10,#240);",
  ));
  check_child_placement(&m);

  // Reversed order: rep_1 is the parent, so item_1 (#240) is in the parent.
  let m = model(&assembly(
    "#310=(REPRESENTATION_RELATIONSHIP('','',#110,#210)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#311)SHAPE_REPRESENTATION_RELATIONSHIP());
#311=ITEM_DEFINED_TRANSFORMATION('','',#240,#10);",
  ));
  check_child_placement(&m);
}

#[test]
fn mapped_item_placement() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#100=PRODUCT_DEFINITION('root','',#900,#901);
#101=PRODUCT_DEFINITION_SHAPE('','',#100);
#102=SHAPE_DEFINITION_REPRESENTATION(#101,#110);
#110=SHAPE_REPRESENTATION('',(#10,#250,#251),#1);
#220=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#10,#236),#1);
#230=PLANE('',#10);
#231=CARTESIAN_POINT('',(1.,0.,0.));
#232=VERTEX_POINT('',#231);
#233=VERTEX_LOOP('',#232);
#234=FACE_BOUND('',#233,.T.);
#235=ADVANCED_FACE('',(#234),#230,.T.);
#237=CLOSED_SHELL('',(#235));
#236=MANIFOLD_SOLID_BREP('',#237);
#238=CARTESIAN_POINT('',(10.,0.,0.));
#239=DIRECTION('',(0.,1.,0.));
#240=AXIS2_PLACEMENT_3D('',#238,#9,#239);
#241=CARTESIAN_POINT('',(0.,0.,7.));
#242=AXIS2_PLACEMENT_3D('',#241,#9,$);
#249=REPRESENTATION_MAP(#10,#220);
#250=MAPPED_ITEM('',#249,#240);
#251=MAPPED_ITEM('',#249,#242);
"
  ));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  assert_eq!(m.occurrences.len(), 2);
  let p: Vec<_> = m
    .occurrences
    .iter()
    .map(|o| o.transform * m.vertices[0].position)
    .collect();
  assert!((p[0] - Vec3::new(10.0, 1.0, 0.0)).length() < 1e-12, "{p:?}");
  assert!((p[1] - Vec3::new(1.0, 0.0, 7.0)).length() < 1e-12, "{p:?}");
}

#[test]
fn surface_of_revolution_and_unsupported_surface() {
  let m = model(&format!(
    "{DEGREE_CONTEXT}
#20=CARTESIAN_POINT('',(1.,0.,0.));
#21=CARTESIAN_POINT('',(2.,0.,1.));
#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#20,#21),.UNSPECIFIED.,.F.,.F.,(2,2),(0.,1.),.UNSPECIFIED.);
#23=AXIS1_PLACEMENT('',#8,$);
#24=SURFACE_OF_REVOLUTION('',#22,#23);
#25=VERTEX_POINT('',#20);
#26=VERTEX_LOOP('',#25);
#27=FACE_BOUND('',#26,.T.);
#28=ADVANCED_FACE('',(#27),#24,.T.);
#29=DEGENERATE_TOROIDAL_SURFACE('',#10,1.,2.,.T.);
#30=ADVANCED_FACE('',(#27),#29,.T.);
#31=OPEN_SHELL('',(#28,#30));
#32=SHELL_BASED_SURFACE_MODEL('',(#31));
"
  ));
  assert!(m.warnings.is_empty(), "{:?}", m.warnings);
  assert_eq!(m.faces.len(), 2);
  let SurfaceGeom::Revolution {
    axis_dir,
    axis_origin,
    curve,
  } = &m.surfaces[0]
  else {
    panic!("revolution expected");
  };
  assert_eq!(*axis_dir, Vec3::new(0.0, 0.0, 1.0));
  assert_eq!(*axis_origin, Vec3::new(0.0, 0.0, 0.0));
  assert!(matches!(m.curves[curve.index()], CurveGeom::BSpline(_)));
  assert!(matches!(
    &m.surfaces[1],
    SurfaceGeom::Unsupported { type_name, step_id: 29 } if type_name == "DEGENERATE_TOROIDAL_SURFACE"
  ));
}

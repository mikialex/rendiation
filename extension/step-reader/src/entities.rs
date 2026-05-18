use ruststep::{
  ast::{EntityInstance, Parameter, SubSuperRecord},
  primitive::Logical,
  tables::PlaceHolder,
  Holder,
};
use serde::{Deserialize, Serialize};

use crate::table::Table;

// ── Enums ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BSplineCurveForm {
  PolylineForm,
  CircularArc,
  EllipticArc,
  ParabolicArc,
  HyperbolicArc,
  Unspecified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotType {
  UniformKnots,
  Unspecified,
  QuasiUniformKnots,
  PiecewiseBezierKnots,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BSplineSurfaceForm {
  PlanarSurf,
  CylindricalSurf,
  ConicalSurf,
  SphericalSurf,
  ToroidalSurf,
  SurfOfRevolution,
  RuledSurf,
  GeneralisedCone,
  QuadricSurf,
  SurfOfLinearExtrusion,
  Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreferredSurfaceCurveRepresentation {
  Curve3D,
  PcurveS1,
  PcurveS2,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimmingPreference {
  Cartesian,
  Parameter,
  Unspecified,
}

// ── Primitives ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = cartesian_point)]
#[holder(generate_deserialize)]
pub struct CartesianPoint {
  pub label: String,
  pub coordinates: Vec<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = direction)]
#[holder(generate_deserialize)]
pub struct Direction {
  pub label: String,
  pub direction_ratios: Vec<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vector)]
#[holder(generate_deserialize)]
pub struct Vector {
  pub label: String,
  #[holder(use_place_holder)]
  pub orientation: Direction,
  pub magnitude: f64,
}

// ── Placements ──────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis1_placement)]
#[holder(generate_deserialize)]
pub struct Axis1Placement {
  pub label: String,
  #[holder(use_place_holder)]
  pub location: CartesianPoint,
  #[holder(use_place_holder)]
  pub axis: Direction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis2_placement_2d)]
#[holder(generate_deserialize)]
pub struct Axis2Placement2d {
  pub label: String,
  #[holder(use_place_holder)]
  pub location: CartesianPoint,
  #[holder(use_place_holder)]
  pub ref_direction: Direction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis2_placement_3d)]
#[holder(generate_deserialize)]
pub struct Axis2Placement3d {
  pub label: String,
  #[holder(use_place_holder)]
  pub location: CartesianPoint,
  #[holder(use_place_holder)]
  pub axis: Direction,
  #[holder(use_place_holder)]
  pub ref_direction: Direction,
}

// ── Curves ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = line)]
#[holder(generate_deserialize)]
pub struct Line {
  pub label: String,
  #[holder(use_place_holder)]
  pub pnt: CartesianPoint,
  #[holder(use_place_holder)]
  pub dir: Vector,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = polyline)]
#[holder(generate_deserialize)]
pub struct Polyline {
  pub label: String,
  #[holder(use_place_holder)]
  pub points: Vec<CartesianPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = circle)]
#[holder(generate_deserialize)]
pub struct Circle {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = ellipse)]
#[holder(generate_deserialize)]
pub struct Ellipse {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub semi_axis1: f64,
  pub semi_axis2: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = hyperbola)]
#[holder(generate_deserialize)]
pub struct Hyperbola {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub semi_axis: f64,
  pub semi_imag_axis: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = parabola)]
#[holder(generate_deserialize)]
pub struct Parabola {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub focal_dist: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = b_spline_curve_with_knots)]
#[holder(generate_deserialize)]
pub struct BSplineCurveWithKnots {
  pub label: String,
  pub degree: i64,
  #[holder(use_place_holder)]
  pub control_points_list: Vec<CartesianPoint>,
  pub curve_form: BSplineCurveForm,
  pub closed_curve: Logical,
  pub self_intersect: Logical,
  pub knot_multiplicities: Vec<i64>,
  pub knots: Vec<f64>,
  pub knot_spec: KnotType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = bezier_curve)]
#[holder(generate_deserialize)]
pub struct BezierCurve {
  pub label: String,
  pub degree: i64,
  #[holder(use_place_holder)]
  pub control_points_list: Vec<CartesianPoint>,
  pub curve_form: BSplineCurveForm,
  pub closed_curve: Logical,
  pub self_intersect: Logical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = rational_b_spline_curve)]
#[holder(generate_deserialize)]
pub struct RationalBSplineCurve {
  #[holder(use_place_holder)]
  pub non_rational_b_spline_curve: NonRationalBSplineCurve,
  pub weights_data: Vec<f64>,
}

/// Union of the non-rational B-spline curve subtypes that can appear inside
/// a `RATIONAL_B_SPLINE_CURVE` Complex Entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum NonRationalBSplineCurve {
  #[holder(use_place_holder)]
  BSplineCurveWithKnots(Box<BSplineCurveWithKnots>),
  #[holder(use_place_holder)]
  BezierCurve(Box<BezierCurve>),
  #[holder(use_place_holder)]
  QuasiUniformCurve(Box<BSplineCurveWithKnots>),
  #[holder(use_place_holder)]
  UniformCurve(Box<BSplineCurveWithKnots>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = trimmed_curve)]
#[holder(generate_deserialize)]
pub struct TrimmedCurve {
  pub label: String,
  #[holder(use_place_holder)]
  pub basis_curve: CurveAny,
  pub trim_1: Vec<TrimSelect>,
  pub trim_2: Vec<TrimSelect>,
  pub sense_agreement: bool,
  pub master_representation: TrimmingPreference,
}

/// A trimming-select element: either a numeric parameter value or a
/// reference to a CartesianPoint. The `#[serde(untagged)]` enum lets serde
/// try each variant in order — `f64` matches `Parameter::Real`, and
/// `PlaceHolder<CartesianPointHolder>` matches `Parameter::Ref(Name::Entity(id))`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum TrimSelect {
  /// Trimming at a parameter value along the curve.
  Value(f64),
  /// Trimming at a CartesianPoint reference or inline definition.
  /// Uses `CartesianPointHolder` (not `CartesianPoint`) so that
  /// `PlaceHolder` can deserialize both `#id` refs and inline data.
  Point(PlaceHolder<CartesianPointHolder>),
}

impl Serialize for TrimSelect {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    match self {
      TrimSelect::Value(v) => serializer.serialize_f64(*v),
      TrimSelect::Point(_) => serializer.serialize_str("#REF"),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionCode {
  Discontinuous,
  Continuous,
  ContSameGradient,
  ContSameGradientSameCurvature,
}

/// A segment within a `COMPOSITE_CURVE`. Although embedded (not a top-level
/// named entity in the STEP exchange structure), it derives `Holder` so that
/// its `parent_curve` field can use `PlaceHolder<CurveAny>` for cross-reference
/// deserialization. The `generate_deserialize` visitor reads fields positionally,
/// matching STEP's anonymous parameter-list format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = composite_curve_segment)]
#[holder(generate_deserialize)]
pub struct CompositeCurveSegment {
  pub transition: TransitionCode,
  pub same_sense: bool,
  #[holder(use_place_holder)]
  pub parent_curve: CurveAny,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = composite_curve)]
#[holder(generate_deserialize)]
pub struct CompositeCurve {
  pub label: String,
  #[holder(use_place_holder)]
  pub segments: Vec<CompositeCurveSegment>,
  pub self_intersect: Logical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = offset_curve_3d)]
#[holder(generate_deserialize)]
pub struct OffsetCurve3d {
  pub label: String,
  #[holder(use_place_holder)]
  pub basis_curve: CurveAny,
  pub distance: f64,
  #[holder(use_place_holder)]
  pub ref_direction: Direction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = pcurve)]
#[holder(generate_deserialize)]
pub struct Pcurve {
  pub label: String,
  #[holder(use_place_holder)]
  pub basis_surface: SurfaceAny,
  #[holder(use_place_holder)]
  pub reference_to_curve: DefinitionalRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_curve)]
#[holder(generate_deserialize)]
pub struct SurfaceCurve {
  pub label: String,
  #[holder(use_place_holder)]
  pub curve_3d: CurveAny,
  #[holder(use_place_holder)]
  pub associated_geometry: Vec<PcurveOrSurface>,
  pub master_representation: PreferredSurfaceCurveRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum PcurveOrSurface {
  #[holder(use_place_holder)]
  Pcurve(Box<Pcurve>),
  #[holder(use_place_holder)]
  Surface(Box<SurfaceAny>),
}

// ── Curve enum (union of all curve types) ───────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum CurveAny {
  #[holder(use_place_holder)]
  Line(Box<Line>),
  #[holder(use_place_holder)]
  Polyline(Box<Polyline>),
  #[holder(use_place_holder)]
  Circle(Box<Circle>),
  #[holder(use_place_holder)]
  Ellipse(Box<Ellipse>),
  #[holder(use_place_holder)]
  Hyperbola(Box<Hyperbola>),
  #[holder(use_place_holder)]
  Parabola(Box<Parabola>),
  #[holder(use_place_holder)]
  BSplineCurveWithKnots(Box<BSplineCurveWithKnots>),
  #[holder(use_place_holder)]
  BezierCurve(Box<BezierCurve>),
  #[holder(use_place_holder)]
  RationalBSplineCurve(Box<RationalBSplineCurve>),
  #[holder(use_place_holder)]
  TrimmedCurve(Box<TrimmedCurve>),
  #[holder(use_place_holder)]
  CompositeCurve(Box<CompositeCurve>),
  #[holder(use_place_holder)]
  OffsetCurve3d(Box<OffsetCurve3d>),
  #[holder(use_place_holder)]
  Pcurve(Box<Pcurve>),
  #[holder(use_place_holder)]
  SurfaceCurve(Box<SurfaceCurve>),
}

// ── Surfaces ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = plane)]
#[holder(generate_deserialize)]
pub struct Plane {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = spherical_surface)]
#[holder(generate_deserialize)]
pub struct SphericalSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = cylindrical_surface)]
#[holder(generate_deserialize)]
pub struct CylindricalSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = toroidal_surface)]
#[holder(generate_deserialize)]
pub struct ToroidalSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub major_radius: f64,
  pub minor_radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = conical_surface)]
#[holder(generate_deserialize)]
pub struct ConicalSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub position: Axis2Placement3d,
  pub radius: f64,
  pub semi_angle: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = b_spline_surface_with_knots)]
#[holder(generate_deserialize)]
pub struct BSplineSurfaceWithKnots {
  pub label: String,
  pub u_degree: i64,
  pub v_degree: i64,
  #[holder(use_place_holder)]
  pub control_points_list: Vec<Vec<CartesianPoint>>,
  pub surface_form: BSplineSurfaceForm,
  pub u_closed: Logical,
  pub v_closed: Logical,
  pub self_intersect: Logical,
  pub u_multiplicities: Vec<i64>,
  pub v_multiplicities: Vec<i64>,
  pub u_knots: Vec<f64>,
  pub v_knots: Vec<f64>,
  pub knot_spec: KnotType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = bezier_surface)]
#[holder(generate_deserialize)]
pub struct BezierSurface {
  pub label: String,
  pub u_degree: i64,
  pub v_degree: i64,
  #[holder(use_place_holder)]
  pub control_points_list: Vec<Vec<CartesianPoint>>,
  pub surface_form: BSplineSurfaceForm,
  pub u_closed: Logical,
  pub v_closed: Logical,
  pub self_intersect: Logical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = rational_b_spline_surface)]
#[holder(generate_deserialize)]
pub struct RationalBSplineSurface {
  #[holder(use_place_holder)]
  pub non_rational_b_spline_surface: NonRationalBSplineSurface,
  pub weights_data: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum NonRationalBSplineSurface {
  #[holder(use_place_holder)]
  BSplineSurfaceWithKnots(Box<BSplineSurfaceWithKnots>),
  #[holder(use_place_holder)]
  BezierSurface(Box<BezierSurface>),
  #[holder(use_place_holder)]
  QuasiUniformSurface(Box<BSplineSurfaceWithKnots>),
  #[holder(use_place_holder)]
  UniformSurface(Box<BSplineSurfaceWithKnots>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_linear_extrusion)]
#[holder(generate_deserialize)]
pub struct SurfaceOfLinearExtrusion {
  pub label: String,
  #[holder(use_place_holder)]
  pub swept_curve: CurveAny,
  #[holder(use_place_holder)]
  pub extrusion_axis: Vector,
  pub depth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_revolution)]
#[holder(generate_deserialize)]
pub struct SurfaceOfRevolution {
  pub label: String,
  #[holder(use_place_holder)]
  pub swept_curve: CurveAny,
  #[holder(use_place_holder)]
  pub axis_position: Axis1Placement,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = offset_surface)]
#[holder(generate_deserialize)]
pub struct OffsetSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub basis_surface: SurfaceAny,
  pub distance: f64,
  pub self_intersect: Logical,
}

// ── Surface enum ────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum SurfaceAny {
  #[holder(use_place_holder)]
  Plane(Box<Plane>),
  #[holder(use_place_holder)]
  SphericalSurface(Box<SphericalSurface>),
  #[holder(use_place_holder)]
  CylindricalSurface(Box<CylindricalSurface>),
  #[holder(use_place_holder)]
  ToroidalSurface(Box<ToroidalSurface>),
  #[holder(use_place_holder)]
  ConicalSurface(Box<ConicalSurface>),
  #[holder(use_place_holder)]
  BSplineSurfaceWithKnots(Box<BSplineSurfaceWithKnots>),
  #[holder(use_place_holder)]
  BezierSurface(Box<BezierSurface>),
  #[holder(use_place_holder)]
  RationalBSplineSurface(Box<RationalBSplineSurface>),
  #[holder(use_place_holder)]
  SurfaceOfLinearExtrusion(Box<SurfaceOfLinearExtrusion>),
  #[holder(use_place_holder)]
  SurfaceOfRevolution(Box<SurfaceOfRevolution>),
  #[holder(use_place_holder)]
  OffsetSurface(Box<OffsetSurface>),
}

// ── Topology ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vertex_point)]
#[holder(generate_deserialize)]
pub struct VertexPoint {
  pub label: String,
  #[holder(use_place_holder)]
  pub vertex_geometry: CartesianPoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_curve)]
#[holder(generate_deserialize)]
pub struct EdgeCurve {
  pub label: String,
  #[holder(use_place_holder)]
  pub edge_start: VertexPoint,
  #[holder(use_place_holder)]
  pub edge_end: VertexPoint,
  #[holder(use_place_holder)]
  pub edge_geometry: CurveAny,
  pub same_sense: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_edge)]
#[holder(generate_deserialize)]
pub struct OrientedEdge {
  pub label: String,
  #[holder(use_place_holder)]
  pub edge_element: EdgeCurve,
  pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_loop)]
#[holder(generate_deserialize)]
pub struct EdgeLoop {
  pub label: String,
  #[holder(use_place_holder)]
  pub edge_list: Vec<OrientedEdge>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_bound)]
#[holder(generate_deserialize)]
pub struct FaceBound {
  pub label: String,
  #[holder(use_place_holder)]
  pub bound: EdgeLoop,
  pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_surface)]
#[holder(generate_deserialize)]
pub struct FaceSurface {
  pub label: String,
  #[holder(use_place_holder)]
  pub bounds: Vec<FaceBound>,
  #[holder(use_place_holder)]
  pub face_geometry: SurfaceAny,
  pub same_sense: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_face)]
#[holder(generate_deserialize)]
pub struct OrientedFace {
  pub label: String,
  #[holder(use_place_holder)]
  pub face_element: FaceSurface,
  pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell)]
#[holder(generate_deserialize)]
pub struct Shell {
  pub label: String,
  #[holder(use_place_holder)]
  pub shell_element: Vec<OrientedFace>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_shell)]
#[holder(generate_deserialize)]
pub struct OrientedShell {
  pub label: String,
  #[holder(use_place_holder)]
  pub shell_element: Shell,
  pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell_based_surface_model)]
#[holder(generate_deserialize)]
pub struct ShellBasedSurfaceModel {
  pub label: String,
  #[holder(use_place_holder)]
  pub sbms_boundary: Vec<Shell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = manifold_solid_brep)]
#[holder(generate_deserialize)]
pub struct ManifoldSolidBrep {
  pub label: String,
  #[holder(use_place_holder)]
  pub outer: Shell,
  #[holder(use_place_holder)]
  pub voids: Vec<OrientedShell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = faceted_brep)]
#[holder(generate_deserialize)]
pub struct FacetedBrep {
  pub label: String,
  #[holder(use_place_holder)]
  pub outer: Shell,
}

// ── Assembly / Navigation ───────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation)]
#[holder(generate_deserialize)]
pub struct Representation {
  pub name: String,
  #[holder(use_place_holder)]
  pub items: Vec<RepresentationItem>,
  #[holder(use_place_holder)]
  pub context_of_items: RepresentationContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation_item)]
#[holder(generate_deserialize)]
pub struct RepresentationItem {
  pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation_context)]
#[holder(generate_deserialize)]
pub struct RepresentationContext {
  pub context_identifier: String,
  pub context_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = mapped_item)]
#[holder(generate_deserialize)]
pub struct MappedItem {
  pub label: String,
  #[holder(use_place_holder)]
  pub mapping_source: RepresentationItem,
  #[holder(use_place_holder)]
  pub mapping_target: RepresentationItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product)]
#[holder(generate_deserialize)]
pub struct Product {
  pub id: String,
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub frame_of_reference: Vec<RepresentationItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_formation)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionFormation {
  pub id: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub of_product: Product,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_context)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionContext {
  pub name: String,
  #[holder(use_place_holder)]
  pub life_cycle_stage: RepresentationItem,
  #[holder(use_place_holder)]
  pub frame_of_reference: RepresentationContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition)]
#[holder(generate_deserialize)]
pub struct ProductDefinition {
  pub id: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub formation: ProductDefinitionFormation,
  #[holder(use_place_holder)]
  pub frame_of_reference: ProductDefinitionContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_shape)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionShape {
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub definition: ProductDefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_definition_representation)]
#[holder(generate_deserialize)]
pub struct ShapeDefinitionRepresentation {
  pub label: String,
  #[holder(use_place_holder)]
  pub definition: ProductDefinitionShape,
  #[holder(use_place_holder)]
  pub used_representation: ShapeRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentation {
  pub name: String,
  #[holder(use_place_holder)]
  pub items: Vec<RepresentationItem>,
  #[holder(use_place_holder)]
  pub context_of_items: RepresentationContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = context_dependent_shape_representation)]
#[holder(generate_deserialize)]
pub struct ContextDependentShapeRepresentation {
  pub name: String,
  #[holder(use_place_holder)]
  pub representation_relation: ShapeRepresentationRelationship,
  #[holder(use_place_holder)]
  pub represented_product_relation: ProductDefinitionShape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation_relationship)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentationRelationship {
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub rep_1: ShapeRepresentation,
  #[holder(use_place_holder)]
  pub rep_2: ShapeRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation_relationship_with_transformation)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentationRelationshipWithTransformation {
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub rep_1: ShapeRepresentation,
  #[holder(use_place_holder)]
  pub rep_2: ShapeRepresentation,
  #[holder(use_place_holder)]
  pub transformation_operator: ItemDefinedTransformation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = next_assembly_usage_occurrence)]
#[holder(generate_deserialize)]
pub struct NextAssemblyUsageOccurrence {
  pub id: String,
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub relating_product_definition: ProductDefinition,
  #[holder(use_place_holder)]
  pub related_product_definition: ProductDefinition,
  pub reference_designator: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = item_defined_transformation)]
#[holder(generate_deserialize)]
pub struct ItemDefinedTransformation {
  pub name: String,
  pub description: String,
  #[holder(use_place_holder)]
  pub transform_item_1: RepresentationItem,
  #[holder(use_place_holder)]
  pub transform_item_2: RepresentationItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = geometric_set)]
#[holder(generate_deserialize)]
pub struct GeometricSet {
  pub label: String,
  #[holder(use_place_holder)]
  pub elements: Vec<RepresentationItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = geometric_curve_set)]
#[holder(generate_deserialize)]
pub struct GeometricCurveSet {
  pub label: String,
  #[holder(use_place_holder)]
  pub elements: Vec<RepresentationItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = definitional_representation)]
#[holder(generate_deserialize)]
pub struct DefinitionalRepresentation {
  pub label: String,
  #[holder(use_place_holder)]
  pub representation_item: Vec<RepresentationItem>,
  #[holder(use_place_holder)]
  pub context_of_items: RepresentationContext,
}

// ── Presentation / Visual ───────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = styled_item)]
#[holder(generate_deserialize)]
pub struct StyledItem {
  pub name: String,
  #[holder(use_place_holder)]
  pub item: RepresentationItem,
  #[holder(use_place_holder)]
  pub styles: Vec<PresentationStyleAssignment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = presentation_style_assignment)]
#[holder(generate_deserialize)]
pub struct PresentationStyleAssignment {
  pub label: String,
  #[holder(use_place_holder)]
  pub styles: Vec<SurfaceStyleUsage>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_style_usage)]
#[holder(generate_deserialize)]
pub struct SurfaceStyleUsage {
  pub side: SurfaceSide,
  #[holder(use_place_holder)]
  pub style: SurfaceSideStyle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceSide {
  Positive,
  Negative,
  Both,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_side_style)]
#[holder(generate_deserialize)]
pub struct SurfaceSideStyle {
  pub name: String,
  #[holder(use_place_holder)]
  pub styles: Vec<PresentationStyleSelect>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum PresentationStyleSelect {
  #[holder(use_place_holder)]
  SurfaceStyleFillArea(Box<SurfaceStyleFillArea>),
  #[holder(use_place_holder)]
  SurfaceStyleRendering(Box<SurfaceStyleRendering>),
  #[holder(use_place_holder)]
  SurfaceStyleTransparency(Box<SurfaceStyleTransparency>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_style_fill_area)]
#[holder(generate_deserialize)]
pub struct SurfaceStyleFillArea {
  pub name: String,
  #[holder(use_place_holder)]
  pub fill_area: FillAreaStyleColour,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = fill_area_style_colour)]
#[holder(generate_deserialize)]
pub struct FillAreaStyleColour {
  pub name: String,
  #[holder(use_place_holder)]
  pub fill_colour: ColourRGB,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = colour_rgb)]
#[holder(generate_deserialize)]
pub struct ColourRGB {
  pub name: String,
  pub red: f64,
  pub green: f64,
  pub blue: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_style_transparency)]
#[holder(generate_deserialize)]
pub struct SurfaceStyleTransparency {
  pub name: String,
  pub transparency: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_style_rendering)]
#[holder(generate_deserialize)]
pub struct SurfaceStyleRendering {
  pub name: String,
  #[holder(use_place_holder)]
  pub surface_colour: ColourRGB,
  pub reflectance: SurfaceStyleRenderingProperties,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceStyleRenderingProperties {
  #[serde(default)]
  pub ambient_coefficient: f64,
  #[serde(default)]
  pub diffuse_coefficient: f64,
  #[serde(default)]
  pub specular_coefficient: f64,
  #[serde(default)]
  pub specular_exponent: f64,
  pub specular_colour: Option<ColourRGB>,
}

// ── UnrecognizedEntity ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = unrecognized)]
#[holder(generate_deserialize)]
pub struct UnrecognizedEntity {
  pub entity_name: String,
  pub raw_data: String,
  pub is_simple: bool,
}

// ── push_instance ───────────────────────────────────────────────────

pub fn push_instance(table: &mut Table, instance: &EntityInstance) -> ruststep::error::Result<()> {
  match instance {
    EntityInstance::Simple { id, record } => match record.name.as_str() {
      // primitives
      "CARTESIAN_POINT" => {
        table
          .cartesian_point
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "DIRECTION" => {
        table
          .direction
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "VECTOR" => {
        table.vector.insert(*id, Deserialize::deserialize(record)?);
      }

      // placements
      "AXIS1_PLACEMENT" => {
        table
          .axis1_placement
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "AXIS2_PLACEMENT_2D" => {
        table
          .axis2_placement_2d
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "AXIS2_PLACEMENT_3D" => {
        table
          .axis2_placement_3d
          .insert(*id, Deserialize::deserialize(record)?);
      }

      // curves
      "LINE" => {
        table.line.insert(*id, Deserialize::deserialize(record)?);
      }
      "POLYLINE" => {
        table
          .polyline
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "CIRCLE" => {
        table.circle.insert(*id, Deserialize::deserialize(record)?);
      }
      "ELLIPSE" => {
        table.ellipse.insert(*id, Deserialize::deserialize(record)?);
      }
      "HYPERBOLA" => {
        table
          .hyperbola
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PARABOLA" => {
        table
          .parabola
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "B_SPLINE_CURVE_WITH_KNOTS" => {
        table
          .b_spline_curve_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "BEZIER_CURVE" => {
        table
          .bezier_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "QUASI_UNIFORM_CURVE" => {
        table
          .b_spline_curve_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "UNIFORM_CURVE" => {
        table
          .b_spline_curve_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "TRIMMED_CURVE" => {
        table
          .trimmed_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "COMPOSITE_CURVE" => {
        table
          .composite_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "OFFSET_CURVE_3D" => {
        table
          .offset_curve_3d
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PCURVE" => {
        table.pcurve.insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_CURVE" => {
        table
          .surface_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SEAM_CURVE" => {
        table
          .surface_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }

      // surfaces
      "PLANE" => {
        table.plane.insert(*id, Deserialize::deserialize(record)?);
      }
      "SPHERICAL_SURFACE" => {
        table
          .spherical_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "CYLINDRICAL_SURFACE" => {
        table
          .cylindrical_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "TOROIDAL_SURFACE" => {
        table
          .toroidal_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "CONICAL_SURFACE" => {
        table
          .conical_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "B_SPLINE_SURFACE_WITH_KNOTS" => {
        table
          .b_spline_surface_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "BEZIER_SURFACE" => {
        table
          .bezier_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "UNIFORM_SURFACE" => {
        table
          .b_spline_surface_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "QUASI_UNIFORM_SURFACE" => {
        table
          .b_spline_surface_with_knots
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_OF_LINEAR_EXTRUSION" => {
        table
          .surface_of_linear_extrusion
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_OF_REVOLUTION" => {
        table
          .surface_of_revolution
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "OFFSET_SURFACE" => {
        table
          .offset_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }

      // topology
      "VERTEX_POINT" => {
        table
          .vertex_point
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "EDGE_CURVE" => {
        table
          .edge_curve
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "ORIENTED_EDGE" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 5 {
            table.oriented_edge.insert(
              *id,
              OrientedEdgeHolder {
                label: Deserialize::deserialize(&params[0])?,
                edge_element: Deserialize::deserialize(&params[3])?,
                orientation: Deserialize::deserialize(&params[4])?,
              },
            );
          }
        }
      }
      "EDGE_LOOP" => {
        table
          .edge_loop
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "FACE_BOUND" => {
        table
          .face_bound
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "FACE_OUTER_BOUND" => {
        table
          .face_bound
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "FACE_SURFACE" => {
        table
          .face_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "ADVANCED_FACE" => {
        table
          .face_surface
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "ORIENTED_FACE" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 4 {
            table.oriented_face.insert(
              *id,
              OrientedFaceHolder {
                label: Deserialize::deserialize(&params[0])?,
                face_element: Deserialize::deserialize(&params[2])?,
                orientation: Deserialize::deserialize(&params[3])?,
              },
            );
          }
        }
      }
      "OPEN_SHELL" => {
        table.shell.insert(*id, Deserialize::deserialize(record)?);
      }
      "CLOSED_SHELL" => {
        table.shell.insert(*id, Deserialize::deserialize(record)?);
      }
      "ORIENTED_OPEN_SHELL" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 4 {
            table.oriented_shell.insert(
              *id,
              OrientedShellHolder {
                label: Deserialize::deserialize(&params[0])?,
                shell_element: Deserialize::deserialize(&params[2])?,
                orientation: Deserialize::deserialize(&params[3])?,
              },
            );
          }
        }
      }
      "ORIENTED_CLOSED_SHELL" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 4 {
            table.oriented_shell.insert(
              *id,
              OrientedShellHolder {
                label: Deserialize::deserialize(&params[0])?,
                shell_element: Deserialize::deserialize(&params[2])?,
                orientation: Deserialize::deserialize(&params[3])?,
              },
            );
          }
        }
      }
      "SHELL_BASED_SURFACE_MODEL" => {
        table
          .shell_based_surface_model
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "MANIFOLD_SOLID_BREP" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 2 {
            table.manifold_solid_brep.insert(
              *id,
              ManifoldSolidBrepHolder {
                label: Deserialize::deserialize(&params[0])?,
                outer: Deserialize::deserialize(&params[1])?,
                voids: Vec::new(),
              },
            );
          }
        }
      }
      "BREP_WITH_VOIDS" => {
        table
          .manifold_solid_brep
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "FACETED_BREP" => {
        table
          .faceted_brep
          .insert(*id, Deserialize::deserialize(record)?);
      }

      // assembly / navigation
      "REPRESENTATION" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() == 3 {
            table.representation.insert(
              *id,
              RepresentationHolder {
                name: Deserialize::deserialize(&params[0])?,
                items: Deserialize::deserialize(&params[1])?,
                context_of_items: Deserialize::deserialize(&params[2])?,
              },
            );
          }
        }
      }
      "REPRESENTATION_ITEM" => {
        table
          .representation_item
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "REPRESENTATION_CONTEXT" => {
        table
          .representation_context
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "MAPPED_ITEM" => {
        table
          .mapped_item
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PRODUCT" => {
        table.product.insert(*id, Deserialize::deserialize(record)?);
      }
      "PRODUCT_CONTEXT" => {
        table
          .product_definition_context
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PRODUCT_DEFINITION_FORMATION" => {
        table
          .product_definition_formation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE" => {
        if let Parameter::List(params) = &record.parameter {
          if params.len() >= 3 {
            table.product_definition_formation.insert(
              *id,
              ProductDefinitionFormationHolder {
                id: Deserialize::deserialize(&params[0])?,
                description: Deserialize::deserialize(&params[1])?,
                of_product: Deserialize::deserialize(&params[2])?,
              },
            );
          }
        }
      }
      "PRODUCT_DEFINITION" => {
        table
          .product_definition
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PRODUCT_DEFINITION_SHAPE" => {
        table
          .product_definition_shape
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SHAPE_DEFINITION_REPRESENTATION" => {
        table
          .shape_definition_representation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SHAPE_REPRESENTATION" => {
        table
          .shape_representation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "ADVANCED_BREP_SHAPE_REPRESENTATION" => {
        table
          .shape_representation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => {
        table
          .context_dependent_shape_representation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SHAPE_REPRESENTATION_RELATIONSHIP" => {
        table
          .shape_representation_relationship
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
        table
          .next_assembly_usage_occurrence
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "ITEM_DEFINED_TRANSFORMATION" => {
        table
          .item_defined_transformation
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "GEOMETRIC_SET" => {
        table
          .geometric_set
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "GEOMETRIC_CURVE_SET" => {
        table
          .geometric_curve_set
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "DEFINITIONAL_REPRESENTATION" => {
        table
          .definitional_representation
          .insert(*id, Deserialize::deserialize(record)?);
      }

      // presentation / visual
      "STYLED_ITEM" => {
        table
          .styled_item
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "PRESENTATION_STYLE_ASSIGNMENT" => {
        table
          .presentation_style_assignment
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_STYLE_USAGE" => {
        table
          .surface_style_usage
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_SIDE_STYLE" => {
        table
          .surface_side_style
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_STYLE_FILL_AREA" => {
        table
          .surface_style_fill_area
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "FILL_AREA_STYLE_COLOUR" => {
        table
          .fill_area_style_colour
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "COLOUR_RGB" => {
        table
          .colour_rgb
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_STYLE_TRANSPARENCY" => {
        table
          .surface_style_transparency
          .insert(*id, Deserialize::deserialize(record)?);
      }
      "SURFACE_STYLE_RENDERING" => {
        table
          .surface_style_rendering
          .insert(*id, Deserialize::deserialize(record)?);
      }

      _ => {
        table.unrecognized.insert(
          *id,
          UnrecognizedEntityHolder {
            entity_name: record.name.to_string(),
            raw_data: format!("{record:?}"),
            is_simple: true,
          },
        );
      }
    },

    EntityInstance::Complex {
      id,
      subsuper: SubSuperRecord(records),
    } => {
      use crate::entities::NonRationalBSplineCurveHolder as NRBC;
      use crate::entities::NonRationalBSplineSurfaceHolder as NRBS;

      // ── Rational B-Spline Curve Complex Entities ──
      if records.len() == 7 {
        match (
          records[0].name.as_str(),
          &records[0].parameter,
          records[1].name.as_str(),
          &records[1].parameter,
          records[2].name.as_str(),
          &records[2].parameter,
          records[3].name.as_str(),
          &records[3].parameter,
          records[4].name.as_str(),
          &records[4].parameter,
          records[5].name.as_str(),
          &records[5].parameter,
          records[6].name.as_str(),
          &records[6].parameter,
        ) {
          (
            "BOUNDED_CURVE",
            _,
            "B_SPLINE_CURVE",
            Parameter::List(bsp_params),
            "B_SPLINE_CURVE_WITH_KNOTS",
            Parameter::List(knots_params),
            "CURVE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "RATIONAL_B_SPLINE_CURVE",
            Parameter::List(weights),
            "REPRESENTATION_ITEM",
            Parameter::List(label),
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_curve.insert(
              *id,
              RationalBSplineCurveHolder {
                non_rational_b_spline_curve: PlaceHolder::Owned(NRBC::BSplineCurveWithKnots(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Deserialize::deserialize(&weights[0])?,
              },
            );
          }
          (
            "BEZIER_CURVE",
            _,
            "BOUNDED_CURVE",
            _,
            "B_SPLINE_CURVE",
            Parameter::List(bsp_params),
            "CURVE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "RATIONAL_B_SPLINE_CURVE",
            Parameter::List(weights),
            "REPRESENTATION_ITEM",
            Parameter::List(label),
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            table.rational_b_spline_curve.insert(
              *id,
              RationalBSplineCurveHolder {
                non_rational_b_spline_curve: PlaceHolder::Owned(NRBC::BezierCurve(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Deserialize::deserialize(&weights[0])?,
              },
            );
          }
          (
            "BOUNDED_CURVE",
            _,
            "B_SPLINE_CURVE",
            Parameter::List(bsp_params),
            "B_SPLINE_CURVE_WITH_KNOTS",
            Parameter::List(knots_params),
            "CURVE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "QUASI_UNIFORM_CURVE",
            _,
            "REPRESENTATION_ITEM",
            Parameter::List(label),
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_curve.insert(
              *id,
              RationalBSplineCurveHolder {
                non_rational_b_spline_curve: PlaceHolder::Owned(NRBC::QuasiUniformCurve(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Vec::new(),
              },
            );
          }
          (
            "BOUNDED_CURVE",
            _,
            "B_SPLINE_CURVE",
            Parameter::List(bsp_params),
            "B_SPLINE_CURVE_WITH_KNOTS",
            Parameter::List(knots_params),
            "CURVE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "UNIFORM_CURVE",
            _,
            "REPRESENTATION_ITEM",
            Parameter::List(label),
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_curve.insert(
              *id,
              RationalBSplineCurveHolder {
                non_rational_b_spline_curve: PlaceHolder::Owned(NRBC::UniformCurve(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Vec::new(),
              },
            );
          }
          _ => {
            table.unrecognized.insert(
              *id,
              UnrecognizedEntityHolder {
                entity_name: records
                  .iter()
                  .map(|r| r.name.as_str())
                  .collect::<Vec<_>>()
                  .join(" & "),
                raw_data: format!("{records:?}"),
                is_simple: false,
              },
            );
          }
        }
      } else if records.len() == 8 {
        match (
          records[0].name.as_str(),
          &records[0].parameter,
          records[1].name.as_str(),
          &records[1].parameter,
          records[2].name.as_str(),
          &records[2].parameter,
          records[3].name.as_str(),
          &records[3].parameter,
          records[4].name.as_str(),
          &records[4].parameter,
          records[5].name.as_str(),
          &records[5].parameter,
          records[6].name.as_str(),
          &records[6].parameter,
          records[7].name.as_str(),
          &records[7].parameter,
        ) {
          // ── Rational B-Spline Surface Complex Entities ──
          (
            "B_SPLINE_SURFACE",
            Parameter::List(bsp_params),
            "B_SPLINE_SURFACE_WITH_KNOTS",
            Parameter::List(knots_params),
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "RATIONAL_B_SPLINE_SURFACE",
            Parameter::List(weights),
            "REPRESENTATION_ITEM",
            Parameter::List(label),
            "SURFACE",
            _,
            "BOUNDED_SURFACE",
            _,
            "FACE_SURFACE",
            _,
          )
          | (
            "BOUNDED_SURFACE",
            _,
            "B_SPLINE_SURFACE",
            Parameter::List(bsp_params),
            "B_SPLINE_SURFACE_WITH_KNOTS",
            Parameter::List(knots_params),
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "RATIONAL_B_SPLINE_SURFACE",
            Parameter::List(weights),
            "REPRESENTATION_ITEM",
            Parameter::List(label),
            "SURFACE",
            _,
            "FACE_SURFACE",
            _,
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_surface.insert(
              *id,
              RationalBSplineSurfaceHolder {
                non_rational_b_spline_surface: PlaceHolder::Owned(NRBS::BSplineSurfaceWithKnots(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Deserialize::deserialize(&weights[0])?,
              },
            );
          }
          (
            "BEZIER_SURFACE",
            _,
            "B_SPLINE_SURFACE",
            Parameter::List(bsp_params),
            "BOUNDED_SURFACE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "RATIONAL_B_SPLINE_SURFACE",
            Parameter::List(weights),
            "REPRESENTATION_ITEM",
            Parameter::List(label),
            "SURFACE",
            _,
            "FACE_SURFACE",
            _,
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            table.rational_b_spline_surface.insert(
              *id,
              RationalBSplineSurfaceHolder {
                non_rational_b_spline_surface: PlaceHolder::Owned(NRBS::BezierSurface(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Deserialize::deserialize(&weights[0])?,
              },
            );
          }
          (
            "B_SPLINE_SURFACE",
            Parameter::List(bsp_params),
            "B_SPLINE_SURFACE_WITH_KNOTS",
            Parameter::List(knots_params),
            "BOUNDED_SURFACE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "QUASI_UNIFORM_SURFACE",
            _,
            "REPRESENTATION_ITEM",
            Parameter::List(label),
            "SURFACE",
            _,
            "FACE_SURFACE",
            _,
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_surface.insert(
              *id,
              RationalBSplineSurfaceHolder {
                non_rational_b_spline_surface: PlaceHolder::Owned(NRBS::QuasiUniformSurface(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Vec::new(),
              },
            );
          }
          (
            "B_SPLINE_SURFACE",
            Parameter::List(bsp_params),
            "B_SPLINE_SURFACE_WITH_KNOTS",
            Parameter::List(knots_params),
            "BOUNDED_SURFACE",
            _,
            "GEOMETRIC_REPRESENTATION_ITEM",
            _,
            "UNIFORM_SURFACE",
            _,
            "REPRESENTATION_ITEM",
            Parameter::List(label),
            "SURFACE",
            _,
            "FACE_SURFACE",
            _,
          ) => {
            let mut params = label.clone();
            params.extend(bsp_params.clone());
            params.extend(knots_params.clone());
            table.rational_b_spline_surface.insert(
              *id,
              RationalBSplineSurfaceHolder {
                non_rational_b_spline_surface: PlaceHolder::Owned(NRBS::UniformSurface(
                  Deserialize::deserialize(&Parameter::List(params))?,
                )),
                weights_data: Vec::new(),
              },
            );
          }

          // ── Assembly Complex Entities ──
          (
            "REPRESENTATION_ITEM",
            Parameter::List(rep_params),
            "SHAPE_REPRESENTATION",
            Parameter::List(shape_params),
            "SHAPE_REPRESENTATION_RELATIONSHIP",
            Parameter::List(rel_params),
            "SHAPE_REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
            Parameter::List(trans_params),
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
          ) => {
            let mut params = rep_params.clone();
            params.extend(shape_params.clone());
            params.extend(rel_params.clone());
            params.extend(trans_params.clone());
            table
              .shape_representation_relationship_with_transformation
              .insert(*id, Deserialize::deserialize(&Parameter::List(params))?);
          }
          _ => {
            table.unrecognized.insert(
              *id,
              UnrecognizedEntityHolder {
                entity_name: records
                  .iter()
                  .map(|r| r.name.as_str())
                  .collect::<Vec<_>>()
                  .join(" & "),
                raw_data: format!("{records:?}"),
                is_simple: false,
              },
            );
          }
        }
      } else {
        table.unrecognized.insert(
          *id,
          UnrecognizedEntityHolder {
            entity_name: records
              .iter()
              .map(|r| r.name.as_str())
              .collect::<Vec<_>>()
              .join(" & "),
            raw_data: format!("{records:?}"),
            is_simple: false,
          },
        );
      }
    }
  }
  Ok(())
}

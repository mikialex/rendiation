//! Clean B-rep model resolved from the step-reader `Table`: all references resolved into dense
//! ids, all geometry converted to f64 plain data, orientation flags kept exactly as in STEP.
//! No geometric processing happens here. Angles are converted to radians (see
//! `BrepModel::plane_angle_unit_rad`), lengths stay in file length units.

pub use parametric_normalized_model::{EdgeId, FaceId};
use rendiation_algebra::*;
use step_reader::table::Table;

mod assembly;
mod builder;
mod geometry;
mod lookup;
mod topology;
mod units;

#[cfg(test)]
mod tests;

macro_rules! define_id {
  ($name:ident) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct $name(pub u32);
    impl $name {
      pub fn index(self) -> usize {
        self.0 as usize
      }
    }
  };
}

define_id!(VertexId);
define_id!(SurfaceId);
define_id!(CurveId);
define_id!(Curve2Id);
define_id!(BodyId);

#[derive(Debug, Default)]
pub struct BrepModel {
  pub faces: Vec<Face>,
  pub edges: Vec<Edge>,
  pub vertices: Vec<Vertex>,
  pub surfaces: Vec<SurfaceGeom>,
  pub curves: Vec<CurveGeom>,
  pub curves_2d: Vec<Curve2Geom>,
  pub bodies: Vec<Body>,
  /// Every placed occurrence of a body (assembly instances). A body that is never referenced by
  /// an assembly gets one identity occurrence.
  pub occurrences: Vec<Occurrence>,
  /// `UNCERTAINTY_MEASURE_WITH_UNIT` length value, in file length units.
  pub uncertainty: Option<f64>,
  /// File length unit expressed in millimetres (1.0 for mm, 1000.0 for m), when known.
  pub length_unit_mm: Option<f64>,
  /// File plane angle unit in radians (`π / 180` for degrees), when known. Every angle stored
  /// in this model (cone semi angle, trim parameters on conics, angular pcurve coordinates) is
  /// already converted to radians with this factor.
  pub plane_angle_unit_rad: Option<f64>,
  /// Non fatal problems found while resolving (unresolved references, skipped entities...).
  pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Vertex {
  pub step_id: u64,
  pub position: Vec3<f64>,
}

#[derive(Debug, Clone)]
pub struct Face {
  pub step_id: u64,
  pub surface: SurfaceId,
  /// `FACE_SURFACE.same_sense`: face normal agrees with the surface normal `∂u × ∂v`.
  pub same_sense: bool,
  pub bounds: Vec<FaceBound>,
}

#[derive(Debug, Clone)]
pub struct FaceBound {
  pub step_id: u64,
  /// `FACE_OUTER_BOUND` (informative only, do not rely on it).
  pub is_outer: bool,
  /// `FACE_BOUND.orientation`: false means the loop is traversed reversed.
  pub orientation: bool,
  pub kind: BoundKind,
}

#[derive(Debug, Clone)]
pub enum BoundKind {
  /// `EDGE_LOOP`, edges in loop order.
  Edges(Vec<OrientedEdgeUse>),
  /// `VERTEX_LOOP` (degenerate bound, cone apex / sphere pole).
  Vertex(VertexId),
}

#[derive(Debug, Clone, Copy)]
pub struct OrientedEdgeUse {
  pub edge: EdgeId,
  /// `ORIENTED_EDGE.orientation`: true means traversed from edge start to edge end.
  pub orientation: bool,
}

#[derive(Debug, Clone)]
pub struct Edge {
  pub step_id: u64,
  pub start: VertexId,
  pub end: VertexId,
  /// The 3D curve (after unwrapping `SURFACE_CURVE` / `SEAM_CURVE`, which become `curve`
  /// plus `pcurves`).
  pub curve: CurveId,
  /// `EDGE_CURVE.same_sense`: the curve parameter increases from edge start to edge end.
  pub same_sense: bool,
  /// Pcurves attached through `SURFACE_CURVE.associated_geometry`.
  pub pcurves: Vec<PcurveRef>,
  /// True for `SEAM_CURVE` geometry.
  pub is_seam: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PcurveRef {
  pub surface: SurfaceId,
  pub curve: Curve2Id,
  pub step_id: u64,
}

/// Right handed orthonormal frame, built from `AXIS2_PLACEMENT_3D` following ISO 10303-42
/// `build_axes` (default axis `z`, default ref direction `x`, ref direction projected
/// perpendicular to the axis).
#[derive(Debug, Clone, Copy)]
pub struct Frame {
  pub origin: Vec3<f64>,
  pub x: Vec3<f64>,
  pub y: Vec3<f64>,
  pub z: Vec3<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Frame2 {
  pub origin: Vec2<f64>,
  pub x: Vec2<f64>,
  pub y: Vec2<f64>,
}

/// B-spline data shared by curves and surfaces. Knots are already expanded (multiplicities
/// applied). `weights` is `None` for non rational entities.
#[derive(Debug, Clone)]
pub struct BSplineCurveData<P> {
  pub degree: usize,
  pub knots: Vec<f64>,
  pub control_points: Vec<P>,
  pub weights: Option<Vec<f64>>,
  pub closed: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct BSplineSurfaceData {
  pub degree_u: usize,
  pub degree_v: usize,
  pub knots_u: Vec<f64>,
  pub knots_v: Vec<f64>,
  /// STEP layout: `control_points[u_index][v_index]`.
  pub control_points: Vec<Vec<Vec3<f64>>>,
  pub weights: Option<Vec<Vec<f64>>>,
  pub u_closed: Option<bool>,
  pub v_closed: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum SurfaceGeom {
  Plane(Frame),
  Cylinder {
    frame: Frame,
    radius: f64,
  },
  Cone {
    frame: Frame,
    radius: f64,
    semi_angle: f64,
  },
  Sphere {
    frame: Frame,
    radius: f64,
  },
  Torus {
    frame: Frame,
    major_radius: f64,
    minor_radius: f64,
  },
  BSpline(BSplineSurfaceData),
  /// `SURFACE_OF_LINEAR_EXTRUSION`: `S(u,v) = C(u) + v·vector` (vector includes magnitude).
  Extrusion {
    curve: CurveId,
    vector: Vec3<f64>,
  },
  /// `SURFACE_OF_REVOLUTION`: `u` rotation angle around `axis`, `v` generatrix parameter.
  Revolution {
    curve: CurveId,
    axis_origin: Vec3<f64>,
    axis_dir: Vec3<f64>,
  },
  Offset {
    basis: SurfaceId,
    distance: f64,
  },
  Unsupported {
    step_id: u64,
    type_name: String,
  },
}

#[derive(Debug, Clone, Copy)]
pub enum TrimSelect {
  Parameter(f64),
  Point(Vec3<f64>),
}

#[derive(Debug, Clone)]
pub enum CurveGeom {
  /// `LINE`: `P(t) = origin + t·vector` (vector includes magnitude).
  Line {
    origin: Vec3<f64>,
    vector: Vec3<f64>,
  },
  Circle {
    frame: Frame,
    radius: f64,
  },
  Ellipse {
    frame: Frame,
    semi_axis1: f64,
    semi_axis2: f64,
  },
  BSpline(BSplineCurveData<Vec3<f64>>),
  /// `POLYLINE`: parameter `i` at point `i`.
  Polyline(Vec<Vec3<f64>>),
  Trimmed {
    basis: CurveId,
    trim1: Vec<TrimSelect>,
    trim2: Vec<TrimSelect>,
    sense_agreement: bool,
  },
  /// Segments as `(parent curve, same_sense)`.
  Composite(Vec<(CurveId, bool)>),
  /// `OFFSET_CURVE_3D`: offset by `distance` along `ref_direction × tangent` (normalized).
  Offset {
    basis: CurveId,
    distance: f64,
    ref_direction: Vec3<f64>,
  },
  Unsupported {
    step_id: u64,
    type_name: String,
  },
}

#[derive(Debug, Clone)]
pub enum Curve2Geom {
  Line {
    origin: Vec2<f64>,
    vector: Vec2<f64>,
  },
  Circle {
    frame: Frame2,
    radius: f64,
  },
  Ellipse {
    frame: Frame2,
    semi_axis1: f64,
    semi_axis2: f64,
  },
  BSpline(BSplineCurveData<Vec2<f64>>),
  Polyline(Vec<Vec2<f64>>),
  Trimmed {
    basis: Curve2Id,
    trim1: Vec<TrimSelect2>,
    trim2: Vec<TrimSelect2>,
    sense_agreement: bool,
  },
  Composite(Vec<(Curve2Id, bool)>),
  Unsupported {
    step_id: u64,
    type_name: String,
  },
}

#[derive(Debug, Clone, Copy)]
pub enum TrimSelect2 {
  Parameter(f64),
  Point(Vec2<f64>),
}

#[derive(Debug, Clone)]
pub struct Body {
  pub step_id: u64,
  /// Shells as `(face, OrientedFace orientation)`; faces referenced directly have orientation
  /// true. Includes outer shell, voids and shell based surface models.
  pub shells: Vec<Vec<(FaceId, bool)>>,
}

#[derive(Debug, Clone)]
pub struct Occurrence {
  pub body: BodyId,
  /// Body coordinates to world coordinates.
  pub transform: Mat4<f64>,
  /// Human readable chain of STEP ids that produced the transform (debugging).
  pub path: String,
}

/// Resolve the whole table. Entities that can not be resolved are reported in `warnings` and
/// skipped; this function never fails as a whole.
///
/// Resolution order: units and tolerance (needed to convert angles), then every body
/// (`MANIFOLD_SOLID_BREP`, `BREP_WITH_VOIDS`, `SHELL_BASED_SURFACE_MODEL`, `FACETED_BREP`) in
/// STEP id order, pulling faces, edges, vertices and geometry on demand (deduplicated by STEP
/// id), then the assembly structure.
pub fn build_brep_model(table: &Table) -> BrepModel {
  builder::Builder::new(table).build()
}

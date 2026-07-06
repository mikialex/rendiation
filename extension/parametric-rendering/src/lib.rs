#![feature(array_windows)]

use curve3d::RationalBezierCurve3d;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use surface::RationalBezierSurface;

pub mod curve3d;
pub mod device;
pub mod mesh;
pub mod step;
pub mod surface;
pub mod validation;

use curve3d::*;
use device::*;
use mesh::*;
use rendiation_step_reader;
use surface::*;
pub use validation::*;

pub struct ParametricRenderingData {
  /// Unique geometry — control points are in local coordinates (no transform baked in).
  pub surfaces: Vec<TrimmedSurface>,
  /// Unique geometry — control points are in local coordinates (no transform baked in).
  pub curves_3d: Vec<RationalBezierCurve3d<f32>>,
  /// (index into `surfaces`, world-space transform matrix).
  pub surfaces_instance: Vec<(usize, Mat4<f32>)>,
  /// (index into `curves_3d`, world-space transform matrix).
  pub curves_3d_instance: Vec<(usize, Mat4<f32>)>,
}

/// Describes where a placement transform came from in the STEP document.
#[derive(Debug, Clone)]
pub enum PlacementSource {
  /// Simple Axis2Placement3d within a ShapeRepresentation.
  Axis2Placement3d {
    axis_id: u64,
    shape_representation_id: u64,
  },
  /// Composed from an assembly transformation chain.
  Assembly {
    /// BREP entity ID.
    brep_id: u64,
    /// Chain steps: each records (parent_sr_id, child_sr_id, idt_id, ax1_id, ax2_id).
    chain: Vec<AssemblyChainStep>,
  },
  /// No placement (identity matrix).
  Identity,
}

/// A single step in an assembly transform chain.
#[derive(Debug, Clone)]
pub struct AssemblyChainStep {
  pub parent_sr_id: u64,
  pub child_sr_id: u64,
  pub idt_id: u64,
  pub axis2_1_id: u64,
  pub axis2_2_id: u64,
}

pub struct TrimmedSurface {
  pub debug_label: String,
  pub surface: RationalBezierSurface<f32>,
  /// Trim boundary loops, one per FaceBound.  Each inner vec is a closed
  /// loop of quadratic Bézier curves in the patch-local [0,1]² domain.
  /// Empty vec means the surface is not trimmed.
  pub trim_loops: Vec<Vec<QuadraticBezierCurve2d<f32>>>,
  /// Whether to flip normals (and triangle winding) relative to du×dv.
  /// Propagated from FaceSurface.same_sense / OrientedFace.orientation.
  pub is_back_face: bool,
}

impl TrimmedSurface {
  /// Whether this surface has any trim loops.
  pub fn is_trimmed(&self) -> bool {
    self.trim_loops.iter().any(|l| !l.is_empty())
  }

  /// Flatten all trim loops into a single list (for consumers that don't
  /// care about loop boundaries).
  pub fn all_trim_curves(&self) -> impl Iterator<Item = &QuadraticBezierCurve2d<f32>> {
    self.trim_loops.iter().flat_map(|l| l.iter())
  }
}

/// A quadratic Bézier curve in 2D parametric space.
///
/// Defined by start point, control point, and end point:
/// `B(t) = (1-t)²·start + 2(1-t)t·ctrl + t²·end`
#[derive(Clone)]
pub struct QuadraticBezierCurve2d<T> {
  pub start: Vec2<T>,
  pub ctrl: Vec2<T>,
  pub end: Vec2<T>,
}

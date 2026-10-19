use std::ops::Range;

use rendiation_algebra::*;

use crate::ConvertError;
use crate::brep::{EdgeId, FaceId};
use crate::math::{RationalBezierCurve2, RationalBezierSurface};

pub struct StepConvertOutput {
  pub patches: Vec<TrimmedBezierPatch>,
  /// One entry per face occurrence (a face referenced by several bodies or assembly occurrences
  /// produces several instances sharing the same patches).
  pub instances: Vec<FaceInstance>,
  pub report: ConvertReport,
}

/// A rational Bézier patch restricted to the region bounded by its trim loops.
pub struct TrimmedBezierPatch {
  pub surface: RationalBezierSurface,
  /// Closed loops in the patch-local `[0,1]²` domain with the material on the left
  /// (outer boundaries counter-clockwise, holes clockwise). Never empty: an untrimmed patch
  /// carries one rectangle loop made of `PatchBoundary` curves.
  pub loops: Vec<TrimLoop>,
  /// The face normal is `-(∂S/∂u × ∂S/∂v)`. Combine with `FaceInstance::flip_normal` by xor.
  pub flip_normal: bool,
  pub face: FaceId,
  /// Cell index of this patch in the face's P space (debugging and adjacency).
  pub cell: (usize, usize),
}

impl TrimmedBezierPatch {
  pub fn is_untrimmed(&self) -> bool {
    self.loops.len() == 1
      && self.loops[0]
        .curves
        .iter()
        .all(|c| c.source == TrimCurveSource::PatchBoundary)
  }
}

pub struct TrimLoop {
  /// `curves[i].end() == curves[i + 1].start()` and `last.end() == first.start()` exactly.
  pub curves: Vec<TrimCurve>,
}

#[derive(Clone)]
pub struct TrimCurve {
  pub curve: RationalBezierCurve2,
  pub source: TrimCurveSource,
  /// True if the curve reproduces the source geometry exactly (up to float rounding), false if
  /// it was fitted within `ConvertCtx::fit_tol`.
  pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimCurveSource {
  /// Part of a B-rep edge. `reversed` is true when the curve runs from the edge end vertex
  /// towards the edge start vertex.
  Edge { edge: EdgeId, reversed: bool },
  /// Part of the patch boundary introduced by splitting the face into patches.
  PatchBoundary,
  /// Degenerate boundary segment at a surface pole (sphere pole, cone apex).
  Pole,
}

pub struct FaceInstance {
  pub face: FaceId,
  /// Range into `StepConvertOutput::patches`.
  pub patches: Range<usize>,
  pub transform: Mat4<f64>,
  /// From `OrientedFace.orientation` / shell orientation of this occurrence.
  pub flip_normal: bool,
}

#[derive(Default)]
pub struct ConvertReport {
  pub parse_errors: Vec<String>,
  pub brep_warnings: Vec<String>,
  pub faces: Vec<FaceReport>,
  /// Model tolerance used for this conversion (see `ConvertCtx::tol`).
  pub tol: f64,
}

pub struct FaceReport {
  pub face: FaceId,
  pub step_id: u64,
  pub surface_kind: &'static str,
  pub result: Result<FaceStats, ConvertError>,
  pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FaceStats {
  pub patch_count: usize,
  pub trim_curve_count: usize,
  pub fitted_curve_count: usize,
}

use std::ops::Range;

use rendiation_algebra::*;
use serde::{Deserialize, Serialize};

use crate::*;

/// A whole model lowered into trimmed rational Bézier patches.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParametricNormalizedModel {
  pub patches: Vec<TrimmedBezierPatch>,
  /// One entry per face occurrence (a face referenced by several bodies or assembly occurrences
  /// produces several instances sharing the same patches).
  pub instances: Vec<FaceInstance>,
  /// Indexed by `FaceId`. Contains every face of the source model, also the ones that produced
  /// no patch.
  pub faces: Vec<FaceInfo>,
  /// Indexed by `EdgeId`.
  pub edges: Vec<EdgeInfo>,
  /// Model tolerance in model length units: points closer than this are the same point.
  pub tolerance: f64,
  /// Model length unit expressed in millimetres (1.0 for mm, 1000.0 for m), when known.
  pub length_unit_mm: Option<f64>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaceId(pub u32);

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u32);

impl FaceId {
  pub fn index(self) -> usize {
    self.0 as usize
  }
}

impl EdgeId {
  pub fn index(self) -> usize {
    self.0 as usize
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceInfo {
  /// Entity id of the face in the source file (STEP instance id), for debugging.
  pub source_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeInfo {
  /// Entity id of the edge in the source file (STEP instance id), for debugging.
  pub source_id: u64,
}

/// A rational Bézier patch restricted to the region bounded by its trim loops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimmedBezierPatch {
  pub surface: RationalBezierSurface,
  /// Closed loops in the patch-local `[0,1]²` domain with the material on the left
  /// (outer boundaries counter-clockwise, holes clockwise). Never empty: an untrimmed patch
  /// carries one rectangle loop made of `PatchBoundary` curves.
  pub loops: Vec<TrimLoop>,
  /// The face normal is `-(∂S/∂u × ∂S/∂v)`. Combine with `FaceInstance::flip_normal` by xor.
  pub flip_normal: bool,
  pub face: FaceId,
  /// Cell index of this patch in the face's patch grid (debugging and adjacency).
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimLoop {
  /// `curves[i].end() == curves[i + 1].start()` and `last.end() == first.start()` exactly.
  pub curves: Vec<TrimCurve>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimCurve {
  pub curve: RationalBezierCurve2,
  pub source: TrimCurveSource,
  /// True if the curve reproduces the source geometry exactly (up to float rounding), false if
  /// it was fitted within the conversion fit tolerance.
  pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimCurveSource {
  /// Part of a B-rep edge. `reversed` is true when the curve runs from the edge end vertex
  /// towards the edge start vertex.
  Edge { edge: EdgeId, reversed: bool },
  /// Part of the patch boundary introduced by splitting the face into patches.
  PatchBoundary,
  /// Degenerate boundary segment at a surface pole (sphere pole, cone apex).
  Pole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceInstance {
  pub face: FaceId,
  /// Range into `ParametricNormalizedModel::patches`.
  pub patches: Range<usize>,
  pub transform: Mat4<f64>,
  /// From `OrientedFace.orientation` / shell orientation of this occurrence.
  pub flip_normal: bool,
}

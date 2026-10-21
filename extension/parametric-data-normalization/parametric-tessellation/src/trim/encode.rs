use std::time::{Duration, Instant};

use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::trim::*;

/// One quadratic segment as uploaded, two vec4: `(p0.x, p0.y, p1.x, p1.y)` and
/// `(p2.x, p2.y, max_u, bitcast<f32>(flags))`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EncodedSegment {
  pub p0: Vec2<f32>,
  pub p1: Vec2<f32>,
  pub p2: Vec2<f32>,
  /// Largest u of the three control points.
  pub max_u: f32,
  /// `SEGMENT_FLAG_*` bits.
  pub flags: u32,
}

/// The segment only exists to make the winding of an acceleration structure cell correct (not
/// trim geometry), it takes no part in distance computations.
pub const SEGMENT_FLAG_AUXILIARY: u32 = 1;
/// The control points are collinear (in f64, before rounding): the distance takes the chord.
pub const SEGMENT_FLAG_LINE: u32 = 2;

impl EncodedSegment {
  /// `SEGMENT_FLAG_LINE` is added for collinear control points.
  pub fn new(q: &TrimQuadratic, flags: u32) -> Self {
    let [a, b, c] = q.points;
    let collinear = (b.x - a.x) * (c.y - a.y) == (c.x - a.x) * (b.y - a.y);
    let flags = flags | if collinear { SEGMENT_FLAG_LINE } else { 0 };
    let p = q.points.map(|p| Vec2::new(p.x as f32, p.y as f32));
    Self {
      p0: p[0],
      p1: p[1],
      p2: p[2],
      max_u: p[0].x.max(p[1].x).max(p[2].x),
      flags,
    }
  }

  pub fn min(&self) -> Vec2<f32> {
    Vec2::new(
      self.p0.x.min(self.p1.x).min(self.p2.x),
      self.p0.y.min(self.p1.y).min(self.p2.y),
    )
  }

  pub fn max(&self) -> Vec2<f32> {
    Vec2::new(self.max_u, self.p0.y.max(self.p1.y).max(self.p2.y))
  }

  /// The control point box intersects the closed rectangle.
  pub fn box_intersects(&self, rect: &TrimRect) -> bool {
    let (min, max) = (self.min(), self.max());
    min.x <= rect.max.x && max.x >= rect.min.x && min.y <= rect.max.y && max.y >= rect.min.y
  }

  /// The segment lies on a side of the domain. Clamped samples never reach it, so it can not
  /// make the winding vary inside a rectangle (it still contributes to the winding).
  pub fn on_domain_boundary(&self) -> bool {
    let p = [self.p0, self.p1, self.p2];
    [0., 1.]
      .iter()
      .any(|&side| p.iter().all(|p| p.x == side) || p.iter().all(|p| p.y == side))
  }
}

/// `TRIM_HEADER_WORDS` u32 per patch at the start of the table, indexed by patch index.
pub const TRIM_HEADER_WORDS: usize = 4;

/// The whole patch domain is material (untrimmed patch, or a trim covering the whole domain).
pub const TRIM_KIND_INSIDE: u32 = 0;
/// No material at all (degenerate trim).
pub const TRIM_KIND_OUTSIDE: u32 = 1;
pub const TRIM_KIND_ALL_SEGMENTS: u32 = 2;
pub const TRIM_KIND_BAND_GRID: u32 = 3;

/// The table words `[patch * TRIM_HEADER_WORDS..]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchTrimHeader {
  /// First segment of the patch in the segment pool, the structure data refer to segments by
  /// local index (relative to this).
  pub segment_offset: u32,
  pub segment_count: u32,
  /// Start of the acceleration structure data in the table.
  pub accel_offset: u32,
  /// `TRIM_KIND_*`.
  pub kind: u32,
}

/// The uv margin by which the leaf classification expands a sub patch rectangle when the
/// fragments dilate the trim by `dilation_px` pixels: `dilation · 2^-level / size_px` (`size_px`
/// the projected size of the sub patch), rounded up to a power of two in `[2^-22, 1]` by the
/// float bits so that the f32 GPU version and the CPU reference agree, 0 without dilation.
/// Approximate: the projected size of the bounds overestimates the size in some directions.
pub fn dilation_margin(dilation_px: f32, level: u32, size_px: f32) -> f32 {
  if dilation_px <= 0. {
    return 0.;
  }
  let m = dilation_px * f32::from_bits((127 - level.min(126)) << 23) / size_px;
  // also for a zero size (infinite m)
  if size_px <= 0. || m >= 1. || m.is_nan() {
    return 1.;
  }
  let bits = m.to_bits();
  let exponent = ((bits >> 23) & 0xff) as i32 - 127 + ((bits & 0x7f_ffff) != 0) as i32;
  f32::from_bits(((exponent.max(-22) + 127) as u32) << 23)
}

/// A closed rectangle in the patch domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimRect {
  pub min: Vec2<f32>,
  pub max: Vec2<f32>,
}

impl TrimRect {
  pub const DOMAIN: Self = Self {
    min: Vec2 { x: 0., y: 0. },
    max: Vec2 { x: 1., y: 1. },
  };

  pub fn center(&self) -> Vec2<f32> {
    (self.min + self.max) * 0.5
  }

  pub fn expanded(&self, margin: f32) -> Self {
    Self {
      min: self.min - Vec2::splat(margin),
      max: self.max + Vec2::splat(margin),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectClass {
  Outside,
  Inside,
  /// May contain both, always a valid answer.
  Partial,
}

/// The upload data of the trims of a whole model: the segment pool and one u32 table holding
/// the patch headers followed by the acceleration structure data of each patch.
#[derive(Debug, Clone, Default)]
pub struct TrimEncoding {
  pub segments: Vec<EncodedSegment>,
  pub table: Vec<u32>,
  pub stats: TrimEncodingStats,
}

#[derive(Debug, Clone, Default)]
pub struct TrimEncodingStats {
  pub patches: usize,
  /// Patches with a trim that is not the domain rectangle.
  pub trimmed: usize,
  /// Trimmed patches whose trim turned out to cover the whole domain (or nothing).
  pub downgraded_inside: usize,
  pub downgraded_outside: usize,
  /// Segment count of each encoded trimmed patch.
  pub segment_counts: Vec<usize>,
  pub uv_tolerances: Vec<f64>,
  pub approximation_failures: usize,
  pub build_time: Duration,
}

/// Convert and encode the trims of every patch of `model` with the structure `accel`.
pub fn encode_trims(
  model: &ParametricNormalizedModel,
  config: &TrimQuadraticConfig,
  accel: &dyn TrimAccel,
) -> TrimEncoding {
  let start = Instant::now();
  let mut encoding = TrimEncoding {
    table: vec![0; model.patches.len() * TRIM_HEADER_WORDS],
    ..Default::default()
  };
  encoding.stats.patches = model.patches.len();
  for (index, patch) in model.patches.iter().enumerate() {
    if patch.is_untrimmed() {
      encoding.set_header(index, header(0, 0, 0, TRIM_KIND_INSIDE));
      continue;
    }
    encoding.stats.trimmed += 1;
    let quadratics = trim_quadratics(patch, config);
    encoding.stats.approximation_failures += quadratics.failures;
    encoding.stats.uv_tolerances.push(quadratics.uv_tolerance);

    let mut segments: Vec<_> = quadratics
      .segments
      .iter()
      .map(|q| EncodedSegment::new(q, 0))
      .collect();
    let mut words = Vec::new();
    accel.build(&mut segments, &mut words);

    let h = header(
      encoding.segments.len() as u32,
      segments.len() as u32,
      encoding.table.len() as u32,
      accel.kind(),
    );
    let view = PatchTrimView {
      header: h,
      segments: &segments,
      data: &words,
    };
    match accel.rect_query(view, &TrimRect::DOMAIN) {
      RectClass::Inside => {
        encoding.stats.downgraded_inside += 1;
        encoding.set_header(index, header(0, 0, 0, TRIM_KIND_INSIDE));
      }
      RectClass::Outside => {
        encoding.stats.downgraded_outside += 1;
        encoding.set_header(index, header(0, 0, 0, TRIM_KIND_OUTSIDE));
      }
      RectClass::Partial => {
        encoding.stats.segment_counts.push(segments.len());
        encoding.set_header(index, h);
        encoding.segments.extend(segments);
        encoding.table.extend(words);
      }
    }
  }
  encoding.stats.build_time = start.elapsed();
  encoding
}

fn header(
  segment_offset: u32,
  segment_count: u32,
  accel_offset: u32,
  kind: u32,
) -> PatchTrimHeader {
  PatchTrimHeader {
    segment_offset,
    segment_count,
    accel_offset,
    kind,
  }
}

impl TrimEncoding {
  pub fn header(&self, patch: usize) -> PatchTrimHeader {
    let w = &self.table[patch * TRIM_HEADER_WORDS..];
    header(w[0], w[1], w[2], w[3])
  }

  fn set_header(&mut self, patch: usize, h: PatchTrimHeader) {
    let w = &mut self.table[patch * TRIM_HEADER_WORDS..];
    w[..TRIM_HEADER_WORDS].copy_from_slice(&[
      h.segment_offset,
      h.segment_count,
      h.accel_offset,
      h.kind,
    ]);
  }

  /// The data of one patch as the structure queries see it. `None` for the constant kinds.
  pub fn patch_view(&self, patch: usize) -> Option<PatchTrimView<'_>> {
    let header = self.header(patch);
    accel_for_kind(header.kind)?;
    let start = header.segment_offset as usize;
    Some(PatchTrimView {
      header,
      segments: &self.segments[start..start + header.segment_count as usize],
      data: &self.table[header.accel_offset as usize..],
    })
  }

  /// Winding number of the (clamped) sample, the f32 path of the shader: structure point query
  /// plus the kernel over the candidates. Only `!= 0` is meaningful for constant cells.
  pub fn winding(&self, patch: usize, uv: Vec2<f32>) -> i32 {
    let uv = clamp_uv(uv);
    let header = self.header(patch);
    match header.kind {
      TRIM_KIND_INSIDE => 1,
      TRIM_KIND_OUTSIDE => 0,
      kind => {
        let accel = accel_for_kind(kind).expect("unknown trim kind");
        let view = self.patch_view(patch).unwrap();
        let mut winding = 0;
        let w0 = accel.point_query(view, uv, &mut |s| winding += crossing(s, uv));
        w0 + winding
      }
    }
  }

  pub fn is_inside(&self, patch: usize, uv: Vec2<f32>) -> bool {
    self.winding(patch, uv) != 0
  }

  /// The vector from the (clamped) sample to the approximate nearest point of the trim curves
  /// (`nearest_point`) over every non auxiliary segment of the patch, `None` without segments.
  pub fn nearest_vector(&self, patch: usize, uv: Vec2<f32>) -> Option<Vec2<f32>> {
    let uv = clamp_uv(uv);
    let view = self.patch_view(patch)?;
    view
      .segments
      .iter()
      .filter(|s| s.flags & SEGMENT_FLAG_AUXILIARY == 0)
      .map(|s| nearest_point(s, uv))
      .min_by(|a, b| a.length2().total_cmp(&b.length2()))
  }

  /// Classification of a closed rectangle of the domain (the leaf classification).
  pub fn classify_rect(&self, patch: usize, rect: &TrimRect) -> RectClass {
    let header = self.header(patch);
    match header.kind {
      TRIM_KIND_INSIDE => RectClass::Inside,
      TRIM_KIND_OUTSIDE => RectClass::Outside,
      kind => {
        let accel = accel_for_kind(kind).expect("unknown trim kind");
        accel.rect_query(self.patch_view(patch).unwrap(), rect)
      }
    }
  }
}

/// The query implementation of a stored kind (queries only read the self describing data, so
/// the default configured instances serve every encoding).
pub fn accel_for_kind(kind: u32) -> Option<&'static dyn TrimAccel> {
  static ALL_SEGMENTS: AllSegments = AllSegments;
  static BAND_GRID: BandGrid = BandGrid::DEFAULT;
  match kind {
    TRIM_KIND_ALL_SEGMENTS => Some(&ALL_SEGMENTS),
    TRIM_KIND_BAND_GRID => Some(&BAND_GRID),
    _ => None,
  }
}

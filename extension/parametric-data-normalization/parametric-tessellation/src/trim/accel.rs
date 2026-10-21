use rendiation_algebra::*;

use crate::trim::*;

/// A trim acceleration structure: the CPU build into the shared encoding, and the CPU mirror of
/// its shader queries (operation by operation, on the encoded data, so the tests also check the
/// encoding the GPU decodes).
///
/// Point query contract: for a clamped sample `p`, `winding(p) = w0 + Σ crossing(s, p)` over the
/// candidate segments passed to `visit`, `w0` being the returned value. Rectangle query: the
/// class of a closed rectangle, conservative (`Partial` is always valid). Distance contract (not
/// used yet): the candidates contain every non auxiliary segment within `distance_radius` of `p`.
pub trait TrimAccel: Sync {
  /// `TRIM_KIND_*` stored in the patch header.
  fn kind(&self) -> u32;

  /// Append the structure data of one patch to `data` (offsets inside are relative to the start
  /// of `data`). Auxiliary segments may be appended to `segments`.
  fn build(&self, segments: &mut Vec<EncodedSegment>, data: &mut Vec<u32>);

  fn point_query(
    &self,
    patch: PatchTrimView,
    uv: Vec2<f32>,
    visit: &mut dyn FnMut(&EncodedSegment),
  ) -> i32;

  fn rect_query(&self, patch: PatchTrimView, rect: &TrimRect) -> RectClass;

  fn distance_radius(&self) -> f32;
}

/// What a query sees of one patch.
#[derive(Clone, Copy)]
pub struct PatchTrimView<'a> {
  pub header: PatchTrimHeader,
  /// The patch's segments, structures refer to them by local index.
  pub segments: &'a [EncodedSegment],
  /// The table from `header.accel_offset` on.
  pub data: &'a [u32],
}

impl PatchTrimView<'_> {
  /// Kernel over all segments.
  pub fn winding_all(&self, uv: Vec2<f32>) -> i32 {
    self.segments.iter().map(|s| crossing(s, uv)).sum()
  }

  /// `Partial` if the box of a segment not on the domain boundary touches the rectangle,
  /// otherwise the class of its center.
  fn classify_by_boxes<'s>(
    &self,
    rect: &TrimRect,
    candidates: impl IntoIterator<Item = &'s EncodedSegment>,
  ) -> RectClass {
    if candidates
      .into_iter()
      .any(|s| !s.on_domain_boundary() && s.box_intersects(rect))
    {
      return RectClass::Partial;
    }
    if self.winding_all(clamp_uv(rect.center())) != 0 {
      RectClass::Inside
    } else {
      RectClass::Outside
    }
  }
}

/// The reference structure: no data, every segment is a candidate.
pub struct AllSegments;

impl TrimAccel for AllSegments {
  fn kind(&self) -> u32 {
    TRIM_KIND_ALL_SEGMENTS
  }

  fn build(&self, _: &mut Vec<EncodedSegment>, _: &mut Vec<u32>) {}

  fn point_query(
    &self,
    patch: PatchTrimView,
    _: Vec2<f32>,
    visit: &mut dyn FnMut(&EncodedSegment),
  ) -> i32 {
    patch.segments.iter().for_each(visit);
    0
  }

  fn rect_query(&self, patch: PatchTrimView, rect: &TrimRect) -> RectClass {
    patch.classify_by_boxes(rect, patch.segments)
  }

  fn distance_radius(&self) -> f32 {
    f32::INFINITY
  }
}

/// Uniform horizontal bands plus a 2 bit pre classification grid.
///
/// Data words (relative to `accel_offset`):
/// - `[0]`: `band_log2 | grid_log2 << 8`, `nb = 1 << band_log2` bands, `n = 1 << grid_log2`.
/// - `grid_words(n)` words of the `n×n` grid, cell `cy * n + cx` in bits
///   `2 * (cell % 16)..` of word `cell / 16`: `GRID_UNDETERMINED`, `GRID_OUTSIDE`, `GRID_INSIDE`.
///   A cell is constant if no control box of a segment off the domain boundary touches the
///   closed cell, its class is the kernel result at the cell center. Cell of a sample: `floor(uv * n)` (exact in f32).
/// - `nb` band words `start | count << 16`, `start` relative to the list base (the word after the
///   band words). Band `b` of a sample: `floor(v * nb)`.
/// - The band lists: local segment indices, one u32 each, sorted by `max_u` descending (ties by
///   index). A segment is in every band whose closed v range `[b, b + 1] / nb` touches its
///   control box (horizontal segments included, the rectangle query needs them). The point query
///   stops at the first segment with `max_u < u`.
#[derive(Debug, Clone, Copy)]
pub struct BandGrid {
  /// Bands are the smallest power of two up to `1 << max_band_log2` whose fullest band has at
  /// most `target_per_band` segments.
  pub max_band_log2: u32,
  pub target_per_band: usize,
  pub grid_log2: u32,
}

pub const GRID_UNDETERMINED: u32 = 0;
pub const GRID_OUTSIDE: u32 = 1;
pub const GRID_INSIDE: u32 = 2;

pub fn grid_words(n: usize) -> usize {
  (n * n).div_ceil(16)
}

impl BandGrid {
  pub const DEFAULT: Self = Self {
    max_band_log2: 4,
    target_per_band: 4,
    grid_log2: 4,
  };
}

impl Default for BandGrid {
  fn default() -> Self {
    Self::DEFAULT
  }
}

/// Decoded layout of the band grid data of one patch.
#[derive(Debug, Clone, Copy)]
pub struct BandGridLayout {
  pub band_log2: u32,
  pub grid_log2: u32,
  pub band_base: usize,
  pub list_base: usize,
}

impl BandGridLayout {
  pub fn decode(data: &[u32]) -> Self {
    let band_log2 = data[0] & 0xff;
    let grid_log2 = (data[0] >> 8) & 0xff;
    let band_base = 1 + grid_words(1 << grid_log2);
    Self {
      band_log2,
      grid_log2,
      band_base,
      list_base: band_base + (1 << band_log2),
    }
  }

  pub fn grid_cell(&self, data: &[u32], cx: u32, cy: u32) -> u32 {
    let cell = (cy << self.grid_log2) + cx;
    (data[1 + (cell / 16) as usize] >> (2 * (cell % 16))) & 3
  }

  /// Local segment indices of band `b`.
  pub fn band<'a>(&self, data: &'a [u32], b: u32) -> &'a [u32] {
    let word = data[self.band_base + b as usize];
    let start = self.list_base + (word & 0xffff) as usize;
    &data[start..start + (word >> 16) as usize]
  }
}

/// Closed v range of band `b` of `nb`, exact in f32.
fn band_range(b: usize, nb: usize) -> (f32, f32) {
  (b as f32 / nb as f32, (b + 1) as f32 / nb as f32)
}

fn in_band(segment: &EncodedSegment, (lo, hi): (f32, f32)) -> bool {
  segment.min().y <= hi && segment.max().y >= lo
}

impl BandGrid {
  /// The band scan alone (no grid), the part that must agree bitwise with `AllSegments`.
  pub fn band_scan(
    patch: PatchTrimView,
    uv: Vec2<f32>,
    visit: &mut dyn FnMut(&EncodedSegment),
  ) -> i32 {
    let layout = BandGridLayout::decode(patch.data);
    let b = (uv.y * (1 << layout.band_log2) as f32) as u32;
    for &index in layout.band(patch.data, b) {
      let segment = &patch.segments[index as usize];
      if segment.max_u < uv.x {
        break;
      }
      visit(segment);
    }
    0
  }
}

impl TrimAccel for BandGrid {
  fn kind(&self) -> u32 {
    TRIM_KIND_BAND_GRID
  }

  fn build(&self, segments: &mut Vec<EncodedSegment>, data: &mut Vec<u32>) {
    assert!(self.max_band_log2 <= 8 && self.grid_log2 <= 8);
    let band_members = |nb: usize| -> Vec<Vec<u32>> {
      (0..nb)
        .map(|b| {
          let range = band_range(b, nb);
          let mut members: Vec<_> = (0..segments.len() as u32)
            .filter(|&i| in_band(&segments[i as usize], range))
            .collect();
          members.sort_by(|&a, &b| {
            let (sa, sb) = (&segments[a as usize], &segments[b as usize]);
            sb.max_u.total_cmp(&sa.max_u).then(a.cmp(&b))
          });
          members
        })
        .collect()
    };
    let mut band_log2 = 0;
    let mut bands = band_members(1);
    while band_log2 < self.max_band_log2
      && bands.iter().map(Vec::len).max().unwrap_or(0) > self.target_per_band
    {
      band_log2 += 1;
      bands = band_members(1 << band_log2);
    }

    let base = data.len();
    data.push(band_log2 | self.grid_log2 << 8);

    let n = 1usize << self.grid_log2;
    let grid_start = data.len();
    data.resize(grid_start + grid_words(n), 0);
    let view = PatchTrimView {
      header: PatchTrimHeader {
        segment_offset: 0,
        segment_count: segments.len() as u32,
        accel_offset: 0,
        kind: TRIM_KIND_ALL_SEGMENTS,
      },
      segments,
      data: &[],
    };
    for cy in 0..n {
      for cx in 0..n {
        let cell = TrimRect {
          min: Vec2::new(cx as f32 / n as f32, cy as f32 / n as f32),
          max: Vec2::new((cx + 1) as f32 / n as f32, (cy + 1) as f32 / n as f32),
        };
        let class = match AllSegments.rect_query(view, &cell) {
          RectClass::Partial => GRID_UNDETERMINED,
          RectClass::Outside => GRID_OUTSIDE,
          RectClass::Inside => GRID_INSIDE,
        };
        let index = cy * n + cx;
        data[grid_start + index / 16] |= class << (2 * (index % 16));
      }
    }

    let mut start = 0;
    for band in &bands {
      assert!(start < 1 << 16 && band.len() < 1 << 16);
      data.push(start as u32 | (band.len() as u32) << 16);
      start += band.len();
    }
    for band in &bands {
      data.extend(band);
    }
    debug_assert_eq!(
      BandGridLayout::decode(&data[base..]).list_base,
      data.len() - base - start
    );
  }

  fn point_query(
    &self,
    patch: PatchTrimView,
    uv: Vec2<f32>,
    visit: &mut dyn FnMut(&EncodedSegment),
  ) -> i32 {
    let layout = BandGridLayout::decode(patch.data);
    let n = (1 << layout.grid_log2) as f32;
    match layout.grid_cell(patch.data, (uv.x * n) as u32, (uv.y * n) as u32) {
      GRID_OUTSIDE => 0,
      GRID_INSIDE => 1,
      _ => Self::band_scan(patch, uv, visit),
    }
  }

  fn rect_query(&self, patch: PatchTrimView, rect: &TrimRect) -> RectClass {
    let layout = BandGridLayout::decode(patch.data);
    let cells = |min: f32, max: f32, log2: u32| {
      let n = (1u32 << log2) as f32;
      let last = (1u32 << log2) - 1;
      let first = ((min.max(0.) * n) as u32).min(last);
      let end = ((max.min(1.) * n).ceil() as u32).clamp(first + 1, last + 1);
      first..end
    };
    let (xs, ys) = (
      cells(rect.min.x, rect.max.x, layout.grid_log2),
      cells(rect.min.y, rect.max.y, layout.grid_log2),
    );
    let first = layout.grid_cell(patch.data, xs.start, ys.start);
    let constant = first != GRID_UNDETERMINED
      && ys.clone().all(|cy| {
        xs.clone()
          .all(|cx| layout.grid_cell(patch.data, cx, cy) == first)
      });
    if constant {
      return if first == GRID_INSIDE {
        RectClass::Inside
      } else {
        RectClass::Outside
      };
    }
    let candidates = cells(rect.min.y, rect.max.y, layout.band_log2)
      .flat_map(|b| layout.band(patch.data, b))
      .map(|&i| &patch.segments[i as usize]);
    patch.classify_by_boxes(rect, candidates)
  }

  fn distance_radius(&self) -> f32 {
    0.
  }
}

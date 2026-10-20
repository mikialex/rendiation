//! Split a chain at the cell boundary lines, classify the pieces and extract the kept runs.

use super::cell::*;
use super::chain::*;
use super::geom::*;
use crate::*;

/// Kept runs of one cell: closed cycles fully inside, and open fragments running from an
/// entry point to an exit point on the cell boundary.
#[derive(Default)]
pub(super) struct CellRuns {
  pub cycles: Vec<Vec<TrimCurve>>,
  pub fragments: Vec<Vec<TrimCurve>>,
}

pub(super) fn clip_chain(cell: &Cell, chain: &Chain, runs: &mut CellRuns) {
  let (pieces, kept) = split_and_classify(cell, chain);
  if pieces.is_empty() {
    return;
  }
  let n = pieces.len();
  if chain.cyclic && kept.iter().all(|k| *k) {
    runs.cycles.push(pieces);
    return;
  }
  // Start right after a dropped piece so no run wraps around the cyclic start.
  let start = if chain.cyclic {
    match (0..n).find(|k| !kept[*k]) {
      Some(k) => k + 1,
      None => return,
    }
  } else {
    0
  };
  let mut pieces: Vec<_> = pieces.into_iter().map(Some).collect();
  let mut current = Vec::new();
  for step in 0..n {
    let k = (start + step) % n;
    if kept[k] {
      current.extend(pieces[k].take());
    } else if !current.is_empty() {
      runs.fragments.push(std::mem::take(&mut current));
    }
  }
  if !current.is_empty() {
    runs.fragments.push(current);
  }
}

struct RawPiece {
  curve: TrimCurve,
  /// The curve's bounding box does not come near the cell: outside, no split was attempted.
  far: bool,
}

/// Pieces of the chain (consecutive pieces share endpoints exactly, also around a cyclic
/// chain) and their kept flags.
fn split_and_classify(cell: &Cell, chain: &Chain) -> (Vec<TrimCurve>, Vec<bool>) {
  let near = bbox_expand(cell.bbox(), 4. * cell.eps);
  let n = chain.curves.len();

  // Vertices snapped onto the cell lines once, shared by both adjacent curves.
  let mut vertices: Vec<_> = chain
    .curves
    .iter()
    .map(|c| cell.snap(c.curve.start()))
    .collect();
  vertices.push(if chain.cyclic {
    vertices[0]
  } else {
    cell.snap(chain.curves[n - 1].curve.end())
  });

  let mut raw = Vec::new();
  let mut points = vec![vertices[0]];
  for (k, c) in chain.curves.iter().enumerate() {
    let bbox = chain.bboxes[k];
    if !bbox_overlaps(bbox, near) {
      raw.push(RawPiece {
        curve: c.clone(),
        far: true,
      });
      points.push(vertices[k + 1]);
      continue;
    }
    let mut t0 = 0.;
    let splits = cell.split_points(&c.curve, bbox);
    for (t1, q) in splits.into_iter().chain([(1., vertices[k + 1])]) {
      raw.push(RawPiece {
        curve: TrimCurve {
          curve: c.curve.sub_curve(t0, t1).normalize_end_weights(),
          source: c.source,
          exact: c.exact,
        },
        far: false,
      });
      points.push(q);
      t0 = t1;
    }
  }
  for (k, piece) in raw.iter_mut().enumerate() {
    let curve = &mut piece.curve.curve;
    *curve = curve.with_endpoints(points[k], points[k + 1]);
  }

  // Collapse pieces shorter than eps into a single vertex, keeping the better snapped one.
  let mut kept_raw = Vec::with_capacity(raw.len());
  let mut merged_points = vec![points[0]];
  for (k, piece) in raw.into_iter().enumerate() {
    if control_polygon_length(&piece.curve.curve) <= cell.eps {
      let last = merged_points.len() - 1;
      if cell.rank(points[k + 1]) > cell.rank(merged_points[last]) {
        merged_points[last] = points[k + 1];
      }
    } else {
      kept_raw.push(piece);
      merged_points.push(points[k + 1]);
    }
  }
  if kept_raw.is_empty() {
    return (Vec::new(), Vec::new());
  }
  if chain.cyclic {
    let last = merged_points.len() - 1;
    if cell.rank(merged_points[last]) > cell.rank(merged_points[0]) {
      merged_points[0] = merged_points[last];
    } else {
      merged_points[last] = merged_points[0];
    }
  }

  let mut pieces = Vec::with_capacity(kept_raw.len());
  let mut kept = Vec::with_capacity(kept_raw.len());
  for (k, mut piece) in kept_raw.into_iter().enumerate() {
    let curve = &mut piece.curve.curve;
    let (a, b) = (merged_points[k], merged_points[k + 1]);
    if curve.start() != a || curve.end() != b {
      *curve = curve.with_endpoints(a, b);
    }
    kept.push(!piece.far && cell.keeps(cell.classify(curve), curve));
    pieces.push(piece.curve);
  }
  (pieces, kept)
}

//! Input preparation: validated loops with normalized end weights, and the periodic copies of
//! the loops that are relevant to a region, assembled into curve chains.

use std::ops::RangeInclusive;

use rendiation_algebra::*;

use super::geom::*;
use crate::face::PLoop;
use crate::*;

/// A validated input loop. Every curve has unit end weights, so endpoints are stored exactly
/// and survive translations bitwise; consecutive curves share their endpoints exactly.
pub(super) struct PreparedLoop {
  pub curves: Vec<TrimCurve>,
  pub bboxes: Vec<BBox>,
  pub bbox: BBox,
  /// Lattice shift (in periods per axis) from the loop start to its end, zero when closed.
  pub shift: [i64; 2],
}

impl PreparedLoop {
  pub fn is_band(&self) -> bool {
    self.shift != [0, 0]
  }
}

/// A chain of curves in one periodic copy (or a run of consecutive copies for band loops).
/// Consecutive curves share endpoints exactly. An open chain starts and ends outside the region
/// it was built for.
pub(super) struct Chain {
  pub curves: Vec<TrimCurve>,
  pub bboxes: Vec<BBox>,
  pub cyclic: bool,
}

pub(super) fn validate_periods(p_period: [Option<f64>; 2]) -> ConvertResult<[Option<f64>; 2]> {
  for n in p_period.iter().flatten() {
    if !(n.is_finite() && *n >= 1. && n.fract() == 0.) {
      return Err(ConvertError::invalid(
        None,
        format!("trim: P space period {n} is not a positive integer"),
      ));
    }
  }
  Ok(p_period)
}

pub(super) fn prepare_loops(
  loops: &[PLoop],
  periods: [Option<f64>; 2],
  eps: f64,
) -> ConvertResult<Vec<PreparedLoop>> {
  loops
    .iter()
    .enumerate()
    .map(|(index, l)| prepare_loop(index, l, periods, eps))
    .collect()
}

fn prepare_loop(
  index: usize,
  l: &PLoop,
  periods: [Option<f64>; 2],
  eps: f64,
) -> ConvertResult<PreparedLoop> {
  let topology = |msg: String| ConvertError::topology(None, format!("trim: loop {index}: {msg}"));
  if l.curves.is_empty() {
    return Err(topology("loop has no curves".into()));
  }
  let mut shift = [0; 2];
  let mut offset = V2::new(0., 0.);
  for axis in 0..2 {
    let w = l.winding[axis];
    if w == 0 {
      continue;
    }
    let Some(n) = periods[axis] else {
      return Err(topology(format!(
        "winding {w} along axis {axis}, which is not periodic"
      )));
    };
    shift[axis] = w as i64;
    set_coord(&mut offset, axis, w as f64 * n);
  }

  let mut curves = Vec::with_capacity(l.curves.len());
  for (k, c) in l.curves.iter().enumerate() {
    let cp = &c.curve.control_points;
    if cp.len() < 2 {
      return Err(ConvertError::invalid(
        None,
        format!("trim: loop {index} curve {k} has less than two control points"),
      ));
    }
    if cp
      .iter()
      .any(|p| !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite() && p.z > 0.))
    {
      return Err(ConvertError::invalid(
        None,
        format!("trim: loop {index} curve {k} has non finite coordinates or non positive weights"),
      ));
    }
    curves.push(TrimCurve {
      curve: c.curve.normalize_end_weights(),
      source: c.source,
      exact: c.exact,
    });
  }

  let gap_check = |a: V2, b: V2, what: &str| {
    let gap = (a - b).length();
    if gap.is_finite() && gap <= eps {
      Ok(())
    } else {
      Err(topology(format!(
        "{what}: gap {gap:e} between ({}, {}) and ({}, {})",
        a.x, a.y, b.x, b.y
      )))
    }
  };
  for k in 1..curves.len() {
    let end = curves[k - 1].curve.end();
    let start = curves[k].curve.start();
    if end != start {
      gap_check(
        end,
        start,
        &format!("curves {} and {k} are not connected", k - 1),
      )?;
      curves[k].curve = curves[k].curve.with_endpoints(end, curves[k].curve.end());
    }
  }
  let first = curves[0].curve.start();
  let last = curves.len() - 1;
  let end = curves[last].curve.end();
  if shift == [0, 0] {
    if end != first {
      gap_check(end, first, "loop is not closed")?;
      curves[last].curve = curves[last]
        .curve
        .with_endpoints(curves[last].curve.start(), first);
    }
  } else {
    gap_check(
      end,
      first + offset,
      "band loop end does not match its start shifted by winding times period",
    )?;
  }

  let bboxes: Vec<_> = curves.iter().map(|c| c.curve.control_bbox()).collect();
  let bbox = bboxes
    .iter()
    .copied()
    .reduce(bbox_union)
    .unwrap_or(bboxes[0]);
  Ok(PreparedLoop {
    curves,
    bboxes,
    bbox,
    shift,
  })
}

/// Lattice index ranges (per axis, in periods) of the translates of `bbox` overlapping
/// `region`. Non periodic axes only allow index 0. `None` when nothing overlaps.
pub(super) fn lattice_ranges(
  bbox: BBox,
  region: BBox,
  periods: [Option<f64>; 2],
) -> Option<[RangeInclusive<i64>; 2]> {
  let range = |axis: usize| {
    let (lo, hi) = (coord(bbox.0, axis), coord(bbox.1, axis));
    let (rlo, rhi) = (coord(region.0, axis), coord(region.1, axis));
    match periods[axis] {
      Some(n) => {
        let k0 = ((rlo - hi) / n).ceil() as i64;
        let k1 = ((rhi - lo) / n).floor() as i64;
        (k0 <= k1).then_some(k0..=k1)
      }
      None => (lo <= rhi && rlo <= hi).then_some(0..=0),
    }
  };
  Some([range(0)?, range(1)?])
}

pub(super) fn lattice_offset(k: [i64; 2], periods: [Option<f64>; 2]) -> V2 {
  V2::new(
    periods[0].map_or(0., |n| k[0] as f64 * n),
    periods[1].map_or(0., |n| k[1] as f64 * n),
  )
}

fn translated(curves: &[TrimCurve], bboxes: &[BBox], offset: V2) -> (Vec<TrimCurve>, Vec<BBox>) {
  let curves = curves
    .iter()
    .map(|c| TrimCurve {
      curve: c.curve.translate(offset),
      source: c.source,
      exact: c.exact,
    })
    .collect();
  let bboxes = bboxes.iter().map(|b| bbox_translate(*b, offset)).collect();
  (curves, bboxes)
}

/// All chains (periodic copies of the loops) whose curves may reach into `region`.
pub(super) fn region_chains(
  loops: &[PreparedLoop],
  periods: [Option<f64>; 2],
  region: BBox,
) -> Vec<Chain> {
  let mut chains = Vec::new();
  for l in loops {
    let Some([ru, rv]) = lattice_ranges(l.bbox, region, periods) else {
      continue;
    };
    if !l.is_band() {
      for ku in ru {
        for kv in rv.clone() {
          let offset = lattice_offset([ku, kv], periods);
          let (curves, bboxes) = translated(&l.curves, &l.bboxes, offset);
          chains.push(Chain {
            curves,
            bboxes,
            cyclic: true,
          });
        }
      }
      continue;
    }

    // Translates `key + m·shift` with the same key are consecutive copies of one infinite
    // band curve: group the relevant ones and concatenate the copies between them.
    let d = l.shift;
    let mut groups: Vec<([i64; 2], i64, i64)> = Vec::new();
    for ku in ru {
      for kv in rv.clone() {
        let m = if d[0] != 0 {
          ku.div_euclid(d[0])
        } else {
          kv.div_euclid(d[1])
        };
        let key = [ku - m * d[0], kv - m * d[1]];
        match groups.iter_mut().find(|g| g.0 == key) {
          Some(g) => {
            g.1 = g.1.min(m);
            g.2 = g.2.max(m);
          }
          None => groups.push((key, m, m)),
        }
      }
    }
    for (key, m0, m1) in groups {
      let mut curves: Vec<TrimCurve> = Vec::new();
      let mut bboxes = Vec::new();
      for m in m0..=m1 {
        let offset = lattice_offset([key[0] + m * d[0], key[1] + m * d[1]], periods);
        let (mut copy, copy_boxes) = translated(&l.curves, &l.bboxes, offset);
        if let Some(prev) = curves.last() {
          let join = prev.curve.end();
          let first = &mut copy[0].curve;
          *first = first.with_endpoints(join, first.end());
        }
        curves.extend(copy);
        bboxes.extend(copy_boxes);
      }
      chains.push(Chain {
        curves,
        bboxes,
        cyclic: false,
      });
    }
  }
  chains
}

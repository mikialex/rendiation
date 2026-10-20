//! Per axis P space maps shared by all surface kinds.

use std::f64::consts::{FRAC_PI_2, TAU};

use super::*;

/// Map of an angular axis built per face: equal circular arcs of at most π/2, P = 0 at
/// `range.0`. Periodic (with period 2π) when the range spans a full turn within
/// `ctx.angular_tol`, ranges longer than a full turn are rejected.
pub fn angular_map(range: (f64, f64), ctx: &ConvertCtx) -> ConvertResult<PiecewiseParamMap> {
  check_range(range)?;
  let (a0, a1) = range;
  let span = a1 - a0;
  let full = (span - TAU).abs() <= ctx.angular_tol.max(16. * f64::EPSILON * TAU);
  if !full && span > TAU {
    return Err(ConvertError::topology(
      None,
      format!("angular range {a0}..{a1} exceeds a full turn"),
    ));
  }
  let a1 = if full { a0 + TAU } else { a1 };
  Ok(arc_map(a0, a1, full.then_some(TAU)))
}

/// Non periodic angular map over `range` (sphere latitude), arcs of at most π/2.
pub fn bounded_angular_map(range: (f64, f64)) -> ConvertResult<PiecewiseParamMap> {
  check_range(range)?;
  Ok(arc_map(range.0, range.1, None))
}

fn arc_map(a0: f64, a1: f64, period: Option<f64>) -> PiecewiseParamMap {
  let arcs = split_angle_range(a0, a1, FRAC_PI_2);
  let mut breaks = vec![a0];
  breaks.extend(arcs.iter().map(|a| a.1));
  PiecewiseParamMap::new(breaks, vec![SegmentMap::CircularArc; arcs.len()], period)
}

/// One affine segment over exactly `range`.
pub fn affine_map(range: (f64, f64)) -> ConvertResult<PiecewiseParamMap> {
  check_range(range)?;
  Ok(PiecewiseParamMap::single_affine(range.0, range.1))
}

fn check_range(range: (f64, f64)) -> ConvertResult<()> {
  if !(range.0.is_finite() && range.1.is_finite() && range.0 < range.1) {
    return Err(ConvertError::numerical(
      None,
      format!("invalid native range {}..{}", range.0, range.1),
    ));
  }
  Ok(())
}

/// P cells of a fixed map (B-spline knot spans, swept curve segments) intersecting `range`.
///
/// Preconditions: for a periodic map `range.0` lies in `[b0, b0 + period]` (`b0` the first
/// break) and the range spans at most one period, cell indices `>= segment_count` then stand
/// for segment `index mod segment_count`. For a non periodic map the range lies inside the
/// breaks. Both are checked with a tolerance relative to the map length.
pub fn fixed_map_cells(map: &PiecewiseParamMap, range: (f64, f64)) -> ConvertResult<Vec<usize>> {
  let n = map.segment_count();
  let (lo, hi) = (map.breaks[0], map.breaks[n]);
  let eps = 1e-9 * (hi - lo);
  if !(range.0.is_finite() && range.1.is_finite() && range.0 <= range.1) {
    return Err(ConvertError::numerical(
      None,
      format!("invalid native range {}..{}", range.0, range.1),
    ));
  }
  let (r0, r1) = match map.period {
    Some(period) => {
      if range.0 < lo - eps || range.0 > lo + period + eps {
        return Err(ConvertError::numerical(
          None,
          format!(
            "range start {} not inside the base period [{lo}, {}] of a periodic axis",
            range.0,
            lo + period
          ),
        ));
      }
      if range.1 - range.0 > period + eps {
        return Err(ConvertError::topology(
          None,
          format!("range {}..{} exceeds the period {period}", range.0, range.1),
        ));
      }
      (range.0.max(lo), range.1.max(lo))
    }
    None => {
      if range.0 < lo - eps || range.1 > hi + eps {
        return Err(ConvertError::numerical(
          None,
          format!(
            "range {}..{} outside the domain {lo}..{hi}",
            range.0, range.1
          ),
        ));
      }
      (range.0.clamp(lo, hi), range.1.clamp(lo, hi))
    }
  };
  let first = map.to_p(r0).floor().max(0.) as usize;
  let mut last = (map.to_p(r1).ceil() as usize).saturating_sub(1).max(first);
  let mut first = first;
  if map.period.is_none() {
    first = first.min(n - 1);
    last = last.min(n - 1);
  }
  Ok((first..=last).collect())
}

/// For native `x`: P cell (clamped into the segments for non periodic maps, so out of range
/// values extrapolate the end segments), local Bézier parameter and `dt/dx`.
pub fn map_local(map: &PiecewiseParamMap, x: f64) -> (i64, f64, f64) {
  let n = map.segment_count() as i64;
  let p = map.to_p(x);
  let mut cell = p.floor() as i64;
  if map.period.is_none() {
    cell = cell.clamp(0, n - 1);
  }
  let t = p - cell as f64;
  let s = cell.rem_euclid(n) as usize;
  let span = map.breaks[s + 1] - map.breaks[s];
  let dt = match map.segments[s] {
    SegmentMap::Affine => 1. / span,
    SegmentMap::CircularArc => {
      let h = 0.5 * (arc_angle_from_t(t, span) - 0.5 * span);
      1. / (4. * (0.25 * span).tan() * h.cos().powi(2))
    }
  };
  (cell, t, dt)
}

/// Segment index of P cell `cell` (periodic wrap).
pub fn segment_of_cell(map: &PiecewiseParamMap, cell: i64) -> usize {
  let n = map.segment_count() as i64;
  match map.period {
    Some(_) => cell.rem_euclid(n) as usize,
    None => cell.clamp(0, n - 1) as usize,
  }
}

/// The representative `x + k·period` closest to `reference`.
pub fn nearest_representative(x: f64, period: Option<f64>, reference: Option<f64>) -> f64 {
  match (period, reference) {
    (Some(period), Some(r)) if r.is_finite() => x + period * ((r - x) / period).round(),
    _ => x,
  }
}

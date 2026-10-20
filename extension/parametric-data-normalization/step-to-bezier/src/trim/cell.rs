//! Geometry of one unit cell: snapping onto its boundary lines, splitting curves at the lines,
//! piece classification and the counter-clockwise boundary parameter.

use rendiation_algebra::*;

use super::geom::*;
use crate::math::*;

/// Parameter tolerance for the Bernstein root finder.
const ROOT_T_EPS: f64 = 1e-12;
/// Roots closer than this to a curve end are the curve end itself (handled by vertex snapping).
const ROOT_END_EPS: f64 = 1e-12;
/// Interior samples used to classify a piece (plus both endpoints).
const CLASSIFY_SAMPLES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PieceClass {
  Inside,
  Outside,
  /// Lies on the line of the given edge (`Edge` index) within `eps`.
  OnEdge(usize),
}

/// Unit cell `[min, max]` in P space. Edges are numbered counter-clockwise: 0 bottom
/// (travel +u), 1 right (+v), 2 top (−u), 3 left (−v). Corner `k` is the start of edge `k`.
/// The boundary parameter `s ∈ [0, 4)` runs counter-clockwise from corner 0.
#[derive(Debug, Clone, Copy)]
pub(super) struct Cell {
  pub index: (usize, usize),
  pub min: V2,
  pub max: V2,
  pub eps: f64,
}

impl Cell {
  pub fn new(index: (usize, usize), eps: f64) -> Self {
    let min = V2::new(index.0 as f64, index.1 as f64);
    Self {
      index,
      min,
      max: min + V2::new(1., 1.),
      eps,
    }
  }

  pub fn bbox(&self) -> BBox {
    (self.min, self.max)
  }

  pub fn corner(&self, k: usize) -> V2 {
    match k % 4 {
      0 => self.min,
      1 => V2::new(self.max.x, self.min.y),
      2 => self.max,
      _ => V2::new(self.min.x, self.max.y),
    }
  }

  pub fn edge_direction(edge: usize) -> V2 {
    match edge % 4 {
      0 => V2::new(1., 0.),
      1 => V2::new(0., 1.),
      2 => V2::new(-1., 0.),
      _ => V2::new(0., -1.),
    }
  }

  /// Axis index fixed along the edge and its value.
  pub fn edge_line(&self, edge: usize) -> (usize, f64) {
    match edge % 4 {
      0 => (1, self.min.y),
      1 => (0, self.max.x),
      2 => (1, self.max.y),
      _ => (0, self.min.x),
    }
  }

  /// Snap `p` onto every cell boundary line closer than `eps` (only near the cell edge span),
  /// so points near a corner become the exact corner.
  pub fn snap(&self, mut p: V2) -> V2 {
    let eps = self.eps;
    for axis in 0..2 {
      let other = 1 - axis;
      let c = coord(p, other);
      if c < coord(self.min, other) - eps || c > coord(self.max, other) + eps {
        continue;
      }
      for value in [coord(self.min, axis), coord(self.max, axis)] {
        if (coord(p, axis) - value).abs() <= eps {
          set_coord(&mut p, axis, value);
        }
      }
    }
    p
  }

  /// Number of coordinates lying exactly on a boundary value (2 for corners).
  pub fn rank(&self, p: V2) -> u8 {
    u8::from(p.x == self.min.x || p.x == self.max.x)
      + u8::from(p.y == self.min.y || p.y == self.max.y)
  }

  /// Signed distance like value: positive inside, negative outside, zero on the boundary.
  pub fn depth(&self, p: V2) -> f64 {
    (p.x - self.min.x)
      .min(self.max.x - p.x)
      .min(p.y - self.min.y)
      .min(self.max.y - p.y)
  }

  /// Interior parameters where `curve` crosses or touches one of the four boundary lines inside
  /// the edge span (± eps), with the crossing point snapped exactly onto the line (and onto the
  /// corner when near one). Sorted by parameter, near duplicates merged.
  pub fn split_points(&self, curve: &RationalBezierCurve2, bbox: BBox) -> Vec<(f64, V2)> {
    let eps = self.eps;
    let mut out = Vec::new();
    for edge in 0..4 {
      let (axis, value) = self.edge_line(edge);
      if coord(bbox.0, axis) > value + eps || coord(bbox.1, axis) < value - eps {
        continue;
      }
      let axis_enum = if axis == 0 { Axis::U } else { Axis::V };
      let other = 1 - axis;
      for t in axis_line_crossings(curve, axis_enum, value, ROOT_T_EPS) {
        if t <= ROOT_END_EPS || t >= 1. - ROOT_END_EPS {
          continue;
        }
        let mut q = curve.eval(t);
        set_coord(&mut q, axis, value);
        let c = coord(q, other);
        if c < coord(self.min, other) - eps || c > coord(self.max, other) + eps {
          continue;
        }
        out.push((t, self.snap(q)));
      }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, V2)> = Vec::with_capacity(out.len());
    for (t, q) in out {
      match merged.last_mut() {
        Some(last) if t - last.0 <= ROOT_T_EPS => {
          if self.rank(q) > self.rank(last.1) {
            last.1 = q;
          }
        }
        _ => merged.push((t, q)),
      }
    }
    merged
  }

  /// Classify a piece that does not cross any boundary line in its interior.
  pub fn classify(&self, curve: &RationalBezierCurve2) -> PieceClass {
    let eps = self.eps;
    let samples: Vec<_> = (0..=CLASSIFY_SAMPLES)
      .map(|k| curve.eval(k as f64 / CLASSIFY_SAMPLES as f64))
      .collect();
    let (min_depth, max_depth) = samples
      .iter()
      .map(|p| self.depth(*p))
      .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| {
        (lo.min(d), hi.max(d))
      });
    if max_depth > eps {
      return PieceClass::Inside;
    }
    if min_depth < -eps {
      return PieceClass::Outside;
    }
    let mut on_edges = (0..4).filter(|edge| {
      let (axis, value) = self.edge_line(*edge);
      samples
        .iter()
        .all(|p| (coord(*p, axis) - value).abs() <= eps)
    });
    match (on_edges.next(), on_edges.next()) {
      (Some(edge), None) => PieceClass::OnEdge(edge),
      // Within eps of two lines: a sliver at a corner, nothing to keep.
      (Some(_), Some(_)) => PieceClass::Outside,
      _ if max_depth > 0. => PieceClass::Inside,
      _ => PieceClass::Outside,
    }
  }

  /// Whether a piece of the given class is part of the trimmed region of this cell. Pieces on
  /// an edge line are kept only when their material side (left) faces into the cell.
  pub fn keeps(&self, class: PieceClass, curve: &RationalBezierCurve2) -> bool {
    match class {
      PieceClass::Inside => true,
      PieceClass::Outside => false,
      PieceClass::OnEdge(edge) => {
        let d = curve.end() - curve.start();
        Cell::edge_direction(edge).dot(d) > 0.
      }
    }
  }

  /// Counter-clockwise boundary parameter of a point lying exactly on the boundary.
  pub fn boundary_param(&self, p: V2) -> Option<f64> {
    let in_x = p.x >= self.min.x && p.x <= self.max.x;
    let in_y = p.y >= self.min.y && p.y <= self.max.y;
    let s = if p.y == self.min.y && in_x {
      p.x - self.min.x
    } else if p.x == self.max.x && in_y {
      1. + (p.y - self.min.y)
    } else if p.y == self.max.y && in_x {
      2. + (self.max.x - p.x)
    } else if p.x == self.min.x && in_y {
      3. + (self.max.y - p.y)
    } else {
      return None;
    };
    Some(if s >= 4. { 0. } else { s })
  }

  /// Angle of direction `v` at boundary parameter `s`, measured counter-clockwise from the
  /// boundary travel direction leaving that point, in `[−π/2, 3π/2)`. Directions into the
  /// cell are in `[0, π]` (edge points) or `[0, π/2]` (corners).
  pub fn boundary_angle(s: f64, v: V2) -> f64 {
    let b = Cell::edge_direction(s.floor() as usize);
    let a = cross(b, v).atan2(b.dot(v));
    if a < -std::f64::consts::FRAC_PI_2 {
      a + 2. * std::f64::consts::PI
    } else {
      a
    }
  }
}

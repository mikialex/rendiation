use std::f64::consts::{FRAC_PI_2, PI, TAU};

use rendiation_algebra::*;

use super::geom::*;
use super::*;
use crate::brep::EdgeId;
use crate::math::*;

fn edge_source(id: u32) -> TrimCurveSource {
  TrimCurveSource::Edge {
    edge: EdgeId(id),
    reversed: false,
  }
}

fn tc(curve: RationalBezierCurve2, source: TrimCurveSource) -> TrimCurve {
  TrimCurve {
    curve,
    source,
    exact: true,
  }
}

/// Closed polygon loop, each side its own edge id starting at `first_id`.
fn polygon(points: &[(f64, f64)], first_id: u32) -> PLoop {
  let n = points.len();
  let p = |k: usize| V2::new(points[k % n].0, points[k % n].1);
  PLoop {
    curves: (0..n)
      .map(|k| {
        tc(
          RationalBezierCurve2::line(p(k), p(k + 1)),
          edge_source(first_id + k as u32),
        )
      })
      .collect(),
    winding: [0, 0],
  }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64, ccw: bool, first_id: u32) -> (PLoop, Vec<(f64, f64)>) {
  let mut pts = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
  if !ccw {
    pts.reverse();
  }
  (polygon(&pts, first_id), pts)
}

/// Circle of rational quadratic quarter arcs, exactly closed.
fn circle(center: V2, r: f64, ccw: bool, first_id: u32) -> PLoop {
  let x = V2::new(1., 0.);
  let y = V2::new(0., 1.);
  let mut curves: Vec<_> = split_angle_range(0., TAU, FRAC_PI_2)
    .into_iter()
    .map(|(a0, a1)| elliptic_arc_2d(center, x, y, r, r, a0, a1))
    .collect();
  let first = curves[0].start();
  let last = curves.len() - 1;
  curves[last] = curves[last].with_endpoints(curves[last].start(), first);
  if !ccw {
    curves.reverse();
    curves = curves.iter().map(|c| c.reversed()).collect();
  }
  PLoop {
    curves: curves
      .into_iter()
      .enumerate()
      .map(|(k, c)| tc(c, edge_source(first_id + k as u32)))
      .collect(),
    winding: [0, 0],
  }
}

fn cells_in(i: std::ops::Range<usize>, j: std::ops::Range<usize>) -> Vec<(usize, usize)> {
  i.flat_map(|i| j.clone().map(move |j| (i, j))).collect()
}

fn loop_area(l: &TrimLoop) -> f64 {
  loop_signed_area(l.curves.iter().map(|c| &c.curve), 8)
}

fn trim_area(t: &CellTrim) -> f64 {
  match t {
    CellTrim::Empty => 0.,
    CellTrim::Trimmed(loops) => loops.iter().map(loop_area).sum(),
  }
}

/// Signed area of closed input loops.
fn region_area(loops: &[PLoop]) -> f64 {
  loops
    .iter()
    .map(|l| loop_signed_area(l.curves.iter().map(|c| &c.curve), 8))
    .sum()
}

/// Region area per period for band loops each covering exactly one period along u: `−∮ y dx`.
fn band_region_area(loops: &[PLoop]) -> f64 {
  loops
    .iter()
    .flat_map(|l| &l.curves)
    .map(|c| {
      let mut sum = 0.;
      let n = 400;
      for k in 0..n {
        // midpoint rule is plenty with this many samples for smooth curves
        let t = (k as f64 + 0.5) / n as f64;
        let (p, d) = c.curve.eval_with_derivative(t);
        sum -= p.y * d.x / n as f64;
      }
      sum
    })
    .sum()
}

struct Checked {
  results: Vec<((usize, usize), CellTrim)>,
}

impl Checked {
  fn area(&self, cell: (usize, usize)) -> f64 {
    self
      .results
      .iter()
      .find(|(c, _)| *c == cell)
      .map(|(_, t)| trim_area(t))
      .unwrap()
  }

  fn total(&self) -> f64 {
    self.results.iter().map(|(_, t)| trim_area(t)).sum()
  }

  fn trim(&self, cell: (usize, usize)) -> &CellTrim {
    &self.results.iter().find(|(c, _)| *c == cell).unwrap().1
  }

  fn is_empty(&self, cell: (usize, usize)) -> bool {
    matches!(self.trim(cell), CellTrim::Empty)
  }

  fn is_full_rect(&self, cell: (usize, usize)) -> bool {
    match self.trim(cell) {
      CellTrim::Trimmed(loops) => {
        loops.len() == 1
          && loops[0].curves.len() == 4
          && loops[0]
            .curves
            .iter()
            .all(|c| c.source == TrimCurveSource::PatchBoundary)
      }
      CellTrim::Empty => false,
    }
  }
}

/// Run `trim_cells` and check the per loop invariants: exact closure, everything inside the
/// unit square (control points included), sources (edge pieces lie on the input curve of their edge, patch boundary
/// pieces lie on the cell boundary), per cell area within `[0, 1]`.
fn run(loops: &[PLoop], period: [Option<f64>; 2], cells: &[(usize, usize)]) -> Checked {
  let trims = trim_cells(loops, period, cells, &TrimParams::default()).unwrap();
  assert_eq!(trims.len(), cells.len());
  let mut by_edge: Vec<(TrimCurveSource, Vec<&RationalBezierCurve2>)> = Vec::new();
  for l in loops {
    for c in &l.curves {
      match by_edge.iter_mut().find(|(s, _)| *s == c.source) {
        Some((_, list)) => list.push(&c.curve),
        None => by_edge.push((c.source, vec![&c.curve])),
      }
    }
  }
  let tol = 1e-9;
  for (cell, trim) in cells.iter().zip(&trims) {
    let CellTrim::Trimmed(out) = trim else {
      continue;
    };
    assert!(!out.is_empty(), "cell {cell:?}: Trimmed without loops");
    for (li, l) in out.iter().enumerate() {
      let n = l.curves.len();
      assert!(n > 0, "cell {cell:?}: empty loop");
      for k in 0..n {
        let (a, b) = (&l.curves[k].curve, &l.curves[(k + 1) % n].curve);
        assert!(
          a.end() == b.start(),
          "cell {cell:?} loop {li}: curve {k} end {:?} != next start {:?}",
          a.end(),
          b.start()
        );
      }
      for (k, c) in l.curves.iter().enumerate() {
        let inside = |p: V2| p.x >= -tol && p.x <= 1. + tol && p.y >= -tol && p.y <= 1. + tol;
        for s in 0..=16 {
          let p = c.curve.eval(s as f64 / 16.);
          assert!(
            inside(p),
            "cell {cell:?} loop {li} curve {k}: point {p:?} outside"
          );
        }
        for cp in &c.curve.control_points {
          let p = cp.to_euclid();
          assert!(
            inside(p),
            "cell {cell:?} loop {li} curve {k}: control point {p:?} outside"
          );
        }
        let offset = V2::new(cell.0 as f64, cell.1 as f64);
        match c.source {
          TrimCurveSource::PatchBoundary => {
            assert_eq!(c.curve.degree(), 1);
            let (s, e) = (c.curve.start(), c.curve.end());
            let on_x = (s.x == e.x) && (s.x == 0. || s.x == 1.);
            let on_y = (s.y == e.y) && (s.y == 0. || s.y == 1.);
            assert!(
              on_x || on_y,
              "cell {cell:?}: boundary walk {s:?} -> {e:?} not on boundary"
            );
          }
          source => {
            let originals = &by_edge
              .iter()
              .find(|(s, _)| *s == source)
              .expect("unknown source in output")
              .1;
            for s in [0.25, 0.5, 0.75] {
              let p = c.curve.eval(s) + offset;
              let d = shifted_distance(originals, p, period);
              assert!(
                d < 1e-7,
                "cell {cell:?}: piece of {source:?} is {d} off its edge"
              );
            }
          }
        }
      }
    }
    let area = trim_area(trim);
    assert!(
      area > -1e-9 && area < 1. + 1e-9,
      "cell {cell:?}: area {area} out of [0, 1]"
    );
  }
  Checked {
    results: cells.iter().copied().zip(trims).collect(),
  }
}

fn shifted_distance(curves: &[&RationalBezierCurve2], p: V2, period: [Option<f64>; 2]) -> f64 {
  let range = |n: Option<f64>| match n {
    Some(_) => -3..=3,
    None => 0..=0,
  };
  let mut best = f64::INFINITY;
  for ku in range(period[0]) {
    for kv in range(period[1]) {
      let shift = V2::new(
        ku as f64 * period[0].unwrap_or(0.),
        kv as f64 * period[1].unwrap_or(0.),
      );
      for c in curves {
        best = best.min(closest_point_on_curve(*c, p - shift, None).1);
      }
    }
  }
  best
}

/// Signed area of the polygon clipped to the cell (Sutherland–Hodgman, correct for concave
/// input as far as the area is concerned).
fn clipped_polygon_area(poly: &[(f64, f64)], cell: (usize, usize)) -> f64 {
  let (x0, y0) = (cell.0 as f64, cell.1 as f64);
  let mut pts: Vec<V2> = poly.iter().map(|p| V2::new(p.0, p.1)).collect();
  let planes: [(V2, f64); 4] = [
    (V2::new(1., 0.), x0),
    (V2::new(-1., 0.), -(x0 + 1.)),
    (V2::new(0., 1.), y0),
    (V2::new(0., -1.), -(y0 + 1.)),
  ];
  for (n, c) in planes {
    let mut out = Vec::new();
    for k in 0..pts.len() {
      let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
      let (da, db) = (a.dot(n) - c, b.dot(n) - c);
      if da >= 0. {
        out.push(a);
      }
      if (da >= 0.) != (db >= 0.) {
        out.push(a + (b - a) * (da / (da - db)));
      }
    }
    pts = out;
    if pts.is_empty() {
      return 0.;
    }
  }
  0.5
    * (0..pts.len())
      .map(|k| cross(pts[k], pts[(k + 1) % pts.len()]))
      .sum::<f64>()
}

fn check_polygon_areas(checked: &Checked, polys: &[Vec<(f64, f64)>]) {
  for (cell, trim) in &checked.results {
    let expected: f64 = polys.iter().map(|p| clipped_polygon_area(p, *cell)).sum();
    let got = trim_area(trim);
    assert!(
      (expected - got).abs() < 1e-9,
      "cell {cell:?}: area {got}, expected {expected}"
    );
  }
}

/// Winding number of a point (cell-local) with respect to the output loops (dense polylines).
fn output_winding(trim: &CellTrim, p: V2) -> i32 {
  let CellTrim::Trimmed(loops) = trim else {
    return 0;
  };
  let mut angle = 0.;
  for l in loops {
    for c in &l.curves {
      let n = 128;
      for k in 0..n {
        let a = c.curve.eval(k as f64 / n as f64) - p;
        let b = c.curve.eval((k + 1) as f64 / n as f64) - p;
        angle += cross(a, b).atan2(a.dot(b));
      }
    }
  }
  (angle / TAU).round() as i32
}

/// Compare membership on a sample grid of every cell with the expected region (`None` near the
/// region boundary).
fn check_membership(checked: &Checked, inside: impl Fn(V2) -> Option<bool>) {
  let m = 9;
  for (cell, trim) in &checked.results {
    for a in 0..m {
      for b in 0..m {
        let local = V2::new((a as f64 + 0.5) / m as f64, (b as f64 + 0.37) / m as f64);
        let p = local + V2::new(cell.0 as f64, cell.1 as f64);
        let Some(expected) = inside(p) else {
          continue;
        };
        let w = output_winding(trim, local);
        assert_eq!(
          w,
          i32::from(expected),
          "cell {cell:?}: winding {w} at {p:?}, expected inside = {expected}"
        );
      }
    }
  }
}

fn circle_membership(center: V2, r: f64, p: V2) -> Option<bool> {
  let d = (p - center).length();
  if (d - r).abs() < 2e-3 {
    None
  } else {
    Some(d < r)
  }
}

#[test]
fn square_inside_one_cell() {
  let (l, pts) = rect(0.2, 0.3, 0.7, 0.9, true, 0);
  let c = run(&[l], [None, None], &cells_in(0..2, 0..2));
  check_polygon_areas(&c, &[pts]);
  assert!((c.area((0, 0)) - 0.3).abs() < 1e-12);
  assert!(c.is_empty((1, 0)) && c.is_empty((0, 1)) && c.is_empty((1, 1)));
  let CellTrim::Trimmed(loops) = c.trim((0, 0)) else {
    panic!()
  };
  assert_eq!(loops.len(), 1);
  assert!(
    loops[0]
      .curves
      .iter()
      .all(|c| matches!(c.source, TrimCurveSource::Edge { .. }))
  );
}

#[test]
fn square_spanning_cells() {
  for (x0, x1) in [(0.5, 2.5), (0.25, 2.75), (0.5, 1.5)] {
    let (l, pts) = rect(x0, x0, x1, x1, true, 0);
    let c = run(&[l], [None, None], &cells_in(0..4, 0..4));
    check_polygon_areas(&c, std::slice::from_ref(&pts));
    let expected = (x1 - x0) * (x1 - x0);
    assert!((c.total() - expected).abs() < 1e-9);
    if x1 > 2. {
      assert!(c.is_full_rect((1, 1)));
    }
  }
}

#[test]
fn square_with_holes() {
  let (outer, po) = rect(0.25, 0.25, 2.75, 2.75, true, 0);
  // One hole fully inside cell (1, 1), one hole crossing cell lines.
  let (h1, p1) = rect(1.2, 1.2, 1.8, 1.8, false, 10);
  let (h2, p2) = rect(0.6, 1.9, 2.4, 2.4, false, 20);
  let c = run(&[outer, h1, h2], [None, None], &cells_in(0..3, 0..3));
  check_polygon_areas(&c, &[po, p1, p2]);
  let expected = 2.5 * 2.5 - 0.36 - 1.8 * 0.5;
  assert!((c.total() - expected).abs() < 1e-9);
  // Cell (1, 1): rectangle plus the hole, the cell boundary is in material.
  let CellTrim::Trimmed(loops) = c.trim((1, 1)) else {
    panic!()
  };
  assert_eq!(loops.len(), 1 + 1, "fragment loop plus hole cycle");
}

#[test]
fn hole_only_cell_is_rectangle_plus_hole() {
  let (outer, po) = rect(-0.5, -0.5, 1.5, 1.5, true, 0);
  let (hole, ph) = rect(0.3, 0.3, 0.6, 0.6, false, 10);
  let c = run(&[outer, hole], [None, None], &[(0, 0)]);
  check_polygon_areas(&c, &[po, ph]);
  let CellTrim::Trimmed(loops) = c.trim((0, 0)) else {
    panic!()
  };
  assert_eq!(loops.len(), 2);
  assert!(loops.iter().any(|l| {
    l.curves.len() == 4
      && l
        .curves
        .iter()
        .all(|c| c.source == TrimCurveSource::PatchBoundary)
  }));
}

/// A comb whose teeth cross the line `y = 1` several times: connecting exits to the next
/// entry in loop order would be wrong here.
#[test]
fn comb_crossing_cell_line() {
  let mut pts = vec![(0.1, 0.2), (2.9, 0.2), (2.9, 1.8)];
  let teeth = 7;
  let w = 2.8 / (2 * teeth) as f64;
  for k in 0..teeth {
    let xr = 2.9 - (2 * k) as f64 * w;
    pts.push((xr - w, 1.8));
    pts.push((xr - w, 0.6));
    pts.push((xr - 2. * w, 0.6));
    pts.push((xr - 2. * w, 1.8));
  }
  pts.pop();
  let l = polygon(&pts, 0);
  assert!(region_area(std::slice::from_ref(&l)) > 0.);
  let c = run(
    std::slice::from_ref(&l),
    [None, None],
    &cells_in(0..3, 0..2),
  );
  check_polygon_areas(&c, &[pts.clone()]);
  assert!((c.total() - region_area(&[l])).abs() < 1e-9);

  // U shape opening downwards crossing y = 1 twice in one cell plus a comb from the top.
  let u = vec![
    (0.1, 0.5),
    (0.3, 0.5),
    (0.3, 1.5),
    (0.7, 1.5),
    (0.7, 0.5),
    (0.9, 0.5),
    (0.9, 1.8),
    (0.1, 1.8),
  ];
  let c = run(&[polygon(&u, 0)], [None, None], &cells_in(0..1, 0..2));
  check_polygon_areas(&c, &[u]);
}

#[test]
fn circles_of_rational_arcs() {
  for (center, r) in [
    (V2::new(1.1, 0.9), 0.7),
    (V2::new(1.5, 1.5), 1.2),
    (V2::new(0.5, 0.5), 0.5), // tangent to all four edges of cell (0, 0)
    (V2::new(1., 1.), 0.5),   // centered on a corner
    (V2::new(2., 1.5), 1.),   // through corners (1, 1.5)… and tangent to x = 3
  ] {
    let l = circle(center, r, true, 0);
    let c = run(
      std::slice::from_ref(&l),
      [None, None],
      &cells_in(0..4, 0..4),
    );
    assert!(
      (c.total() - PI * r * r).abs() < 1e-9,
      "circle {center:?} {r}: total {} expected {}",
      c.total(),
      PI * r * r
    );
    check_membership(&c, |p| circle_membership(center, r, p));
  }
  // Inscribed circle: a single cycle, neighbors empty.
  let l = circle(V2::new(0.5, 0.5), 0.5, true, 0);
  let c = run(&[l], [None, None], &cells_in(0..2, 0..2));
  assert!(c.is_empty((1, 0)) && c.is_empty((0, 1)) && c.is_empty((1, 1)));
  let CellTrim::Trimmed(loops) = c.trim((0, 0)) else {
    panic!()
  };
  assert_eq!(loops.len(), 1);
}

#[test]
fn disk_with_circular_hole() {
  let outer = circle(V2::new(1.5, 1.5), 1.4, true, 0);
  let hole = circle(V2::new(1.3, 1.6), 0.6, false, 10);
  let c = run(&[outer, hole], [None, None], &cells_in(0..3, 0..3));
  assert!((c.total() - PI * (1.4 * 1.4 - 0.36)).abs() < 1e-9);
  check_membership(&c, |p| {
    let a = circle_membership(V2::new(1.5, 1.5), 1.4, p)?;
    let b = circle_membership(V2::new(1.3, 1.6), 0.6, p)?;
    Some(a && !b)
  });
}

struct Lcg(u64);

impl Lcg {
  fn next(&mut self) -> f64 {
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    (self.0 >> 11) as f64 / (1u64 << 53) as f64
  }
}

#[test]
fn random_star_polygons() {
  let mut rng = Lcg(12345);
  for _ in 0..40 {
    let center = V2::new(2. + rng.next(), 2. + rng.next());
    let n = 5 + (rng.next() * 20.) as usize;
    let pts: Vec<_> = (0..n)
      .map(|k| {
        let a = TAU * (k as f64 + 0.3 * rng.next()) / n as f64;
        let r = 0.3 + 1.2 * rng.next();
        (center.x + r * a.cos(), center.y + r * a.sin())
      })
      .collect();
    let l = polygon(&pts, 0);
    let c = run(
      std::slice::from_ref(&l),
      [None, None],
      &cells_in(0..5, 0..5),
    );
    check_polygon_areas(&c, &[pts]);
    assert!((c.total() - region_area(&[l])).abs() < 1e-9);
  }
}

/// Stars with vertices snapped onto cell lines and corners: vertices exactly on the
/// boundary, edges through corners, edges lying on cell lines.
#[test]
fn star_polygons_on_grid_points() {
  let mut rng = Lcg(777);
  for _ in 0..40 {
    let n = 5 + (rng.next() * 12.) as usize;
    let pts: Vec<_> = (0..n)
      .map(|k| {
        let a = TAU * (k as f64 + 0.3 * rng.next()) / n as f64;
        let r = 0.5 + 1.4 * rng.next();
        let snap = |v: f64| (v * 2.).round() / 2.;
        let (x, y) = (2.5 + r * a.cos(), 2.5 + r * a.sin());
        if rng.next() < 0.6 {
          (snap(x), snap(y))
        } else {
          (x, snap(y))
        }
      })
      .collect();
    // Skip self intersecting results of the snapping (simple area based sanity check).
    let l = polygon(&pts, 0);
    let area = region_area(std::slice::from_ref(&l));
    if area <= 1e-6 || !is_simple(&pts) {
      continue;
    }
    let c = run(
      std::slice::from_ref(&l),
      [None, None],
      &cells_in(0..5, 0..5),
    );
    check_polygon_areas(&c, &[pts]);
    assert!((c.total() - area).abs() < 1e-9);
  }
}

fn is_simple(pts: &[(f64, f64)]) -> bool {
  let n = pts.len();
  let p = |k: usize| V2::new(pts[k % n].0, pts[k % n].1);
  for a in 0..n {
    if p(a) == p(a + 1) {
      return false;
    }
    for b in a + 1..n {
      let adjacent = b == a + 1 || (a == 0 && b == n - 1);
      let (p1, p2, q1, q2) = (p(a), p(a + 1), p(b), p(b + 1));
      let o = |a: V2, b: V2, c: V2| cross(b - a, c - a);
      let (d1, d2, d3, d4) = (o(q1, q2, p1), o(q1, q2, p2), o(p1, p2, q1), o(p1, p2, q2));
      if adjacent {
        // Collinear overlapping neighbors fold back.
        if o(p1, p2, q2) == 0. && (q2 - p2).dot(p1 - p2) > 0. {
          return false;
        }
        continue;
      }
      if d1 * d2 <= 0. && d3 * d4 <= 0. {
        return false;
      }
    }
  }
  true
}

#[test]
fn boundary_on_cell_edges() {
  // Exactly cell (1, 0).
  let (l, pts) = rect(1., 0., 2., 1., true, 0);
  let c = run(&[l], [None, None], &cells_in(0..3, 0..2));
  check_polygon_areas(&c, &[pts]);
  assert!(!c.is_empty((1, 0)));
  for cell in [(0, 0), (2, 0), (0, 1), (1, 1), (2, 1)] {
    assert!(c.is_empty(cell), "{cell:?} must be empty");
  }
  let CellTrim::Trimmed(loops) = c.trim((1, 0)) else {
    panic!()
  };
  assert_eq!(loops.len(), 1);
  assert!(
    loops[0]
      .curves
      .iter()
      .all(|c| matches!(c.source, TrimCurveSource::Edge { .. }))
  );

  // L shape along cell lines.
  let pts = vec![(0., 0.), (2., 0.), (2., 1.), (1., 1.), (1., 2.), (0., 2.)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..3, 0..3));
  check_polygon_areas(&c, &[pts]);
  assert!(c.is_empty((1, 1)));
  assert!(c.is_empty((2, 0)));

  // Hole equal to a cell inside a larger square: that cell is empty.
  let (outer, po) = rect(0., 0., 3., 3., true, 0);
  let (hole, ph) = rect(1., 1., 2., 2., false, 10);
  let c = run(&[outer, hole], [None, None], &cells_in(0..3, 0..3));
  check_polygon_areas(&c, &[po, ph]);
  assert!(c.is_empty((1, 1)));
}

/// A boundary lying on the shared edge of two cells ends up in exactly one of them.
#[test]
fn shared_edge_goes_to_material_side() {
  let (l, pts) = rect(0.3, 0.3, 1., 0.7, true, 0);
  let c = run(&[l], [None, None], &cells_in(0..2, 0..1));
  check_polygon_areas(&c, &[pts]);
  assert!(c.is_empty((1, 0)));
  let (l, pts) = rect(1., 0.3, 1.6, 0.7, true, 0);
  let c = run(&[l], [None, None], &cells_in(0..2, 0..1));
  check_polygon_areas(&c, &[pts]);
  assert!(c.is_empty((0, 0)));
}

#[test]
fn corner_and_touch_degeneracies() {
  // Diamond through cell corners and edge midpoints.
  let pts = vec![(1., 0.), (2., 1.), (1., 2.), (0., 1.)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..2, 0..2));
  check_polygon_areas(&c, &[pts]);
  // Diagonal square crossing exactly through the corner (1, 1).
  let pts = vec![(0.5, 0.5), (1.5, 0.5), (1.5, 1.5), (0.5, 1.5)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..2, 0..2));
  check_polygon_areas(&c, &[pts]);
  let pts = vec![(0.2, 0.2), (1., 1.), (1.8, 0.2), (1.8, 1.8), (0.2, 1.8)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..2, 0..2));
  check_polygon_areas(&c, &[pts]);
  // Triangle whose apex touches the top edge from inside.
  let pts = vec![(0.2, 0.2), (0.8, 0.2), (0.5, 1.)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..1, 0..2));
  check_polygon_areas(&c, &[pts]);
  assert!(c.is_empty((0, 1)));
  let CellTrim::Trimmed(loops) = c.trim((0, 0)) else {
    panic!()
  };
  assert_eq!(loops.len(), 1);
  assert_eq!(loops[0].curves.len(), 3);
  // Triangle touching the edge from outside with its apex.
  let pts = vec![(0.2, 1.8), (0.5, 1.), (0.8, 1.8)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..1, 0..2));
  check_polygon_areas(&c, &[pts]);
  assert!(c.is_empty((0, 0)));
  // Two squares touching at the corner (1, 1) (a vertex shared by both loops).
  let (a, pa) = rect(0.5, 0.5, 1., 1., true, 0);
  let (b, pb) = rect(1., 1., 1.5, 1.5, true, 10);
  let c = run(&[a, b], [None, None], &cells_in(0..2, 0..2));
  check_polygon_areas(&c, &[pa, pb]);
  // Bow tie sharing a vertex on a cell edge.
  let (a, pa) = (
    polygon(&[(0.2, 0.5), (1., 0.5), (0.2, 0.8)], 0),
    vec![(0.2, 0.5), (1., 0.5), (0.2, 0.8)],
  );
  let (b, pb) = (
    polygon(&[(1., 0.5), (1.8, 0.2), (1.8, 0.5)], 10),
    vec![(1., 0.5), (1.8, 0.2), (1.8, 0.5)],
  );
  let c = run(&[a, b], [None, None], &cells_in(0..2, 0..1));
  check_polygon_areas(&c, &[pa, pb]);
}

#[test]
fn tiny_pieces_and_near_boundary_vertices() {
  let d = 1e-11;
  // Vertices a hair off the cell lines, and a sub eps edge.
  let pts = vec![
    (0.3, 0.3),
    (1. - d, 0.3),
    (1. + d, 0.3 + d),
    (1.7, 0.3),
    (1.7, 1. + d),
    (1.7, 1.6),
    (0.3, 1.6),
    (0.3, 1. - d),
  ];
  let l = polygon(&pts, 0);
  let c = run(
    std::slice::from_ref(&l),
    [None, None],
    &cells_in(0..2, 0..2),
  );
  assert!((c.total() - region_area(&[l])).abs() < 1e-8);
  for cell in [(0, 0), (1, 0), (0, 1), (1, 1)] {
    let expected = clipped_polygon_area(&pts, cell);
    assert!((c.area(cell) - expected).abs() < 1e-8);
  }
  // A rectangle whose corner is a hair away from the cell corner.
  let pts = vec![(0.5, 0.5), (1. + d, 0.5), (1. + d, 1. - d), (0.5, 1. - d)];
  let c = run(&[polygon(&pts, 0)], [None, None], &cells_in(0..2, 0..2));
  assert!((c.area((0, 0)) - 0.25).abs() < 1e-8);
  assert!(c.is_empty((1, 0)) && c.is_empty((1, 1)) && c.is_empty((0, 1)));
}

/// Wavy band curve along u over one period `[u0, u0 + n]`, made of `segments` cubic pieces.
fn wavy_band(u0: f64, n: f64, v: f64, amp: f64, forward: bool, first_id: u32) -> PLoop {
  let segments = 6;
  let f = |u: f64| V2::new(u, v + amp * (TAU * (u - u0) / n * 2.).sin());
  let df = |u: f64| V2::new(1., amp * TAU / n * 2. * (TAU * (u - u0) / n * 2.).cos());
  let mut curves = Vec::new();
  for k in 0..segments {
    let (a, b) = (
      u0 + n * k as f64 / segments as f64,
      u0 + n * (k + 1) as f64 / segments as f64,
    );
    let h = (b - a) / 3.;
    let mut pts = [f(a), f(a) + df(a) * h, f(b) - df(b) * h, f(b)];
    if k == segments - 1 {
      pts[3] = f(u0) + V2::new(n, 0.);
    }
    if k > 0 {
      let prev: &RationalBezierCurve2 = &curves[k - 1];
      pts[0] = prev.end();
    }
    curves.push(RationalBezierCurve2::from_euclid(&pts, None));
  }
  if !forward {
    curves.reverse();
    curves = curves.iter().map(|c| c.reversed()).collect();
  }
  PLoop {
    curves: curves
      .into_iter()
      .enumerate()
      .map(|(k, c)| tc(c, edge_source(first_id + k as u32)))
      .collect(),
    winding: [if forward { 1 } else { -1 }, 0],
  }
}

fn band_value(u0: f64, n: f64, v: f64, amp: f64, u: f64) -> f64 {
  v + amp * (TAU * (u - u0) / n * 2.).sin()
}

#[test]
fn periodic_strip_between_band_loops() {
  let n = 4.;
  let bottom = wavy_band(0.3, n, 0.9, 0.35, true, 0);
  let top = wavy_band(1.7, n, 2.2, 0.4, false, 100);
  let loops = [bottom, top];
  let c = run(&loops, [Some(n), None], &cells_in(0..4, 0..4));
  let expected = band_region_area(&loops);
  assert!((expected - n * 1.3).abs() < 1e-6);
  assert!(
    (c.total() - expected).abs() < 1e-6,
    "total {} expected {expected}",
    c.total()
  );
  for i in 0..4 {
    assert!(c.is_empty((i, 3)));
  }
  check_membership(&c, |p| {
    let lo = band_value(0.3, n, 0.9, 0.35, p.x);
    let hi = band_value(1.7, n, 2.2, 0.4, p.x);
    if (p.y - lo).abs() < 2e-2 || (p.y - hi).abs() < 2e-2 {
      None
    } else {
      Some(p.y > lo && p.y < hi)
    }
  });
}

#[test]
fn periodic_band_with_flat_lines_on_cell_edges() {
  // Strip v ∈ [1, 2] bounded by straight band loops lying on cell lines.
  let n = 4.;
  let line_band = |v: f64, forward: bool, id: u32| {
    let (a, b) = if forward { (0., n) } else { (n, 0.) };
    PLoop {
      curves: vec![tc(
        RationalBezierCurve2::line(V2::new(a, v), V2::new(b, v)),
        edge_source(id),
      )],
      winding: [if forward { 1 } else { -1 }, 0],
    }
  };
  let loops = [line_band(1., true, 0), line_band(2., false, 1)];
  let c = run(&loops, [Some(n), None], &cells_in(0..4, 0..4));
  for i in 0..4 {
    assert!(c.is_empty((i, 0)));
    assert!(c.is_empty((i, 2)));
    assert!(!c.is_empty((i, 1)));
    assert!((c.area((i, 1)) - 1.).abs() < 1e-12);
  }
}

/// Region above a single band loop, closed at the top by a pole segment lying on a cell line,
/// and the same region without the pole segment (bounded only by the domain).
#[test]
fn periodic_region_above_band_up_to_pole() {
  let n = 4.;
  let band = wavy_band(0.6, n, 1.3, 0.25, true, 0);
  let pole = PLoop {
    curves: vec![tc(
      RationalBezierCurve2::line(V2::new(n, 3.), V2::new(0., 3.)),
      TrimCurveSource::Pole,
    )],
    winding: [-1, 0],
  };
  let loops = [band.clone(), pole];
  let c = run(&loops, [Some(n), None], &cells_in(0..4, 0..4));
  let expected = band_region_area(&loops);
  assert!((c.total() - expected).abs() < 1e-6);
  for i in 0..4 {
    assert!(c.is_empty((i, 3)));
    let CellTrim::Trimmed(cell_loops) = c.trim((i, 2)) else {
      panic!()
    };
    assert!(
      cell_loops
        .iter()
        .flat_map(|l| &l.curves)
        .any(|c| c.source == TrimCurveSource::Pole)
    );
    assert!((c.area((i, 2)) - 1.).abs() < 1e-12);
  }

  let c = run(&[band], [Some(n), None], &cells_in(0..4, 0..3));
  assert!((c.total() - expected).abs() < 1e-6);
  for i in 0..4 {
    assert!(c.is_full_rect((i, 2)));
    assert!(c.is_empty((i, 0)));
  }
}

#[test]
fn doubly_periodic_torus() {
  let (nu, nv) = (4., 3.);
  let period = [Some(nu), Some(nv)];
  let all = cells_in(0..4, 0..3);
  // Disk crossing both seams.
  let center = V2::new(0.1, 2.9);
  let r = 0.7;
  let shifted_member = |p: V2| {
    let mut result = Some(false);
    for ku in -1..=1 {
      for kv in -1..=1 {
        let c = center + V2::new(ku as f64 * nu, kv as f64 * nv);
        match circle_membership(c, r, p) {
          None => return None,
          Some(true) => result = Some(true),
          Some(false) => {}
        }
      }
    }
    result
  };
  let c = run(&[circle(center, r, true, 0)], period, &all);
  assert!((c.total() - PI * r * r).abs() < 1e-9);
  check_membership(&c, shifted_member);

  // Torus minus that disk: holes only, far cells are full through the unbounded fallback.
  let c = run(&[circle(center, r, false, 0)], period, &all);
  assert!((c.total() - (nu * nv - PI * r * r)).abs() < 1e-9);
  check_membership(&c, |p| shifted_member(p).map(|b| !b));
  assert!(c.is_full_rect((2, 1)));

  // Strip between two u band loops on the torus.
  let loops = [
    wavy_band(0.3, nu, 0.8, 0.25, true, 0),
    wavy_band(1.1, nu, 2.1, 0.3, false, 100),
  ];
  let c = run(&loops, period, &all);
  assert!((c.total() - band_region_area(&loops)).abs() < 1e-6);
}

#[test]
fn cylinder_with_seam_edges() {
  // Full cylinder side as one closed loop using a seam edge twice.
  let n = 4.;
  let loops = [PLoop {
    curves: vec![
      tc(
        RationalBezierCurve2::line(V2::new(0., 0.5), V2::new(n, 0.5)),
        edge_source(0),
      ),
      tc(
        RationalBezierCurve2::line(V2::new(n, 0.5), V2::new(n, 1.5)),
        edge_source(1),
      ),
      tc(
        RationalBezierCurve2::line(V2::new(n, 1.5), V2::new(0., 1.5)),
        edge_source(2),
      ),
      tc(
        RationalBezierCurve2::line(V2::new(0., 1.5), V2::new(0., 0.5)),
        edge_source(1),
      ),
    ],
    winding: [0, 0],
  }];
  let c = run(&loops, [Some(n), None], &cells_in(0..4, 0..2));
  assert!((c.total() - n).abs() < 1e-12);
  for i in 0..4 {
    assert!((c.area((i, 0)) - 0.5).abs() < 1e-12);
    assert!((c.area((i, 1)) - 0.5).abs() < 1e-12);
  }
}

#[test]
fn no_loops_means_full() {
  let c = run(&[], [None, None], &[(0, 0), (3, 2)]);
  assert!(c.is_full_rect((0, 0)) && c.is_full_rect((3, 2)));
}

#[test]
fn invalid_input_is_an_error() {
  let params = TrimParams::default();
  let mut open = polygon(&[(0.2, 0.2), (0.8, 0.2), (0.5, 0.8)], 0);
  open.curves[2].curve = RationalBezierCurve2::line(V2::new(0.5, 0.8), V2::new(0.25, 0.2));
  let err = trim_cells(&[open], [None, None], &[(0, 0)], &params)
    .err()
    .unwrap();
  assert_eq!(err.kind, ErrorKind::Topology);

  let band = wavy_band(0., 4., 1., 0.2, true, 0);
  let err = trim_cells(
    std::slice::from_ref(&band),
    [None, None],
    &[(0, 0)],
    &params,
  )
  .err()
  .unwrap();
  assert_eq!(err.kind, ErrorKind::Topology);

  let err = trim_cells(
    std::slice::from_ref(&band),
    [Some(3.), None],
    &[(0, 0)],
    &params,
  )
  .err()
  .unwrap();
  assert_eq!(err.kind, ErrorKind::Topology);

  let err = trim_cells(&[band], [Some(2.5), None], &[(0, 0)], &params)
    .err()
    .unwrap();
  assert_eq!(err.kind, ErrorKind::InvalidData);

  let empty = PLoop {
    curves: Vec::new(),
    winding: [0, 0],
  };
  assert!(trim_cells(&[empty], [None, None], &[(0, 0)], &params).is_err());
}

#[test]
fn sources_and_exact_flags_are_preserved() {
  let mut l = circle(V2::new(1.3, 1.2), 0.8, true, 40);
  l.curves[1].exact = false;
  let c = run(&[l], [None, None], &cells_in(0..2, 0..2));
  let mut inexact = 0;
  for (_, t) in &c.results {
    let CellTrim::Trimmed(loops) = t else {
      continue;
    };
    for curve in loops.iter().flat_map(|l| &l.curves) {
      match curve.source {
        TrimCurveSource::Edge { edge, .. } => {
          assert!((40..44).contains(&edge.0));
          assert_eq!(curve.exact, edge.0 != 41);
          inexact += usize::from(!curve.exact);
        }
        TrimCurveSource::PatchBoundary => assert!(curve.exact),
        TrimCurveSource::Pole => panic!(),
      }
    }
  }
  assert!(inexact >= 2, "edge 41 crosses two cells");
}

#[test]
fn circles_through_corners_and_tangent_to_lines() {
  let s = std::f64::consts::FRAC_1_SQRT_2;
  for (center, r) in [
    // Through all four corners of cell (1, 1), crossing at interior arc parameters.
    (V2::new(1.5, 1.5), s),
    // Arc endpoints on cell lines with the tangent along the line (x = 2, y = 2, ...).
    (V2::new(1., 1.), 1.),
    (V2::new(2., 2.), 1.),
    // Tangent to y = 1 from below and to x = 3 from the left at interior parameters.
    (V2::new(2.3, 0.6), 0.4),
  ] {
    for ccw in [true, false] {
      let l = circle(center, r, ccw, 0);
      let mut loops = vec![l];
      let mut expected = PI * r * r;
      if !ccw {
        let (outer, _) = rect(0., 0., 3.45, 3.45, true, 100);
        loops.push(outer);
        expected = 3.45 * 3.45 - expected;
      }
      let c = run(&loops, [None, None], &cells_in(0..4, 0..4));
      assert!(
        (c.total() - expected).abs() < 1e-9,
        "circle {center:?} {r} ccw {ccw}: total {} expected {expected}",
        c.total()
      );
      check_membership(&c, |p| {
        let inside_circle = circle_membership(center, r, p)?;
        if ccw {
          Some(inside_circle)
        } else {
          let outer = p.x < 3.45 && p.y < 3.45;
          Some(outer && !inside_circle)
        }
      });
    }
  }
}

/// Stars whose sides are rational quadratic bulges (alternating in and out).
#[test]
fn random_curved_stars() {
  let mut rng = Lcg(4242);
  for _ in 0..25 {
    let center = V2::new(2. + rng.next(), 2. + rng.next());
    let n = 5 + (rng.next() * 10.) as usize;
    let pts: Vec<_> = (0..n)
      .map(|k| {
        let a = TAU * (k as f64 + 0.3 * rng.next()) / n as f64;
        let r = 0.6 + 1.0 * rng.next();
        center + V2::new(a.cos(), a.sin()) * r
      })
      .collect();
    let curves: Vec<_> = (0..n)
      .map(|k| {
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        let mid = (a + b) * 0.5;
        let normal = V2::new(b.y - a.y, a.x - b.x) * (0.3 * (rng.next() - 0.5));
        let w = 0.5 + rng.next();
        tc(
          RationalBezierCurve2::from_euclid(&[a, mid + normal, b], Some(&[1., w, 1.])),
          edge_source(k as u32),
        )
      })
      .collect();
    let l = PLoop {
      curves,
      winding: [0, 0],
    };
    let area = region_area(std::slice::from_ref(&l));
    let c = run(
      std::slice::from_ref(&l),
      [None, None],
      &cells_in(0..5, 0..5),
    );
    assert!(
      (c.total() - area).abs() < 1e-9,
      "total {} expected {area}",
      c.total()
    );
  }
}

/// Two parallel band loops winding (1, 1) on a torus: a diagonal strip.
#[test]
fn torus_diagonal_band_loops() {
  let (nu, nv) = (4., 3.);
  let band = |v0: f64, forward: bool, id: u32| {
    let (a, b) = (V2::new(0., v0), V2::new(nu, v0 + nv));
    let (a, b) = if forward { (a, b) } else { (b, a) };
    let mid = (a + b) * 0.5;
    let w = if forward { 1 } else { -1 };
    PLoop {
      curves: vec![
        tc(RationalBezierCurve2::line(a, mid), edge_source(id)),
        tc(RationalBezierCurve2::line(mid, b), edge_source(id + 1)),
      ],
      winding: [w, w],
    }
  };
  let loops = [band(0.5, true, 0), band(1.5, false, 10)];
  let c = run(&loops, [Some(nu), Some(nv)], &cells_in(0..4, 0..3));
  assert!((c.total() - nu).abs() < 1e-9, "total {}", c.total());
  check_membership(&c, |p| {
    let rel = (p.y - 0.75 * p.x - 0.5).rem_euclid(nv);
    if rel < 1e-3 || (rel - 1.).abs() < 1e-3 || rel > nv - 1e-3 {
      None
    } else {
      Some(rel < 1.)
    }
  });
}

/// Band loops along v (winding in the second axis).
#[test]
fn v_periodic_band_loops() {
  let nv = 4.;
  let vband = |u: f64, up: bool, id: u32| {
    let (a, b) = if up {
      (V2::new(u, 0.25), V2::new(u, 0.25 + nv))
    } else {
      (V2::new(u, 0.25 + nv), V2::new(u, 0.25))
    };
    PLoop {
      curves: vec![tc(
        RationalBezierCurve2::from_euclid(&[a, V2::new(u + 0.8, 2.), b], None),
        edge_source(id),
      )],
      winding: [0, if up { 1 } else { -1 }],
    }
  };
  // Material on the left: the down going loop on the left side, the up going one on the right.
  let loops = [vband(0.4, false, 0), vband(2.1, true, 1)];
  let c = run(&loops, [None, Some(nv)], &cells_in(0..4, 0..4));
  let expected = 1.7 * nv;
  assert!((c.total() - expected).abs() < 1e-9, "total {}", c.total());
  for j in 0..4 {
    assert!(c.is_empty((3, j)));
  }
}

/// Random non overlapping tiles on a half unit grid, each its own loop (CCW tiles, or CW holes
/// inside a covering outer square): many coincident edges, shared corners and vertices on cell
/// lines, edges on shared cell edges.
#[test]
fn random_grid_tiles() {
  let mut rng = Lcg(99);
  for round in 0..30 {
    let res = 8;
    let mut used = vec![false; res * res];
    let mut polys = Vec::new();
    let mut loops = Vec::new();
    let holes = round % 3 == 2;
    if holes {
      let (outer, po) = rect(0., 0., 4., 4., true, 1000);
      loops.push(outer);
      polys.push(po);
    }
    for _ in 0..12 {
      let (x, y) = (
        (rng.next() * res as f64) as usize,
        (rng.next() * res as f64) as usize,
      );
      let (w, h) = (
        1 + (rng.next() * 3.) as usize,
        1 + (rng.next() * 3.) as usize,
      );
      if x + w > res || y + h > res {
        continue;
      }
      // Holes must not touch each other or the outer boundary (that would not be a valid
      // region), tiles may share edges (a union of regions).
      let pad = usize::from(holes);
      if holes && (x == 0 || y == 0 || x + w == res || y + h == res) {
        continue;
      }
      let (i0, j0) = (x.saturating_sub(pad), y.saturating_sub(pad));
      let (i1, j1) = ((x + w + pad).min(res), (y + h + pad).min(res));
      if (i0..i1).any(|i| (j0..j1).any(|j| used[i * res + j])) {
        continue;
      }
      for i in x..x + w {
        for j in y..y + h {
          used[i * res + j] = true;
        }
      }
      let f = |v: usize| v as f64 * 0.5;
      let (l, p) = rect(
        f(x),
        f(y),
        f(x + w),
        f(y + h),
        !holes,
        10 * loops.len() as u32,
      );
      loops.push(l);
      polys.push(p);
    }
    if let Err(e) = trim_cells(
      &loops,
      [None, None],
      &cells_in(0..4, 0..4),
      &TrimParams::default(),
    ) {
      println!("round {round} {e}");
      for p in &polys {
        println!("  {p:?}");
      }
    }
    let c = run(&loops, [None, None], &cells_in(0..4, 0..4));
    check_polygon_areas(&c, &polys);
  }
}

/// Random tiles on a u periodic domain (period 4), possibly wrapping around the seam.
#[test]
fn random_periodic_tiles() {
  let mut rng = Lcg(2024);
  let n = 4.;
  for _ in 0..30 {
    let res = 8;
    let mut used = vec![false; res * res];
    let mut polys = Vec::new();
    let mut loops = Vec::new();
    let shift = (rng.next() * 3.) as usize;
    for _ in 0..10 {
      let (x, y) = (
        (rng.next() * res as f64) as usize,
        (rng.next() * res as f64) as usize,
      );
      let (w, h) = (
        1 + (rng.next() * 3.) as usize,
        1 + (rng.next() * 3.) as usize,
      );
      if y + h > res || (x..x + w).any(|i| (y..y + h).any(|j| used[(i % res) * res + j])) {
        continue;
      }
      for i in x..x + w {
        for j in y..y + h {
          used[(i % res) * res + j] = true;
        }
      }
      // Express the tile at a random periodic copy.
      let f = |v: usize| v as f64 * 0.5;
      let dx = shift as f64 * n - n;
      let (l, p) = rect(
        f(x) + dx,
        f(y),
        f(x + w) + dx,
        f(y + h),
        true,
        10 * loops.len() as u32,
      );
      loops.push(l);
      polys.push(p);
    }
    let c = run(&loops, [Some(n), None], &cells_in(0..4, 0..4));
    for (cell, trim) in &c.results {
      let expected: f64 = polys
        .iter()
        .flat_map(|p| {
          (-3..=3).map(move |k| {
            let shifted: Vec<_> = p.iter().map(|q| (q.0 + k as f64 * n, q.1)).collect();
            clipped_polygon_area(&shifted, *cell)
          })
        })
        .sum();
      let got = trim_area(trim);
      assert!(
        (expected - got).abs() < 1e-9,
        "cell {cell:?}: area {got}, expected {expected}"
      );
    }
  }
}

/// Random disks (and holes in a full torus) on a doubly periodic domain.
#[test]
fn random_torus_circles() {
  let mut rng = Lcg(31337);
  let (nu, nv) = (3., 2.);
  for round in 0..20 {
    let center = V2::new(rng.next() * 6. - 1.5, rng.next() * 4. - 1.);
    let r = 0.2 + 0.7 * rng.next();
    let hole = round % 2 == 1;
    let l = circle(center, r, !hole, 0);
    let c = run(&[l], [Some(nu), Some(nv)], &cells_in(0..3, 0..2));
    let disk = PI * r * r;
    let expected = if hole { nu * nv - disk } else { disk };
    assert!(
      (c.total() - expected).abs() < 1e-9,
      "total {} expected {expected}",
      c.total()
    );
  }
}

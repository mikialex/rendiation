use parametric_normalized_model::*;
use rendiation_algebra::*;

/// Ground truth for the trim tests: the original rational trim curves flattened into fine
/// polylines, f64 winding number and distance.
pub struct TrimReference {
  pub polylines: Vec<Vec<Vec2<f64>>>,
}

impl TrimReference {
  /// `tolerance`: maximum sampled deviation of a polyline piece from its curve.
  pub fn new(patch: &TrimmedBezierPatch, tolerance: f64) -> Self {
    let polylines = patch
      .loops
      .iter()
      .map(|trim_loop| {
        let mut points = Vec::new();
        for curve in &trim_loop.curves {
          let c = &curve.curve;
          points.push(c.start());
          flatten(c, (0., c.start()), (1., c.end()), tolerance, 0, &mut points);
        }
        points
      })
      .collect();
    Self { polylines }
  }

  /// Winding number of the closed polylines, positive inside counter clockwise loops.
  pub fn winding(&self, uv: Vec2<f64>) -> i32 {
    let mut winding = 0;
    for points in &self.polylines {
      for (i, &a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        let left = (b.x - a.x) * (uv.y - a.y) - (uv.x - a.x) * (b.y - a.y);
        if a.y <= uv.y && b.y > uv.y && left > 0. {
          winding += 1;
        } else if b.y <= uv.y && a.y > uv.y && left < 0. {
          winding -= 1;
        }
      }
    }
    winding
  }

  pub fn distance(&self, uv: Vec2<f64>) -> f64 {
    let mut distance = f64::INFINITY;
    for points in &self.polylines {
      for (i, &a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        let ab = b - a;
        let len2 = ab.dot(ab);
        let t = if len2 > 0. {
          ((uv - a).dot(ab) / len2).clamp(0., 1.)
        } else {
          0.
        };
        distance = distance.min((uv - (a + ab * t)).length());
      }
    }
    distance
  }
}

/// Appends the points after `p0` up to and including `p1`.
fn flatten(
  c: &RationalBezierCurve2,
  (t0, p0): (f64, Vec2<f64>),
  (t1, p1): (f64, Vec2<f64>),
  tolerance: f64,
  depth: u32,
  out: &mut Vec<Vec2<f64>>,
) {
  let tm = (t0 + t1) * 0.5;
  let pm = c.eval(tm);
  let deviation = [0.25, 0.5, 0.75]
    .iter()
    .map(|s| {
      let p = c.eval(t0 + (t1 - t0) * s);
      (p - (p0 + (p1 - p0) * *s)).length()
    })
    .fold(0., f64::max);
  let curved = c.degree() > 1 && depth < 3;
  if depth < 30 && (deviation > tolerance || curved) {
    flatten(c, (t0, p0), (tm, pm), tolerance, depth + 1, out);
    flatten(c, (tm, pm), (t1, p1), tolerance, depth + 1, out);
  } else {
    out.push(p1);
  }
}

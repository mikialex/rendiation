//! Synthetic trims for the trim tests of this crate and of the GPU version: a unit square
//! surface with trim loops covering the hard cases of the point classification.

use std::f64::consts::FRAC_1_SQRT_2;

use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::test_scenes::TestRng;
use crate::trim::*;

pub const TRIM_TOLERANCE: f64 = 1e-5;

pub fn trim_config() -> TrimQuadraticConfig {
  TrimQuadraticConfig {
    tolerance_3d: TRIM_TOLERANCE,
    min_uv_tolerance: 1e-6,
    max_uv_tolerance: 1e-3,
  }
}

/// The unit square in the xy plane, parametric speed 1, so the uv tolerance is `TRIM_TOLERANCE`.
pub fn unit_surface() -> RationalBezierSurface {
  let p = |x: f64, y: f64| Vec4::new(x, y, 0., 1.);
  RationalBezierSurface::new(1, 1, vec![p(0., 0.), p(1., 0.), p(0., 1.), p(1., 1.)])
}

pub fn curve(curve: RationalBezierCurve2, source: TrimCurveSource) -> TrimCurve {
  TrimCurve {
    curve,
    source,
    exact: true,
  }
}

pub const EDGE: TrimCurveSource = TrimCurveSource::Edge {
  edge: EdgeId(0),
  reversed: false,
};

pub fn polygon(points: &[(f64, f64)], source: TrimCurveSource) -> Vec<TrimCurve> {
  (0..points.len())
    .map(|i| {
      let (a, b) = (points[i], points[(i + 1) % points.len()]);
      curve(
        RationalBezierCurve::line(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1)),
        source,
      )
    })
    .collect()
}

pub fn domain_boundary() -> Vec<TrimCurve> {
  polygon(
    &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)],
    TrimCurveSource::PatchBoundary,
  )
}

/// Four rational quadratic quarter arcs, counter clockwise unless `clockwise`.
pub fn circle(center: (f64, f64), r: f64, clockwise: bool) -> Vec<TrimCurve> {
  let at = |x: f64, y: f64| Vec2::new(center.0 + r * x, center.1 + r * y);
  let mut curves: Vec<_> = [(1., 0.), (0., 1.), (-1., 0.), (0., -1.)]
    .iter()
    .map(|&(x, y)| {
      let (a, b, c) = (at(x, y), at(x - y, y + x), at(-y, x));
      curve(
        RationalBezierCurve::from_euclid(&[a, b, c], Some(&[1., FRAC_1_SQRT_2, 1.])),
        EDGE,
      )
    })
    .collect();
  if clockwise {
    curves.reverse();
    for c in &mut curves {
      c.curve = c.curve.reversed();
    }
  }
  curves
}

pub fn cubic(points: [(f64, f64); 4]) -> TrimCurve {
  let p = points.map(|(x, y)| Vec2::new(x, y));
  curve(RationalBezierCurve::from_euclid(&p, None), EDGE)
}

pub fn quadratic(points: [(f64, f64); 3]) -> TrimCurve {
  let p = points.map(|(x, y)| Vec2::new(x, y));
  curve(RationalBezierCurve::from_euclid(&p, None), EDGE)
}

pub fn trim_patch(loops: Vec<Vec<TrimCurve>>) -> TrimmedBezierPatch {
  TrimmedBezierPatch {
    surface: unit_surface(),
    loops: loops
      .into_iter()
      .map(|curves| TrimLoop { curves })
      .collect(),
    flip_normal: false,
    face: FaceId(0),
    cell: (0, 0),
  }
}

/// Synthetic trims covering the hard cases.
pub fn trim_scenes() -> Vec<TrimmedBezierPatch> {
  // Clockwise cubic blob hole.
  let blob = vec![
    cubic([(0.7, 0.5), (0.6, 0.52), (0.55, 0.7), (0.7, 0.8)]),
    cubic([(0.7, 0.8), (0.85, 0.9), (0.95, 0.6), (0.85, 0.55)]),
    cubic([(0.85, 0.55), (0.8, 0.52), (0.75, 0.48), (0.7, 0.5)]),
  ];
  // Vertices sharing v values with horizontal edges, so rays pass exactly through vertices.
  let star = polygon(
    &[
      (0.1, 0.2),
      (0.4, 0.2),
      (0.5, 0.05),
      (0.6, 0.2),
      (0.9, 0.2),
      (0.7, 0.5),
      (0.9, 0.8),
      (0.6, 0.8),
      (0.5, 0.95),
      (0.4, 0.8),
      (0.1, 0.8),
      (0.3, 0.5),
    ],
    EDGE,
  );
  // Joints at horizontal and vertical tangents (rounded diamond of polynomial quadratics).
  let diamond = vec![
    quadratic([(0.5, 0.1), (0.9, 0.1), (0.9, 0.5)]),
    quadratic([(0.9, 0.5), (0.9, 0.9), (0.5, 0.9)]),
    quadratic([(0.5, 0.9), (0.1, 0.9), (0.1, 0.5)]),
    quadratic([(0.1, 0.5), (0.1, 0.1), (0.5, 0.1)]),
  ];
  // Right side on the patch boundary u = 1.
  let mut right = polygon(&[(0.4, 0.2), (1., 0.2)], EDGE);
  right.truncate(1);
  right.extend(
    polygon(&[(1., 0.2), (1., 0.9)], TrimCurveSource::PatchBoundary)
      .into_iter()
      .take(1),
  );
  right.extend(
    polygon(&[(1., 0.9), (0.4, 0.9), (0.4, 0.2)], EDGE)
      .into_iter()
      .take(2),
  );
  vec![
    trim_patch(vec![
      domain_boundary(),
      circle((0.3, 0.3), 0.15, true),
      blob,
    ]),
    trim_patch(vec![star]),
    trim_patch(vec![diamond]),
    trim_patch(vec![right]),
    // Nested: island inside a hole.
    trim_patch(vec![
      polygon(
        &[(0.05, 0.05), (0.95, 0.05), (0.95, 0.95), (0.05, 0.95)],
        EDGE,
      ),
      circle((0.5, 0.5), 0.35, true),
      circle((0.5, 0.5), 0.15, false),
    ]),
  ]
}

/// Random points (also outside the domain) and points on the critical lines: the v of every
/// segment end point and band boundary, the u of every `max_u`.
pub fn sample_points(
  encoding: &TrimEncoding,
  patch: usize,
  rng: &mut TestRng,
  random: usize,
) -> Vec<Vec2<f32>> {
  let mut points: Vec<_> = (0..random)
    .map(|_| Vec2::new(rng.range(-0.1, 1.1) as f32, rng.range(-0.1, 1.1) as f32))
    .collect();
  let Some(view) = encoding.patch_view(patch) else {
    return points;
  };
  for s in view.segments {
    for e in [s.p0, s.p2] {
      points.push(e);
      points.push(Vec2::new(e.x - 1e-3, e.y));
      points.push(Vec2::new(e.x + 1e-3, e.y));
      points.push(Vec2::new(rng.unit() as f32, e.y));
    }
    points.push(Vec2::new(s.max_u, rng.unit() as f32));
  }
  if view.header.kind == TRIM_KIND_BAND_GRID {
    let nb = 1 << BandGridLayout::decode(view.data).band_log2;
    for b in 0..=nb {
      points.push(Vec2::new(rng.unit() as f32, b as f32 / nb as f32));
    }
  }
  points
}

/// `random_model` with the trim loops of `trim_scenes` on every second patch (cycling through
/// the scenes), for the tests of the trimmed tessellation and drawing.
pub fn trimmed_random_model(
  seed: u64,
  faces: usize,
  depth: (f64, f64),
) -> ParametricNormalizedModel {
  let mut model = crate::test_scenes::random_model(seed, faces, depth);
  let scenes = trim_scenes();
  for (i, patch) in model.patches.iter_mut().skip(1).step_by(2).enumerate() {
    patch.loops = scenes[i % scenes.len()].loops.clone();
  }
  model
}

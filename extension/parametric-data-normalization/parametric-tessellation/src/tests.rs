use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::verify::*;
use crate::*;

pub struct TestRng(pub u64);

impl TestRng {
  /// Uniform in `[0, 1)`.
  pub fn next(&mut self) -> f64 {
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    (self.0 >> 11) as f64 / (1u64 << 53) as f64
  }

  pub fn range(&mut self, min: f64, max: f64) -> f64 {
    min + (max - min) * self.next()
  }
}

/// A wavy patch over about `[-2, 2]²` in xy, centered at the origin.
pub fn random_surface(rng: &mut TestRng, rational: bool) -> RationalBezierSurface {
  let degrees = [(3, 3), (5, 5), (2, 3), (5, 3), (1, 4)];
  let (n, m) = degrees[(rng.next() * degrees.len() as f64) as usize];
  let amplitude = rng.range(0.1, 1.5);
  let mut cp = Vec::new();
  for j in 0..=m {
    for i in 0..=n {
      let x = 4. * i as f64 / n as f64 - 2. + rng.range(-0.2, 0.2);
      let y = 4. * j as f64 / m as f64 - 2. + rng.range(-0.2, 0.2);
      let z = amplitude * rng.range(-1., 1.);
      let w = if rational { rng.range(0.5, 1.5) } else { 1. };
      cp.push(Vec4::from_euclid(Vec3::new(x, y, z), w));
    }
  }
  RationalBezierSurface::new(n, m, cp)
}

fn transpose(s: &RationalBezierSurface) -> RationalBezierSurface {
  let cp = (0..=s.degree_u)
    .flat_map(|i| (0..=s.degree_v).map(move |j| s.control_point(i, j)))
    .collect();
  RationalBezierSurface::new(s.degree_v, s.degree_u, cp)
}

fn test_camera() -> Camera {
  Camera {
    view: Mat4::identity(),
    focal_length: 800.,
    viewport: Vec2::new(1600., 1200.),
    near: 0.1,
  }
}

fn item(surface: RationalBezierSurface, depth: f64, tilt: f64) -> TessellationItem {
  let to_camera = Mat4::translate((0., 0., -depth)) * Mat4::rotate_x(tilt);
  TessellationItem {
    instance: 0,
    patch: 0,
    surface: surface.transform(to_camera),
  }
}

#[test]
fn error_within_threshold() {
  let mut rng = TestRng(3);
  let config = TessellationConfig::default();
  for case in 0..40 {
    let surface = random_surface(&mut rng, case % 2 == 1);
    let items = [item(surface, rng.range(2.5, 30.), rng.range(-1.2, 1.2))];
    let mut t = Tessellator::new(&items, test_camera(), config);
    let leaves = t.run();
    let report = measure_error(&mut t, &leaves, 6);
    assert!(report.samples > 0);
    assert!(
      report.max_pixel_error <= config.pixel_error,
      "case {case}: {report:?} {:?}",
      t.stats
    );
  }
}

#[test]
fn near_plane_crossing_terminates() {
  let mut rng = TestRng(5);
  let items = [item(random_surface(&mut rng, true), 0.5, 1.)];
  let config = TessellationConfig {
    max_level: 6,
    ..Default::default()
  };
  let mut t = Tessellator::new(&items, test_camera(), config);
  let leaves = t.run();
  assert!(!leaves.is_empty());
  assert!(t.stats.forced_leaves > 0);
  let report = check_watertight(&mut t, &leaves);
  assert_eq!(report.mismatched_vertices, 0);
}

/// A patch split into four, one part with reversed u and one transposed, so shared boundaries
/// have bitwise equal control points in different orientations.
#[test]
fn watertight_across_patches() {
  let mut rng = TestRng(9);
  for case in 0..12 {
    let s = random_surface(&mut rng, case % 2 == 0);
    let parts = [
      s.sub_patch((0., 0.5), (0., 0.5)),
      s.sub_patch((1., 0.5), (0., 0.5)),
      transpose(&s.sub_patch((0., 0.5), (0.5, 1.))),
      s.sub_patch((0.5, 1.), (0.5, 1.)),
    ];
    let (depth, tilt) = (rng.range(3., 10.), rng.range(-1., 1.));
    let items: Vec<_> = parts.into_iter().map(|p| item(p, depth, tilt)).collect();
    let mut t = Tessellator::new(&items, test_camera(), TessellationConfig::default());
    let leaves = t.run();
    let report = check_watertight(&mut t, &leaves);
    assert_eq!(report.mismatched_vertices, 0, "case {case}: {report:?}");
    assert_eq!(report.uncovered_vertices, 0, "case {case}: {report:?}");
    assert_eq!(report.non_manifold_lines, 0, "case {case}: {report:?}");
    // The four outer sides are split in two lines each.
    assert_eq!(report.open_lines, 8, "case {case}: {report:?}");
    assert!(
      report.max_t_junction_relative < 1e-12,
      "case {case}: {report:?}"
    );
  }
}

//! Synthetic patches and models for the tests of this crate and of the GPU version
//! (`rendiation-parametric-rendering`).

use std::f64::consts::FRAC_PI_2;

use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::*;

pub mod trim;

/// A tiny deterministic random generator.
pub struct TestRng(pub u64);

impl TestRng {
  /// Uniform in `[0, 1)`.
  pub fn unit(&mut self) -> f64 {
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    (self.0 >> 11) as f64 / (1u64 << 53) as f64
  }

  pub fn range(&mut self, min: f64, max: f64) -> f64 {
    min + (max - min) * self.unit()
  }
}

/// A wavy patch over about `[-2, 2]²` in xy, centered at the origin.
pub fn random_surface(rng: &mut TestRng, rational: bool) -> RationalBezierSurface {
  let degrees = [(3, 3), (5, 5), (2, 3), (5, 3), (1, 4)];
  let (n, m) = degrees[(rng.unit() * degrees.len() as f64) as usize];
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

/// The same surface with u and v swapped.
pub fn transpose(s: &RationalBezierSurface) -> RationalBezierSurface {
  let cp = (0..=s.degree_u)
    .flat_map(|i| (0..=s.degree_v).map(move |j| s.control_point(i, j)))
    .collect();
  RationalBezierSurface::new(s.degree_v, s.degree_u, cp)
}

/// `s` split into four parts, one with reversed u and one transposed, so the shared boundaries
/// have bitwise equal control points in different orientations.
pub fn split_in_four(s: &RationalBezierSurface) -> [RationalBezierSurface; 4] {
  [
    s.sub_patch((0., 0.5), (0., 0.5)),
    s.sub_patch((1., 0.5), (0., 0.5)),
    transpose(&s.sub_patch((0., 0.5), (0.5, 1.))),
    s.sub_patch((0.5, 1.), (0.5, 1.)),
  ]
}

/// Identity view, 1600×1200 pixels, focal length 800.
pub fn test_camera() -> Camera {
  Camera {
    view: Mat4::identity(),
    focal_length: 800.,
    viewport: Vec2::new(1600., 1200.),
    near: 0.1,
  }
}

/// Placement in front of `test_camera`: rotated around x by `tilt`, at `depth`.
pub fn placement(depth: f64, tilt: f64) -> Mat4<f64> {
  Mat4::translate((0., 0., -depth)) * Mat4::rotate_x(tilt)
}

/// Builds a model face by face: every face is a set of untrimmed patches with instances.
#[derive(Default)]
pub struct TestModelBuilder {
  pub model: ParametricNormalizedModel,
}

impl TestModelBuilder {
  /// Adds a face made of `patches`, returns its id.
  pub fn face(&mut self, patches: impl IntoIterator<Item = RationalBezierSurface>) -> FaceId {
    let face = FaceId(self.model.faces.len() as u32);
    self.model.faces.push(FaceInfo {
      source_id: face.0 as u64,
    });
    for (index, surface) in patches.into_iter().enumerate() {
      self.model.patches.push(TrimmedBezierPatch {
        surface,
        loops: vec![boundary_loop()],
        flip_normal: index % 3 == 1,
        face,
        cell: (index, 0),
      });
    }
    face
  }

  /// Adds an instance of `face` (all its patches, which must be the last added ones or
  /// contiguous).
  pub fn instance(&mut self, face: FaceId, transform: Mat4<f64>, flip_normal: bool) {
    let indices: Vec<_> = (0..self.model.patches.len())
      .filter(|&i| self.model.patches[i].face == face)
      .collect();
    let range = indices[0]..indices[indices.len() - 1] + 1;
    assert_eq!(range.len(), indices.len());
    self.model.instances.push(FaceInstance {
      face,
      patches: range,
      transform,
      flip_normal,
    });
  }

  /// Marks patch `index` as trimmed (an extra empty hole loop, the tessellation ignores trim).
  pub fn mark_trimmed(&mut self, index: usize) {
    self.model.patches[index]
      .loops
      .push(TrimLoop { curves: Vec::new() });
  }
}

fn boundary_loop() -> TrimLoop {
  let corners = [
    Vec2::new(0., 0.),
    Vec2::new(1., 0.),
    Vec2::new(1., 1.),
    Vec2::new(0., 1.),
  ];
  TrimLoop {
    curves: (0..4)
      .map(|i| TrimCurve {
        curve: RationalBezierCurve2::line(corners[i], corners[(i + 1) % 4]),
        source: TrimCurveSource::PatchBoundary,
        exact: true,
      })
      .collect(),
  }
}

/// Random single patch faces (one instance each) and faces split in four (two instances, one
/// flipped), spread in front of `test_camera` at depths in `depth`.
pub fn random_model(seed: u64, faces: usize, depth: (f64, f64)) -> ParametricNormalizedModel {
  let mut rng = TestRng(seed);
  let mut builder = TestModelBuilder::default();
  for f in 0..faces {
    let s = random_surface(&mut rng, f % 2 == 1);
    let z = rng.range(depth.0, depth.1);
    let spread = z * 0.6;
    let offset = Vec3::new(
      rng.range(-spread, spread),
      rng.range(-spread, spread) * 0.7,
      0.,
    );
    let transform =
      |rng: &mut TestRng| Mat4::translate(offset) * placement(z, rng.range(-1.2, 1.2));
    if f % 3 == 2 {
      let face = builder.face(split_in_four(&s));
      let t = transform(&mut rng);
      builder.instance(face, t, false);
      let t = Mat4::translate((rng.range(-1., 1.), 0., 0.)) * transform(&mut rng);
      builder.instance(face, t * Mat4::rotate_z(FRAC_PI_2), true);
    } else {
      let face = builder.face([s]);
      let t = transform(&mut rng);
      builder.instance(face, t, f % 4 == 0);
    }
  }
  let trimmed = builder.model.patches.len() / 2;
  builder.mark_trimmed(trimmed);
  builder.model
}

/// The CLI `--camera fit:ZOOM` camera: looking at the model bounds from a fixed diagonal
/// direction, the distance divided by `zoom`, 1920×1080 with a 45° vertical field of view.
pub fn fit_camera(model: &ParametricNormalizedModel, zoom: f64) -> Camera {
  let fov_y = 45f64.to_radians();
  let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
  let mut max = min * -1.;
  for instance in &model.instances {
    for patch in &model.patches[instance.patches.clone()] {
      for p in &patch.surface.control_points {
        let p = (instance.transform * *p).to_euclid();
        min = min.min(p);
        max = max.max(p);
      }
    }
  }
  let center = (min + max) * 0.5;
  let radius = ((max - min).length() * 0.5).max(1e-9);
  let distance = radius / (fov_y * 0.5).sin() / zoom;
  let direction = Vec3::new(1., 0.8, 1.3).normalize();
  Camera::look_at(
    center + direction * distance,
    center,
    Vec3::new(0., 1., 0.),
    fov_y,
    Vec2::new(1920., 1080.),
    distance * 1e-3,
  )
}

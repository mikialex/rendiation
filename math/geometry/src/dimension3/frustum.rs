use crate::*;

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Facet)]
pub struct Frustum<T: Scalar = f32> {
  pub planes: [Plane<T>; 6],
}

impl Frustum<f64> {
  pub fn into_f32(self) -> Frustum<f32> {
    Frustum {
      planes: self.planes.map(|v| v.into_f32()),
    }
  }
}

impl<T: Scalar> Default for Frustum<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Scalar> Frustum<T> {
  pub fn new() -> Self {
    Self {
      planes: [Plane::new(Vec3::splat(T::one()).into_normalized(), T::one()); 6],
    }
  }

  /// the matrix must in opengl standard ndc
  pub fn new_from_matrix(m: Mat4<T>) -> Self {
    let mut f = Frustum::<T>::default();
    f.set_from_matrix(m);
    f
  }

  /// the matrix must in opengl standard ndc
  #[rustfmt::skip]
  pub fn set_from_matrix(&mut self, m: Mat4<T>) -> &Self {
    self.planes[0].set_components(m.a4 - m.a1, m.b4 - m.b1, m.c4 - m.c1, m.d4 - m.d1); // right
    self.planes[1].set_components(m.a4 + m.a1, m.b4 + m.b1, m.c4 + m.c1, m.d4 + m.d1); // left
    self.planes[2].set_components(m.a4 - m.a2, m.b4 - m.b2, m.c4 - m.c2, m.d4 - m.d2); // top
    self.planes[3].set_components(m.a4 + m.a2, m.b4 + m.b2, m.c4 + m.c2, m.d4 + m.d2); // bottom
    self.planes[4].set_components(m.a4 - m.a3, m.b4 - m.b3, m.c4 - m.c3, m.d4 - m.d3); // far
    self.planes[5].set_components(m.a4 + m.a3, m.b4 + m.b3, m.c4 + m.c3, m.d4 + m.d3); // near
    self
  }

  /// the matrix must in opengl standard ndc
  pub fn new_from_matrix_ndc(m: Mat4<T>, ndc: &[T; 6]) -> Self {
    let mut f = Frustum::<T>::default();
    f.set_from_matrix_ndc(m, ndc);
    f
  }

  /// the matrix must in opengl standard ndc
  #[rustfmt::skip]
  pub fn set_from_matrix_ndc(&mut self, m: Mat4<T>, ndc:&[T; 6]) -> &Self {
    self.planes[0].set_components(m.a4 * ndc[1] - m.a1, m.b4 * ndc[1] - m.b1, m.c4 * ndc[1] - m.c1, m.d4 * ndc[1] - m.d1);  // right
    self.planes[1].set_components(m.a1 - m.a4 * ndc[0], m.b1 - m.b4 * ndc[0],  m.c1 - m.c4 * ndc[0], m.d1 - m.d4 * ndc[0]);  // left
    self.planes[2].set_components(m.a4 * ndc[3] - m.a2, m.b4 * ndc[3] - m.b2, m.c4 * ndc[3] - m.c2, m.d4 * ndc[3] - m.d2);  // top
    self.planes[3].set_components(m.a2 - m.a4 * ndc[2], m.b2 - m.b4 * ndc[2],  m.c2 - m.c4 * ndc[2], m.d2 - m.d4 * ndc[2]);  // bottom
    self.planes[4].set_components(m.a4 * ndc[5] - m.a3, m.b4 * ndc[5] - m.b3, m.c4 * ndc[5] - m.c3, m.d4 * ndc[5] - m.d3);  // far
    self.planes[5].set_components(m.a3 - m.a4 * ndc[4], m.b3 - m.b4 * ndc[4],  m.c3 - m.c4 * ndc[4], m.d3 - m.d4 * ndc[4]);  // near
    self
  }
}

impl<T: Scalar> Frustum<T> {
  fn compute_vertices(&self) -> Vec<Vec3<T>> {
    let eps = T::epsilon();
    let mut vertices: Vec<Vec3<T>> = Vec::new();

    for i in 0..4 {
      for j in (i + 1)..5 {
        for k in (j + 1)..6 {
          if let Some(p) = Plane::intersect_three(
            &self.planes[i],
            &self.planes[j],
            &self.planes[k],
          ) {
            let valid = self
              .planes
              .iter()
              .all(|plane| plane.distance_to(&p) >= -eps);
            if valid {
              if !vertices.iter().any(|v: &Vec3<T>| (*v - p).length2() < eps) {
                vertices.push(p);
              }
            }
          }
        }
      }
    }

    assert!(
      vertices.len() >= 8,
      "Frustum is not a well-formed closed 6-face polyhedron: found {} vertices, expected at least 8",
      vertices.len()
    );
    vertices
  }

  /// Decompose the convex polyhedron into tetrahedra from an interior reference point.
  fn compute_volume_and_centroid(&self) -> (T, Vec3<T>) {
    let vertices = self.compute_vertices();
    let eps = T::epsilon();

    let n_verts = vertices.len();
    let ref_point = {
      let mut sum = Vec3::new(T::zero(), T::zero(), T::zero());
      for v in &vertices {
        sum = sum + *v;
      }
      sum / T::by_usize_div(n_verts, 1)
    };

    let mut total_volume = T::zero();
    let mut centroid_accum = Vec3::new(T::zero(), T::zero(), T::zero());

    for plane in &self.planes {
      let n = *plane.normal;

      let mut face_verts: Vec<Vec3<T>> = vertices
        .iter()
        .filter(|v| plane.distance_to(v).abs() < eps)
        .copied()
        .collect();

      let mut unique: Vec<Vec3<T>> = Vec::new();
      for v in &face_verts {
        if !unique.iter().any(|u: &Vec3<T>| (*u - *v).length2() < eps) {
          unique.push(*v);
        }
      }
      face_verts = unique;

      assert!(
        face_verts.len() >= 3,
        "Frustum face has fewer than 3 vertices"
      );

      let v0 = face_verts[0];
      let u = (face_verts[1] - v0).normalize();
      let w = n.cross(u);

      face_verts[1..].sort_by(|a, b| {
        let da = *a - v0;
        let db = *b - v0;
        let angle_a = w.dot(da).atan2(u.dot(da));
        let angle_b = w.dot(db).atan2(u.dot(db));
        angle_a
          .partial_cmp(&angle_b)
          .unwrap_or(std::cmp::Ordering::Equal)
      });

      for tri_idx in 1..(face_verts.len() - 1) {
        let a = face_verts[0];
        let b = face_verts[tri_idx];
        let c = face_verts[tri_idx + 1];

        let triple = (a - ref_point).dot((b - ref_point).cross(c - ref_point));
        let vol_tet = triple.abs() / (T::two() * T::three());

        total_volume = total_volume + vol_tet;

        let tet_centroid = (ref_point + a + b + c) * (T::half() * T::half());
        centroid_accum = centroid_accum + tet_centroid * vol_tet;
      }
    }

    let centroid = if total_volume > T::zero() {
      centroid_accum / total_volume
    } else {
      Vec3::new(T::zero(), T::zero(), T::zero())
    };

    (total_volume, centroid)
  }
}

impl<T: Scalar> SpaceEntity<T, 3> for Frustum<T> {
  type Matrix = Mat4<T>;

  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.planes.iter_mut().for_each(|p| {
      p.apply_matrix(mat);
    });
    self
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 3> for Frustum<T> {
  fn measure(&self) -> T {
    self.compute_volume_and_centroid().0
  }
}

impl<T: Scalar> SolidEntity<T, 3> for Frustum<T> {
  type Center = Vec3<T>;

  fn centroid(&self) -> Self::Center {
    self.compute_volume_and_centroid().1
  }
}

impl<T: Scalar> ContainAble<T, Vec3<T>, 3> for Frustum<T> {
  fn contains(&self, target: &Vec3<T>) -> bool {
    for p in &self.planes {
      let distance = p.distance_to(target);
      if distance < T::zero() {
        return false;
      }
    }

    true
  }
}

impl<T: Scalar> IntersectAble<Frustum<T>, bool, ()> for Sphere<T> {
  fn intersect(&self, other: &Frustum<T>, p: &()) -> bool {
    IntersectAble::<Sphere<T>, bool, ()>::intersect(other, self, p)
  }
}
impl<T: Scalar> IntersectAble<Sphere<T>, bool> for Frustum<T> {
  fn intersect(&self, sphere: &Sphere<T>, _: &()) -> bool {
    let neg_radius = -sphere.radius;

    for p in &self.planes {
      let distance = p.distance_to(&sphere.center);
      if distance < neg_radius {
        return false;
      }
    }

    true
  }
}

impl<T: Scalar> IntersectAble<Frustum<T>, bool, ()> for Box3<T> {
  fn intersect(&self, other: &Frustum<T>, p: &()) -> bool {
    IntersectAble::<Box3<T>, bool, ()>::intersect(other, self, p)
  }
}
impl<T: Scalar> IntersectAble<Box3<T>, bool> for Frustum<T> {
  fn intersect(&self, box3: &Box3<T>, _: &()) -> bool {
    for p in &self.planes {
      if p.distance_to(&box3.max_corner(*p.normal)) < T::zero() {
        return false;
      }
    }

    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_plane<T: Scalar>(nx: T, ny: T, nz: T, d: T) -> Plane<T> {
    Plane::new(Vec3::new(nx, ny, nz).into_normalized(), d)
  }

  /// Cube [-1, 1]^3: volume = 8, centroid = (0, 0, 0)
  #[test]
  fn cube_centered() {
    let frustum = Frustum {
      planes: [
        make_plane(-1., 0., 0., 1.), // right face x=1, inward normal left
        make_plane(1., 0., 0., 1.),  // left face x=-1, inward normal right
        make_plane(0., -1., 0., 1.), // top face y=1
        make_plane(0., 1., 0., 1.),  // bottom face y=-1
        make_plane(0., 0., -1., 1.), // far face z=1
        make_plane(0., 0., 1., 1.),  // near face z=-1
      ],
    };

    let vol = frustum.measure();
    let c = frustum.centroid();

    assert!((vol - 8.0).abs() < 1e-5, "volume should be 8, got {}", vol);
    assert!((c.x - 0.0).abs() < 1e-5, "centroid x should be 0, got {}", c.x);
    assert!((c.y - 0.0).abs() < 1e-5, "centroid y should be 0, got {}", c.y);
    assert!((c.z - 0.0).abs() < 1e-5, "centroid z should be 0, got {}", c.z);
  }

  /// Cube [0, 2]^3: volume = 8, centroid = (1, 1, 1)
  #[test]
  fn cube_offset() {
    let frustum = Frustum {
      planes: [
        make_plane(1., 0., 0., 0.),  // x=0
        make_plane(-1., 0., 0., 2.), // x=2
        make_plane(0., 1., 0., 0.),  // y=0
        make_plane(0., -1., 0., 2.), // y=2
        make_plane(0., 0., 1., 0.),  // z=0
        make_plane(0., 0., -1., 2.), // z=2
      ],
    };

    let vol = frustum.measure();
    let c = frustum.centroid();

    assert!((vol - 8.0).abs() < 1e-5, "volume should be 8, got {}", vol);
    assert!((c.x - 1.0).abs() < 1e-5, "centroid x should be 1, got {}", c.x);
    assert!((c.y - 1.0).abs() < 1e-5, "centroid y should be 1, got {}", c.y);
    assert!((c.z - 1.0).abs() < 1e-5, "centroid z should be 1, got {}", c.z);
  }
}

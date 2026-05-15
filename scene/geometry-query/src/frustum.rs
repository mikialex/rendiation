use crate::*;

pub struct FrustumIntersectionTestHelper<T: Scalar> {
  corners: [Vec3<T>; 8],
  edges: Vec<Vec3<T>>,
}

impl<T: Scalar> FrustumIntersectionTestHelper<T> {
  pub fn new(f: &Frustum<T>) -> Option<Self> {
    let corners = compute_frustum_corners(f)?;
    // side edges first: more likely to be separating in 3D rendering;
    // deduplication keeps the first occurrence when edges are parallel
    let all_edges = [
      corners[4] - corners[0],
      corners[5] - corners[1],
      corners[6] - corners[2],
      corners[7] - corners[3],
      corners[1] - corners[0],
      corners[2] - corners[0],
      corners[3] - corners[1],
      corners[3] - corners[2],
      corners[5] - corners[4],
      corners[6] - corners[4],
      corners[7] - corners[5],
      corners[7] - corners[6],
    ];

    let eps = T::epsilon();
    let mut edges: Vec<Vec3<T>> = Vec::with_capacity(12);

    for &e in &all_edges {
      let e_len_sq = e.x * e.x + e.y * e.y + e.z * e.z;
      if e_len_sq == T::zero() {
        continue;
      }
      let is_parallel = edges.iter().any(|&k| {
        let cross = e.cross(k);
        let cross_len_sq = cross.x * cross.x + cross.y * cross.y + cross.z * cross.z;
        let k_len_sq = k.x * k.x + k.y * k.y + k.z * k.z;
        cross_len_sq <= eps * eps * e_len_sq * k_len_sq
      });
      if !is_parallel {
        edges.push(e);
      }
    }

    Some(Self { corners, edges })
  }
}

/// Returns true if the AABB intersects the frustum.
///
/// `helper` is a precomputed cache from [`FrustumIntersectionTestHelper::new`].
/// Reuse it across multiple AABB tests against the same frustum to avoid
/// recomputing corners and edge directions.
///
/// When `helper` is `None` (degenerate frustum or precise test disabled),
/// falls back to a conservative p-vertex test: the AABB is considered
/// intersecting as long as its farthest corner along each plane normal is
/// in front of that plane. No false negatives, but possible false positives.
pub fn frustum_intersect_aabb<T: Scalar>(
  helper: Option<&FrustumIntersectionTestHelper<T>>,
  f: &Frustum<T>,
  box3: &Box3<T>,
) -> bool {
  if box3.is_empty() {
    return false;
  }

  let helper = match helper {
    Some(h) => h,
    None => {
      // degenerate frustum or precise test disabled: fall back to conservative
      // p-vertex test — no false negatives, possible false positives
      for p in &f.planes {
        if p.distance_to(&box3.max_corner(*p.normal)) < T::zero() {
          return false;
        }
      }
      return true;
    }
  };

  // 3 AABB face normals (coordinate axes)
  for axis_idx in 0..3 {
    let (min_f, max_f) = project_frustum_on_axis(&helper.corners, axis_idx);
    let (min_b, max_b) = match axis_idx {
      0 => (box3.min.x, box3.max.x),
      1 => (box3.min.y, box3.max.y),
      _ => (box3.min.z, box3.max.z),
    };
    if max_f < min_b || max_b < min_f {
      return false;
    }
  }

  // 6 frustum plane normals (p-vertex test)
  for p in &f.planes {
    if p.distance_to(&box3.max_corner(*p.normal)) < T::zero() {
      return false;
    }
  }

  let aabb_axes = [
    Vec3::new(T::one(), T::zero(), T::zero()),
    Vec3::new(T::zero(), T::one(), T::zero()),
    Vec3::new(T::zero(), T::zero(), T::one()),
  ];

  for e_f in &helper.edges {
    for e_a in &aabb_axes {
      let axis = e_f.cross(*e_a);
      if axis.x == T::zero() && axis.y == T::zero() && axis.z == T::zero() {
        continue;
      }
      let (min_f, max_f) = project_points_on_axis(&helper.corners, axis);
      let (min_b, max_b) = project_points_on_axis(&aabb_corners(box3), axis);
      if max_f < min_b || max_b < min_f {
        return false;
      }
    }
  }

  true
}

/// helper： see `frustum_intersect_aabb`
pub fn frustum_intersect_line_segment<T: Scalar>(
  helper: Option<&FrustumIntersectionTestHelper<T>>,
  f: &Frustum<T>,
  a: Vec3<T>,
  b: Vec3<T>,
) -> bool {
  let helper = match helper {
    Some(h) => h,
    None => return f.contains(&a) || f.contains(&b),
  };

  // 6 frustum plane normals: both endpoints behind any plane → outside
  for p in &f.planes {
    if p.distance_to(&a) < T::zero() && p.distance_to(&b) < T::zero() {
      return false;
    }
  }

  let seg_dir = b - a;
  let seg_len_sq = seg_dir.x * seg_dir.x + seg_dir.y * seg_dir.y + seg_dir.z * seg_dir.z;
  if seg_len_sq == T::zero() {
    return true;
  }

  for e_f in &helper.edges {
    let axis = e_f.cross(seg_dir);
    if axis.x == T::zero() && axis.y == T::zero() && axis.z == T::zero() {
      continue;
    }
    let (min_f, max_f) = project_points_on_axis(&helper.corners, axis);
    let da = a.dot(axis);
    let db = b.dot(axis);
    let (min_s, max_s) = if da < db { (da, db) } else { (db, da) };
    if max_f < min_s || max_s < min_f {
      return false;
    }
  }

  true
}

/// helper： see `frustum_intersect_aabb`
pub fn frustum_intersect_triangle<T: Scalar>(
  helper: Option<&FrustumIntersectionTestHelper<T>>,
  f: &Frustum<T>,
  a: Vec3<T>,
  b: Vec3<T>,
  c: Vec3<T>,
) -> bool {
  let helper = match helper {
    Some(h) => h,
    None => return f.contains(&a) || f.contains(&b) || f.contains(&c),
  };

  // 6 frustum plane normals: all 3 vertices behind any plane → outside
  for p in &f.planes {
    if p.distance_to(&a) < T::zero()
      && p.distance_to(&b) < T::zero()
      && p.distance_to(&c) < T::zero()
    {
      return false;
    }
  }

  // triangle face normal
  let ab = b - a;
  let ac = c - a;
  let tri_normal = ab.cross(ac);
  let tri_normal_len_sq =
    tri_normal.x * tri_normal.x + tri_normal.y * tri_normal.y + tri_normal.z * tri_normal.z;
  if tri_normal_len_sq != T::zero() {
    let (min_f, max_f) = project_points_on_axis(&helper.corners, tri_normal);
    let tri_val = a.dot(tri_normal);
    if max_f < tri_val || tri_val < min_f {
      return false;
    }
  }

  // cross product axes: frustum edges × triangle edges
  let tri_edges = [ab, ac, c - b];

  for e_f in &helper.edges {
    for &e_t in &tri_edges {
      let e_t_len_sq = e_t.x * e_t.x + e_t.y * e_t.y + e_t.z * e_t.z;
      if e_t_len_sq == T::zero() {
        continue;
      }
      let axis = e_f.cross(e_t);
      if axis.x == T::zero() && axis.y == T::zero() && axis.z == T::zero() {
        continue;
      }
      let (min_f, max_f) = project_points_on_axis(&helper.corners, axis);
      let da = a.dot(axis);
      let db = b.dot(axis);
      let dc = c.dot(axis);
      let (min_t, max_t) = {
        let mut min = da;
        let mut max = da;
        if db < min {
          min = db;
        }
        if db > max {
          max = db;
        }
        if dc < min {
          min = dc;
        }
        if dc > max {
          max = dc;
        }
        (min, max)
      };
      if max_f < min_t || max_t < min_f {
        return false;
      }
    }
  }

  true
}

fn aabb_corners<T: Scalar>(box3: &Box3<T>) -> [Vec3<T>; 8] {
  [
    Vec3::new(box3.min.x, box3.min.y, box3.min.z),
    Vec3::new(box3.min.x, box3.min.y, box3.max.z),
    Vec3::new(box3.min.x, box3.max.y, box3.min.z),
    Vec3::new(box3.min.x, box3.max.y, box3.max.z),
    Vec3::new(box3.max.x, box3.min.y, box3.min.z),
    Vec3::new(box3.max.x, box3.min.y, box3.max.z),
    Vec3::new(box3.max.x, box3.max.y, box3.min.z),
    Vec3::new(box3.max.x, box3.max.y, box3.max.z),
  ]
}

fn project_points_on_axis<T: Scalar>(points: &[Vec3<T>; 8], axis: Vec3<T>) -> (T, T) {
  let mut min = points[0].dot(axis);
  let mut max = min;
  for p in &points[1..] {
    let v = p.dot(axis);
    if v < min {
      min = v;
    }
    if v > max {
      max = v;
    }
  }
  (min, max)
}

fn project_frustum_on_axis<T: Scalar>(corners: &[Vec3<T>; 8], axis_idx: usize) -> (T, T) {
  let get = |v: &Vec3<T>| match axis_idx {
    0 => v.x,
    1 => v.y,
    _ => v.z,
  };
  let mut min = get(&corners[0]);
  let mut max = min;
  for c in &corners[1..] {
    let v = get(c);
    if v < min {
      min = v;
    }
    if v > max {
      max = v;
    }
  }
  (min, max)
}

fn compute_frustum_corners<T: Scalar>(f: &Frustum<T>) -> Option<[Vec3<T>; 8]> {
  let p = &f.planes;
  // planes: 0=right, 1=left, 2=top, 3=bottom, 4=far, 5=near
  Some([
    Plane::intersect_three(&p[5], &p[1], &p[2])?, // near left top
    Plane::intersect_three(&p[5], &p[0], &p[2])?, // near right top
    Plane::intersect_three(&p[5], &p[1], &p[3])?, // near left bottom
    Plane::intersect_three(&p[5], &p[0], &p[3])?, // near right bottom
    Plane::intersect_three(&p[4], &p[1], &p[2])?, // far left top
    Plane::intersect_three(&p[4], &p[0], &p[2])?, // far right top
    Plane::intersect_three(&p[4], &p[1], &p[3])?, // far left bottom
    Plane::intersect_three(&p[4], &p[0], &p[3])?, // far right bottom
  ])
}

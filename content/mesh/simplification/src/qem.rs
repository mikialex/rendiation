use std::ops::{Add, AddAssign};

use crate::*;

/// ```txt
/// [a00, a10, a20, b0]
/// [   , a11, a21, b1]
/// [   ,    , a22, b2]
/// [   ,    ,    , c ]
/// ```
// a00*x^2 + a11*y^2 + a22*z^2 + 2*(a10*xy + a20*xz + a21*yz) + b0*x + b1*y + b2*z + c
#[derive(Clone, Copy, Default)]
pub struct Quadric {
  a00: f32,
  a11: f32,
  a22: f32,
  a10: f32,
  a20: f32,
  a21: f32,
  b0: f32,
  b1: f32,
  b2: f32,
  c: f32,
  /// weight, linearly apply on all matrix element
  w: f32,
}

impl Add for Quadric {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Self {
      a00: self.a00 + other.a00,
      a11: self.a11 + other.a11,
      a22: self.a22 + other.a22,
      a10: self.a10 + other.a10,
      a20: self.a20 + other.a20,
      a21: self.a21 + other.a21,
      b0: self.b0 + other.b0,
      b1: self.b1 + other.b1,
      b2: self.b2 + other.b2,
      c: self.c + other.c,
      w: self.w + other.w,
    }
  }
}

impl AddAssign for Quadric {
  fn add_assign(&mut self, other: Self) {
    self.a00 += other.a00;
    self.a11 += other.a11;
    self.a22 += other.a22;
    self.a10 += other.a10;
    self.a20 += other.a20;
    self.a21 += other.a21;
    self.b0 += other.b0;
    self.b1 += other.b1;
    self.b2 += other.b2;
    self.c += other.c;
    self.w += other.w;
  }
}

#[inline(always)]
pub(crate) fn inverse_or_zeroed(value: f32) -> f32 {
  if value != 0.0 {
    1.0 / value
  } else {
    0.0
  }
}

impl Quadric {
  // /// we could also using Quadric to express the point to point squared distance.
  // /// just encode (x - X) ^ 2 + (y - Y)^2 + (z - Z)^2 into the quadric
  // pub fn from_point(x: f32, y: f32, z: f32, w: f32) -> Self {
  //   Self {
  //     a00: w,
  //     a11: w,
  //     a22: w,
  //     a10: 0.0,
  //     a20: 0.0,
  //     a21: 0.0,
  //     b0: -2.0 * x * w,
  //     b1: -2.0 * y * w,
  //     b2: -2.0 * z * w,
  //     c: (x * x + y * y + z * z) * w,
  //     w,
  //   }
  // }

  pub fn from_plane(a: f32, b: f32, c: f32, d: f32, w: f32) -> Self {
    let aw = a * w;
    let bw = b * w;
    let cw = c * w;
    let dw = d * w;

    Self {
      a00: a * aw,
      a11: b * bw,
      a22: c * cw,
      a10: a * bw,
      a20: a * cw,
      a21: b * cw,
      b0: a * dw,
      b1: b * dw,
      b2: c * dw,
      c: d * dw,
      w,
    }
  }

  pub fn from_triangle(p0: Vec3<f32>, p1: Vec3<f32>, p2: Vec3<f32>, weight: f32) -> Self {
    let p10 = p1 - p0;
    let p20 = p2 - p0;

    let mut normal = p10.cross(p20);
    let area = normal.normalize_self();

    let distance = normal.x * p0.x + normal.y * p0.y + normal.z * p0.z;

    // we use sqrtf(area) so that the error is scaled linearly; this tends to improve silhouettes
    Self::from_plane(
      normal.x,
      normal.y,
      normal.z,
      -distance,
      area.sqrt() * weight,
    )
  }

  /// the actually plane is passing p0-p1, with normal that point to p2
  pub fn from_triangle_edge(p0: Vec3<f32>, p1: Vec3<f32>, p2: Vec3<f32>, weight: f32) -> Self {
    let mut p10 = p1 - p0;
    let length = p10.normalize_self();

    // p20p = length of projection of p2-p0 onto normalize(p1 - p0)
    let p20 = p2 - p0;
    let p20p = p20.dot(p10);

    // normal = altitude of triangle from point p2 onto edge p1-p0
    let normal = (p20 - p10 * p20p).normalize();

    let distance = normal.dot(p0);

    // note: the weight is scaled linearly with edge length; this has to match the triangle weight
    Self::from_plane(normal.x, normal.y, normal.z, -distance, length * weight)
  }

  pub fn error(&self, v: &Vec3<f32>) -> f32 {
    let mut rx = self.b0;
    let mut ry = self.b1;
    let mut rz = self.b2;

    rx += self.a10 * v.y;
    ry += self.a21 * v.z;
    rz += self.a20 * v.x;

    rx *= 2.0;
    ry *= 2.0;
    rz *= 2.0;

    rx += self.a00 * v.x;
    ry += self.a11 * v.y;
    rz += self.a22 * v.z;

    let mut r = self.c;
    r += rx * v.x;
    r += ry * v.y;
    r += rz * v.z;

    let s = inverse_or_zeroed(self.w);

    r.abs() * s
  }
}

pub fn fill_quadrics(
  indices: &[u32],
  vertex_positions: &[Vec3<f32>],
  remap: &[u32],
  vertex_kind: &[VertexKind],
  BorderLoops {
    openout: loop_,
    openinc: loopback,
  }: &BorderLoops,
) -> Vec<Quadric> {
  let mut vertex_quadrics = vec![Quadric::default(); vertex_positions.len()];

  // for each triangle
  for i in indices.array_chunks::<3>().copied() {
    let [i0, i1, i2] = i;
    let (i0, i1, i2) = (i0 as usize, i1 as usize, i2 as usize);

    let q = Quadric::from_triangle(
      vertex_positions[i0],
      vertex_positions[i1],
      vertex_positions[i2],
      1.0,
    );

    vertex_quadrics[remap[i0] as usize] += q;
    vertex_quadrics[remap[i1] as usize] += q;
    vertex_quadrics[remap[i2] as usize] += q;

    // for each edge
    const NEXT: [usize; 3] = [1, 2, 0];
    for e in 0..3 {
      let i0 = i[e] as usize;
      let i1 = i[NEXT[e]] as usize;

      let k0 = vertex_kind[i0];
      let k1 = vertex_kind[i1];

      // check that either i0 or i1 are border/seam and are on the same edge loop
      // note that we need to add the error even for edged that connect e.g. border & locked
      // if we don't do that, the adjacent border->border edge won't have correct errors for corners
      if k0 != VertexKind::Border
        && k0 != VertexKind::SimpleSeam
        && k1 != VertexKind::Border
        && k1 != VertexKind::SimpleSeam
      {
        continue;
      }

      if (k0 == VertexKind::Border || k0 == VertexKind::SimpleSeam) && loop_[i0] != i1 as u32 {
        continue;
      }

      if (k1 == VertexKind::Border || k1 == VertexKind::SimpleSeam) && loopback[i1] != i0 as u32 {
        continue;
      }

      // seam edges should occur twice (i0->i1 and i1->i0) - skip redundant edges
      if VertexKind::has_opposite(k0, k1) && remap[i1] > remap[i0] {
        continue;
      }

      let i2 = i[NEXT[NEXT[e]]] as usize;

      // we try hard to maintain border edge geometry; seam edges can move more freely
      // due to topological restrictions on collapses, seam quadrics slightly improves collapse
      // structure but aren't critical
      const EDGE_WEIGHT_SEAM: f32 = 1.0;
      const EDGE_WEIGHT_BORDER: f32 = 10.0;

      let edge_weight = if k0 == VertexKind::Border || k1 == VertexKind::Border {
        EDGE_WEIGHT_BORDER
      } else {
        EDGE_WEIGHT_SEAM
      };

      let q = Quadric::from_triangle_edge(
        vertex_positions[i0],
        vertex_positions[i1],
        vertex_positions[i2],
        edge_weight,
      );

      vertex_quadrics[remap[i0] as usize] += q;
      vertex_quadrics[remap[i1] as usize] += q;
    }
  }

  vertex_quadrics
}

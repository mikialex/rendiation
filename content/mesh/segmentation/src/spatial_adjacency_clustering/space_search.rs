use space_algorithm::bvh::{FlattenBVH, SAH};

use crate::*;

pub trait SpaceSearchAcceleration<V: Positioned<Position = Vec3<f32>>> {
  fn build(indices: &[u32], vertices: &[V]) -> Self;
  fn search_nearest(
    &self,
    position: Vec3<f32>,
    should_skip: impl Fn(u32) -> bool,
    indices: &[u32],
    vertices: &[V],
  ) -> u32;
}

struct BVHSpaceSearchAcceleration {
  bvh: FlattenBVH<Box3<f32>>,
}

impl<V> SpaceSearchAcceleration<V> for BVHSpaceSearchAcceleration
where
  V: Positioned<Position = Vec3<f32>>,
{
  fn build<'b>(indices: &'b [u32], vertices: &'b [V]) -> Self {
    let bvh = FlattenBVH::new(
      indices.array_chunks::<3>().copied().map(|[a, b, c]| {
        let va = vertices[a as usize].position();
        let vb = vertices[b as usize].position();
        let vc = vertices[c as usize].position();
        Triangle::new(va, vb, vc).to_bounding()
      }),
      &mut SAH::new(4),
      &Default::default(),
    );
    Self { bvh }
  }

  fn search_nearest(
    &self,
    position: Vec3<f32>,
    should_skip: impl Fn(u32) -> bool,
    indices: &[u32],
    vertices: &[V],
  ) -> u32 {
    let mut result = !0;
    let minimal = std::cell::Cell::new(f32::MAX); // todo improve
    self.bvh.traverse(
      |node| {
        if node.bounding.contains(&position) {
          return true;
        }
        node.bounding.nearest_point(position).distance2(position) > minimal.get()
      },
      |node| {
        for tri in node.primitive_range.clone() {
          if should_skip(tri as u32) {
            continue;
          }
          // we only check first vertex;
          let v = vertices[indices[tri * 3] as usize].position();
          let distance = v.distance2(position);
          if distance < minimal.get() {
            minimal.set(distance);
            result = tri;
          }
        }
        true
      },
    );
    result as u32
  }
}

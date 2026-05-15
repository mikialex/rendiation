use rendiation_geometry::Box3;
use rendiation_geometry::Ray3;

use super::{Bvh, BvhNode};

impl Bvh {
  /// Returns an iterator over all leaves whose AABBs intersect the given AABB.
  pub fn intersect_aabb<'a>(&'a self, aabb: &'a Box3<f32>) -> impl Iterator<Item = u32> + 'a {
    self.leaves(|node: &BvhNode| node.aabb().intersects(aabb))
  }

  /// Casts a ray on this BVH using the provided leaf ray-cast function.
  ///
  /// The `primitive_check` delegates the ray-casting task to an external function that
  /// is assumed to map a leaf index to an actual geometry to cast a ray on. The `f32` argument
  /// given to that closure is the distance to the closest ray hit found so far (or is equal to
  /// `max_time_of_impact` if no projection was found so far).
  pub fn cast_ray(
    &self,
    ray: &Ray3<f32>,
    max_time_of_impact: f32,
    primitive_check: impl Fn(u32, f32) -> Option<f32>,
  ) -> Option<(u32, f32)> {
    self.find_best(
      max_time_of_impact,
      |node: &BvhNode, best_so_far| node.cast_ray(ray, best_so_far),
      primitive_check,
    )
  }
}

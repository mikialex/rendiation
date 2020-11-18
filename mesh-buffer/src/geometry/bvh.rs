use super::{AnyGeometry, AnyGeometryRefContainer, MeshBufferIntersectConfig};
use rendiation_math_entity::*;
use space_indexer::{bvh::*, utils::TreeBuildOption};

impl<'a, G> AnyGeometryRefContainer<'a, G>
where
  G: AnyGeometry,
  G::Primitive: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectConfig>,
{
  pub fn build_bvh<B, S>(&self, strategy: &mut S, option: &TreeBuildOption) -> FlattenBVH<B>
  where
    B: BVHBounding,
    S: BVHBuildStrategy<B>,
    B: From<G::Primitive>,
  {
    FlattenBVH::new(self.primitive_iter().map(|p| B::from(p)), strategy, option)
  }

  pub fn intersect_list_bvh<B>(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> IntersectionList3D
  where
    B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  {
    let mut result = IntersectionList3D::new();
    bvh.traverse(
      |branch| branch.bounding.intersect(&ray, &()),
      |leaf| {
        leaf
          .iter_primitive(bvh)
          .map(|&i| self.geometry.primitive_at(i))
          .for_each(|p| result.push_nearest(p.intersect(&ray, conf)))
      },
    );
    result
  }
}

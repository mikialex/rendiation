use super::*;
use rendiation_math_entity::*;
use space_indexer::bvh::*;

impl<'a, V, T, G> FlattenBVHBuildSource<Box3> for AbstractGeometryRef<'a, G>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
{
  type Iter;
  fn iter_primitive_bounding(&self) -> Self::Iter {
    self.primitive_iter().map(|(i, p)| {});
    todo!()
  }
}

pub trait GeometryBVH: AbstractGeometry {
  fn gen_bvh<S: BVHBuildStrategy<Box3>>(
    &self,
    conf: &BVHOption,
    strategy: S,
  ) -> FlattenBVH<Box3, S> {
    todo!()
  }
}

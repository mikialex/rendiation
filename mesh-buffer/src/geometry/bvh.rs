use super::primitive::*;
use super::AbstractGeometry;
use rendiation_math_entity::*;
use space_indexer::bvh::*;

// struct GeometryWithBVH<G: AbstractGeometry> {
//   geometry: G,
// }

pub trait FlattenBVHBuildSourceC<B: BVHBounding> {
  fn get_items_count(&self) -> usize;
  fn get_items_bounding_box(&self, item_index: usize) -> B;
}

impl<V, T, G> FlattenBVHBuildSourceC<Box3> for G
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
{
  fn get_items_count(&self) -> usize {
    self.primitive_iter().len()
  }
  fn get_items_bounding_box(&self, item_index: usize) -> Box3 {
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

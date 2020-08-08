use super::AbstractGeometry;
use rendiation_math_entity::*;
use space_indexer::bvh::*;

pub trait GeometryBVH: AbstractGeometry {
  fn gen_bvh(&self, conf: &BVHOption, strategy: impl BVHBuildStrategy<Box3>) -> FlattenBVH<Box3> {
    todo!()
  }
}

use super::AbstractGeometry;
use space_indexer::bvh::*;
use rendiation_math_entity::Box3;

pub struct BVHAcclerationed<G: AbstractGeometry, >{
    geometry: G,
    bvh: FlattenBVH<Box3, BalanceTree>
}
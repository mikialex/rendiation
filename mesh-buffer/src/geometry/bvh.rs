// use super::*;
// use rendiation_math_entity::*;
// use space_indexer::bvh::*;

// impl<'a, V, B, P, T, G> FlattenBVHBuildSource<B> for AbstractGeometryRef<'a, G>
// where
//   V: Positioned3D,
//   B: BVHBounding,
//   P: Into<B> + PrimitiveData<V>,
//   T: PrimitiveTopology<V, Primitive = P>,
//   G: AbstractGeometry<Vertex = V, Topology = T>,
// {
//   type Iter = std::iter::Map;
//   fn iter_primitive_bounding(&self) -> Self::Iter {
//     self.primitive_iter().map(|(i, p)| i.into())
//   }
// }

// pub trait GeometryBVH: AbstractGeometry {
//   fn gen_bvh<S: BVHBuildStrategy<Box3>>(
//     &self,
//     conf: &BVHOption,
//     strategy: S,
//   ) -> FlattenBVH<Box3, S> {
//     todo!()
//   }
// }

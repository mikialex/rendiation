// use super::*;
// use rendiation_math_entity::*;
// use space_indexer::bvh::*;
// use std::iter::Map;

// impl<'a, V, B, P, T, G, I> BVHSourceIterator<B> for AbstractPrimitiveIter<'a, G>
// where
//   V: Positioned3D,
//   B: BVHBounding,
//   P: Into<B> + PrimitiveData<V>,
//   T: PrimitiveTopology<V, Primitive = P>,
//   G: AbstractGeometry<Vertex = V, Topology = T>,
//   I: ExactSizeIterator<Item = T::Primitive>,
//   for<'b> AbstractPrimitiveIter<'b, G>: IntoIterator<Item = T::Primitive, IntoIter = I>,
// {
//   type IntoIter = Map<I, fn(T::Primitive) -> B>;
//   fn into_s_iter(self) -> Self::IntoIter {
//     self.into_iter().map(mapper)
//   }

//   // type Iter = Map<I, fn(T::Primitive) -> B>;
//   // fn iter_primitive_bounding(&self) -> Self::Iter {
//   //   self.primitive_iter().into_iter().map(mapper)
//   // }
// }

// fn mapper<P: Into<B>, B>(p: P) -> B {
//   p.into()
// }

#[test]
fn test() {
  use super::*;
  use crate::tessellation::{sphere::SphereGeometryParameter, IndexedBufferTessellator};
  use rendiation_math_entity::*;
  use space_indexer::bvh::*;
  // use std::iter::Map;

  let geometry: IndexedGeometry = SphereGeometryParameter::default().create_mesh(&()).into();

  let ray = Ray3::new((0., 0., 0.).into(), (0., 1., 0.).into());
  let re: NearestPoint3D = ray.intersect(
    &geometry.wrap(),
    &MeshBufferIntersectionConfig {
      line_precision: LineRayIntersectionLocalTolerance(1.),
    },
  );

  let bvh: FlattenBVH<Box3, _> = FlattenBVH::new(
    &mut geometry.primitive_iter().into_iter().map(|p| todo!()),
    BalanceTree,
    BVHOption::default(),
  );
}

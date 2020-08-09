use super::*;
use rendiation_math_entity::*;
use space_indexer::bvh::*;
use std::iter::Map;

impl<'a, V, B, P, T, G, I> FlattenBVHBuildSource<B> for AbstractGeometryRef<'a, G>
where
  V: Positioned3D,
  B: BVHBounding,
  P: Into<B> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  I: ExactSizeIterator<Item = T::Primitive>,
  for<'b> AbstractPrimitiveIter<'b, G>: IntoIterator<Item = T::Primitive, IntoIter = I>,
{
  type Iter = Map<I, fn(T::Primitive) -> B>;
  fn iter_primitive_bounding(&self) -> Self::Iter {
    self.primitive_iter().into_iter().map(mapper)
  }
}

fn mapper<P: Into<B>, B>(p: P) -> B {
  p.into()
}

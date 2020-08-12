use super::{
  AbstractGeometry, AbstractPrimitiveIter, IntoExactSizeIterator, MeshBufferIntersectionConfig,
  PrimitiveData, PrimitiveTopology,
};
use rendiation_math_entity::*;
use space_indexer::bvh::*;

pub struct BVHAcceleratedGeometry<G, B>
where
  G: AbstractGeometry,
  B: BVHBounding,
{
  geometry: G,
  bvh: Option<FlattenBVH<B>>,
}

impl<V, P, T, G, B>
  IntersectAble<BVHAcceleratedGeometry<G, B>, IntersectionList3D, MeshBufferIntersectionConfig>
  for Ray3
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  B: BVHBounding + IntersectAble<Ray3, bool, ()> + From<T::Primitive>,
  for<'a> AbstractPrimitiveIter<'a, G>: IntoExactSizeIterator<Item = T::Primitive>,
{
  fn intersect(
    &self,
    geometry: &BVHAcceleratedGeometry<G, B>,
    conf: &MeshBufferIntersectionConfig,
  ) -> IntersectionList3D {
    let mut result = IntersectionList3D::new();
    let geo_view = geometry.geometry.wrap();
    geometry.bvh.as_ref().map(|bvh| {
      bvh.traverse(
        |branch| branch.bounding.intersect(self, &()),
        |leaf| {
          leaf
            .iter_primitive(bvh)
            .map(|&i| geo_view.primitive_at(i).unwrap())
            .for_each(|p| result.push_nearest(p.intersect(self, conf)))
        },
      );
    });
    result
  }
}

impl<V, P, T, G, B> BVHAcceleratedGeometry<G, B>
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  B: BVHBounding + IntersectAble<Ray3, bool, ()> + From<T::Primitive>,
  for<'a> AbstractPrimitiveIter<'a, G>: IntoExactSizeIterator<Item = T::Primitive>,
{
  pub fn new(geometry: G) -> Self {
    Self {
      geometry,
      bvh: None,
    }
  }

  pub fn check_update_bvh<S: BVHBuildStrategy<B>>(
    &mut self,
    strategy: &mut S,
    option: &BVHOption,
  ) -> &FlattenBVH<B> {
    let geometry = &self.geometry;
    self.bvh.get_or_insert_with(|| {
      FlattenBVH::new(
        geometry.primitive_iter().into_iter().map(|p| p.into()),
        strategy,
        option,
      )
    })
  }

  pub fn geometry_mut(&mut self) -> &mut G {
    self.bvh = None;
    &mut self.geometry
  }

  pub fn geometry(&self) -> &G {
    &self.geometry
  }
}

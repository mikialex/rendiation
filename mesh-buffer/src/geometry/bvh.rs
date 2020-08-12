use super::{
  AbstractGeometry, AbstractPrimitiveIter, MeshBufferIntersectionConfig, PrimitiveData,
  PrimitiveTopology,
};
use rendiation_math_entity::*;
use space_indexer::bvh::*;

pub struct BVHAcceleratedGeometry<G: AbstractGeometry> {
  geometry: G,
  bvh: Option<FlattenBVH<Box3, BalanceTree>>,
}

impl<V, P, T, G>
  IntersectAble<BVHAcceleratedGeometry<G>, IntersectionList3D, MeshBufferIntersectionConfig>
  for Ray3
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  for<'a> AbstractPrimitiveIter<'a, G>: IntoIterator<Item = T::Primitive>,
{
  fn intersect(
    &self,
    geometry: &BVHAcceleratedGeometry<G>,
    conf: &MeshBufferIntersectionConfig,
  ) -> IntersectionList3D {
    let mut result = IntersectionList3D(Vec::new());
    let geo_view = geometry.geometry.wrap();
    geometry.bvh.as_ref().map(|bvh| {
      bvh.traverse(
        |branch| branch.bounding.intersect(self, &()),
        |leaf| {
          leaf
            .iter_primitive(bvh)
            .map(|&i| geo_view.primitive_at(i).unwrap())
            .for_each(|p| {
              result.push_nearest(p.intersect(self, conf));
            })
        },
      );
    });
    result
  }
}

impl<G: AbstractGeometry> BVHAcceleratedGeometry<G> {
  pub fn new(geometry: G) -> Self {
    Self {
      geometry,
      bvh: None,
    }
  }

  pub fn geometry_mut(&mut self) -> &mut G {
    &mut self.geometry
  }

  pub fn geometry(&self) -> &G {
    &self.geometry
  }
}

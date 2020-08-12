use super::{AbstractGeometry, MeshBufferIntersectionConfig, PrimitiveData, PrimitiveTopology, AbstractPrimitiveIter};
use rendiation_math_entity::*;
use space_indexer::bvh::*;

pub struct BVHAccleratedGeometry<G: AbstractGeometry> {
  geometry: G,
  bvh: Option<FlattenBVH<Box3, BalanceTree>>,
}

impl<V, P, T, G>
  IntersectAble<BVHAccleratedGeometry<G>, IntersectionList3D, MeshBufferIntersectionConfig>
  for Ray3
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> +
    IntersectAble<Ray3, bool, ()> +
    PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  for<'a> AbstractPrimitiveIter<'a, G>: IntoIterator<Item = T::Primitive>,
{
  fn intersect(&self, geometry: &BVHAccleratedGeometry<G>, conf: &MeshBufferIntersectionConfig) -> IntersectionList3D {
    let result = Vec::new();
    let geo_view = geometry.geometry.wrap();
    geometry.bvh.as_ref().map(|bvh|{
      bvh.traverse(|branch|{
        branch.bounding.intersect(self, &())
      },|leaf|{
        leaf.iter_primitive(bvh).for_each(|i|geo_view.privimitve_at(i).unwrap());
      });
    });
    IntersectionList3D(result)
  }
}

// impl<V, P, T, G>
//   IntersectAble<BVHAccleratedGeometry<G>, IntersectionList3D, MeshBufferIntersectionConfig>
//   for Ray3
// where
//   V: Positioned3D,
//   P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
//   T: PrimitiveTopology<V, Primitive = P>,
//   G: AbstractGeometry<Vertex = V, Topology = T>,
//   for<'a> AbstractPrimitiveIter<'a, G>: IntoIterator<Item = T::Primitive>,
// {
//   fn intersect(&self, geometry: &BVHAccleratedGeometry<G>, conf: &MeshBufferIntersectionConfig) -> IntersectionList3D {
//     IntersectionList3D(
//       geometry.warp()
//         .primitive_iter()
//         .into_iter()
//         .filter_map(|p| p.intersect(self, conf).0)
//         .collect(),
//     )
//   }
// }

impl<G: AbstractGeometry> BVHAccleratedGeometry<G> {
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

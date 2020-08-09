use super::{
  AbstractGeometry, AbstractGeometryRef, AbstractPrimitiveIter, PrimitiveData, PrimitiveTopology,
};
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{
  HitPoint3D, IntersectionList3D, LineRayIntersectionLocalTolerance, LineSegment, Point3,
  Positioned3D, Ray3, Triangle,
};

impl<'a, V, P, T, G> IntersectAble<AbstractGeometryRef<'a, G>, IntersectionList3D, Config> for Ray3
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  for<'b> AbstractPrimitiveIter<'b, G>: IntoIterator<Item = T::Primitive>,
{
  fn intersect(&self, geometry: &AbstractGeometryRef<'a, G>, conf: &Config) -> IntersectionList3D {
    IntersectionList3D(
      geometry
        .primitive_iter()
        .into_iter()
        .filter_map(|p| p.intersect(self, conf).0)
        .collect(),
    )
  }
}

impl<'a, V, P, T, G> IntersectAble<AbstractGeometryRef<'a, G>, NearestPoint3D, Config> for Ray3
where
  V: Positioned3D,
  P: IntersectAble<Ray3, NearestPoint3D, MeshBufferIntersectionConfig> + PrimitiveData<V>,
  T: PrimitiveTopology<V, Primitive = P>,
  G: AbstractGeometry<Vertex = V, Topology = T>,
  for<'b> AbstractPrimitiveIter<'b, G>: IntoIterator<Item = T::Primitive>,
{
  fn intersect(&self, geometry: &AbstractGeometryRef<'a, G>, conf: &Config) -> NearestPoint3D {
    let mut closest: Option<HitPoint3D> = None;
    geometry.primitive_iter().into_iter().for_each(|p| {
      let hit = p.intersect(self, conf);
      if let NearestPoint3D(Some(h)) = hit {
        if let Some(clo) = &closest {
          if h.distance < clo.distance {
            closest = Some(h)
          }
        } else {
          closest = Some(h)
        }
      }
    });
    NearestPoint3D(closest)
  }
}

pub struct MeshBufferIntersectionConfig {
  pub line_precision: LineRayIntersectionLocalTolerance,
}

type Config = MeshBufferIntersectionConfig;

impl<T: Positioned3D> IntersectAble<Ray3, NearestPoint3D, Config> for Triangle<T> {
  fn intersect(&self, ray: &Ray3, _p: &Config) -> NearestPoint3D {
    self.intersect(ray, &())
  }
}

impl<T: Positioned3D> IntersectAble<Ray3, NearestPoint3D, Config> for LineSegment<T> {
  fn intersect(&self, _ray: &Ray3, _: &Config) -> NearestPoint3D {
    todo!()
  }
}

impl<T: Positioned3D> IntersectAble<Ray3, NearestPoint3D, Config> for Point3<T> {
  fn intersect(&self, _ray: &Ray3, _: &Config) -> NearestPoint3D {
    todo!()
  }
}

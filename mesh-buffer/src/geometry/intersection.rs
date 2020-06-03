use super::{NoneIndexedGeometry, PrimitiveTopology};
use crate::geometry::indexed_geometry::IndexedGeometry;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::IntersectionList3D;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{Face3, Line3, Ray3, PositionedPoint3, Point3};

impl<V: PositionedPoint3, T: PrimitiveTopology<V>>
  IntersectAble<IndexedGeometry<V, T>, IntersectionList3D, Config> for Ray3
{
  fn intersect(&self, geometry: &IndexedGeometry<V, T>, p: &Config) -> IntersectionList3D {
    let mut result = Vec::new();
    for (primitive, _) in geometry.primitive_iter() {
      if let NearestPoint3D(Some(hit)) = primitive.intersect(self, p) {
        result.push(hit)
      }
    }
    IntersectionList3D(result)
  }
}

impl<V: PositionedPoint3, T: PrimitiveTopology<V>>
  IntersectAble<NoneIndexedGeometry<V, T>, IntersectionList3D, Config> for Ray3
{
  fn intersect(&self, geometry: &NoneIndexedGeometry<V, T>, p: &Config) -> IntersectionList3D {
    let mut result = Vec::new();
    for primitive in geometry.primitive_iter() {
      if let NearestPoint3D(Some(hit)) = primitive.intersect(self, p) {
        result.push(hit)
      }
    }
    IntersectionList3D(result)
  }
}

pub trait MeshBufferIntersectionConfigProvider {
  fn line_precision(&self) -> f32;
}

type Config = Box<dyn MeshBufferIntersectionConfigProvider>;

pub struct MeshBufferIntersectionConfig {
  line_precision: f32,
}

impl MeshBufferIntersectionConfigProvider for MeshBufferIntersectionConfig {
  fn line_precision(&self) -> f32 {
    self.line_precision
  }
}

impl<T: PositionedPoint3> IntersectAble<Ray3, NearestPoint3D, Config> for Face3<T> {
  fn intersect(&self, _ray: &Ray3, _p: &Config) -> NearestPoint3D {
    todo!()
    // IntersectAble::<Face3, NearestPoint3D>::intersect(ray, self, p)
  }
}

impl<T: PositionedPoint3> IntersectAble<Ray3, NearestPoint3D, Config> for Line3<T> {
  fn intersect(&self, _ray: &Ray3, _: &Config) -> NearestPoint3D {
    todo!()
  }
}

impl<T: PositionedPoint3> IntersectAble<Ray3, NearestPoint3D, Config> for Point3<T> {
  fn intersect(&self, _ray: &Ray3, _: &Config) -> NearestPoint3D {
    todo!()
  }
}

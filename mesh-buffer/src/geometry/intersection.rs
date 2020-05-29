use super::{NoneIndexedGeometry, PrimitiveTopology};
use crate::geometry::indexed_geometry::IndexedGeometry;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::IntersectionList;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{Face3, Line3, Ray, PositionedPoint, Point};

impl<V: PositionedPoint, T: PrimitiveTopology<V>>
  IntersectAble<IndexedGeometry<V, T>, IntersectionList, Config> for Ray
{
  fn intersect(&self, geometry: &IndexedGeometry<V, T>, p: &Config) -> IntersectionList {
    let mut result = Vec::new();
    for (primitive, _) in geometry.primitive_iter() {
      if let Some(NearestPoint3D(hit)) = primitive.intersect(self, p) {
        result.push(hit)
      }
    }
    IntersectionList(result)
  }
}

impl<V: PositionedPoint, T: PrimitiveTopology<V>>
  IntersectAble<NoneIndexedGeometry<V, T>, IntersectionList, Config> for Ray
{
  fn intersect(&self, geometry: &NoneIndexedGeometry<V, T>, p: &Config) -> IntersectionList {
    let mut result = Vec::new();
    for primitive in geometry.primitive_iter() {
      if let Some(NearestPoint3D(hit)) = primitive.intersect(self, p) {
        result.push(hit)
      }
    }
    IntersectionList(result)
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

impl<T: PositionedPoint> IntersectAble<Ray, Option<NearestPoint3D>, Config> for Face3<T> {
  fn intersect(&self, ray: &Ray, p: &Config) -> Option<NearestPoint3D> {
    todo!()
    // IntersectAble::<Face3, Option<NearestPoint3D>>::intersect(ray, self, p)
  }
}

impl<T: PositionedPoint> IntersectAble<Ray, Option<NearestPoint3D>, Config> for Line3<T> {
  fn intersect(&self, ray: &Ray, _: &Config) -> Option<NearestPoint3D> {
    todo!()
  }
}

impl<T: PositionedPoint> IntersectAble<Ray, Option<NearestPoint3D>, Config> for Point<T> {
  fn intersect(&self, ray: &Ray, _: &Config) -> Option<NearestPoint3D> {
    todo!()
  }
}

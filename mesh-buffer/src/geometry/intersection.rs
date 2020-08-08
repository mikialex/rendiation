use super::AbstractGeometry;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{
  IntersectionList3D, LineSegment, Point3, Positioned3D, Ray3, Triangle,
};

pub trait GeometryRayIntersection: AbstractGeometry {
  fn intersect<G: AbstractGeometry>(&self, ray: &Ray3, p: &Config) -> IntersectionList3D {
    let mut result = Vec::new();
    for (primitive, _) in self.primitive_iter() {
      if let NearestPoint3D(Some(hit)) = primitive.intersect(ray, p) {
        result.push(hit)
      }
    }
    IntersectionList3D(result)
  }
}

pub struct MeshBufferIntersectionConfig {
  line_precision: f32,
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

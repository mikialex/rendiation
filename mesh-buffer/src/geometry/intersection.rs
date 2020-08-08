use super::AbstractGeometry;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::{
  IntersectionList3D, LineRayIntersectionLocalTolerance, LineSegment, Point3, Positioned3D, Ray3,
  Triangle,
};

pub trait GeometryRayIntersection: AbstractGeometry {
  fn intersect_list(&self, ray: &Ray3, conf: &Config) -> IntersectionList3D {
    IntersectionList3D(
      self
        .primitive_iter()
        .filter_map(|(p, _)| p.intersect(ray, conf).0)
        .collect(),
    )
  }

  fn intersect_nearest(&self, _ray: &Ray3, _conf: &Config) -> NearestPoint3D {
    todo!()
    // self.primitive_iter().fold(None, |re, (p, _)| {
    //   let new_re =  primitive.intersect(ray, p);
    //   re.map_or_else(||new_re, |r|{

    //   })
    // })

    // for (primitive, _) in self.primitive_iter() {
    //   if let NearestPoint3D(Some(hit)) = primitive.intersect(ray, p) {
    //     result.push(hit)
    //   }
    // }
  }
}

// maybe we should still need impl intersection trait for geometry. use marco for convenience

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

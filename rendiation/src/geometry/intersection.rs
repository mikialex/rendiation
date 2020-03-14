use crate::geometry::standard_geometry::StandardGeometry;
use rendiation_math_entity::IntersectAble;
use rendiation_math_entity::IntersectionList;
use rendiation_math_entity::NearestPoint3D;
use rendiation_math_entity::Ray;

impl IntersectAble<StandardGeometry, IntersectionList> for Ray {
  fn intersect(&self, geometry: &StandardGeometry) -> IntersectionList {
    let mut result = Vec::new();
    for primitive in geometry.primitive_iter() {
      if let Some(NearestPoint3D(hit)) = self.intersect(&primitive) {
        result.push(hit)
      }
    }
    IntersectionList(result)
  }
}

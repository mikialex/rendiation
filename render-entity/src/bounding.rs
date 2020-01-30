use rendiation_math_entity::*;

pub struct BoundingData {
  pub bounding_box: Box3,
  pub bounding_sphere: Sphere,
}

impl BoundingData {
  pub fn if_intersect_ray(&self, ray: &Ray) -> bool {
    ray.if_intersect(&self.bounding_sphere) && ray.if_intersect(&self.bounding_box)
  }
}

pub trait Bounding<T> {
  fn create(item: &T) -> BoundingData;
  fn update(item: &T, bounding: BoundingData);
}

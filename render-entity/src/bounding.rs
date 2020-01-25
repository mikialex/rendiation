use rendiation_math_entity::*;

pub struct BoundingData{
  pub bounding_box: Box3,
  pub bounding_sphere: Sphere,
}

pub trait Bounding<T>{
  fn create(item: &T) -> BoundingData;
  fn update(item: &T, bounding: BoundingData);
} 
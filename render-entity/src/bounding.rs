use rendiation_math_entity::*;

pub trait Bounding<T> {
  fn create(item: &T) -> BoundingData;
  fn update(item: &T, bounding: BoundingData);
}

pub struct BoundingData {
  pub bounding_box: Box3,
  pub bounding_sphere: Sphere,
}

impl BoundingData {
  pub fn new_from_box(box3: Box3) -> Self {
    let bounding_sphere = Sphere::new_from_box(box3);
    Self {
      bounding_box: box3,
      bounding_sphere,
    }
  }

}

impl IntersectAble<Ray3, bool> for BoundingData{
  fn intersect(&self, ray: &Ray3, _: &()) -> bool {
    ray.intersect(&self.bounding_sphere, &()) && self.bounding_box.intersect(ray, &())
  }
}

impl IntersectAble<Frustum, bool> for BoundingData{
  fn intersect(&self, f: &Frustum, _: &()) -> bool {
    f.intersect(&self.bounding_sphere, &()) && f.intersect(&self.bounding_box, &())
  }
}
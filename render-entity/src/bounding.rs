use rendiation_math_entity::*;

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

  pub fn if_intersect_ray(&self, ray: &Ray) -> bool {
    ray.intersect(&self.bounding_sphere) && ray.intersect(&self.bounding_box)
  }

  pub fn if_intersect_frustum(&self, f: &Frustum) -> bool {
    todo!()
    // f.intersect(&self.bounding_sphere) && f.intersect(&self.bounding_box)
  }
}

pub trait Bounding<T> {
  fn create(item: &T) -> BoundingData;
  fn update(item: &T, bounding: BoundingData);
}

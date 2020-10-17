use rand::random;
use rendiation_math::Vec3;
use rendiation_math_entity::Box3;

pub fn generate_boxes_in_space(count: usize, space_size: f32, box_size: f32) -> Vec<Box3> {
  (0..count)
    .into_iter()
    .map(|_| {
      let center = Vec3::new(random(), random(), random()) * space_size;
      let half_size = Vec3::new(random(), random(), random()) * box_size;
      Box3::new(center + half_size, center - half_size)
    })
    .collect()
}

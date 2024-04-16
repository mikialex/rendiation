use crate::*;

pub trait Material:
  Send + Sync + 'static + dyn_clone::DynClone + LightTransportSurface<Intersection>
{
}

impl<T> Material for T where
  T: Send + Sync + 'static + dyn_clone::DynClone + LightTransportSurface<Intersection>
{
}

dyn_clone::clone_trait_object!(Material);

#[derive(Clone)]
pub struct Model {
  pub shape: Box<dyn Shape>,
  pub material: Box<dyn Material>,
  pub world_matrix: Mat4<f32>,
  pub world_matrix_inverse: Mat4<f32>,
  pub normal_matrix: Mat4<f32>, // object space direction to world_space
}

impl Model {
  pub fn update_nearest_hit<'b>(
    &'b self,
    world_ray: Ray3,
    result: &mut Option<(Intersection, &'b Self)>,
    min_distance: &mut f32,
  ) {
    let Self {
      world_matrix,
      world_matrix_inverse,
      normal_matrix,
      ..
    } = self;

    let local_ray = world_ray.apply_matrix_into(*world_matrix_inverse);

    if let Some(mut intersection) = self.shape.intersect(local_ray) {
      intersection.apply_matrix(*world_matrix, *normal_matrix);
      let distance = intersection.position.distance(world_ray.origin);

      if distance < *min_distance {
        intersection.adjust_hit_position();
        *min_distance = distance;
        *result = Some((intersection, self))
      }
    }
  }

  pub fn has_any_hit(&self, world_ray: Ray3) -> bool {
    let local_ray = world_ray.apply_matrix_into(self.world_matrix_inverse);
    self.shape.has_any_intersect(local_ray)
  }

  pub fn get_intersection_stat(&self, world_ray: Ray3) -> IntersectionStatistic {
    let local_ray = world_ray.apply_matrix_into(self.world_matrix_inverse);
    self.shape.intersect_statistic(local_ray)
  }
}

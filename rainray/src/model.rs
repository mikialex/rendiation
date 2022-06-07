use rendiation_algebra::Vec3;

use crate::*;

#[derive(Clone)]
pub struct Model {
  pub shape: Box<dyn Shape>,
  pub material: Box<dyn Material>,
  pub world_matrix: Mat4<f32>,
  pub world_matrix_inverse: Mat4<f32>,
  pub normal_matrix: Mat4<f32>, // object space direction to world_space
}

impl Model {
  pub fn new<M, G>(shape: G, material: M) -> Self
  where
    M: Material,
    G: Shape,
  {
    let shape = Box::new(shape);
    let material = Box::new(material);
    Model {
      shape,
      material,
      world_matrix: Default::default(),
      world_matrix_inverse: Default::default(),
      normal_matrix: Default::default(),
    }
  }
  pub fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
  ) -> BSDFSampleResult {
    let light_dir = self
      .material
      .sample_light_dir_use_bsdf_importance(view_dir, intersection);
    let bsdf = self.material.bsdf(view_dir, light_dir.sample, intersection);
    BSDFSampleResult { light_dir, bsdf }
  }

  pub fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
  ) -> Vec3<f32> {
    self.material.bsdf(view_dir, light_dir, intersection)
  }

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

    if let PossibleIntersection(Some(mut intersection)) = self.shape.intersect(local_ray) {
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

pub struct BSDFSampleResult {
  pub light_dir: ImportanceSampled<NormalizedVec3<f32>>,
  pub bsdf: Vec3<f32>,
}

pub struct ImportanceSampled<T> {
  pub sample: T,
  pub pdf: f32,
}

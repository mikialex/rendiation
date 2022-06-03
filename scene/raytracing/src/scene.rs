use crate::*;
use arena::Handle;
use arena_tree::ArenaTreeNodeHandle;
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

#[derive(Copy, Clone)]
pub struct RayTracingScene;
impl SceneContent for RayTracingScene {
  type BackGround = Box<dyn Background>;
  type Model = Model;
  type Light = Light;
  type Texture2D = ();
  type TextureCube = ();
  type SceneExt = SceneAcceleration;
}

#[derive(Default)]
pub struct SceneAcceleration {
  models_in_bvh: Vec<ModelInstance>,
  models_unbound: Vec<ModelInstance>,
  models_bvh: Option<FlattenBVH<Box3>>,
}

pub trait RayTracingSceneExt {
  fn create_node(&mut self, builder: impl Fn(&mut SceneNodeDataImpl, &mut Self)) -> &mut Self;
  fn model_node(&mut self, shape: impl Shape, material: impl Material) -> &mut Self;
  fn model_node_with_modify(
    &mut self,
    shape: impl Shape,
    material: impl Material,
    m: impl Fn(&mut SceneNodeDataImpl),
  ) -> &mut Self;
  fn background(&mut self, background: impl Background) -> &mut Self;
  fn update(&mut self);
  fn get_any_hit(&self, world_ray: Ray3) -> bool;
  fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic;
  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32, &ModelInstance)>;
  fn test_point_visible_to_point(&self, point_a: Vec3<f32>, point_b: Vec3<f32>) -> bool;
}

impl RayTracingSceneExt for Scene<RayTracingScene> {
  fn create_node(&mut self, builder: impl Fn(&mut SceneNodeDataImpl, &mut Self)) -> &mut Self {
    let node = self.root().create_child();
    node.mutate(|node| builder(node, self));
    self
  }

  fn model_node(&mut self, shape: impl Shape, material: impl Material) -> &mut Self {
    let node = self.root().create_child();
    let model = Model::new(shape, material, node);
    self.models.push(model);
    self
  }

  fn model_node_with_modify(
    &mut self,
    shape: impl Shape,
    material: impl Material,
    m: impl Fn(&mut SceneNodeDataImpl),
  ) -> &mut Self {
    let node = self.root().create_child();
    node.mutate(|node| m(node));
    let model = Model::new(shape, material, node);
    self.models.push(model);
    self
  }

  fn background(&mut self, background: impl Background) -> &mut Self {
    let background: Box<dyn Background> = Box::new(background);
    self.background = background.into();
    self
  }

  fn update(&mut self) {
    let mut models_unbound = Vec::new();
    let mut models_in_bvh = Vec::new();
    let mut models_in_bvh_source = Vec::new();

    self.maintain();

    for (i, model) in self.models.iter().enumerate() {
      model.node.visit(|node_data| {
        let world_matrix_inverse = node_data.world_matrix.inverse_or_identity();
        let instance = ModelInstance {
          world_matrix: node_data.world_matrix,
          world_matrix_inverse,
          normal_matrix: world_matrix_inverse.transpose(),
          model: i,
        };
        if let Some(mut bbox) = model.shape.get_bbox(self) {
          models_in_bvh.push(instance);
          models_in_bvh_source.push(*bbox.apply_matrix(node_data.world_matrix));
        } else {
          models_unbound.push(instance);
        }
      });
    }

    // for (i, light) in self.lights.iter().enumerate() {
    //   //
    // }

    let models_bvh = FlattenBVH::new(
      models_in_bvh_source.into_iter(),
      &mut SAH::new(6),
      &TreeBuildOption::default(),
    );

    self.extension.models_bvh = models_bvh.into();
    self.extension.models_in_bvh = models_in_bvh;
    self.extension.models_unbound = models_unbound;
  }

  fn get_any_hit(&self, world_ray: Ray3) -> bool {
    let mut find = false;
    let ext = &self.extension;
    let bvh = ext.models_bvh.as_ref().unwrap();
    bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        find = leaf.iter_primitive(bvh).any(|&i| {
          let model = &ext.models_in_bvh[i];
          model.has_any_hit(world_ray, self)
        });
        !find
      },
    );
    if find {
      return true;
    }

    for model in &ext.models_unbound {
      if model.has_any_hit(world_ray, self) {
        return true;
      }
    }
    false
  }

  fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic {
    let mut box_c = 0;
    let mut stat = IntersectionStatistic::default();
    let ext = &self.extension;
    let bvh = ext.models_bvh.as_ref().unwrap();
    bvh.traverse(
      |branch| {
        box_c += 1;
        branch.bounding.intersect(&world_ray, &())
      },
      |leaf| {
        leaf.iter_primitive(bvh).for_each(|&i| {
          let model = &ext.models_in_bvh[i];
          stat += model.get_intersection_stat(world_ray, self);
        });
        true
      },
    );

    for model in &ext.models_unbound {
      stat += model.get_intersection_stat(world_ray, self);
    }
    stat
  }

  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32, &ModelInstance)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result = None;
    let ext = &self.extension;

    let bvh = ext.models_bvh.as_ref().unwrap();

    bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        leaf.iter_primitive(bvh).for_each(|&i| {
          let model = &ext.models_in_bvh[i];
          model.update_nearest_hit(world_ray, self, &mut result, &mut min_distance);
        });
        true
      },
    );

    for model in &ext.models_unbound {
      model.update_nearest_hit(world_ray, self, &mut result, &mut min_distance);
    }

    result.map(|(intersection, model)| (intersection, min_distance, model))
  }

  fn test_point_visible_to_point(&self, point_a: Vec3<f32>, point_b: Vec3<f32>) -> bool {
    let ray = Ray3::from_point_to_point(point_a, point_b);
    let distance = (point_a - point_b).length();

    if let Some(hit_result) = self.get_min_dist_hit(ray) {
      hit_result.1 > distance
    } else {
      true
    }
  }
}

pub type NodeHandle = ArenaTreeNodeHandle<SceneNode>;
pub type ModelHandle = usize;
pub type LightHandle = Handle<Light>;

pub struct ModelInstance {
  pub world_matrix: Mat4<f32>,
  pub world_matrix_inverse: Mat4<f32>,
  pub normal_matrix: Mat4<f32>, // object space direction to world_space
  pub model: ModelHandle,
}

impl ModelInstance {
  pub fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    scene: &Scene<RayTracingScene>,
  ) -> BSDFSampleResult {
    let model = scene.models.get(self.model).unwrap();
    let light_dir = model
      .material
      .sample_light_dir_use_bsdf_importance(view_dir, intersection);
    let bsdf = model
      .material
      .bsdf(view_dir, light_dir.sample, intersection);
    BSDFSampleResult { light_dir, bsdf }
  }

  pub fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    scene: &Scene<RayTracingScene>,
  ) -> Vec3<f32> {
    let model = scene.models.get(self.model).unwrap();
    model.material.bsdf(view_dir, light_dir, intersection)
  }

  pub fn update_nearest_hit<'b>(
    &'b self,
    world_ray: Ray3,
    scene: &Scene<RayTracingScene>,
    result: &mut Option<(Intersection, &'b ModelInstance)>,
    min_distance: &mut f32,
  ) {
    let ModelInstance {
      world_matrix,
      world_matrix_inverse,
      normal_matrix,
      ..
    } = self;

    let local_ray = world_ray.apply_matrix_into(*world_matrix_inverse);
    let model = scene.models.get(self.model).unwrap();

    if let PossibleIntersection(Some(mut intersection)) = model.shape.intersect(local_ray, scene) {
      intersection.apply_matrix(*world_matrix, *normal_matrix);
      let distance = intersection.position.distance(world_ray.origin);

      if distance < *min_distance {
        intersection.adjust_hit_position();
        *min_distance = distance;
        *result = Some((intersection, self))
      }
    }
  }

  pub fn has_any_hit(&self, world_ray: Ray3, scene: &Scene<RayTracingScene>) -> bool {
    let local_ray = world_ray.apply_matrix_into(self.world_matrix_inverse);
    let model = scene.models.get(self.model).unwrap();
    model.shape.has_any_intersect(local_ray, scene)
  }

  pub fn get_intersection_stat(
    &self,
    world_ray: Ray3,
    scene: &Scene<RayTracingScene>,
  ) -> IntersectionStatistic {
    let local_ray = world_ray.apply_matrix_into(self.world_matrix_inverse);
    let model = scene.models.get(self.model).unwrap();
    model.shape.intersect_statistic(local_ray, scene)
  }
}

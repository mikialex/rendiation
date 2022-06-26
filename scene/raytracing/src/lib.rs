#![feature(explicit_generic_args_with_impl_trait)]
#![feature(generic_const_exprs)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::many_single_char_names)]
#![allow(unstable_name_collisions)]
#![allow(incomplete_features)]

mod frame;
mod integrator;
mod sampling;
pub use sampling::*;

pub use frame::*;
pub use integrator::*;

pub mod background;
pub mod light;
pub mod material;
pub mod math;
pub mod model;
pub mod shape;

pub use background::*;
pub use light::*;
pub use material::*;
pub use math::*;
pub use model::*;
pub use shape::*;

use rendiation_algebra::*;

pub use rendiation_scene_core::*;

use arena::Handle;
use arena_tree::ArenaTreeNodeHandle;
pub use rendiation_scene_core::*;
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

pub struct ModelNode {
  model: Model,
  node: SceneNode,
}

// pub struct LightNode {
//   model: Light,
//   node: SceneNode,
// }

#[derive(Copy, Clone)]
pub struct RayTracingScene;
impl SceneContent for RayTracingScene {
  type BackGround = Box<dyn RayTracingBackground>;
  type Model = ModelNode;
  type Light = ();
  type Texture2D = ();
  type TextureCube = ();
  type SceneExt = ();
}

#[derive(Default)]
pub struct SceneAcceleration {
  models_in_bvh: Vec<Model>,
  models_unbound: Vec<Model>,
  models_bvh: Option<FlattenBVH<Box3>>,
  env: Option<Box<dyn RayTracingBackground>>,
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
  fn background(&mut self, background: impl RayTracingBackground) -> &mut Self;
  fn build_traceable(&mut self) -> SceneAcceleration;
}

impl RayTracingSceneExt for Scene<RayTracingScene> {
  fn create_node(&mut self, builder: impl Fn(&mut SceneNodeDataImpl, &mut Self)) -> &mut Self {
    let node = self.root().create_child();
    node.mutate(|node| builder(node, self));
    self
  }

  fn model_node(&mut self, shape: impl Shape, material: impl Material) -> &mut Self {
    let node = self.root().create_child();
    let model = ModelNode {
      model: Model::new(shape, material),
      node,
    };
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
    let model = ModelNode {
      model: Model::new(shape, material),
      node,
    };
    self.models.push(model);
    self
  }

  fn background(&mut self, background: impl RayTracingBackground) -> &mut Self {
    let background: Box<dyn RayTracingBackground> = Box::new(background);
    self.background = background.into();
    self
  }

  fn build_traceable(&mut self) -> SceneAcceleration {
    self.maintain();

    let mut result = SceneAcceleration::default();

    let mut models_in_bvh_source = Vec::new();

    for model in self.models.iter() {
      model.node.visit(|node_data| {
        let mut instance = model.model.clone();
        instance.world_matrix_inverse = node_data.world_matrix.inverse_or_identity();
        instance.normal_matrix = instance.world_matrix_inverse.transpose();
        instance.world_matrix = node_data.world_matrix;

        if let Some(mut bbox) = model.model.shape.get_bbox() {
          result.models_in_bvh.push(instance);
          models_in_bvh_source.push(*bbox.apply_matrix(node_data.world_matrix));
        } else {
          result.models_unbound.push(instance);
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

    result.models_bvh = models_bvh.into();
    result.env = self.background.clone();
    result
  }
}

impl RayTraceable for SceneAcceleration {
  fn get_any_hit(&self, world_ray: Ray3) -> bool {
    let mut find = false;
    let bvh = self.models_bvh.as_ref().unwrap();
    bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        find = leaf.iter_primitive(bvh).any(|&i| {
          let model = &self.models_in_bvh[i];
          model.has_any_hit(world_ray)
        });
        !find
      },
    );
    if find {
      return true;
    }

    for model in &self.models_unbound {
      if model.has_any_hit(world_ray) {
        return true;
      }
    }
    false
  }

  fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic {
    let mut box_c = 0;
    let mut stat = IntersectionStatistic::default();
    let bvh = self.models_bvh.as_ref().unwrap();
    bvh.traverse(
      |branch| {
        box_c += 1;
        branch.bounding.intersect(&world_ray, &())
      },
      |leaf| {
        leaf.iter_primitive(bvh).for_each(|&i| {
          let model = &self.models_in_bvh[i];
          stat += model.get_intersection_stat(world_ray);
        });
        true
      },
    );

    for model in &self.models_unbound {
      stat += model.get_intersection_stat(world_ray);
    }
    stat
  }

  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32, &Model)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result = None;

    let bvh = self.models_bvh.as_ref().unwrap();

    bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        leaf.iter_primitive(bvh).for_each(|&i| {
          let model = &self.models_in_bvh[i];
          model.update_nearest_hit(world_ray, &mut result, &mut min_distance);
        });
        true
      },
    );

    for model in &self.models_unbound {
      model.update_nearest_hit(world_ray, &mut result, &mut min_distance);
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

  fn sample_environment(&self, world_ray: Ray3) -> Vec3<f32> {
    if let Some(env) = &self.env {
      env.sample(&world_ray)
    } else {
      Vec3::zero()
    }
  }
}

pub type NodeHandle = ArenaTreeNodeHandle<SceneNode>;
pub type ModelHandle = usize;
pub type LightHandle = Handle<Light>;

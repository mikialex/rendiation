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
  type Model = Box<dyn RayTracingModel>;
  type Light = ();
  type Texture2D = ();
  type TextureCube = ();
  type SceneExt = ();
}

pub trait RayTracingModel {
  fn get_shape(&self) -> Box<dyn Shape>;
  fn get_material(&self) -> Box<dyn Material>;
  fn get_node(&self) -> &SceneNode;
}

impl RayTracingModel for ModelNode {
  fn get_shape(&self) -> Box<dyn Shape> {
    self.model.shape.clone()
  }

  fn get_material(&self) -> Box<dyn Material> {
    self.model.material.clone()
  }

  fn get_node(&self) -> &SceneNode {
    &self.node
  }
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
    let _ = self.models.insert(Box::new(model));
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
    let _ = self.models.insert(Box::new(model));
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

    for (_, model) in self.models.iter() {
      model.get_node().visit(|node_data| {
        let mut model = Model {
          shape: model.get_shape(),
          material: model.get_material(),
          world_matrix: Default::default(),
          world_matrix_inverse: Default::default(),
          normal_matrix: Default::default(), // object space direction to world_space
        };
        model.world_matrix_inverse = node_data.world_matrix.inverse_or_identity();
        model.normal_matrix = model.world_matrix_inverse.transpose();
        model.world_matrix = node_data.world_matrix;

        if let Some(mut bbox) = model.shape.get_bbox() {
          result.models_in_bvh.push(model);
          models_in_bvh_source.push(*bbox.apply_matrix(node_data.world_matrix));
        } else {
          result.models_unbound.push(model);
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

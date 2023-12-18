#![allow(clippy::or_fun_call)]
#![allow(clippy::many_single_char_names)]
#![allow(unstable_name_collisions)]

mod integrator;
pub use integrator::*;

mod background;
mod light;
mod model;
mod shape;

pub use background::*;
pub use light::*;
pub use model::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_lighting_transport::*;
pub use rendiation_scene_core::*;
use rendiation_statistics::*;
pub use shape::*;
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};
use tree::ComputedDerivedTree;

#[derive(Clone)]
pub struct RayTracingSceneModel {
  pub shape: Box<dyn Shape>,
  pub material: Box<dyn Material>,
}

// pub struct LightNode {
//   model: Light,
//   node: SceneNode,
// }

pub struct SceneAcceleration {
  models_in_bvh: Vec<Model>,
  models_unbound: Vec<Model>,
  models_bvh: Option<FlattenBVH<Box3>>,
  env: Option<Box<dyn RayTracingBackground>>,
  worlds: ComputedDerivedTree<SceneNodeDerivedData>,
}

pub struct RayTracingCamera {
  pub proj: CameraProjectionEnum,
  pub world: Mat4<f32>,
}

impl HyperRayCaster<f32, Vec3<f32>, Vec2<f32>> for RayTracingCamera {
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> HyperRay<f32, Vec3<f32>> {
    self
      .proj
      .cast_ray(normalized_position)
      .unwrap()
      .apply_matrix_into(self.world)
  }
}

impl SceneAcceleration {
  pub fn build_camera(&self, camera: &SceneCamera) -> RayTracingCamera {
    let camera = camera.read();
    RayTracingCamera {
      proj: camera.projection.clone(),
      world: self
        .worlds
        .get_computed(camera.node.raw_handle().index())
        .world_matrix,
    }
  }
}

pub trait RayTracingSceneExt {
  fn create_node(&mut self, builder: impl Fn(&SceneNode, &mut Self)) -> &mut Self;
  fn model_node(&mut self, shape: impl Shape, material: impl Material) -> &mut Self;
  fn model_node_with_modify(
    &mut self,
    shape: impl Shape,
    material: impl Material,
    m: impl Fn(&SceneNode),
  ) -> &mut Self;
  fn background(&mut self, background: impl RayTracingBackground) -> &mut Self;
  fn build_traceable(&mut self) -> SceneAcceleration;
}

impl RayTracingSceneExt for Scene {
  fn create_node(&mut self, builder: impl Fn(&SceneNode, &mut Self)) -> &mut Self {
    let node = self.create_root_child();
    builder(&node, self);
    self
  }

  fn model_node(&mut self, shape: impl Shape, material: impl Material) -> &mut Self {
    let node = self.create_root_child();
    let model = RayTracingSceneModel {
      shape: Box::new(shape),
      material: Box::new(material),
    };
    let model = ModelEnum::Foreign(Box::new(model));
    let model = SceneModelImpl::new(model, node);
    let _ = self.insert_model(model.into());
    self
  }

  fn model_node_with_modify(
    &mut self,
    shape: impl Shape,
    material: impl Material,
    m: impl Fn(&SceneNode),
  ) -> &mut Self {
    let node = self.create_root_child();
    m(&node);
    let model = RayTracingSceneModel {
      shape: Box::new(shape),
      material: Box::new(material),
    };
    let model = ModelEnum::Foreign(Box::new(model));
    let model = SceneModelImpl::new(model, node);
    let _ = self.insert_model(model.into());
    self
  }

  fn background(&mut self, background: impl RayTracingBackground) -> &mut Self {
    let background: Box<dyn RayTracingBackground> = Box::new(background);
    self.set_background(background.create_scene_background());
    self
  }

  fn build_traceable(&mut self) -> SceneAcceleration {
    let mut result = SceneAcceleration {
      models_in_bvh: Default::default(),
      models_unbound: Default::default(),
      models_bvh: Default::default(),
      env: Default::default(),
      worlds: self.compute_full_derived(),
    };

    let mut models_in_bvh_source = Vec::new();

    for (_, model) in self.read().core.read().models.iter() {
      let model = model.read();
      if let ModelEnum::Foreign(foreign) = &model.model {
        if let Some(retraceable) = foreign.as_any().downcast_ref::<RayTracingSceneModel>() {
          let world_info = result.worlds.get_computed(model.node.raw_handle().index());

          let mut model = Model {
            shape: retraceable.shape.clone(),
            material: retraceable.material.clone(),
            world_matrix: Default::default(),
            world_matrix_inverse: Default::default(),
            normal_matrix: Default::default(), // object space direction to world_space
          };
          model.world_matrix_inverse = world_info.world_matrix.inverse_or_identity();
          model.normal_matrix = model.world_matrix_inverse.transpose();
          model.world_matrix = world_info.world_matrix;

          if let Some(mut bbox) = model.shape.get_bbox() {
            result.models_in_bvh.push(model);
            models_in_bvh_source.push(*bbox.apply_matrix(world_info.world_matrix));
          } else {
            result.models_unbound.push(model);
          }
        }
      }
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

    if let Some(bg) = &self.read().core.read().background {
      match bg {
        SceneBackGround::Solid(bg) => {
          result.env = Some(Box::new(*bg));
        }
        SceneBackGround::Env(_) => {}
        SceneBackGround::Foreign(foreign) => {
          if let Some(retraceable_bg) = foreign
            .as_ref()
            .as_any()
            .downcast_ref::<std::sync::Arc<dyn RayTracingBackground>>()
          {
            result.env = Some(dyn_clone::clone_box(&**retraceable_bg));
          }
        }
      }
    }

    result
  }
}

impl RayTraceContentBase for SceneAcceleration {
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

  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32)> {
    self
      .get_min_dist_hit_with_model(world_ray)
      .map(|(a, b, _)| (a, b))
  }
}

impl RayTraceContentForStat for SceneAcceleration {
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
}

impl RayTraceContentForPathTracing for SceneAcceleration {
  fn get_min_dist_hit_with_model(&self, world_ray: Ray3) -> Option<(Intersection, f32, &Model)> {
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

  fn sample_environment(&self, world_ray: Ray3) -> Vec3<f32> {
    if let Some(env) = &self.env {
      env.sample(&world_ray)
    } else {
      Vec3::zero()
    }
  }
}

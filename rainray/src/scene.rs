use crate::*;
use arena_tree::NextTraverseVisit;
use rendiation_algebra::*;
use rendiation_geometry::{Box3, Ray3};
use sceno::SceneBackend;
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

pub struct RainrayScene;

impl SceneBackend for RainrayScene {
  type Model = Model;
  type Material = Box<dyn RainrayMaterial>;
  type Mesh = Box<dyn RainRayGeometry>;
  type Background = Box<dyn Background>;
  type Light = Box<dyn Light>;
}

pub type Scene = sceno::Scene<RainrayScene>;
pub type SceneNode = sceno::SceneNode<RainrayScene>;
pub type NodeHandle = sceno::SceneNodeHandle<RainrayScene>;
pub type ModelHandle = sceno::ModelHandle<RainrayScene>;
pub type MeshHandle = sceno::MeshHandle<RainrayScene>;
pub type MaterialHandle = sceno::MaterialHandle<RainrayScene>;

pub struct ModelInstance<'a> {
  pub node: &'a SceneNode,
  pub matrix_world_inverse: Mat4<f32>,
  pub normal_matrix: Mat4<f32>, // object space direction to world_space
  pub material: &'a dyn RainrayMaterial,
  pub geometry: &'a dyn RainRayGeometry,
}

impl<'a> ModelInstance<'a> {
  pub fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    _scene: &RayTraceScene<'a>,
  ) -> BSDFSampleResult {
    let light_dir = self
      .material
      .sample_light_dir_use_bsdf_importance(view_dir, intersection);
    let bsdf = self
      .material
      .bsdf(view_dir, light_dir.sample, &intersection);
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
    scene: &RayTraceScene<'a>,
    result: &mut Option<(Intersection, &'b ModelInstance<'a>)>,
    min_distance: &mut f32,
  ) {
    let ModelInstance {
      matrix_world_inverse,
      normal_matrix,
      node,
      geometry,
      ..
    } = self;

    let local_ray = world_ray.apply_matrix_into(*matrix_world_inverse);

    if let PossibleIntersection(Some(mut intersection)) = geometry.intersect(local_ray, scene) {
      intersection.apply_matrix(node.world_matrix, *normal_matrix);
      let distance = intersection.position.distance(world_ray.origin);

      if distance < *min_distance {
        intersection.adjust_hit_position();
        *min_distance = distance;
        *result = Some((intersection, self))
      }
    }
  }

  pub fn has_any_hit(&self, world_ray: Ray3, scene: &RayTraceScene<'a>) -> bool {
    let local_ray = world_ray.apply_matrix_into(self.matrix_world_inverse);
    self.geometry.has_any_intersect(local_ray, scene)
  }

  pub fn get_intersection_stat(
    &self,
    world_ray: Ray3,
    scene: &RayTraceScene<'a>,
  ) -> IntersectionStatistic {
    let local_ray = world_ray.apply_matrix_into(self.matrix_world_inverse);
    self.geometry.acceleration_traverse_count(local_ray, scene)
  }
}

pub struct LightInstance<'a> {
  pub node: &'a SceneNode,
  pub light: &'a dyn Light,
}

pub struct RayTraceScene<'a> {
  pub scene: &'a Scene,
  pub lights: Vec<LightInstance<'a>>,
  pub models_in_bvh: Vec<ModelInstance<'a>>,
  pub models_unbound: Vec<ModelInstance<'a>>,
  pub models_bvh: FlattenBVH<Box3>,
}

impl<'a> RayTraceScene<'a> {
  // need a distance version
  pub fn get_any_hit(&self, world_ray: Ray3) -> bool {
    let mut find = false;
    self.models_bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        find = leaf.iter_primitive(&self.models_bvh).any(|&i| {
          let model = &self.models_in_bvh[i];
          model.has_any_hit(world_ray, self)
        });
        !find
      },
    );
    if find {
      return true;
    }

    for model in &self.models_unbound {
      if model.has_any_hit(world_ray, self) {
        return true;
      }
    }
    return false;
  }

  pub fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic {
    let mut box_c = 0;
    let mut stat = IntersectionStatistic::default();
    self.models_bvh.traverse(
      |branch| {
        box_c += 1;
        branch.bounding.intersect(&world_ray, &())
      },
      |leaf| {
        leaf.iter_primitive(&self.models_bvh).for_each(|&i| {
          let model = &self.models_in_bvh[i];
          stat += model.get_intersection_stat(world_ray, self);
        });
        true
      },
    );

    for model in &self.models_unbound {
      stat += model.get_intersection_stat(world_ray, self);
    }
    stat
  }

  pub fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32, &ModelInstance)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result = None;

    self.models_bvh.traverse(
      |branch| branch.bounding.intersect(&world_ray, &()),
      |leaf| {
        leaf.iter_primitive(&self.models_bvh).for_each(|&i| {
          let model = &self.models_in_bvh[i];
          model.update_nearest_hit(world_ray, self, &mut result, &mut min_distance);
        });
        true
      },
    );

    for model in &self.models_unbound {
      model.update_nearest_hit(world_ray, self, &mut result, &mut min_distance);
    }

    result.map(|(intersection, model)| (intersection, min_distance, model))
  }

  pub fn test_point_visible_to_point(&self, point_a: Vec3<f32>, point_b: Vec3<f32>) -> bool {
    let ray = Ray3::from_point_to_point(point_a, point_b);
    let distance = (point_a - point_b).length();

    if let Some(hit_result) = self.get_min_dist_hit(ray) {
      hit_result.1 > distance
    } else {
      true
    }
  }
}

pub trait RainraySceneExt {
  fn convert(&self) -> RayTraceScene;
}

impl RainraySceneExt for Scene {
  fn convert(&self) -> RayTraceScene {
    let scene_light = &self.lights;
    let scene_model = &self.models;
    let scene_materials = &self.materials;
    let scene_geometries = &self.meshes;

    let mut lights = Vec::new();
    let mut models_unbound = Vec::new();
    let mut models_in_bvh = Vec::new();
    let mut models_in_bvh_source = Vec::new();

    let root = self.get_root_handle();
    self.nodes.traverse(root, &mut Vec::new(), |this, _| {
      let node_data = this.data();
      node_data.payloads.iter().for_each(|payload| match payload {
        sceno::SceneNodePayload::Model(model) => {
          let model = scene_model.get(*model).unwrap();
          let matrix_world_inverse = node_data.world_matrix.inverse_or_identity();
          let instance = ModelInstance {
            node: node_data,
            matrix_world_inverse,
            normal_matrix: matrix_world_inverse.transpose(),
            material: scene_materials.get(model.material).unwrap().as_ref(),
            geometry: scene_geometries.get(model.geometry).unwrap().as_ref(),
          };
          if let Some(mut bbox) = instance.geometry.get_bbox(self) {
            models_in_bvh.push(instance);
            models_in_bvh_source.push(*bbox.apply_matrix(node_data.world_matrix));
          } else {
            models_unbound.push(instance);
          }
        }
        sceno::SceneNodePayload::Light(light) => {
          let light = scene_light.get(*light).unwrap().as_ref();
          lights.push(LightInstance {
            node: node_data,
            light,
          });
        }
      });
      NextTraverseVisit::VisitChildren
    });

    let models_bvh = FlattenBVH::new(
      models_in_bvh_source.into_iter(),
      &mut SAH::new(6),
      &TreeBuildOption::default(),
    );

    RayTraceScene {
      scene: self,
      lights,
      models_unbound,
      models_in_bvh,
      models_bvh,
    }
  }
}

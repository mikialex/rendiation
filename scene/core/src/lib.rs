use bytemuck::*;
use database::*;
use reactive::*;
use reactive_derive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_texture_core::*;

mod animation;
mod buffer;
mod camera;
mod light;
mod material;
mod mesh;
mod model;
mod node;
mod reader;
mod skin;
mod texture;
mod writer;

pub use animation::*;
pub use buffer::*;
pub use camera::*;
pub use light::*;
pub use material::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use reader::*;
pub use skin::*;
pub use texture::*;
pub use writer::*;

pub fn register_scene_core_data_model() {
  register_scene_self_data_model();
  register_scene_node_data_model();
  register_scene_model_data_model();

  register_scene_texture2d_data_model();
  register_scene_sampler_data_model();
  register_scene_texture_cube_data_model();

  register_camera_data_model();

  register_directional_light_data_model();
  register_point_light_data_model();
  register_spot_light_data_model();

  register_std_model_data_model();

  register_attribute_mesh_data_model();
  register_instance_mesh_data_model();

  register_unlit_material_data_model();
  register_pbr_sg_material_data_model();
  register_pbr_mr_material_data_model();
  register_scene_animation_data_model();
  register_scene_skin_data_model();
}

declare_entity!(SceneEntity);

declare_component!(SceneSolidBackground, SceneEntity, Option<Vec3<f32>>);

declare_component!(SceneHDRxEnvBackgroundIntensity, SceneEntity, Option<f32>);
declare_foreign_key!(
  SceneHDRxEnvBackgroundCubeMap,
  SceneEntity,
  SceneTextureCubeEntity
);

pub fn register_scene_self_data_model() {
  global_database()
    .declare_entity::<SceneEntity>()
    .declare_component::<SceneSolidBackground>()
    .declare_component::<SceneHDRxEnvBackgroundIntensity>()
    .declare_foreign_key::<SceneHDRxEnvBackgroundCubeMap>();
}

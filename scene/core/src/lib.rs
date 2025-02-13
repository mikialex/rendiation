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
mod node;
mod reader;
mod texture;
mod writer;

pub use animation::*;
pub use buffer::*;
pub use camera::*;
pub use light::*;
pub use material::*;
pub use mesh::*;
pub use node::*;
pub use reader::*;
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

declare_entity!(SceneModelEntity);
declare_foreign_key!(SceneModelBelongsToScene, SceneModelEntity, SceneEntity);
declare_foreign_key!(SceneModelRefNode, SceneModelEntity, SceneNodeEntity);
declare_foreign_key!(
  SceneModelStdModelRenderPayload,
  SceneModelEntity,
  StandardModelEntity
);
pub fn register_scene_model_data_model() {
  global_database()
    .declare_entity::<SceneModelEntity>()
    .declare_foreign_key::<SceneModelBelongsToScene>()
    .declare_foreign_key::<SceneModelRefNode>()
    .declare_foreign_key::<SceneModelStdModelRenderPayload>();
}

pub struct SceneModelDataView {
  pub model: EntityHandle<StandardModelEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl SceneModelDataView {
  pub fn write(
    &self,
    writer: &mut EntityWriter<SceneModelEntity>,
  ) -> EntityHandle<SceneModelEntity> {
    writer
      .component_value_writer::<SceneModelStdModelRenderPayload>(self.model.some_handle())
      .component_value_writer::<SceneModelBelongsToScene>(self.scene.some_handle())
      .component_value_writer::<SceneModelRefNode>(self.node.some_handle())
      .new_entity()
  }
}

declare_entity!(StandardModelEntity);
declare_foreign_key!(
  StandardModelRefUnlitMaterial,
  StandardModelEntity,
  UnlitMaterialEntity
);
declare_foreign_key!(
  StandardModelRefPbrSGMaterial,
  StandardModelEntity,
  PbrSGMaterialEntity
);
declare_foreign_key!(
  StandardModelRefPbrMRMaterial,
  StandardModelEntity,
  PbrMRMaterialEntity
);
declare_foreign_key!(
  StandardModelRefAttributesMeshEntity,
  StandardModelEntity,
  AttributesMeshEntity
);

pub fn register_std_model_data_model() {
  global_database()
    .declare_entity::<StandardModelEntity>()
    .declare_foreign_key::<StandardModelRefAttributesMeshEntity>()
    .declare_foreign_key::<StandardModelRefUnlitMaterial>()
    .declare_foreign_key::<StandardModelRefPbrSGMaterial>()
    .declare_foreign_key::<StandardModelRefPbrMRMaterial>();
}

pub struct StandardModelDataView {
  pub material: SceneMaterialDataView,
  pub mesh: EntityHandle<AttributesMeshEntity>,
}

impl StandardModelDataView {
  pub fn write(
    self,
    writer: &mut EntityWriter<StandardModelEntity>,
  ) -> EntityHandle<StandardModelEntity> {
    match self.material {
      SceneMaterialDataView::UnlitMaterial(m) => {
        writer.component_value_writer::<StandardModelRefUnlitMaterial>(m.some_handle());
      }
      SceneMaterialDataView::PbrSGMaterial(m) => {
        writer.component_value_writer::<StandardModelRefPbrSGMaterial>(m.some_handle());
      }
      SceneMaterialDataView::PbrMRMaterial(m) => {
        writer.component_value_writer::<StandardModelRefPbrMRMaterial>(m.some_handle());
      }
    }
    writer.component_value_writer::<StandardModelRefAttributesMeshEntity>(self.mesh.some_handle());

    writer.new_entity()
  }
}

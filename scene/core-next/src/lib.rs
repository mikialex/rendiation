use database::*;
use reactive::*;
use rendiation_algebra::*;

mod animation;
mod camera;
mod light;
mod material;
mod mesh;
mod texture;

pub use animation::*;
pub use camera::*;
pub use light::*;
pub use material::*;
pub use mesh::*;
pub use texture::*;

pub fn register_scene_core_data_model() {
  register_scene_self_data_model();
  register_scene_node_data_model();

  register_scene_texture2d_data_model();
  register_scene_sampler_data_model();

  register_camera_data_model();

  register_directional_light_data_model();
  register_point_light_data_model();
  register_spot_light_data_model();

  register_std_model_data_model();

  register_attribute_mesh_data_model();
  register_instance_mesh_data_model();

  register_flat_material_data_model();
  register_pbr_sg_material_data_model();
  register_pbr_mr_material_data_model();
}

declare_entity!(SceneEntity);
declare_component!(SceneSolidBackground, SceneEntity, Option<Vec3<f32>>);

pub fn register_scene_self_data_model() {
  global_database()
    .declare_entity::<SceneEntity>()
    .declare_component::<SceneSolidBackground>();
}

declare_entity!(BufferEntity);
declare_component!(BufferEntityData, BufferEntity, ExternalRefPtr<Vec<u8>>);

declare_entity!(SceneModelEntity);
declare_foreign_key!(SceneModelBelongsToScene, SceneModelEntity, SceneEntity);
declare_foreign_key!(
  SceneModelStdModelRenderPayload,
  SceneModelEntity,
  StandardModelEntity
);

declare_entity!(StandardModelEntity);
declare_foreign_key!(
  StandardModelRefFlatMaterial,
  StandardModelEntity,
  FlatMaterialEntity
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
  StandardModelRefAttributeMesh,
  StandardModelEntity,
  AttributeMeshEntity
);

pub fn register_std_model_data_model() {
  global_database()
    .declare_entity::<StandardModelEntity>()
    .declare_foreign_key::<StandardModelRefAttributeMesh>()
    .declare_foreign_key::<StandardModelRefFlatMaterial>()
    .declare_foreign_key::<StandardModelRefPbrSGMaterial>()
    .declare_foreign_key::<StandardModelRefPbrMRMaterial>();
}

declare_entity!(SceneNodeEntity);
declare_component!(SceneNodeParentIdx, SceneNodeEntity, u32); // should we add generation?
declare_component!(SceneNodeLocalMatrixComponent, SceneNodeEntity, Mat4<f32>);
declare_component!(SceneNodeVisibleComponent, SceneNodeEntity, bool, true);
pub fn register_scene_node_data_model() {
  global_database()
    .declare_entity::<SceneNodeEntity>()
    .declare_component::<SceneNodeParentIdx>()
    .declare_component::<SceneNodeLocalMatrixComponent>()
    .declare_component::<SceneNodeVisibleComponent>();
}

// todo reuse
pub fn scene_node_connectivity() -> Box<dyn ReactiveOneToManyRelationship<u32, u32>> {
  Box::new(
    global_watch()
      .watch::<SceneNodeParentIdx>()
      .into_one_to_many_by_idx(),
  )
}

// todo reuse
pub fn scene_node_derive_visible() -> impl ReactiveCollection<u32, bool> {
  tree_payload_derive_by_parent_decide_children(
    scene_node_connectivity(),
    global_watch()
      .watch::<SceneNodeVisibleComponent>()
      .into_boxed(), // todo avoid extra boxing
    |this, parent| parent.map(|p| *p && *this).unwrap_or(*this),
  )
}

// todo reuse
pub fn scene_node_derive_world_mat() -> impl ReactiveCollection<u32, Mat4<f32>> {
  tree_payload_derive_by_parent_decide_children(
    scene_node_connectivity(),
    global_watch()
      .watch::<SceneNodeLocalMatrixComponent>()
      .into_boxed(), // todo avoid extra boxing
    |this, parent| parent.map(|p| *p * *this).unwrap_or(*this),
  )
}

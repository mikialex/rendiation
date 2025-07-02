use crate::*;

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
declare_foreign_key!(StandardModelRefSkin, StandardModelEntity, SceneSkinEntity);

pub fn register_std_model_data_model() {
  global_database()
    .declare_entity::<StandardModelEntity>()
    .declare_foreign_key::<StandardModelRefAttributesMeshEntity>()
    .declare_foreign_key::<StandardModelRefUnlitMaterial>()
    .declare_foreign_key::<StandardModelRefPbrSGMaterial>()
    .declare_foreign_key::<StandardModelRefPbrMRMaterial>()
    .declare_foreign_key::<StandardModelRefSkin>();
}

pub struct StandardModelDataView {
  pub material: SceneMaterialDataView,
  pub mesh: EntityHandle<AttributesMeshEntity>,
  pub skin: Option<EntityHandle<SceneSkinEntity>>,
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

pub fn scene_model_world_bounding(
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = Box3<f64>> {
  let mesh_local_bounding = attribute_mesh_local_bounding();
  let node_world_mat = scene_node_derive_world_mat();

  let std_mesh_local_bounding = mesh_local_bounding
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<StandardModelRefAttributesMeshEntity>());

  let scene_model_world_mat =
    node_world_mat.one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelRefNode>());

  let scene_model_local_bounding = std_mesh_local_bounding
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>());

  scene_model_world_mat
    .collective_intersect(scene_model_local_bounding)
    .collective_map(|(mat, local)| {
      let f64_box = Box3::new(local.min.map(|v| v as f64), local.max.map(|v| v as f64));
      f64_box.apply_matrix_into(mat)
    })
}

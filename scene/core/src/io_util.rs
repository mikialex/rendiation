use crate::*;

pub struct SceneWriter {
  pub scene: EntityHandle<SceneEntity>,
  pub scene_writer: EntityWriter<SceneEntity>,
  pub camera_writer: EntityWriter<SceneCameraEntity>,
  pub mesh_writer: AttributesMeshEntityFromAttributesMeshWriter,
  pub tex_writer: EntityWriter<SceneTexture2dEntity>,
  pub sampler_writer: EntityWriter<SceneSamplerEntity>,
  pub node_writer: EntityWriter<SceneNodeEntity>,
  pub std_model_writer: EntityWriter<StandardModelEntity>,
  pub model_writer: EntityWriter<SceneModelEntity>,
  pub flat_mat_writer: EntityWriter<FlatMaterialEntity>,
  pub pbr_sg_mat_writer: EntityWriter<PbrSGMaterialEntity>,
  pub pbr_mr_mat_writer: EntityWriter<PbrMRMaterialEntity>,
}

impl SceneWriter {
  pub fn set_solid_background(&mut self, solid: Vec3<f32>) {
    self
      .scene_writer
      .write_component_data::<SceneSolidBackground>(self.scene, Some(solid));
  }

  pub fn create_root_child(&mut self) -> EntityHandle<SceneNodeEntity> {
    self.node_writer.new_entity()
  }
  pub fn create_child(
    &mut self,
    parent: EntityHandle<SceneNodeEntity>,
  ) -> EntityHandle<SceneNodeEntity> {
    let child = self.create_root_child();
    self
      .node_writer
      .write_component_data::<SceneNodeParentIdx>(child, Some(parent.into_raw()));
    child
  }

  pub fn create_scene_model(
    &mut self,
    material: SceneMaterialDataView,
    mesh: EntityHandle<AttributesMeshEntity>,
    node: EntityHandle<SceneNodeEntity>,
  ) -> EntityHandle<SceneModelEntity> {
    let std_model = StandardModelDataView { material, mesh };
    let std_model = std_model.write(&mut self.std_model_writer);
    let sm = SceneModelDataView {
      model: std_model,
      scene: self.scene,
      node,
    };

    sm.write(&mut self.model_writer)
  }

  pub fn set_local_matrix(&mut self, node: EntityHandle<SceneNodeEntity>, mat: Mat4<f32>) {
    self
      .node_writer
      .write_component_data::<SceneNodeLocalMatrixComponent>(node, mat);
  }

  pub fn get_local_mat(&self, node: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f32>> {
    self
      .node_writer
      .read_component_data::<SceneNodeLocalMatrixComponent>(node)
  }

  pub fn from_global(scene: EntityHandle<SceneEntity>) -> Self {
    Self {
      scene,
      scene_writer: global_entity_of().entity_writer(),
      camera_writer: global_entity_of().entity_writer(),
      mesh_writer: AttributesMesh::create_writer(),
      tex_writer: global_entity_of().entity_writer(),
      sampler_writer: global_entity_of().entity_writer(),
      node_writer: global_entity_of().entity_writer(),
      std_model_writer: global_entity_of().entity_writer(),
      model_writer: global_entity_of().entity_writer(),
      flat_mat_writer: global_entity_of().entity_writer(),
      pbr_sg_mat_writer: global_entity_of().entity_writer(),
      pbr_mr_mat_writer: global_entity_of().entity_writer(),
    }
  }
  pub fn write_attribute_mesh(
    &mut self,
    mesh: AttributesMesh,
  ) -> EntityHandle<mesh::AttributesMeshEntity> {
    mesh.write(&mut self.mesh_writer)
  }

  pub fn texture_sample_pair_writer(&mut self) -> TexSamplerWriter {
    TexSamplerWriter {
      tex_writer: &mut self.tex_writer,
      sampler_writer: &mut self.sampler_writer,
    }
  }
}

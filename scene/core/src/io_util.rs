use crate::*;

pub struct Scene3dWriter {
  pub scene: EntityHandle<SceneEntity>,
  pub mesh_writer: AttributeMeshEntityFromAttributeMeshDataWriter,
  pub tex_writer: EntityWriter<SceneTexture2dEntity>,
  pub sampler_writer: EntityWriter<SceneSamplerEntity>,
  pub node_writer: EntityWriter<SceneNodeEntity>,
  pub std_model_writer: EntityWriter<StandardModelEntity>,
  pub model_writer: EntityWriter<SceneModelEntity>,
  pub flat_mat_writer: EntityWriter<FlatMaterialEntity>,
  pub pbr_sg_mat_writer: EntityWriter<PbrSGMaterialEntity>,
  pub pbr_mr_mat_writer: EntityWriter<PbrMRMaterialEntity>,
}

impl Scene3dWriter {
  pub fn from_global(scene: EntityHandle<SceneEntity>) -> Self {
    Self {
      scene,
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
  ) -> EntityHandle<mesh::AttributeMeshEntity> {
    mesh.write(&mut self.mesh_writer)
  }

  pub fn texture_sample_pair_writer(&mut self) -> TexSamplerWriter {
    TexSamplerWriter {
      tex_writer: &mut self.tex_writer,
      sampler_writer: &mut self.sampler_writer,
    }
  }
}

use crate::*;

pub struct SceneWriter {
  pub scene: EntityHandle<SceneEntity>,
  pub scene_writer: EntityWriter<SceneEntity>,
  pub camera_writer: EntityWriter<SceneCameraEntity>,
  pub mesh_writer: AttributesMeshEntityFromAttributesMeshWriter,
  pub tex_writer: EntityWriter<SceneTexture2dEntity>,
  pub buffer_writer: EntityWriter<BufferEntity>,
  pub cube_writer: EntityWriter<SceneTextureCubeEntity>,
  pub sampler_writer: EntityWriter<SceneSamplerEntity>,
  pub node_writer: EntityWriter<SceneNodeEntity>,
  pub std_model_writer: EntityWriter<StandardModelEntity>,
  pub model_writer: EntityWriter<SceneModelEntity>,
  pub unlit_mat_writer: EntityWriter<UnlitMaterialEntity>,
  pub pbr_sg_mat_writer: EntityWriter<PbrSGMaterialEntity>,
  pub pbr_mr_mat_writer: EntityWriter<PbrMRMaterialEntity>,
  pub point_light_writer: EntityWriter<PointLightEntity>,
  pub directional_light_writer: EntityWriter<DirectionalLightEntity>,
  pub spot_light_writer: EntityWriter<SpotLightEntity>,
  pub animation: EntityWriter<SceneAnimationEntity>,
  pub animation_channel: EntityWriter<SceneAnimationChannelEntity>,
}

impl SceneWriter {
  pub fn replace_target_scene(
    &mut self,
    new_scene: EntityHandle<SceneEntity>,
  ) -> EntityHandle<SceneEntity> {
    let scene_backup = self.scene;
    self.scene = new_scene;
    scene_backup
  }
  pub fn write_other_scene<R>(
    &mut self,
    scene: EntityHandle<SceneEntity>,
    f: impl FnOnce(&mut Self) -> R,
  ) -> R {
    let scene_backup = self.replace_target_scene(scene);
    let r = f(self);
    self.scene = scene_backup;
    r
  }

  pub fn reset_background_to_solid(&mut self) {
    self
      .scene_writer
      .write_foreign_key::<SceneHDRxEnvBackgroundCubeMap>(self.scene, None);
    self
      .scene_writer
      .write::<SceneHDRxEnvBackgroundIntensity>(self.scene, None);
  }

  pub fn set_solid_background(&mut self, solid: Vec3<f32>) {
    self.reset_background_to_solid();
    self
      .scene_writer
      .write::<SceneSolidBackground>(self.scene, Some(solid));
  }

  pub fn set_hdr_env_background(
    &mut self,
    cube_map: EntityHandle<SceneTextureCubeEntity>,
    intensity: f32,
  ) {
    self.reset_background_to_solid();
    self
      .scene_writer
      .write_foreign_key::<SceneHDRxEnvBackgroundCubeMap>(self.scene, Some(cube_map));
    self
      .scene_writer
      .write::<SceneHDRxEnvBackgroundIntensity>(self.scene, Some(intensity));
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
      .write::<SceneNodeParentIdx>(child, Some(parent.into_raw()));
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
      .write::<SceneNodeLocalMatrixComponent>(node, mat);
  }

  pub fn get_local_mat(&self, node: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f32>> {
    self
      .node_writer
      .try_read::<SceneNodeLocalMatrixComponent>(node)
  }

  pub fn from_global(scene: EntityHandle<SceneEntity>) -> Self {
    Self {
      scene,
      scene_writer: global_entity_of().entity_writer(),
      camera_writer: global_entity_of().entity_writer(),
      mesh_writer: AttributesMesh::create_writer(),
      tex_writer: global_entity_of().entity_writer(),
      cube_writer: global_entity_of().entity_writer(),
      sampler_writer: global_entity_of().entity_writer(),
      node_writer: global_entity_of().entity_writer(),
      std_model_writer: global_entity_of().entity_writer(),
      model_writer: global_entity_of().entity_writer(),
      unlit_mat_writer: global_entity_of().entity_writer(),
      pbr_sg_mat_writer: global_entity_of().entity_writer(),
      pbr_mr_mat_writer: global_entity_of().entity_writer(),
      point_light_writer: global_entity_of().entity_writer(),
      directional_light_writer: global_entity_of().entity_writer(),
      spot_light_writer: global_entity_of().entity_writer(),
      animation: global_entity_of().entity_writer(),
      animation_channel: global_entity_of().entity_writer(),
      buffer_writer: global_entity_of().entity_writer(),
    }
  }
  pub fn write_attribute_mesh(&mut self, mesh: AttributesMesh) -> AttributesMeshEntities {
    mesh.write(&mut self.mesh_writer, &mut self.buffer_writer)
  }

  pub fn texture_sample_pair_writer(&mut self) -> TexSamplerWriter {
    TexSamplerWriter {
      tex_writer: &mut self.tex_writer,
      sampler_writer: &mut self.sampler_writer,
    }
  }

  pub fn cube_texture_writer(&mut self) -> TexCubeWriter {
    TexCubeWriter {
      tex_writer: &mut self.tex_writer,
      cube_writer: &mut self.cube_writer,
    }
  }
}

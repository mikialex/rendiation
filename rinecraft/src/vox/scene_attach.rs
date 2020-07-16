use super::world::World;
use crate::{
  shading::{create_block_shading, BlockShadingParamGroup},
  util::CameraGPU,
};
use rendiation_scenegraph::*;
use rendiation_webgpu::*;
use std::collections::BTreeMap;

pub struct WorldSceneAttachment {
  pub root_node_index: SceneNodeHandle<WebGPUBackend>,
  pub block_shading: ShadingHandle<WebGPUBackend>,
  pub blocks: BTreeMap<
    (i32, i32),
    (
      SceneNodeHandle<WebGPUBackend>,
      RenderObjectHandle<WebGPUBackend>,
      GeometryHandle<WebGPUBackend>,
    ),
  >,
}

impl WorldSceneAttachment{
  pub fn has_block_attach_to_scene(&self, block_position: (i32, i32)) -> bool {
    self.blocks.contains_key(&block_position)
  }
}

impl World {
  pub fn attach_scene(
    &mut self,
    scene: &mut Scene<WebGPUBackend>,
    renderer: &mut WGPURenderer,
    camera_gpu: &CameraGPU,
    target: &TargetStates,
  ) {
    if self.scene_data.is_some() {
      return;
    }

    let block_atlas = self.chunks.world_machine.get_block_atlas(renderer);
    let sampler = WGPUSampler::default(renderer);

    let shading_params = BlockShadingParamGroup {
      texture_view: &block_atlas.view(),
      sampler: &sampler,
      u_mvp_matrix: scene
        .resources
        .get_uniform(camera_gpu.gpu_mvp_matrix)
        .resource(),
      u_camera_world_position: scene
        .resources
        .get_uniform(camera_gpu.gpu_camera_position)
        .resource(),
    }
    .create_bindgroup(renderer);

    let block_shading = create_block_shading(renderer, target);
    let bindgroup_index = scene
      .resources
      .add_shading_param_group(SceneShadingParameterGroupData::new(
        ParameterGroupTypeId(0),
        shading_params,
      ))
      .index();
    let block_shading = scene
      .resources
      .add_shading(SceneShadingData::new(block_shading).push_parameter(bindgroup_index));
    let block_shading = block_shading.index();

    let root_node_index = scene.create_new_node().handle();
    scene.add_to_scene_root(root_node_index);

    self.scene_data = Some(WorldSceneAttachment {
      root_node_index,
      block_shading,
      blocks: BTreeMap::new(),
    })
  }

  pub fn detach_scene(&mut self) {
    // free the resource in scene
    todo!()
  }
}

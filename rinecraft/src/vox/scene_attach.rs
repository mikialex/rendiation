use super::block_coords::*;
use super::{chunks::WorldChunkData, world::World, world_machine::WorldMachine};
use crate::{
  shading::{BlockShader, BlockShadingParamGroup, CopyParam},
  util::CameraGPU,
};
use rendiation_math::{Vec3, Vec4};
use rendiation_mesh_buffer::{geometry::IndexedGeometry, vertex::Vertex};
use rendiation_scenegraph::*;
use rendiation_shader_library::fog::FogData;
use rendiation_webgpu::*;
use std::{collections::BTreeMap, time::Instant};

pub struct WorldSceneAttachment {
  pub root_node_index: SceneNodeHandle<WebGPU>,
  pub block_shading: ShadingHandle<WebGPU, BlockShader>,
  pub blocks: BTreeMap<
    ChunkCoords,
    (
      SceneNodeHandle<WebGPU>,
      DrawcallHandle<WebGPU>,
      GeometryHandle<WebGPU, Vertex>,
    ),
  >,
}

impl WorldSceneAttachment {
  pub fn has_block_attach_to_scene(&self, block_position: ChunkCoords) -> bool {
    self.blocks.contains_key(&block_position)
  }

  pub fn sync_chunks_in_scene(
    &mut self,
    chunks: &mut WorldChunkData,
    scene: &mut Scene<WebGPU>,
    resources: &mut ResourceManager<WebGPU>,
    renderer: &mut WGPURenderer,
  ) {
    for (chunk, g) in chunks.chunks_to_sync_scene.lock().unwrap().drain() {
      // if chunks.chunks.get(&chunk).is_none() {
      //   return;
      // }

      // remove node in scene;
      if let Some((node_index, drawcall_handle, geometry_index)) = self.blocks.get(&chunk) {
        scene.node_remove_child_by_handle(self.root_node_index, *node_index);
        scene.free_node(*node_index);
        scene.delete_drawcall(*drawcall_handle);
        resources.delete_geometry_with_buffers(*geometry_index);
        self.blocks.remove(&chunk);
      }

      // add new node in scene;
      let scene_geometry = g.create_resource_instance_handle(renderer, resources);

      let drawcall = scene.create_drawcall(scene_geometry, self.block_shading);
      let new_node = scene.create_new_node();
      new_node.data_mut().append_drawcall(drawcall);
      let node_index = new_node.handle();

      scene.node_add_child_by_handle(self.root_node_index, node_index);

      self
        .blocks
        .insert(chunk, (node_index, drawcall, scene_geometry));
    }
  }
}

impl World {
  pub fn attach_scene(
    &mut self,
    scene: &mut Scene<WebGPU>,
    resources: &mut ResourceManager<WebGPU>,
    renderer: &mut WGPURenderer,
    camera_gpu: &CameraGPU,
    target: &TargetStates,
  ) {
    if self.scene_data.is_some() {
      return;
    }

    let block_atlas = self.world_machine.get_block_atlas(renderer);
    let sampler = WGPUSampler::default(renderer);

    let fog = FogData {
      fog_color: Vec4::new(0.1, 0.2, 0.3, 1.0),
      fog_end: 60.,
      fog_start: 30.,
    };
    let fog = resources.bindable.uniform_buffers.add(fog);

    resources.maintain_gpu(renderer);

    let block_atlas = resources.bindable.textures.insert(block_atlas);
    let sampler = resources
      .bindable
      .samplers
      .insert(WGPUSampler::default(renderer));
    let block_shading_pipeline = BlockShader::create_pipeline(renderer);
    let block_shading_pipeline = resources.shading_gpu.insert(block_shading_pipeline);

    let bindgroup_index =
      resources
        .bindgroups
        .add(BlockShadingParamGroup::create_resource_instance(
          camera_gpu.gpu_mvp_matrix,
          fog,
          block_atlas,
          sampler,
        ));

    let block_shading = BlockShader::create_resource_instance(bindgroup_index);
    let block_shading = resources
      .shadings
      .add_shading(block_shading, block_shading_pipeline);

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

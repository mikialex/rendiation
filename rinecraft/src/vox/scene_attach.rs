use super::block_coords::*;
use super::{chunks::WorldChunkData, world::World};
use crate::{
  shading::{create_block_shading, BlockShadingParamGroup},
  util::CameraGPU,
};
use rendiation_mesh_buffer::{geometry::IndexedGeometry, wgpu::as_bytes};
use rendiation_scenegraph::*;
use rendiation_webgpu::*;
use std::collections::BTreeMap;

pub struct WorldSceneAttachment {
  pub root_node_index: SceneNodeHandle<WebGPUBackend>,
  pub block_shading: ShadingHandle<WebGPUBackend>,
  pub blocks: BTreeMap<
    ChunkCoords,
    (
      SceneNodeHandle<WebGPUBackend>,
      RenderObjectHandle<WebGPUBackend>,
      GeometryHandle<WebGPUBackend>,
    ),
  >,
}

impl WorldSceneAttachment {
  pub fn has_block_attach_to_scene(&self, block_position: ChunkCoords) -> bool {
    self.blocks.contains_key(&block_position)
  }

  pub fn sync_chunk_in_scene(
    &mut self,
    chunk: &ChunkCoords,
    chunks: &WorldChunkData,
    scene: &mut Scene<WebGPUBackend>,
    renderer: &mut WGPURenderer,
  ) {
    // remove node in scene;
    if let Some((node_index, render_object_index, geometry_index)) = self.blocks.get(chunk) {
      scene.node_remove_child_by_handle(self.root_node_index, *node_index);
      scene.free_node(*node_index);
      scene.delete_render_object(*render_object_index);
      scene
        .resources
        .delete_geometry_with_buffers(*geometry_index);
      self.blocks.remove(chunk);
    }

    // add new node in scene;
    let mesh_buffer = chunks.create_mesh_buffer(*chunk);
    let scene_geometry = create_add_geometry(&mesh_buffer, renderer, scene);

    let render_object_index = scene.create_render_object(scene_geometry, self.block_shading);
    let new_node = scene.create_new_node();
    new_node.data_mut().add_render_object(render_object_index);
    let node_index = new_node.handle();

    scene.node_add_child_by_handle(self.root_node_index, node_index);

    self
      .blocks
      .insert(*chunk, (node_index, render_object_index, scene_geometry));
  }
}

pub fn create_add_geometry(
  geometry: &IndexedGeometry,
  renderer: &mut WGPURenderer,
  scene: &mut Scene<WebGPUBackend>,
) -> GeometryHandle<WebGPUBackend> {
  let mut geometry_data = SceneGeometryData::new();
  let index_buffer = WGPUBuffer::new(
    renderer,
    as_bytes(&geometry.index),
    wgpu::BufferUsage::INDEX,
  );
  let vertex_buffer = WGPUBuffer::new(
    renderer,
    as_bytes(&geometry.data),
    wgpu::BufferUsage::VERTEX,
  );
  geometry_data.index_buffer = Some(scene.resources.add_index_buffer(index_buffer).index());
  geometry_data.vertex_buffers = vec![(
    AttributeTypeId(0), // todo
    scene.resources.add_vertex_buffer(vertex_buffer).index(),
  )];
  geometry_data.draw_range = 0..geometry.get_full_count();
  scene.resources.add_geometry(geometry_data).index()
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

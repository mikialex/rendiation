use super::block_coords::*;
use super::{chunks::WorldChunkData, world::World, world_machine::WorldMachine};
use crate::{
  shading::{create_block_shading, BlockShadingParamGroup, CopyParam},
  util::CameraGPU,
  BlockShader,
};
use rendiation_math::{Vec3, Vec4};
use rendiation_mesh_buffer::{geometry::IndexedGeometry, wgpu::as_bytes};
use rendiation_scenegraph::*;
use rendiation_shader_library::fog::FogData;
use rendiation_webgpu::*;
use std::{collections::BTreeMap, time::Instant};

pub struct WorldSceneAttachment {
  pub root_node_index: SceneNodeHandle<WGPURenderer>,
  pub block_shading: ShadingHandle<WGPURenderer, BlockShader>,
  pub blocks: BTreeMap<
    ChunkCoords,
    (
      SceneNodeHandle<WGPURenderer>,
      RenderObjectHandle<WGPURenderer>,
      GeometryHandle<WGPURenderer>,
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
    scene: &mut Scene<WGPURenderer>,
    renderer: &mut WGPURenderer,
  ) {
    for (chunk, g) in chunks.chunks_to_sync_scene.lock().unwrap().drain() {
      // if chunks.chunks.get(&chunk).is_none() {
      //   return;
      // }

      // remove node in scene;
      if let Some((node_index, render_object_index, geometry_index)) = self.blocks.get(&chunk) {
        scene.node_remove_child_by_handle(self.root_node_index, *node_index);
        scene.free_node(*node_index);
        scene.delete_render_object(*render_object_index);
        scene
          .resources
          .delete_geometry_with_buffers(*geometry_index);
        self.blocks.remove(&chunk);
      }

      // add new node in scene;
      let scene_geometry = create_add_geometry(&g, renderer, scene);

      let render_object_index = scene.create_render_object(scene_geometry, self.block_shading);
      let new_node = scene.create_new_node();
      new_node.data_mut().add_render_object(render_object_index);
      let node_index = new_node.handle();

      scene.node_add_child_by_handle(self.root_node_index, node_index);

      self
        .blocks
        .insert(chunk, (node_index, render_object_index, scene_geometry));
    }
  }
}

pub fn create_add_geometry(
  geometry: &IndexedGeometry,
  renderer: &mut WGPURenderer,
  scene: &mut Scene<WGPURenderer>,
) -> GeometryHandle<WGPURenderer> {
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
    scene: &mut Scene<WGPURenderer>,
    renderer: &mut WGPURenderer,
    camera_gpu: &CameraGPU,
    target: &TargetStates,
  ) {
    if self.scene_data.is_some() {
      return;
    }

    let resources = &mut scene.resources;

    let block_atlas = self.world_machine.get_block_atlas(renderer);
    let sampler = WGPUSampler::default(renderer);

    let fog = FogData {
      fog_color: Vec4::new(0.1, 0.2, 0.3, 1.0),
      fog_end: 60.,
      fog_start: 30.,
    };
    let fog = resources.add_uniform(fog);

    resources.maintain_gpu(renderer);

    let block_atlas = resources.bindable.textures.insert(block_atlas);
    let sampler = resources
      .bindable
      .samplers
      .insert(WGPUSampler::default(renderer));
    let block_shading = create_block_shading(renderer, target);

    let bindgroup_index =
      resources
        .bindgroups
        .add_bindgroup(BlockShadingParamGroup::create_resource_instance(
          camera_gpu.gpu_mvp_matrix,
          fog,
          block_atlas,
          sampler,
        ));

    let block_shading =
      resources.add_shading(SceneShadingData::new(block_shading).push_parameter(bindgroup_index));
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

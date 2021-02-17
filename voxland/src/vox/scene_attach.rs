use super::{block, block_coords::*};
use super::{chunks::WorldChunkData, world::World, world_machine::WorldMachine};
use crate::{
  camera::VoxlandCamera,
  shading::{BlockShader, BlockShadingParamGroup, CopyParam},
};
use rendiation_algebra::{Vec3, Vec4};
use rendiation_renderable_mesh::{geometry::IndexedGeometry, vertex::Vertex};
use rendiation_scenegraph::*;
use rendiation_shader_library::{fog::FogData, ShaderGraphProvider};
use rendiation_webgpu::*;
use std::{collections::BTreeMap, time::Instant};

pub struct ChunkSceneAttachInfo {
  node: SceneNodeHandle<WebGPU>,
  dc: DrawcallHandle<WebGPU>,
  geom: GeometryHandle<WebGPU, Vertex>,
  bindgroup: BindGroupHandle<WebGPU, BlockShadingParamGroup>,
  shading: ShadingHandle<WebGPU, BlockShader>,
}

impl ChunkSceneAttachInfo {
  pub fn new(
    camera: &VoxlandCamera,
    geom: IndexedGeometry,
    res: &mut ResourceManager<WebGPU>,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WebGPU>,
    att: &WorldSceneAttachment,
  ) -> Self {
    let geom = geom.create_resource_instance_handle(renderer, res);

    let new_node = scene.create_new_node(res);
    let node = new_node.handle();

    let bindgroup = res.add_bindgroup(BlockShadingParamGroup::create_resource_instance(
      camera.gpu_handle(),
      new_node.data().render_data.matrix_data,
      att.fog,
      att.block_texture,
      att.block_sampler,
    ));

    let shading = BlockShader::create_resource_instance(bindgroup);
    let shading = res.shadings.add_shading(shading, renderer);

    let dc = scene.create_drawcall(geom, shading);
    let new_node = scene.get_node_mut(node);
    new_node.data_mut().append_drawcall(dc);

    scene.node_add_child_by_handle(att.root_node_index, node);

    Self {
      node,
      dc,
      geom,
      bindgroup,
      shading,
    }
  }

  pub fn delete(
    self,
    res: &mut ResourceManager<WebGPU>,
    scene: &mut Scene<WebGPU>,
    att: &WorldSceneAttachment,
  ) {
    scene.node_remove_child_by_handle(att.root_node_index, self.node);
    scene.free_node(self.node, res);
    scene.delete_drawcall(self.dc);
    res.delete_bindgroup(self.bindgroup);
    res.shadings.delete_shading(self.shading);
    res.delete_geometry_with_buffers(self.geom);
  }
}

pub struct WorldSceneAttachment {
  pub root_node_index: SceneNodeHandle<WebGPU>,
  pub block_texture: TextureHandle<WebGPU>,
  pub block_sampler: SamplerHandle<WebGPU>,
  pub fog: UniformHandle<WebGPU, FogData>,
  pub blocks: BTreeMap<ChunkCoords, ChunkSceneAttachInfo>,
}

impl WorldSceneAttachment {
  pub fn has_block_attach_to_scene(&self, block_position: ChunkCoords) -> bool {
    self.blocks.contains_key(&block_position)
  }

  pub fn sync_chunks_in_scene(
    &mut self,
    chunks: &mut WorldChunkData,
    scene: &mut Scene<WebGPU>,
    res: &mut ResourceManager<WebGPU>,
    renderer: &mut WGPURenderer,
    camera: &VoxlandCamera,
  ) {
    for (chunk, g) in chunks.chunks_to_sync_scene.drain() {
      if let Some(b) = self.blocks.remove(&chunk) {
        b.delete(res, scene, self)
      }
      self.blocks.insert(
        chunk,
        ChunkSceneAttachInfo::new(camera, g, res, renderer, scene, self),
      );
    }
  }
}

impl World {
  pub fn attach_scene(
    &mut self,
    scene: &mut Scene<WebGPU>,
    resources: &mut ResourceManager<WebGPU>,
    renderer: &mut WGPURenderer,
    camera: &VoxlandCamera,
    target: &TargetStates,
  ) {
    if self.scene_data.is_some() {
      return;
    }

    let block_atlas = self.world_machine.get_block_atlas(renderer, resources);
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

    let root_node_index = scene.create_new_node(resources).handle();
    scene.add_to_scene_root(root_node_index);

    self.scene_data = Some(WorldSceneAttachment {
      root_node_index,
      fog,
      block_sampler: sampler,
      block_texture: block_atlas,
      blocks: BTreeMap::new(),
    })
  }

  pub fn detach_scene(&mut self) {
    // free the resource in scene
    todo!()
  }
}

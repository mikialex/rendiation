use super::block_coords::*;
use super::{
  block::{BLOCK_FACES, BLOCK_WORLD_SIZE},
  chunks::WorldChunkData,
  scene_attach::WorldSceneAttachment,
};
use crate::vox::block::Block;
use crate::vox::block::BlockFace;
use crate::vox::chunk::*;
use crate::{
  shading::{create_block_shading, BlockShadingParamGroup},
  util::CameraGPU,
  vox::world_machine::*,
};
use render_target::TargetStates;
use rendiation_math::*;
use rendiation_mesh_buffer::geometry::IndexedGeometry;
use rendiation_render_entity::{PerspectiveCamera, TransformedObject};
use rendiation_scenegraph::*;
use rendiation_webgpu::*;
use std::collections::HashMap;
use std::collections::{BTreeMap, HashSet};

pub struct World {
  pub chunks: WorldChunkData,
  pub chunk_visible_distance: i32,
  pub chunk_geometry_update_set: HashSet<ChunkCoords>,
  pub scene_data: Option<WorldSceneAttachment>,
}

impl World {
  pub fn new() -> Self {
    World {
      chunk_visible_distance: 4,
      chunk_geometry_update_set: HashSet::new(),
      scene_data: None,
      chunks: WorldChunkData {
        chunks: HashMap::new(),
        world_machine: WorldMachineImpl::new(),
      },
    }
  }

  pub fn assure_chunk_has_generated(
    world_machine: &mut impl WorldMachine,
    chunks: &mut HashMap<ChunkCoords, Chunk>,
    chunk_key: ChunkCoords,
  ) -> bool {
    let mut exist = true;
    chunks.entry(chunk_key).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_key);
      exist = false;
      Chunk::new(chunk_key, world_machine)
    });
    exist
  }

  // create new chunk , remove old chunk
  pub fn update(&mut self, renderer: &mut WGPURenderer, scene: &mut Scene<WebGPUBackend>) {
    let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
    let camera_position = camera.world_matrix.position();

    let ChunkCoords(stand_point_chunk) = ChunkCoords::from_world_position(camera_position);
    let x_low = stand_point_chunk.0 - self.chunk_visible_distance;
    let x_high = stand_point_chunk.0 + self.chunk_visible_distance;
    let z_low = stand_point_chunk.1 - self.chunk_visible_distance;
    let z_high = stand_point_chunk.1 + self.chunk_visible_distance;

    let mut create_list: Vec<ChunkCoords> = Vec::new();
    for x in x_low..x_high {
      for z in z_low..z_high {
        if !self.chunks.assure_chunk_has_generated((x, z).into()) {
          create_list.push((x, z).into());
        }
        if let Some(scene_data) = &mut self.scene_data {
          if !scene_data.has_block_attach_to_scene((x, z).into()) {
            create_list.push((x, z).into());
          }
        }
      }
    }

    // dispatch change to adjacent chunk
    for chunk_key in create_list {
      self.chunk_geometry_update_set.insert(chunk_key);
      self.chunks.assure_chunk_surround_has_generated(chunk_key);
    }

    // sync change to scene
    if let Some(scene_data) = &mut self.scene_data {
      for chunk_to_update in &self.chunk_geometry_update_set {
        scene_data.sync_chunk_in_scene(chunk_to_update, &self.chunks, scene, renderer)
      }
    }
    self.chunk_geometry_update_set.clear();
  }

  pub fn block_face_opposite_position(
    block_position: BlockWorldCoords,
    face: BlockFace,
  ) -> Option<BlockWorldCoords> {
    let mut result = block_position.0;
    match face {
      BlockFace::XZMin => result.y -= 1,
      BlockFace::XZMax => result.y += 1,
      BlockFace::XYMin => result.z -= 1,
      BlockFace::XYMax => result.z += 1,
      BlockFace::YZMin => result.x -= 1,
      BlockFace::YZMax => result.x += 1,
    };

    if result.y < 0 {
      return None;
    }

    if result.y >= CHUNK_HEIGHT as i32 {
      return None;
    }
    Some(BlockWorldCoords(result))
  }
}

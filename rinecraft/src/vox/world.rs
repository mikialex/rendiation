use super::block_coords::*;
use super::{
  block::{BLOCK_FACES, BLOCK_WORLD_SIZE},
  chunks::WorldChunkData,
  io::WorldIOManager,
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
use std::{
  collections::{BTreeMap, HashSet},
  sync::{Arc, Mutex},
};

pub struct World {
  pub io: WorldIOManager,
  pub world_machine: Arc<WorldMachine>,
  pub chunks: WorldChunkData,
  pub chunk_visible_distance: i32,
  pub scene_data: Option<WorldSceneAttachment>,
}

impl World {
  pub fn new() -> Self {
    World {
      io: WorldIOManager::new(),
      world_machine: Arc::new(WorldMachine::new()),
      chunk_visible_distance: 4,
      scene_data: None,
      chunks: WorldChunkData::new(),
    }
  }

  pub fn assure_chunk_has_generated(
    &self,
    chunk_key: ChunkCoords,
    machine: Arc<WorldMachine>,
    should_update_geometry: bool,
  ) {
    // let mut data = self.chunks.lock().unwrap();
    let chunks_clone = self.chunks.chunks.clone();
    let chunks = self.chunks.chunks.lock().unwrap();
    let chunks_in_generating_clone = self.chunks.chunks_in_generating.clone();
    let mut chunks_in_generating = self.chunks.chunks_in_generating.lock().unwrap();
    let chunks_to_sync_scene = self.chunks.chunks_to_sync_scene.clone();

    if !chunks.contains_key(&chunk_key) && !chunks_in_generating.contains(&chunk_key) {
      chunks_in_generating.insert(chunk_key);

      tokio::task::spawn_blocking(move || {
        let chunk = Chunk::new(chunk_key, machine.as_ref());
        let chunks_clone2 = chunks_clone.clone();

        {
          let mut chunks = chunks_clone2.lock().unwrap();
          // println!("{:?}", chunk_key);
          chunks_in_generating_clone
            .lock()
            .unwrap()
            .remove(&chunk_key);
          chunks.insert(chunk_key, chunk);
        }

        if should_update_geometry {
          World::create_chunk_geometry_worker(
            chunks_to_sync_scene,
            chunks_clone2,
            chunk_key,
            machine,
          )
        }
      });
    }
  }

  // pub fn assure_chunk_surround_has_generated(
  //   &mut self,
  //   chunk_key: ChunkCoords,
  //   machine: Arc<WorldMachine>,
  // ) {
  //   self.assure_chunk_has_generated(chunk_key, machine, true);
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMin), machine, false);
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMax), machine, false);
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMin), machine, false);
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMax), machine, false);
  // }

  fn create_chunk_geometry_worker(
    chunks_to_sync_scene: Arc<Mutex<HashMap<ChunkCoords, IndexedGeometry>>>,
    chunks: Arc<Mutex<HashMap<ChunkCoords, Chunk>>>,
    chunk: ChunkCoords,
    machine: Arc<WorldMachine>,
  ) {
    tokio::task::spawn_blocking(move || {
      let g = WorldChunkData::create_mesh_buffer(chunks, chunk, machine.as_ref());
      chunks_to_sync_scene.lock().unwrap().insert(chunk, g);
    });
  }

  // create new chunk, remove old chunk
  pub fn update(&mut self, renderer: &mut WGPURenderer, scene: &mut Scene<WebGPUBackend>) {
    let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
    let camera_position = camera.world_matrix.position();

    let ChunkCoords(stand_point_chunk) = ChunkCoords::from_world_position(camera_position);
    let x_low = stand_point_chunk.0 - self.chunk_visible_distance;
    let x_high = stand_point_chunk.0 + self.chunk_visible_distance;
    let z_low = stand_point_chunk.1 - self.chunk_visible_distance;
    let z_high = stand_point_chunk.1 + self.chunk_visible_distance;

    for x in x_low..x_high {
      for z in z_low..z_high {
        let chunk_key = (x, z).into();
        self.assure_chunk_has_generated(chunk_key, self.world_machine.clone(), true);
      }
    }

    // sync change to scene
    if let Some(scene_data) = &mut self.scene_data {
      scene_data.sync_chunks_in_scene(&mut self.chunks, scene, renderer)
    }
  }
}

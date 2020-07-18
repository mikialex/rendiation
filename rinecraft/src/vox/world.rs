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
  pub chunks: Arc<Mutex<WorldChunkData>>,
  pub chunk_visible_distance: i32,
  pub scene_data: Option<WorldSceneAttachment>,
}

impl World {
  pub fn new() -> Self {
    World {
      io: WorldIOManager::new(),
      world_machine: Arc::new(WorldMachine::new()),
      chunk_visible_distance: 3,
      scene_data: None,
      chunks: Arc::new(Mutex::new(WorldChunkData::new())),
    }
  }

  pub fn assure_chunk_has_generated(&self, chunk_key: ChunkCoords, machine: Arc<WorldMachine>) {
    let mut data = self.chunks.lock().unwrap();
    if !data.chunks.contains_key(&chunk_key) && !data.chunks_in_generating.contains(&chunk_key) {
      data.chunks_in_generating.insert(chunk_key);

      let chunk_c = self.chunks.clone();

      println!("spawn");
      tokio::task::spawn_blocking(move || {
        let chunk = Chunk::new(chunk_key, machine.as_ref());
        {
          let mut chunks = chunk_c.lock().unwrap();
          println!("{:?}", chunk_key);
          chunks.chunks_in_generating.remove(&chunk_key);
          chunks.chunks.insert(chunk_key, chunk);
          chunks.chunks_to_sync_scene.insert(chunk_key);
        }
      });
    }
  }

  // pub fn assure_chunk_surround_has_generated(&mut self, chunk_key: ChunkCoords) {
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMin));
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMax));
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMin));
  //   self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMax));
  // }

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
        self.assure_chunk_has_generated(chunk_key, self.world_machine.clone());
        // self.chunks.assure_chunk_surround_has_generated(chunk_key);
      }
    }

    // sync change to scene
    if let Some(scene_data) = &mut self.scene_data {
      let mut data = self.chunks.lock().unwrap();
      for chunk_to_update in &data.chunks_to_sync_scene {
        scene_data.sync_chunk_in_scene(chunk_to_update, &data, scene, renderer, &self.world_machine)
      }
      data.chunks_to_sync_scene.clear();
    }
  }
}

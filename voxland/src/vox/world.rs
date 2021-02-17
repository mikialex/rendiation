use super::block_coords::*;
use super::{chunks::WorldChunkData, io::WorldIOManager, scene_attach::WorldSceneAttachment};
use crate::vox::block::BlockFace;
use crate::vox::chunk::*;
use crate::vox::world_machine::*;
use crate::{camera::VoxlandCamera, vox::block::Block};
use rendiation_render_entity::{Camera, TransformedObject};
use rendiation_renderable_mesh::geometry::IndexedGeometry;
use rendiation_scenegraph::*;
use rendiation_webgpu::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

  pub async fn assure_chunk_has_generated(
    &self,
    chunk_key: ChunkCoords,
    machine: Arc<WorldMachine>,
  ) {
    if !self.chunks.chunks.lock().unwrap().contains_key(&chunk_key) {
      let chunk = tokio::task::spawn_blocking(move || Chunk::new(chunk_key, machine.as_ref()))
        .await
        .unwrap();
      self.chunks.chunks.lock().unwrap().insert(chunk_key, chunk);
    }
  }

  pub async fn assure_chunk_geometry_has_generated(
    &mut self,
    chunk_key: ChunkCoords,
    machine: Arc<WorldMachine>,
  ) {
    {
      let chunks = self.chunks.chunks.lock().unwrap();
      if let Some(c) = chunks.get(&chunk_key) {
        if c.geometry_generated {
          return;
        }
      }
    }

    use futures::future::join_all;

    let chunk_data = vec![
      self.assure_chunk_has_generated(chunk_key, machine.clone()),
      self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMin), machine.clone()),
      self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::XYMax), machine.clone()),
      self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMin), machine.clone()),
      self.assure_chunk_has_generated(chunk_key.get_side_chunk(ChunkSide::ZYMax), machine.clone()),
    ];
    join_all(chunk_data).await;
    let chunks = self.chunks.chunks.clone();
    let geometry = tokio::task::spawn_blocking(move || {
      WorldChunkData::create_mesh_buffer(chunks, chunk_key, machine.as_ref())
    })
    .await
    .unwrap();
    self.chunks.chunks_to_sync_scene.insert(chunk_key, geometry);
    println!("insert");
    self
      .chunks
      .chunks
      .lock()
      .unwrap()
      .get_mut(&chunk_key)
      .unwrap()
      .geometry_generated = true;
  }

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
  pub async fn update(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WebGPU>,
    resources: &mut ResourceManager<WebGPU>,
    camera: &VoxlandCamera,
  ) {
    let camera_position = camera.camera().matrix().position();

    let ChunkCoords(stand_point_chunk) = ChunkCoords::from_world_position(camera_position);
    let x_low = stand_point_chunk.0 - self.chunk_visible_distance;
    let x_high = stand_point_chunk.0 + self.chunk_visible_distance;
    let z_low = stand_point_chunk.1 - self.chunk_visible_distance;
    let z_high = stand_point_chunk.1 + self.chunk_visible_distance;

    for x in x_low..x_high {
      for z in z_low..z_high {
        let chunk_key = (x, z).into();
        self
          .assure_chunk_geometry_has_generated(chunk_key, self.world_machine.clone())
          .await;
      }
    }

    // sync change to scene
    if let Some(scene_data) = &mut self.scene_data {
      scene_data.sync_chunks_in_scene(&mut self.chunks, scene, resources, renderer, camera)
    }
  }
}

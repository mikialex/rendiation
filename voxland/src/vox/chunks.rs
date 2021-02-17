use super::{
  block::{build_block_face, Block, BlockFace, BLOCK_FACES, BLOCK_WORLD_SIZE},
  block_coords::*,
  chunk::*,
  world_machine::WorldMachine,
};
use rendiation_renderable_mesh::geometry::IndexedGeometry;
use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

pub struct WorldChunkData {
  pub chunks: Arc<Mutex<HashMap<ChunkCoords, Chunk>>>,
  pub chunks_to_sync_scene: HashMap<ChunkCoords, IndexedGeometry>,
}

impl WorldChunkData {
  pub fn new() -> Self {
    Self {
      chunks: Arc::new(Mutex::new(HashMap::new())),
      chunks_to_sync_scene: HashMap::new(),
    }
  }

  pub fn try_get_block(
    chunks: &HashMap<ChunkCoords, Chunk>,
    block_position: BlockWorldCoords,
  ) -> Option<Block> {
    let chunk_position = block_position.to_chunk_coords();
    let chunk_op = chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = block_position.to_local_mod();
      Some(chunk.get_block(chunk_local_position))
    } else {
      None
    }
  }

  pub fn check_block_face_visibility(
    chunks: &HashMap<ChunkCoords, Chunk>,
    block_position: BlockWorldCoords,
    face: BlockFace,
  ) -> bool {
    if let Some(opposite_position) = block_position.face_opposite(face) {
      if let Some(block) = WorldChunkData::try_get_block(chunks, opposite_position) {
        if block.is_void() {
          // this is verbose but clear
          true // surface
        } else {
          false // inner
        }
      } else {
        false // chunk edge
      }
    } else {
      true // top bottom world of world
    }
  }

  pub fn create_mesh_buffer(
    chunks: Arc<Mutex<HashMap<ChunkCoords, Chunk>>>,
    chunk_position: ChunkCoords,
    machine: &WorldMachine,
  ) -> IndexedGeometry {
    let chunks = chunks.lock().unwrap();
    let chunk = chunks.get(&chunk_position).unwrap();

    let mut new_index = Vec::new();
    let mut new_vertex = Vec::new();
    let (world_offset_x, world_offset_z) = chunk_position.world_start();

    for (block, x, y, z) in chunk.iter() {
      if block.is_void() {
        continue;
      }

      let min_x = x as f32 * BLOCK_WORLD_SIZE + world_offset_x;
      let min_y = y as f32 * BLOCK_WORLD_SIZE;
      let min_z = z as f32 * BLOCK_WORLD_SIZE + world_offset_z;

      let max_x = (x + 1) as f32 * BLOCK_WORLD_SIZE + world_offset_x;
      let max_y = (y + 1) as f32 * BLOCK_WORLD_SIZE;
      let max_z = (z + 1) as f32 * BLOCK_WORLD_SIZE + world_offset_z;

      let local: BlockLocalCoords = (x, y, z).into();
      let world_position = local.to_world(chunk_position);
      for face in BLOCK_FACES.iter() {
        if WorldChunkData::check_block_face_visibility(&chunks, world_position, *face) {
          build_block_face(
            machine,
            *block,
            &(min_x, min_y, min_z),
            &(max_x, max_y, max_z),
            *face,
            &mut new_index,
            &mut new_vertex,
          );
        }
      }
    }

    IndexedGeometry::new(new_vertex, new_index)
  }
}

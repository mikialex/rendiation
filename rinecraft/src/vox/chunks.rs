use super::{
  block::{build_block_face, Block, BlockFace, BLOCK_FACES, BLOCK_WORLD_SIZE},
  block_coords::*,
  chunk::*,
  world::World,
  world_machine::WorldMachine,
};
use futures::*;
use rendiation_math::Vec3;
use rendiation_mesh_buffer::geometry::IndexedGeometry;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tokio::prelude::*;

pub struct WorldChunkData {
  pub chunks: HashMap<ChunkCoords, Chunk>,
  pub chunks_in_generating: HashSet<ChunkCoords>,
  pub chunks_to_sync_scene: HashSet<ChunkCoords>,
  // pub chunks_in_updating_geometry: HashSet<ChunkCoords>,
  // pub chunks_to_update_gpu: HashMap<ChunkCoords, IndexedGeometry>,
  pub world_machine: WorldMachine,
}

impl WorldChunkData {
  pub fn new() -> Self {
    Self {
      chunks: HashMap::new(),
      chunks_in_generating: HashSet::new(),
      chunks_to_sync_scene: HashSet::new(),
      // chunks_in_updating_geometry: HashSet::new(),
      // chunks_to_update_gpu: HashMap::new(),
      world_machine: WorldMachine::new(),
    }
  }

  pub fn try_get_block(&self, block_position: BlockWorldCoords) -> Option<Block> {
    let chunk_position = block_position.to_chunk_coords();
    let chunk_op = self.chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = block_position.to_local_mod();
      Some(chunk.get_block(chunk_local_position))
    } else {
      None
    }
  }

  pub fn check_block_face_visibility(
    &self,
    block_position: BlockWorldCoords,
    face: BlockFace,
  ) -> bool {
    if let Some(opposite_position) = block_position.face_opposite(face) {
      if let Some(block) = self.try_get_block(opposite_position) {
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

  pub fn create_mesh_buffer(&self, chunk_position: ChunkCoords) -> IndexedGeometry {
    let chunk = self.chunks.get(&chunk_position).unwrap();

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
        if self.check_block_face_visibility(world_position, *face) {
          build_block_face(
            &self.world_machine,
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

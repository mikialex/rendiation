use super::{
  block::{build_block_face, Block, BlockFace, BLOCK_FACES, BLOCK_WORLD_SIZE},
  chunk::{Chunk, CHUNK_ABS_WIDTH},
  util::{get_local_block_position, local_to_world, query_block_in_chunk},
  world::World,
  world_machine::WorldMachineImpl,
};
use rendiation_math::Vec3;
use rendiation_mesh_buffer::geometry::IndexedGeometry;
use std::collections::HashMap;

pub struct WorldChunkData {
  pub chunks: HashMap<(i32, i32), Chunk>,
  pub world_machine: WorldMachineImpl,
}

impl WorldChunkData {
  pub fn assure_chunk_has_generated(&mut self, chunk_key: (i32, i32)) -> bool {
    let mut exist = true;
    let world_machine = &mut self.world_machine;
    self.chunks.entry(chunk_key).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_key);
      exist = false;
      Chunk::new(chunk_key, world_machine)
    });
    exist
  }

  pub fn try_get_block(&self, block_position: &Vec3<i32>) -> Option<Block> {
    let chunk_position = query_block_in_chunk(block_position);
    let chunk_op = self.chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = get_local_block_position(block_position);
      Some(chunk.get_block(chunk_local_position))
    } else {
      None
    }
  }

  pub fn check_block_face_visibility(&self, block_position: &Vec3<i32>, face: BlockFace) -> bool {
    if let Some(opposite_position) = World::block_face_opposite_position(*block_position, face) {
      if let Some(block) = self.try_get_block(&opposite_position) {
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

  pub fn create_mesh_buffer(&self, chunk_position: (i32, i32)) -> IndexedGeometry {
    let chunk = self.chunks.get(&chunk_position).unwrap();

    let mut new_index = Vec::new();
    let mut new_vertex = Vec::new();
    let world_offset_x = chunk_position.0 as f32 * CHUNK_ABS_WIDTH;
    let world_offset_z = chunk_position.1 as f32 * CHUNK_ABS_WIDTH;

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

      let world_position = local_to_world(&Vec3::new(x, y, z), chunk_position);
      for face in BLOCK_FACES.iter() {
        if self.check_block_face_visibility(&world_position, *face) {
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

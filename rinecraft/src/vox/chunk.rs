use crate::vox::block::*;
use crate::vox::world::*;
use rendiation::*;
use rendiation_math::Vec3;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub const CHUNK_WIDTH: usize = 8;
pub const CHUNK_HEIGHT: usize = 32;

pub const CHUNK_ABS_WIDTH: f32 = (CHUNK_WIDTH as f32) * BLOCK_WORLD_SIZE;

pub type ChunkData = Vec<Vec<Vec<Block>>>;

pub struct Chunk {
  pub chunk_position: (i32, i32),
  data: ChunkData,
  pub geometry: Option<StandardGeometry>,
}

impl Hash for Chunk {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.chunk_position.hash(state);
  }
}

impl PartialEq for Chunk {
  fn eq(&self, other: &Self) -> bool {
    self.chunk_position == other.chunk_position
  }
}

impl Eq for Chunk {}

pub fn world_gen(x: i32, y: i32, z: i32) -> Block {
  if y <= x.abs() && y <= z.abs() {
    Block::Solid {
      style: SolidBlockType::Stone,
    }
  } else {
    Block::Void
  }
}

impl Chunk {
  pub fn new(chunk_id: (i32, i32)) -> Self {
    let chunk_x = chunk_id.0;
    let chunk_z = chunk_id.1;
    let mut x_row = Vec::new();
    for i in 0..CHUNK_WIDTH + 1 {
      let mut y_row = Vec::new();
      for j in 0..CHUNK_WIDTH + 1 {
        let mut z_row = Vec::new();
        for k in 0..CHUNK_HEIGHT + 1 {
          z_row.push(world_gen(
            chunk_x * (CHUNK_WIDTH as i32) + i as i32,
            k as i32,
            chunk_z * (CHUNK_WIDTH as i32) + j as i32,
          ));
        }
        y_row.push(z_row);
      }
      x_row.push(y_row);
    }

    Chunk {
      chunk_position: (chunk_x, chunk_z),
      data: x_row,
      geometry: None,
    }
  }

  // pub fn get_x_positive_side_chunk(
  //   chunks: &mut HashMap<(i32, i32), Chunk>,
  //   chunk_position: (i32, i32),
  // ) -> &mut ChunkData {

  // }

  pub fn get_data(&self) -> &ChunkData {
    &self.data
  }

  pub fn get_data_mut(&mut self) -> &mut ChunkData {
    self.geometry = None;
    &mut self.data
  }

  pub fn get_block(&self, block_local_position: Vec3<i32>) -> &Block {
    &self.data[block_local_position.x as usize][block_local_position.z as usize]
      [block_local_position.y as usize]
  }

  pub fn update_geometry(
    chunks: &mut HashMap<(i32, i32), Chunk>,
    chunk_position: (i32, i32),
    renderer: &mut WGPURenderer,
  ) {
    let chunk = chunks.entry(chunk_position).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_position);
      Chunk::new(chunk_position)
    });
    if chunk.geometry.is_some() {
      return;
    }

    let data = chunk.get_data();

    let mut new_index = Vec::new();
    let mut new_vertex = Vec::new();
    let world_offset_x = chunk_position.0 as f32 * CHUNK_ABS_WIDTH;
    let world_offset_z = chunk_position.1 as f32 * CHUNK_ABS_WIDTH;
    for x in 0..CHUNK_WIDTH + 1 {
      for z in 0..CHUNK_WIDTH + 1 {
        for y in 0..CHUNK_HEIGHT + 1 {
          let block = data[x][z][y];

          if let Block::Void = block {
            continue;
          }

          let min_x = x as f32 * BLOCK_WORLD_SIZE + world_offset_x;
          let min_y = y as f32 * BLOCK_WORLD_SIZE;
          let min_z = z as f32 * BLOCK_WORLD_SIZE + world_offset_z;

          let max_x = (x + 1) as f32 * BLOCK_WORLD_SIZE + world_offset_x;
          let max_y = (y + 1) as f32 * BLOCK_WORLD_SIZE;
          let max_z = (z + 1) as f32 * BLOCK_WORLD_SIZE + world_offset_z;

          for face in BLOCK_FACES.iter() {
            let world_position =
              World::get_block_position(&Vec3::new(x as i32, y as i32, z as i32), chunk_position);
            if World::check_block_face_visibility(chunks, &world_position, *face) {
              build_block_face(
                &(min_x, min_y, min_z),
                &(max_x, max_y, max_z),
                *face,
                &mut new_index,
                &mut new_vertex,
              );
            }
          }
        }
      }
    }

    chunk.geometry = Some(StandardGeometry::new(new_vertex, new_index, renderer));
  }
}

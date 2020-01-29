use crate::vox::block::*;
use rendiation::*;
use std::hash::{Hash, Hasher};

pub const CHUNK_WIDTH: usize = 8;
pub const CHUNK_HEIGHT: usize = 32;

pub struct Chunk {
  pub chunk_position: (i32, i32),
  data: Vec<Vec<Vec<Block>>>,
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
  if y <= x && y <= z {
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

  pub fn get_data(&self) -> &Vec<Vec<Vec<Block>>> {
    &self.data
  }

  pub fn get_data_mut(&mut self) -> &mut Vec<Vec<Vec<Block>>> {
    self.geometry = None;
    &mut self.data
  }

  pub fn create_geometry(
    data: &Vec<Vec<Vec<Block>>>,
    renderer: &mut WGPURenderer,
  ) -> StandardGeometry {
    let mut new_index = Vec::new();
    let mut new_vertex = Vec::new();
    for x in 0..CHUNK_WIDTH + 1 {
      for z in 0..CHUNK_WIDTH + 1 {
        for y in 0..CHUNK_HEIGHT + 1 {
          let block = &data[x][z][y];

          if let Block::Void = block {
            continue;
          }

          let min_x = x as f32 * BLOCK_WORLD_SIZE;
          let min_y = y as f32 * BLOCK_WORLD_SIZE;
          let min_z = z as f32 * BLOCK_WORLD_SIZE;

          let max_x = (x + 1) as f32 * BLOCK_WORLD_SIZE;
          let max_y = (y + 1) as f32 * BLOCK_WORLD_SIZE;
          let max_z = (z + 1) as f32 * BLOCK_WORLD_SIZE;

          for face in BLOCK_FACES.iter() {
            // if self.check_block_face_visibility(*face, (x, z, y)) {
            build_block_face(
              &(min_x, min_y, min_z),
              &(max_x, max_y, max_z),
              *face,
              &mut new_index,
              &mut new_vertex,
            );
            // }
          }
        }
      }
    }

    StandardGeometry::new(new_vertex, new_index, renderer)
  }
}

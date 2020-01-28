use crate::geometry::StandardGeometry;
use crate::vox::block::*;
use rendiation::WGPURenderer;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;

pub struct Chunk {
  pub chunk_position: (i32, i32, i32),
  pub data: Vec<Vec<Vec<Block>>>,
  pub geometry_dirty: bool,
  pub geometry: StandardGeometry,
}

pub fn world_gen(x: i32, y: i32, z: i32) -> Block {
  if y < x && y < z {
    Block::Solid {
      style: SolidBlockType::Stone,
    }
  } else {
    Block::Void
  }
}

impl Chunk {
  pub fn new(renderer: &mut WGPURenderer, chunk_x: i32, chunk_z: i32) -> Self {
    let mut xrow = Vec::new();
    for i in 0..CHUNK_WIDTH + 2 {
      let mut yrow = Vec::new();
      for j in 0..CHUNK_WIDTH + 2 {
        let mut zrow = Vec::new();
        for k in 0..CHUNK_HEIGHT + 2 {
          zrow.push(world_gen(
            chunk_x * (CHUNK_WIDTH as i32) + i as i32 - 1,
            k as i32,
            chunk_z * (CHUNK_WIDTH as i32) + j as i32 - 1,
          ));
        }
        yrow.push(zrow);
      }
      xrow.push(yrow);
    }

    let geometry = Chunk::create_geometry(&xrow, renderer);

    Chunk {
      chunk_position: (chunk_x, chunk_z, 0),
      data: xrow,
      geometry_dirty: false,
      geometry,
    }
  }

  fn create_geometry(data: &Vec<Vec<Vec<Block>>>, renderer: &mut WGPURenderer) -> StandardGeometry {
    let mut new_index = Vec::new();
    let mut new_vertex = Vec::new();
    for x in 1..CHUNK_WIDTH - 1 {
      for z in 1..CHUNK_WIDTH - 1 {
        for y in 1..CHUNK_HEIGHT - 1 {
          let block = &data[x][z][y];
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

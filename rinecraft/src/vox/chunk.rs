use crate::vox::block::*;
use crate::vox::util::local_to_world;
use crate::vox::world::*;
use crate::vox::world_machine::WorldMachine;
use rendiation::*;
use rendiation_math::Vec3;
use rendiation_math_entity::*;
use rendiation_render_entity::BoundingData;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub const CHUNK_WIDTH: usize = 8;
pub const CHUNK_HEIGHT: usize = 64;

pub const CHUNK_ABS_WIDTH: f32 = (CHUNK_WIDTH as f32) * BLOCK_WORLD_SIZE;
pub const CHUNK_ABS_HEIGHT: f32 = (CHUNK_HEIGHT as f32) * BLOCK_WORLD_SIZE;

pub enum ChunkSide {
  XYMin,
  XYMax,
  ZYMin,
  ZYMax,
}

pub type ChunkData = Vec<Vec<Vec<Block>>>;

pub struct Chunk {
  pub chunk_position: (i32, i32),
  pub data: ChunkData,
  pub geometry: Option<StandardGeometry>,
  pub bounding: BoundingData,
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

impl Chunk {
  pub fn new(chunk_id: (i32, i32), world_machine: &impl WorldMachine) -> Self {
    let chunk_x = chunk_id.0;
    let chunk_z = chunk_id.1;
    let mut x_row = Vec::new();
    for i in 0..CHUNK_WIDTH {
      let mut y_row = Vec::new();
      for j in 0..CHUNK_WIDTH {
        let mut z_row = Vec::new();
        for k in 0..CHUNK_HEIGHT {
          z_row.push(world_machine.world_gen(
            chunk_x * (CHUNK_WIDTH as i32) + i as i32,
            k as i32,
            chunk_z * (CHUNK_WIDTH as i32) + j as i32,
          ));
        }
        y_row.push(z_row);
      }
      x_row.push(y_row);
    }

    let min = Vec3::new(
      chunk_x as f32 * CHUNK_ABS_WIDTH,
      0.,
      chunk_z as f32 * CHUNK_ABS_WIDTH,
    );
    let max = Vec3::new(
      (chunk_x + 1) as f32 * CHUNK_ABS_WIDTH,
      CHUNK_ABS_HEIGHT,
      (chunk_z + 1) as f32 * CHUNK_ABS_WIDTH,
    );
    let bounding = BoundingData::new_from_box(Box3::new(min, max));

    Chunk {
      chunk_position: (chunk_x, chunk_z),
      data: x_row,
      geometry: None,
      bounding,
    }
  }

  pub fn get_block(&self, block_local_position: Vec3<usize>) -> Block {
    self.data[block_local_position.x][block_local_position.z][block_local_position.y]
  }

  pub fn set_block(&mut self, block_local_position: Vec3<usize>, block: Block) {
    self.data[block_local_position.x][block_local_position.z][block_local_position.y] = block;
  }

  pub fn create_geometry(
    world_machine: &impl WorldMachine,
    chunks: &HashMap<(i32, i32), Chunk>,
    chunk_position: (i32, i32),
    renderer: &mut WGPURenderer,
  ) -> StandardGeometry {
    let chunk = chunks.get(&chunk_position).unwrap();

    // let data = &chunk.data;

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
        if World::check_block_face_visibility(chunks, &world_position, *face) {
          build_block_face(
            world_machine,
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

    let mut geom = StandardGeometry::new(new_vertex, new_index);
    geom.update_gpu(renderer);
    geom
  }

  pub fn iter<'a>(&'a self) -> ChunkDataIterator<'a> {
    ChunkDataIterator {
      chunk: self,
      position: (0, 0, 0),
      over: false,
    }
  }
}

pub struct ChunkDataIterator<'a> {
  chunk: &'a Chunk,
  position: (usize, usize, usize),
  over: bool,
}

impl<'a> ChunkDataIterator<'a> {
  fn step_position(&mut self) {
    self.position.2 += 1;
    if self.position.2 == CHUNK_HEIGHT {
      self.position.2 = 0;
      self.position.1 += 1;
    }
    if self.position.1 == CHUNK_WIDTH {
      self.position.1 = 0;
      self.position.0 += 1;
    }
    if self.position.0 == CHUNK_WIDTH {
      self.over = true
    }
  }
}

impl<'a> Iterator for ChunkDataIterator<'a> {
  type Item = (&'a Block, usize, usize, usize);

  fn next(&mut self) -> Option<(&'a Block, usize, usize, usize)> {
    if self.over {
      return None;
    }
    let result = Some((
      &self.chunk.data[self.position.0][self.position.1][self.position.2],
      self.position.0,
      self.position.2,
      self.position.1,
    ));
    self.step_position();
    result
  }
}

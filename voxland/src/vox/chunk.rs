use super::{block_coords::*, world_machine::WorldMachine};
use crate::vox::block::*;
use rendiation_algebra::Vec3;
use rendiation_geometry::*;
use rendiation_render_entity::BoundingInfo;
use std::{
  future::Future,
  hash::{Hash, Hasher},
  sync::Arc,
};

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
  pub geometry_generated: bool,

  pub chunk_position: ChunkCoords,
  pub data: ChunkData,
  pub bounding: BoundingInfo,
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
  pub fn new(chunk_position: ChunkCoords, world_machine: &WorldMachine) -> Self {
    let ChunkCoords((chunk_x, chunk_z)) = chunk_position;
    let mut x_row = Vec::new();
    for i in 0..CHUNK_WIDTH {
      let mut y_row = Vec::new();
      for j in 0..CHUNK_WIDTH {
        let mut z_row = Vec::new();
        let x = chunk_x * (CHUNK_WIDTH as i32) + i as i32;
        let z = chunk_z * (CHUNK_WIDTH as i32) + j as i32;
        let level_cache = world_machine.create_chunk_level_cache((x, z));
        for y in 0..CHUNK_HEIGHT {
          z_row.push(world_machine.world_gen(x, y as i32, z, &level_cache));
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
    let bounding = BoundingInfo::new_from_box(Box3::new3(min, max));

    Chunk {
      geometry_generated: false,
      chunk_position,
      data: x_row,
      bounding,
    }
  }

  pub fn get_block(&self, block_local_position: BlockLocalCoords) -> Block {
    let block_local_position = block_local_position.0;
    self.data[block_local_position.x][block_local_position.z][block_local_position.y]
  }

  pub fn set_block(&mut self, block_local_position: BlockLocalCoords, block: Block) {
    let block_local_position = block_local_position.0;
    self.data[block_local_position.x][block_local_position.z][block_local_position.y] = block;
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

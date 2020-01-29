use crate::vox::chunk::CHUNK_WIDTH;
use crate::vox::block::Block;
use crate::vox::block::BlockFace;
use crate::vox::chunk::Chunk;
use crate::vox::chunk::CHUNK_ABS_WIDTH;
use rendiation::*;
use rendiation_math::*;
use rendiation_math_entity::*;
use std::collections::HashMap;

pub struct World {
  chunk_visible_distance: usize,
  chunks: HashMap<(i32, i32), Chunk>,
}

impl World {
  pub fn new() -> Self {
    let mut chunks = HashMap::new();
    chunks.insert((0, 0), Chunk::new((0, 0)));
    World {
      chunk_visible_distance: 2,
      chunks,
    }
  }

  pub fn update(&mut self, renderer: &mut WGPURenderer, view_position: &Vec3<f32>) {
    let stand_point_chunk = World::query_point_in_chunk(view_position);

    Chunk::update_geometry(&mut self.chunks, stand_point_chunk, renderer);
  }

  pub fn query_point_in_chunk(point: &Vec3<f32>) -> (i32, i32) {
    let x = (point.x / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (point.z / CHUNK_ABS_WIDTH).floor() as i32;
    (x, z)
  }
  pub fn query_block_in_chunk(block_position: &Vec3<i32>) -> (i32, i32) {
    let x = (block_position.x as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (block_position.z as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    (x, z)
  }
  pub fn get_local_block_position(block_position: &Vec3<i32>) -> Vec3<i32> {
    Vec3::new(
      block_position.x % CHUNK_WIDTH as i32,
      block_position.y,
      block_position.z % CHUNK_WIDTH as i32,
    )
  }
  pub fn get_block<'a>(
    chunks: &'a mut HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
  ) -> &'a Block {
    let chunk_position = World::query_block_in_chunk(block_position);
    let chunk = World::get_chunk_or_create(chunks, chunk_position);
    let chunk_local_position = World::get_local_block_position(block_position);
    chunk.get_block(chunk_local_position)
  }
  pub fn get_chunk_or_create(
    chunks: &mut HashMap<(i32, i32), Chunk>,
    chunk_position: (i32, i32)
  ) -> &mut Chunk{
    let chunk_to_update = chunks.entry(chunk_position).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_position);
      Chunk::new(chunk_position)
    });
    chunk_to_update
  }


  pub fn render(&self, pass: &mut WGPURenderPass) {
    for (_key, chunk) in &self.chunks {
      if let Some(geometry) = &chunk.geometry {
        geometry.render(pass);
      }
    }
  }

  pub fn pick_block(&self, ray: &Ray) -> BlockPickResult {
    todo!()
  }

  pub fn block_face_opposite(&self, block_position: Vec3<i32>, face: BlockFace) -> Vec3<i32> {
    todo!()
  }
}

pub struct BlockPickResult {
  world_position: Vec3<f32>,
  block_position: Vec3<i32>,
  face: BlockFace,
}

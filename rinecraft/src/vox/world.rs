use crate::vox::block::BlockFace;
use crate::vox::block::BLOCK_WORLD_SIZE;
use crate::vox::chunk::Chunk;
use crate::vox::chunk::CHUNK_WIDTH;
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
    let chunk_to_update = self
      .chunks
      .entry(stand_point_chunk)
      .or_insert_with(|| Chunk::new(stand_point_chunk));

    if chunk_to_update.geometry.is_none() {
      chunk_to_update.geometry = Some(Chunk::create_geometry(chunk_to_update.get_data(), renderer));
    }
  }

  pub fn query_point_in_chunk(point: &Vec3<f32>) -> (i32, i32) {
    let chunk_abs_width = (CHUNK_WIDTH as f32) * BLOCK_WORLD_SIZE;
    let x = (point.x / chunk_abs_width).floor() as i32;
    let z = (point.z / chunk_abs_width).floor() as i32;
    (x, z)
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

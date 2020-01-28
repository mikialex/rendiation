use crate::vox::block::BlockFace;
use rendiation_math_entity::*;
use rendiation_math::*;
use rendiation::*;
use crate::vox::chunk::Chunk;

pub struct World {
  chunk_visible_distance: usize,
  chunks: Vec<Chunk>,
}

impl World{
  pub fn new() -> Self {
    World{
      chunk_visible_distance: 2,
      chunks: vec![Chunk::new(0, 0)],
    }
  }

  pub fn update(&mut self, renderer: &mut WGPURenderer, view_position: Vec3<f32>){

  }

  pub fn render(&self, pass: &mut WGPURenderPass) {
    for chunk in &self.chunks {
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


pub struct BlockPickResult{
  world_position: Vec3<f32>,
  block_position: Vec3<i32>,
  face: BlockFace,
}
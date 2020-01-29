use crate::vox::block::Block;
use crate::vox::block::BlockFace;
use crate::vox::chunk::Chunk;
use crate::vox::chunk::CHUNK_ABS_WIDTH;
use crate::vox::chunk::CHUNK_HEIGHT;
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
    self.chunks.entry(stand_point_chunk).or_insert_with(|| {
      println!("chunk generate {:?}", stand_point_chunk);
      Chunk::new(stand_point_chunk)
    });
    if let Some(geometry) = Chunk::create_geometry(&self.chunks, stand_point_chunk, renderer) {
      self.chunks.get_mut(&stand_point_chunk).unwrap().geometry = Some(geometry);
    }
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
    let x = if block_position.x % CHUNK_WIDTH as i32 > 0 {
      block_position.x % CHUNK_WIDTH as i32
    } else {
      block_position.x % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
    };

    let z = if block_position.z % CHUNK_WIDTH as i32 > 0 {
      block_position.z % CHUNK_WIDTH as i32
    } else {
      block_position.z % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
    };

    assert!(x >= 0);
    assert!(z >= 0);

    Vec3::new(x, block_position.y, z)
  }

  pub fn get_block_position(
    local_block_position: &Vec3<i32>,
    chunk_position: (i32, i32),
  ) -> Vec3<i32> {
    Vec3::new(
      local_block_position.x + chunk_position.0 * CHUNK_WIDTH as i32,
      local_block_position.y,
      local_block_position.z + chunk_position.1 * CHUNK_WIDTH as i32,
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

  pub fn try_get_block<'a>(
    chunks: &'a HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
  ) -> Option<&'a Block> {
    let chunk_position = World::query_block_in_chunk(block_position);
    let chunk_op = chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = World::get_local_block_position(block_position);
      Some(chunk.get_block(chunk_local_position))
    } else {
      None
    }
  }

  pub fn check_block_face_visibility(
    chunks: &HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
    face: BlockFace,
  ) -> bool {
    if let Some(opposite_position) = World::block_face_opposite_position(*block_position, face) {
      if let Some(block) = World::try_get_block(chunks, &opposite_position) {
        if let Block::Void = block {
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

  pub fn get_chunk_or_create(
    chunks: &mut HashMap<(i32, i32), Chunk>,
    chunk_position: (i32, i32),
  ) -> &mut Chunk {
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

  pub fn pick_block(&self, ray: &Ray) -> Option<BlockPickResult> {
    let mut nearest: Option<BlockPickResult> = None;
    for (_, chunk) in &self.chunks {
      if let Some(hit) = chunk.pick_block(ray) {
        if let Some(n) = &nearest {
          if hit.distance < n.distance {
            nearest = Some(hit)
          }
        } else {
          nearest = Some(hit)
        }
      }
    }
    nearest
  }

  pub fn block_face_opposite_position(
    block_position: Vec3<i32>,
    face: BlockFace,
  ) -> Option<Vec3<i32>> {
    let mut result = block_position;
    match face {
      BlockFace::XZMin => result.y -= 1,
      BlockFace::XZMax => result.y += 1,
      BlockFace::XYMin => result.z -= 1,
      BlockFace::XYMax => result.z += 1,
      BlockFace::YZMin => result.x -= 1,
      BlockFace::YZMax => result.x += 1,
    };

    if result.y < 0 {
      return None;
    }

    if result.y >= CHUNK_HEIGHT as i32 {
      return None;
    }
    Some(result)
  }
}

#[derive(Debug)]
pub struct BlockPickResult {
  world_position: Vec3<f32>,
  block_position: Vec3<i32>,
  face: BlockFace,
  distance: f32,
}

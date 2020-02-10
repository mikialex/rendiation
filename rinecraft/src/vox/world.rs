use crate::vox::block::{Block, BlockFace};
use crate::vox::chunk::*;
use crate::vox::util::*;
use rendiation::*;
use rendiation_math::*;
use std::collections::HashMap;
use std::collections::HashSet;

pub struct World {
  pub chunk_visible_distance: usize,
  pub chunks: HashMap<(i32, i32), Chunk>,
  pub chunk_geometry_update_set: HashSet<(i32, i32)>,
}

impl World {
  pub fn new() -> Self {
    let chunks = HashMap::new();
    World {
      chunk_visible_distance: 2,
      chunks,
      chunk_geometry_update_set: HashSet::new(),
    }
  }

  pub fn assure_chunk(chunks: &mut HashMap<(i32, i32), Chunk>, chunk_key: (i32, i32)) -> bool {
    let mut exist = true;
    chunks.entry(chunk_key).or_insert_with(|| {
      println!("chunk generate {:?}", chunk_key);
      exist = false;
      Chunk::new(chunk_key)
    });
    exist
  }

  pub fn update(&mut self, renderer: &mut WGPURenderer, view_position: &Vec3<f32>) {
    let stand_point_chunk = query_point_in_chunk(view_position);
    let x_low = stand_point_chunk.0 - self.chunk_visible_distance as i32;
    let x_high = stand_point_chunk.0 + self.chunk_visible_distance as i32;
    let z_low = stand_point_chunk.1 - self.chunk_visible_distance as i32;
    let z_high = stand_point_chunk.1 + self.chunk_visible_distance as i32;
    let mut create_list = Vec::new();
    for x in x_low..x_high {
      for z in z_low..z_high {
        if !World::assure_chunk(&mut self.chunks, (x, z)) {
          create_list.push((x, z));
        }
        if self.chunks.get(&(x, z)).unwrap().geometry.is_none() {
          create_list.push((x, z));
        }
      }
    }

    for chunk_key in create_list {
      self.chunk_geometry_update_set.insert(chunk_key);
      World::assure_chunk(&mut self.chunks, (chunk_key.0 + 1, chunk_key.1));
      World::assure_chunk(&mut self.chunks, (chunk_key.0 - 1, chunk_key.1));
      World::assure_chunk(&mut self.chunks, (chunk_key.0, chunk_key.1 + 1));
      World::assure_chunk(&mut self.chunks, (chunk_key.0, chunk_key.1 - 1));
    }

    for chunk_to_update_key in &self.chunk_geometry_update_set {
      if let Some(geometry) = Chunk::create_geometry(&self.chunks, *chunk_to_update_key, renderer) {
        self.chunks.get_mut(&chunk_to_update_key).unwrap().geometry = Some(geometry);
      }
    }
    self.chunk_geometry_update_set.clear();
  }

  pub fn try_get_block<'a>(
    chunks: &'a HashMap<(i32, i32), Chunk>,
    block_position: &Vec3<i32>,
  ) -> Option<&'a Block> {
    let chunk_position = query_block_in_chunk(block_position);
    let chunk_op = chunks.get(&chunk_position);
    if let Some(chunk) = chunk_op {
      let chunk_local_position = get_local_block_position(block_position);
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

  pub fn render(&self, pass: &mut WGPURenderPass) {
    for (_key, chunk) in &self.chunks {
      if let Some(geometry) = &chunk.geometry {
        geometry.render(pass);
      }
    }
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

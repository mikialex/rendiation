use crate::vox::chunk::*;
use rendiation_math::Vec3;

pub fn query_point_in_chunk(point: Vec3<f32>) -> (i32, i32) {
  let x = (point.x / CHUNK_ABS_WIDTH).floor() as i32;
  let z = (point.z / CHUNK_ABS_WIDTH).floor() as i32;
  (x, z)
}

pub fn world_to_local(block_position: &Vec3<i32>) -> ((i32, i32), Vec3<usize>) {
  (
    query_block_in_chunk(block_position),
    get_local_block_position(block_position),
  )
}

pub fn query_block_in_chunk(block_position: &Vec3<i32>) -> (i32, i32) {
  let x = (block_position.x as f32 / CHUNK_ABS_WIDTH).floor() as i32;
  let z = (block_position.z as f32 / CHUNK_ABS_WIDTH).floor() as i32;
  (x, z)
}

pub fn get_local_block_position(block_position: &Vec3<i32>) -> Vec3<usize> {
  let x = if block_position.x % CHUNK_WIDTH as i32 >= 0 {
    block_position.x % CHUNK_WIDTH as i32
  } else {
    block_position.x % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
  };

  let z = if block_position.z % CHUNK_WIDTH as i32 >= 0 {
    block_position.z % CHUNK_WIDTH as i32
  } else {
    block_position.z % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
  };

  assert!(x >= 0);
  assert!(z >= 0);

  Vec3::new(x as usize, block_position.y as usize, z as usize)
}

pub fn local_to_world(local_block_position: &Vec3<usize>, chunk_position: (i32, i32)) -> Vec3<i32> {
  Vec3::new(
    local_block_position.x as i32 + chunk_position.0 * CHUNK_WIDTH as i32,
    local_block_position.y as i32,
    local_block_position.z as i32 + chunk_position.1 * CHUNK_WIDTH as i32,
  )
}

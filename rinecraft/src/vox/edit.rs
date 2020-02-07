use crate::vox::block::*;
use crate::vox::chunk::*;
use crate::vox::world::*;
use rendiation_math::Vec3;
use rendiation_math_entity::Ray;
use rendiation_math_entity::*;

#[derive(Debug)]
pub struct BlockPickResult {
  pub world_position: Vec3<f32>,
  pub block_position: Vec3<i32>,
  pub face: BlockFace,
  pub distance2: f32,
}

fn pick_block(chunk: &Chunk, ray: &Ray, previous_result: &Option<BlockPickResult>)
 -> Option<BlockPickResult> {
  if chunk.bounding.if_intersect_ray(ray) {
    let mut closest: Option<BlockPickResult> = None;
    for x in 0..CHUNK_WIDTH {
      for z in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_HEIGHT {
          let block = chunk.data[x][z][y];

          if let Block::Void = block {
            continue;
          }

          let local_position = Vec3::new(x, y, z);
          let world_position = World::get_block_position(&local_position, chunk.chunk_position);

          let min = Vec3::new(
            world_position.x as f32 * BLOCK_WORLD_SIZE,
            world_position.y as f32 * BLOCK_WORLD_SIZE,
            world_position.z as f32 * BLOCK_WORLD_SIZE,
          );
          let max = Vec3::new(
            (world_position.x + 1) as f32 * BLOCK_WORLD_SIZE,
            (world_position.y + 1) as f32 * BLOCK_WORLD_SIZE,
            (world_position.z + 1) as f32 * BLOCK_WORLD_SIZE,
          );

          let box3 = Box3::new(min, max);
          let hit = ray.intersect(&box3);
          if let Some(h) = hit {
            let length2 = (h - ray.origin).length2();
            if let Some(clo) = &closest {
              if length2 < clo.distance2 {
                closest = Some(BlockPickResult {
                  world_position: h,
                  block_position: Vec3::new(0, 0, 0),
                  face: BlockFace::XYMax,
                  distance2: length2,
                })
              }
            } else {
              closest = Some(BlockPickResult {
                world_position: h,
                block_position: Vec3::new(0, 0, 0),
                face: BlockFace::XYMax,
                distance2: length2,
              })
            }
          }
        }
      }
    }
    closest
  } else {
    None
  }
}


impl World {
  pub fn pick_block(&self, ray: &Ray) -> Option<BlockPickResult> {
    let mut nearest: Option<BlockPickResult> = None;
    let mut hit_count = 0;
    for (_, chunk) in &self.chunks {
      if let Some(hit) = pick_block(chunk, ray, &nearest) {
        hit_count += 1;
        if let Some(n) = &nearest {
          if hit.distance2 < n.distance2 {
            nearest = Some(hit)
          }
        } else {
          nearest = Some(hit)
        }
      }
    }
    println!("chunk hit {}", hit_count);
    nearest
  }

  pub fn add_block(&mut self, block_position: &Vec3<i32>, block: Block) {}

  pub fn delete_block(&mut self, block_position: &Vec3<i32>) {}

  pub fn add_block_by_ray(&mut self, ray: &Ray, block: Block) {
    let pick_result = self.pick_block(ray);
  }

  pub fn delete_block_by_ray(&mut self, ray: &Ray) {
    let pick_result = self.pick_block(ray);
  }
}

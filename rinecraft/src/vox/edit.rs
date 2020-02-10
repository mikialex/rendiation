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

fn get_block_bbox(world_position: Vec3<i32>) -> Box3 {
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
  Box3::new(min, max)
}

// todo optimize
fn pick_block(
  chunk: &Chunk,
  ray: &Ray,
  previous_result: &Option<BlockPickResult>,
) -> Option<BlockPickResult> {
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
          let world_position = World::local_to_world(&local_position, chunk.chunk_position);
          let box3 = get_block_bbox(world_position);
          let hit = ray.intersect(&box3);
          if let Some(h) = hit {
            let length2 = (h - ray.origin).length2();
            if let Some(clo) = &closest {
              if length2 < clo.distance2 {
                closest = Some(BlockPickResult {
                  world_position: h,
                  block_position: world_position,
                  face: BlockFace::XYMax, // do face decide later
                  distance2: length2,
                })
              }
            } else {
              closest = Some(BlockPickResult {
                world_position: h,
                block_position: world_position,
                face: BlockFace::XYMax,
                distance2: length2,
              })
            }
          }
        }
      }
    }

    const E: f32 = 0.0001;
    // face decide
    if let Some(r) = &mut closest {
      let box3 = get_block_bbox(r.block_position);
      if (box3.max.x - r.world_position.x).abs() < E {
        r.face = BlockFace::YZMax;
      } else if (box3.min.x - r.world_position.x).abs() < E {
        r.face = BlockFace::YZMin;
      } else if (box3.max.y - r.world_position.y).abs() < E {
        r.face = BlockFace::XZMax;
      } else if (box3.min.y - r.world_position.y).abs() < E {
        r.face = BlockFace::XZMin;
      } else if (box3.max.z - r.world_position.z).abs() < E {
        r.face = BlockFace::XYMax;
      } else if (box3.min.z - r.world_position.z).abs() < E {
        r.face = BlockFace::XYMin;
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
    for (_, chunk) in &self.chunks {
      if let Some(hit) = pick_block(chunk, ray, &nearest) {
        if let Some(n) = &nearest {
          if hit.distance2 < n.distance2 {
            nearest = Some(hit)
          }
        } else {
          nearest = Some(hit)
        }
      }
    }
    println!("chunk hit {:?}", nearest);
    nearest
  }

  pub fn add_block(&mut self, block_position: &Vec3<i32>, block: Block) {
    let (chunk_key, local_position) = World::world_to_local(block_position);
    let chunk = self.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, block);
    chunk.geometry = None;
    self.chunk_geometry_update_set.insert(chunk_key);
  }

  pub fn delete_block(&mut self, block_position: &Vec3<i32>) {
    let (chunk_key, local_position) = World::world_to_local(block_position);

    let chunk = self.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, Block::Void);
    chunk.geometry = None;
    self.chunk_geometry_update_set.insert(chunk_key);
  }

  pub fn add_block_by_ray(&mut self, ray: &Ray, block: Block) {
    let pick_result = self.pick_block(ray);
    if let Some(re) = pick_result {
      if let Some(b) = &World::block_face_opposite_position(re.block_position, re.face) {
        self.add_block(
          b,
          Block::Solid {
            style: SolidBlockType::Stone,
          },
        );
      }
    }
  }

  pub fn delete_block_by_ray(&mut self, ray: &Ray) {
    let pick_result = self.pick_block(ray);
    if let Some(re) = pick_result {
      self.delete_block(&re.block_position);
    }
  }
}

use crate::vox::world_machine::STONE;
use crate::vox::block::*;
use crate::vox::chunk::*;
use crate::vox::util::*;
use crate::vox::world::*;
use rendiation_math::Vec3;
use rendiation_math_entity::Ray;
use rendiation_math_entity::*;
use std::collections::HashSet;
use super::world_machine::VOID;

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

          if block.is_void() {
            continue;
          }

          let local_position = Vec3::new(x, y, z);
          let world_position = local_to_world(&local_position, chunk.chunk_position);
          let box3 = get_block_bbox(world_position);
          let hit = ray.intersect(&box3, &());
          if let NearestPoint3D(Some(h)) = hit {
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

  fn notify_side_chunk_dirty(update_set: &mut HashSet<(i32, i32)>, chunk_key: (i32, i32), point: &Vec3<usize>) {
    fn get_side_affect_chunk(side: ChunkSide, chunk: (i32, i32)) -> (i32, i32) {
      match side {
        ChunkSide::XYMax => (chunk.0 + 1, chunk.1),
        ChunkSide::XYMin => (chunk.0 - 1, chunk.1),
        ChunkSide::ZYMax => (chunk.0, chunk.1 + 1),
        ChunkSide::ZYMin => (chunk.0, chunk.1 - 1),
      }
    }
    println!("{:?}", point);
    if point.x == 0 {
      update_set.insert(get_side_affect_chunk(ChunkSide::XYMin, chunk_key));
      println!("{:?}", get_side_affect_chunk(ChunkSide::XYMin, chunk_key));
    } else if point.x == CHUNK_WIDTH -1 {
      update_set.insert(get_side_affect_chunk(ChunkSide::XYMax, chunk_key));
      println!("{:?}", get_side_affect_chunk(ChunkSide::XYMax, chunk_key));
    } 
    
    if point.z == 0 {
      update_set.insert(get_side_affect_chunk(ChunkSide::ZYMin, chunk_key));
      println!("{:?}", get_side_affect_chunk(ChunkSide::ZYMin, chunk_key));
    } else if point.z == CHUNK_WIDTH -1 {
      update_set.insert(get_side_affect_chunk(ChunkSide::ZYMax, chunk_key));
      println!("{:?}", get_side_affect_chunk(ChunkSide::ZYMax, chunk_key));
    }
  }

  pub fn add_block(&mut self, block_position: &Vec3<i32>, block: Block) {
    let (chunk_key, local_position) = world_to_local(block_position);
    let chunk = self.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, block);
    
    self.chunk_geometry_update_set.insert(chunk_key);
    World::notify_side_chunk_dirty(&mut self.chunk_geometry_update_set, chunk_key, &local_position);
  }

  pub fn delete_block(&mut self, block_position: &Vec3<i32>) {
    let (chunk_key, local_position) = world_to_local(block_position);

    let chunk = self.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, VOID);

    self.chunk_geometry_update_set.insert(chunk_key);
    World::notify_side_chunk_dirty(&mut self.chunk_geometry_update_set, chunk_key, &local_position);
  }

  pub fn add_block_by_ray(&mut self, ray: &Ray, block: usize) {
    let pick_result = self.pick_block(ray);
    if let Some(re) = pick_result {
      if let Some(b) = &World::block_face_opposite_position(re.block_position, re.face) {
        self.add_block(
          b,
          STONE,
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

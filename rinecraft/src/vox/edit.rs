use super::{block_coords::*, world_machine::VOID};
use crate::vox::block::*;
use crate::vox::chunk::*;
use crate::vox::world::*;
use crate::vox::world_machine::STONE;
use rendiation_math::Vec3;
use rendiation_math_entity::Ray3;
use rendiation_math_entity::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct BlockPickResult {
  pub world_position: Vec3<f32>,
  pub block_position: BlockWorldCoords,
  pub face: BlockFace,
  pub distance2: f32,
}

// todo optimize
fn pick_block(
  chunk: &Chunk,
  ray: &Ray3,
  previous_result: &Option<BlockPickResult>,
) -> Option<BlockPickResult> {
  if chunk.bounding.intersect(ray, &()) {
    let mut closest: Option<BlockPickResult> = None;
    for x in 0..CHUNK_WIDTH {
      for z in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_HEIGHT {
          let block = chunk.data[x][z][y];

          if block.is_void() {
            continue;
          }

          let local_position: BlockLocalCoords = (x, y, z).into();
          let world_position = local_position.to_world(chunk.chunk_position);
          let box3 = world_position.get_block_bbox();
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
      let box3 = r.block_position.get_block_bbox();
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
  pub fn pick_block(&self, ray: &Ray3) -> Option<BlockPickResult> {
    let mut nearest: Option<BlockPickResult> = None;
    for (_, chunk) in &self.chunks.chunks {
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

  fn notify_side_chunk_dirty(
    update_set: &mut HashSet<ChunkCoords>,
    chunk_key: ChunkCoords,
    point: BlockLocalCoords,
  ) {
    let point = point.0;
    if point.x == 0 {
      update_set.insert(chunk_key.get_side_chunk(ChunkSide::XYMin));
    } else if point.x == CHUNK_WIDTH - 1 {
      update_set.insert(chunk_key.get_side_chunk(ChunkSide::XYMax));
    }

    if point.z == 0 {
      update_set.insert(chunk_key.get_side_chunk(ChunkSide::ZYMin));
    } else if point.z == CHUNK_WIDTH - 1 {
      update_set.insert(chunk_key.get_side_chunk(ChunkSide::ZYMax));
    }
  }

  pub fn add_block(&mut self, block_position: BlockWorldCoords, block: Block) {
    let (chunk_key, local_position) = block_position.to_local_pair();
    let chunk = self.chunks.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, block);

    self.chunks.chunks_to_sync_scene.insert(chunk_key);
    World::notify_side_chunk_dirty(
      &mut self.chunks.chunks_to_sync_scene,
      chunk_key,
      local_position,
    );
  }

  pub fn delete_block(&mut self, block_position: BlockWorldCoords) {
    let (chunk_key, local_position) = block_position.to_local_pair();

    let chunk = self.chunks.chunks.get_mut(&chunk_key).unwrap();
    chunk.set_block(local_position, VOID);

    self.chunks.chunks_to_sync_scene.insert(chunk_key);
    World::notify_side_chunk_dirty(
      &mut self.chunks.chunks_to_sync_scene,
      chunk_key,
      local_position,
    );
  }

  pub fn add_block_by_ray(&mut self, ray: &Ray3, block: usize) {
    let pick_result = self.pick_block(ray);
    if let Some(re) = pick_result {
      if let Some(b) = re.block_position.face_opposite(re.face) {
        self.add_block(b, STONE);
      }
    }
  }

  pub fn delete_block_by_ray(&mut self, ray: &Ray3) {
    let pick_result = self.pick_block(ray);
    if let Some(re) = pick_result {
      self.delete_block(re.block_position);
    }
  }
}

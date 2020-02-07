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

impl IntersectAble<Chunk, BlockPickResult> for Ray {
  fn intersect(&self, chunk: &Chunk) -> Option<BlockPickResult> {
    if chunk.bounding.if_intersect_ray(self) {
      // Some(BlockPickResult {
      //   world_position: Vec3::new(0., 0., 0.),
      //   block_position: Vec3::new(0, 0, 0),
      //   face: BlockFace::XYMax,
      //   distance: 1.,
      // })
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
              world_position.x as f32 * CHUNK_ABS_WIDTH,
              world_position.y as f32 * CHUNK_ABS_WIDTH,
              world_position.z as f32 * CHUNK_ABS_WIDTH,
            );
            let max = Vec3::new(
              (world_position.x + 1) as f32 * CHUNK_ABS_WIDTH,
              (world_position.y + 1) as f32 * CHUNK_ABS_WIDTH,
              (world_position.z + 1) as f32 * CHUNK_ABS_WIDTH,
            );

            let box3 = Box3::new(min, max);
            let hit = self.intersect(&box3);
            if let Some(h) = hit {
              let length2 = (h - self.origin).length2();
              if let Some(clo) = &closest {
                if length2 < clo.distance2 {
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
      }
      closest
    } else {
      None
    }
  }
}

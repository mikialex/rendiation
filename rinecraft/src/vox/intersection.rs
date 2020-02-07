use rendiation_math_entity::Ray;
use crate::vox::block::BlockFace;
use crate::vox::chunk::Chunk;
use rendiation_math::Vec3;
use rendiation_math_entity::IntersectAble;

#[derive(Debug)]
pub struct BlockPickResult {
  pub world_position: Vec3<f32>,
  pub block_position: Vec3<i32>,
  pub face: BlockFace,
  pub distance: f32,
}

impl IntersectAble<Chunk, BlockPickResult> for Ray {
  fn intersect(&self, chunk: &Chunk) -> Option<BlockPickResult> {
    if chunk.bounding.if_intersect_ray(self) {
      Some(BlockPickResult {
        world_position: Vec3::new(0., 0., 0.),
        block_position: Vec3::new(0, 0, 0),
        face: BlockFace::XYMax,
        distance: 1.,
      })
    } else {
      None
    }
  }
}

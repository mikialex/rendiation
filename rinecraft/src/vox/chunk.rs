use crate::vox::block::build_block_face;
use crate::vox::block::BLOCK_FACES;
use crate::vox::block::BLOCK_WORLD_SIZE;
use crate::vox::block::Block;
use crate::geometry::StandardGeometry;
use rendiation_math::Vec3;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;

pub struct Chunk {
  pub chunk_position: (i32, i32, i32),
  pub data: Vec<Vec<Vec<Box<dyn Block>>>>,
  pub geometry_dirty: bool,
  pub geometry: StandardGeometry,
}

impl Chunk{
    fn update_geometry(&mut self){
        let mut new_index = Vec::new();
        let mut new_vertex = Vec::new();
        for x in 1..CHUNK_WIDTH - 1 {
            for z in 1..CHUNK_WIDTH - 1 {
              for y in 1..CHUNK_HEIGHT - 1 {
                let block = &self.data[x][z][y];
                let min_x = x as f32 * BLOCK_WORLD_SIZE;
                let min_y = y as f32 * BLOCK_WORLD_SIZE;
                let min_z = z as f32 * BLOCK_WORLD_SIZE;
            
                let max_x = (x + 1) as f32 * BLOCK_WORLD_SIZE;
                let max_y = (y + 1) as f32 * BLOCK_WORLD_SIZE;
                let max_z = (z + 1) as f32 * BLOCK_WORLD_SIZE;
            
                for face in BLOCK_FACES.iter() {
                    // if self.check_block_face_visibility(*face, (x, z, y)) {
                    build_block_face(
                        &(min_x, min_y, min_z),
                        &(max_x, max_y, max_z),
                        *face,
                        &mut new_index,
                        &mut new_vertex,
                    );
                    // }
                }
              }
            }
          }
    }
}
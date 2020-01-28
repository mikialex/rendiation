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

    }
}
use rendiation::*;

#[derive(Clone, Copy)]
pub struct Block{
  id: Option<usize>
}

impl Block{
  pub const fn new(id: usize) -> Self {
    Block{
      id: Some(id)
    }
  }
  
  pub fn void() -> Self {
    Block{
      id: None
    }
  }

  pub fn is_void(&self) -> bool {
    self.id.is_none()
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlockFace {
  XYMin,
  XYMax,
  XZMin,
  XZMax,
  YZMin,
  YZMax,
}

pub const BLOCK_WORLD_SIZE: f32 = 1.0;

pub const BLOCK_FACES: [BlockFace; 6] = [
  BlockFace::XYMin,
  BlockFace::XYMax,
  BlockFace::XZMin,
  BlockFace::XZMax,
  BlockFace::YZMin,
  BlockFace::YZMax,
];

pub fn build_block_face(
  min: &(f32, f32, f32),
  max: &(f32, f32, f32),
  face: BlockFace,
  index: &mut Vec<u16>,
  vertex: &mut Vec<Vertex>,
) {
  let data_origin = vertex.len() as u16;

  let min_x = min.0;
  let min_y = min.1;
  let min_z = min.2;

  let max_x = max.0;
  let max_y = max.1;
  let max_z = max.2;

  let normal = match face {
    BlockFace::XYMin => [0.0, 0.0, -1.0],
    BlockFace::XYMax => [0.0, 0.0, 1.0],
    BlockFace::XZMin => [0.0, -1.0, 0.0],
    BlockFace::XZMax => [0.0, 1.0, 0.0],
    BlockFace::YZMin => [-1.0, 0.0, 0.0],
    BlockFace::YZMax => [1.0, 0.0, 0.0],
  };

  let tex_coords = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

  let table = match face {
    BlockFace::XYMin => [
      [min_x, min_y, min_z],
      [min_x, max_y, min_z],
      [max_x, min_y, min_z],
      [max_x, max_y, min_z],
    ],
    BlockFace::XYMax => [
      [min_x, min_y, max_z],
      [max_x, min_y, max_z],
      [min_x, max_y, max_z],
      [max_x, max_y, max_z],
    ],
    BlockFace::XZMin => [
      [min_x, min_y, min_z],
      [max_x, min_y, min_z],
      [min_x, min_y, max_z],
      [max_x, min_y, max_z],
    ],
    BlockFace::XZMax => [
      [min_x, max_y, min_z],
      [min_x, max_y, max_z],
      [max_x, max_y, min_z],
      [max_x, max_y, max_z],
    ],
    BlockFace::YZMin => [
      [min_x, min_y, min_z],
      [min_x, min_y, max_z],
      [min_x, max_y, min_z],
      [min_x, max_y, max_z],
    ],
    BlockFace::YZMax => [
      [max_x, min_y, min_z],
      [max_x, max_y, min_z],
      [max_x, min_y, max_z],
      [max_x, max_y, max_z],
    ],
  };

  for i in 0..4 {
    vertex.push(Vertex {
      position: table[i].into(),
      normal: normal.into(),
      uv: tex_coords[i].into(),
    });
  }

  index.push(data_origin + 0);
  index.push(data_origin + 1);
  index.push(data_origin + 2);
  index.push(data_origin + 3);
  index.push(data_origin + 2);
  index.push(data_origin + 1);
}

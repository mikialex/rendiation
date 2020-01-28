
use crate::vertex::Vertex;

pub trait Block {
  // fn build_geometry(&self, chunk: &Chunk, x: usize, y: usize, z: usize);
}

#[derive(Copy, Clone, PartialEq)]
pub enum BlockFace {
  XYMin,
  XYMax,
  XZMin,
  XZMax,
  YZMin,
  YZMax,
}

pub const BLOCK_WORLD_SIZE: f32 = 1.0;

#[derive(Copy, Clone)]
pub struct SolidBlock {
  pub solid_block_type: SolidBlockType,
}

#[derive(Copy, Clone, PartialEq)]
pub enum SolidBlockType {
  Stone,
}

// impl Block for SolidBlock {

//   fn build_geomtry(&self, chunk: &Chunk, x: usize, y: usize, z: usize) {
//     let min_x = x as f32 * BLOCK_WORLD_SIZE;
//     let min_y = y as f32 * BLOCK_WORLD_SIZE;
//     let min_z = z as f32 * BLOCK_WORLD_SIZE;

//     let max_x = (x + 1) as f32 * BLOCK_WORLD_SIZE;
//     let max_y = (y + 1) as f32 * BLOCK_WORLD_SIZE;
//     let max_z = (z + 1) as f32 * BLOCK_WORLD_SIZE;

//     for face in BLOCK_FACES.iter() {
//       if chunk.check_block_face_visibility(*face, (x, z, y)) {
//         build_block_face(
//           *self,
//           &(min_x, min_y, min_z),
//           &(max_x, max_y, max_z),
//           *face,
//           &mut chunk.geometry.borrow_mut(),
//         );
//       }
//     }
//   }
// }

const BLOCK_FACES: [BlockFace; 6] = [
  BlockFace::XYMin,
  BlockFace::XYMax,
  BlockFace::XZMin,
  BlockFace::XZMax,
  BlockFace::YZMin,
  BlockFace::YZMax,
];

fn build_block_face(
  _block: SolidBlock,
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
      [max_x, max_y, min_z],
      [max_x, max_y, min_z],
      [max_x, min_y, max_z],
      [max_x, max_y, max_z],
    ],
  };

  for i in 0..4 {
    vertex.push(Vertex {
      position: table[i].into(),
      // normals: normal.into(),
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

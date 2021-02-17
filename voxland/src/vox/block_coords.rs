use super::{
  block::{BlockFace, BLOCK_WORLD_SIZE},
  chunk::{ChunkSide, CHUNK_ABS_WIDTH, CHUNK_HEIGHT, CHUNK_WIDTH},
};
use rendiation_algebra::Vec3;
use rendiation_geometry::Box3;

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct ChunkCoords(pub (i32, i32));

impl ChunkCoords {
  /// calculate chunk coords from a point, (which chunk is point in)
  pub fn from_world_position(point: Vec3<f32>) -> Self {
    let x = (point.x / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (point.z / CHUNK_ABS_WIDTH).floor() as i32;
    ChunkCoords((x, z))
  }

  pub fn world_start(&self) -> (f32, f32) {
    let p = self.0;
    (p.0 as f32 * CHUNK_ABS_WIDTH, p.1 as f32 * CHUNK_ABS_WIDTH)
  }

  pub fn get_side_chunk(&self, side: ChunkSide) -> Self {
    let chunk = self.0;
    match side {
      ChunkSide::XYMax => (chunk.0 + 1, chunk.1),
      ChunkSide::XYMin => (chunk.0 - 1, chunk.1),
      ChunkSide::ZYMax => (chunk.0, chunk.1 + 1),
      ChunkSide::ZYMin => (chunk.0, chunk.1 - 1),
    }
    .into()
  }
}

impl From<(i32, i32)> for ChunkCoords {
  fn from(other: (i32, i32)) -> ChunkCoords {
    ChunkCoords(other)
  }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BlockLocalCoords(pub Vec3<usize>);

impl From<(usize, usize, usize)> for BlockLocalCoords {
  fn from(other: (usize, usize, usize)) -> BlockLocalCoords {
    BlockLocalCoords(Vec3::new(other.0, other.1, other.2))
  }
}

impl BlockLocalCoords {
  pub fn to_world(&self, chunk_position: ChunkCoords) -> BlockWorldCoords {
    let chunk_position = chunk_position.0;
    let local_block_position = self.0;
    (
      local_block_position.x as i32 + chunk_position.0 * CHUNK_WIDTH as i32,
      local_block_position.y as i32,
      local_block_position.z as i32 + chunk_position.1 * CHUNK_WIDTH as i32,
    )
      .into()
  }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BlockWorldCoords(pub Vec3<i32>);

impl From<(i32, i32, i32)> for BlockWorldCoords {
  fn from(other: (i32, i32, i32)) -> BlockWorldCoords {
    BlockWorldCoords(Vec3::new(other.0, other.1, other.2))
  }
}

impl BlockWorldCoords {
  pub fn to_local_mod(&self) -> BlockLocalCoords {
    let block_position = self.0;
    let x = if block_position.x % CHUNK_WIDTH as i32 >= 0 {
      block_position.x % CHUNK_WIDTH as i32
    } else {
      block_position.x % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
    };

    let z = if block_position.z % CHUNK_WIDTH as i32 >= 0 {
      block_position.z % CHUNK_WIDTH as i32
    } else {
      block_position.z % CHUNK_WIDTH as i32 + CHUNK_WIDTH as i32
    };

    debug_assert!(x >= 0);
    debug_assert!(z >= 0);

    BlockLocalCoords(Vec3::new(x as usize, block_position.y as usize, z as usize))
  }

  pub fn to_local_pair(&self) -> (ChunkCoords, BlockLocalCoords) {
    (self.to_chunk_coords(), self.to_local_mod())
  }

  pub fn to_chunk_coords(&self) -> ChunkCoords {
    let block_position = self.0;
    let x = (block_position.x as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (block_position.z as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    ChunkCoords((x, z))
  }

  pub fn face_opposite(&self, face: BlockFace) -> Option<BlockWorldCoords> {
    let mut result = self.0;
    match face {
      BlockFace::XZMin => result.y -= 1,
      BlockFace::XZMax => result.y += 1,
      BlockFace::XYMin => result.z -= 1,
      BlockFace::XYMax => result.z += 1,
      BlockFace::YZMin => result.x -= 1,
      BlockFace::YZMax => result.x += 1,
    };

    if result.y < 0 {
      return None;
    }

    if result.y >= CHUNK_HEIGHT as i32 {
      return None;
    }
    Some(BlockWorldCoords(result))
  }

  pub fn get_block_bbox(&self) -> Box3 {
    let world_position = self.0;
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
    Box3::new3(min, max)
  }
}

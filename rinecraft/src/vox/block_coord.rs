#[derive(Copy, Clone)]
struct ChunkCoord(pub (usize, usize));

impl ChunkCoord{
  /// caculate chunk coord from a point, (which chunk is point in)
  fn from_world_position(point: Vec3<f32>) -> Self {
    let x = (point.x / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (point.z / CHUNK_ABS_WIDTH).floor() as i32;
    ChunkCoord((x, z))
  }

  fn from_block_world_coord(block_position: BlockWorldCoord) -> Self {
    let x = (block_position.x as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    let z = (block_position.z as f32 / CHUNK_ABS_WIDTH).floor() as i32;
    ChunkCoord((x, z))
  }
}

#[derive(Copy, Clone)]
struct BlockLocalCoord(pub Vec3<usize>);

impl From<(usize, usize, usize)> for BlockLocalCoord {
  fn from(other: (usize, usize, usize)) -> BlockLocalCoord {
    BlockWorldCoord(Vec3::new(other.0, other.1, other.2))
  }
}

impl BlockLocalCoord {
  pub fn to_world(&self, chunk_position: (i32, i32)) -> BlockWorldCoord {
    (
      local_block_position.x as i32 + chunk_position.0 * CHUNK_WIDTH as i32,
      local_block_position.y as i32,
      local_block_position.z as i32 + chunk_position.1 * CHUNK_WIDTH as i32,
    )
      .into()
  }
}


#[derive(Copy, Clone)]
struct BlockWorldCoord(pub Vec3<i32>);

impl From<(i32, i32, i32)> for BlockWorldCoord {
  fn from(other: (i32, i32, i32)) -> BlockWorldCoord {
    BlockWorldCoord(Vec3::new(other.0, other.1, other.2))
  }
}

impl BlockWorldCoord {
  pub fn to_local_mod(&self) -> BlockWorldCoord {
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
  
    assert_debug!(x >= 0);
    assert_debug!(z >= 0);
  
    Vec3::new(x as usize, block_position.y as usize, z as usize)
  }

  pub fn to_local_pair(&self) -> (ChunkCoord, BlockWorldCoord) {
    (ChunkCoord::from_block_world_coord(self), self.to_local_mod())
  }
}

use crate::vox::block::Block;
use crate::vox::block_meta::BlockRegistry;
use rendiation::*;
use super::block_meta::BlockMetaInfo;
use noise::*;

pub trait WorldMachine {
  //  y is up
  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block;
  fn get_block_info(&self, block: usize) -> &BlockMetaInfo;
}

pub struct WorldMachineImpl {
  pub block_registry: BlockRegistry,
  pub perlin: Fbm,
  _version: usize,
  _seed: usize,
}

impl WorldMachineImpl {
  pub fn new() -> Self {
    let block_registry = BlockRegistry::new_default();
    let perlin = Fbm::new();
    WorldMachineImpl {
      block_registry,
      perlin,
      _version: 0,
      _seed: 0,
    }
  }

  pub fn get_block_atlas(&mut self, renderer: &mut WGPURenderer) -> WGPUTexture{
    self.block_registry.create_atlas(renderer)
  }
}

pub const STONE: Block = Block::new(0);
// pub const DIRT: Block = Block::new(1);
// pub const GRASS: Block = Block::new(2);
pub const VOID: Block = Block::void();

// // this is per x,z block,cache the level height generate by the noise function;
// struct LevelCache {
//   terrain_type: TerrainType,
//   land_level: usize,
//   rock_level: usize,
//   hard_rock_level: usize,
// }

impl WorldMachine for WorldMachineImpl {

  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block {
    let height = 30. + self.perlin.get([x as f64 / 50., z as f64 / 50.]) * 20.;
    let h = height.floor() as i32;
    if y <= h {
      STONE
      // if y >= 3 {
      //   GRASS
      // } else if y >= 2{
      //   DIRT
      // }else {
      //   STONE
      // }
    } else {
      Block::void()
    }
  }

  
  fn get_block_info(&self, block_id: usize) -> &BlockMetaInfo {
    &self.block_registry.lut[block_id]
  }
}

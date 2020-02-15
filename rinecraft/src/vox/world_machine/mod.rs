use crate::vox::block::Block;
use crate::vox::block_meta::BlockRegistry;
use rendiation::*;
use super::block_meta::BlockMetaInfo;

pub trait WorldMachine {
  //  y is up
  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block;
  fn get_block_info(&self, block: usize) -> &BlockMetaInfo;
}

pub struct WorldMachineImpl {
  pub block_registry: BlockRegistry,
  version: usize,
  seed: usize,
}

impl WorldMachineImpl {
  pub fn new() -> Self {
    let block_registry = BlockRegistry::new_default();
    WorldMachineImpl {
      block_registry,
      version: 0,
      seed: 0,
    }
  }

  pub fn get_block_atlas(&mut self, renderer: &mut WGPURenderer) -> WGPUTexture{
    self.block_registry.create_atlas(renderer)
  }
}

pub const STONE: Block = Block::new(0);
pub const DIRT: Block = Block::new(1);
pub const GRASS: Block = Block::new(2);
pub const VOID: Block = Block::void();

impl WorldMachine for WorldMachineImpl {

  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block {
    if y <= x.abs() && y <= z.abs() {
      if y >= 3 {
        GRASS
      } else if y >= 2{
        DIRT
      }else {
        STONE
      }
    } else {
      Block::void()
    }
  }

  
  fn get_block_info(&self, block_id: usize) -> &BlockMetaInfo {
    &self.block_registry.lut[block_id]
  }
}

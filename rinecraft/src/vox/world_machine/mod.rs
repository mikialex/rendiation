use crate::vox::block::Block;
use crate::vox::block_meta::BlockRegistry;
use rendiation::*;

pub trait WorldMachine {
  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block;
}

pub struct WorldMachineImpl {
  block_registry: BlockRegistry,
  version: usize,
  seed: usize,
  block_texture_atlas: Texture2D<image::DynamicImage>
}

impl WorldMachineImpl {
  pub fn new() -> Self {
    let block_registry = BlockRegistry::new_default();
    let block_texture_atlas = block_registry.create_atlas();
    WorldMachineImpl {
      block_registry,
      version: 0,
      seed: 0,
      block_texture_atlas
    }
  }
}

pub const STONE: Block = Block::new(0);
impl WorldMachine for WorldMachineImpl {
  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block {
    if y <= x.abs() && y <= z.abs() {
      STONE
    } else {
      Block::void()
    }
  }
}

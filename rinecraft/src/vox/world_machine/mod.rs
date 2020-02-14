use crate::vox::block::Block;
use crate::vox::block_meta::BlockRegistry;

pub struct WorldMeta {
  block_registry: BlockRegistry,
  version: usize,
  seed: usize,
}

pub trait WorldMachine {
  fn world_gen(&self, x: i32, y: i32, z: i32) -> Block;
  fn get_meta(&self) -> &WorldMeta;
}

pub struct WorldMachineImpl {
  meta: WorldMeta,
}

impl WorldMachineImpl {
  pub fn new() -> Self {
    WorldMachineImpl {
      meta: WorldMeta {
        block_registry: BlockRegistry::new_default(),
        version: 0,
        seed: 0,
      },
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
  fn get_meta(&self) -> &WorldMeta {
    &self.meta
  }
}

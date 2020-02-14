use crate::vox::block_meta::BlockRegistry;

pub struct WorldMeta {
  block_registry: BlockRegistry,
  version: usize,
  seed: usize,
}

pub trait WorldMachine {
  fn world_gen(x: i32, y: i32, z: i32) -> usize;
  fn get_meta(&self) -> &WorldMeta;
}

use crate::vox::chunk::CHUNK_HEIGHT;
use super::block_meta::BlockMetaInfo;
use crate::vox::block::Block;
use crate::vox::block_meta::BlockRegistry;
use noise::*;
use rendiation_webgpu::*;
use std::collections::BTreeMap;

pub trait WorldMachine {
  //  y is up
  fn world_gen(&mut self, x: i32, y: i32, z: i32) -> Block;
  fn get_block_info(&self, block: usize) -> &BlockMetaInfo;
}

pub struct WorldMachineImpl {
  pub level_cache: BTreeMap<(i32, i32), LevelCache>,
  pub block_registry: BlockRegistry,
  pub fbm_noise: Fbm,
  _version: usize,
  _seed: usize,
}

impl WorldMachineImpl {
  pub fn new() -> Self {
    let block_registry = BlockRegistry::new_default();
    let fbm_noise = Fbm::new();
    WorldMachineImpl {
      level_cache: BTreeMap::new(),
      block_registry,
      fbm_noise,
      _version: 0,
      _seed: 0,
    }
  }

  pub fn get_block_atlas(&mut self, renderer: &mut WGPURenderer) -> WGPUTexture {
    self.block_registry.create_atlas(renderer)
  }
}

pub const STONE: Block = Block::new(0);
pub const DIRT: Block = Block::new(1);
pub const GRASS: Block = Block::new(2);
pub const VOID: Block = Block::void();

// this is per x,z block,cache the level height generate by the noise function;
pub struct LevelCache {
  // terrain_type: TerrainType,
  land_level: i32,
  rock_level: i32,
  // hard_rock_level: usize,
  generation_time: usize,
}

impl LevelCache {
  pub fn new(x: i32, z: i32, fbm_noise: &Fbm) -> Self {
    let map_scale = 50.;
    let land_level = 30. + fbm_noise.get([x as f64 / map_scale, z as f64 / map_scale]) * 20.;
    let rock_level =
      land_level - 5. + fbm_noise.get([x as f64 / map_scale, z as f64 / map_scale]) * 20.;
    Self {
      land_level: land_level as i32,
      rock_level: rock_level as i32,
      generation_time: 0,
    }
  }
}

impl WorldMachine for WorldMachineImpl {
  fn world_gen(&mut self, x: i32, y: i32, z: i32) -> Block {
    let fbm = &self.fbm_noise;
    let level_cache = self
      .level_cache
      .entry((x, z))
      .or_insert_with(|| LevelCache::new(x, z, fbm));

    let dirt_height = level_cache.land_level;
    let stone_height = level_cache.rock_level;
    let result = if y <= dirt_height {
      if y == dirt_height {
        GRASS
      } else if y > stone_height {
        DIRT
      } else {
        STONE
      }
    } else {
      VOID
    };

    level_cache.generation_time +=1;
    if level_cache.generation_time == CHUNK_HEIGHT{
      self.level_cache.remove(&(x, z));
    }

    result
  }

  fn get_block_info(&self, block_id: usize) -> &BlockMetaInfo {
    &self.block_registry.lut[block_id]
  }
}

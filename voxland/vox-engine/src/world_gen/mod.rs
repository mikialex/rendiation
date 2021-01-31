use crate::vox::block::Block;
pub mod block_meta;
use super::block_coords::ChunkCoords;
use block_meta::*;
use noise::*;
use rendiation_ral::ResourceManager;
use rendiation_webgpu::*;
use std::collections::BTreeMap;

pub struct WorldMachine {
  pub level_cache: BTreeMap<ChunkCoords, LevelCache>,
  pub block_registry: BlockRegistry,
  pub fbm_noise: Fbm,
  _version: usize,
  _seed: usize,
}

impl WorldMachine {
  pub fn new() -> Self {
    let block_registry = BlockRegistry::new_default();
    let fbm_noise = Fbm::new();
    Self {
      level_cache: BTreeMap::new(),
      block_registry,
      fbm_noise,
      _version: 0,
      _seed: 0,
    }
  }

  pub fn create_chunk_level_cache(&self, (x, z): (i32, i32)) -> LevelCache {
    LevelCache::new(x, z, &self.fbm_noise)
  }

  pub fn world_gen(&self, x: i32, y: i32, z: i32, level_cache: &LevelCache) -> Block {
    let fbm = &self.fbm_noise;

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
    result
  }

  pub fn get_block_info(&self, block_id: usize) -> &BlockMetaInfo {
    &self.block_registry.lut[block_id]
  }

  pub fn get_block_atlas(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WebGPU>,
  ) -> WGPUTexture {
    self.block_registry.create_atlas(renderer, resource)
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

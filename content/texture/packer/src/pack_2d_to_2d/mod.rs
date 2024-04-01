pub mod pack_impl;

use std::sync::atomic::AtomicU32;

use rendiation_texture::{Size, TextureRange};

pub trait TexturePackerInit {
  fn init_by_config(config: PackerConfig) -> Self;
}

pub trait BaseTexturePacker {
  fn reset(&mut self);
  /// for packer implementation, config should also call reset
  fn config(&mut self, config: PackerConfig);
}

impl<T: Default + BaseTexturePacker> TexturePackerInit for T {
  fn init_by_config(config: PackerConfig) -> Self {
    let mut packer = T::default();
    packer.config(config);
    packer
  }
}

#[derive(Debug)]
pub enum PackError {
  SpaceNotEnough,
}

#[derive(Debug)]
pub enum UnpackError {
  UnpackItemNotExist,
}

/// padding should handle in user side
pub trait TexturePacker: BaseTexturePacker {
  fn pack(&mut self, input: Size) -> Result<PackResult, PackError>;
}

pub trait PackableChecker: TexturePacker {
  /// this should have lower cost than pack, and not request mutable self
  fn can_pack(&self, input: Size) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackId(pub u32);

static GLOBAL_INCREASE_PACK_ID: AtomicU32 = AtomicU32::new(0);
impl Default for PackId {
  fn default() -> Self {
    PackId(GLOBAL_INCREASE_PACK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
  }
}

pub trait RePackablePacker: BaseTexturePacker {
  fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithId, PackError>;
  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError>;
}

#[derive(Debug, Clone, Copy)]
pub struct PackerConfig {
  pub allow_90_rotation: bool,
  pub full_size: Size,
}

impl Default for PackerConfig {
  fn default() -> Self {
    Self {
      allow_90_rotation: false,
      full_size: Size::from_usize_pair_min_one((512, 512)),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResult {
  pub range: TextureRange,
  /// clockwise not matter, should agree with the outer implementation
  pub rotated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResultWithId {
  pub result: PackResult,
  pub id: PackId,
}

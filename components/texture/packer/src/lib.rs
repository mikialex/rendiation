use std::sync::atomic::AtomicUsize;

use rendiation_texture::{Size, TextureRange};

pub mod shelf;
pub mod skyline;

pub trait BaseTexturePacker {
  fn reset(&mut self);
  /// config should also call reset
  fn config(&mut self, config: PackerConfig);
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackId(usize);

static GLOBAL_INCREASE_PACK_ID: AtomicUsize = AtomicUsize::new(0);
impl Default for PackId {
  fn default() -> Self {
    PackId(GLOBAL_INCREASE_PACK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
  }
}

pub trait RePackablePacker: BaseTexturePacker {
  fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithId, PackError>;
  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError>;
}

/// Some packer strategy maybe yield better result when input is batched
/// Impl this to specialize implementation. Or use the AutoBatchTexturePacker
/// to provide a default implementation;
pub trait BatchTexturePacker: BaseTexturePacker {
  fn batch_pack(
    &mut self,
    input: &[Size],
    config: PackerConfig,
  ) -> Result<AllPackResult, PackError>;
}

pub struct AutoBatchTexturePacker<P> {
  pub packer: P,
}

impl<P: BaseTexturePacker> BaseTexturePacker for AutoBatchTexturePacker<P> {
  fn config(&mut self, config: PackerConfig) {
    self.packer.config(config)
  }

  fn reset(&mut self) {
    self.packer.reset()
  }
}

impl<P: TexturePacker> BatchTexturePacker for AutoBatchTexturePacker<P> {
  fn batch_pack(
    &mut self,
    inputs: &[Size],
    config: PackerConfig,
  ) -> Result<AllPackResult, PackError> {
    self.config(config);

    let size_all = config.init_size;
    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
      results.push(self.packer.pack(*input)?);
    }
    Ok(AllPackResult { size_all, results })
  }
}

#[derive(Debug, Clone, Copy)]
pub struct PackerConfig {
  pub allow_90_rotation: bool,
  pub init_size: Size,
}

impl Default for PackerConfig {
  fn default() -> Self {
    Self {
      allow_90_rotation: false,
      init_size: Size::from_usize_pair_min_one((512, 512)),
    }
  }
}

pub struct PackResult {
  pub range: TextureRange,
  pub rotated: bool, // should clockwise matters?
}

pub struct AllPackResult {
  pub results: Vec<PackResult>,
  pub size_all: Size,
}

pub struct PackResultWithId {
  pub result: PackResult,
  pub id: PackId,
}

use rendiation_texture::Size;

pub mod skyline;

pub trait TexturePackStrategyBase {
  fn reset(&mut self);
  /// config should also call reset
  fn config(&mut self, config: PackerConfig);
}

pub enum PackError {
  SpaceNotEnough,
}

/// padding should handle in user side
pub trait TexturePackStrategy: TexturePackStrategyBase {
  fn pack(&mut self, input: Size) -> Result<PackResult, PackError>;
}

pub trait PackableChecker: TexturePackStrategy {
  /// this should have lower cost than pack, and not request mutable self
  fn can_pack(&self, input: Size) -> bool;
}

/// Some packer strategy maybe yield better result when input is batched
/// Impl this to specialize implementation. Or use the AutoBatchTexturePacker
/// to provide a default implementation;
pub trait BatchTexturePackStrategy: TexturePackStrategyBase {
  fn batch_pack(
    &mut self,
    input: &[Size],
    config: PackerConfig,
  ) -> Result<AllPackResult, PackError>;
}

pub struct AutoBatchTexturePacker<P> {
  pub packer: P,
}

impl<P: TexturePackStrategyBase> TexturePackStrategyBase for AutoBatchTexturePacker<P> {
  fn config(&mut self, config: PackerConfig) {
    self.packer.config(config)
  }

  fn reset(&mut self) {
    self.packer.reset()
  }
}

impl<P: TexturePackStrategy> BatchTexturePackStrategy for AutoBatchTexturePacker<P> {
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

pub struct PackResult {
  pub offset: (usize, usize),
  pub size: Size,
  pub rotated: bool, // should clockwise matters?
}

pub struct AllPackResult {
  pub results: Vec<PackResult>,
  pub size_all: Size,
}

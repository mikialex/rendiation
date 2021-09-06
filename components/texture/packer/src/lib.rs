use rendiation_texture::Size;

// pub mod skyline;

pub trait TexturePackStrategyBase {
  fn config(&mut self, config: PackerStrategyConfig);
  fn finish(&mut self) -> AllPackResult;
}

pub enum PackError {
  SpaceNotEnough,
}

// padding should handle in user side
pub trait TexturePackStrategy: TexturePackStrategyBase {
  fn pack(&mut self, input: Size) -> Result<PackResult, PackError>;
}

pub trait PackableChecker: TexturePackStrategy {
  /// this should have lower cost than pack, and not request mutable self
  fn can_pack(&self, input: Size) -> bool;
}

/// Some packer strategy maybe has better result when input is batched
/// Impl this to specialize implementation. Or use the AutoBatchTexturePacker
/// to provide a default implementation;
pub trait BatchTexturePackStrategy: TexturePackStrategyBase {
  fn batch_pack(&mut self, input: &[Size]) -> Result<AllPackResult, PackError>;
}

pub struct AutoBatchTexturePacker<P> {
  pub packer: P,
}

impl<P: TexturePackStrategyBase> TexturePackStrategyBase for AutoBatchTexturePacker<P> {
  fn config(&mut self, config: PackerStrategyConfig) {
    self.packer.config(config)
  }

  fn finish(&mut self) -> AllPackResult {
    self.packer.finish()
  }
}

impl<P: TexturePackStrategy> BatchTexturePackStrategy for AutoBatchTexturePacker<P> {
  fn batch_pack(&mut self, inputs: &[Size]) -> Result<AllPackResult, PackError> {
    for input in inputs {
      self.packer.pack(*input)?;
    }
    Ok(self.finish())
  }
}

pub struct PackerStrategyConfig {
  pub allow_90_rotation: bool,
  pub init_size: Size,
  pub growable: bool,
}

pub struct PackResult {
  pub offset: (usize, usize),
  pub size: Size,
  pub rotated: bool, // should clockwise matters?
}

pub struct AllPackResult {
  pub results: Vec<PackResult>,
  pub size_final: Size,
}

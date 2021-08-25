use rendiation_texture::Size;

pub trait TexturePackStrategyBase {
  fn config(&mut self, config: PackerStrategyConfig);
  fn finish(&mut self) -> AllPackResult;
}

pub enum PackError {
  SpaceNotEnough,
}

pub trait TexturePackStrategy: TexturePackStrategyBase {
  fn pack(&mut self, input: Input) -> Result<PackResult, PackError>;
}

pub trait BatchTexturePackStrategy: TexturePackStrategyBase {
  fn batch_pack(&mut self, input: &[Input]) -> Result<AllPackResult, PackError>;
}

pub struct PackerStrategyConfig {
  pub allow_90_rotation: bool,
  pub init_size: Size,
  pub growable: bool,
}

pub struct Input {
  pub size: Size,
  pub padding: usize,
}

pub struct PackResult {
  pub offset: (usize, usize),
  pub size: Size,
}

pub struct AllPackResult {
  pub results: Vec<PackResult>,
  pub size_final: Size,
}

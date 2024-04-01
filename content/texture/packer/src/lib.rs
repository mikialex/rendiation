pub mod growable;
pub mod pack_2d_to_2d;
pub mod pack_2d_to_3d;

pub trait TexturePackerInit {
  type Config: Clone;

  fn init_by_config(config: Self::Config) -> Self;
}

/// padding should handle in user side
pub trait TexturePacker {
  type Input: Clone;
  type PackOutput: Clone;
  fn pack(&mut self, input: Self::Input) -> Result<Self::PackOutput, PackError>;
}

pub trait PackableChecker: TexturePacker {
  /// this should have lower cost than pack, and not request mutable self
  fn can_pack(&self, input: Self::Input) -> bool;
}

pub trait RePackablePacker {
  type Input: Clone;
  type PackOutput: Clone;
  fn pack_with_id(
    &mut self,
    input: Self::Input,
  ) -> Result<PackResultWithId<Self::PackOutput>, PackError>;
  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResultWithId<T> {
  pub result: T,
  pub id: PackId,
}

#[derive(Debug)]
pub enum PackError {
  SpaceNotEnough,
}

#[derive(Debug)]
pub enum UnpackError {
  UnpackItemNotExist,
}

mod flat;
pub use flat::*;
mod physical_sg;
pub use physical_sg::*;
mod physical_mr;
pub use physical_mr::*;
mod utils;
pub use utils::*;

use crate::*;

fn tex_sample_handle_of_material<M: MaterialReferenceTexture>(
  scope: impl ReactiveCollection<AllocIdx<M>, ()>,
  texture2ds: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair>,
) -> impl ReactiveCollection<(u8, AllocIdx<M>), TextureSamplerHandlePair> {
  // storage_of::<M>().listen_to_reactive_collection(M::check_change);
  //   .filter_by_keyset(scope)
}

fn material_referenced_textures<M: MaterialReferenceTexture>(
  scope: impl ReactiveCollection<AllocIdx<M>, ()>,
) -> impl ReactiveCollection<(u8, AllocIdx<M>), AllocIdx<SceneTexture2DType>> {
  // storage_of::<M>().listen_to_reactive_collection(M::check_change);
  //   .filter_by_keyset(scope)
}

pub trait MaterialReferenceTexture: IncrementalBase {
  type TextureType: CKey + Into<u8>;

  fn get_texture(&self, ty: Self::TextureType) -> &SceneTexture2D;
  fn check_change(
    change: Self::Delta,
  ) -> ChangeReaction<(Self::TextureType, AllocIdx<SceneTexture2DType>)>;

  fn expand_self(&self, change: &mut dyn Fn((Self::TextureType, AllocIdx<SceneTexture2DType>)));
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PhysicalMetallicRoughnessMaterialTextureType {
  BaseColor,
  MetallicRoughness,
  Emissive,
  Normal,
}

impl Into<u8> for PhysicalMetallicRoughnessMaterialTextureType {
  fn into(self) -> u8 {
    self as u8
  }
}

impl MaterialReferenceTexture for PhysicalMetallicRoughnessMaterial {
  type TextureType = PhysicalMetallicRoughnessMaterialTextureType;

  fn get_texture(&self, ty: Self::TextureType) -> &SceneTexture2D {
    match ty {
      PhysicalMetallicRoughnessMaterialTextureType::BaseColor => todo!(),
      PhysicalMetallicRoughnessMaterialTextureType::MetallicRoughness => todo!(),
      PhysicalMetallicRoughnessMaterialTextureType::Emissive => todo!(),
      PhysicalMetallicRoughnessMaterialTextureType::Normal => todo!(),
    }
  }

  fn check_change(
    change: Self::Delta,
  ) -> ChangeReaction<(Self::TextureType, AllocIdx<SceneTexture2DType>)> {
    todo!()
  }

  fn expand_self(&self, change: &mut dyn Fn((Self::TextureType, AllocIdx<SceneTexture2DType>))) {
    todo!()
  }
}

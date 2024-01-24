mod flat;
pub use flat::*;
mod physical_sg;
pub use physical_sg::*;
mod physical_mr;
pub use physical_mr::*;
mod utils;
pub use utils::*;

use crate::*;

pub trait ReactiveCollectionNewExt<K: CKey, V: CValue>: ReactiveCollection<K, V> + Sized {
  fn collective_key_map_filter<K2: CKey>(
    self,
    filter: impl Fn(K) -> Option<K2>,
  ) -> impl ReactiveCollection<K2, V> {
  }

  fn collective_key_lifting<K2: CKey>(
    self,
    lift: impl Fn(K) -> K2,
    un_lift: impl Fn(K2) -> Option<K>,
  ) -> impl ReactiveCollection<K2, V> {
  }
}
impl<K: CKey, V: CValue, T: ReactiveCollection<K, V>> ReactiveCollectionNewExt<K, V> for T {}

pub trait ReactiveRelationNewExt<O: CKey, M: CKey>:
  ReactiveOneToManyRelationship<O, M> + Sized
{
  fn collective_remap_value<X>(
    self,
    value_map: impl ReactiveCollection<O, X>,
  ) -> impl ReactiveCollection<M, X>
  where
    X: CValue,
  {
  }
}

impl<O: CKey, M: CKey, T: ReactiveOneToManyRelationship<O, M>> ReactiveRelationNewExt<O, M> for T {}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct MaterialTextureAddress {
  pub material_type_id: TypeId,
  pub material_alloc_id: u32,
  pub material_texture_id: u32,
}

pub trait MaterialReferenceTexture: IncrementalBase {
  type TextureType: CKey + Into<u8>;
  type TextureUniform: CValue;

  fn get_texture(&self, ty: Self::TextureType) -> &SceneTexture2D;
  fn check_change(
    change: Self::Delta,
  ) -> ChangeReaction<(Self::TextureType, AllocIdx<SceneTexture2DType>)>;

  fn expand_self(&self, change: &mut dyn Fn((Self::TextureType, AllocIdx<SceneTexture2DType>)));

  fn create_reference_collection(
    scope: impl ReactiveCollection<AllocIdx<Self>, ()>,
  ) -> impl ReactiveCollection<(u8, AllocIdx<Self>), AllocIdx<SceneTexture2DType>> {
    // todo, custom listen to
  }

  fn create_reference_relation(
    reference_collection: impl ReactiveCollection<(u8, AllocIdx<Self>), AllocIdx<SceneTexture2DType>>,
  ) -> impl ReactiveOneToManyRelationship<AllocIdx<SceneTexture2DType>, (u8, AllocIdx<Self>)> {
    reference_collection.into_one_to_many_by_hash()
  }

  // fn create_texture_uniforms(
  //   scope: impl ReactiveCollection<AllocIdx<Self>, ()>,
  //   texture2ds: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair>,
  // ) -> impl ReactiveCollection<AllocIdx<Self>, Self::TextureUniform> {
  //   // M::create_reference_relation(scope).collective_remap_value(texture2ds)
  // }
}

pub fn material_textures<M: MaterialReferenceTexture>(
  scope: impl ReactiveCollection<AllocIdx<StandardModel>, ()>,
) -> (
  RxCForker<(u8, AllocIdx<M>), AllocIdx<SceneTexture2DType>>,
  impl ReactiveCollection<MaterialTextureAddress, AllocIdx<SceneTexture2DType>>,
) {
  let m_scope = storage_of::<StandardModel>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope);

  let m_referenced_textures = M::create_reference_collection(())
    .into_boxed()
    .into_forker();

  // let lift_referenced_textures = m_referenced_textures.clone().collective_key_lifting(|v| {},
  // un_lift); todo
  let lift_referenced_textures = ();

  (m_referenced_textures, lift_referenced_textures)
}

pub struct SceneTextureMaterialsRelations {
  mr_mat:
    RxCForker<(u8, AllocIdx<PhysicalMetallicRoughnessMaterial>), AllocIdx<SceneTexture2DType>>,
  // sg_mat: RxCForker<(u8, AllocIdx<PhysicalSpecularGlossinessMaterial>),
  // AllocIdx<SceneTexture2DType>>,
}

pub fn all_std_model_materials_textures(
  scope: impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone,
  foreign: impl ReactiveCollection<MaterialTextureAddress, AllocIdx<SceneTexture2DType>>,
) -> (
  SceneTextureMaterialsRelations,
  impl ReactiveCollection<MaterialTextureAddress, AllocIdx<SceneTexture2DType>>,
) {
  let (mr_mat, mr_lifted) = material_textures::<PhysicalMetallicRoughnessMaterial>(scope.clone());
  // let (sg_mat, sg_lifted) =
  // material_textures::<PhysicalMetallicRoughnessMaterial>(scope.clone());

  let forker = SceneTextureMaterialsRelations { mr_mat };

  let all_texture = mr_lifted.collective_select(foreign);

  (forker, all_texture)
}

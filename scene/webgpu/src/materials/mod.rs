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

pub trait AllocIdCollectionGPUExt<K: 'static> {
  // todo, remove parallel?
  fn collective_execute_gpu_map<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K, &ResourceGPUCtx) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, V>
  where
    V: CValue;

  fn collective_create_uniforms<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync;
}

impl<K, T> AllocIdCollectionGPUExt<K> for T
where
  T: ReactiveCollection<AllocIdx<K>, ()>,
  K: IncrementalBase,
{
  fn collective_execute_gpu_map<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K, &ResourceGPUCtx) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, V>
  where
    V: CValue,
  {
    let gpu = gpu.clone();
    self.collective_execute_map_by(move || {
      let gpu = gpu.clone();
      let creator = storage_of::<K>().create_key_mapper(move |m, _| mapper(m, &gpu));
      move |k, _| creator(*k)
    })
  }

  fn collective_create_uniforms<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync,
  {
    self.collective_execute_gpu_map(gpu, move |k, gpu| {
      let uniform = mapper(k);
      create_uniform(uniform, &gpu.device)
    })
  }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct MaterialTextureAddress {
  pub material_type_id: TypeId,
  pub material_alloc_id: u32,
  pub material_texture_id: u32,
}

pub trait MaterialReferenceTexture: IncrementalBase {
  type TextureType: CKey + Into<u8>;
  type TextureUniform: CValue + Default + Std140;

  fn get_texture(&self, ty: Self::TextureType) -> Option<&SceneTexture2D>;
  fn check_change(
    change: Self::Delta,
  ) -> ChangeReaction<(Self::TextureType, AllocIdx<SceneTexture2DType>)>;

  fn expand_self(&self, change: &mut dyn Fn((Self::TextureType, AllocIdx<SceneTexture2DType>)));
  fn update_texture_uniform(ty: Self::TextureType, handle: u32, target: &mut Self::TextureUniform);

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

  fn create_texture_uniforms(
    reference_collection: impl ReactiveCollection<(u8, AllocIdx<Self>), AllocIdx<SceneTexture2DType>>,
    texture2ds: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair>,
  ) -> impl ReactiveCollection<AllocIdx<Self>, UniformBufferDataView<Self::TextureUniform>> {
    // reference_collection
    //   .into_one_to_many_by_hash()
    //   .collective_remap_value(texture2ds)
  }
}

pub fn material_textures<M: MaterialReferenceTexture + DowncastFromMaterialEnum>(
  std_scope: impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone,
) -> RxCForker<(u8, AllocIdx<M>), AllocIdx<SceneTexture2DType>> {
  let relations = global_material_relations::<M>();
  let referenced_mat = std_scope.clone().many_to_one_reduce_key(relations.clone());

  M::create_reference_collection(referenced_mat)
    .into_boxed()
    .into_forker()
}

pub type TextureMaterialReferenceFork<M> =
  RxCForker<(u8, AllocIdx<M>), AllocIdx<SceneTexture2DType>>;

pub struct SceneTextureMaterialsRelations {
  pub mr: TextureMaterialReferenceFork<PhysicalMetallicRoughnessMaterial>,
  pub sg: TextureMaterialReferenceFork<PhysicalSpecularGlossinessMaterial>,
}

pub fn all_std_model_materials_textures(
  scope: impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone,
) -> SceneTextureMaterialsRelations {
  SceneTextureMaterialsRelations {
    mr: material_textures::<PhysicalMetallicRoughnessMaterial>(scope.clone()),
    sg: material_textures::<PhysicalSpecularGlossinessMaterial>(scope.clone()),
  }
}

pub(super) fn setup_tex(
  ctx: &mut GPURenderPassCtx,
  binding_sys: &GPUTextureBindingSystem,
  tex: &Option<Texture2DWithSamplingData>,
) {
  if let Some(tex) = tex {
    todo!()
  }
}

pub(super) fn bind_and_sample(
  binding: &mut ShaderBindGroupDirectBuilder,
  reg: &SemanticRegistry,
  tex: &Option<Texture2DWithSamplingData>,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
  default_value: Node<Vec4<f32>>,
) -> Node<Vec4<f32>> {
  // let texture = binding.binding::<GPU2DTextureView>();
  // let sampler = binding.binding::<GPUSamplerView>();
  // texture.sample(sampler, uv)
  todo!()
}

pub(super) fn bind_and_sample_enabled(
  binding: &mut ShaderBindGroupDirectBuilder,
  reg: &SemanticRegistry,
  tex: &Option<Texture2DWithSamplingData>,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
  default_value: Node<Vec4<f32>>,
) -> (Node<Vec4<f32>>, Node<bool>) {
  // let texture = binding.binding::<GPU2DTextureView>();
  // let sampler = binding.binding::<GPUSamplerView>();
  // texture.sample(sampler, uv)
  todo!()
}

pub(super) fn setup_normal_tex(
  ctx: &mut GPURenderPassCtx,
  binding_sys: &GPUTextureBindingSystem,
  norm: &Option<NormalMapping>,
) {
  if let Some(norm) = norm {
    todo!()
  }
}

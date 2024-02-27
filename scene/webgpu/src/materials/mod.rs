mod flat;
use __core::marker::PhantomData;
pub use flat::*;
mod physical_sg;
pub use physical_sg::*;
mod physical_mr;
pub use physical_mr::*;
mod utils;
pub use utils::*;

use crate::*;

pub trait ReactiveCollectionNewExt<K: CKey, V: CValue>: ReactiveCollection<K, V> + Sized {
  fn collective_key_lifting<K2: CKey>(
    self,
    lift_pair: (impl Fn(K) -> K2, impl Fn(K2) -> Option<K>),
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
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, V>
  where
    V: CValue;

  fn collective_create_uniforms_by_key<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync;
}

impl<K, T> AllocIdCollectionGPUExt<K> for T
where
  T: ReactiveCollection<AllocIdx<K>, AnyChanging>,
  K: IncrementalBase,
{
  fn collective_execute_gpu_map<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K, &ResourceGPUCtx) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, V>
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

  fn collective_create_uniforms_by_key<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync,
  {
    self.collective_execute_gpu_map(gpu, move |k, gpu| {
      let uniform = mapper(k);
      create_uniform(uniform, &gpu.device)
    })
  }
}

pub trait CollectionGPUExt<K: CKey, V: CValue> {
  fn collective_create_uniforms(
    self,
    gpu: ResourceGPUCtx,
  ) -> impl ReactiveCollection<K, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync;
}
impl<K: CKey, V: CValue, T> CollectionGPUExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
{
  fn collective_create_uniforms(
    self,
    gpu: ResourceGPUCtx,
  ) -> impl ReactiveCollection<K, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync,
  {
    let gpu = gpu.clone();
    self.collective_execute_map_by(move || {
      let gpu = gpu.clone();
      move |_, uniform| create_uniform(uniform, &gpu.device)
    })
  }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct MaterialTextureAddress {
  pub material_type_id: TypeId,
  pub material_alloc_id: u32,
  pub material_texture_id: u32,
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
#[derivative(Copy(bound = ""))]
pub struct MaterialTextureChangeProcessor<M> {
  m_type: PhantomData<M>,
}

pub(super) fn pick_tex(t: &Option<Texture2DWithSamplingData>) -> Option<&SceneTexture2D> {
  t.as_ref().map(|t| &t.texture)
}

pub(super) fn pick_tex_id(
  t: &Option<Texture2DWithSamplingData>,
) -> Option<AllocIdx<SceneTexture2DType>> {
  pick_tex(t).map(|t| t.alloc_index().into())
}

pub(super) fn pick_tex_d(
  t: &DeltaOf<Option<Texture2DWithSamplingData>>,
) -> Option<AllocIdx<SceneTexture2DType>> {
  t.as_ref()
    .map(merge_maybe_ref)
    .map(|t| t.texture.alloc_index().into())
}

pub(super) fn pick_normal_tex_d(
  t: &DeltaOf<Option<NormalMapping>>,
) -> Option<AllocIdx<SceneTexture2DType>> {
  // Some(merge_maybe(t?).texture.alloc_index().into())
  todo!()
}

use derivative::Derivative;
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
#[derivative(Copy(bound = ""))]
#[derivative(Eq(bound = ""))]
#[derivative(PartialEq(bound = ""))]
#[derivative(Hash(bound = ""))]
#[derivative(Debug(bound = ""))]
pub struct MaterialRefTextureId<M> {
  pub material: AllocIdx<M>,
  pub texture_variant: u8,
}

pub fn lift_pair<M: Any>() -> (
  impl Fn(MaterialRefTextureId<M>) -> (TypeId, u32, u8),
  impl Fn((TypeId, u32, u8)) -> Option<MaterialRefTextureId<M>>,
) {
  let lift = |v: MaterialRefTextureId<M>| (TypeId::of::<M>(), v.material.index, v.texture_variant);
  let un_lift = |v: (TypeId, u32, u8)| {
    if v.0 == TypeId::of::<M>() {
      Some(MaterialRefTextureId::<M> {
        material: v.1.into(),
        texture_variant: v.2,
      })
    } else {
      None
    }
  };
  (lift, un_lift)
}

impl<M> LinearIdentified for MaterialRefTextureId<M> {
  fn alloc_index(&self) -> u32 {
    self.material.alloc_index()
  }
}

impl<M: MaterialReferenceTexture>
  ChangeProcessor<M, MaterialRefTextureId<M>, AllocIdx<SceneTexture2DType>>
  for MaterialTextureChangeProcessor<M>
{
  fn react_change(
    &self,
    change: (&M::Delta, &M),
    idx: AllocIdx<M>,
    callback: &dyn Fn(MaterialRefTextureId<M>, ValueChange<AllocIdx<SceneTexture2DType>>),
  ) {
    let m = change.1;
    m.react_change(change.0, &|t_type, new_tex| {
      let previous = m.get_texture(t_type).map(|v| v.alloc_index().into());

      let change = match (new_tex, previous) {
        (None, None) => None,
        (None, Some(pre)) => ValueChange::Remove(pre).into(),
        (Some(new), None) => ValueChange::Delta(new, None).into(),
        (Some(new), Some(pre)) => ValueChange::Delta(new, Some(pre)).into(),
      };

      if let Some(change) = change {
        use num_traits::ToPrimitive;
        callback(
          MaterialRefTextureId {
            material: idx,
            texture_variant: t_type.to_u8().unwrap(),
          },
          change,
        );
      }
    });
  }

  fn create_iter(
    &self,
    v: &M,
    idx: AllocIdx<M>,
  ) -> impl Iterator<Item = (MaterialRefTextureId<M>, AllocIdx<SceneTexture2DType>)> {
    v.create_iter().map(move |(t_id, t)| {
      use num_traits::ToPrimitive;
      (
        MaterialRefTextureId {
          material: idx,
          texture_variant: t_id.to_u8().unwrap(),
        },
        t,
      )
    })
  }

  fn access(&self, v: &M, k: &MaterialRefTextureId<M>) -> Option<AllocIdx<SceneTexture2DType>> {
    use num_traits::FromPrimitive;
    let ty = M::TextureType::from_u8(k.texture_variant).unwrap();
    v.get_texture(ty).map(|t| t.alloc_index().into())
  }
}

pub trait MaterialReferenceTexture: IncrementalBase {
  type TextureType: CKey + Copy + num_traits::FromPrimitive + num_traits::ToPrimitive;
  type TextureUniform: CValue + Default + Std140;

  fn get_texture(&self, ty: Self::TextureType) -> Option<&SceneTexture2D>;

  fn react_change(
    &self,
    delta: &Self::Delta,
    callback: &dyn Fn(Self::TextureType, Option<AllocIdx<SceneTexture2DType>>),
  );

  fn create_iter(&self) -> impl Iterator<Item = (Self::TextureType, AllocIdx<SceneTexture2DType>)>;

  fn update_texture_uniform(ty: Self::TextureType, handle: u32, target: &mut Self::TextureUniform);

  fn create_reference_collection(
    scope: impl ReactiveCollection<AllocIdx<Self>, ()>,
  ) -> impl ReactiveCollection<MaterialRefTextureId<Self>, AllocIdx<SceneTexture2DType>> {
    storage_of::<Self>().listen_to_reactive_collection_custom(MaterialTextureChangeProcessor {
      m_type: PhantomData,
    })
  }

  fn create_texture_uniforms(
    // scope: impl ReactiveCollection<AllocIdx<Self>, ()>,
    reference_collection: impl ReactiveCollection<
      MaterialRefTextureId<Self>,
      AllocIdx<SceneTexture2DType>,
    >,
    texture2ds: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair>,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<Self>, UniformBufferDataView<Self::TextureUniform>>
  {
    // scope.collective_map(|_| Self::TextureUniform::default());

    // todo, should we impl custom collection here?

    // reference_collection
    //   .into_one_to_many_by_hash()
    //   .collective_remap_value(texture2ds)
  }
}

// struct MaterialTextureUniform<M: MaterialReferenceTexture> {
//   upstream_range: S,
//   textures_source: TS,
//   uniforms: FastHashMap<AllocIdx<M>, UniformBufferDataView<M::TextureUniform>>,
// }

// impl<M: MaterialReferenceTexture>
//   ReactiveCollection<AllocIdx<M>, UniformBufferDataView<M::TextureUniform>>
//   for MaterialTextureUniform<M>
// {
//   fn poll_changes(
//     &self,
//     cx: &mut Context,
//   ) -> PollCollectionChanges<AllocIdx<M>, UniformBufferDataView<M::TextureUniform>> {
//     todo!()
//   }

//   fn access(&self) -> PollCollectionCurrent<AllocIdx<M>,
// UniformBufferDataView<M::TextureUniform>> {     todo!()
//   }

//   fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
//     todo!()
//   }
// }

pub fn material_textures<M: MaterialReferenceTexture + DowncastFromMaterialEnum>(
  std_scope: impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone,
) -> RxCForker<MaterialRefTextureId<M>, AllocIdx<SceneTexture2DType>> {
  let relations = global_material_relations::<M>();
  let referenced_mat = std_scope.clone().many_to_one_reduce_key(relations.clone());

  M::create_reference_collection(referenced_mat)
    .into_boxed()
    .into_forker()
}

pub type TextureMaterialReferenceFork<M> =
  RxCForker<MaterialRefTextureId<M>, AllocIdx<SceneTexture2DType>>;

pub struct SceneTextureMaterialsRelations {
  pub mr: TextureMaterialReferenceFork<PhysicalMetallicRoughnessMaterial>,
  pub sg: TextureMaterialReferenceFork<PhysicalSpecularGlossinessMaterial>,
}

impl SceneTextureMaterialsRelations {
  pub fn normalized_path(
    &self,
  ) -> impl ReactiveCollection<(TypeId, u32, u8), AllocIdx<SceneTexture2DType>> {
    let mr = self.mr.clone().collective_key_lifting(lift_pair());
    let sg = self.sg.clone().collective_key_lifting(lift_pair());
    mr.collective_select(sg)
  }
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
  tex: Option<&Texture2DWithSamplingData>,
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

pub struct MaterialsGPUResource {
  flat: FlatMaterialGPUResource,
  //..
}

impl MaterialsGPUResource {
  pub fn prepare_render(&self, mat: &MaterialEnum) -> SceneMaterialRenderComponent {
    match mat {
      MaterialEnum::Flat(mat) => {
        SceneMaterialRenderComponent::Flat(self.flat.prepare_render(mat.alloc_index().into()))
      }
      _ => todo!(),
    }
  }
}

pub enum SceneMaterialRenderComponent<'a> {
  Flat(FlatMaterialGPU<'a>),
}

impl<'a> ShaderHashProvider for SceneMaterialRenderComponent<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(self).hash(hasher);
    // match self {}
    todo!()
  }
}
impl<'a> ShaderHashProviderAny for SceneMaterialRenderComponent<'a> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    todo!()
  }
}
// todo
impl<'a> GraphicsShaderProvider for SceneMaterialRenderComponent<'a> {}
impl<'a> ShaderPassBuilder for SceneMaterialRenderComponent<'a> {}

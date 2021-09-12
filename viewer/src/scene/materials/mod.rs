use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
};
pub mod bindable;
pub use bindable::*;
pub mod states;
use rendiation_webgpu::GPU;
pub use states::*;

pub mod basic;
pub use basic::*;

use rendiation_algebra::Mat4;

use super::{
  Camera, CameraBindgroup, MaterialHandle, Mesh, ReferenceFinalization, Scene, SceneSampler,
  SceneTexture2D, TransformGPU, TypedMaterialHandle, ValueID, ViewerRenderPass, WatchedArena,
};

impl Scene {
  fn add_material_inner<M: Material + 'static, F: FnOnce(MaterialHandle) -> M>(
    &mut self,
    creator: F,
  ) -> MaterialHandle {
    self
      .materials
      .insert_with(|handle| Box::new(creator(handle)))
  }

  pub fn add_material<M>(&mut self, material: M) -> TypedMaterialHandle<M>
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    let handle = self.add_material_inner(|handle| MaterialCell::new(material, handle));
    TypedMaterialHandle {
      handle,
      ty: PhantomData,
    }
  }
}

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource {
  type GPU;
  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
  ) {
    // default do nothing
  }

  fn setup_pass<'a>(
    &'a self,
    _pass: &mut wgpu::RenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    // default do nothing
  }
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  material: T,
  last_material: Option<T>,
  gpu: Option<T::GPU>,
  handle: MaterialHandle,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T, handle: MaterialHandle) -> Self {
    Self {
      material,
      last_material: None,
      gpu: None,
      handle,
    }
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub model_gpu: &'a TransformGPU,
  pub pipelines: &'a mut PipelineResourceManager,
  pub pass: &'a dyn ViewerRenderPass,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub textures: &'a mut WatchedArena<SceneTexture2D>,
  pub samplers: &'a mut WatchedArena<SceneSampler>,
  pub reference_finalization: &'a ReferenceFinalization,
}

pub struct PipelineCreateCtx<'a> {
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub pass: &'a dyn ViewerRenderPass,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub pipelines: &'a PipelineResourceManager,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub pass: &'a dyn ViewerRenderPass,
}

pub trait Material {
  fn on_ref_resource_changed(&mut self);
  fn update<'a>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a>);
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, ctx: &SceneMaterialPassSetupCtx<'a>);
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a>) {
    self
      .gpu
      .get_or_insert_with(|| T::create(&mut self.material, self.handle, gpu, ctx))
      .update(&self.material, gpu, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    self.gpu.as_ref().unwrap().setup_pass(pass, ctx)
  }
  fn on_ref_resource_changed(&mut self) {

    // todo optimize use last material
    self.gpu = None;
  }
}

pub type CommonMaterialPipelineVariantContainer =
  TopologyPipelineVariant<StatePipelineVariant<PipelineUnit>>;

pub struct CommonPipelineVariantKey(ValueID<PreferredMaterialStates>, wgpu::PrimitiveTopology);

impl AsRef<ValueID<PreferredMaterialStates>> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &ValueID<PreferredMaterialStates> {
    &self.0
  }
}

impl AsRef<wgpu::PrimitiveTopology> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &wgpu::PrimitiveTopology {
    &self.1
  }
}

pub struct PipelineResourceManager {
  pub materials: HashMap<TypeId, Box<dyn Any>>,
  pub basic: CommonMaterialPipelineVariantContainer,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self {
      materials: HashMap::new(),
      basic: Default::default(),
    }
  }
}

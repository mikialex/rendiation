use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
};
pub mod bindable;
pub use bindable::*;
pub mod states;
pub use states::*;

pub mod basic;
pub use basic::*;

use rendiation_algebra::Mat4;

use crate::Renderer;

use super::{
  Camera, CameraBindgroup, Material, MaterialHandle, Mesh, ReferenceFinalization, RenderStyle,
  Scene, SceneSampler, SceneTexture2D, StandardForward, TransformGPU, TypedMaterialHandle,
  WatchedArena,
};

impl Scene {
  fn add_material_inner<M: Material, F: FnOnce(MaterialHandle) -> M>(
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
    M::GPU: MaterialGPUResource<StandardForward, Source = M>,
  {
    let handle = self.add_material_inner(|handle| MaterialCell::new(material, handle));
    TypedMaterialHandle {
      handle,
      ty: PhantomData,
    }
  }
}

pub trait MaterialCPUResource {
  type GPU;
  fn create<S>(
    &mut self,
    handle: MaterialHandle,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource<S>: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) {
    // default do nothing
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    // default do nothing
  }
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  material: T,
  gpu: Option<T::GPU>,
  handle: MaterialHandle,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T, handle: MaterialHandle) -> Self {
    Self {
      material,
      gpu: None,
      handle,
    }
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a, S> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub model_gpu: &'a TransformGPU,
  pub pipelines: &'a mut PipelineResourceManager,
  pub style: &'a S,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub textures: &'a mut WatchedArena<SceneTexture2D>,
  pub samplers: &'a mut WatchedArena<SceneSampler>,
  pub reference_finalization: &'a ReferenceFinalization,
}

pub struct PipelineCreateCtx<'a> {
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub active_mesh: &'a Box<dyn Mesh>,
}

pub struct SceneMaterialPassSetupCtx<'a, S> {
  pub pipelines: &'a PipelineResourceManager,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub style: &'a S,
}

pub trait MaterialStyleAbility<S: RenderStyle> {
  fn update<'a>(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>);
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  );
}

impl<T, S> MaterialStyleAbility<S> for MaterialCell<T>
where
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<S, Source = T>,
  S: RenderStyle,
{
  fn update<'a>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>,
  ) {
    self
      .gpu
      .get_or_insert_with(|| T::create(&mut self.material, self.handle, renderer, ctx))
      .update(&self.material, renderer, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    self.gpu.as_ref().unwrap().setup_pass(pass, ctx)
  }
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource + 'static,
  MaterialCell<T>: MaterialStyleAbility<StandardForward>,
{
  fn on_ref_resource_changed(&mut self) {
    self.gpu = None;
  }
}

pub struct PipelineResourceManager {
  pub materials: HashMap<TypeId, Box<dyn Any>>,
  pub basic: StatePipelineVariant,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self {
      materials: HashMap::new(),
      basic: Default::default(),
    }
  }
}

use std::{
  any::Any,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};
pub mod bindable;
pub use bindable::*;
pub mod states;
pub use states::*;

pub mod state_material;
pub use state_material::*;

pub mod basic;
pub use basic::*;
pub mod fatline;
pub use fatline::*;
pub mod env_background;
pub use env_background::*;

use rendiation_algebra::Mat4;
use rendiation_webgpu::{
  BindGroupLayoutManager, PipelineRequester, PipelineResourceManager, PipelineUnit,
  PipelineVariantContainer, TopologyPipelineVariant, GPU,
};

use crate::*;

impl Scene {
  fn add_material_inner<M: Material + 'static, F: FnOnce(MaterialHandle) -> M>(
    &mut self,
    creator: F,
  ) -> MaterialHandle {
    self
      .components
      .materials
      .insert_with(|handle| Box::new(creator(handle)))
  }

  pub fn add_material<M>(&mut self, material: M) -> TypedMaterialHandle<M>
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
    M::GPU: PipelineRequester,
    <M::GPU as PipelineRequester>::Container:
      PipelineVariantContainer<<M::GPU as PipelineRequester>::Key>,
  {
    let handle = self.add_material_inner(|handle| MaterialCell::new(material, handle));
    TypedMaterialHandle {
      handle,
      ty: PhantomData,
    }
  }

  pub fn get_mut_material<M>(&mut self, handle: TypedMaterialHandle<M>) -> &mut M
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    &mut self
      .components
      .materials
      .get_mut(handle.handle)
      .unwrap()
      .as_any_mut()
      .downcast_mut::<MaterialCell<M>>()
      .unwrap()
      .material
  }

  pub fn get_material<M>(&self, handle: TypedMaterialHandle<M>) -> &M
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    &self
      .components
      .materials
      .get(handle.handle)
      .unwrap()
      .as_any()
      .downcast_ref::<MaterialCell<M>>()
      .unwrap()
      .material
  }
}

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource: Clone {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized + PipelineRequester {
  type Source: MaterialCPUResource<GPU = Self>;

  /// This Hook will be called before this material rendering(set_pass) happens if any change recorded.
  ///
  /// If return true, means the following procedure will use simple full refresh update logic:
  /// just rebuild the entire gpu resource. This is just for convenient case.
  /// You can also impl incremental update logic to improve performance in high dynamic scenario
  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    _bindgroup_changed: bool,
  ) -> bool {
    true
  }

  fn pipeline_key(&self, source: &Self::Source, ctx: &PipelineCreateCtx) -> Self::Key;
  fn create_pipeline(
    &self,
    source: &Self::Source,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline;

  fn setup_pass_bindgroup<'a>(
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
  property_changed: bool,
  bindgroups_dirty: bool,
  material: T,
  last_material: Option<T>,
  gpu: Option<T::GPU>,
  handle: MaterialHandle,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T, handle: MaterialHandle) -> Self {
    Self {
      property_changed: true,
      bindgroups_dirty: true,
      material,
      last_material: None,
      gpu: None,
      handle,
    }
  }

  fn refresh_cache(&mut self) {
    self.property_changed = false;
    self.bindgroups_dirty = false;
    self.last_material = self.material.clone().into();
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub model_info: Option<(&'b Mat4<f32>, &'b TransformGPU)>,
  pub active_mesh: Option<&'b dyn Mesh>,
  pub base: &'b mut SceneMaterialRenderPrepareCtxBase<'a>,
}

impl<'a> SceneMaterialRenderPrepareCtxBase<'a> {
  pub fn material_ctx_empty<'b>(&'a mut self) -> SceneMaterialRenderPrepareCtx<'a, 'b> {
    SceneMaterialRenderPrepareCtx {
      model_info: None,
      active_mesh: None,
      base: self,
    }
  }
}

impl<'a, 'b> Deref for SceneMaterialRenderPrepareCtx<'a, 'b> {
  type Target = SceneMaterialRenderPrepareCtxBase<'a>;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}

impl<'a, 'b> DerefMut for SceneMaterialRenderPrepareCtx<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.base
  }
}

pub struct SceneMaterialRenderPrepareCtxBase<'a> {
  pub active_camera: &'a CameraData,
  pub camera_gpu: &'a CameraBindgroup,
  pub pass: &'a PassTargetFormatInfo,
  pub textures: &'a mut WatchedArena<SceneTexture2D>,
  pub texture_cubes: &'a mut WatchedArena<SceneTextureCube>,
  pub reference_finalization: &'a ReferenceFinalization,
  pub resources: &'a mut GPUResourceCache,
}

impl<'a, 'b> SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub fn pipeline_ctx(&mut self) -> (&mut PipelineResourceManager, PipelineCreateCtx) {
    (
      &mut self.base.resources.pipeline_resource,
      PipelineCreateCtx {
        layouts: &self.base.resources.layouts,
        active_mesh: self.active_mesh,
        pass: self.base.pass,
      },
    )
  }
}

pub struct PipelineCreateCtx<'a> {
  pub layouts: &'a BindGroupLayoutManager,
  pub active_mesh: Option<&'a dyn Mesh>,
  pub pass: &'a PassTargetFormatInfo,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub resources: &'a GPUResourceCache,
  pub model_gpu: Option<&'a TransformGPU>,
  pub active_mesh: Option<&'a dyn Mesh>,
  pub camera_gpu: &'a CameraBindgroup,
  pub pass: &'a PassTargetFormatInfo,
}

impl<'a> SceneMaterialPassSetupCtx<'a> {
  pub fn pipeline_ctx(&self) -> PipelineCreateCtx {
    PipelineCreateCtx {
      layouts: &self.resources.layouts,
      active_mesh: self.active_mesh,
      pass: self.pass,
    }
  }
}

pub trait Material {
  /// When material's referenced bindable resources(outer ubo, texture) reference has changed
  /// This will be called, and the implementation should dirty it's inner bindgroups
  fn on_ref_resource_changed(&mut self);
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>);
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, ctx: &SceneMaterialPassSetupCtx<'a>);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Material for MaterialCell<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: PipelineRequester,
  <T::GPU as PipelineRequester>::Container:
    PipelineVariantContainer<<T::GPU as PipelineRequester>::Key>,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>) {
    if let Some(self_gpu) = &mut self.gpu {
      if self.property_changed || self.bindgroups_dirty {
        if self_gpu.update(&self.material, gpu, ctx, self.bindgroups_dirty) {
          self.gpu = T::create(&mut self.material, self.handle, gpu, ctx).into();
        }
        self.refresh_cache();
      }
    } else {
      self.gpu = T::create(&mut self.material, self.handle, gpu, ctx).into();
      self.refresh_cache();
    }

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();
    let container = pipelines.get_cache_mut::<T::GPU>();

    let m_gpu = self.gpu.as_mut().unwrap();
    let key = m_gpu.pipeline_key(&self.material, &pipeline_ctx);

    container.request(&key, || {
      m_gpu.create_pipeline(&self.material, &gpu.device, &pipeline_ctx)
    });
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    let gpu = self.gpu.as_ref().unwrap();

    let container = ctx.resources.pipeline_resource.get_cache::<T::GPU>();
    let pipeline_ctx = ctx.pipeline_ctx();
    let key = gpu.pipeline_key(&self.material, &pipeline_ctx);
    let pipeline = container.retrieve(&key);
    pass.set_pipeline(pipeline);

    gpu.setup_pass_bindgroup(pass, ctx)
  }

  fn on_ref_resource_changed(&mut self) {
    self.bindgroups_dirty = true;
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self.property_changed = true;
    self
  }
}

pub type CommonPipelineCache = TopologyPipelineVariant<StatePipelineVariant<PipelineUnit>>;

pub struct CommonPipelineVariantKey(pub ValueID<MaterialStates>, pub wgpu::PrimitiveTopology);

impl AsRef<ValueID<MaterialStates>> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &ValueID<MaterialStates> {
    &self.0
  }
}

impl AsRef<wgpu::PrimitiveTopology> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &wgpu::PrimitiveTopology {
    &self.1
  }
}

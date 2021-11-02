use std::{
  any::Any,
  cell::{Cell, RefCell},
  ops::{Deref, DerefMut},
  rc::Rc,
};
pub mod states;
pub use states::*;

pub mod state_material;
pub use state_material::*;

pub mod wrapper;
pub use wrapper::*;

pub mod flat;
pub use flat::*;
pub mod basic;
pub use basic::*;
pub mod fatline;
pub use fatline::*;
pub mod env_background;
pub use env_background::*;

use rendiation_algebra::Mat4;
use rendiation_webgpu::{
  BindGroupLayoutManager, GPURenderPass, PipelineBuilder, PipelineRequester,
  PipelineResourceManager, PipelineUnit, PipelineVariantContainer, TopologyPipelineVariant, GPU,
};

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource: Clone {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized + PipelineRequester {
  type Source: MaterialCPUResource<GPU = Self>;

  /// This Hook will be called before this material rendering(set_pass)
  ///
  /// If return true, means the following procedure will use simple full refresh update logic:
  /// just rebuild the entire gpu resource. This is just for convenient case.
  /// You can also impl incremental update logic to improve performance in high dynamic scenario
  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    _bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> bool {
    true
  }

  fn pipeline_key(
    &self,
    source: &Self::Source,
    ctx: &PipelineCreateCtx,
  ) -> <Self::Container as PipelineVariantContainer>::Key;
  fn create_pipeline(
    &self,
    source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline;

  fn setup_pass_bindgroup<'a>(
    &self,
    _pass: &mut GPURenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx,
  ) {
    // default do nothing
  }
}

pub struct BindGroupDirtyWatcher {
  dirty: Cell<bool>,
}
impl Default for BindGroupDirtyWatcher {
  fn default() -> Self {
    Self {
      dirty: Cell::new(false),
    }
  }
}

impl BindGroupDirtyNotifier for BindGroupDirtyWatcher {
  fn notify_dirty(&self) {
    self.dirty.set(true);
  }
}

pub struct MaterialCell<T: MaterialCPUResource> {
  inner: Rc<RefCell<MaterialCellInner<T>>>,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T) -> Self {
    let material = MaterialCellInner::new(material);
    Self {
      inner: Rc::new(RefCell::new(material)),
    }
  }
}

impl<T: MaterialCPUResource> Clone for MaterialCell<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct MaterialCellInner<T>
where
  T: MaterialCPUResource,
{
  property_changed: bool,
  material: T,
  bindgroup_watcher: Rc<BindGroupDirtyWatcher>,
  last_material: Option<T>, // todo
  gpu: Option<T::GPU>,
}

impl<T: MaterialCPUResource> MaterialCellInner<T> {
  pub fn new(material: T) -> Self {
    Self {
      property_changed: true,
      bindgroup_watcher: Default::default(),
      material,
      last_material: None,
      gpu: None,
    }
  }

  fn refresh_cache(&mut self) {
    self.property_changed = false;
    self.bindgroup_watcher.dirty.set(false);
    self.last_material = self.material.clone().into();
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub model_info: Option<(&'b Mat4<f32>, &'b TransformGPU)>,
  pub active_mesh: Option<&'b dyn Mesh>,
  pub base: &'b mut SceneMaterialRenderPrepareCtxBase<'a>,
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
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>);
  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Material for MaterialCellInner<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: PipelineRequester,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>) {
    if let Some(self_gpu) = &mut self.gpu {
      if self.property_changed || self.bindgroup_watcher.dirty.get() {
        if self_gpu.update(&self.material, gpu, ctx, &self.bindgroup_watcher) {
          self.gpu = T::create(&mut self.material, gpu, ctx, &self.bindgroup_watcher).into();
        }
        self.refresh_cache();
      }
    } else {
      self.gpu = T::create(&mut self.material, gpu, ctx, &self.bindgroup_watcher).into();
      self.refresh_cache();
    }

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();
    let container = pipelines.get_cache_mut::<T::GPU>();

    let m_gpu = self.gpu.as_mut().unwrap();
    let key = m_gpu.pipeline_key(&self.material, &pipeline_ctx);

    container.request(&key, || {
      let mut builder = Default::default();
      m_gpu.create_pipeline(&self.material, &mut builder, &gpu.device, &pipeline_ctx)
    });
  }

  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx) {
    let gpu = self.gpu.as_ref().unwrap();

    let container = ctx.resources.pipeline_resource.get_cache::<T::GPU>();
    let pipeline_ctx = ctx.pipeline_ctx();
    let key = gpu.pipeline_key(&self.material, &pipeline_ctx);
    let pipeline = container.retrieve(&key);
    pass.set_pipeline_owned(pipeline);

    gpu.setup_pass_bindgroup(pass, ctx)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self.property_changed = true;
    self
  }
}

impl<T> Material for MaterialCell<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: PipelineRequester,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, ctx)
  }

  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx) {
    let inner = self.inner.borrow();
    inner.setup_pass(pass, ctx)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    {
      let mut inner = self.inner.borrow_mut();
      inner.as_any_mut();
    }
    self
  }
}

pub type CommonPipelineCache<T = PipelineUnit> = TopologyPipelineVariant<StatePipelineVariant<T>>;
